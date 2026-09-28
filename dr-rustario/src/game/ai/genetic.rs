//! Dr. Rustario's genetic training and the `ga dr` entry points.
//!
//! `ga dr auto` runs two stages: imitation ([`crate::game::ai::imitation`]) teaches a network
//! to rank placements like the deterministic ai, because a genetic algorithm cannot select among
//! members that all score zero; then the run scores candidates on the viruses they destroy from
//! the first bottle within `PILL_BUDGET` pills.

use crate::game::ai::agent::Hold;
use crate::game::ai::headless_game::{HeadlessGameFixture, HeadlessGameOptions, VIRUSES_TO_CLEAR};
use crate::game::ai::imitation;
use crate::game::ai::models::{self, DrNeuralGenome, DrNeuralNetwork, DR_NEURAL_GENOME_SIZE};
use crate::game::ai::run::{survived_the_budget, PILL_BUDGET, PROVEN_LEVEL, TOP_TRAINING_LEVEL};
use crate::game::random::RandomMode;
use engine::ai::{
    EndGame, Fitness, GameResult, GeneticAlgorithm, Genome, GenomeMutation, HyperParameters,
    Objective, Phase, RateLimits, Seed,
};
use rayon::prelude::*;

/// whole games a finished model is reported on; they decide nothing
const UNSEEN_SEEDS: u128 = 5;

/// far from the block training walks through, so reports are on bottles never trained against
const UNSEEN_SEED_BLOCK: u128 = 1 << 96;

const POPULATION: usize = 250;

/// Whole games each candidate plays per generation. A seed moves a model's score about 8%, so
/// four leave a fitness error near 4% and selection past a well-taught seed picks luck; raising
/// this is the lever, and [`crate::game::ai::run::PROBE_SEEDS`] claws back some of its cost.
const SEEDS_PER_GAME: usize = 4;

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
const TAUGHT_ENOUGH: u32 = 4000;

/// how many networks stage one will teach before it settles for the best of a bad lot
const PRETRAIN_ATTEMPTS: u64 = 25;

struct NeuralFitness {
    fixture: HeadlessGameFixture,
}

impl Fitness<DR_NEURAL_GENOME_SIZE> for NeuralFitness {
    fn evaluate(&self, genome: &Genome<DR_NEURAL_GENOME_SIZE>) -> GameResult {
        self.fixture.play((*genome).into())
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

    /// Every [`CHECKPOINT_GENERATIONS`] generations, report the best genome on unseen seeds and
    /// print its weights.
    fn checkpoint(&self, generation: usize, genome: &Genome<DR_NEURAL_GENOME_SIZE>) {
        if generation == 0 || !generation.is_multiple_of(CHECKPOINT_GENERATIONS) {
            return;
        }
        let what = format!("generation {}", generation);
        println!();
        report_play(&what, *genome);
        print_weights(&what, without_a_hold_opinion(*genome));
    }

    /// Re-asks the finish line (every seed survives its budget) on the unseen block, so a run
    /// does not end on a lucky generation.
    fn confirm(&self, genome: &Genome<DR_NEURAL_GENOME_SIZE>) -> bool {
        let fixture = unseen_fixture(self.fixture.seeds_per_game());
        survived_the_budget(&fixture.play_run((*genome).into(), Seed::from(UNSEEN_SEED_BLOCK)))
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
                ..HeadlessGameOptions::default()
            },
            EndGame::NONE,
        ),
    }
}

fn neural_mutation() -> GenomeMutation<DR_NEURAL_GENOME_SIZE> {
    GenomeMutation::of_max(
        RateLimits::new(0.1..=0.20),
        RateLimits::new(0.1..=0.20),
        5,
        rand::random(),
    )
}

/// The run: as many viruses as possible from the first bottle within [`PILL_BUDGET`] pills.
/// A seeded population is mutated far more gently than a random one, since the wide rates
/// destroy most of what a taught model already does.
fn clear_phase(seeded: bool) -> Phase {
    clear_phase_shaken(seeded, 1.0)
}

/// The same phase with mutation and crossover scaled by `shake`. A median that falls while the
/// best stands still means mutation is too strong for the seed.
fn clear_phase_shaken(seeded: bool, shake: f64) -> Phase {
    let (rates, step) = if seeded {
        (0.03 * shake..=0.08 * shake, 0.05 * shake)
    } else {
        (0.1 * shake..=0.20 * shake, 0.1 * shake)
    };
    Phase {
        objective: Objective::Progress,
        end_game: budgeted_end_game(),
        seeds_per_game: SEEDS_PER_GAME,
        mutation_rate: RateLimits::new(rates.clone()),
        crossover_rate: RateLimits::new(rates),
        mutation_step: step,
        // uncapped: a run is stopped by hand, and checkpoints keep what it found
        max_generations: usize::MAX,
    }
}

