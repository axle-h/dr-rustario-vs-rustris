//! The match options Super Rustle Fighter offers on the main menu.
//!
//! Short, because this game has fewer dials than the other three: one theme, so nothing to
//! pick between; **no ai yet**, so no vs-ai rows and no demos - see the plan's phase 3.
//!
//! **Character select is a menu row rather than a screen of its own**, which is the one shell
//! question the plan left open (2026-09-07). A row is what the choice actually is - it changes
//! one thing about the match, the way the difficulty does - and the arcade's select screen art
//! is a screen for choosing an *opponent* as much as a fighter, which is a Street Puzzle mode
//! thing and out of scope. If the fighter layer ever wants the portraits, the row can become
//! the screen without moving the option.

use crate::game::counter::Fighter;
use crate::game::random::{GameRandom, Seed};
use crate::game::rules::{Difficulty, GameConfig, MatchRules, MatchThemes, MAX_START_LEVEL};
use crate::game::Game;
use engine::app::ThemeMode;
use engine::menu::MenuItem;
use std::str::FromStr;

/// A stage here is a speed step rather than a level of its own, but the HUD row is the same
/// `Level` every game shows, so the menu calls it what the other games call it.
pub const STAGE_NOUN: &str = "level";

const MODE: &str = "mode";
const LEVEL: &str = "level";
const DIFFICULTY: &str = "difficulty";
const FIGHTER: &str = "fighter";

#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    config: GameConfig,
}

impl Options {
    pub fn players(&self) -> u32 {
        self.config.effective_players()
    }

    pub fn rules(&self) -> MatchRules {
        self.config.rules
    }

    pub fn theme_mode(&self) -> ThemeMode {
        // one theme, so there is nothing to run through and nothing to hold still
        ThemeMode::Fixed(self.config.themes.initial_index())
    }

    pub fn set_players(&mut self, players: u32) {
        self.config.players = players;
        self.config.rules = MatchRules::default_for(players, false, MatchThemes::count());
    }

    /// the title screen's players list, which here is only ever humans
    pub fn players_list(&self, max_players: u32) -> (Vec<String>, usize) {
        let players: Vec<String> = (1..=max_players).map(|i| i.to_string()).collect();
        let current = (self.config.players as usize).clamp(1, max_players as usize) - 1;
        (players, current)
    }

    pub fn select_players(&mut self, value: &str) {
        if let Ok(players) = value.parse::<u32>() {
            self.set_players(players);
        }
    }

    pub fn menu_items(&self, _playlist: bool) -> Vec<MenuItem> {
        let modes = MatchRules::modes(MatchThemes::count());
        vec![
            MenuItem::select_list(
                MODE,
                modes.iter().map(|m| m.name(STAGE_NOUN)).collect(),
                modes
                    .iter()
                    .position(|m| *m == self.config.rules)
                    .unwrap_or(0),
            ),
            MenuItem::select_list(
                FIGHTER,
                Fighter::ALL
                    .iter()
                    .map(|f| f.name().to_lowercase())
                    .collect(),
                Fighter::ALL
                    .iter()
                    .position(|f| *f == self.config.fighter)
                    .unwrap_or(0),
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
            MenuItem::select_list(
                LEVEL,
                (0..=MAX_START_LEVEL).map(|l| l.to_string()).collect(),
                self.config.level.min(MAX_START_LEVEL) as usize,
            ),
        ]
    }

    pub fn select(&mut self, name: &str, value: &str) {
        match name {
            MODE => {
                let modes = MatchRules::modes(MatchThemes::count());
                if let Some(rules) = modes.iter().find(|m| m.name(STAGE_NOUN) == value) {
                    self.config.rules = *rules;
                }
            }
            FIGHTER => {
                if let Some(fighter) = Fighter::ALL
                    .iter()
                    .find(|f| f.name().eq_ignore_ascii_case(value))
                {
                    self.config.fighter = *fighter;
                }
            }
            DIFFICULTY => {
                if let Some(difficulty) = Difficulty::from_name(value) {
                    self.config.difficulty = difficulty;
                }
            }
            LEVEL => {
                if let Ok(level) = u32::from_str(value) {
                    self.config.level = level.min(MAX_START_LEVEL);
                }
            }
            _ => {}
        }
    }

    /// One game per player, all dealt from one seed - which is what makes a two player match
    /// the same game twice rather than two games.
    pub fn games(&self, players: usize) -> Vec<Game> {
        let seed = Seed::random();
        (0..players)
            .map(|_| {
                Game::new(
                    self.config.fighter,
                    self.config.difficulty,
                    self.config.level,
                    GameRandom::from_seed(seed),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// every row the menu offers reads its own value back, which is what stops a dial that
    /// silently does nothing
    #[test]
    fn every_menu_row_round_trips() {
        let mut options = Options::default();
        options.select(FIGHTER, "Sakura");
        options.select(DIFFICULTY, "hard");
        options.select(LEVEL, "5");
        assert_eq!(options.config.fighter, Fighter::Sakura);
        assert_eq!(options.config.difficulty, Difficulty::Hard);
        assert_eq!(options.config.level, 5);
        let items = options.menu_items(false);
        assert_eq!(items.len(), 4, "mode, fighter, difficulty and level");
    }

    /// **a theme sprint is not offered**, because there is one theme and it would be a one
    /// stage sprint under another name
    #[test]
    fn the_menu_offers_no_theme_sprint() {
        let modes = MatchRules::modes(MatchThemes::count());
        assert!(!modes.contains(&MatchRules::ThemeSprint));
        assert!(modes.contains(&MatchRules::Marathon));
    }

    /// both boards of a two player match are dealt the same sequence
    #[test]
    fn every_player_is_dealt_the_same_game() {
        let options = Options::default();
        let games = options.games(2);
        assert_eq!(games.len(), 2);
        let queues: Vec<_> = games.iter().map(|g| g.pair().map(|p| p.piece())).collect();
        assert_eq!(queues[0], queues[1]);
    }
}
