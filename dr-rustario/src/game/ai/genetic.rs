//! The Dr. Rustario end of training: it plays the headless game for the genetic algorithm and
//! provides the `ga dr` subcommand's entry points.
//!
//! `ga dr auto` runs two stages, in order, the second starting from what the first left behind:
//!
//! 1. **Imitation** ([`crate::game::ai::imitation`]). A network is taught by gradient descent
//!    to rank placements the way the deterministic ai ranks them. This exists because a genetic
//!    algorithm cannot select between members that all score zero, and from random weights
//!    almost all of them do. Networks are taught until one of them plays [`TAUGHT_ENOUGH`]
//!    viruses, since where a network starts moves how well it plays far more than it moves how
//!    well it agrees with its teacher.
//! 2. **The run**. Candidates play the game itself, starting on the first bottle, clearing it,
//!    moving on to the next, and are scored on the viruses they took out of a fixed budget of
//!    pills ([`crate::game::ai::run::PILL_BUDGET`]). That one number is both halves of what a
//!    good player is: being buried stops the count, and dawdling spends the budget on fewer
//!    bottles.
//!
//! It used to be three, the third asking a model that had stopped dying to stop dawdling, and
//! it is two because that third stage could not do its job. It scored on *bottles finished*,
//! which [`GameResult`] averages over the seeds and rounds to a whole number, so two hundred
//! and fifty candidates were ranked on a fitness with about five distinct values and selection
//! fell through to the tiebreak - the game's own score, which is exponential in the viruses a
//! single combo takes. A hundred and fifty generations of chasing combos cost it the survival
//! the stage before had bought, and `ga dr auto` threw the whole stage away. The clock belongs
//! in the one fitness that is doing the selecting, which is where it is now.
//!
//! Two things stop a run ending on luck. The finish line - every seed played to the end of its
//! budget without a burial - is asked again on seeds nothing has trained against
//! ([`NeuralFitness::confirm`]), because the best of two hundred and fifty candidates clears
//! any bar on the seeds it happened to be dealt. And what a finished run *embeds* is not simply
//! its best member either: the top of the final population is played off against each other
//! over one fixed block of unseen seeds, since the ordering inside a generation is the ordering
//! on that generation's four seeds and nothing more.
//!
//! Each stage can also be run on its own; see the `ga dr` arms in the launcher.

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

/// Whole games a finished model is reported on. These decide nothing - the finish line is
/// inside the fitness, where the search can see it - they are how a stage says what it left
/// behind.
const UNSEEN_SEEDS: u128 = 5;

/// Where those seeds live. Far away from the block training walks through, so a model is
/// reported on bottles it has never been shown.
const UNSEEN_SEED_BLOCK: u128 = 1 << 96;

/// How many candidates a generation holds. Every one of them plays [`SEEDS_PER_GAME`] whole
/// games, and a good model's game runs to thousands of pills, so this is what a generation
/// costs: at a thousand candidates over three seeds a generation took minutes.
const POPULATION: usize = 250;

/// Whole games each candidate plays per generation, and the other half of what a generation
/// costs. Two of them measured luck rather than skill: taking the best of two hundred and fifty
/// candidates over two seeds is an extreme of two hundred and fifty noisy samples, so somebody
/// cleared both every generation whatever the population was worth, and the winner went on to
/// clear a fresh seed one time in five. Four is dear, which is what the probe seeds in
/// [`crate::game::ai::headless_game`] are for: only candidates worth the rest of them play it.
const SEEDS_PER_GAME: usize = 4;

/// Which input says a candidate is a placement of the held pill, in
/// [`crate::game::ai::evaluator::raw_inputs`]'s order: the last of them.
const HELD_INPUT: usize = engine::ai::BOTTLE_FEATURE_INPUTS - 1;

/// What fraction of a generation is carried into the next one unchanged.
///
/// The engine's default is 0.005, which at a population of two hundred and fifty is **one**
/// elite - and every generation is played on a fresh block of seeds, elites included, so that
/// one genome has to win again on games it has never seen or it is gone. That is far too thin
/// to do what elitism is for. Seeded from a model that already clears eighteen bottles, most
/// mutations are harmful, and a run of it showed exactly what a population with nothing holding
/// its top up looks like: over twenty five generations the best member stood still while the
/// median fell thirty one percent and the ninety fifth percentile fell fifteen.
///
/// Ten of two hundred and fifty is still only four percent, so there is no shortage of churn;
/// it is enough that a good genome surviving one unlucky seed block is the normal case rather
/// than a coin toss.
const ELITE_RATE: f64 = 0.04;

/// How often a run stops to print the best genome it has, as the weights [`models`] embeds.
///
/// A run is not capped: its finish line is the whole game cleared inside the budget, which
/// nothing has come near, so in practice it goes until it is stopped. That is deliberate -
/// there is always a better model and a cap only decides in advance how much better - and it is
/// affordable only because a run stopped halfway has something to show for itself. Every
/// checkpoint prints a paste-ready model and how it plays on the seeds nothing trains against,
/// so the log carries the answer rather than the `generation-record` csv being the only copy.
const CHECKPOINT_GENERATIONS: usize = 25;

