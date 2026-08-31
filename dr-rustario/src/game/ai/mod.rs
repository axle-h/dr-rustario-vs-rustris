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
pub mod explain;
#[cfg(all(not(test), not(target_os = "emscripten")))]
pub mod genetic;
#[cfg(all(not(test), not(target_os = "emscripten")))]
pub mod harness;
#[cfg(all(not(test), not(target_os = "emscripten")))]
mod headless_game;
#[cfg(all(not(test), not(target_os = "emscripten")))]
pub mod imitation;
pub mod input_sequence;
pub mod models;
#[cfg(all(not(test), not(target_os = "emscripten")))]
pub mod probe;
mod run;

pub use models::{DrNeuralGenome, DrNeuralNetwork, DR_NEURAL_GENOME_SIZE};
pub use n64::{N64Ai, DEFAULT_SKILL, SKILLS, SKILL_ORDER};

/// Which brain an ai player is thinking with. The default is the **trained network**, which a
/// `ga dr` run produces and which is now the strongest player here: the 1-player demo, the
/// hardest difficulty and player 1 of the 2-player demo all field it. Dr. Mario 64's own
/// deterministic opponent, ported in [n64], is what the three difficulties below the top play -
/// its six rows of weights are the only difficulty *dial* either of them has - and the linear
/// scorer is the hand written baseline that training is measured against.
#[derive(Clone, Copy, Debug)]
pub enum DrAiKind {
    N64(N64Ai),
    Neural(DrNeuralNetwork),
    Linear,
}

impl DrAiKind {
    /// one of the N64 ai's six rows of weights, which is what a difficulty picks between
    pub fn n64(skill: u8) -> Self {
        Self::N64(N64Ai::with_skill(skill))
    }

    /// the `nth` weakest of the six rows, as measured in [`SKILL_ORDER`]
    pub fn n64_nth_weakest(nth: usize) -> Self {
        Self::n64(SKILL_ORDER[nth.min(SKILLS - 1)])
    }
}

impl Default for DrAiKind {
    /// The trained network, which is now the better player: over twenty seeds at the training
    /// budget it destroyed 20,016 viruses and finished 422 bottles against the strongest N64
    /// row's 18,093 and 405, winning seventeen of the twenty. That is the first time it has
    /// beaten the ai it learned from - it was 40% *slower* per bottle before the placement
    /// search learned to tuck and the entrance height was fed to it - so this is what a
    /// 1-player demo watches and what the hardest difficulty plays.
    fn default() -> Self {
        Self::Neural(models::survival_trained())
    }
}
