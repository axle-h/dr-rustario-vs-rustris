//! Match options specific to Rustris.

use crate::game::ai::TetrisNeuralNetwork;
use crate::game::random::RandomMode;
pub use engine::session::MatchRules;
use std::time::Duration;
use strum::IntoEnumIterator;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, strum::IntoStaticStr, strum::EnumIter, strum::EnumString,
)]
pub enum MatchThemes {
    /// Run themes in order, switching at the next level
    #[strum(serialize = "all")]
    All,
    #[strum(serialize = "gameboy")]
    GameBoy,
    #[strum(serialize = "nes")]
    Nes,
    #[strum(serialize = "snes")]
    Snes,
    #[strum(serialize = "modern")]
    Modern,
}

impl MatchThemes {
    pub fn names() -> Vec<&'static str> {
        Self::iter().map(|e| e.into()).collect()
    }

    pub fn count() -> usize {
        Self::iter().filter(|i| *i as usize > 0).count()
    }

    /// the theme every player starts on
    pub fn initial_index(&self) -> usize {
        match self {
            MatchThemes::All | MatchThemes::GameBoy => 0,
            MatchThemes::Nes => 1,
            MatchThemes::Snes => 2,
            MatchThemes::Modern => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiDifficulty {
    /// The most speed limited, and plays the survival model, which rarely attacks
    Easy,
    /// Speed limited, and plays the survival model, which rarely attacks
    Normal,
    /// Speed limited
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

    /// The model this difficulty plays: easy and normal keep to the survival model,
    /// everything harder plays the high scoring tetris clear model
    pub fn network(&self) -> TetrisNeuralNetwork {
        match self {
            AiDifficulty::Easy | AiDifficulty::Normal => TetrisNeuralNetwork::survival_trained(),
            _ => TetrisNeuralNetwork::tetris_clear_trained(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiMode {
    /// No ai players
    Off,
    /// Single player game played by the ai at full speed
    Demo,
    /// Two player game played by the ai at full speed: the survival model
    /// as player 1 against the tetris clear model as player 2
    VsDemo,
    /// Two player game where player 2 is the ai
    Opponent(AiDifficulty),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameConfig {
    pub players: u32,
    pub level: u32,
    pub rules: MatchRules,
    pub themes: MatchThemes,
    pub ai: AiMode,
    pub random: RandomMode,
}

impl GameConfig {
    pub fn new(players: u32, level: u32, rules: MatchRules, themes: MatchThemes) -> Self {
        Self {
            players,
            level,
            rules,
            themes,
            ai: AiMode::Off,
            random: RandomMode::Bag,
        }
    }

    /// The number of players actually in the match, taking the ai mode into account
    pub fn effective_players(&self) -> u32 {
        match self.ai {
            AiMode::Off => self.players,
            AiMode::Demo => 1,
            AiMode::VsDemo => 2,
            AiMode::Opponent(_) => 2,
        }
    }

    /// The ai controlled players (0-indexed), the key delay they play at and the model they play
    pub fn ai_players(&self) -> Vec<(u32, Duration, TetrisNeuralNetwork)> {
        match self.ai {
            AiMode::Off => vec![],
            AiMode::Demo => vec![(0, Duration::ZERO, TetrisNeuralNetwork::default())],
            AiMode::VsDemo => vec![
                (0, Duration::ZERO, TetrisNeuralNetwork::survival_trained()),
                (1, Duration::ZERO, TetrisNeuralNetwork::tetris_clear_trained()),
            ],
            AiMode::Opponent(difficulty) => {
                vec![(1, difficulty.key_delay(), difficulty.network())]
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
