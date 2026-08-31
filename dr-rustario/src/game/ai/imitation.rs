//! Learning to play from the deterministic ai, which is where a `ga dr auto` run starts.
//!
//! A genetic algorithm can only select between the members it is given, and from random weights
//! it is given almost nothing to select between: most of a first generation clears no virus at
//! all, so the fitness that is supposed to rank them is zero for most of the population. What
//! this module does is hand it a population that is already playing.
//!
//! [`crate::game::ai::n64`] plays, and every pill it is dealt becomes a [`Lesson`]: the
//! placements the agent would be choosing between, exactly as [`evaluator::inputs`] would feed
//! them, and what the deterministic ai thought each of them was worth. A network is then
//! trained by gradient descent to reproduce that opinion. It never plays a game while it is
//! learning and it is never scored on one; it is being taught to rank, and the ranking is what
//! playing is made of.
//!
//! What it learns is the N64's taste, not its ceiling: it is a small network reading a summary
//! of the bottle, where the original reads the bottle itself, so it agrees with it on about
//! half of all pills and plays at around four fifths of its strength. That is the floor the
//! genetic algorithm starts from rather than the roof it is aiming at.

use crate::game::ai::evaluator::{self, Scorer};
use crate::game::ai::features::BottleAnalysis;
use crate::game::ai::input_sequence::Translation;
use crate::game::ai::models::{DrNeuralNetwork, DR_NEURAL_GENOME_SIZE};
use crate::game::ai::n64::{N64Ai, SKILLS, SKILL_ORDER};
use crate::game::ai::placement::{Placement, PlacementSearch};
use crate::game::bottle::Bottle;
use crate::game::random::{GameRandom, RandomMode};
use crate::game::{Game, GameSpeed};
use engine::ai::{Seed, Tensor, BOTTLE_FEATURE_INPUTS};
use engine::game::{Game as _, GameEvent};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaChaRng;
use std::time::Duration;

/// one frame of the headless game, at the rate the real one runs
const STEP: Duration = Duration::from_millis(16);

/// pills without a virus destroyed before a game is called off as going nowhere
const STALL_PILLS: u32 = 200;

/// The virus levels games are started at while the lessons are gathered. A game carries on up
/// through the levels above it, so starting some of them high is what puts a bottle full of
/// viruses in the corpus alongside the nearly empty ones.
const LESSON_LEVELS: [u32; 5] = [0, 5, 10, 15, 20];

/// how many pills are taught from by default
pub const LESSON_PILLS: usize = 10_000;

/// passes over the corpus
pub const EPOCHS: usize = 10;

/// every tenth pill is held back, so what the report says was measured on lessons never taught
const HELD_OUT: usize = 10;

const LEARNING_RATE: f64 = 0.05;
const LEARNING_DECAY: f64 = 0.6;

/// One pill: every placement the agent is choosing between, and what the deterministic ai made
/// of each of them.
///
/// Reaching for the held pill is not among them, and neither the teacher nor the student has a
/// hold: see [`crate::game::ai::agent::DrAiAgent`] for the measurement that settled that.
pub struct Lesson {
    rows: Vec<[f32; BOTTLE_FEATURE_INPUTS]>,
    /// by row: the priority, centred over the pill and brought to unit spread
    target: Vec<Option<f32>>,
    /// by row: whether the ai rated it as highly as anything else on offer
    top: Vec<bool>,
    /// how many of the rows belong to the pill in play, which is now all of them
    own: usize,
    held_out: bool,
}

/// How well a network reproduces the lessons it is measured on.
pub struct Report {
    /// how often it picks a placement the ai rated as highly as the one it played
    pub agreement: f64,
    /// how often it would rather have the pill it can hold than the one in play
    pub holds: f64,
    pub lessons: usize,
}

impl Lesson {
    fn of(placements: &[Placement], priorities: &[Option<i32>], own: usize) -> Option<Self> {
        let features: Vec<_> = placements.iter().map(|p| p.features()).collect();
        let scored: Vec<f64> = priorities.iter().filter_map(|p| p.map(f64::from)).collect();
        if scored.len() < 2 {
            return None;
        }

        // the ai's priorities are on a wildly different scale from one pill to the next - the
        // weights change with the situation, and the whole total is multiplied through when the
        // bottle is lopsided - so every pill is brought to the same spread, and clipped, since
        // a chain worth thousands would otherwise be the only thing a fit ever looked at
        let mean = scored.iter().sum::<f64>() / scored.len() as f64;
        let deviation =
            (scored.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / scored.len() as f64).sqrt();
        if deviation < 1e-9 {
            // every placement is worth the same, so there is nothing here to learn
            return None;
        }
        let best = scored.iter().copied().fold(f64::MIN, f64::max);

        let rows = evaluator::inputs(&features)
            .into_iter()
            .map(|row| row.map(|value| value as f32))
            .collect();
        let target = (0..placements.len())
            .map(|i| {
                priorities.get(i).copied().flatten().map(|priority| {
                    (((priority as f64 - mean) / deviation).clamp(-3.0, 3.0)) as f32
                })
            })
            .collect();
        let top = (0..placements.len())
            .map(|i| priorities.get(i).copied().flatten() == Some(best as i32))
            .collect();

        Some(Self {
            rows,
            target,
            top,
            own,
            held_out: false,
        })
    }

