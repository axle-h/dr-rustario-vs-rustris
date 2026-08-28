//! The Dr. Rustario end of training: it plays the headless game for the genetic algorithm and
//! provides the `ga dr` subcommand's entry points.
//!
//! `ga dr auto` runs three stages, in order, each one starting from what the last left behind:
//!
//! 1. **Imitation** ([`crate::game::ai::imitation`]). A network is taught by gradient descent
//!    to rank placements the way the deterministic ai ranks them. This exists because a genetic
//!    algorithm cannot select between members that all score zero, and from random weights
//!    almost all of them do. Networks are taught until one of them plays [`TAUGHT_ENOUGH`]
//!    viruses, since where a network starts moves how well it plays far more than it moves how
//!    well it agrees with its teacher.
//! 2. **Survival**. Candidates play the game itself, starting on the first bottle, clearing it,
//!    moving on to the next, and are scored on the viruses they took out before they were
//!    buried. There is no pill budget and nothing rewards speed - a model may take as long as
//!    it likes over a bottle - so this stage trains purely for staying alive. It ends the first
//!    time a candidate finishes the run: one of its seeds out of the last bottle and every
//!    other one of them at least as far as [`PROVEN_LEVEL`]. That is the whole of the test, and
//!    it is inside the fitness rather than bolted on after it, so what training selects for and
//!    what ends training are the same thing.
//! 3. **Efficiency**. The same game with a pill budget, scored on bottles finished rather than
//!    viruses destroyed. A model that has stopped dying has nothing left to gain from stage
//!    two, and this is what asks it to stop dawdling: taking the clear in front of it rather
//!    than tidying, and finishing a bottle in three hundred pills rather than nine hundred.
//!    Survival is not thrown away by it, since a model that buries itself finishes no more
//!    bottles - but it is checked afterwards all the same.
//!
//! Each stage can also be run on its own; see the `ga dr` arms in the launcher.

use crate::game::ai::headless_game::{HeadlessGameFixture, HeadlessGameOptions, VIRUSES_TO_CLEAR};
use crate::game::ai::imitation;
use crate::game::ai::models::{self, DrNeuralGenome, DrNeuralNetwork, DR_NEURAL_GENOME_SIZE};
use crate::game::ai::run::{run_finished, PROVEN_LEVEL, TOP_TRAINING_LEVEL};
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

/// The pills the efficiency stage gives a candidate to finish as many bottles as it can in.
/// Enough for a good model to get well up the levels, so that dawdling over the early ones
/// costs it something: at the hundred and sixty odd pills a bottle a good model takes, this
/// reaches somewhere around bottle twenty of the thirty one stage two asks for. It was half
/// this while the run stopped at bottle twenty, and it is the whole of what stage three costs -
/// that stage is a fixed [`EFFICIENCY_GENERATIONS`] generations, so doubling the budget doubles
/// the time it takes.
const PILL_BUDGET: u32 = 3000;

/// The efficiency stage has no finish line to reach - there is always a faster model - so it is
/// bounded by generations instead.
const EFFICIENCY_GENERATIONS: usize = 150;

/// a trial teaches from fewer pills than a real run, since it is checking the shape of the
/// thing rather than producing a model
const TRIAL_LESSON_PILLS: usize = 3000;

/// How well a taught network has to play before stage two starts from it, counted in viruses
/// over [`UNSEEN_SEEDS`] whole games from the first bottle. Where a network starts moves this
/// about far more than it moves how well the network agrees with the ai, so stage one keeps
/// teaching until one of them clears the bar rather than taking whatever the first few give.
const TAUGHT_ENOUGH: u32 = 1500;

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
}

