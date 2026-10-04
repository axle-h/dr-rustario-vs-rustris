//! `ga dr duel <games per level> [sprint|marathon] [a] [b]`: two brains head to head, headless,
//! each sending the other the garbage its combos make. A brain is `n64`, `embedded` (the default
//! for `b`) or a file of weights as a checkpoint prints.

use crate::game::ai::agent::DrAiAgent;
use crate::game::ai::genetic::{load_weights, COMPARE_SEED_BLOCK};
use crate::game::ai::headless_game::{HeadlessGame, HeadlessGameOptions};
use crate::game::ai::models::{self, DrNeuralNetwork};
use crate::game::ai::run::START_LEVELS;
use crate::game::random::{GameRandom, RandomMode};
use crate::game::{Game, GameSpeed};
use engine::ai::{EndGame, Seed};
use rayon::prelude::*;

/// pills either side may place before a duel is called a draw
const DUEL_PILLS: u32 = 2000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Rules {
    /// the first to clear the bottle wins, as the cartridge's versus does
    Sprint,
    /// bottle after bottle until one side is buried
    Marathon,
}

#[derive(Clone, Copy)]
#[allow(clippy::large_enum_variant)]
pub(super) enum Brain {
    N64,
    Network(DrNeuralNetwork),
}

impl Brain {
    fn of(name: &str) -> Result<Self, String> {
        Ok(match name {
            "n64" => Brain::N64,
            "embedded" => Brain::Network(models::survival_trained()),
            path => Brain::Network(load_weights(path)?.into()),
        })
    }

    fn agent(&self) -> DrAiAgent {
        match self {
            Brain::N64 => DrAiAgent::n64(),
            Brain::Network(network) => DrAiAgent::new(*network),
        }
    }
}

/// how one side of a duel went
#[derive(Clone, Copy, Default)]
struct Side {
    viruses: u32,
    bottles: u32,
    pills: u32,
    attacks: u32,
    sent: u32,
    buried: bool,
}

pub(super) struct Duel {
    level: u32,
    /// the winning side, or `None` for a draw
    winner: Option<usize>,
    sides: [Side; 2],
}

/// Both brains dealt the same bottle and pills from `level`, stepped a frame at a time, and only
/// then handed what the other sent, so neither side moves first.
fn duel(brains: [Brain; 2], rules: Rules, seed: Seed, level: u32) -> Duel {
    let options = HeadlessGameOptions {
        top_level: match rules {
            Rules::Sprint => 0,
            Rules::Marathon => u32::MAX,
        },
        ..HeadlessGameOptions::default()
    };
    let deal = |brain: Brain| {
        let random = GameRandom::from_seed(seed.into(), RandomMode::Bag);
        let game = Game::new(level, GameSpeed::Medium, random).expect("could not deal a bottle");
        HeadlessGame::new(
            game,
            brain.agent(),
            options,
            EndGame::of_pieces(DUEL_PILLS),
            seed,
        )
    };
    let mut players = [deal(brains[0]), deal(brains[1])];

    let (winner, results) = loop {
        let results = [players[0].update(), players[1].update()];
        let [a, b] = players.each_mut();
        for attack in a.sent().to_vec() {
            b.receive_attack(attack);
        }
        for attack in b.sent().to_vec() {
            a.receive_attack(attack);
        }
        if results.iter().all(Option::is_none) {
            continue;
        }
        // a side is out when it is buried; it is through when the sprint's bottle is clear
        let out = |side: usize| results[side].is_some_and(|r| r.game_over());
        let through = |side: usize| rules == Rules::Sprint && players[side].stages() > 0;
        let ahead =
            |side: usize| (through(side) || out(1 - side)) && !through(1 - side) && !out(side);
        let winner = (0..2).find(|&side| ahead(side));
        break (winner, results);
    };

    let sides = [0, 1].map(|side| {
        let traffic = players[side].traffic();
        Side {
            viruses: players[side].viruses(),
            bottles: players[side].stages(),
            pills: players[side].pills(),
            attacks: traffic.attacks,
            sent: traffic.blocks_sent,
            buried: results[side].is_some_and(|r| r.game_over()),
        }
    });
    Duel {
        level,
        winner,
        sides,
    }
}