/// How many of the final population are played off against each other for the right to be
/// embedded. The generation's own ordering is the ordering on that generation's four seeds, so
/// its winner is the luckiest of two hundred and fifty as much as it is the best of them; over
/// one fixed block of unseen seeds these few can be compared on the same games.
const PLAYOFF: usize = 8;

/// a trial teaches from fewer pills than a real run, since it is checking the shape of the
/// thing rather than producing a model
const TRIAL_LESSON_PILLS: usize = 3000;

/// How well a taught network has to play before stage two starts from it, counted in viruses
/// over [`UNSEEN_SEEDS`] whole games from the first bottle. Where a network starts moves this
/// about far more than it moves how well the network agrees with the ai, so stage one keeps
/// teaching until one of them clears the bar rather than taking whatever the first few give.
///
/// **It has to be set against what the features can actually do or it does not bind.** At 1500,
/// which is what it was while the model read thirty two inputs, every one of fifty clones of
/// the current nineteen clears it - so stage one would stop on attempt 1 and hand the genetic
/// algorithm a single random draw out of a distribution running from about 2400 to 4455. That
/// is the exact failure the bar exists to prevent. `ga dr screen` puts the median at 3814, so
/// 4000 asks for a good draw and usually gets one inside [`PRETRAIN_ATTEMPTS`]; if it does not,
/// the best of them is taken anyway. Re-measure this whenever the feature set changes.
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

    /// Every [`CHECKPOINT_GENERATIONS`] generations, say how the best genome so far plays on the
    /// seeds nothing trains against and print it as the weights to embed. A run has no
    /// generation cap, so this is how one that is stopped halfway is worth having run at all.
    fn checkpoint(&self, generation: usize, genome: &Genome<DR_NEURAL_GENOME_SIZE>) {
        if generation == 0 || !generation.is_multiple_of(CHECKPOINT_GENERATIONS) {
            return;
        }
        let what = format!("generation {}", generation);
        println!();
        report_play(&what, *genome);
        print_weights(&what, without_a_hold_opinion(*genome));
    }

    /// The second opinion on a candidate that has just tripped the finish line, and the whole of
    /// why a run no longer ends on a lucky generation. It is asked the same question - every
    /// seed played to the end of its budget without a burial - over the block of seeds nothing
    /// trains against. It costs one candidate's run, and only when a generation claims to be
    /// finished.
    fn confirm(&self, genome: &Genome<DR_NEURAL_GENOME_SIZE>) -> bool {
        let fixture = unseen_fixture(self.fixture.seeds_per_game());
        survived_the_budget(&fixture.play_run((*genome).into(), Seed::from(UNSEEN_SEED_BLOCK)))
    }
}

fn neural_fitness() -> NeuralFitness {
    neural_fitness_with(Hold::Off)
}

/// the same, with the held pill on offer, which is the only way a run can put a price on a swap
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

/// The run: play the game from the first bottle and take as many viruses as you can out of
/// [`PILL_BUDGET`] pills.
///
/// The end game is both caps at once. The pills are the clock, and are what the fitness is
/// really measured against; the viruses stop a game that has cleared every bottle training asks
/// for, which nothing has ever done but which would otherwise run forever.
///
/// How hard the population is shaken depends on where it started. From random weights there is
/// nothing to preserve and the search has to be wide. From a taught model there is a great deal
/// to preserve - it already clears bottles - and shaking it as hard would throw most of a
/// generation away: at the wide rates the median member of a taught population scores a
/// twentieth of what the model it was mutated from does.
fn clear_phase(seeded: bool) -> Phase {
    clear_phase_shaken(seeded, 1.0)
}

/// The same phase with the population shaken `shake` times as hard, which is the one dial worth
/// turning when a run stops climbing.
///
/// How wide the search should be depends on how good the seed already is, and the seed got
/// better: with tucks and the entrance height, imitation alone now produces a model that clears
/// what a whole training run used to. The rates here were set against a weaker one, and a
/// population whose median falls while its best member stands still is one where mutation is
/// destroying more than it finds - which is the same failure the wide rates showed on a taught
/// seed, one step further along.
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
        // uncapped: the finish line is the whole game, and short of that a run is stopped
        // rather than finished. [`CHECKPOINT_GENERATIONS`] is what makes that practical.
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

/// Run a phase and hand back what it is worth embedding: not simply the best member of the last
/// generation, but the winner of a playoff between the top of it over one fixed block of unseen
/// seeds. Within a generation the ordering is the ordering on that generation's four seeds, and
/// the top few of two hundred and fifty are separated by less than that ordering's own noise.
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

/// Play the finalists over the same unseen seeds and hand back whichever destroyed the most
/// viruses, saying what each of them managed.
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

/// Play `genome` on seeds it has never trained against, report every game, and say whether they
/// add up to a candidate that is still standing - the same line the fitness draws, so a model
/// is reported against exactly what it was trained to do.
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

