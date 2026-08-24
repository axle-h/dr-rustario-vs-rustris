//! The match options Rustris offers on the main menu.

use crate::game::random::{random_tetrominos, RandomMode};
use crate::game::rules::{AiDifficulty, AiMode, GameConfig, MatchRules, MatchThemes};
use crate::game::Game;
use engine::app::ThemeMode;
use engine::menu::MenuItem;
use std::str::FromStr;

pub const STAGE_NOUN: &str = "level";
const MAX_START_LEVEL: u32 = 9;

const THEMES: &str = "themes";
const MODE: &str = "mode";
const LEVEL: &str = "level";
const RANDOM: &str = "random";
const VS_AI_PREFIX: &str = "vs ";
const VS_AI_SUFFIX: &str = " ai";
const AI_DEMO_1P: &str = "1-player ai demo";
const AI_DEMO_2P: &str = "2-player ai demo";

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
        self.config.players = players;
        self.config.rules = MatchRules::default_by_players(players);
    }

    /// the title screen's players list: humans, then the ai opponents and the ai demos
    pub fn players_list(&self, max_players: u32) -> (Vec<String>, usize) {
        let mut players = (1..=max_players).map(|i| i.to_string()).collect::<Vec<String>>();
        if max_players > 1 {
            players.extend(
                AiDifficulty::ALL
                    .iter()
                    .map(|d| format!("{}{}{}", VS_AI_PREFIX, d.name(), VS_AI_SUFFIX)),
            );
        }
        players.push(AI_DEMO_1P.to_string());
        if max_players > 1 {
            players.push(AI_DEMO_2P.to_string());
        }
        let current = match self.config.ai {
            AiMode::Off => (self.config.players as usize).clamp(1, max_players as usize) - 1,
            AiMode::Opponent(difficulty) => players
                .iter()
                .position(|p| *p == format!("{}{}{}", VS_AI_PREFIX, difficulty.name(), VS_AI_SUFFIX))
                .unwrap_or(0),
            AiMode::Demo => players.iter().position(|p| p == AI_DEMO_1P).unwrap_or(0),
            AiMode::VsDemo => players.iter().position(|p| p == AI_DEMO_2P).unwrap_or(0),
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
            self.config.ai = AiMode::Demo;
        } else if value == AI_DEMO_2P {
            self.set_players(2);
            self.config.ai = AiMode::VsDemo;
        } else if let Some(difficulty) = ai_difficulty {
            self.set_players(2);
            self.config.ai = AiMode::Opponent(difficulty);
        } else {
            self.set_players(value.parse::<u32>().unwrap_or(1));
            self.config.ai = AiMode::Off;
        }
    }

    pub fn players(&self) -> u32 {
        self.config.players
    }

    pub fn is_single_player(&self) -> bool {
        self.config.players == 1
    }

    /// `compact` leaves out the mode and randomiser, for a mixed match's second game
    pub fn menu_items(&self, compact: bool) -> Vec<MenuItem> {
        let modes = Self::modes(self.config.players);
        let items = vec![
            MenuItem::select_list(
                THEMES,
                MatchThemes::names()
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                self.config.themes as usize,
            ),
            MenuItem::select_list(
                MODE,
                modes.iter().map(|m| m.name(STAGE_NOUN)).collect(),
                modes
                    .iter()
                    .position(|&m| m == self.config.rules)
                    .unwrap_or(0),
            ),
            MenuItem::select_list(
                LEVEL,
                (0..=MAX_START_LEVEL).map(|i| i.to_string()).collect(),
                self.config.level as usize,
            ),
            MenuItem::select_list(
                RANDOM,
                RandomMode::names()
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                self.config.random as usize,
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
            THEMES => self.config.themes = MatchThemes::from_str(value).unwrap(),
            MODE => {
                let modes = Self::modes(self.config.players);
                if let Some(mode) = modes.iter().find(|m| m.name(STAGE_NOUN) == value) {
                    self.config.rules = *mode;
                }
            }
            LEVEL => self.config.level = value.parse::<u32>().unwrap(),
            RANDOM => self.config.random = RandomMode::from_str(value).unwrap(),
            _ => return false,
        }
        true
    }


    /// `count` games sharing one seed, so players face the same pieces
    pub fn games(&self, count: usize) -> Vec<Game> {
        random_tetrominos(self.config.random, count)
            .into_iter()
            .map(|rand| Game::new(self.config.level, rand))
            .collect()
    }

    pub fn theme_mode(&self) -> ThemeMode {
        match self.config.themes {
            MatchThemes::All => ThemeMode::All,
            themes => ThemeMode::Fixed(themes.initial_index()),
        }
    }

    pub fn rules(&self) -> MatchRules {
        self.config.rules
    }

    pub fn ai(&self) -> AiMode {
        self.config.ai
    }

    /// the players the AI plays for, how fast, and the model they play
    pub fn ai_players(
        &self,
    ) -> Vec<(u32, std::time::Duration, crate::game::ai::TetrisNeuralNetwork)> {
        self.config.ai_players()
    }
}