    fn input(&self, row: usize) -> Tensor<BOTTLE_FEATURE_INPUTS> {
        Tensor::vector(self.rows[row].map(f64::from))
    }

    /// which row a network likes best
    fn choice(&self, network: &DrNeuralNetwork) -> usize {
        let scores: Vec<f64> = (0..self.rows.len())
            .map(|row| network.forward(&self.input(row)).value())
            .collect();
        (0..scores.len())
            .max_by(|a, b| scores[*a].total_cmp(&scores[*b]))
            .unwrap_or(0)
    }
}

/// Play the deterministic ai and write down what it thought of every pill it was dealt.
///
/// The teacher is the **strongest** of the N64's six rows of weights, which is
/// `n64::DEFAULT_SKILL` and so [`SKILL_ORDER`]'s last entry. The rows are personalities
/// rather than a ladder and the order between them is measured, so which row this is moves when
/// that measurement is run again, and it is sensitive to how a row is scored: ranked on bottles
/// less four per burial it is row 1, and ranked the way the fitness ranks a model - viruses
/// inside a pill budget - it is row 4, which clears 18% more of them while burying itself twice
/// as often. Teaching from anything but the strongest would put a ceiling under the student
/// that nothing later in training could lift, since the run only ever mutates about where the
/// teaching left it.
pub fn lessons(pills: usize) -> Vec<Lesson> {
    let ai = N64Ai::with_skill(SKILL_ORDER[SKILLS - 1]);
    let mut lessons: Vec<Lesson> = vec![];

    for (game, level) in LESSON_LEVELS.iter().cycle().enumerate() {
        if lessons.len() >= pills {
            break;
        }
        let random = GameRandom::from_seed(Seed::from(game as u128 + 1).into(), RandomMode::Bag);
        let Ok(mut game) = Game::new(*level, GameSpeed::Medium, random) else {
            continue;
        };

        let mut stalled = 0;
        let mut game_over = false;
        let mut viruses = game.bottle().virus_count();
        while !game_over && stalled < STALL_PILLS && lessons.len() < pills {
            let now = game.bottle().virus_count();
            if now < viruses {
                stalled = 0;
            }
            viruses = now;

            if game.bottle().pill().is_none() {
                game.update(STEP);
                game_over |= advance(&mut game, &mut stalled, &mut viruses);
                continue;
            }

            let before = game.bottle().stats();
            let placements = game.bottle().placements(before);
            let reading = ai.read(game.bottle(), &placements);
            let Some(chosen) = reading.chosen else { break };
            let priorities = reading.priorities;
            let own = placements.len();

            if let Some(mut lesson) = Lesson::of(&placements, &priorities, own) {
                lesson.held_out = lessons.len() % HELD_OUT == HELD_OUT - 1;
                lessons.push(lesson);
            }

            press(&mut game, &placements[chosen]);
            stalled += 1;
            game.update(STEP);
            game_over |= advance(&mut game, &mut stalled, &mut viruses);
        }
    }

    lessons
}

/// Teach a network from scratch to rank placements the way the deterministic ai ranks them.
pub fn teach(lessons: &[Lesson], epochs: usize, seed: u64) -> DrNeuralNetwork {
    let mut rng = ChaChaRng::seed_from_u64(seed);
    let weights: Vec<f64> = (0..DR_NEURAL_GENOME_SIZE)
        .map(|_| rng.random::<f64>() * 2.0 - 1.0)
        .collect();
    let mut network = DrNeuralNetwork::new(&weights);

    let taught: Vec<usize> = (0..lessons.len())
        .filter(|i| !lessons[*i].held_out)
        .collect();
    let mut order = taught.clone();
    for epoch in 0..epochs {
        let rate = LEARNING_RATE / (1.0 + LEARNING_DECAY * epoch as f64);
        // the corpus is one game after another, so a pass straight down it would spend its last
        // steps on whatever the last game happened to look like
        shuffle(&mut order, &mut rng);
        for lesson in order.iter().map(|i| &lessons[*i]) {
            for (row, target) in lesson.target.iter().enumerate() {
                let Some(target) = target else { continue };
                network.train_step(
                    &lesson.input(row),
                    &Tensor::vector([f64::from(*target)]),
                    rate,
                );
            }
        }
    }

    // Every lesson is a placement of the pill in play, so the input that says "this is the
    // pill you are holding instead" is zero on all of them - and a weight the corpus gives no
    // gradient for keeps whatever the initial draw left there. Left alone, a taught network
    // comes out with a strong opinion about swapping that it learned from nothing at all,
    // which is the whole of why hold used to be a disaster here. Silenced, it is indifferent
    // to a swap, and the genetic algorithm is free to decide what one is worth.
    network.silence_input(HELD_INPUT);
    network
}

