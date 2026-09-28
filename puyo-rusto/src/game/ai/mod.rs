//! Puyo Rusto's ai: the board it thinks on, what a board is worth, the placements it can reach
//! and the search that picks between them. Built from the open literature, since no Puyo cpu
//! has been decompiled into anything readable:
//!
//! * [ama](https://github.com/citrus610/ama) (MIT): the evaluation terms, the quiescence search
//!   in [`quiet`] and the beam's shape.
//! * takapt's beam search, searching past the queue down invented continuations; see [`beam`].
//! * Ikeda, Tomizawa, Viennot and Tanaka, *Playing PuyoPuyo: two search algorithms for
//!   constructing chain and tactical heuristics*.
//!
//! There is no neural model: the search is the ai, and `ga puyo` trains nothing.

pub mod beam;
pub mod eval;
pub mod field;
pub mod input_sequence;
pub mod placement;
pub mod quiet;
pub mod skill;

pub mod agent;
#[cfg(all(not(test), not(any(target_os = "emscripten", target_os = "android"))))]
pub mod harness;

pub use skill::{Skill, SKILLS, SKILL_ORDER};

/// Which brain an agent thinks with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PuyoAiKind {
    /// one of the six rows of [`skill::ROWS`], by index
    Scorer(usize),
    /// drops the pair in a random column: the demo fallback when a search cannot run, and a
    /// deliberately bad player for the tests
    Placeholder,
}

impl Default for PuyoAiKind {
    fn default() -> Self {
        Self::best()
    }
}

impl PuyoAiKind {
    /// the `nth` weakest row in [`SKILL_ORDER`], which is what a difficulty picks
    pub fn nth_weakest(nth: usize) -> Self {
        Self::Scorer(SKILL_ORDER[nth.min(SKILLS - 1)])
    }

    pub fn best() -> Self {
        Self::nth_weakest(SKILLS - 1)
    }

    pub fn skill(&self) -> Option<&'static Skill> {
        match self {
            PuyoAiKind::Scorer(row) => Some(&skill::ROWS[*row % SKILLS]),
            PuyoAiKind::Placeholder => None,
        }
    }
}
