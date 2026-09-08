//! `ga cross` - what each game's ai actually throws, and what the six crossings between them
//! deliver, so that a price is measured rather than guessed.
//!
//! An attack is priced by the *sender*, in the receiver's own units, because only the sender
//! knows what the clear took (see [`engine::game::ForeignPrices`]). That leaves six directed
//! prices between three games and no way to reason a number out of first principles: a
//! garbage block, a row and a nuisance puyo are not the same thing, and neither are the
//! clears that earn them.
//!
//! So this measures the one thing that *is* comparable. Each game's ai plays alone, and every
//! attack it sends is kept **as the sender priced it** - `strength` at home and
//! [`Attack::strength_for`] abroad, which is the shipped table rather than a number worked out
//! on paper. Two rates come off that:
//!
//! * **at home**: what a player of a game faces from an opponent of the same game, per minute,
//!   as a share of their own board;
//! * **abroad**: what the same player faces from each *other* game at the current prices.
//!
//! The ratio of the two is the number to tune on. **1.0 is a foreign opponent pressing exactly
//! as hard as a home one**, which is the only definition of fair here that does not need a
//! currency inventing - and the two crossings that already existed and play well, Dr. Rustario
//! against Rustris, sit at roughly a half and nine tenths of it. That band is the target, and
//! nothing should be over 1.
//!
//! The `parity` column is the price that *would* put a crossing at 1.0 exactly, as a guide for
//! setting a new one; it is not what ships. Two things make it too hot on its own: the three
//! ais are not equally strong, and a Puyo player alone is in a chain-building paradise with
//! nothing arriving to offset - it throws four boards a minute where Rustris throws a third of
//! one.
//!
//! ```shell
//! cargo run --release -- ga cross [seeds] [minutes] [difficulty] [ai]
//! ```
//!
//! Each game plays `seeds` boards of up to `minutes` minutes of game time, dealt by
//! [`VersusMode`] at the versus difficulty dial `difficulty`, driven by the same ai a fielded
//! `ai` opponent is - **at that opponent's own key delay**, since a row that presses keys
//! every 400 ms throws far less than one at full speed and it is the fielded one a price has
//! to be fair to. A board that is buried stops there and its time counts only as far as it
//! got, so a rate is always per minute *played*.

use crate::games::{AiBrain, GameKind};
use crate::modes::{AiDifficulty, Difficulty, VersusAi, VersusMode};
use engine::game::{ids, Attack, Game, GameEvent, GameId, StageState};
use std::time::Duration;

/// one frame, as the match loop steps a game
const STEP: Duration = Duration::from_millis(16);

const DEFAULT_SEEDS: u32 = 5;
const DEFAULT_MINUTES: u64 = 10;
const DEFAULT_DIFFICULTY: u32 = 5;

/// the engine id of a game, which is what a price is keyed on
fn game_id(game: GameKind) -> GameId {
    match game {
        GameKind::DrRustario => ids::DR_RUSTARIO,
        GameKind::Rustris => ids::RUSTRIS,
        GameKind::Puyo => ids::PUYO,
        GameKind::RustleFighter => ids::RUSTLE_FIGHTER,
    }
}

/// How many cells of board one unit of a game's attack fills, which is what makes two games'
/// rates readable side by side.
///
/// A row of Rustris garbage is the whole width less its hole; a Dr. Rustario garbage block and
/// a nuisance puyo are one cell each.
fn cells_per_unit(game: GameKind) -> u32 {
    match game {
        GameKind::Rustris => 9,
        // a Dr. Rustario garbage block, a nuisance puyo and a counter gem are one cell each
        GameKind::DrRustario | GameKind::Puyo | GameKind::RustleFighter => 1,
    }
}

/// what one game's ai did over every seed
struct Measured {
    game: GameKind,
    /// game time actually played, which is short of the cap for any board that was buried
    played: Duration,
    /// how many boards ended in a game over rather than reaching the cap
    buried: u32,
    seeds: u32,
    /// every attack sent, as the sender priced it for every game it can reach
    sent: Vec<Attack>,
    /// pieces locked, which says whether the ai was playing at all
    locked: u32,
    /// stages finished: a bottle cleared, ten lines, thirty puyos
    stages: u32,
    /// the board this game is played on, for reading a rate as a share of it
    cells: u32,
}