/// the clock and the ceiling: [`PILL_BUDGET`] pills to destroy every virus training asks for
fn budgeted_end_game() -> EndGame {
    EndGame {
        pieces: PILL_BUDGET,
        ..EndGame::of_cleared(VIRUSES_TO_CLEAR)
    }
}

/// Run a phase and return the winner of a [`PLAYOFF`] among the top of the final population.
fn run(phase: Phase, population_seed: Option<DrNeuralGenome>) -> DrNeuralGenome {
    let mut algorithm = GeneticAlgorithm::new(
        neural_fitness(),
        neural_mutation(),
        HyperParameters::new(POPULATION, ELITE_RATE, 0.5),
        vec![phase],
        population_seed,
    );
    let stats = algorithm.run();
    let finalists: Vec<DrNeuralGenome> = algorithm
        .population()
        .iter()
        .take(PLAYOFF)
        .map(|organism| organism.genome())
        .collect();
    playoff(&finalists).unwrap_or_else(|| stats.max().genome())
}

/// Play the finalists over the same unseen seeds and return the one destroying the most viruses.
fn playoff(finalists: &[DrNeuralGenome]) -> Option<DrNeuralGenome> {
    if finalists.is_empty() {
        return None;
    }
    println!(
        "\nplayoff: the top {} of the final population over {} unseen seeds",
        finalists.len(),
        UNSEEN_SEEDS
    );
    let played: Vec<(u32, u32, usize, DrNeuralGenome)> = finalists
        .par_iter()
        .enumerate()
        .map(|(seat, genome)| {
            let results = unseen_results(*genome);
            let viruses: u32 = results.iter().map(GameResult::cleared).sum();
            let bottles: u32 = results.iter().map(GameResult::bonus).sum();
            (viruses, bottles, seat, *genome)
        })
        .collect();

    let mut ranked = played;
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    for (viruses, bottles, seat, _) in &ranked {
        println!(
            "  seat {}: {} viruses, {} bottles over {} games",
            seat + 1,
            viruses,
            bottles,
            UNSEEN_SEEDS
        );
    }
    ranked.first().map(|(_, _, _, genome)| *genome)
}

/// Report `genome` on unseen seeds and whether it survived the budget, as the fitness asks.
fn report_unseen(genome: DrNeuralGenome) -> bool {
    let results = unseen_results(genome);
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
    survived_the_budget(&results)
}

