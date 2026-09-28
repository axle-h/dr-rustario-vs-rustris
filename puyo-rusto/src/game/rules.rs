//! The dials: how many colours a match deals, how fast puyos fall, and how long a stage is.

use crate::game::ai::{PuyoAiKind, SKILLS};
use engine::animate::nuisance::NuisanceFall;
pub use engine::session::MatchRules;
use std::time::Duration;
use strum::IntoEnumIterator;

/// How hard the match is: how many colours are dealt and how much nuisance you start with.
/// Puyo Nexus, [Tsu (rule)](https://puyonexus.com/wiki/Tsu_(rule)).
///
/// The colour count is fixed for the whole match and never driven by
/// `speed_index`: players reach stages at different times while sharing one
/// seed, so a change part way through would deal them different games.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Difficulty {
    VeryEasy,
    Easy,
    #[default]
    Normal,
    Hard,
    VeryHard,
}

impl Difficulty {
    pub const ALL: [Difficulty; 5] = [
        Difficulty::VeryEasy,
        Difficulty::Easy,
        Difficulty::Normal,
        Difficulty::Hard,
        Difficulty::VeryHard,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Difficulty::VeryEasy => "very easy",
            Difficulty::Easy => "easy",
            Difficulty::Normal => "normal",
            Difficulty::Hard => "hard",
            Difficulty::VeryHard => "very hard",
        }
    }

    pub fn from_name(name: &str) -> Option<Difficulty> {
        Difficulty::ALL.into_iter().find(|d| d.name() == name)
    }

    /// how many of the five colours this match deals
    pub fn colors(&self) -> usize {
        match self {
            Difficulty::VeryEasy | Difficulty::Easy => 3,
            Difficulty::Normal => 4,
            Difficulty::Hard | Difficulty::VeryHard => 5,
        }
    }

    /// rows of nuisance already on the board when the match starts
    pub fn starting_nuisance_rows(&self) -> u32 {
        match self {
            Difficulty::Easy | Difficulty::VeryHard => 2,
            _ => 0,
        }
    }

    /// the very hard setting also drops puyos a little faster from the off
    pub fn speed_bonus(&self) -> u32 {
        match self {
            Difficulty::VeryHard => 2,
            _ => 0,
        }
    }
}

/// How many puyos have to be cleared to finish a stage, which is a speed step; about Rustris's
/// ten lines of play.
pub const PUYOS_PER_STAGE: u32 = 30;

/// the fastest a pair falls of its own accord
pub const MIN_FALL_DELAY: Duration = Duration::from_millis(90);

/// How long a pair takes to fall one row at each speed step, then [`MIN_FALL_DELAY`].
pub const FALL_DELAY_MS: [u64; 12] = [800, 700, 600, 520, 450, 380, 320, 260, 210, 170, 130, 100];

/// How long a pair takes to fall one row under soft drop, taken as the faster of this and
/// gravity. A constant at every speed step as in the original (Puyo Nexus,
/// [Soft Drop](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop)), but slower than its two
/// frames so that it crosses the whole board in about a second like the other two games; pinned
/// by `a_soft_drop_crosses_the_board_at_the_pace_the_other_games_do`.
pub const SOFT_DROP_DELAY: Duration = Duration::from_millis(83);

/// how long a resting pair may still be nudged about before it locks
pub const LOCK_DELAY: Duration = Duration::from_millis(400);

/// The rules' own pause on a popped group, the floor under a chain step: a theme's destroy
/// animation adds to it, since the match screen skips `game.update` while one blocks.
pub const POP_DELAY: Duration = Duration::from_millis(90);

/// the pause while loose puyos fall, after a lock or between chain steps
pub const SETTLE_DELAY: Duration = Duration::from_millis(120);

/// the pause after the queue has emptied onto the board, before the next pair
pub const NUISANCE_DELAY: Duration = Duration::from_millis(150);

/// How nuisance falls in from over the top of the board: under gravity, with each column held
/// back a little so the row breaks up, taking just over a second for the longest fall.
pub const NUISANCE_FALL: NuisanceFall = NuisanceFall {
    initial_speed: 7.0,
    acceleration: 26.0,
    max_speed: 26.0,
    column_jitter: Duration::from_millis(60),
};

/// the pause before a new pair appears
pub const SPAWN_DELAY: Duration = Duration::from_millis(120);

/// how long one row of gravity takes at this speed step
pub fn fall_delay(speed_index: u32) -> Duration {
    FALL_DELAY_MS
        .get(speed_index as usize)
        .map(|ms| Duration::from_millis(*ms))
        .unwrap_or(MIN_FALL_DELAY)
}

