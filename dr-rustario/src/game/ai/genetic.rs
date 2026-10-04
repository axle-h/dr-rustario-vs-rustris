//! Dr. Rustario's genetic training and the `ga dr` entry points.
//!
//! `ga dr auto` runs two stages: imitation ([`crate::game::ai::imitation`]) teaches a network
//! to rank placements like the deterministic ai, because a genetic algorithm cannot select among
//! members that all score zero; then the run scores candidates on [`crate::game::ai::run::merit`]: games from
//! each of [`START_LEVELS`] under the garbage the n64 port throws, the viruses destroyed first
//! and the garbage sent second.

use crate::game::ai::agent::{Choices, DrAiAgent, Hold};
use crate::game::ai::duel::{duels, second_wins, Brain, Rules};
use crate::game::ai::headless_game::{
    summarise, HeadlessGameFixture, HeadlessGameOptions, Played, VIRUSES_TO_CLEAR,
};
use crate::game::ai::imitation;
use crate::game::ai::models::{self, DrNeuralGenome, DrNeuralNetwork, DR_NEURAL_GENOME_SIZE};
use crate::game::ai::run::{
    survived_the_budget, Fire, Incoming, Traffic, COMBO_DISCOUNT, GAME_PILLS, GARBAGE_PRICE,
    N64_FIRE, PILL_BUDGET, PROVEN_LEVEL, SEEDS_PER_LEVEL, START_LEVELS, TOP_TRAINING_LEVEL,
};
use crate::game::random::RandomMode;
use engine::ai::{
    EndGame, Fitness, GameResult, GeneticAlgorithm, Genome, GenomeMutation, HyperParameters,
    Objective, Phase, RateLimits, Rung, Seed,
};
use rayon::prelude::*;
use std::ops::Range;

/// whole games a finished model is reported on; they decide nothing
const UNSEEN_SEEDS: u128 = 5;

/// far from the block training walks through, so reports are on bottles never trained against
pub(super) const UNSEEN_SEED_BLOCK: u128 = 1 << 96;

/// small, so each candidate can afford [`SEEDS_PER_LEVEL`] games
const POPULATION: usize = 200;

/// Games the leaders of a generation play. A seed moves a model's score a long way, so
/// selection past a well-taught seed picks luck unless there are many; raising
/// [`SEEDS_PER_LEVEL`] is the lever.
const TRAINING_GAMES: usize = START_LEVELS.len() * SEEDS_PER_LEVEL;

/// How a generation is raced: everyone plays a sixth of the block, the better half half of it,
/// and the best eighth, where elites and most parents come from, all of it. Every rung is a
/// whole number of games per start level, so each plays the levels alike.
const RACING: [Rung; 3] = [
    Rung {
        games: START_LEVELS.len() * (SEEDS_PER_LEVEL / 6),
        share: 1.0,
    },
    Rung {
        games: START_LEVELS.len() * (SEEDS_PER_LEVEL / 2),
        share: 0.5,
    },
    Rung {
        games: TRAINING_GAMES,
        share: 0.125,
    },
];

/// Where checkpoints and the playoff duel the n64, apart from [`COMPARE_SEED_BLOCK`], so a run
/// is still judged afterwards on duels it never picked on.
const DUEL_SEED_BLOCK: u128 = 1 << 108;

/// duels from each start level a checkpoint or playoff plays under each rule
const DUELS_PER_LEVEL: usize = 400;

/// a file of this name in the working directory stops a run after its next generation
const STOP_FILE: &str = "STOP";

/// Training games from each start level a checkpoint or playoff is judged on, all unseen. Merit's
/// standard error is about 130 over the square root of the games, so fewer cannot tell models
/// apart; they take seconds.
pub(super) const UNSEEN_PER_LEVEL: usize = 200;

/// where `ga dr garbage` plays, apart from training and the unseen block
const MEASURE_SEED_BLOCK: u128 = 1 << 100;

/// where `ga dr compare` plays, apart from every other block, so no model was picked on it
pub(super) const COMPARE_SEED_BLOCK: u128 = 1 << 104;

/// the held-pill input, last in [`crate::game::ai::evaluator::raw_inputs`]'s order
const HELD_INPUT: usize = engine::ai::BOTTLE_FEATURE_INPUTS - 1;

/// Fraction of a generation carried over unchanged. Every generation plays fresh seeds, elites
/// included, so the engine's default of one elite in 250 loses good genomes to one bad block.
const ELITE_RATE: f64 = 0.04;

/// How often an uncapped run prints its best genome as paste-ready [`models`] weights, with its
/// play on unseen seeds, so a run stopped by hand leaves a usable model in its log.
const CHECKPOINT_GENERATIONS: usize = 25;

/// How many of the final population are played off on one fixed unseen block before embedding,
/// since a generation's own ordering is only its ordering on that generation's seeds.
const PLAYOFF: usize = 8;

/// fewer pills than a real run, since a trial checks shape rather than producing a model
const TRIAL_LESSON_PILLS: usize = 3000;

