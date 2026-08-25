//! The Rustris AI: board features, placement search and the agent that plays them. The generic
//! half - the neural network and the genetic algorithm - lives in [engine::ai]. The training
//! entry points (the `ga` subcommand) are not compiled for the browser; the playing agent and
//! its trained networks always are.
pub mod recording;
mod apply_inputs;
mod input_search;
pub mod input_sequence;
pub mod action_evaluator;
mod board_features;
pub mod agent;
mod headless_game;
pub mod models;
#[cfg(not(target_os = "emscripten"))]
pub mod genetic;
#[cfg(not(target_os = "emscripten"))]
pub mod harness;
pub mod linear;

pub use models::{NeuralGenome, TetrisNeuralNetwork, NEURAL_GENOME_SIZE};
