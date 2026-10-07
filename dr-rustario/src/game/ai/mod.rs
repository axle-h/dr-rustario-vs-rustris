//! The Dr. Rustario AI: bottle features, placement search and the agent that plays them; the
//! network and genetic algorithm live in [engine::ai]. The `ga dr` training entry points are
//! not compiled for the browser or Android.
mod evaluator;
mod features;
mod n64;
mod placement;
// these drive a real [crate::game::Game], which the test build swaps for a mock
#[cfg(not(test))]
pub mod agent;
#[cfg(all(not(test), not(any(target_os = "emscripten", target_os = "android"))))]
pub mod align;
#[cfg(all(not(test), not(any(target_os = "emscripten", target_os = "android"))))]
pub mod duel;
#[cfg(all(not(test), not(any(target_os = "emscripten", target_os = "android"))))]
pub mod explain;
#[cfg(all(not(test), not(any(target_os = "emscripten", target_os = "android"))))]
pub mod genetic;
#[cfg(all(not(test), not(any(target_os = "emscripten", target_os = "android"))))]
pub mod harness;
#[cfg(all(not(test), not(any(target_os = "emscripten", target_os = "android"))))]
mod headless_game;
#[cfg(all(not(test), not(any(target_os = "emscripten", target_os = "android"))))]
pub mod imitation;
pub mod input_sequence;
pub mod models;
#[cfg(all(not(test), not(any(target_os = "emscripten", target_os = "android"))))]
pub mod passes;
#[cfg(all(not(test), not(any(target_os = "emscripten", target_os = "android"))))]
pub mod probe;
mod run;

pub use models::{DrNeuralGenome, DrNeuralNetwork, DR_NEURAL_GENOME_SIZE};
pub use n64::{N64Ai, DEFAULT_SKILL, SKILLS, SKILL_ORDER};

/// Which brain an ai player thinks with. Every difficulty plays the n64 port; the demos field the
/// trained network, the two player one against the n64's best row.
#[derive(Clone, Copy, Debug)]
#[allow(clippy::large_enum_variant)]
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

    /// the network embedded in [`models`]
    pub fn trained() -> Self {
        Self::Neural(models::survival_trained())
    }
}

impl Default for DrAiKind {
    /// The strongest of the N64 ai's six rows.
    fn default() -> Self {
        Self::N64(N64Ai::new())
    }
}