/// Viruses over [`UNSEEN_SEEDS`] games a taught network must play before stage two starts from
/// it. It has to sit above the median `ga dr screen` reports or stage one stops on its first
/// clone; re-measure it whenever the feature set changes.
const TAUGHT_ENOUGH: u32 = 4130;

/// how many networks stage one will teach before it settles for the best of a bad lot
const PRETRAIN_ATTEMPTS: u64 = 25;

struct NeuralFitness {
    fixture: HeadlessGameFixture,
}

impl Fitness<DR_NEURAL_GENOME_SIZE> for NeuralFitness {
    fn evaluate(&self, genome: &Genome<DR_NEURAL_GENOME_SIZE>) -> GameResult {
        self.fixture.play((*genome).into())
    }

    fn evaluate_games(
        &self,
        genome: &Genome<DR_NEURAL_GENOME_SIZE>,
        games: Range<usize>,
    ) -> Vec<GameResult> {
        self.fixture.play_games((*genome).into(), games)
    }

    fn summarise(&self, games: &[GameResult]) -> GameResult {
        summarise(games)
    }

    /// the stop file is taken as it is read, so the next run in the same place is not stopped
    fn stop_requested(&self) -> bool {
        std::fs::remove_file(STOP_FILE).is_ok()
    }

    fn next_seed(&mut self) {
        self.fixture.next_seed();
    }

    fn current_seed(&self) -> Seed {
        self.fixture.current_seed()
    }

    fn seeds_per_game(&self) -> usize {
        self.fixture.seeds_per_game()
    }

    fn set_seeds_per_game(&mut self, seeds_per_game: usize) {
        self.fixture.set_seeds_per_game(seeds_per_game);
    }

    fn set_end_game(&mut self, end_game: EndGame) {
        self.fixture.set_end_game(end_game);
    }

    /// Every [`CHECKPOINT_GENERATIONS`] generations, report the best genome on unseen seeds, as
    /// trained, as solo pace and against the n64, and print and save its weights.
    fn checkpoint(&self, generation: usize, genome: &Genome<DR_NEURAL_GENOME_SIZE>) {
        if generation == 0 || !generation.is_multiple_of(CHECKPOINT_GENERATIONS) {
            return;
        }
        let what = format!("generation {}", generation);
        println!();
        report_training(&what, network_agent(*genome));
        report_play(&what, *genome);
        report_duels(&what, *genome);
        print_weights(&what, without_a_hold_opinion(*genome));
        save_weights(
            &format!("gen-{}.weights", generation),
            &what,
            without_a_hold_opinion(*genome),
        );
    }
}

fn neural_fitness() -> NeuralFitness {
    neural_fitness_with(Hold::Off)
}

/// the same, with the held pill on offer or not
fn neural_fitness_with(hold: Hold) -> NeuralFitness {
    NeuralFitness {
        fixture: HeadlessGameFixture::new(
            RandomMode::Bag,
            rand::random(),
            HeadlessGameOptions {
                hold,
                incoming: Some(N64_FIRE),
                ..HeadlessGameOptions::default()
            },
            EndGame::NONE,
        )
        .with_levels(&START_LEVELS),
    }
}

/// Training's games on `block`: [`GAME_PILLS`] from each start level in turn, under `fire`.
pub(super) fn training_fixture(
    block: Seed,
    games: usize,
    fire: Option<Fire>,
) -> HeadlessGameFixture {
    let mut fixture = HeadlessGameFixture::new(
        RandomMode::Bag,
        block,
        HeadlessGameOptions {
            incoming: fire,
            ..HeadlessGameOptions::default()
        },
        EndGame::of_pieces(GAME_PILLS),
    )
    .with_levels(&START_LEVELS);
    fixture.set_seeds_per_game(games);
    fixture
}

/// a genome as training plays it
pub(super) fn network_agent(genome: DrNeuralGenome) -> impl Fn() -> DrAiAgent + Sync {
    let network: DrNeuralNetwork = genome.into();
    move || DrAiAgent::new(network)
}

/// what [`training_fixture`] plays on the unseen block, the same for every genome and the n64
fn unseen_training(agent: impl Fn() -> DrAiAgent + Sync) -> Vec<Played> {
    let block = Seed::from(UNSEEN_SEED_BLOCK);
    training_fixture(block, START_LEVELS.len() * UNSEEN_PER_LEVEL, Some(N64_FIRE))
        .games(agent, block)
}

