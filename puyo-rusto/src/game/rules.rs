//! The dials: how many colours a match deals, how fast puyos fall, and how long a stage is.

use crate::game::ai::{PuyoAiKind, SKILLS};
pub use engine::session::MatchRules;
use std::time::Duration;
use strum::IntoEnumIterator;

/// How hard the match is, in the game's own terms.
///
/// Puyo Nexus, [Tsu (rule)](https://puyonexus.com/wiki/Tsu_(rule)): the difficulty setting is
/// what decides "how many colors the player will receive and how many Garbage Puyos they will
/// start out with on their field". Five colours is a much harder game than three, because a
/// chain needs its colours to come back round.
///
/// The colour count is fixed for a whole match and is **never** driven by
/// [`crate::game::Game::speed_index`]. Stages advance per player while the pair pool is dealt
/// from one shared seed, so a colour count that changed part way through would deal the player
/// who got there first a colour the other is not drawing yet - and from then on the two would
/// be playing different games. `speed_index` may change how a game feels, never what it deals.
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

/// How many puyos have to be cleared to finish a stage.
///
/// Puyo has no natural level, so a stage is a speed step: play carries on seamlessly and the
/// pairs simply start falling faster. Roughly seven or eight groups' worth, which is a
/// comparable stretch of play to Rustris's ten lines.
pub const PUYOS_PER_STAGE: u32 = 30;

/// the fastest a pair falls of its own accord
pub const MIN_FALL_DELAY: Duration = Duration::from_millis(90);

/// How long a pair takes to fall one row at each speed step.
///
/// Past the end of the table it stays at [`MIN_FALL_DELAY`]; a stage beyond that changes
/// nothing but the number on the HUD, which is the same shape Rustris's own curve has.
pub const FALL_DELAY_MS: [u64; 12] = [800, 700, 600, 520, 450, 380, 320, 260, 210, 170, 130, 100];

/// How long a pair takes to fall one row while the player holds soft drop.
///
/// Puyo Nexus, [Soft Drop](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop): soft drop speed
/// is *hardcoded* at half a cell per frame - two frames a row, whatever the speed step - and the
/// original does not even read the pad once regular gravity is faster than that. So it is a
/// constant here and not a divisor of [`fall_delay`]: dividing made soft drop run away with the
/// speed steps, reaching four times the original's rate by the bottom of [`FALL_DELAY_MS`] and
/// crossing the whole board inside two frames, which is a hard drop and not a soft one.
///
/// Five frames rather than the original's two, which is the one place here that does not take
/// the source's number, for two reasons. The original has no hard drop, so *its* soft drop is
/// the fast way down and is nearly one; this game has both, and a soft drop that races the hard
/// drop leaves the hard drop nothing to be. And it slides a pair eight pixels a frame *within*
/// its cell, so two frames a row is continuous motion - this game interpolates now
/// ([`engine::game::Game::fall_progress`]) but the board under it is still a grid.
///
/// Five is not a taste: it is the other two games measured and matched, and what is matched is
/// how long a soft drop takes to cross **the whole board**, not the rate per row. That
/// distinction is the whole of it, because these boards are not the same height - Rustris is 20
/// rows and Dr. Rustario's bottle 16 against this game's [`crate::game::board::VISIBLE_ROWS`]. At the speed
/// each game starts on:
///
/// | game | per row | the whole board |
/// |--|--|--|
/// | Rustris, level 0 | 50 ms | 1.00 s |
/// | Dr. Rustario, low | 67 ms | 1.07 s |
/// | this, at 83 ms | 83 ms | 1.00 s |
///
/// Matching the per-row rate instead is what 50 ms was, and over twelve rows it crossed in
/// 0.6s - half again as fast as either of the others, which is what it felt like.
///
/// The other two divide their gravity by 20 and floor it, so their soft drop quickens as the
/// level does. This does not, because this game's gravity curve is far flatter than theirs
/// (800 ms down to 100, against Rustris's 1000 down to 7): the same divisor here bottoms out
/// at 5 ms, and a constant is what the original has anyway.
///
/// Taken as the faster of this and gravity, so holding down can only ever hurry a pair along
/// and never hold it up.
pub const SOFT_DROP_DELAY: Duration = Duration::from_millis(83);

/// how long a resting pair may still be nudged about before it locks
pub const LOCK_DELAY: Duration = Duration::from_millis(400);

/// the pause on a popped group before it disappears, so a chain can be watched
pub const POP_DELAY: Duration = Duration::from_millis(280);

/// the pause while loose puyos fall, after a lock or between chain steps
pub const SETTLE_DELAY: Duration = Duration::from_millis(120);

/// the pause after the queue has emptied onto the board, before the next pair
pub const NUISANCE_DELAY: Duration = Duration::from_millis(150);