impl Measured {
    fn minutes(&self) -> f64 {
        (self.played.as_secs_f64() / 60.0).max(f64::MIN_POSITIVE)
    }

    fn attacks_per_minute(&self) -> f64 {
        self.sent.len() as f64 / self.minutes()
    }

    /// this game's own units thrown per minute of play
    fn home_per_minute(&self) -> f64 {
        self.sent.iter().map(|a| a.strength).sum::<u32>() as f64 / self.minutes()
    }

    fn mean_attack(&self) -> f64 {
        if self.sent.is_empty() {
            0.0
        } else {
            self.sent.iter().map(|a| a.strength).sum::<u32>() as f64 / self.sent.len() as f64
        }
    }

    /// what these attacks are worth per minute to a player of `receiver`, in that game's
    /// units, at the prices the sending game ships
    fn foreign_per_minute(&self, receiver: GameKind) -> f64 {
        self.sent
            .iter()
            .map(|a| a.strength_for(game_id(receiver)))
            .sum::<u32>() as f64
            / self.minutes()
    }

    /// how many of them cross at all, rather than being worth nothing to that receiver
    fn crossing(&self, receiver: GameKind) -> usize {
        self.sent
            .iter()
            .filter(|a| a.strength_for(game_id(receiver)) > 0)
            .count()
    }

    /// a rate of `units` of `game`'s attack per minute, as a share of `game`'s own board
    fn boards_per_minute(units: f64, game: GameKind, cells: u32) -> f64 {
        units * cells_per_unit(game) as f64 / cells as f64
    }

    /// what a player of this game faces per minute from an opponent of the same game, as a
    /// share of their board - the number every crossing into it is measured against
    fn home_boards_per_minute(&self) -> f64 {
        Self::boards_per_minute(self.home_per_minute(), self.game, self.cells)
    }
}

/// every attack size seen, smallest first, as `size x count`
fn histogram(sent: &[Attack], of: impl Fn(&Attack) -> u32) -> String {
    let mut counts: Vec<(u32, usize)> = vec![];
    for size in sent.iter().map(&of).filter(|size| *size > 0) {
        match counts.iter_mut().find(|(s, _)| *s == size) {
            Some((_, count)) => *count += 1,
            None => counts.push((size, 1)),
        }
    }
    counts.sort_by_key(|(size, _)| *size);
    counts
        .iter()
        .map(|(size, count)| format!("{size}x{count}"))
        .collect::<Vec<String>>()
        .join(" ")
}

/// play one board of one game to the cap or to a burial, keeping what it threw
fn measure(
    game: GameKind,
    seed: u64,
    minutes: u64,
    difficulty: Difficulty,
    ai: AiDifficulty,
) -> Measured {
    let mode = VersusMode::dealing(seed, difficulty);
    let mut board = mode
        .new_games(game, 1, 0)
        .expect("a board")
        .pop()
        .expect("a board");
    let mut brains: Vec<Box<dyn AiBrain>> = VersusAi::Opponent(ai)
        .ai_players(game)
        .into_iter()
        .map(|(_, brain)| brain)
        .collect();

    let cap = Duration::from_secs(60 * minutes);
    let mut measured = Measured {
        game,
        played: Duration::ZERO,
        buried: 0,
        seeds: 1,
        sent: vec![],
        locked: 0,
        stages: 0,
        cells: Game::board_width(&board) * Game::board_height(&board),
    };
    let mut completed = 0;
    while measured.played < cap {
        for brain in brains.iter_mut() {
            brain.act(&mut board, STEP);
        }
        let mut events = Game::drain_events(&mut board);
        Game::update(&mut board, STEP);
        events.extend(Game::drain_events(&mut board));
        measured.played += STEP;

        let mut stage_complete = false;
        for event in events {
            match event {
                GameEvent::Lock { .. } => measured.locked += 1,
                GameEvent::AttackSent(attack) => measured.sent.push(attack),
                GameEvent::StageComplete => stage_complete = true,
                GameEvent::GameOver => {
                    measured.buried = 1;
                    return measured;
                }
                _ => {}
            }
        }
        if stage_complete {
            completed += 1;
            measured.stages += 1;
            if Game::stage_state(&board) == StageState::StageComplete {
                Game::next_stage(&mut board).expect("the next stage");
            }
            Game::set_completed_stages(&mut board, completed);
        }
    }
    measured
}

