//! The parts of a learned agent that are not game rules: the neural network it scores
//! placements with, and the genetic algorithm that trains one. Both games extract their own
//! board features into a [FeatureNetwork] and supply their own [genetic::Fitness].
//!
//! The training half is not compiled for the browser; the network itself always is.

mod coefficient;
mod game_result;
mod genome;
mod neural;
mod seed;
mod end_game;
mod pacer;
#[cfg(not(target_os = "emscripten"))]
mod generation_record;
#[cfg(not(target_os = "emscripten"))]
mod generation_stats;
#[cfg(not(target_os = "emscripten"))]
mod mutation;
#[cfg(not(target_os = "emscripten"))]
mod objective;
#[cfg(not(target_os = "emscripten"))]
mod organism;
#[cfg(not(target_os = "emscripten"))]
mod genetic;

pub use coefficient::{raw_coefficient_range, Coefficient, DEFAULT_MUTATION_STEP};
pub use end_game::EndGame;
pub use game_result::GameResult;
pub use genome::Genome;
pub use neural::{
    ActivationFunction, BottleFeatureNetwork, BottleNeuralGenome, FeatureNetwork, NeuralGenome,
    NeuralNetwork, Tensor, BOTTLE_FEATURE_INPUTS, BOTTLE_NEURAL_GENOME_SIZE, FEATURE_INPUTS,
    NEURAL_GENOME_SIZE,
};
pub use pacer::KeyPacer;
pub use seed::Seed;

#[cfg(not(target_os = "emscripten"))]
pub use genetic::{Fitness, GeneticAlgorithm, HyperParameters};
#[cfg(not(target_os = "emscripten"))]
pub use generation_stats::GenerationStatistics;
#[cfg(not(target_os = "emscripten"))]
pub use mutation::{GenomeMutation, RateLimits};
#[cfg(not(target_os = "emscripten"))]
pub use objective::{Objective, Phase};
#[cfg(not(target_os = "emscripten"))]
pub use organism::Organism;
