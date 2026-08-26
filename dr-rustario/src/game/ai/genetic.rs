//! The Dr. Rustario end of training: it plays the headless game for the genetic algorithm and
//! provides the `ga dr` subcommand's entry points.
//!
//! Training is a single stage with a single goal: play the game and see how far you get. A
//! candidate starts on the first bottle, clears it, moves on to the next, and is scored on the
//! viruses it took out before it was buried. There is no pill budget and nothing rewards speed -
//! a model may take as long as it likes over a bottle - so this trains purely for survival.
//!
//! Training runs until a candidate clears every bottle on every seed it was trained on, and then
//! has to prove it on a block of seeds it has never played. Only if it does that does the run
//! stop; otherwise training carries on from that candidate.

use crate::game::ai::headless_game::{
    HeadlessGameFixture, HeadlessGameOptions, TOP_TRAINING_LEVEL, VIRUSES_TO_CLEAR,
};
use crate::game::ai::models::{self, DrNeuralGenome, DrNeuralNetwork, DR_NEURAL_GENOME_SIZE};
use crate::game::random::RandomMode;
use engine::ai::{
    EndGame, Fitness, GameResult, GeneticAlgorithm, Genome, GenomeMutation, HyperParameters,
    Objective, Phase, RateLimits, Seed,
};
use rayon::prelude::*;

/// seeds a candidate has to clear the whole game on before a run is called finished
const VERIFY_SEEDS: u128 = 5;

/// Where the verification seeds live. Far away from the block training walks through, so a
/// candidate proves itself on bottles it has never been shown.
const VERIFY_SEED_BLOCK: u128 = 1 << 96;
/// bounded so a run always ends with a model to embed

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

/// the one phase: play the game from the first bottle and get as far as you can
fn clear_phase() -> Phase {
    Phase {
        objective: Objective::Progress,
        // reaching this means every bottle up to the top training level came out
        end_game: EndGame::of_cleared(VIRUSES_TO_CLEAR),
        // three whole games to a genome
        seeds_per_game: 3,
        mutation_rate: RateLimits::new(0.1..=0.20),
        crossover_rate: RateLimits::new(0.1..=0.20),
        mutation_step: 0.1,
        // no cap: a run ends when a candidate clears the game and proves it, not on a count
        max_generations: usize::MAX,
    }
}

fn run(population_seed: Option<DrNeuralGenome>) -> DrNeuralGenome {
    GeneticAlgorithm::new(
        neural_fitness(),
        neural_mutation(),
        HyperParameters::default(),
        vec![clear_phase()],
        population_seed,
    )
    .run()
    .max()
    .genome()
}

/// Play `genome` on seeds it has never trained against and report whether it cleared every
/// bottle on every one of them.
fn verify(genome: DrNeuralGenome) -> bool {
    let block = Seed::from(VERIFY_SEED_BLOCK);
    let fixture = HeadlessGameFixture::new(
        RandomMode::Bag,
        block,
        HeadlessGameOptions::default(),
        EndGame::of_cleared(VIRUSES_TO_CLEAR),
    );
    let network: DrNeuralNetwork = genome.into();

    let results: Vec<_> = (0..VERIFY_SEEDS)
        .into_par_iter()
        .map(|seed| (seed, fixture.play_seed(network, block + Seed::from(seed))))
        .collect();

    let mut verified = true;
    for (seed, result) in results.iter() {
        let cleared = !result.game_over() && result.cleared() >= VIRUSES_TO_CLEAR;
        println!(
            "  verify seed {}: {} bottles, {} viruses, {} pills - {}",
            seed + 1,
            result.bonus(),
            result.cleared(),
            result.pieces(),
            if cleared { "cleared" } else { "buried" }
        );
        verified &= cleared;
    }
    verified
}

/// Train until a candidate clears every bottle on its training seeds and then does it again on
/// seeds it has never played. A candidate that fails verification is the population's new seed.
pub fn ga_main_auto() -> Result<(), String> {
    let mut population_seed = None;
    loop {
        let candidate = run(population_seed);
        println!(
            "a candidate cleared every bottle up to level {} on its training seeds, \
             checking it on {} it has never played",
            TOP_TRAINING_LEVEL, VERIFY_SEEDS
        );
        if verify(candidate) {
            println!("verified: {}", candidate);
            return Ok(());
        }
        println!("not verified, training on from this candidate");
        population_seed = Some(candidate);
    }
}

/// the same phase, seeded from the built in model rather than from scratch
pub fn ga_main_tune() -> Result<(), String> {
    let mut population_seed = Some(models::virus_clear_trained().into());
    loop {
        let candidate = run(population_seed);
        if verify(candidate) {
            println!("verified: {}", candidate);
            return Ok(());
        }
        println!("not verified, training on from this candidate");
        population_seed = Some(candidate);
    }
}

/// play the built in model on a few seeds and report how far it gets
pub fn ga_diagnose() -> Result<(), String> {
    println!(
        "built in neural network, bottles 0 to {} ({} viruses in all)",
        TOP_TRAINING_LEVEL, VIRUSES_TO_CLEAR
    );
    verify(models::virus_clear_trained().into());
    Ok(())
}

pub fn ga_main() -> Result<(), String> {
    ga_main_auto()
}