/// Report an agent on the training games of the unseen block: its merit and what it is made of,
/// with the guard rails beside it, the choices it made and its pace with the garbage left out.
pub(super) fn report_training(what: &str, agent: impl Fn() -> DrAiAgent + Sync) -> f64 {
    let played = unseen_training(agent);
    let merit: f64 = played.iter().map(|p| p.result.merit()).sum::<f64>() / played.len() as f64;
    // the standard error of that mean, which is how far apart two models must be to differ
    let spread = (played
        .iter()
        .map(|p| (p.result.merit() - merit).powi(2))
        .sum::<f64>()
        / (played.len().max(2) - 1) as f64
        / played.len() as f64)
        .sqrt();
    let viruses: u32 = played.iter().map(|p| p.result.cleared()).sum();
    let pills: u32 = played.iter().map(|p| p.result.pieces()).sum();
    let buried = played.iter().filter(|p| p.result.game_over()).count();
    let stalled = played.iter().filter(|p| p.stalled).count();
    let traffic: Traffic = played.iter().map(|p| p.traffic).sum();
    let choices: Choices = played.iter().map(|p| p.choices).sum();
    println!(
        "{}: merit {:.1} ± {:.1} a game over {} games, {} buried ({} of them called off); {:.3} \
         viruses a pill; sent {} blocks in {} combos ({:.3} a pill), received {}",
        what,
        merit,
        spread,
        played.len(),
        buried,
        stalled,
        viruses as f64 / pills.max(1) as f64,
        traffic.blocks_sent,
        traffic.attacks,
        traffic.blocks_sent as f64 / pills.max(1) as f64,
        traffic.blocks_received
    );
    for level in START_LEVELS {
        let at: Vec<&Played> = played.iter().filter(|p| p.level == level).collect();
        let viruses: u32 = at.iter().map(|p| p.result.cleared()).sum();
        let pills: u32 = at.iter().map(|p| p.result.pieces()).sum();
        let sent: u32 = at.iter().map(|p| p.traffic.blocks_sent).sum();
        println!(
            "  from level {:2}: {:.3} viruses a pill, {} bottles, {} blocks sent, {} of {} buried, \
             {} called off",
            level,
            viruses as f64 / pills.max(1) as f64,
            at.iter().map(|p| p.result.bonus()).sum::<u32>(),
            sent,
            at.iter().filter(|p| p.result.game_over()).count(),
            at.len(),
            at.iter().filter(|p| p.stalled).count()
        );
    }
    println!("  {}", choices);
    merit
}

fn neural_mutation() -> GenomeMutation<DR_NEURAL_GENOME_SIZE> {
    GenomeMutation::of_max(
        RateLimits::new(0.1..=0.20),
        RateLimits::new(0.1..=0.20),
        5,
        rand::random(),
    )
}

/// The run: [`crate::game::ai::run::merit`] over games from every start level under fire. A seeded population is
/// mutated far more gently than a random one, since the wide rates destroy most of what a taught
/// model already does.
fn versus_phase(seeded: bool) -> Phase {
    versus_phase_shaken(seeded, 1.0)
}

/// The same phase with mutation and crossover scaled by `shake`. A median that falls while the
/// best stands still means mutation is too strong for the seed.
fn versus_phase_shaken(seeded: bool, shake: f64) -> Phase {
    let (rates, step) = if seeded {
        (0.009 * shake..=0.024 * shake, 0.015 * shake)
    } else {
        (0.1 * shake..=0.20 * shake, 0.1 * shake)
    };
    Phase {
        objective: Objective::Merit,
        end_game: EndGame::of_pieces(GAME_PILLS),
        seeds_per_game: TRAINING_GAMES,
        mutation_rate: RateLimits::new(rates.clone()),
        crossover_rate: RateLimits::new(rates),
        mutation_step: step,
        // uncapped: a run is stopped by hand or by the stop file, and checkpoints keep what it found
        max_generations: usize::MAX,
        racing: vec![],
    }
    .with_racing(RACING.to_vec())
}

/// a report's clock and ceiling: [`PILL_BUDGET`] pills to destroy every virus from the first bottle
fn budgeted_end_game() -> EndGame {
    EndGame {
        pieces: PILL_BUDGET,
        ..EndGame::of_cleared(VIRUSES_TO_CLEAR)
    }
}

/// Run a phase and return the winner of a [`PLAYOFF`] among the top of the final population.
fn run(phase: Phase, population_seed: Option<DrNeuralGenome>) -> DrNeuralGenome {
    println!(
        "games of {} pills from levels {:?}, garbage at {:.2} viruses a block times {}; {}",
        GAME_PILLS,
        START_LEVELS,
        GARBAGE_PRICE,
        COMBO_DISCOUNT,
        racing_settings(&phase)
    );
    println!(
        "touch {} in the working directory to stop after the next generation",
        STOP_FILE
    );
    report_training("n64 top row", DrAiAgent::n64);
    if let Some(seed) = population_seed {
        report_training("seed model", network_agent(seed));
    }
    let mut algorithm = GeneticAlgorithm::new(
        neural_fitness(),
        neural_mutation(),
        HyperParameters::new(POPULATION, ELITE_RATE, 0.5),
        vec![phase],
        population_seed,
    );
    let stats = algorithm.run();
    playoff(&finalists(&algorithm, population_seed)).unwrap_or_else(|| stats.max().genome())
}

/// The top [`PLAYOFF`] of the final population, and the seed last, so a run can never hand back
/// a model worse than the one it started from.
fn finalists<F: Fitness<DR_NEURAL_GENOME_SIZE>>(
    algorithm: &GeneticAlgorithm<DR_NEURAL_GENOME_SIZE, F>,
    population_seed: Option<DrNeuralGenome>,
) -> Vec<DrNeuralGenome> {
    algorithm
        .population()
        .iter()
        .take(PLAYOFF)
        .map(|organism| organism.genome())
        .chain(population_seed)
        .collect()
}

