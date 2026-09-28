//! Timings, the difficulty settings and the stage ladder.
//!
//! Only [`ERASE_DELAY`] and [`MAX_ROUND_SECONDS`] are the original's; the original has no speed
//! ladder.

use crate::game::counter::Fighter;
use engine::animate::nuisance::NuisanceFall;
use std::time::Duration;

/// The original's `+0x21e`: forty frames the chain loop waits between passes, for the break
/// animation.
pub const ERASE_DELAY: Duration = Duration::from_nanos(40 * 1_000_000_000 / 60);

pub const SETTLE_DELAY: Duration = Duration::from_millis(120);

pub const DELIVERY_DELAY: Duration = Duration::from_millis(150);

pub const SPAWN_DELAY: Duration = Duration::from_millis(120);

pub const LOCK_DELAY: Duration = Duration::from_millis(400);

pub const SOFT_DROP_DELAY: Duration = Duration::from_millis(83);

/// Gravity per speed step, matching Puyo Rusto's since it is the same piece.
pub const FALL_DELAY_MS: [u64; 12] = [800, 700, 600, 520, 450, 380, 320, 260, 210, 170, 130, 100];

pub const MIN_FALL_DELAY: Duration = Duration::from_millis(90);

pub fn fall_delay(speed_index: u32) -> Duration {
    FALL_DELAY_MS
        .get(speed_index as usize)
        .map(|ms| Duration::from_millis(*ms))
        .unwrap_or(MIN_FALL_DELAY)
}

/// gems destroyed per speed step
pub const GEMS_PER_STAGE: u32 = 40;

pub const MAX_START_LEVEL: u32 = 9;
pub const MAX_LEVEL: u32 = 99;
pub const MAX_SCORE: u32 = 9_999_999;

/// The original's round clock cap; see [`crate::game::score::time_tier`].
pub const MAX_ROUND_SECONDS: u32 = crate::game::score::MAX_ROUND_SECONDS;

/// How counter gems fall in; see [`engine::render::GameRender::attack_fall`].
pub const COUNTER_FALL: NuisanceFall = NuisanceFall {
    initial_speed: 7.0,
    acceleration: 26.0,
    max_speed: 26.0,
    column_jitter: Duration::from_millis(60),
};

/// How buried you start, and the [`crate::game::score::Level`] that scales every attack.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, strum::EnumIter)]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard];

    pub fn name(&self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Normal => "normal",
            Difficulty::Hard => "hard",
        }
    }

    pub fn from_name(name: &str) -> Option<Difficulty> {
        Difficulty::ALL
            .into_iter()
            .find(|d| d.name().eq_ignore_ascii_case(name))
    }

    pub fn level(&self) -> crate::game::score::Level {
        match self {
            Difficulty::Easy => crate::game::score::Level::Easy,
            Difficulty::Normal => crate::game::score::Level::Normal,
            Difficulty::Hard => crate::game::score::Level::Hard,
        }
    }

    pub fn starting_counter_rows(&self) -> u32 {
        match self {
            Difficulty::Easy | Difficulty::Normal => 0,
            Difficulty::Hard => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_erase_delay_is_forty_frames() {
        assert_eq!(ERASE_DELAY.as_millis(), 666, "forty frames at sixty");
    }

    #[test]
    fn the_fall_ladder_only_ever_speeds_up() {
        for step in 1..FALL_DELAY_MS.len() as u32 {
            assert!(fall_delay(step) < fall_delay(step - 1));
        }
        assert_eq!(fall_delay(99), MIN_FALL_DELAY, "and it bottoms out");
    }

    #[test]
    fn every_difficulty_names_itself_and_reads_back() {
        for difficulty in Difficulty::ALL {
            assert_eq!(Difficulty::from_name(difficulty.name()), Some(difficulty));
        }
        assert_eq!(Difficulty::from_name("nonsense"), None);
    }
}

pub use engine::session::MatchRules;

/// Which theme a match runs on. There is only the arcade one, so no `all`, and
/// [`MatchRules::modes`] leaves the theme sprint off the menu.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    strum::IntoStaticStr,
    strum::EnumIter,
    strum::EnumString,
)]
pub enum MatchThemes {
    #[default]
    #[strum(serialize = "arcade")]
    Arcade,
}

impl MatchThemes {
    pub fn names() -> Vec<&'static str> {
        use strum::IntoEnumIterator;
        MatchThemes::iter().map(|theme| theme.into()).collect()
    }

    pub fn count() -> usize {
        1
    }

    /// an index into [`crate::theme::all_themes`]
    pub fn initial_index(&self) -> usize {
        0
    }
}

/// The match options Super Rustle Fighter offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameConfig {
    pub players: u32,
    /// decides the pattern your garbage lands in on the other board
    pub fighter: Fighter,
    pub difficulty: Difficulty,
    /// the speed step play opens on
    pub level: u32,
    pub rules: MatchRules,
    pub themes: MatchThemes,
}

impl GameConfig {
    pub fn new(players: u32, level: u32, rules: MatchRules) -> GameConfig {
        GameConfig {
            players,
            fighter: Fighter::default(),
            difficulty: Difficulty::default(),
            level,
            rules,
            themes: MatchThemes::default(),
        }
    }

    pub fn effective_players(&self) -> u32 {
        self.players
    }

    /// Always false: this game has no ai.
    pub fn is_ai_player(&self, _player: u32) -> bool {
        false
    }
}

impl Default for GameConfig {
    fn default() -> GameConfig {
        GameConfig::new(1, 0, MatchRules::Marathon)
    }
}
