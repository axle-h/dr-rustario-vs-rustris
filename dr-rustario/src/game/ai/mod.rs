//! The Dr. Rustario AI: bottle features, placement search and the agent that plays them. The
//! generic half - the neural network and the genetic algorithm - lives in [engine::ai]. The
//! training entry points (the `ga dr` subcommand) are not compiled for the browser; the playing
//! agent and its trained network always are.
mod evaluator;
mod features;
mod n64;
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
pub use n64::N64Ai;

/// Which brain an ai player is thinking with. The default is Dr. Mario 64's own deterministic
/// opponent, ported in [n64]; the neural network is what a `ga dr` run trains, and the linear
/// scorer is the hand written baseline that training is measured against.
#[derive(Clone, Copy, Debug)]
pub enum DrAiKind {
    N64(N64Ai),
    Neural(DrNeuralNetwork),
    Linear,
}

impl Default for DrAiKind {
    fn default() -> Self {
        Self::N64(N64Ai::new())
    }
}