/// Play the finalists over the same unseen training games and the same duels with the n64, and
/// return the one that wins most marathons. Training never meets an opponent, so its merit can
/// rise while it fights worse; merit is reported beside.
fn playoff(finalists: &[DrNeuralGenome]) -> Option<DrNeuralGenome> {
    if finalists.is_empty() {
        return None;
    }
    println!(
        "\nplayoff: {} finalists over {} unseen training games and {} duels with the n64 under \
         each rule, the seed last if there was one",
        finalists.len(),
        START_LEVELS.len() * UNSEEN_PER_LEVEL,
        START_LEVELS.len() * DUELS_PER_LEVEL
    );
    let mut ranked: Vec<(f64, f64, usize, DrNeuralGenome)> = finalists
        .iter()
        .enumerate()
        .map(|(seat, genome)| {
            let what = format!("  seat {}", seat + 1);
            let merit = report_training(&what, network_agent(*genome));
            let marathons = report_duels(&what, *genome);
            (marathons, merit, seat, *genome)
        })
        .collect();
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.total_cmp(&a.1)));
    if let Some((marathons, merit, seat, _)) = ranked.first() {
        println!(
            "the playoff goes to seat {}, winning {:.1}% of decided marathons at merit {:.1}",
            seat + 1,
            100.0 * marathons,
            merit
        );
    }
    ranked.first().map(|(_, _, _, genome)| *genome)
}

/// Duel `genome` against the n64's top row on [`DUEL_SEED_BLOCK`] under both rules, and return
/// its share of the decided marathons.
fn report_duels(what: &str, genome: DrNeuralGenome) -> f64 {
    let brains = [Brain::N64, Brain::Network(genome.into())];
    let block = Seed::from(DUEL_SEED_BLOCK);
    let games = START_LEVELS.len() * DUELS_PER_LEVEL;
    let (marathons, marathon_spread) = second_wins(&duels(brains, Rules::Marathon, block, games));
    let (sprints, sprint_spread) = second_wins(&duels(brains, Rules::Sprint, block, games));
    println!(
        "{} against the n64 over {} duels each: wins {:.1}% ± {:.1}% of decided marathons, \
         {:.1}% ± {:.1}% of decided sprints",
        what,
        games,
        100.0 * marathons,
        100.0 * marathon_spread,
        100.0 * sprints,
        100.0 * sprint_spread
    );
    marathons
}

/// how many games a candidate plays, racing or not
fn racing_settings(phase: &Phase) -> String {
    if phase.racing.is_empty() {
        return format!("each candidate plays {} games", phase.seeds_per_game);
    }
    let rungs: Vec<String> = phase
        .racing
        .iter()
        .enumerate()
        .map(|(index, rung)| {
            if index == 0 {
                format!("everyone plays {} games", rung.games)
            } else {
                format!("the best {:.1}% play to {}", 100.0 * rung.share, rung.games)
            }
        })
        .collect();
    format!("raced: {}", rungs.join(", "))
}

/// Report `genome` on unseen seeds and whether it survived the budget, as the fitness asks.
fn report_unseen(genome: DrNeuralGenome) -> bool {
    let (results, choices) = unseen_play(genome);
    for (seed, result) in results.iter().enumerate() {
        println!(
            "  seed {}: {} bottles, {} viruses, {} pills, {}",
            seed + 1,
            result.bonus(),
            result.cleared(),
            result.pieces(),
            if result.game_over() {
                "buried"
            } else {
                "still standing"
            }
        );
    }
    let proven = results
        .iter()
        .filter(|result| result.bonus() > PROVEN_LEVEL)
        .count();
    println!(
        "  {} of {} seeds reached bottle {} inside {} pills",
        proven, UNSEEN_SEEDS, PROVEN_LEVEL, PILL_BUDGET
    );
    println!("  {}", choices);
    survived_the_budget(&results)
}

/// The run, played for `generations` or until stopped.
fn survive(population_seed: Option<DrNeuralGenome>, generations: usize) -> DrNeuralGenome {
    without_a_hold_opinion(run(
        versus_phase(population_seed.is_some()).with_max_generations(generations),
        population_seed,
    ))
}