/// `games` duels on the seeds of `block`, from each of [`START_LEVELS`] in turn
pub(super) fn duels(brains: [Brain; 2], rules: Rules, block: Seed, games: usize) -> Vec<Duel> {
    (0..games)
        .into_par_iter()
        .map(|index| {
            let level = START_LEVELS[index % START_LEVELS.len()];
            duel(brains, rules, block + Seed::from(index as u128), level)
        })
        .collect()
}

/// the second brain's share of the decided duels, and that share's standard error
pub(super) fn second_wins(duels: &[Duel]) -> (f64, f64) {
    let decided: Vec<bool> = duels
        .iter()
        .filter_map(|d| d.winner.map(|w| w == 1))
        .collect();
    let n = decided.len().max(1) as f64;
    let share = decided.iter().filter(|won| **won).count() as f64 / n;
    (share, (share * (1.0 - share) / n).sqrt())
}

pub fn ga_main_duel(args: &[String]) -> Result<(), String> {
    let usage = "usage: ga dr duel <games per level> [sprint|marathon] [a] [b]";
    let per_level: usize = args.first().and_then(|s| s.parse().ok()).ok_or(usage)?;
    let rules = match args.get(1).map(String::as_str).unwrap_or("sprint") {
        "sprint" => Rules::Sprint,
        "marathon" => Rules::Marathon,
        _ => return Err(usage.to_string()),
    };
    let names = [
        args.get(2).map(String::as_str).unwrap_or("n64"),
        args.get(3).map(String::as_str).unwrap_or("embedded"),
    ];
    let brains = [Brain::of(names[0])?, Brain::of(names[1])?];

    let games = START_LEVELS.len() * per_level;
    let duels = duels(brains, rules, Seed::from(COMPARE_SEED_BLOCK), games);

    println!(
        "{} against {}, {} duels, {}",
        names[0],
        names[1],
        games,
        match rules {
            Rules::Sprint => "first to clear the bottle",
            Rules::Marathon => "bottle after bottle until one is buried",
        }
    );
    println!(
        "level\twins a\twins b\tdraws\tviruses a\tviruses b\tsent a\tsent b\tburied a\tburied b"
    );
    for level in START_LEVELS.iter().copied().map(Some).chain([None]) {
        let these: Vec<&Duel> = duels
            .iter()
            .filter(|d| level.is_none_or(|level| d.level == level))
            .collect();
        let wins = |side: usize| these.iter().filter(|d| d.winner == Some(side)).count();
        let total = |of: fn(&Side) -> u32, side: usize| -> u32 {
            these.iter().map(|d| of(&d.sides[side])).sum()
        };
        let buried = |side: usize| these.iter().filter(|d| d.sides[side].buried).count();
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            level.map_or("all".to_string(), |l| l.to_string()),
            wins(0),
            wins(1),
            these.len() - wins(0) - wins(1),
            total(|s| s.viruses, 0),
            total(|s| s.viruses, 1),
            total(|s| s.sent, 0),
            total(|s| s.sent, 1),
            buried(0),
            buried(1),
        );
    }

    let (share, spread) = second_wins(&duels);
    let pills = |side: usize| {
        duels
            .iter()
            .map(|d| d.sides[side].pills)
            .sum::<u32>()
            .max(1) as f64
    };
    let attacks = |side: usize| duels.iter().map(|d| d.sides[side].attacks).sum::<u32>() as f64;
    let bottles = |side: usize| duels.iter().map(|d| d.sides[side].bottles).sum::<u32>();
    println!(
        "{} wins {:.1}% ± {:.1}% of the decided duels; attacks a pill {:.3} against {:.3}; bottles \
         cleared {} against {}",
        names[1],
        100.0 * share,
        100.0 * spread,
        attacks(1) / pills(1),
        attacks(0) / pills(0),
        bottles(1),
        bottles(0),
    );
    Ok(())
}
