//! `ga dr passes [model] [boards]`: replay a model on the unseen training games and show where
//! it passed on finishing a bottle, cleared only vitamins with a kill on offer, or cleared nothing
//! at all with one on offer, and what came of it. `model` is `n64`, `embedded` (the default) or a file of weights as a checkpoint prints.

use crate::game::ai::agent::DrAiAgent;
use crate::game::ai::evaluator::{inputs, raw_inputs, Scorer};
use crate::game::ai::explain::{read_pair, FeatureScenario};
use crate::game::ai::features::{BottleAnalysis, BottleFeatures};
use crate::game::ai::genetic::{
    load_weights, network_agent, report_play, report_training, training_fixture, UNSEEN_PER_LEVEL,
    UNSEEN_SEED_BLOCK,
};
use crate::game::ai::models::{self, DrNeuralNetwork};
use crate::game::ai::placement::{Placement, PlacementSearch, Reach};
use crate::game::ai::probe::NOW_NAMES;
use crate::game::ai::run::{N64_FIRE, START_LEVELS};
use crate::game::ai::N64Ai;
use crate::game::bottle::{Bottle, BOTTLE_HEIGHT, BOTTLE_WIDTH};
use engine::ai::{Seed, Tensor, BOTTLE_FEATURE_INPUTS};
use engine::game::Game as _;
use rayon::prelude::*;

/// half the bottle, where the spawn columns start to crowd a pill
const HIGH: i32 = 8;

/// what made the choice, so the tool can make it again on a copy of the bottle
#[derive(Clone, Copy)]
#[allow(clippy::large_enum_variant)]
enum Brain {
    N64(N64Ai),
    Network(DrNeuralNetwork),
}

impl Brain {
    fn agent(&self) -> DrAiAgent {
        match self {
            Brain::N64(_) => DrAiAgent::n64(),
            Brain::Network(network) => DrAiAgent::new(*network),
        }
    }