/// The run, played until its confirmed finish line or stopped by hand.
fn survive(population_seed: Option<DrNeuralGenome>) -> DrNeuralGenome {
    without_a_hold_opinion(run(clear_phase(population_seed.is_some()), population_seed))
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
    let mut rows: Vec<(u64, f64, u32, u32)> = (0..clones)
        .into_par_iter()
        .map(|clone| {
            let network = imitation::teach_without(&corpus, epochs, clone, &silenced);
            let report = imitation::measure(&corpus, &network);
            let results = unseen_results(network.into());
            (
                clone,
                report.agreement,
                results.iter().map(|r| r.cleared()).sum(),
                results.iter().map(|r| r.bonus()).sum(),
            )
        })
        .collect();
    rows.sort_by_key(|(clone, _, _, _)| *clone);

    for (clone, agreement, viruses, bottles) in &rows {
        println!(
            "  clone {:2}: agreement {:.1}%, {} viruses, {} bottles",
            clone + 1,
            100.0 * agreement,
            viruses,
            bottles
        );
    }

    let mut viruses: Vec<u32> = rows.iter().map(|(_, _, v, _)| *v).collect();
    viruses.sort_unstable();
    let mut agreement: Vec<f64> = rows.iter().map(|(_, a, _, _)| *a).collect();
    agreement.sort_by(f64::total_cmp);
    let cleared = viruses.iter().filter(|v| **v >= TAUGHT_ENOUGH).count();
    println!(
        "median {} viruses, mean {}, best {}, {} of {} cleared {}; median agreement {:.1}%",
        median(&viruses),
        viruses.iter().sum::<u32>() / viruses.len().max(1) as u32,
        viruses.last().copied().unwrap_or(0),
        cleared,
        viruses.len(),
        TAUGHT_ENOUGH,
        100.0 * agreement[agreement.len() / 2]
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
    let weights: [f64; DR_NEURAL_GENOME_SIZE] = genome.into();
    println!("\n// {}, for models::survival_trained", what);
    println!("    DrNeuralNetwork::new(&[");
    for line in weights.chunks(8) {
        let numbers: Vec<String> = line.iter().map(|w| format!("{:.6}", w)).collect();
        println!("        {},", numbers.join(", "));
    }
    println!("    ])");
}

/// Report whole games from the first bottle through the real agent, as the fitness scores them.
fn report_play(what: &str, genome: DrNeuralGenome) {
    let results = unseen_results(genome);
    let viruses: u32 = results.iter().map(|r| r.cleared()).sum();
    let bottles: u32 = results.iter().map(|r| r.bonus()).sum();
    let pills: u32 = results.iter().map(|r| r.pieces()).sum();
    let buried = results.iter().filter(|r| r.game_over()).count();
    println!(
        "{}: {} viruses, {} bottles, {} pills, {} buried, over {} games",
        what, viruses, bottles, pills, buried, UNSEEN_SEEDS
    );
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
    let block = Seed::from(UNSEEN_SEED_BLOCK);
    let fixture = unseen_fixture(UNSEEN_SEEDS as usize);
    let network: DrNeuralNetwork = genome.into();
    (0..UNSEEN_SEEDS)
        .into_par_iter()
        .map(|seed| fixture.play_seed(network, block + Seed::from(seed)))
        .collect()
}

/// Both stages in order.
pub fn ga_main_auto() -> Result<(), String> {
    println!("== stage 1: imitation ==");
    let taught = pretrain(imitation::LESSON_PILLS, TAUGHT_ENOUGH);

    println!("\n== stage 2: the run, {} pill budget ==", PILL_BUDGET);
    let trained = survive(Some(taught));
    report_play("trained model", trained);

    println!("\nthe model to embed, on {} unseen seeds", UNSEEN_SEEDS);
    report_unseen(trained);
    print_weights("trained model", trained);
    Ok(())
}

/// the same run, seeded from the built in model rather than taught from scratch
pub fn ga_main_tune() -> Result<(), String> {
    let seed: DrNeuralGenome = models::survival_trained().into();
    report_play("embedded model", seed);
    let trained = survive(Some(seed));
    report_play("trained model", trained);
    report_unseen(trained);
    print_weights("trained model", trained);
    Ok(())
}

/// `ga dr survive`: the run on its own, from random weights, with no imitation before it
pub fn ga_main_survive() -> Result<(), String> {
    let trained = survive(None);
    report_play("trained model", trained);
    print_weights("trained model", trained);
    Ok(())
}

/// `ga dr trial [population] [generations] [seed] [shake]`: a short bounded run that checks a
/// change still leaves the algorithm something to climb.
///
/// `seed` is `scratch` (random weights, the default), `taught` (a quick imitation), `tune` (the
/// embedded model) or `hold` (taught, with the held pill on offer); `shake` scales the rates.
pub fn ga_main_trial(args: &[String]) -> Result<(), String> {
    let population: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(200);
    let generations: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
    let from = args.get(2).map(String::as_str).unwrap_or("scratch");

    let seed = match from {
        "scratch" => None,
        "taught" | "hold" => Some(pretrain(TRIAL_LESSON_PILLS, TAUGHT_ENOUGH)),
        "tune" => Some(models::survival_trained().into()),
        other => return Err(format!("unknown trial seed '{}'", other)),
    };
    let hold = if from == "hold" { Hold::On } else { Hold::Off };
    let shake: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let mut phase = clear_phase_shaken(seed.is_some(), shake);
    phase.max_generations = generations;

    println!(
        "\npopulation {}, {} generations, {} pill budget, hold {:?}, shaken x{}, seeded from {}",
        population,
        generations,
        PILL_BUDGET,
        hold,
        shake,
        match from {
            "tune" => "the embedded model",
            "scratch" => "random weights",
            _ => "imitation",
        }
    );
    let stats = GeneticAlgorithm::new(
        neural_fitness_with(hold),
        neural_mutation(),
        HyperParameters::new(population, ELITE_RATE, 0.5),
        vec![phase],
        seed,
    )
    .run();

    let best = stats.max().result();
    println!(
        "best after {} generations: {} viruses, {} bottles, {} pills",
        generations,
        best.cleared(),
        best.bonus(),
        best.pieces()
    );
    Ok(())
}

/// play the built in model on a few seeds and report how far it gets
pub fn ga_diagnose() -> Result<(), String> {
    println!(
        "embedded model, bottles 0 to {} in {} pills, {} viruses in all",
        TOP_TRAINING_LEVEL, PILL_BUDGET, VIRUSES_TO_CLEAR
    );
    report_unseen(models::survival_trained().into());
    Ok(())
}

pub fn ga_main() -> Result<(), String> {
    ga_main_auto()
}
