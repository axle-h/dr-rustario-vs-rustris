//! The Dr. Rustario AI: bottle features, placement search and the agent that plays them. The
//! generic half - the neural network and the genetic algorithm - lives in [engine::ai]. The
//! training entry points (the `ga dr` subcommand) are not compiled for the browser; the playing
//! agent and its trained network always are.
mod evaluator;
mod features;
mod placement;
// these drive a real [crate::game::Game], which the crate's own test build swaps for a mock,
// so they are compiled out of it; the launcher links the real thing
#[cfg(not(test))]
pub mod agent;
#[cfg(all(not(test), not(target_os = "emscripten")))]
pub mod genetic;
#[cfg(all(not(test), not(target_os = "emscripten")))]
pub mod harness;
#[cfg(all(not(test), not(target_os = "emscripten")))]
mod headless_game;
pub mod input_sequence;
pub mod models;

pub use models::{DrNeuralGenome, DrNeuralNetwork, DR_NEURAL_GENOME_SIZE};
