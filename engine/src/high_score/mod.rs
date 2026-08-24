pub mod event;
pub mod render;
pub mod table;

use table::Ranking;

/// Which table a match competes for: a game (or the vs. playlist), the mode within it and
/// how that mode ranks its entries. `game` and `mode` structure `high_scores.yml` and title
/// the high score screens, so they are display names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HighScoreKey {
    pub game: String,
    pub mode: String,
    pub ranking: Ranking,
}

impl HighScoreKey {
    pub fn new(game: impl Into<String>, mode: impl Into<String>, ranking: Ranking) -> Self {
        Self {
            game: game.into(),
            mode: mode.into(),
            ranking,
        }
    }

    /// e.g. "Rustris, 1 level sprint, level 3"
    pub fn title(&self) -> String {
        format!("{}, {}", self.game, self.mode)
    }
}

/// A table entry waiting for a name: `score` is points or, for a sprint's best-times table,
/// milliseconds (see [`table::Ranking`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewHighScore {
    pub player: u32,
    pub score: u32,
}

impl NewHighScore {
    pub fn new(player: u32, score: u32) -> Self {
        Self { player, score }
    }
}