/// The starting speed step the menu offers, which is what a Puyo level is.
pub const MAX_START_LEVEL: u32 = 9;

/// the biggest speed step the HUD ever has to show, so it can size the digits
pub const MAX_LEVEL: u32 = 99;

/// the biggest score the HUD ever has to show
pub const MAX_SCORE: u32 = 9_999_999;

/// Which themes a match runs through; `all` runs every one in [`crate::theme::all_themes`]'s
/// order.
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
    /// run every theme in order, switching at the next level
    #[default]
    #[strum(serialize = "all")]
    All,
    #[strum(serialize = "genesis")]
    Genesis,
    #[strum(serialize = "snes")]
    Snes,
    #[strum(serialize = "particle")]
    Particle,
}

impl MatchThemes {
    pub fn names() -> Vec<&'static str> {
        Self::iter().map(|e| e.into()).collect()
    }

    pub fn count() -> usize {
        Self::iter().filter(|i| *i as usize > 0).count()
    }

    /// The theme every player starts on: an index into [`crate::theme::all_themes`], which is
    /// in this enum's order less `all`. `options.rs` reads this rather than matching variants.
    pub fn initial_index(&self) -> usize {
        match self {
            MatchThemes::All | MatchThemes::Genesis => 0,
            MatchThemes::Snes => 1,
            MatchThemes::Particle => 2,
        }
    }
}

/// How well the ai plays, under the four names and key delays every game in the compendium
/// shares (`ai_difficulties_agree` in the launcher); each picks a row of
/// [`crate::game::ai::skill::SKILL_ORDER`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiDifficulty {
    Easy,
    Normal,
    Hard,
    Impossible,
}

impl AiDifficulty {
    pub const ALL: [Self; 4] = [Self::Easy, Self::Normal, Self::Hard, Self::Impossible];
    pub const EASY_KEY_DELAY: Duration = Duration::from_millis(500);
    pub const NORMAL_KEY_DELAY: Duration = Duration::from_millis(400);
    pub const HARD_KEY_DELAY: Duration = Duration::from_millis(300);

    pub fn name(&self) -> &'static str {
        match self {
            AiDifficulty::Easy => "easy",
            AiDifficulty::Normal => "normal",
            AiDifficulty::Hard => "hard",
            AiDifficulty::Impossible => "impossible",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.name() == name)
    }

    /// the shortest time the agent may leave between two key presses
    pub fn key_delay(&self) -> Duration {
        match self {
            AiDifficulty::Easy => Self::EASY_KEY_DELAY,
            AiDifficulty::Normal => Self::NORMAL_KEY_DELAY,
            AiDifficulty::Hard => Self::HARD_KEY_DELAY,
            AiDifficulty::Impossible => Duration::ZERO,
        }
    }

    /// The brain this difficulty thinks with: a row of the ranked
    /// [`crate::game::ai::skill::SKILL_ORDER`], so a harder setting is a better player too.
    pub fn brain(&self) -> PuyoAiKind {
        match self {
            AiDifficulty::Easy => PuyoAiKind::nth_weakest(0),
            AiDifficulty::Normal => PuyoAiKind::nth_weakest(1),
            AiDifficulty::Hard => PuyoAiKind::nth_weakest(SKILLS - 2),
            AiDifficulty::Impossible => PuyoAiKind::nth_weakest(SKILLS - 1),
        }
    }
}

/// Who is playing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AiMode {
    /// no ai players
    #[default]
    Off,
    /// one board, played by the ai at full speed
    Demo,
    /// both boards played by the ai at full speed
    VsDemo,
    /// two players, the second of them the ai
    Opponent(AiDifficulty),
}

impl AiMode {
    /// nobody is playing: every board is the ai's, at full speed
    pub fn is_demo(&self) -> bool {
        matches!(self, AiMode::Demo | AiMode::VsDemo)
    }
}

/// The match options Puyo Rusto offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameConfig {
    pub players: u32,
    /// how many colours are dealt and how buried you start
    pub difficulty: Difficulty,
    /// the speed step play opens on
    pub level: u32,
    pub rules: MatchRules,
    pub themes: MatchThemes,
    /// who the ai plays
    pub ai: AiMode,
}

impl GameConfig {
    pub fn new(players: u32, level: u32, rules: MatchRules, themes: MatchThemes) -> Self {
        Self {
            players,
            difficulty: Difficulty::default(),
            level,
            rules,
            themes,
            ai: AiMode::Off,
        }
    }

