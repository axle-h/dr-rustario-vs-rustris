//! The match options Puyo Rusto offers on the main menu.

use crate::game::ai::PuyoAiKind;
use crate::game::cell::PuyoSkin;
use crate::game::random::from_seed;
use crate::game::rules::{
    AiDifficulty, AiMode, Difficulty, GameConfig, MatchRules, MatchThemes, MAX_START_LEVEL,
};
use crate::game::Game;
use engine::app::ThemeMode;
use engine::game::random::Seed;
use engine::menu::MenuItem;
use std::str::FromStr;
use std::time::Duration;

/// A Puyo stage is a speed step rather than a level of its own, but the HUD row is the same
/// `Level` every game shows, so the menu calls it what the other two call it.
pub const STAGE_NOUN: &str = "level";

const THEMES: &str = "themes";
const MODE: &str = "mode";
const LEVEL: &str = "level";
const DIFFICULTY: &str = "difficulty";
const VS_AI_PREFIX: &str = "vs ";
const VS_AI_SUFFIX: &str = " ai";
const AI_DEMO_1P: &str = "1-player ai demo";
const AI_DEMO_2P: &str = "2-player ai demo";

#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    config: GameConfig,
}

impl Options {
    /// the modes on offer: a theme sprint runs one level per theme, so it is only offered
    /// when the match runs through every theme rather than sticking to one
    fn modes(&self) -> Vec<MatchRules> {
        MatchRules::modes(self.theme_count())
    }

    /// how many themes this match will run through
    fn theme_count(&self) -> usize {
        match self.config.themes {
            MatchThemes::All => MatchThemes::count(),
            _ => 1,
        }
    }

    pub fn set_players(&mut self, players: u32) {
        self.config.players = players;
        self.config.rules = MatchRules::default_by_players(players);
    }

    /// the title screen's players list: humans, then the ai opponents and the ai demos
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
        if max_players > 1 {
            players.push(AI_DEMO_2P.to_string());
        }
        let current = match self.config.ai {
            AiMode::Off => (self.config.players as usize).clamp(1, max_players as usize) - 1,
            AiMode::Opponent(difficulty) => players
                .iter()
                .position(|p| {
                    *p == format!("{}{}{}", VS_AI_PREFIX, difficulty.name(), VS_AI_SUFFIX)
                })
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

    pub fn ai(&self) -> AiMode {
        self.config.ai
    }

    /// the players the ai plays for, how fast, and the brain they think with
    pub fn ai_players(&self) -> Vec<(u32, Duration, PuyoAiKind)> {
        self.config.ai_players()
    }

    pub fn players(&self) -> u32 {
        self.config.players
    }

    pub fn is_single_player(&self) -> bool {
        self.config.players == 1
    }

    /// `compact` leaves out the mode, for a mixed match's second game
    pub fn menu_items(&self, compact: bool) -> Vec<MenuItem> {
        let modes = self.modes();
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
                DIFFICULTY,
                Difficulty::ALL
                    .iter()
                    .map(|d| d.name().to_string())
                    .collect(),
                Difficulty::ALL
                    .iter()
                    .position(|d| *d == self.config.difficulty)
                    .unwrap_or(0),
            ),
        ];
        if compact {
            items
                .into_iter()
                .filter(|item| item.name() != MODE)
                .collect()
        } else {
            items
        }
    }

    /// returns true if the selection was one of these options
    pub fn select(&mut self, name: &str, value: &str) -> bool {
        match name {
            THEMES => {
                self.config.themes = MatchThemes::from_str(value).unwrap();
                // sticking to one theme takes the theme sprint off the table
                if !self.modes().contains(&self.config.rules) {
                    self.config.rules = MatchRules::default_by_players(self.config.players);
                }
            }
            MODE => {
                if let Some(mode) = self.modes().iter().find(|m| m.name(STAGE_NOUN) == value) {
                    self.config.rules = *mode;
                }
            }
            LEVEL => self.config.level = value.parse::<u32>().unwrap(),
            DIFFICULTY => {
                if let Some(difficulty) = Difficulty::from_name(value) {
                    self.config.difficulty = difficulty;
                }
            }
            _ => return false,
        }
        true
    }

    /// `count` games sharing one seed, so every player is dealt the same pairs - and a set of
    /// puyos each, dealt from that same seed, so they are not dealt the same ones to look at
    /// and no two matches look alike
    pub fn games(&self, count: usize) -> Vec<Game> {
        let difficulty = self.config.difficulty;
        let seed = Seed::random();
        let skins = PuyoSkin::deal(seed, count);
        from_seed(seed, count, difficulty.colors())
            .into_iter()
            .zip(skins)
            .map(|(random, skin)| Game::new(difficulty, self.config.level, random, skin))
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode_names(options: &Options) -> Vec<String> {
        options.modes().iter().map(|m| m.name(STAGE_NOUN)).collect()
    }

    #[test]
    fn the_modern_theme_is_called_particle_as_it_is_in_the_other_games() {
        assert_eq!(MatchThemes::names(), vec!["all", "particle"]);
    }

    #[test]
    fn two_players_may_run_a_marathon_too() {
        let mut options = Options::default();
        options.set_players(2);
        assert!(mode_names(&options).contains(&"marathon".to_string()));
        // ... though a 2-player match still opens on the 1 level sprint
        assert_eq!(options.rules(), MatchRules::ONE_STAGE_SPRINT);
    }

    /// Phase 2 ships one theme, so there is nothing for a theme sprint to run through yet -
    /// it joins the list of its own accord when phase 3 adds the retro themes
    #[test]
    fn a_theme_sprint_needs_more_than_one_theme() {
        let options = Options::default();
        assert_eq!(MatchThemes::count(), 1);
        assert!(!mode_names(&options).contains(&"theme sprint".to_string()));
    }

    #[test]
    fn the_difficulty_dial_is_the_games_own_five_settings() {
        let mut options = Options::default();
        assert_eq!(options.config.difficulty, Difficulty::Normal);
        options.select(DIFFICULTY, "very hard");
        assert_eq!(options.config.difficulty, Difficulty::VeryHard);
        // a colour count of five is what that setting means to the game itself
        assert_eq!(options.config.difficulty.colors(), 5);
    }

    #[test]
    fn every_ai_mode_is_picked_by_the_name_it_is_listed_under() {
        let mut options = Options::default();
        let (names, _) = options.players_list(2);
        for name in names {
            options.select_players(&name);
            let (again, current) = options.players_list(2);
            assert_eq!(again[current], name);
        }
    }

    #[test]
    fn the_ai_plays_the_second_board_against_a_human_and_both_in_a_demo() {
        let mut options = Options::default();
        options.select_players("vs hard ai");
        assert_eq!(
            options
                .ai_players()
                .iter()
                .map(|(player, ..)| *player)
                .collect::<Vec<u32>>(),
            vec![1]
        );
        options.select_players(AI_DEMO_2P);
        assert_eq!(
            options
                .ai_players()
                .iter()
                .map(|(player, ..)| *player)
                .collect::<Vec<u32>>(),
            vec![0, 1]
        );
        options.select_players(AI_DEMO_1P);
        assert_eq!(options.players(), 1);
    }
}