/// `ga dr screen [pills] [clones] [silenced] [epochs]`: teach every clone in parallel from one
/// corpus and report the median, since best-of-N misorders feature sets.
///
/// `silenced` is a comma list of inputs zeroed throughout training ([`imitation::teach_without`]),
/// in `evaluator::raw_inputs`'s order as `ga dr explain` lists it.
pub fn ga_main_screen(args: &[String]) -> Result<(), String> {
    let pills: usize = args
        .first()
        .and_then(|s| s.parse().ok())
        .unwrap_or(imitation::LESSON_PILLS);
    let clones: u64 = args
        .get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(PRETRAIN_ATTEMPTS);
    let epochs: usize = args
        .get(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(imitation::EPOCHS);
    let silenced: Vec<usize> = match args.get(2) {
        None => vec![],
        Some(list) => list
            .split(',')
            .filter(|piece| !piece.trim().is_empty())
            .map(|piece| {
                piece
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| format!("'{}' is not an input index", piece.trim()))
                    .and_then(|input| match input < engine::ai::BOTTLE_FEATURE_INPUTS {
                        true => Ok(input),
                        false => Err(format!(
                            "input {} does not exist; there are {}",
                            input,
                            engine::ai::BOTTLE_FEATURE_INPUTS
                        )),
                    })
            })
            .collect::<Result<Vec<usize>, String>>()?,
    };

    println!(
        "screening {} of {} inputs, {} clones over {} pills, {} epochs",
        engine::ai::BOTTLE_FEATURE_INPUTS - silenced.len(),
        engine::ai::BOTTLE_FEATURE_INPUTS,
        clones,
        pills,
        epochs
    );
    for input in &silenced {
        println!(
            "  silenced: {:2} {}",
            input,
            super::explain::INPUTS[*input].name
        );
    }

    let corpus = imitation::lessons(pills);
    let mut rows: Vec<(u64, f64, u32, u32, f64)> = (0..clones)
        .into_par_iter()
        .map(|clone| {
            let network = imitation::teach_without(&corpus, epochs, clone, &silenced);
            let report = imitation::measure(&corpus, &network);
            let (results, choices) = unseen_play(network.into());
            (
                clone,
                report.agreement,
                results.iter().map(|r| r.cleared()).sum(),
                results.iter().map(|r| r.bonus()).sum(),
                choices.tidy_rate(),
            )
        })
        .collect();
    rows.sort_by_key(|(clone, _, _, _, _)| *clone);

    for (clone, agreement, viruses, bottles, tidy) in &rows {
        println!(
            "  clone {:2}: agreement {:.1}%, {} viruses, {} bottles, vitamins over a kill {:.1}%",
            clone + 1,
            100.0 * agreement,
            viruses,
            bottles,
            100.0 * tidy
        );
    }

    let mut viruses: Vec<u32> = rows.iter().map(|(_, _, v, _, _)| *v).collect();
    viruses.sort_unstable();
    let mut agreement: Vec<f64> = rows.iter().map(|(_, a, _, _, _)| *a).collect();
    agreement.sort_by(f64::total_cmp);
    let mut tidy: Vec<f64> = rows.iter().map(|(_, _, _, _, t)| *t).collect();
    tidy.sort_by(f64::total_cmp);
    let cleared = viruses.iter().filter(|v| **v >= TAUGHT_ENOUGH).count();
    println!(
        "median {} viruses, mean {}, best {}, {} of {} cleared {}; median agreement {:.1}%, \
         median vitamins over a kill {:.1}%",
        median(&viruses),
        viruses.iter().sum::<u32>() / viruses.len().max(1) as u32,
        viruses.last().copied().unwrap_or(0),
        cleared,
        viruses.len(),
        TAUGHT_ENOUGH,
        100.0 * agreement[agreement.len() / 2],
        100.0 * tidy[tidy.len() / 2]
    );
    Ok(())
}

/// the middle of a sorted slice, averaging the two middles of an even one
fn median(sorted: &[u32]) -> u32 {
    match sorted.len() {
        0 => 0,
        n if n % 2 == 1 => sorted[n / 2],
        n => (sorted[n / 2 - 1] + sorted[n / 2]) / 2,
    }
}

/// `ga dr pretrain [pills] [threshold]`: stage one on its own. Teaches networks from the
/// deterministic ai until one plays `threshold` viruses over the verification games, reports
/// how well it learned and how it plays, and prints the weights.
pub fn ga_main_pretrain(args: &[String]) -> Result<(), String> {
    let pills: usize = args
        .first()
        .and_then(|s| s.parse().ok())
        .unwrap_or(imitation::LESSON_PILLS);
    let threshold: u32 = args
        .get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(TAUGHT_ENOUGH);
    let genome = pretrain(pills, threshold);
    print_weights("taught model", genome);
    Ok(())
}

/// Stage one: gather the deterministic corpus once and teach fresh initial weights until one
/// network plays `threshold` viruses, since the start weights move play far more than agreement.
fn pretrain(pills: usize, threshold: u32) -> DrNeuralGenome {
    println!("lessons: {} pills from the n64 ai", pills);
    let corpus = imitation::lessons(pills);
    println!(
        "training: {} epochs over {} pills, target {} viruses in {} games, {} attempts",
        imitation::EPOCHS,
        corpus.len(),
        threshold,
        UNSEEN_SEEDS,
        PRETRAIN_ATTEMPTS
    );

    let mut best: Option<(u32, DrNeuralGenome)> = None;
    for clone in 0..PRETRAIN_ATTEMPTS {
        let network = imitation::teach(&corpus, imitation::EPOCHS, clone);
        let report = imitation::measure(&corpus, &network);
        let genome: DrNeuralGenome = network.into();
        let results = unseen_results(genome);
        let viruses: u32 = results.iter().map(|r| r.cleared()).sum();
        let bottles: u32 = results.iter().map(|r| r.bonus()).sum();
        println!(
            "  attempt {}: agreement {:.1}% of {} held out pills, {} viruses, {} bottles",
            clone + 1,
            100.0 * report.agreement,
            report.lessons,
            viruses,
            bottles
        );
        if best.is_none_or(|(most, _)| viruses > most) {
            best = Some((viruses, genome));
        }
        if viruses >= threshold {
            break;
        }
    }

    let (viruses, genome) = best.expect("no clone was taught");
    if viruses < threshold {
        println!(
            "no attempt reached {} viruses in {}; taking the best at {}",
            threshold, PRETRAIN_ATTEMPTS, viruses
        );
    }
    report_play("taught model", genome);
    genome
}

