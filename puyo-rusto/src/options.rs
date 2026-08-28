//! The match options Puyo Rusto offers on the main menu.

use crate::game::ai::PuyoAiKind;
use crate::game::cell::PuyoSkin;
use crate::game::random::from_seed;
use crate::game::rules::{
    AiDifficulty, AiMode, Difficulty, GameConfig, GameMusic, MatchRules, MatchThemes,
    MAX_START_LEVEL,
};
use crate::game::Game;
use engine::app::{MusicChoice, ThemeMode};
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
const MUSIC: &str = "music";
const RANDOM_MUSIC: &str = "random";
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
            MenuItem::select_list(
                MUSIC,
                music_names(),
                match self.config.music {
                    None => 0,
                    Some(track) => track as usize + 1,
                },
            ),
        ];
        if compact {
            // the mode goes because the other game names it, and the music goes with it: a
            // match is played on one track, so a mixed match cannot take two games' answers
            items
                .into_iter()
                .filter(|item| item.name() != MODE && item.name() != MUSIC)
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
            MUSIC => self.config.music = GameMusic::from_name(value),
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

    /// which track the match is played on: a pinned one by its place in the theme's table,
    /// or the theme's own pick
    pub fn music_choice(&self) -> MusicChoice {
        match self.config.music {
            None => MusicChoice::Random,
            Some(track) => MusicChoice::Track(track as usize),
        }
    }
}

/// the music row's values: dealing one is the first of them, and the default
fn music_names() -> Vec<String> {
    std::iter::once(RANDOM_MUSIC.to_string())
        .chain(GameMusic::ALL.iter().map(|m| m.name().to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode_names(options: &Options) -> Vec<String> {
        options.modes().iter().map(|m| m.name(STAGE_NOUN)).collect()
    }

    /// The row reads oldest hardware first with the particle theme last, which is how the
    /// other two games order theirs - and `particle` is the name every game gives the modern
    /// theme, so the vs. mode's particle playlist can pick it out by it.
    #[test]
    fn the_themes_are_listed_oldest_first_and_the_modern_one_is_called_particle() {
        assert_eq!(
            MatchThemes::names(),
            vec!["all", "genesis", "snes", "3ds", "particle"]
        );
    }

    #[test]
    fn two_players_may_run_a_marathon_too() {
        let mut options = Options::default();
        options.set_players(2);
        assert!(mode_names(&options).contains(&"marathon".to_string()));
        // ... though a 2-player match still opens on the 1 level sprint
        assert_eq!(options.rules(), MatchRules::ONE_STAGE_SPRINT);
    }

    /// A theme sprint runs one level per theme, so it is only offered while there is more
    /// than one to run through. Phase 2 shipped a single theme and it was not on the list;
    /// phase 3a added the second and it came back on its own, which is the whole of what
    /// `Options::modes` has to get right here.
    #[test]
    fn a_theme_sprint_is_offered_once_there_is_more_than_one_theme() {
        let options = Options::default();
        assert!(MatchThemes::count() > 1);
        assert!(mode_names(&options).contains(&"theme sprint".to_string()));
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

    /// `random` is the first value and the default, and every track is pinned by the name it
    /// is listed under - the same round trip the ai list is held to below
    #[test]
    fn every_track_is_picked_by_the_name_it_is_listed_under() {
        let mut options = Options::default();
        assert_eq!(options.music_choice(), MusicChoice::Random);
        for (index, name) in music_names().into_iter().enumerate() {
            options.select(MUSIC, &name);
            let items = options.menu_items(false);
            let row = items.iter().find(|item| item.name() == MUSIC).unwrap();
            assert_eq!(
                *row,
                MenuItem::select_list(MUSIC, music_names(), index),
                "{name} did not come back selected"
            );
        }
        // ... and the last of them is a track rather than the deal
        assert_eq!(
            options.music_choice(),
            MusicChoice::Track(GameMusic::ALL.len() - 1)
        );
    }

    /// a mixed match's second game shows neither the mode nor the music: one match, one track
    #[test]
    fn a_compact_menu_leaves_the_music_to_the_other_game() {
        let items = Options::default().menu_items(true);
        assert!(!items
            .iter()
            .any(|item| item.name() == MUSIC || item.name() == MODE));
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