/// How fast nuisance falls in from over the top of the board, in rows a second.
///
/// It is drawn falling rather than simply appearing (see
/// [`engine::render::GameRender::attack_fall_speed`]), which is the one moment of this game
/// the player has no say in and so the one that most needs to be *seen*: a whole rock landing
/// is five rows arriving at once, and where they land decides what is left of the chain
/// underneath. The whole board - a rock landing in an empty well, the longest fall there is -
/// takes a little under a second at this rate, which is still quicker than a pair falls at any
/// speed step and slow enough to watch it arrive. A drop onto a stack is shorter in proportion,
/// so the pause this costs is only ever as long as the drop deserves.
pub const NUISANCE_FALL_ROWS_PER_SECOND: f64 = 14.0;

/// the pause before a new pair appears
pub const SPAWN_DELAY: Duration = Duration::from_millis(120);

/// how long one row of gravity takes at this speed step
pub fn fall_delay(speed_index: u32) -> Duration {
    FALL_DELAY_MS
        .get(speed_index as usize)
        .map(|ms| Duration::from_millis(*ms))
        .unwrap_or(MIN_FALL_DELAY)
}

/// The starting speed step the menu offers, which is what a Puyo "level" is.
pub const MAX_START_LEVEL: u32 = 9;

/// the biggest speed step the HUD ever has to show, so it can size the digits
pub const MAX_LEVEL: u32 = 99;

/// the biggest score the HUD ever has to show
pub const MAX_SCORE: u32 = 9_999_999;

/// Which themes a match runs through.
///
/// Phase 2 of the plan gives Puyo one theme; the retro ones join this list as they are built,
/// and `all` is what runs through every one of them in the order
/// [`crate::theme::all_themes`] builds them.
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

    /// how many themes there are, which is `all` less itself
    pub fn count() -> usize {
        Self::iter().filter(|i| *i as usize > 0).count()
    }

    /// The theme every player starts on: an index into [`crate::theme::all_themes`], which
    /// is in the same order this enum is less `all`.
    ///
    /// `options.rs::theme_mode` reads this rather than matching the variants itself, which
    /// is a difference from the other two games - so a new theme is an arm here, an entry in
    /// `all_themes` and nothing else.
    pub fn initial_index(&self) -> usize {
        match self {
            MatchThemes::All | MatchThemes::Genesis => 0,
            MatchThemes::Snes => 1,
            MatchThemes::Particle => 2,
        }
    }
}

/// How well the ai plays, under the four names every game in the compendium offers.
///
/// The names and the key delays are the other games' exactly - see
/// `ai_difficulties_agree` in the launcher, which holds every game to the same four. What is
/// behind them is not yet: phase 4 of the plan replaces the placeholder brain with one that
/// can play, and nothing on this menu surface changes when it does.
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

    /// The brain this difficulty thinks with: one of the six rows of
    /// [`crate::game::ai::skill`], picked out of the *measured* ranking, so a harder setting
    /// is a better player as well as a faster one. Dr. Rustario's four difficulties pick out
    /// of its six N64 skill rows exactly this way.
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
    /// the game's own five settings: how many colours, and how buried you start
    pub difficulty: Difficulty,
    /// the speed step play opens on
    pub level: u32,
    pub rules: MatchRules,
    pub themes: MatchThemes,
    /// which track a match is played on; `None` deals one at random, which is the default and
    /// what the menu calls it
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

    /// how many boards the match runs, which the ai mode may decide instead of the dial
    pub fn effective_players(&self) -> u32 {
        match self.ai {
            AiMode::Off => self.players,
            AiMode::Demo => 1,
            AiMode::VsDemo | AiMode::Opponent(_) => 2,
        }
    }

    /// the ai controlled players (0-indexed), the key delay they play at and the brain they
    /// think with
    pub fn ai_players(&self) -> Vec<(u32, Duration, PuyoAiKind)> {
        match self.ai {
            AiMode::Off => vec![],
            AiMode::Demo => vec![(0, Duration::ZERO, PuyoAiKind::best())],
            // the two best rows against each other, the way the other two games field their
            // second best against their best
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

    /// the sourced difficulty table: three colours at the bottom, five at the top, with two
    /// rows of nuisance on the two settings that start you buried
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

    /// the colour count only ever goes up with difficulty, never down
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

    /// A soft drop crosses the whole board in about a second, which is what the other two games
    /// take to cross theirs at the speed they start on - and is why this number is not either
    /// of *their* per-row rates, their boards being taller. See [`SOFT_DROP_DELAY`].
    #[test]
    fn a_soft_drop_crosses_the_board_at_the_pace_the_other_games_do() {
        let crossing = SOFT_DROP_DELAY * crate::game::board::VISIBLE_ROWS;
        assert!(
            (Duration::from_millis(900)..=Duration::from_millis(1100)).contains(&crossing),
            "a soft drop crosses the board in {crossing:?}, and the other two games take ~1s"
        );
    }

    /// soft drop is one rate for the whole game and gravity is what eventually catches it up,
    /// which is the way round the original has it - see [`SOFT_DROP_DELAY`]
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
