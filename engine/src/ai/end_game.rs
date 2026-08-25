//! Caps that stop a training game before it would otherwise end.

use crate::ai::game_result::GameResult;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct EndGame {
    pub score: u32,
    /// the game defined progress counter: Rustris lines, Dr. Rustario viruses
    pub cleared: u32,
    pub pieces: u32,
    pub duration: Duration,
}

impl Default for EndGame {
    fn default() -> Self {
        Self::NONE
    }
}

impl EndGame {
    pub const NONE: Self = Self {
        score: u32::MAX,
        cleared: u32::MAX,
        pieces: u32::MAX,
        duration: Duration::MAX
    };

    pub fn of_score(score: u32) -> Self {
        Self {
            score,
            ..Default::default()
        }
    }

    pub fn of_cleared(cleared: u32) -> Self {
        Self {
            cleared,
            ..Default::default()
        }
    }

    pub fn of_pieces(pieces: u32) -> Self {
        Self {
            pieces,
            ..Default::default()
        }
    }

    pub fn of_seconds(seconds: u64) -> Self {
        Self {
            duration: Duration::from_secs(seconds),
            ..Default::default()
        }
    }

    /// whether the result reached one of the phase's caps, ignoring the wall clock
    pub fn reached(&self, result: GameResult) -> bool {
        result.score() >= self.score
            || result.cleared() >= self.cleared
            || result.pieces() >= self.pieces
    }

    pub fn is_end_game(&self, result: GameResult, duration: Duration) -> bool {
        result.score() >= self.score
            || result.cleared() >= self.cleared
            || result.pieces() >= self.pieces
            || duration >= self.duration
    }
}
