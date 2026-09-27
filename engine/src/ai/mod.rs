//! The parts of a learned agent that are not game rules: the neural network it scores
//! placements with, and the genetic algorithm that trains one. Both games extract their own
//! board features into a [FeatureNetwork] and supply their own [genetic::Fitness].
//!
//! The training half is not compiled for the browser or Android, which have nowhere to run it;
//! the network itself always is.

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
pub use objective::{Objective, Phase};
#[cfg(not(any(target_os = "emscripten", target_os = "android")))]
pub use organism::Organism;