/// Silence the held-pill input. Training plays with [`crate::game::ai::agent::Hold::Off`], so
/// its weights drift untested and would otherwise give a random opinion on every swap.
fn without_a_hold_opinion(genome: DrNeuralGenome) -> DrNeuralGenome {
    let mut network: DrNeuralNetwork = genome.into();
    network.silence_input(HELD_INPUT);
    network.into()
}

/// Print a genome as the body of `models::survival_trained`, ready to paste. A genome's own
/// `Display` is the algorithm's raw coefficients, not the network's weights.
fn print_weights(what: &str, genome: DrNeuralGenome) {
    print!("\n{}", weights_text(what, genome));
}

/// what [`print_weights`] prints, in a file [`load_weights`] reads back
fn save_weights(path: &str, what: &str, genome: DrNeuralGenome) {
    if let Err(e) = std::fs::write(path, weights_text(what, genome)) {
        println!("could not save {}: {}", path, e);
    }
}

fn weights_text(what: &str, genome: DrNeuralGenome) -> String {
    let weights: [f64; DR_NEURAL_GENOME_SIZE] = genome.into();
    let mut text = format!("// {}, for models::survival_trained\n", what);
    text.push_str("    DrNeuralNetwork::new(&[\n");
    for line in weights.chunks(8) {
        let numbers: Vec<String> = line.iter().map(|w| format!("{:.6}", w)).collect();
        text.push_str(&format!("        {},\n", numbers.join(", ")));
    }
    text.push_str("    ])\n");
    text
}

/// weights as [`print_weights`] prints them, or any file holding the numbers in order
pub fn load_weights(path: &str) -> Result<DrNeuralGenome, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path, e))?;
    let weights: Vec<f64> = text
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == 'e'))
        .filter(|token| token.contains('.'))
        .map(|token| {
            token
                .parse::<f64>()
                .map_err(|e| format!("{}: {}", token, e))
        })
        .collect::<Result<_, _>>()?;
    let weights: [f64; DR_NEURAL_GENOME_SIZE] = weights.try_into().map_err(|w: Vec<f64>| {
        format!(
            "{} holds {} weights, not {}",
            path,
            w.len(),
            DR_NEURAL_GENOME_SIZE
        )
    })?;
    Ok(DrNeuralNetwork::new(&weights).into())
}

/// Report whole games from the first bottle through the real agent, as the fitness scores them.
pub(super) fn report_play(what: &str, genome: DrNeuralGenome) {
    let (results, choices) = unseen_play(genome);
    let viruses: u32 = results.iter().map(|r| r.cleared()).sum();
    let bottles: u32 = results.iter().map(|r| r.bonus()).sum();
    let pills: u32 = results.iter().map(|r| r.pieces()).sum();
    let buried = results.iter().filter(|r| r.game_over()).count();
    println!(
        "{}: {} viruses, {} bottles, {} pills, {} buried, over {} games",
        what, viruses, bottles, pills, buried, UNSEEN_SEEDS
    );
    println!("  {}", choices);
}

/// the unseen block, under the same budget and caps as training
fn unseen_fixture(seeds_per_game: usize) -> HeadlessGameFixture {
    let mut fixture = HeadlessGameFixture::new(
        RandomMode::Bag,
        Seed::from(UNSEEN_SEED_BLOCK),
        HeadlessGameOptions::default(),
        budgeted_end_game(),
    );
    fixture.set_seeds_per_game(seeds_per_game);
    fixture
}

/// play `genome` on seeds it has never trained against, one whole game each
fn unseen_results(genome: DrNeuralGenome) -> Vec<GameResult> {
    unseen_play(genome).0
}

/// the same games, with every choice made over them
fn unseen_play(genome: DrNeuralGenome) -> (Vec<GameResult>, Choices) {
    let played = unseen_fixture(UNSEEN_SEEDS as usize)
        .games(network_agent(genome), Seed::from(UNSEEN_SEED_BLOCK));
    let choices = played.iter().map(|p| p.choices).sum();
    (played.into_iter().map(|p| p.result).collect(), choices)
}

/// Both stages in order.
pub fn ga_main_auto() -> Result<(), String> {
    println!("== stage 1: imitation ==");
    let taught = pretrain(imitation::LESSON_PILLS, TAUGHT_ENOUGH);

    println!("\n== stage 2: the run, {} pill budget ==", PILL_BUDGET);
    let trained = survive(Some(taught), usize::MAX);
    report_play("trained model", trained);

    println!("\nthe model to embed, on {} unseen seeds", UNSEEN_SEEDS);
    report_unseen(trained);
    print_weights("trained model", trained);
    Ok(())
}

