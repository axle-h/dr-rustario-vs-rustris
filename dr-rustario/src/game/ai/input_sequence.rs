//! The keys the agent presses to reach a placement.

use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Translation {
    Left,
    Right,
    RotateClockwise,
    RotateAnticlockwise,
    HardDrop,
    /// Not a key at all: **stop pressing and let the pill fall until it comes to rest**, which
    /// is the first half of a tuck. What follows it is a move made in the lock delay, and it is
    /// a waypoint rather than a row so that nothing has to be timed - the pill cannot fall past
    /// where it comes to rest, and if garbage arrives while it falls and it rests somewhere
    /// else, the plan carries on from wherever it actually is rather than waiting for a row
    /// that is never coming.
    Rest,
    /// swap the pill in play for the one being held. Everything after it in the sequence
    /// belongs to the pill the swap brings in.
    Hold,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputSequence(Vec<Translation>);

impl InputSequence {
    pub fn new(translations: Vec<Translation>) -> Self {
        Self(translations)
    }

    pub fn with(&self, translation: Translation) -> Self {
        let mut translations = self.0.clone();
        translations.push(translation);
        Self(translations)
    }

    pub fn translations(&self) -> &[Translation] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl IntoIterator for InputSequence {
    type Item = Translation;
    type IntoIter = std::vec::IntoIter<Translation>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// shorter sequences first, so a tie between two placements goes to the simpler one, and
/// identical lengths still order deterministically
impl Ord for InputSequence {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.key().cmp(&other.key()))
    }
}

impl PartialOrd for InputSequence {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl InputSequence {
    fn key(&self) -> Vec<u8> {
        self.0
            .iter()
            .map(|t| match t {
                Translation::Left => 0,
                Translation::Right => 1,
                Translation::RotateClockwise => 2,
                Translation::RotateAnticlockwise => 3,
                Translation::HardDrop => 4,
                Translation::Rest => 5,
                Translation::Hold => 6,
            })
            .collect()
    }
}