    /// how many boards the match runs; the ai mode may decide instead of the dial
    pub fn effective_players(&self) -> u32 {
        match self.ai {
            AiMode::Off => self.players,
            AiMode::Demo => 1,
            AiMode::VsDemo | AiMode::Opponent(_) => 2,
        }
    }

    /// the ai players (0-indexed), with their key delay and brain
    pub fn ai_players(&self) -> Vec<(u32, Duration, PuyoAiKind)> {
        match self.ai {
            AiMode::Off => vec![],
            AiMode::Demo => vec![(0, Duration::ZERO, PuyoAiKind::best())],
            // the two best rows against each other
            AiMode::VsDemo => vec![
                (0, Duration::ZERO, PuyoAiKind::nth_weakest(SKILLS - 2)),
                (1, Duration::ZERO, PuyoAiKind::nth_weakest(SKILLS - 1)),
            ],
            AiMode::Opponent(difficulty) => {
                vec![(1, difficulty.key_delay(), difficulty.brain())]
            }
        }
    }

    pub fn is_ai_player(&self, player: u32) -> bool {
        self.ai_players().iter().any(|(p, _, _)| *p == player)
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        Self::new(1, 0, MatchRules::Marathon, MatchThemes::All)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// three colours at the bottom, five at the top, and two starting nuisance rows on the two
    /// buried settings
    #[test]
    fn difficulty_sets_the_colour_count() {
        assert_eq!(Difficulty::VeryEasy.colors(), 3);
        assert_eq!(Difficulty::Easy.colors(), 3);
        assert_eq!(Difficulty::Normal.colors(), 4);
        assert_eq!(Difficulty::Hard.colors(), 5);
        assert_eq!(Difficulty::VeryHard.colors(), 5);
        assert_eq!(Difficulty::default(), Difficulty::Normal);
    }

    #[test]
    fn two_of_the_settings_start_you_buried() {
        assert_eq!(Difficulty::Easy.starting_nuisance_rows(), 2);
        assert_eq!(Difficulty::VeryHard.starting_nuisance_rows(), 2);
        for difficulty in [Difficulty::VeryEasy, Difficulty::Normal, Difficulty::Hard] {
            assert_eq!(difficulty.starting_nuisance_rows(), 0);
        }
    }

    #[test]
    fn the_hardest_setting_also_drops_faster() {
        assert_eq!(Difficulty::VeryHard.speed_bonus(), 2);
        assert_eq!(Difficulty::Hard.speed_bonus(), 0);
    }

    #[test]
    fn every_difficulty_is_named_and_found_by_name() {
        for difficulty in Difficulty::ALL {
            assert_eq!(Difficulty::from_name(difficulty.name()), Some(difficulty));
        }
        assert_eq!(Difficulty::from_name("impossible"), None);
    }

    #[test]
    fn harder_never_means_fewer_colours() {
        for pair in Difficulty::ALL.windows(2) {
            assert!(pair[0].colors() <= pair[1].colors());
        }
    }

    #[test]
    fn pairs_fall_faster_at_every_step_and_then_level_off() {
        for step in 1..FALL_DELAY_MS.len() as u32 {
            assert!(
                fall_delay(step) < fall_delay(step - 1),
                "step {step} is not faster than the one before"
            );
        }
        assert_eq!(fall_delay(FALL_DELAY_MS.len() as u32), MIN_FALL_DELAY);
        assert_eq!(fall_delay(9999), MIN_FALL_DELAY);
    }

    /// a soft drop crosses the board in about a second, as the other two games' do
    #[test]
    fn a_soft_drop_crosses_the_board_at_the_pace_the_other_games_do() {
        let crossing = SOFT_DROP_DELAY * crate::game::board::VISIBLE_ROWS;
        assert!(
            (Duration::from_millis(900)..=Duration::from_millis(1100)).contains(&crossing),
            "a soft drop crosses the board in {crossing:?}, and the other two games take ~1s"
        );
    }

    /// soft drop is one rate at every step, and gravity eventually overtakes it
    #[test]
    fn soft_drop_is_the_same_speed_at_every_step() {
        for step in 0..FALL_DELAY_MS.len() as u32 + 2 {
            let soft = fall_delay(step).min(SOFT_DROP_DELAY);
            assert!(
                soft <= fall_delay(step),
                "step {step}: soft drop is slower than gravity"
            );
            assert!(
                soft >= SOFT_DROP_DELAY.min(MIN_FALL_DELAY),
                "step {step}: soft drop has run away from its own rate"
            );
        }
    }
}