/// `ga dr tune [weights|embedded] [generations]`: the same run, seeded from the built in model,
/// or a file of weights, rather than taught from scratch
pub fn ga_main_tune(args: &[String]) -> Result<(), String> {
    let seed: DrNeuralGenome = match args.first().map(String::as_str) {
        None | Some("embedded") => models::survival_trained().into(),
        Some(path) => load_weights(path)?,
    };
    let generations: usize = match args.get(1) {
        None => usize::MAX,
        Some(n) => n
            .parse()
            .map_err(|_| format!("{} is not a number of generations", n))?,
    };
    report_play("seed model", seed);
    let trained = survive(Some(seed), generations);
    report_play("trained model", trained);
    report_unseen(trained);
    print_weights("trained model", trained);
    Ok(())
}

/// `ga dr survive`: the run on its own, from random weights, with no imitation before it
pub fn ga_main_survive() -> Result<(), String> {
    let trained = survive(None, usize::MAX);
    report_play("trained model", trained);
    print_weights("trained model", trained);
    Ok(())
}

/// `ga dr trial [population] [generations] [seed] [shake] [games per level]`: a short bounded run that checks a
/// change still leaves the algorithm something to climb.
///
/// `seed` is `scratch` (random weights, the default), `taught` (a quick imitation), `tune` (the
/// embedded model), `hold` (taught, with the held pill on offer) or a file of weights; `shake`
/// scales the rates.
pub fn ga_main_trial(args: &[String]) -> Result<(), String> {
    let population: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(200);
    let generations: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
    let from = args.get(2).map(String::as_str).unwrap_or("scratch");

    let seed = match from {
        "scratch" => None,
        "taught" | "hold" => Some(pretrain(TRIAL_LESSON_PILLS, TAUGHT_ENOUGH)),
        "tune" => Some(models::survival_trained().into()),
        path => Some(load_weights(path)?),
    };
    let hold = if from == "hold" { Hold::On } else { Hold::Off };
    let shake: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let mut phase = versus_phase_shaken(seed.is_some(), shake);
    phase.max_generations = generations;
    // a count of games plays every candidate that many, unraced
    if let Some(per_level) = args.get(4).and_then(|s| s.parse::<usize>().ok()) {
        phase.racing.clear();
        phase.seeds_per_game = START_LEVELS.len() * per_level;
    }

    println!(
        "\npopulation {}, {} generations, {}, games of {} pills, hold {:?}, shaken x{}, seeded \
         from {}",
        population,
        generations,
        racing_settings(&phase),
        GAME_PILLS,
        hold,
        shake,
        match from {
            "tune" => "the embedded model",
            "scratch" => "random weights",
            "taught" | "hold" => "imitation",
            path => path,
        }
    );
    report_training("n64 top row", DrAiAgent::n64);
    if let Some(seed) = seed {
        report_training("seed model", network_agent(seed));
    }
    let mut algorithm = GeneticAlgorithm::new(
        neural_fitness_with(hold),
        neural_mutation(),
        HyperParameters::new(population, ELITE_RATE, 0.5),
        vec![phase],
        seed,
    );
    let stats = algorithm.run();

    let best = stats.max().result();
    println!(
        "best after {} generations: merit {:.1}, {} viruses, {} bottles, {} pills a game",
        generations,
        best.merit(),
        best.cleared(),
        best.bonus(),
        best.pieces()
    );
    report_training("best", network_agent(stats.max().genome()));
    if let Some(winner) = playoff(&finalists(&algorithm, seed)) {
        print_weights("playoff winner", winner);
    }
    Ok(())
}

/// play the built in model on a few seeds and report how far it gets
pub fn ga_diagnose() -> Result<(), String> {
    println!(
        "embedded model, bottles 0 to {} in {} pills, {} viruses in all",
        TOP_TRAINING_LEVEL, PILL_BUDGET, VIRUSES_TO_CLEAR
    );
    report_unseen(models::survival_trained().into());
    println!("\nthe training games of the unseen block, under fire");
    report_training("n64 top row", DrAiAgent::n64);
    report_training(
        "embedded model",
        network_agent(models::survival_trained().into()),
    );
    Ok(())
}

