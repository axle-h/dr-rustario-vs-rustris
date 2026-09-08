//! Timings, the difficulty settings and the stage ladder.
//!
//! Two of these numbers are the game's own and are marked as such; the rest are the
//! compendium's, because the PlayStation game has no notion of a speed ladder and this one
//! does. Where a value is ours it is chosen against the other three games rather than invented
//! - a pair here falls at the same rate a Puyo pair does, because they are the same piece.

use crate::game::counter::Fighter;
use engine::animate::nuisance::NuisanceFall;
use std::time::Duration;

/// **The game's own.** `+0x21e` is set to `0x28` - forty frames at sixty - whenever a pass
/// marks anything, and the chain loop waits it out before erasing and looking again. It is the
/// break animation, and it is why a long chain in this game is something you watch rather than
/// something that happens.
pub const ERASE_DELAY: Duration = Duration::from_nanos(40 * 1_000_000_000 / 60);

/// how long the board rests between a settle and the next look
pub const SETTLE_DELAY: Duration = Duration::from_millis(120);

/// how long counter gems take to arrive, once a break has finished with them
pub const DELIVERY_DELAY: Duration = Duration::from_millis(150);

pub const SPAWN_DELAY: Duration = Duration::from_millis(120);

/// how long a pair rests on the stack before it locks
pub const LOCK_DELAY: Duration = Duration::from_millis(400);

pub const SOFT_DROP_DELAY: Duration = Duration::from_millis(83);

/// Gravity, per speed step. Puyo Rusto's ladder, because a pair of gems and a pair of puyos
/// are the same piece falling and there is no reason for them to disagree.
pub const FALL_DELAY_MS: [u64; 12] = [800, 700, 600, 520, 450, 380, 320, 260, 210, 170, 130, 100];

pub const MIN_FALL_DELAY: Duration = Duration::from_millis(90);

pub fn fall_delay(speed_index: u32) -> Duration {
    FALL_DELAY_MS
        .get(speed_index as usize)
        .map(|ms| Duration::from_millis(*ms))
        .unwrap_or(MIN_FALL_DELAY)
}

/// how many gems have to be destroyed to earn a speed step
pub const GEMS_PER_STAGE: u32 = 40;

pub const MAX_START_LEVEL: u32 = 9;
pub const MAX_LEVEL: u32 = 99;
pub const MAX_SCORE: u32 = 9_999_999;

/// **The game's own.** A round's clock stops at 539 seconds, and every 30 seconds past 75 is
/// another tenth of every attack - see [`crate::game::score::time_tier`].
pub const MAX_ROUND_SECONDS: u32 = crate::game::score::MAX_ROUND_SECONDS;

/// Counter gems arrive all at once, so they are worth watching land.
///
/// A tray that has been filling while you built is the moment the player has been waiting for,
/// and Puyo Rusto answers this for the same reason. See
/// [`engine::render::GameRender::attack_fall`].
pub const COUNTER_FALL: NuisanceFall = NuisanceFall {
    initial_speed: 7.0,
    acceleration: 26.0,
    max_speed: 26.0,
    column_jitter: Duration::from_millis(60),
};

/// How hard the ai plays and how buried you start.
///
/// The level itself is the game's `L`, which scales every attack in the round - see
/// [`crate::game::score::Level`] - so a harder setting is not only a better opponent but a
/// heavier one.
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

    /// the game's own difficulty setting, which scales every attack
    pub fn level(&self) -> crate::game::score::Level {
        match self {
            Difficulty::Easy => crate::game::score::Level::Easy,
            Difficulty::Normal => crate::game::score::Level::Normal,
            Difficulty::Hard => crate::game::score::Level::Hard,
        }
    }

    /// how many rows of counter gems the board starts buried under
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

/// Which theme a match runs on.
///
/// One, and it is the arcade one - see `crate::theme`, and the plan, which records that as a
/// decision rather than a stage. There is no `all` here because there is nothing to run
/// through: with one theme a theme sprint is a one-stage sprint under another name, and
/// [`MatchRules::modes`] leaves it off the menu on exactly that count.
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

    /// the theme every player starts on, as an index into [`crate::theme::all_themes`]
    pub fn initial_index(&self) -> usize {
        0
    }
}

/// The match options Super Rustle Fighter offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameConfig {
    pub players: u32,
    /// who you are playing as, which decides the pattern **your garbage arrives in on the
    /// other board** - see [`crate::game::counter::Fighter`]. It is the one option here that
    /// changes what the opponent has to deal with rather than what you do.
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

    /// how many boards the match runs
    pub fn effective_players(&self) -> u32 {
        self.players
    }

    /// **No ai fields this game yet**, so no player is ever one; phase 3 of the plan is where
    /// that changes. It is answered rather than left out because the launcher asks every game
    /// the same question.
    pub fn is_ai_player(&self, _player: u32) -> bool {
        false
    }
}

impl Default for GameConfig {
    fn default() -> GameConfig {
        GameConfig::new(1, 0, MatchRules::Marathon)
    }
}