/// which input says the candidate is a placement of the held pill, in
/// [`evaluator::raw_inputs`]'s order: the last of them
const HELD_INPUT: usize = BOTTLE_FEATURE_INPUTS - 1;

/// How well `network` reproduces the lessons it was held back from.
pub fn measure(lessons: &[Lesson], network: &DrNeuralNetwork) -> Report {
    let held: Vec<&Lesson> = lessons.iter().filter(|lesson| lesson.held_out).collect();
    let mut agreed = 0.0;
    let mut holds = 0.0;
    for lesson in &held {
        let best = lesson.choice(network);
        if lesson.top[best] {
            agreed += 1.0;
        }
        if best >= lesson.own {
            holds += 1.0;
        }
    }
    let lessons = held.len().max(1) as f64;
    Report {
        agreement: agreed / lessons,
        holds: holds / lessons,
        lessons: held.len(),
    }
}

// ---------------------------------------------------------------------------------------
// playing a headless game, which both this and the probe need
// ---------------------------------------------------------------------------------------

/// how a run of whole games went
#[derive(Default)]
pub struct Played {
    pub viruses: u32,
    pub bottles: u32,
    pub pills: u32,
    pub buried: u32,
}

/// Play `seeds` whole games from the first bottle at `level`, choosing with `choose`, and count
/// what happened. A game is called off if it goes [`STALL_PILLS`] without destroying a virus.
pub fn play_games(
    seeds: u128,
    level: u32,
    mut choose: impl FnMut(&Bottle, &[Placement]) -> Option<usize>,
) -> Played {
    let mut played = Played::default();

    for seed in 0..seeds {
        let random = GameRandom::from_seed(Seed::from(seed + 1).into(), RandomMode::Bag);
        let Ok(mut game) = Game::new(level, GameSpeed::Medium, random) else {
            continue;
        };
        let mut stalled = 0;
        let mut game_over = false;
        let mut viruses = game.bottle().virus_count();

        while !game_over && stalled < STALL_PILLS {
            let now = game.bottle().virus_count();
            if now < viruses {
                played.viruses += viruses - now;
                stalled = 0;
            }
            viruses = now;

            if game.bottle().pill().is_none() {
                game.update(STEP);
                game_over |= advance_counting(&mut game, &mut stalled, &mut viruses, &mut played);
                continue;
            }

            let before = game.bottle().stats();
            let placements = game.bottle().placements(before);
            let Some(chosen) = choose(game.bottle(), &placements) else {
                break;
            };
            press(&mut game, &placements[chosen]);
            played.pills += 1;
            stalled += 1;

            game.update(STEP);
            game_over |= advance_counting(&mut game, &mut stalled, &mut viruses, &mut played);
        }
        played.buried += game_over as u32;
    }
    played
}

/// the placement the hand written baseline likes best, which is what a trained model is
/// measured against
pub fn linear_choice(placements: &[Placement]) -> Option<usize> {
    let features: Vec<_> = placements.iter().map(|p| p.features()).collect();
    let scores = Scorer::Linear.rank(&features);
    (0..placements.len()).max_by(|a, b| scores[*a].total_cmp(&scores[*b]))
}

/// Press the keys that reach `placement`, all in one frame.
///
/// A [`Translation::Rest`] is the one that is not a key: in play the agent waits for gravity to
/// bring the pill down and then walks it sideways in the lock delay, and here the fall is asked
/// for outright, which leaves the bottle in exactly the state the search predicted.
pub(crate) fn press(game: &mut Game, placement: &Placement) {
    for translation in placement.inputs().translations() {
        match translation {
            Translation::Left => game.left(),
            Translation::Right => game.right(),
            Translation::RotateClockwise => game.rotate(true),
            Translation::RotateAnticlockwise => game.rotate(false),
            Translation::HardDrop => game.hard_drop(),
            Translation::Rest => game.rest(),
            Translation::Hold => game.hold(),
        }
    }
}

/// drain what the game has to say and move it on to the next bottle. Returns whether it is over.
fn advance(game: &mut Game, stalled: &mut u32, viruses: &mut u32) -> bool {
    let mut over = false;
    for event in game.drain_events() {
        match event {
            GameEvent::GameOver => over = true,
            GameEvent::StageComplete => {
                *stalled = 0;
                *viruses = 0;
                if game.next_stage().is_err() {
                    over = true;
                }
            }
            _ => (),
        }
    }
    over
}

fn advance_counting(
    game: &mut Game,
    stalled: &mut u32,
    viruses: &mut u32,
    played: &mut Played,
) -> bool {
    let mut over = false;
    for event in game.drain_events() {
        match event {
            GameEvent::GameOver => over = true,
            GameEvent::StageComplete => {
                *stalled = 0;
                *viruses = 0;
                played.bottles += 1;
                if game.next_stage().is_err() {
                    over = true;
                }
            }
            _ => (),
        }
    }
    over
}

fn shuffle(order: &mut [usize], rng: &mut ChaChaRng) {
    for i in (1..order.len()).rev() {
        let j = (rng.random::<f64>() * (i + 1) as f64) as usize;
        order.swap(i, j.min(i));
    }
}