/// `ga dr garbage [seeds per level]`: the n64 port's strongest row as an opponent, which
/// [`N64_FIRE`] and [`GARBAGE_PRICE`] are set from. How often it sends garbage over
/// [`GAME_PILLS`] from each start level and how big, then what a block costs it to receive:
/// the same games again under that fire, the viruses it loses divided by the blocks it took.
pub fn ga_main_garbage(args: &[String]) -> Result<(), String> {
    let seeds: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(40);
    let games = seeds * START_LEVELS.len();
    let block = Seed::from(MEASURE_SEED_BLOCK);
    println!(
        "the n64 top row, {} games of {} pills from each of levels {:?}",
        seeds, GAME_PILLS, START_LEVELS
    );

    let quiet = training_fixture(block, games, None).games(DrAiAgent::n64, block);
    let mut fire = N64_FIRE;
    println!("\nwhat it sends");
    for (i, level) in START_LEVELS.iter().enumerate() {
        let at: Vec<&Played> = quiet.iter().filter(|p| p.level == *level).collect();
        let pills: u32 = at.iter().map(|p| p.result.pieces()).sum();
        let traffic: Traffic = at.iter().map(|p| p.traffic).sum();
        let attacks = traffic.attacks.max(1) as f64;
        fire.by_level[i] = Incoming {
            per_pill: traffic.attacks as f64 / pills.max(1) as f64,
            sizes: std::array::from_fn(|size| traffic.sizes[size] as f64 / attacks),
        };
        println!(
            "  level {:2}: {} attacks in {} pills ({:.4} a pill), {} blocks, sizes 2/3/4/5+ {:?}, \
             {} of {} buried",
            level,
            traffic.attacks,
            pills,
            fire.by_level[i].per_pill,
            traffic.blocks_sent,
            traffic.sizes,
            at.iter().filter(|p| p.result.game_over()).count(),
            at.len()
        );
    }

    let loud = training_fixture(block, games, Some(fire)).games(DrAiAgent::n64, block);
    println!("\nwhat receiving it costs, the same games under that fire");
    let (mut lost, mut received) = (0.0, 0u32);
    for level in START_LEVELS {
        let viruses = |played: &[Played]| -> u32 {
            played
                .iter()
                .filter(|p| p.level == level)
                .map(|p| p.result.cleared())
                .sum()
        };
        let buried = |played: &[Played]| {
            played
                .iter()
                .filter(|p| p.level == level && p.result.game_over())
                .count()
        };
        let took: u32 = loud
            .iter()
            .filter(|p| p.level == level)
            .map(|p| p.traffic.blocks_received)
            .sum();
        let gone = viruses(&quiet) as f64 - viruses(&loud) as f64;
        lost += gone;
        received += took;
        println!(
            "  level {:2}: {} viruses quiet, {} under fire, {} blocks received, {:.3} a block; \
             buried {} quiet, {} under fire",
            level,
            viruses(&quiet),
            viruses(&loud),
            took,
            gone / took.max(1) as f64,
            buried(&quiet),
            buried(&loud)
        );
    }
    println!(
        "\nGARBAGE_PRICE = {:.3} viruses a block over {} blocks",
        lost / received.max(1) as f64,
        received
    );
    println!("N64_FIRE by level:");
    for (level, incoming) in START_LEVELS.iter().zip(fire.by_level.iter()) {
        let sizes: Vec<String> = incoming.sizes.iter().map(|s| format!("{:.3}", s)).collect();
        println!(
            "  {:2}: per_pill {:.4}, sizes [{}]",
            level,
            incoming.per_pill,
            sizes.join(", ")
        );
    }
    Ok(())
}

pub fn ga_main() -> Result<(), String> {
    ga_main_auto()
}

/// `ga dr compare <games per level> <model>...`: every model over the same fresh training games,
/// each with its merit's paired difference from the first. A model is `n64`, `embedded` or a
/// file of weights.
pub fn ga_main_compare(args: &[String]) -> Result<(), String> {
    let per_level: usize = args
        .first()
        .and_then(|s| s.parse().ok())
        .ok_or("expected games per level, then the models")?;
    let block = Seed::from(COMPARE_SEED_BLOCK);
    let fixture = training_fixture(block, START_LEVELS.len() * per_level, Some(N64_FIRE));
    let mut first: Option<Vec<f64>> = None;
    for name in &args[1..] {
        let played = match name.as_str() {
            "n64" => fixture.games(DrAiAgent::n64, block),
            "embedded" => fixture.games(network_agent(models::survival_trained().into()), block),
            path => fixture.games(network_agent(load_weights(path)?), block),
        };
        let merits: Vec<f64> = played.iter().map(|p| p.result.merit()).collect();
        let viruses: u32 = played.iter().map(|p| p.result.cleared()).sum();
        let pills: u32 = played.iter().map(|p| p.result.pieces()).sum();
        let choices: Choices = played.iter().map(|p| p.choices).sum();
        let (merit, spread) = mean_and_error(&merits);
        print!(
            "{}: merit {:.1} ± {:.1}, {:.3} viruses a pill, {} of {} buried, vitamins over a kill \
             {:.1}%, finishes passed {} of {}",
            name,
            merit,
            spread,
            viruses as f64 / pills.max(1) as f64,
            played.iter().filter(|p| p.result.game_over()).count(),
            played.len(),
            100.0 * choices.tidy_rate(),
            choices.finishes_passed,
            choices.finishes_on_offer
        );
        match &first {
            None => first = Some(merits),
            Some(first) => {
                let differences: Vec<f64> = merits.iter().zip(first).map(|(a, b)| a - b).collect();
                let (difference, spread) = mean_and_error(&differences);
                print!("; {:+.1} ± {:.1} on the first", difference, spread);
            }
        }
        println!();
    }
    Ok(())
}

/// a sample's mean and the standard error of that mean
fn mean_and_error(sample: &[f64]) -> (f64, f64) {
    let n = sample.len().max(1) as f64;
    let mean = sample.iter().sum::<f64>() / n;
    let variance = sample.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0);
    (mean, (variance / n).sqrt())
}
