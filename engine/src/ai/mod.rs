//! The parts of a learned agent that are not game rules: network shapes, genome, genetic
//! algorithm and its [genetic::Fitness] seam, which each game fills with its own headless game.
//! Training is not compiled for the browser or Android.

mod coefficient;
mod end_game;
mod game_result;
#[cfg(not(any(target_os = "emscripten", target_os = "android")))]
mod generation_record;
#[cfg(not(any(target_os = "emscripten", target_os = "android")))]
mod generation_stats;
#[cfg(not(any(target_os = "emscripten", target_os = "android")))]
mod genetic;
mod genome;
#[cfg(not(any(target_os = "emscripten", target_os = "android")))]
mod mutation;
mod neural;
#[cfg(not(any(target_os = "emscripten", target_os = "android")))]
mod objective;
#[cfg(not(any(target_os = "emscripten", target_os = "android")))]
mod organism;
mod pacer;
mod seed;

pub use coefficient::{raw_coefficient_range, Coefficient, DEFAULT_MUTATION_STEP};
pub use end_game::EndGame;
pub use game_result::GameResult;
pub use genome::Genome;
pub use neural::{
    ActivationFunction, BottleFeatureNetwork, BottleNeuralGenome, FeatureNetwork, NeuralGenome,
    NeuralNetwork, Tensor, BOTTLE_FEATURE_INPUTS, BOTTLE_FEATURE_WIDTH, BOTTLE_NEURAL_GENOME_SIZE,
    FEATURE_INPUTS, NEURAL_GENOME_SIZE,
};
pub use pacer::KeyPacer;
pub use seed::Seed;

#[cfg(not(any(target_os = "emscripten", target_os = "android")))]
pub use generation_stats::GenerationStatistics;
#[cfg(not(any(target_os = "emscripten", target_os = "android")))]
pub use genetic::{Fitness, GeneticAlgorithm, HyperParameters};
#[cfg(not(any(target_os = "emscripten", target_os = "android")))]
pub use mutation::{GenomeMutation, RateLimits};
#[cfg(not(any(target_os = "emscripten", target_os = "android")))]
pub use objective::{Objective, Phase, Rung};
#[cfg(not(any(target_os = "emscripten", target_os = "android")))]
pub use organism::Organism;
