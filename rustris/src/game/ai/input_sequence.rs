use crate::game::geometry::{Point, Pose, Rotation};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::ops::{Add, Deref};
use strum_macros::{Display, EnumString};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InputSequence(pub Vec<Translation>);

impl Deref for InputSequence {
    type Target = Vec<Translation>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl InputSequence {
    pub fn empty() -> Self {
        Self(vec![])
    }

    pub fn from_slice(seq: &[Translation]) -> Self {
        Self(seq.to_vec())
    }

    pub fn split_at_soft_drop(&self) -> (Self, Option<Self>) {
        if let Some(i) = self.0.iter().position(|&t| t == Translation::SoftDrop) {
            let before = Self::from_slice(&self.0[..i]);
            let after = if i + 1 < self.0.len() {
                Self::from_slice(&self.0[i + 1..])
            } else {
                Self::empty()
            };
            (before, Some(after))
        } else {
            // no soft drop
            (self.clone(), None)
        }
    }

    pub fn new(sequence: Vec<Translation>) -> Self {
        Self(sequence)
    }

    pub fn translations(&self) -> &[Translation] {
        &self.0
    }

    pub fn pop_drop(&mut self) -> Option<Translation> {
        if let Some(last) = self.0.last() {
            if last.is_drop() {
                // If the last translation is a SoftDrop, we pop it
                self.0.pop()
            } else {
                // If the last translation is not a SoftDrop, we return None
                None
            }
        } else {
            None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn count_soft_drops(&self) -> usize {
        self.0.iter().filter(|t| t.is_soft_drop()).count()
    }

    pub fn push(&mut self, translation: Translation) {
        if let Some(last) = self.0.last() {
            if *last == translation.opposite().unwrap_or(translation) {
                // If the translation is the opposite of the last one, remove the last one
                self.0.pop();
            } else {
                self.0.push(translation);
            }
        } else {
            self.0.push(translation);
        }
    }

    pub fn resolve(&self) -> ResolvedInputSequence {
        let mut translate = 0;
        let mut rotation = Rotation::North;
        for translation in self.0.iter() {
            match translation {
                Translation::Left => translate -= 1,
                Translation::Right => translate += 1,
                Translation::RotateClockwise => rotation = rotation.rotate(true),
                Translation::RotateAnticlockwise => rotation = rotation.rotate(false),
                _ => {}
            }
        }
        ResolvedInputSequence {
            rotation,
            translate,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ResolvedInputSequence {
    rotation: Rotation,
    translate: i32,
}

impl ResolvedInputSequence {
    pub fn into_sequence(self) -> InputSequence {
        let mut seq = vec![];
        let translation = if self.translate > 0 {
            Translation::Right
        } else {
            Translation::Left
        };
        for _ in 0..self.translate.abs() {
            seq.push(translation);
        }
        match self.rotation {
            Rotation::North => {}
            Rotation::East => {
                seq.push(Translation::RotateClockwise);
            }
            Rotation::South => {
                seq.push(Translation::RotateClockwise);
                seq.push(Translation::RotateClockwise);
            }
            Rotation::West => {
                seq.push(Translation::RotateAnticlockwise);
            }
        }
        InputSequence(seq)
    }
}

impl Add<Translation> for ResolvedInputSequence {
    type Output = Self;

    fn add(self, rhs: Translation) -> Self::Output {
        match rhs {
            Translation::Left => Self {
                translate: self.translate - 1,
                ..self
            },
            Translation::Right => Self {
                translate: self.translate + 1,
                ..self
            },
            Translation::RotateClockwise => Self {
                rotation: self.rotation.rotate(true),
                ..self
            },
            Translation::RotateAnticlockwise => Self {
                rotation: self.rotation.rotate(false),
                ..self
            },
            _ => self,
        }
    }
}

impl Add<Translation> for InputSequence {
    type Output = Self;

    fn add(mut self, rhs: Translation) -> Self::Output {
        self.push(rhs);
        self
    }
}

impl Default for InputSequence {
    fn default() -> Self {
        Self::empty()
    }
}

impl PartialOrd for InputSequence {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for InputSequence {
    fn cmp(&self, other: &Self) -> Ordering {
        // consider a sequence with a soft drop "longer" than one without
        self.count_soft_drops()
            .cmp(&other.count_soft_drops())
            .then(self.0.len().cmp(&other.0.len()))
            .then(self.0.cmp(&other.0))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, EnumString, Display)]
pub enum Translation {
    Left = 1,
    Right = 2,
    RotateClockwise = 3,
    RotateAnticlockwise = 4,
    HardDrop = 5,
    SoftDrop = 6,
}

impl Translation {
    pub fn into_index(self) -> usize {
        self as usize
    }

    pub fn opposite(self) -> Option<Self> {
        match self {
            Translation::Left => Some(Translation::Right),
            Translation::Right => Some(Translation::Left),
            Translation::RotateClockwise => Some(Translation::RotateAnticlockwise),
            Translation::RotateAnticlockwise => Some(Translation::RotateClockwise),
            _ => None, // HardDrop and SoftDrop do not have opposites
        }
    }

    pub fn apply(self, pose: Pose) -> Pose {
        match self {
            Translation::Left => pose + Point::new(-1, 0),
            Translation::Right => pose + Point::new(1, 0),
            Translation::RotateClockwise => pose.rotate(true),
            Translation::RotateAnticlockwise => pose.rotate(false),
            Translation::HardDrop | Translation::SoftDrop => {
                panic!("undefined translation for pose")
            }
        }
    }

    pub fn is_drop(self) -> bool {
        matches!(self, Translation::HardDrop | Translation::SoftDrop)
    }

    pub fn is_soft_drop(self) -> bool {
        matches!(self, Translation::SoftDrop)
    }
}

impl PartialOrd for Translation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Translation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.into_index().cmp(&other.into_index())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Translation::{Left, Right, RotateAnticlockwise, RotateClockwise, SoftDrop};

    #[test]
    fn rotates_clockwise() {
        let seq = InputSequence::new(vec![RotateClockwise, RotateClockwise]).resolve();
        assert_eq!(
            seq,
            ResolvedInputSequence {
                translate: 0,
                rotation: Rotation::South
            }
        );
    }

    #[test]
    fn rotates_anticlockwise() {
        let seq = InputSequence::new(vec![RotateAnticlockwise, RotateAnticlockwise]).resolve();
        assert_eq!(
            seq,
            ResolvedInputSequence {
                translate: 0,
                rotation: Rotation::South
            }
        );
    }

    #[test]
    fn rotates_360() {
        let seq = InputSequence::new(vec![
            RotateClockwise,
            RotateClockwise,
            RotateClockwise,
            RotateClockwise,
        ])
        .resolve();

        assert_eq!(
            seq,
            ResolvedInputSequence {
                translate: 0,
                rotation: Rotation::North
            }
        );
    }

    #[test]
    fn left() {
        let seq = InputSequence::new(vec![Left, Left]).resolve();
        assert_eq!(
            seq,
            ResolvedInputSequence {
                translate: -2,
                rotation: Rotation::North
            }
        );
    }

    #[test]
    fn right() {
        let seq = InputSequence::new(vec![Right, Right]).resolve();
        assert_eq!(
            seq,
            ResolvedInputSequence {
                translate: 2,
                rotation: Rotation::North
            }
        );
    }

    #[test]
    fn deduplicates_movement() {
        let seq = InputSequence::new(vec![Left, Left, Right]).resolve();
        assert_eq!(
            seq,
            ResolvedInputSequence {
                translate: -1,
                rotation: Rotation::North
            }
        );
    }

    #[test]
    fn split_at_soft_drop_nop() {
        let seq = InputSequence::new(vec![Left, Left, Right]);

        let (before, after) = seq.split_at_soft_drop();
        assert_eq!(before, InputSequence::from_slice(&[Left, Left, Right]));
        assert_eq!(after, None);
    }

    #[test]
    fn split_at_soft_drop_at_end() {
        let seq = InputSequence::new(vec![Left, Left, Right, SoftDrop]);

        let (before, after) = seq.split_at_soft_drop();
        assert_eq!(before, InputSequence::from_slice(&[Left, Left, Right]));
        assert_eq!(after, Some(InputSequence::empty()));
    }

    #[test]
    fn split_at_soft_drop_in_middle() {
        let seq = InputSequence::new(vec![Left, Left, Right, SoftDrop, RotateClockwise]);

        let (before, after) = seq.split_at_soft_drop();
        assert_eq!(before, InputSequence::from_slice(&[Left, Left, Right]));
        assert_eq!(after, Some(InputSequence::from_slice(&[RotateClockwise])));
    }

    #[test]
    fn split_at_soft_drop_multiple_times() {
        let seq = InputSequence::new(vec![
            Left,
            Left,
            Right,
            SoftDrop,
            RotateClockwise,
            SoftDrop,
            RotateAnticlockwise,
            SoftDrop,
        ]);

        let (before, after) = seq.split_at_soft_drop();
        assert_eq!(before, InputSequence::from_slice(&[Left, Left, Right]));
        assert_eq!(
            after,
            Some(InputSequence::from_slice(&[
                RotateClockwise,
                SoftDrop,
                RotateAnticlockwise,
                SoftDrop
            ]))
        );
    }
}