fn neural_fitness() -> NeuralFitness {
    NeuralFitness {
        fixture: HeadlessGameFixture::new(
            RandomMode::Bag,
            rand::random(),
            HeadlessGameOptions::default(),
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

/// Stage two: play the game from the first bottle and get as far as you can.
///
/// How hard the population is shaken depends on where it started. From random weights there is
/// nothing to preserve and the search has to be wide. From a taught model there is a great deal
/// to preserve - it already clears bottles - and shaking it as hard would throw most of a
/// generation away: at the wide rates the median member of a taught population scores a
/// twentieth of what the model it was mutated from does.
fn clear_phase(seeded: bool) -> Phase {
    let (rates, step) = if seeded {
        (0.03..=0.08, 0.05)
    } else {
        (0.1..=0.20, 0.1)
    };
    Phase {
        objective: Objective::Progress,
        // stops a single game once its last bottle has come out; whether the *run* is over is
        // the fitness's own answer, since only it sees the seeds apart
        end_game: EndGame::of_cleared(VIRUSES_TO_CLEAR),
        seeds_per_game: SEEDS_PER_GAME,
        mutation_rate: RateLimits::new(rates.clone()),
        crossover_rate: RateLimits::new(rates),
        mutation_step: step,
        // no cap: a run ends when a candidate clears the game and proves it, not on a count
        max_generations: usize::MAX,
    }
}

/// Stage three: the same game, but with a pill budget and scored on bottles finished rather
/// than viruses destroyed, which is what asks a model that has stopped dying to stop dawdling.
/// It is nudged rather than shaken - the rates are a fifth of stage two's - since it is being
/// sharpened, not searched for.
fn efficiency_phase() -> Phase {
    Phase {
        objective: Objective::Score,
        end_game: EndGame::of_pieces(PILL_BUDGET),
        seeds_per_game: SEEDS_PER_GAME,
        mutation_rate: RateLimits::new(0.01..=0.05),
        crossover_rate: RateLimits::new(0.01..=0.05),
        mutation_step: 0.02,
        max_generations: EFFICIENCY_GENERATIONS,
    }
}

fn run(phase: Phase, population_seed: Option<DrNeuralGenome>) -> DrNeuralGenome {
    GeneticAlgorithm::new(
        neural_fitness(),
        neural_mutation(),
        HyperParameters::new(POPULATION, 0.005, 0.5),
        vec![phase],
        population_seed,
    )
    .run()
    .max()
    .genome()
}

/// Play `genome` on seeds it has never trained against, report every game, and say whether they
/// add up to a finished run - the same line the fitness draws, so a model is reported against
/// what it was trained to do.
fn report_unseen(genome: DrNeuralGenome) -> bool {
    let results = unseen_results(genome);
    for (seed, result) in results.iter().enumerate() {
        println!(
            "  seed {}: {} bottles, {} viruses, {} pills {}",
            seed + 1,
            result.bonus(),
            result.cleared(),
            result.pieces(),
            if result.bonus() > TOP_TRAINING_LEVEL {
                "✅"
            } else {
                "❌"
            }
        );
    }
    let proven = results
        .iter()
        .filter(|result| result.bonus() > PROVEN_LEVEL)
        .count();
    println!(
        "  {} of {} seeds reached bottle {}",
        proven, UNSEEN_SEEDS, PROVEN_LEVEL
    );
    run_finished(&results, TOP_TRAINING_LEVEL)
}

/// Stage two, run to its finish. There is nothing bolted on the end of it: the phase's own test
/// is the finish line, and the finish line asks for a candidate that finished the run rather
/// than one that cleared the seeds in front of it, so a genome cannot pass it without
/// generalising and there is nothing left for a second opinion to overturn.
fn survive(population_seed: Option<DrNeuralGenome>) -> DrNeuralGenome {
    run(clear_phase(population_seed.is_some()), population_seed)
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

/// Print a genome as the weights [`models`] embeds. A finished run is only useful if its result
/// can be got into the binary, and a genome's own `Display` is the raw coefficients the
/// algorithm works in rather than the numbers the network is built from - so this prints the
/// body of `virus_clear_trained`, ready to paste over the one that is there.
fn print_weights(what: &str, genome: DrNeuralGenome) {
    let weights: [f64; DR_NEURAL_GENOME_SIZE] = genome.into();
    println!("\n// {}, for models::virus_clear_trained", what);
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

/// play `genome` on seeds it has never trained against, one whole game each
fn unseen_results(genome: DrNeuralGenome) -> Vec<GameResult> {
    let block = Seed::from(UNSEEN_SEED_BLOCK);
    let fixture = HeadlessGameFixture::new(
        RandomMode::Bag,
        block,
        HeadlessGameOptions::default(),
        EndGame::of_cleared(VIRUSES_TO_CLEAR),
    );
    let network: DrNeuralNetwork = genome.into();
    (0..UNSEEN_SEEDS)
        .into_par_iter()
        .map(|seed| fixture.play_seed(network, block + Seed::from(seed)))
        .collect()
}

/// Every stage in order, which is what a training run is.
pub fn ga_main_auto() -> Result<(), String> {
    println!("== stage 1: imitation ==");
    let taught = pretrain(imitation::LESSON_PILLS, TAUGHT_ENOUGH);

    println!("\n== stage 2: survival ==");
    let survivor = survive(Some(taught));
    report_play("stage 2 model", survivor);

    println!("\n== stage 3: efficiency, {} pill budget ==", PILL_BUDGET);
    let sharpened = run(efficiency_phase(), Some(survivor));
    report_play("stage 3 model", sharpened);

    // stage three is bounded by generations rather than by a finish line, so unlike stage two
    // it can end with a model that has stopped finishing the run. That is the one thing it is
    // not allowed to trade away, so it is asked the same question here.
    println!("\nstage 3 model on {} unseen seeds", UNSEEN_SEEDS);
    if report_unseen(sharpened) {
        print_weights("stage 3 model", sharpened);
    } else {
        println!("the stage 3 model no longer finishes the run, embedding the stage 2 model");
        print_weights("stage 2 model", survivor);
        print_weights("stage 3 model, which does not finish the run", sharpened);
    }
    Ok(())
}

/// the same three stages, seeded from the built in model rather than taught from scratch
pub fn ga_main_tune() -> Result<(), String> {
    let seed: DrNeuralGenome = models::virus_clear_trained().into();
    report_play("embedded model", seed);
    let survivor = survive(Some(seed));
    report_play("stage 2 model", survivor);
    let sharpened = run(efficiency_phase(), Some(survivor));
    report_play("stage 3 model", sharpened);
    print_weights("stage 3 model", sharpened);
    Ok(())
}

/// `ga dr survive`: stage two on its own, from scratch, which is what training used to be
pub fn ga_main_survive() -> Result<(), String> {
    let survivor = survive(None);
    report_play("stage 2 model", survivor);
    print_weights("stage 2 model", survivor);
    Ok(())
}

/// `ga dr trial [population] [generations] [stage]`: a short, bounded run of one stage. This
/// trains nothing and verifies nothing; it is how a change to the features, the fitness or the
/// teaching is checked for having left the algorithm something it can climb, which is visible
/// within a handful of generations or not at all.
///
/// `stage` is `scratch` (the default: stage two from random weights), `taught` (stage two from
/// a quick imitation seed) or `efficiency` (stage three from one).
pub fn ga_main_trial(args: &[String]) -> Result<(), String> {
    let population: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(200);
    let generations: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
    let stage = args.get(2).map(String::as_str).unwrap_or("scratch");

    let seed = match stage {
        "scratch" => None,
        "taught" | "efficiency" => Some(pretrain(TRIAL_LESSON_PILLS, TAUGHT_ENOUGH)),
        other => return Err(format!("unknown trial stage '{}'", other)),
    };
    let mut phase = match stage {
        "efficiency" => efficiency_phase(),
        _ => clear_phase(seed.is_some()),
    };
    phase.max_generations = generations;

    println!(
        "\npopulation {}, {} generations, {} phase, seeded from {}",
        population,
        generations,
        match stage {
            "efficiency" => "efficiency",
            _ => "survival",
        },
        if seed.is_some() {
            "imitation"
        } else {
            "random weights"
        }
    );
    let stats = GeneticAlgorithm::new(
        neural_fitness(),
        neural_mutation(),
        HyperParameters::new(population, 0.005, 0.5),
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
        "embedded model, bottles 0 to {}, {} viruses in all",
        TOP_TRAINING_LEVEL, VIRUSES_TO_CLEAR
    );
    report_unseen(models::virus_clear_trained().into());
    Ok(())
}

pub fn ga_main() -> Result<(), String> {
    ga_main_auto()
}
