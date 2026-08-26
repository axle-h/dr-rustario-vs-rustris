//! The Rustris AI: board features, placement search and the agent that plays them. The generic
//! half - the neural network and the genetic algorithm - lives in [engine::ai]. The training
//! entry points (the `ga` subcommand) are not compiled for the browser; the playing agent and
//! its trained networks always are.
pub mod action_evaluator;
pub mod agent;
mod apply_inputs;
mod board_features;
#[cfg(not(target_os = "emscripten"))]
pub mod genetic;
#[cfg(not(target_os = "emscripten"))]
pub mod harness;
mod headless_game;
mod input_search;
pub mod input_sequence;
pub mod linear;
pub mod models;
pub mod recording;

pub use models::{NeuralGenome, TetrisNeuralNetwork, NEURAL_GENOME_SIZE};
