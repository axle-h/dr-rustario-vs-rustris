//! The match options Dr. Rustario offers on the main menu.

use crate::game::ai::DrNeuralNetwork;
use crate::game::random::{random, RandomMode};
use crate::game::rules::{AiDifficulty, AiMode, GameConfig, MatchRules, MatchThemes, MAX_VIRUS_LEVEL};
use crate::game::{Game, GameSpeed};
use engine::app::ThemeMode;
use engine::menu::MenuItem;
use std::str::FromStr;
use std::time::Duration;

pub const STAGE_NOUN: &str = "level";

const THEMES: &str = "themes";
const MODE: &str = "mode";
const LEVEL: &str = "level";
const SPEED: &str = "speed";
const RANDOM: &str = "random";
const VS_AI_PREFIX: &str = "vs ";
const VS_AI_SUFFIX: &str = " ai";
const AI_DEMO_1P: &str = "1-player ai demo";

#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    config: GameConfig,
}

impl Options {
    fn modes(players: u32) -> Vec<MatchRules> {
        if players == 1 {
            MatchRules::SINGLE_PLAYER_MODES.to_vec()
        } else {
            MatchRules::VS_MODES.to_vec()
        }
    }

    pub fn set_players(&mut self, players: u32) {
        self.config.set_players(players);
        self.config
            .set_rules(MatchRules::default_by_players(players));
    }

    /// the title screen's players list: humans, then the ai opponents and the ai demo. There is
    /// only one Dr. Rustario model, so there is no 2-player demo: it would play itself.
    pub fn players_list(&self, max_players: u32) -> (Vec<String>, usize) {
        let mut players = (1..=max_players)
            .map(|i| i.to_string())
            .collect::<Vec<String>>();
        if max_players > 1 {
            players.extend(
                AiDifficulty::ALL
                    .iter()
                    .map(|d| format!("{}{}{}", VS_AI_PREFIX, d.name(), VS_AI_SUFFIX)),
            );
        }
        players.push(AI_DEMO_1P.to_string());
        let current = match self.config.ai() {
            AiMode::Off => (self.config.players() as usize).clamp(1, max_players as usize) - 1,
            AiMode::Opponent(difficulty) => players
                .iter()
                .position(|p| {
                    *p == format!("{}{}{}", VS_AI_PREFIX, difficulty.name(), VS_AI_SUFFIX)
                })
                .unwrap_or(0),
            AiMode::Demo => players.iter().position(|p| p == AI_DEMO_1P).unwrap_or(0),
        };
        (players, current)
    }

    /// a pick from [`Self::players_list`]
    pub fn select_players(&mut self, value: &str) {
        let ai_difficulty = value
            .strip_prefix(VS_AI_PREFIX)
            .and_then(|s| s.strip_suffix(VS_AI_SUFFIX))
            .and_then(AiDifficulty::from_name);
        if value == AI_DEMO_1P {
            self.set_players(1);
            self.config.set_ai(AiMode::Demo);
        } else if let Some(difficulty) = ai_difficulty {
            self.set_players(2);
            self.config.set_ai(AiMode::Opponent(difficulty));
        } else {
            self.set_players(value.parse::<u32>().unwrap_or(1));
            self.config.set_ai(AiMode::Off);
        }
    }

    pub fn ai(&self) -> AiMode {
        self.config.ai()
    }

    /// the players the ai plays for, how fast, and the model they play
    pub fn ai_players(&self) -> Vec<(u32, Duration, DrNeuralNetwork)> {
        self.config.ai_players()
    }

    pub fn players(&self) -> u32 {
        self.config.players()
    }

    pub fn is_single_player(&self) -> bool {
        self.config.is_single_player()
    }

    /// `compact` leaves out the mode and randomiser, for a mixed match's second game
    pub fn menu_items(&self, compact: bool) -> Vec<MenuItem> {
        let modes = Self::modes(self.config.players());
        let items = vec![
            MenuItem::select_list(
                THEMES,
                MatchThemes::names()
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                self.config.themes() as usize,
            ),
            MenuItem::select_list(
                MODE,
                modes.iter().map(|m| m.name(STAGE_NOUN)).collect(),
                modes
                    .iter()
                    .position(|&m| m == self.config.rules())
                    .unwrap_or(0),
            ),
            MenuItem::select_list(
                LEVEL,
                (0..=MAX_VIRUS_LEVEL).map(|i| i.to_string()).collect(),
                self.config.virus_level() as usize,
            ),
            MenuItem::select_list(
                SPEED,
                GameSpeed::names()
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                self.config.speed() as usize,
            ),
            MenuItem::select_list(
                RANDOM,
                RandomMode::names()
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                self.config.random() as usize,
            ),
        ];
        if compact {
            items
                .into_iter()
                .filter(|item| item.name() != MODE && item.name() != RANDOM)
                .collect()
        } else {
            items
        }
    }

    /// returns true if the selection was one of these options
    pub fn select(&mut self, name: &str, value: &str) -> bool {
        match name {
            THEMES => self
                .config
                .set_themes(MatchThemes::from_str(value).unwrap()),
            MODE => {
                let modes = Self::modes(self.config.players());
                if let Some(mode) = modes.iter().find(|m| m.name(STAGE_NOUN) == value) {
                    self.config.set_rules(*mode);
                }
            }
            LEVEL => self.config.set_virus_level(value.parse::<u32>().unwrap()),
            SPEED => self.config.set_speed(GameSpeed::from_str(value).unwrap()),
            RANDOM => self.config.set_random(RandomMode::from_str(value).unwrap()),
            _ => return false,
        }
        true
    }


    /// `count` games sharing one seed, so players face the same bottles and pills
    pub fn games(&self, count: usize) -> Result<Vec<Game>, String> {
        random(count, self.config.random())
            .into_iter()
            .map(|rand| Game::new(self.config.virus_level(), self.config.speed(), rand))
            .collect()
    }

    pub fn theme_mode(&self) -> ThemeMode {
        match self.config.themes() {
            MatchThemes::All => ThemeMode::All,
            MatchThemes::Nes => ThemeMode::Fixed(0),
            MatchThemes::Snes => ThemeMode::Fixed(1),
            MatchThemes::N64 => ThemeMode::Fixed(2),
            MatchThemes::Particle => ThemeMode::Fixed(3),
        }
    }

    pub fn rules(&self) -> MatchRules {
        self.config.rules()
    }
}
