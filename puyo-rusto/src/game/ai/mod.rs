//! Puyo Rusto's ai: the board it thinks on, what it thinks a board is worth, the placements it
//! can reach and the search that picks between them.
//!
//! The architecture is Dr. Rustario's - a `*AiKind` the agent dispatches on, a placement
//! search that replays real moves, and a difficulty ladder whose rows were *measured* against
//! each other rather than ordered by hand. What is behind it is not: Dr. Mario 64 was
//! decompiled and its scorer could be ported, and there is no equivalent here. Puyo Puyo's own
//! cpu opponents have never been decompiled into anything readable - Mean Bean Machine's is in
//! an unlabelled 68000 disassembly and Puyo VS's own (`Puyolib/AI.cpp`) takes the biggest chain
//! in front of it and otherwise places at random - so this one is built out of the open
//! literature instead:
//!
//! * [ama](https://github.com/citrus610/ama) (MIT), the strongest open Puyo Puyo Tsu ai, is
//!   where the evaluation terms, the quiescence search in [`quiet`] and the beam's shape come
//!   from. It is small enough to read end to end, which is why it was followed rather than
//!   [puyoai](https://github.com/puyoai/puyoai)'s hundred-feature `mayah`.
//! * takapt's beam search - searching past the queue down several invented continuations -
//!   by way of ama's six fixed ones. See [`beam`].
//! * Ikeda, Tomizawa, Viennot and Tanaka, *Playing PuyoPuyo: two search algorithms for
//!   constructing chain and tactical heuristics*, which is what both of the above cite.
//!
//! **There is no neural model here and there will not be one.** The search is the ai; nothing in
//! this module is shaped around a `Genome` and `ga puyo` trains nothing. See
//! `docs/puyo-puyo-plan.md`.

pub mod beam;
pub mod eval;
pub mod field;
pub mod input_sequence;
pub mod placement;
pub mod quiet;
pub mod skill;

pub mod agent;
#[cfg(all(not(test), not(target_os = "emscripten")))]
pub mod harness;

pub use skill::{Skill, SKILLS, SKILL_ORDER};

/// Which brain an agent thinks with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PuyoAiKind {
    /// one of the six rows of [`skill::ROWS`], by index
    Scorer(usize),
    /// drops the pair in a column picked at random. It is not trying to play well and never
    /// was; it is kept because a demo of a board doing *something* is the fallback when a
    /// search cannot run, and because the tests want a player that cannot possibly be
    /// mistaken for a good one
    Placeholder,
}

impl Default for PuyoAiKind {
    fn default() -> Self {
        Self::best()
    }
}

impl PuyoAiKind {
    /// the `nth` weakest row, as measured in [`SKILL_ORDER`], which is what a difficulty picks
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
