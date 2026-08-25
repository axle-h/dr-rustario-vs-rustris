use crate::game::ai::DrNeuralNetwork;
use crate::game::ai::models;
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

pub use engine::session::MatchRules;

/// How fast the ai is allowed to play. There is one Dr. Rustario model, trained to clear the
/// viruses as quickly as it can, so a difficulty only decides how quickly it may press keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiDifficulty {
    /// The most speed limited
    Easy,
    Normal,
    Hard,
    /// Full speed
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

    /// every difficulty plays the one trained model
    pub fn network(&self) -> DrNeuralNetwork {
        models::virus_clear_trained()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiMode {
    /// No ai players
    Off,
    /// Single player game played by the ai at full speed
    Demo,
    /// Two player game where player 2 is the ai
    Opponent(AiDifficulty),
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
            AiMode::Opponent(_) => 2,
        }
    }

    /// The ai controlled players (0-indexed), the key delay they play at and the model they play
    pub fn ai_players(&self) -> Vec<(u32, Duration, DrNeuralNetwork)> {
        match self.ai {
            AiMode::Off => vec![],
            AiMode::Demo => vec![(0, Duration::ZERO, models::virus_clear_trained())],
            AiMode::Opponent(difficulty) => {
                vec![(1, difficulty.key_delay(), difficulty.network())]
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
        if self.rules == MatchRules::ThemeSprint {
            MatchThemes::All
        } else {
            self.themes
        }
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

    #[test]
    fn theme_count() {
        assert_eq!(MatchThemes::count(), 4);
    }
}
