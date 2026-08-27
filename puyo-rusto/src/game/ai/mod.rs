//! Puyo Rusto's ai.
//!
//! **This is a placeholder.** Phase 2 of [the plan](../../../../docs/puyo-puyo-plan.md) put the
//! four difficulty names and the two demos on the menu because the launcher's
//! `every_mode_offers_the_same_ai_opponents_and_demos` holds every game to the same list the
//! moment it appears there - and phase 4 is where a brain that can actually play goes behind
//! them. Until then every difficulty thinks with [`PuyoAiKind::Placeholder`], which drops the
//! pair somewhere legal and nothing more.
//!
//! The shape is Dr. Rustario's: a `*AiKind` the agent dispatches on, so phase 4 adds variants
//! beside this one rather than rewriting the seam.

pub mod agent;

/// Which brain an agent thinks with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PuyoAiKind {
    /// drops the pair in a column picked at random. It is not trying to play well; it is
    /// standing in for a brain until phase 4 writes one
    #[default]
    Placeholder,
}
