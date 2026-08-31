use crate::game::ai::DrAiKind;
use crate::game::random::RandomMode;
use crate::game::GameSpeed;
use std::time::Duration;
use strum::IntoEnumIterator;

pub const MAX_VIRUS_LEVEL: u32 = 30;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, strum::IntoStaticStr, strum::EnumIter, strum::EnumString,
)]
pub enum MatchThemes {
    /// Run themes in order, switching at the next level
    #[strum(serialize = "all")]
    All = 0,

    #[strum(serialize = "nes")]
    Nes = 1,

    #[strum(serialize = "snes")]
    Snes = 2,

    #[strum(serialize = "n64")]
    N64 = 3,

    #[strum(serialize = "particle")]
    Particle = 4,
}

impl MatchThemes {
    pub fn names() -> Vec<&'static str> {
        Self::iter().map(|e| e.into()).collect()
    }
    pub fn count() -> usize {
        Self::iter().filter(|i| *i as usize > 0).count()
    }
}

use crate::game::ai::models::survival_trained;
pub use engine::session::MatchRules;

/// How well and how fast the ai is allowed to play. Every difficulty plays Dr. Mario 64's own
/// deterministic opponent, but on a different one of its six rows of weights - the one dial the
/// original ai has - as well as at a different key rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiDifficulty {
    /// The most speed limited, on the weakest row of weights
    Easy,
    Normal,
    Hard,
    /// Full speed, on the best row of weights
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

    /// Minimum time between simulated key presses
    pub fn key_delay(&self) -> Duration {
        match self {
            AiDifficulty::Easy => Self::EASY_KEY_DELAY,
            AiDifficulty::Normal => Self::NORMAL_KEY_DELAY,
            AiDifficulty::Hard => Self::HARD_KEY_DELAY,
            AiDifficulty::Impossible => Duration::ZERO,
        }
    }

    /// What this difficulty thinks with. The three below the top are rows of the N64 ai's own
    /// weights, out of the six ranked worst to best in [`crate::game::ai::SKILL_ORDER`], so a
    /// harder setting is a better player and not merely a faster one.
    ///
    /// `impossible` is the **trained network**, which now beats the best of those rows: over
    /// twenty seeds at the training budget, 20,016 viruses and 422 bottles against 18,093 and
    /// 405, winning seventeen of the twenty. It sits above the ladder rather than on it - the
    /// network has one model and no rows to pick between, so what it offers is a ceiling and
    /// not a dial.
    pub fn brain(&self) -> DrAiKind {
        match self {
            AiDifficulty::Easy => DrAiKind::n64_nth_weakest(0),
            AiDifficulty::Normal => DrAiKind::n64_nth_weakest(1),
            AiDifficulty::Hard => DrAiKind::n64_nth_weakest(5),
            AiDifficulty::Impossible => DrAiKind::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiMode {
    /// No ai players
    Off,
    /// Single player game played by the ai at full speed
    Demo,
    /// Two player game played by the ai at full speed: the two best rows of the N64 ai's
    /// weights against each other, the runner up as player 1 and the best as player 2
    VsDemo,
    /// Two player game where player 2 is the ai
    Opponent(AiDifficulty),
}

impl AiMode {
    /// nobody is playing: every board is the ai's, at full speed
    pub fn is_demo(&self) -> bool {
        matches!(self, AiMode::Demo | AiMode::VsDemo)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameConfig {
    players: u32,
    virus_level: u32,
    speed: GameSpeed,
    themes: MatchThemes,
    rules: MatchRules,
    random: RandomMode,
    ai: AiMode,
}

impl GameConfig {
    pub fn new(
        players: u32,
        virus_level: u32,
        speed: GameSpeed,
        themes: MatchThemes,
        rules: MatchRules,
        random: RandomMode,
    ) -> Self {
        Self {
            players,
            virus_level,
            speed,
            themes,
            rules,
            random,
            ai: AiMode::Off,
        }
    }

    pub fn ai(&self) -> AiMode {
        self.ai
    }

    pub fn set_ai(&mut self, ai: AiMode) {
        self.ai = ai;
    }

    /// The number of players actually in the match, taking the ai mode into account
    pub fn effective_players(&self) -> u32 {
        match self.ai {
            AiMode::Off => self.players,
            AiMode::Demo => 1,
            AiMode::VsDemo | AiMode::Opponent(_) => 2,
        }
    }

    /// The ai controlled players (0-indexed), the key delay they play at and the brain they
    /// think with
    pub fn ai_players(&self) -> Vec<(u32, Duration, DrAiKind)> {
        match self.ai {
            AiMode::Off => vec![],
            AiMode::Demo => vec![(0, Duration::ZERO, DrAiKind::default())],
            // the trained network against the best of the N64 ai's rows of weights: the one
            // place the network plays, since it is not yet good enough to field as a difficulty
            AiMode::VsDemo => vec![
                (0, Duration::ZERO, DrAiKind::Neural(survival_trained())),
                (1, Duration::ZERO, DrAiKind::n64_nth_weakest(5)),
            ],
            AiMode::Opponent(difficulty) => {
                vec![(1, difficulty.key_delay(), difficulty.brain())]
            }
        }
    }

    pub fn is_ai_player(&self, player: u32) -> bool {
        self.ai_players().iter().any(|(p, _, _)| *p == player)
    }

    pub fn players(&self) -> u32 {
        self.players
    }

    pub fn is_single_player(&self) -> bool {
        self.players == 1
    }

    pub fn virus_level(&self) -> u32 {
        self.virus_level
    }
    pub fn speed(&self) -> GameSpeed {
        self.speed
    }
    pub fn themes(&self) -> MatchThemes {
        self.themes
    }
    pub fn rules(&self) -> MatchRules {
        self.rules
    }
    pub fn random(&self) -> RandomMode {
        self.random
    }

    pub fn set_players(&mut self, players: u32) {
        self.players = players;
    }
    pub fn set_virus_level(&mut self, virus_level: u32) {
        self.virus_level = virus_level.min(MAX_VIRUS_LEVEL);
    }
    pub fn set_speed(&mut self, speed: GameSpeed) {
        self.speed = speed;
    }
    pub fn set_themes(&mut self, themes: MatchThemes) {
        self.themes = themes;
    }
    pub fn set_rules(&mut self, rules: MatchRules) {
        self.rules = rules;
    }
    pub fn set_random(&mut self, random: RandomMode) {
        self.random = random;
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        Self::new(
            1,
            0,
            GameSpeed::Medium,
            MatchThemes::All,
            MatchRules::Marathon,
            RandomMode::default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ai::{DrAiKind, SKILLS, SKILL_ORDER};

    fn skill(brain: DrAiKind) -> u8 {
        match brain {
            DrAiKind::N64(ai) => ai.skill(),
            other => panic!("expected one of the N64 ai's rows, got {:?}", other),
        }
    }

    #[test]
    fn theme_count() {
        assert_eq!(MatchThemes::count(), 4);
    }

    #[test]
    fn the_difficulties_below_the_top_climb_the_measured_ranking() {
        let rows: Vec<u8> = [AiDifficulty::Easy, AiDifficulty::Normal, AiDifficulty::Hard]
            .iter()
            .map(|d| skill(d.brain()))
            .collect();
        assert_eq!(rows, vec![SKILL_ORDER[0], SKILL_ORDER[1], SKILL_ORDER[5]]);

        // each is a different row and they climb it, so a harder setting is a better player and
        // not merely a faster one
        let ranks: Vec<usize> = rows
            .iter()
            .map(|row| SKILL_ORDER.iter().position(|r| r == row).unwrap())
            .collect();
        assert!(
            ranks.windows(2).all(|pair| pair[0] < pair[1]),
            "{:?}",
            ranks
        );
    }

    /// The network beat the best row it learned from, so it is what `impossible` fields - and
    /// it sits above the ladder rather than on it, since one model is a ceiling and not a dial.
    #[test]
    fn the_hardest_difficulty_plays_the_network_and_the_rest_play_rows() {
        assert!(
            matches!(AiDifficulty::Impossible.brain(), DrAiKind::Neural(_)),
            "{:?}",
            AiDifficulty::Impossible.brain()
        );
        for difficulty in [AiDifficulty::Easy, AiDifficulty::Normal, AiDifficulty::Hard] {
            skill(difficulty.brain());
        }
        // and the hardest row is still fielded, by the difficulty below it, so the ladder does
        // not skip the best thing the port has
        assert_eq!(skill(AiDifficulty::Hard.brain()), SKILL_ORDER[SKILLS - 1]);
    }

    #[test]
    fn the_two_player_demo_puts_the_network_against_the_best_row() {
        let mut config = GameConfig::default();
        config.set_ai(AiMode::VsDemo);
        let players = config.ai_players();
        assert_eq!(config.effective_players(), 2);
        assert_eq!(
            players.iter().map(|(p, _, _)| *p).collect::<Vec<u32>>(),
            vec![0, 1]
        );
        // both at full speed: the demo is a contest of models, not of key rates
        assert!(players.iter().all(|(_, delay, _)| delay.is_zero()));
        // the trained network plays player 1
        assert!(
            matches!(players[0].2, DrAiKind::Neural(_)),
            "{:?}",
            players[0].2
        );
        // and the best of the N64's rows defends player 2, which is the model against the ai
        // it learned from and beat
        assert_eq!(skill(players[1].2), SKILL_ORDER[SKILLS - 1]);
    }

    /// [`DrAiKind::default`] is the network now, and the 1-player demo is what takes it - which
    /// is the whole point of changing it, since the demo is what anyone judging how the model
    /// *looks* is watching.
    #[test]
    fn the_one_player_demo_plays_the_network_at_full_speed() {
        let mut config = GameConfig::default();
        config.set_ai(AiMode::Demo);
        let players = config.ai_players();
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].0, 0);
        assert!(players[0].1.is_zero());
        assert!(
            matches!(players[0].2, DrAiKind::Neural(_)),
            "{:?}",
            players[0].2
        );
    }
}