/// The run, played out. Only its own finish line ends it - the whole game cleared inside the
/// budget, on every seed, confirmed on seeds it has never played - so in practice a run is
/// *stopped* rather than finished, and what it leaves behind is the last checkpoint in its log.
/// If one ever does finish, what comes back is the winner of the playoff in [`run`] rather than
/// whichever member happened to top the last generation.
fn survive(population_seed: Option<DrNeuralGenome>) -> DrNeuralGenome {
    without_a_hold_opinion(run(clear_phase(population_seed.is_some()), population_seed))
}

/// `ga dr screen [pills] [clones] [silenced] [epochs]`: can a feature set learn the deterministic ai at
/// all, answered as fast as it can be answered.
///
/// It is [`ga_main_pretrain`] with three differences, and each of them is something stage one
/// gets wrong as a *measurement*.
///
/// It teaches every clone rather than stopping at the first that clears the bar, and reports
/// the **median** as well as the best. Best-of-N is an extreme of N noisy draws and it lies
/// about which feature set is better: two sets measured here scored 1431 and 765 on their best
/// while their medians were 181 and 273, so the two statistics disagreed about the ordering.
/// Only the median separated the set that went on to work from the ones that did not.
///
/// It runs the clones in **parallel**. They share one corpus and nothing else, so a screen
/// costs about what a handful of clones used to.
///
/// And it takes a list of inputs to **silence throughout training**
/// ([`imitation::teach_without`]), which is how a feature is taken away without rebuilding the
/// network around a smaller `BOTTLE_FEATURE_INPUTS`. That is what makes searching for the
/// smallest set that still works affordable: one binary, one corpus, one ablation per run.
/// Indices are [`evaluator::raw_inputs`]'s order, which is what `ga dr explain` lists.
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

/// Stage one. Gather the lessons once and learn them over and over, keeping teaching until one
/// of the networks plays well enough to be worth handing on.
///
/// How well a clone reproduces the ai and how well it *plays* are not the same measure, and
/// where a network starts moves the second far more than the first: four clones of one corpus
/// agreed with the ai on 53% to 56% of pills and played anywhere between 291 and 2526 viruses.
/// So the corpus is gathered once - it is deterministic, and re-gathering it would only cost
/// time - and only the initial weights are drawn again.
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

/// Take away whatever opinion about swapping a run left behind.
///
/// Training plays with [`crate::game::ai::agent::Hold::Off`], so the input that says "this is
/// the pill you are holding" is zero on every candidate of every game: its weights do nothing,
/// and mutation walks them about freely for the whole run. That is harmless while hold is off
/// and is exactly the trap it was off for - a model that has never been asked about a swap
/// would come out with a loud random answer ready for the first agent that asks. Silenced, it
/// comes out indifferent, which is the only honest thing a run that never tested one can say.
fn without_a_hold_opinion(genome: DrNeuralGenome) -> DrNeuralGenome {
    let mut network: DrNeuralNetwork = genome.into();
    network.silence_input(HELD_INPUT);
    network.into()
}

/// Print a genome as the weights [`models`] embeds. A finished run is only useful if its result
/// can be got into the binary, and a genome's own `Display` is the raw coefficients the
/// algorithm works in rather than the numbers the network is built from - so this prints the
/// body of `survival_trained`, ready to paste over the one that is there.
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

/// How a stage's result plays: whole games from the first bottle, through the agent that
/// really plays it, which is exactly what the fitness scores.
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

/// The fixture the unseen block is played on: the same budget and the same caps training
/// itself runs under, so a report and a fitness are measuring the same thing.
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

/// Both stages in order, which is what a training run is.
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

/// `ga dr trial [population] [generations] [seed]`: a short, bounded run. This trains nothing
/// and verifies nothing; it is how a change to the features, the fitness or the teaching is
/// checked for having left the algorithm something it can climb, which is visible within a
/// handful of generations or not at all.
///
/// `seed` is `scratch` (the default: from random weights), `taught` (from a quick imitation
/// seed), `tune` (from the embedded model, which is the seed strength a real run has by the
/// time it is climbing) or `hold` (a taught seed with the held pill on offer, which is how the
/// question in [`crate::game::ai::agent::Hold`] gets an answer). A fourth argument multiplies
/// the mutation and crossover rates, so how hard to shake a population is something two runs
/// can be compared on rather than argued about.
pub fn ga_main_trial(args: &[String]) -> Result<(), String> {
    let population: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(200);
    let generations: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
    let from = args.get(2).map(String::as_str).unwrap_or("scratch");

    let seed = match from {
        "scratch" => None,
        "taught" | "hold" => Some(pretrain(TRIAL_LESSON_PILLS, TAUGHT_ENOUGH)),
        // the embedded model, which is the only seed with a *real* run's strength behind it -
        // a trial taught from `TRIAL_LESSON_PILLS` is a weaker one, and how hard to shake a
        // population is exactly a question about how good its seed is
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