    /// the placement the agent picks, the same way it does, with every score when there are any
    fn choose(&self, bottle: &Bottle, placements: &[Placement]) -> (usize, Option<Vec<f64>>) {
        match self {
            Brain::N64(ai) => (ai.choose(bottle, placements).unwrap_or(0), None),
            Brain::Network(network) => {
                let features: Vec<BottleFeatures> =
                    placements.iter().map(|p| p.features()).collect();
                let scores = Scorer::Network(*network).rank(&features);
                let best = (0..placements.len())
                    .max_by(|a, b| {
                        scores[*a]
                            .total_cmp(&scores[*b])
                            .then_with(|| placements[*b].inputs().cmp(placements[*a].inputs()))
                    })
                    .unwrap_or(0);
                (best, Some(scores))
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    PassedFinish,
    Tidied,
    /// cleared nothing at all with a kill on offer
    Ignored,
}

/// one choice worth looking at, the bottle it was made on and the placement it passed up
struct Moment {
    kind: Kind,
    game: usize,
    level: u32,
    bottle_number: u32,
    pill: u32,
    viruses: u32,
    before: Bottle,
    chosen: Bottle,
    passed_up: Bottle,
    chosen_features: BottleFeatures,
    passed_up_features: BottleFeatures,
    /// the score the chosen placement won by, when the brain has scores
    margin: Option<f64>,
}

/// how a run of passed finishes on one bottle ended
#[derive(Clone, Copy, Debug)]
enum Ending {
    /// finished `pills` pills after the first pass, the last pill sending `last_sent` blocks
    Finished {
        pills: u32,
        sent: u32,
        last_sent: u32,
    },
    Buried {
        pills: u32,
    },
    OutOfPills {
        pills: u32,
    },
}

struct Episode {
    passes: u32,
    first_pill: u32,
    sent_before: u32,
    ending: Option<Ending>,
}

#[derive(Default)]
struct Watched {
    moments: Vec<Moment>,
    episodes: Vec<Episode>,
    bottles_finished: u32,
    /// finishes the agent took the first time one was on offer
    finished_at_once: u32,
    /// pills where the recount disagreed with the agent's own tally, which should be none
    disagreements: u32,
    pills: u32,
    /// pills with a kill on offer, by the most it would have killed: one, or two and more
    kills_on_offer: [u32; 2],
    /// of those, the ones that cleared nothing at all
    ignored: [u32; 2],
    /// vitamins in the bottle as each pill spawned, summed
    loose: u64,
}

/// vitamins in the bottle, the loose blocks a player has put down and not cleared
fn loose(bottle: &Bottle) -> u32 {
    let mut count = 0;
    for y in 0..BOTTLE_HEIGHT {
        for x in 0..BOTTLE_WIDTH {
            let block = bottle.block_at(x, y);
            count += (!block.is_empty() && !block.is_virus()) as u32;
        }
    }
    count
}

fn watch(brain: Brain, index: usize) -> Watched {
    let block = Seed::from(UNSEEN_SEED_BLOCK);
    let fixture = training_fixture(block, START_LEVELS.len() * UNSEEN_PER_LEVEL, Some(N64_FIRE));
    let (mut headless, level) = fixture.headless(brain.agent(), block, index);
    let mut watched = Watched::default();
    let mut episode: Option<Episode> = None;
    let mut sent_at_decision = 0;

    loop {
        let tally = headless.choices();
        let bottle = headless.game().bottle().clone();
        let stage = headless.game().completed_stages();
        let over = headless.update();
        let now = headless.choices();
        let sent = headless.traffic().blocks_sent;

        if now.pills > tally.pills {
            sent_at_decision = sent;
            let stats = bottle.stats();
            let placements = bottle.placements_within(Reach::default(), stats);
            let (chosen, scores) = brain.choose(&bottle, &placements);
            let killed = |p: &Placement| -p.features().delta().viruses();
            let viruses = stats.viruses();
            let most = placements.iter().map(killed).max().unwrap_or(0);
            let passed = viruses > 0 && most == viruses && killed(&placements[chosen]) < viruses;
            let cleared = placements[chosen].features().placement().patterns_cleared() > 0;
            let tidied = most > 0 && killed(&placements[chosen]) == 0 && cleared;
            let ignored = most > 0 && killed(&placements[chosen]) == 0 && !cleared;
            watched.pills += 1;
            watched.loose += loose(&bottle) as u64;
            if most > 0 {
                let size = (most >= 2) as usize;
                watched.kills_on_offer[size] += 1;
                watched.ignored[size] += ignored as u32;
            }
            if passed as u32 != now.finishes_passed - tally.finishes_passed
                || tidied as u32 != now.tidied_instead - tally.tidied_instead
            {
                watched.disagreements += 1;
            }
            let finish_on_offer = viruses > 0 && most == viruses;
            if finish_on_offer && !passed && episode.is_none() {
                watched.finished_at_once += 1;
            }

            if passed || tidied || ignored {
                // the best scored of the placements that kill the most
                let passed_up = (0..placements.len())
                    .filter(|i| killed(&placements[*i]) == most)
                    .max_by(|a, b| match &scores {
                        Some(s) => s[*a].total_cmp(&s[*b]),
                        None => b.cmp(a),
                    })
                    .unwrap();
                let kind = if passed {
                    Kind::PassedFinish
                } else if tidied {
                    Kind::Tidied
                } else {
                    Kind::Ignored
                };
                watched.moments.push(Moment {
                    kind,
                    game: index,
                    level,
                    bottle_number: level + stage,
                    pill: now.pills,
                    viruses: viruses as u32,
                    before: bottle.clone(),
                    chosen: placements[chosen].settled().clone(),
                    passed_up: placements[passed_up].settled().clone(),
                    chosen_features: placements[chosen].features(),
                    passed_up_features: placements[passed_up].features(),
                    margin: scores.map(|s| s[chosen] - s[passed_up]),
                });
            }
            if passed {
                let current = episode.get_or_insert(Episode {
                    passes: 0,
                    first_pill: now.pills,
                    sent_before: headless.traffic().blocks_sent,
                    ending: None,
                });
                current.passes += 1;
            }
        }

        if headless.game().completed_stages() > stage {
            watched.bottles_finished += 1;
            if let Some(mut done) = episode.take() {
                done.ending = Some(Ending::Finished {
                    pills: now.pills - done.first_pill,
                    sent: sent - done.sent_before,
                    last_sent: sent - sent_at_decision,
                });
                watched.episodes.push(done);
            }
        }

        if let Some(result) = over {
            if let Some(mut done) = episode.take() {
                let pills = now.pills - done.first_pill;
                done.ending = Some(if result.game_over() {
                    Ending::Buried { pills }
                } else {
                    Ending::OutOfPills { pills }
                });
                watched.episodes.push(done);
            }
            return watched;
        }
    }
}

/// three bottles side by side under their captions
fn side_by_side(captions: [&str; 3], bottles: [&Bottle; 3]) {
    let pictures: Vec<Vec<String>> = bottles
        .iter()
        .map(|b| format!("{:?}", b).lines().map(str::to_string).collect())
        .collect();
    println!(
        "    {:<16}{:<16}{:<16}",
        captions[0], captions[1], captions[2]
    );
    for ((left, middle), right) in pictures[0].iter().zip(&pictures[1]).zip(&pictures[2]) {
        println!("    {:<16}{:<16}{:<16}", left, middle, right);
    }
}

fn show(moment: &Moment) {
    let what = match moment.kind {
        Kind::PassedFinish => "passed on finishing",
        Kind::Tidied => "cleared only vitamins",
        Kind::Ignored => "cleared nothing",
    };
    let pill = moment
        .before
        .pill()
        .map(|p| {
            let shape = p.shape();
            format!(
                "{}{}",
                shape.left_color().to_char(),
                shape.right_color().to_char()
            )
        })
        .unwrap_or_default();
    println!(
        "\ngame {} from level {}, bottle {}, pill {}: {} with {} viruses left, pill {}{}",
        moment.game,
        moment.level,
        moment.bottle_number,
        moment.pill,
        what,
        moment.viruses,
        pill,
        moment
            .margin
            .map(|m| format!(", chosen by {:.3}", m))
            .unwrap_or_default()
    );
    side_by_side(
        ["before", "chosen", "passed up"],
        [&moment.before, &moment.chosen, &moment.passed_up],
    );
    let chosen = raw_inputs(&moment.chosen_features);
    let passed_up = raw_inputs(&moment.passed_up_features);
    for (input, name) in NOW_NAMES.iter().enumerate() {
        if chosen[input] != passed_up[input] {
            println!(
                "    {:<30}{:>6}{:>8}",
                name, chosen[input], passed_up[input]
            );
        }
    }
}

pub fn ga_main_passes(args: &[String]) -> Result<(), String> {
    let brain = match args.first().map(String::as_str).unwrap_or("embedded") {
        "n64" => Brain::N64(N64Ai::new()),
        "embedded" => Brain::Network(models::survival_trained()),
        path => Brain::Network(load_weights(path)?.into()),
    };
    let boards: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(6);

    match brain {
        Brain::N64(_) => report_training("n64 top row", DrAiAgent::n64),
        Brain::Network(network) => {
            report_play("model", network.into());
            report_training("model", network_agent(network.into()))
        }
    };

    let games = START_LEVELS.len() * UNSEEN_PER_LEVEL;
    let watched: Vec<Watched> = (0..games)
        .into_par_iter()
        .map(|index| watch(brain, index))
        .collect();

    let moments: Vec<&Moment> = watched.iter().flat_map(|w| &w.moments).collect();
    let episodes: Vec<&Episode> = watched.iter().flat_map(|w| &w.episodes).collect();
    let disagreements: u32 = watched.iter().map(|w| w.disagreements).sum();
    let bottles: u32 = watched.iter().map(|w| w.bottles_finished).sum();
    let at_once: u32 = watched.iter().map(|w| w.finished_at_once).sum();
    println!(
        "{} bottles finished over {} games; took a finish the first time it was offered {} times",
        bottles, games, at_once
    );
    if disagreements > 0 {
        println!(
            "the recount disagreed with the agent {} times",
            disagreements
        );
    }

    let passes = moments
        .iter()
        .filter(|m| m.kind == Kind::PassedFinish)
        .count();
    println!(
        "{} finishes passed in {} runs on a bottle:",
        passes,
        episodes.len()
    );
    let mut finished = 0;
    let (mut delay, mut sent, mut last_sent, mut combo_finishes) = (0, 0, 0, 0);
    for episode in &episodes {
        match episode.ending {
            Some(Ending::Finished {
                pills,
                sent: s,
                last_sent: l,
            }) => {
                finished += 1;
                delay += pills;
                sent += s;
                last_sent += l;
                combo_finishes += (l > 0) as u32;
            }
            Some(Ending::Buried { pills }) => println!(
                "  a run of {} passes was buried {} pills later",
                episode.passes, pills
            ),
            Some(Ending::OutOfPills { pills }) => println!(
                "  a run of {} passes ran out of pills {} pills later",
                episode.passes, pills
            ),
            None => (),
        }
    }
    if finished > 0 {
        println!(
            "  {} finished, {:.1} pills late on average, sending {} blocks meanwhile; {} ended on \
             a combo that sent {} blocks",
            finished,
            delay as f64 / finished as f64,
            sent,
            combo_finishes,
            last_sent
        );
    }

    let tidies: Vec<&&Moment> = moments.iter().filter(|m| m.kind == Kind::Tidied).collect();
    let combos = tidies
        .iter()
        .filter(|m| m.chosen_features.placement().patterns_cleared() > 1)
        .count();
    let high = tidies
        .iter()
        .filter(|m| {
            let before = m.chosen_features.global() - m.chosen_features.delta();
            before.entrance_height() >= HIGH
        })
        .count();
    println!(
        "{} tidies: {} cleared two or more patterns, {} had the spawn columns half full",
        tidies.len(),
        combos,
        high
    );
    let low_and_single = tidies
        .iter()
        .filter(|m| {
            let before = m.chosen_features.global() - m.chosen_features.delta();
            before.entrance_height() < HIGH && m.chosen_features.placement().patterns_cleared() < 2
        })
        .count();
    println!(
        "  {} with neither: a lone clear of vitamins, low in the bottle",
        low_and_single
    );

    let pills: u32 = watched.iter().map(|w| w.pills).sum();
    let loose: u64 = watched.iter().map(|w| w.loose).sum();
    println!(
        "{:.1} vitamins in the bottle as a pill spawned, on average over {} pills",
        loose as f64 / pills.max(1) as f64,
        pills
    );
    for (size, what) in ["one virus", "two or more"].iter().enumerate() {
        let offered = watched.iter().map(|w| w.kills_on_offer[size]).sum::<u32>();
        let ignored = watched.iter().map(|w| w.ignored[size]).sum::<u32>();
        println!(
            "a kill of {} on offer {} times, cleared nothing instead {} times ({:.1}%)",
            what,
            offered,
            ignored,
            100.0 * ignored as f64 / offered.max(1) as f64
        );
    }
    let ignored: Vec<&&Moment> = moments.iter().filter(|m| m.kind == Kind::Ignored).collect();
    if !ignored.is_empty() {
        println!("  what the chosen placement had over the kill, on average:");
        let mut means = [0.0; NOW_NAMES.len()];
        for moment in &ignored {
            let chosen = raw_inputs(&moment.chosen_features);
            let passed_up = raw_inputs(&moment.passed_up_features);
            for (input, mean) in means.iter_mut().enumerate() {
                *mean += (chosen[input] - passed_up[input]) / ignored.len() as f64;
            }
        }
        for (input, name) in NOW_NAMES.iter().enumerate() {
            if means[input].abs() >= 0.05 {
                println!("    {:<30}{:>+8.2}", name, means[input]);
            }
        }
    }

    for kind in [Kind::PassedFinish, Kind::Tidied, Kind::Ignored] {
        for moment in moments.iter().filter(|m| m.kind == kind).take(boards) {
            show(moment);
        }
    }
    Ok(())
}

/// how many pills after a passed kill the agent is watched for a combo that would excuse it
const EXCUSE_PILLS: u32 = 10;

/// A pill that cleared nothing with a kill on offer and was not followed by a combo, drawn as
/// the chosen placement against the best scored kill.
pub struct Oddity {
    pub game: usize,
    pub level: u32,
    pub bottle_number: u32,
    pub pill: u32,
    pub viruses: u32,
    /// viruses the kill passed up would have destroyed
    pub kills: u32,
    /// the chosen placement first, then the kill
    pub drawn: FeatureScenario,
    /// every input of each, in its own units
    pub raw: [[f64; BOTTLE_FEATURE_INPUTS]; 2],
    /// How far the kill's score moves when one input takes the chosen placement's value, the
    /// rest held: what pulled the network away from the kill, input by input.
    pub pulls: [f64; BOTTLE_FEATURE_INPUTS],
    /// the score the chosen placement won by
    pub margin: f64,
    /// viruses destroyed over the pills that followed on the same bottle
    pub viruses_after: u32,
    /// how many pills that was: [`EXCUSE_PILLS`] unless the bottle ended first
    pub pills_after: u32,
}

/// an oddity until [`EXCUSE_PILLS`] more pills pass without a combo
struct Pending {
    oddity: Oddity,
    sent: u32,
    viruses: u32,
    pills: u32,
    stage: u32,
}

fn oddities_in(network: DrNeuralNetwork, index: usize) -> Vec<Oddity> {
    let block = Seed::from(UNSEEN_SEED_BLOCK);
    let fixture = training_fixture(block, START_LEVELS.len() * UNSEEN_PER_LEVEL, Some(N64_FIRE));
    let (mut headless, level) = fixture.headless(DrAiAgent::new(network), block, index);
    let mut found = vec![];
    let mut pending: Option<Pending> = None;

    loop {
        let tally = headless.choices();
        let bottle = headless.game().bottle().clone();
        let stage = headless.game().completed_stages();
        let over = headless.update();
        let now = headless.choices();
        let sent = headless.traffic().blocks_sent;
        let decided = now.pills > tally.pills;

        if let Some(watch) = &mut pending {
            let resolved = over.is_some()
                || headless.game().completed_stages() != watch.stage
                || (decided && now.pills - watch.oddity.pill >= EXCUSE_PILLS);
            if sent > watch.sent {
                pending = None;
            } else if resolved {
                let mut watch = pending.take().unwrap();
                watch.oddity.viruses_after = headless.viruses() - watch.viruses;
                watch.oddity.pills_after = headless.pills() - watch.pills;
                found.push(watch.oddity);
            }
        }
        if over.is_some() {
            return found;
        }
        if !decided || pending.is_some() {
            continue;
        }

        let stats = bottle.stats();
        let placements = bottle.placements_within(Reach::default(), stats);
        let features: Vec<BottleFeatures> = placements.iter().map(|p| p.features()).collect();
        let scores = Scorer::Network(network).rank(&features);
        let best = |pick: &dyn Fn(usize) -> bool| {
            (0..placements.len()).filter(|i| pick(*i)).max_by(|a, b| {
                scores[*a]
                    .total_cmp(&scores[*b])
                    .then_with(|| placements[*b].inputs().cmp(placements[*a].inputs()))
            })
        };
        let killed = |i: usize| -features[i].delta().viruses();
        let Some(chosen) = best(&|_| true) else {
            continue;
        };
        let most = (0..placements.len()).map(killed).max().unwrap_or(0);
        if most == 0 || killed(chosen) > 0 || features[chosen].placement().patterns_cleared() > 0 {
            continue;
        }
        let kill = best(&|i| killed(i) == most).unwrap();

        let rows = inputs(&features);
        let score =
            |row: [f64; BOTTLE_FEATURE_INPUTS]| network.forward(&Tensor::vector(row)).value();
        let mut pulls = [0.0; BOTTLE_FEATURE_INPUTS];
        for (input, pull) in pulls.iter_mut().enumerate() {
            let mut moved = rows[kill];
            moved[input] = rows[chosen][input];
            *pull = score(moved) - scores[kill];
        }
        pending = Some(Pending {
            oddity: Oddity {
                game: index,
                level,
                bottle_number: level + stage,
                pill: now.pills,
                viruses: stats.viruses() as u32,
                kills: most as u32,
                drawn: read_pair(0, &bottle, &placements, chosen, kill),
                raw: [raw_inputs(&features[chosen]), raw_inputs(&features[kill])],
                pulls,
                margin: scores[chosen] - scores[kill],
                viruses_after: 0,
                pills_after: 0,
            },
            sent,
            viruses: headless.viruses(),
            pills: headless.pills(),
            stage: headless.game().completed_stages(),
        });
    }
}

/// Play `network` on the unseen training games and keep `wanted` pills that cleared nothing with
/// a kill on offer and were not followed by a combo, half of them passing up two viruses or
/// more, at most one a game.
pub fn oddities(network: DrNeuralNetwork, wanted: usize) -> Vec<Oddity> {
    let games = START_LEVELS.len() * UNSEEN_PER_LEVEL;
    // consecutive games start from each of the levels in turn
    let found: Vec<Vec<Oddity>> = (0..games.min(wanted * 4))
        .into_par_iter()
        .map(|index| oddities_in(network, index))
        .collect();
    let (mut doubles, mut singles) = (vec![], vec![]);
    for game in found {
        let mut game = game.into_iter();
        let double = game.by_ref().find(|o| o.kills >= 2);
        let single = game.find(|o| o.kills == 1);
        doubles.extend(double);
        singles.extend(single);
    }
    let mut picked: Vec<Oddity> = doubles.into_iter().take(wanted.div_ceil(2)).collect();
    let rest = wanted - picked.len();
    picked.extend(singles.into_iter().take(rest));
    picked
}