fn arg<T: std::str::FromStr>(args: &[String], index: usize, default: T) -> T {
    args.get(index)
        .and_then(|a| a.parse::<T>().ok())
        .unwrap_or(default)
}

pub fn cross_main(args: &[String]) -> Result<(), String> {
    let seeds = arg(args, 0, DEFAULT_SEEDS);
    let minutes = arg(args, 1, DEFAULT_MINUTES);
    let difficulty = Difficulty::new(arg(args, 2, DEFAULT_DIFFICULTY));
    let ai = args
        .get(3)
        .and_then(|name| AiDifficulty::from_name(name))
        .unwrap_or(AiDifficulty::Hard);

    println!(
        "ga cross: {seeds} seeds x up to {minutes} minutes a game, at versus difficulty {}, \
         against the {} ai at its own key delay\n",
        arg(args, 2, DEFAULT_DIFFICULTY),
        ai.name()
    );

    let mut all: Vec<Measured> = vec![];
    for game in GameKind::ALL {
        let mut total: Option<Measured> = None;
        for seed in 0..seeds as u64 {
            let one = measure(game, seed, minutes, difficulty, ai);
            println!(
                "  {:<12} seed {seed}: {:.1} min, {} attacks, {} units, {} pieces, {} stages{}",
                game.name(),
                one.minutes(),
                one.sent.len(),
                one.sent.iter().map(|a| a.strength).sum::<u32>(),
                one.locked,
                one.stages,
                if one.buried > 0 { ", buried" } else { "" }
            );
            match total.as_mut() {
                None => total = Some(one),
                Some(total) => {
                    total.played += one.played;
                    total.buried += one.buried;
                    total.seeds += one.seeds;
                    total.sent.extend(one.sent);
                    total.locked += one.locked;
                    total.stages += one.stages;
                }
            }
        }
        all.push(total.expect("at least one seed"));
    }

    println!("\nat home: what a player faces from an opponent of their own game\n");
    println!("| game | played | buried | attacks/min | units/min | mean attack | boards/min |");
    println!("|--|--|--|--|--|--|--|");
    for m in all.iter() {
        println!(
            "| {} | {:.0} min | {}/{} | {:.2} | {:.1} | {:.1} | {:.3} |",
            m.game.name(),
            m.minutes(),
            m.buried,
            m.seeds,
            m.attacks_per_minute(),
            m.home_per_minute(),
            m.mean_attack(),
            m.home_boards_per_minute(),
        );
    }
    println!("\nattack sizes at home, as `size x count`:");
    for m in all.iter() {
        println!(
            "  {:<12} {}",
            m.game.name(),
            histogram(&m.sent, |a| a.strength)
        );
    }

    println!(
        "\nabroad: what the same attacks deliver at the prices each game ships. `home` is the \
         \nlast column above, so `share` of 1.0 is a foreign opponent pressing exactly as hard \
         \nas a home one, and `parity` is the price that would make it so\n"
    );
    println!("| sender | receiver | crossing | units/min | boards/min | share of home | parity |");
    println!("|--|--|--|--|--|--|--|");
    for sender in all.iter() {
        for receiver in all.iter() {
            if sender.game == receiver.game {
                continue;
            }
            let units = sender.foreign_per_minute(receiver.game);
            let boards = Measured::boards_per_minute(units, receiver.game, receiver.cells);
            let home = receiver.home_boards_per_minute();
            let parity = receiver.home_per_minute() / sender.home_per_minute();
            println!(
                "| {} | {} | {}/{} | {:.1} | {:.3} | {:.2} | {:.3} x strength |",
                sender.game.name(),
                receiver.game.name(),
                sender.crossing(receiver.game),
                sender.sent.len(),
                units,
                boards,
                boards / home,
                parity,
            );
        }
    }

    println!("\nattack sizes as they land abroad, as `size x count`:");
    for sender in all.iter() {
        for receiver in all.iter() {
            if sender.game == receiver.game {
                continue;
            }
            println!(
                "  {:<12} -> {:<12} {}",
                sender.game.name(),
                receiver.game.name(),
                histogram(&sender.sent, |a| a.strength_for(game_id(receiver.game)))
            );
        }
    }
    Ok(())
}
