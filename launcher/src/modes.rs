//! The three things the launcher can run: each game on its own, exactly as it was standalone,
//! and a versus mode where every player plays the same playlist of both games.

use crate::games::{AnyGame, GameKind};
use engine::app::{MatchSettings, PlayerSettings, StageChange, ThemeMode};
use engine::high_score::table::Ranking;
use engine::high_score::HighScoreKey;
use engine::menu::sound::MenuSounds;
use engine::menu::MenuItem;
use engine::particles::prescribed::RaceTheme;
use engine::render::Theme;
use engine::session::MatchRules;
use std::ops::Range;
use std::time::Duration;

pub const PLAYERS: &str = "players";
pub const HIGH_SCORES: &str = "high scores";
pub const START: &str = "start";
pub const BACK: &str = "back";

/// Every theme of every game in one list, with each game's slice of it.
pub struct Themes<'a> {
    pub all: Vec<Theme<'a>>,
    pub dr_rustario: Range<usize>,
    pub rustris: Range<usize>,
}

impl<'a> Themes<'a> {
    pub fn range(&self, game: GameKind) -> Range<usize> {
        match game {
            GameKind::DrRustario => self.dr_rustario.clone(),
            GameKind::Rustris => self.rustris.clone(),
        }
    }

    pub fn race(&self, game: GameKind) -> Vec<RaceTheme> {
        match game {
            GameKind::DrRustario => {
                dr_rustario::theme::race_themes(&self.all[self.dr_rustario.clone()])
            }
            GameKind::Rustris => {
                let mut race = rustris::theme::race_themes(&self.all[self.rustris.clone()]);
                for theme in race.iter_mut() {
                    theme.theme += self.rustris.start;
                }
                race
            }
        }
    }

    pub fn race_all(&self) -> Vec<RaceTheme> {
        let mut race = self.race(GameKind::DrRustario);
        race.extend(self.race(GameKind::Rustris));
        race
    }
}

pub type Controller = (u32, Box<dyn FnMut(&mut AnyGame, Duration) + 'static>);

/// A launcher mode: its menus, and how it builds a match.
pub trait Mode {
    fn title(&self) -> String;
    fn menu_sounds(&self) -> MenuSounds;
    fn race(&self, themes: &Themes) -> Vec<RaceTheme>;
    /// the title screen's items, before high scores / start / back
    fn title_items(&self, max_players: u32) -> Vec<MenuItem>;
    fn title_select(&mut self, name: &str, value: &str);
    /// the main menu's items, before start / back
    fn menu_items(&self) -> Vec<MenuItem>;
    fn menu_select(&mut self, name: &str, value: &str);
    fn subtitle(&self) -> String;
    /// the high score table the current options compete for: one table per game and mode.
    /// Start level, speed and difficulty all share their mode's table, so a quicker setup
    /// simply ranks higher.
    fn high_score_key(&self) -> HighScoreKey;
    /// every table this mode can compete for, whatever the options
    fn all_high_score_keys(&self) -> Vec<HighScoreKey>;
    fn settings(&self, themes: &Themes) -> MatchSettings;
    fn games(&self) -> Result<Vec<AnyGame>, String>;
    fn next_stage(&self, themes: &Themes, player: u32, completed: u32)
        -> Option<StageChange<AnyGame>>;
    fn controllers(&self) -> Vec<Controller>;
}

fn players_item(max_players: u32, current: u32) -> Option<MenuItem> {
    if max_players > 1 {
        Some(MenuItem::select_list(
            PLAYERS,
            (1..=max_players).map(|i| i.to_string()).collect(),
            current as usize - 1,
        ))
    } else {
        None
    }
}

/// one table per rules variant of a game
fn game_high_score_keys(game: &str, stage_noun: &str) -> Vec<HighScoreKey> {
    MatchRules::SINGLE_PLAYER_MODES
        .iter()
        .map(|rules| HighScoreKey::new(game, rules.name(stage_noun), rules.ranking()))
        .collect()
}

fn subtitle(name: &str, players: u32) -> String {
    if players == 1 {
        format!("{} single player", name)
    } else {
        format!("{} {}-player vs.", name, players)
    }
}

// ---------------------------------------------------------------- Dr. Rustario

#[derive(Default)]
pub struct DrRustarioMode {
    options: dr_rustario::options::Options,
}

impl DrRustarioMode {
    pub fn new() -> Self {
        let mut mode = Self::default();
        mode.options.set_players(1);
        mode
    }
}

impl Mode for DrRustarioMode {
    fn title(&self) -> String {
        "Dr. Rustario".to_string()
    }

    fn menu_sounds(&self) -> MenuSounds {
        MenuSounds::MODERN
    }

    fn race(&self, themes: &Themes) -> Vec<RaceTheme> {
        themes.race(GameKind::DrRustario)
    }

    fn title_items(&self, max_players: u32) -> Vec<MenuItem> {
        players_item(max_players, self.options.players())
            .into_iter()
            .collect()
    }

    fn title_select(&mut self, name: &str, value: &str) {
        if name == PLAYERS {
            self.options.set_players(value.parse::<u32>().unwrap_or(1));
        }
    }

    fn menu_items(&self) -> Vec<MenuItem> {
        self.options.menu_items(false)
    }

    fn menu_select(&mut self, name: &str, value: &str) {
        self.options.select(name, value);
    }

    fn subtitle(&self) -> String {
        subtitle("", self.options.players()).trim().to_string()
    }

    fn high_score_key(&self) -> HighScoreKey {
        let rules = self.options.rules();
        HighScoreKey::new(
            self.title(),
            rules.name(dr_rustario::options::STAGE_NOUN),
            rules.ranking(),
        )
    }

    fn all_high_score_keys(&self) -> Vec<HighScoreKey> {
        game_high_score_keys(&self.title(), dr_rustario::options::STAGE_NOUN)
    }

    fn settings(&self, themes: &Themes) -> MatchSettings {
        MatchSettings {
            rules: self.options.rules(),
            players: (0..self.options.players())
                .map(|_| PlayerSettings {
                    themes: themes.range(GameKind::DrRustario),
                    theme_mode: self.options.theme_mode(),
                })
                .collect(),
            high_score_key: self.high_score_key(),
            playlist: false,
        }
    }

    fn games(&self) -> Result<Vec<AnyGame>, String> {
        Ok(self
            .options
            .games(self.options.players() as usize)?
            .into_iter()
            .map(AnyGame::DrRustario)
            .collect())
    }

    fn next_stage(&self, _: &Themes, _: u32, _: u32) -> Option<StageChange<AnyGame>> {
        None
    }

    fn controllers(&self) -> Vec<Controller> {
        vec![]
    }
}

// ---------------------------------------------------------------- Rustris

#[derive(Default)]
pub struct RustrisMode {
    options: rustris::options::Options,
}

impl RustrisMode {
    pub fn new() -> Self {
        let mut mode = Self::default();
        mode.options.set_players(1);
        mode
    }
}

impl Mode for RustrisMode {
    fn title(&self) -> String {
        "Rustris".to_string()
    }

    fn menu_sounds(&self) -> MenuSounds {
        rustris::theme::MENU_SOUNDS
    }

    fn race(&self, themes: &Themes) -> Vec<RaceTheme> {
        themes.race(GameKind::Rustris)
    }

    fn title_items(&self, max_players: u32) -> Vec<MenuItem> {
        let (players, current) = self.options.players_list(max_players);
        vec![MenuItem::select_list(PLAYERS, players, current)]
    }

    fn title_select(&mut self, name: &str, value: &str) {
        if name == PLAYERS {
            self.options.select_players(value);
        }
    }

    fn menu_items(&self) -> Vec<MenuItem> {
        self.options.menu_items(false)
    }

    fn menu_select(&mut self, name: &str, value: &str) {
        self.options.select(name, value);
    }

    fn subtitle(&self) -> String {
        subtitle("", self.options.players()).trim().to_string()
    }

    fn high_score_key(&self) -> HighScoreKey {
        let rules = self.options.rules();
        HighScoreKey::new(
            self.title(),
            rules.name(rustris::options::STAGE_NOUN),
            rules.ranking(),
        )
    }

    fn all_high_score_keys(&self) -> Vec<HighScoreKey> {
        game_high_score_keys(&self.title(), rustris::options::STAGE_NOUN)
    }

    fn settings(&self, themes: &Themes) -> MatchSettings {
        MatchSettings {
            rules: self.options.rules(),
            players: (0..self.options.players())
                .map(|_| PlayerSettings {
                    themes: themes.range(GameKind::Rustris),
                    theme_mode: self.options.theme_mode(),
                })
                .collect(),
            high_score_key: self.high_score_key(),
            playlist: false,
        }
    }

    fn games(&self) -> Result<Vec<AnyGame>, String> {
        Ok(self
            .options
            .games(self.options.players() as usize)
            .into_iter()
            .map(AnyGame::Rustris)
            .collect())
    }

    fn next_stage(&self, _: &Themes, _: u32, _: u32) -> Option<StageChange<AnyGame>> {
        None
    }

    fn controllers(&self) -> Vec<Controller> {
        let mut controllers: Vec<Controller> = vec![];
        for (player, key_delay) in self.options.ai_players() {
            let mut agent = rustris::game::ai::agent::AiAgent::default().with_key_delay(key_delay);
            controllers.push((
                player,
                Box::new(move |game: &mut AnyGame, delta| {
                    if let AnyGame::Rustris(game) = game {
                        agent.act(game, delta);
                    }
                }),
            ));
        }
        controllers
    }
}

// ---------------------------------------------------------------- Versus

const PLAYLIST: &str = "playlist";
const DIFFICULTY: &str = "difficulty";

/// How the two games are sequenced. Every player plays the same playlist, so it is always
/// fair. The theme race is a sprint: first to the end of the playlist wins. The other
/// playlists cycle endlessly as marathons: the highest score when everyone is out wins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Playlist {
    /// every theme of each game, the games taking turns; a race to the end of the playlist
    ThemeRace,
    /// the games take turns, each carrying on through its themes; a marathon
    Interleaved,
    /// all of one game, then all of the other; a marathon
    BackToBack,
}

impl Playlist {
    pub const ALL: [Playlist; 3] = [
        Playlist::ThemeRace,
        Playlist::Interleaved,
        Playlist::BackToBack,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Playlist::ThemeRace => "theme race",
            Playlist::Interleaved => "interleaved",
            Playlist::BackToBack => "back to back",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.name() == name)
    }

    /// a race to the end of the playlist rather than an endless marathon around it
    pub fn is_race(&self) -> bool {
        matches!(self, Playlist::ThemeRace)
    }

    /// the race ranks best times, the marathons rank highest scores
    pub fn ranking(&self) -> Ranking {
        if self.is_race() {
            Ranking::LowestTime
        } else {
            Ranking::HighestScore
        }
    }

    pub fn rules(&self, stage_count: usize) -> MatchRules {
        if self.is_race() {
            MatchRules::StageSprint {
                stages: stage_count as u32,
            }
        } else {
            MatchRules::Marathon
        }
    }

    /// which stage of the playlist a player is on after `completed` stages: the race ends
    /// with the playlist, the marathons cycle it forever
    fn stage_index(&self, completed: usize, stage_count: usize) -> Option<usize> {
        if stage_count == 0 {
            None
        } else if self.is_race() {
            (completed < stage_count).then_some(completed)
        } else {
            Some(completed % stage_count)
        }
    }

    /// the stages, as the game and theme of each
    pub fn stages(&self, themes_per_game: usize) -> Vec<(GameKind, ThemeMode)> {
        let order = [GameKind::Rustris, GameKind::DrRustario];
        match self {
            Playlist::ThemeRace => (0..themes_per_game)
                .flat_map(|theme| order.map(|game| (game, ThemeMode::Fixed(theme))))
                .collect(),
            Playlist::Interleaved => (0..themes_per_game)
                .flat_map(|_| order.map(|game| (game, ThemeMode::All)))
                .collect(),
            Playlist::BackToBack => order
                .into_iter()
                .flat_map(|game| (0..themes_per_game).map(move |theme| (game, ThemeMode::Fixed(theme))))
                .collect(),
        }
    }
}

/// One setting for both games' difficulty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];

    pub fn name(&self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Medium => "medium",
            Difficulty::Hard => "hard",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.name() == name)
    }

    fn rustris_level(&self) -> u32 {
        match self {
            Difficulty::Easy => 0,
            Difficulty::Medium => 3,
            Difficulty::Hard => 6,
        }
    }

    fn dr_rustario_level(&self) -> u32 {
        match self {
            Difficulty::Easy => 0,
            Difficulty::Medium => 5,
            Difficulty::Hard => 10,
        }
    }

    fn dr_rustario_speed(&self) -> dr_rustario::game::GameSpeed {
        match self {
            Difficulty::Easy => dr_rustario::game::GameSpeed::Low,
            Difficulty::Medium => dr_rustario::game::GameSpeed::Medium,
            Difficulty::Hard => dr_rustario::game::GameSpeed::High,
        }
    }
}

pub struct VersusMode {
    players: u32,
    playlist: Playlist,
    difficulty: Difficulty,
}

impl VersusMode {
    pub fn new() -> Self {
        Self {
            players: 1,
            playlist: Playlist::ThemeRace,
            difficulty: Difficulty::Easy,
        }
    }

    fn themes_per_game(themes: &Themes) -> usize {
        themes.dr_rustario.len().min(themes.rustris.len())
    }

    fn stages(&self, themes: &Themes) -> Vec<(GameKind, ThemeMode)> {
        self.playlist.stages(Self::themes_per_game(themes))
    }

    /// `count` games of a kind at this difficulty, sharing a seed
    fn new_games(&self, kind: GameKind, count: usize) -> Result<Vec<AnyGame>, String> {
        Ok(match kind {
            GameKind::DrRustario => {
                let mode = dr_rustario::game::random::RandomMode::Bag;
                dr_rustario::game::random::random(count, mode)
                    .into_iter()
                    .map(|rand| {
                        dr_rustario::game::Game::new(
                            self.difficulty.dr_rustario_level(),
                            self.difficulty.dr_rustario_speed(),
                            rand,
                        )
                        .map(AnyGame::DrRustario)
                    })
                    .collect::<Result<Vec<AnyGame>, String>>()?
            }
            GameKind::Rustris => {
                let mode = rustris::game::random::RandomMode::Bag;
                rustris::game::random::random_tetrominos(mode, count)
                    .into_iter()
                    .map(|rand| {
                        AnyGame::Rustris(rustris::game::Game::new(
                            self.difficulty.rustris_level(),
                            rand,
                        ))
                    })
                    .collect()
            }
        })
    }
}

impl Mode for VersusMode {
    fn title(&self) -> String {
        "Dr. Rustario vs. Rustris".to_string()
    }

    fn menu_sounds(&self) -> MenuSounds {
        MenuSounds::MODERN
    }

    fn race(&self, themes: &Themes) -> Vec<RaceTheme> {
        themes.race_all()
    }

    fn title_items(&self, max_players: u32) -> Vec<MenuItem> {
        players_item(max_players, self.players).into_iter().collect()
    }

    fn title_select(&mut self, name: &str, value: &str) {
        if name == PLAYERS {
            self.players = value.parse::<u32>().unwrap_or(1);
        }
    }

    fn menu_items(&self) -> Vec<MenuItem> {
        vec![
            MenuItem::select_list(
                PLAYLIST,
                Playlist::ALL.iter().map(|p| p.name().to_string()).collect(),
                Playlist::ALL
                    .iter()
                    .position(|p| *p == self.playlist)
                    .unwrap_or(0),
            ),
            MenuItem::select_list(
                DIFFICULTY,
                Difficulty::ALL.iter().map(|d| d.name().to_string()).collect(),
                Difficulty::ALL
                    .iter()
                    .position(|d| *d == self.difficulty)
                    .unwrap_or(0),
            ),
        ]
    }

    fn menu_select(&mut self, name: &str, value: &str) {
        match name {
            PLAYLIST => {
                if let Some(playlist) = Playlist::from_name(value) {
                    self.playlist = playlist;
                }
            }
            DIFFICULTY => {
                if let Some(difficulty) = Difficulty::from_name(value) {
                    self.difficulty = difficulty;
                }
            }
            _ => {}
        }
    }

    fn subtitle(&self) -> String {
        subtitle("", self.players).trim().to_string()
    }

    fn high_score_key(&self) -> HighScoreKey {
        HighScoreKey::new(self.title(), self.playlist.name(), self.playlist.ranking())
    }

    fn all_high_score_keys(&self) -> Vec<HighScoreKey> {
        Playlist::ALL
            .iter()
            .map(|playlist| HighScoreKey::new(self.title(), playlist.name(), playlist.ranking()))
            .collect()
    }

    fn settings(&self, themes: &Themes) -> MatchSettings {
        let stages = self.stages(themes);
        let (first, theme_mode) = stages[0];
        MatchSettings {
            rules: self.playlist.rules(stages.len()),
            players: (0..self.players)
                .map(|_| PlayerSettings {
                    themes: themes.range(first),
                    theme_mode,
                })
                .collect(),
            high_score_key: self.high_score_key(),
            playlist: true,
        }
    }

    fn games(&self) -> Result<Vec<AnyGame>, String> {
        self.new_games(GameKind::Rustris, self.players as usize)
    }

    fn next_stage(
        &self,
        themes: &Themes,
        _player: u32,
        completed: u32,
    ) -> Option<StageChange<AnyGame>> {
        let stages = self.stages(themes);
        let index = self.playlist.stage_index(completed as usize, stages.len())?;
        let (kind, theme_mode) = stages[index];
        let previous = completed
            .checked_sub(1)
            .and_then(|i| self.playlist.stage_index(i as usize, stages.len()))
            .map(|i| stages[i].0);
        // the same game again keeps its board and hold: only the theme changes
        let game = if previous == Some(kind) {
            None
        } else {
            Some(self.new_games(kind, 1).ok()?.pop()?)
        };
        Some(StageChange {
            game,
            settings: PlayerSettings {
                themes: themes.range(kind),
                theme_mode,
            },
        })
    }

    fn controllers(&self) -> Vec<Controller> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_race_alternates_games_through_every_theme() {
        let stages = Playlist::ThemeRace.stages(4);
        assert_eq!(stages.len(), 8);
        assert_eq!(stages[0], (GameKind::Rustris, ThemeMode::Fixed(0)));
        assert_eq!(stages[1], (GameKind::DrRustario, ThemeMode::Fixed(0)));
        assert_eq!(stages[7], (GameKind::DrRustario, ThemeMode::Fixed(3)));
    }

    #[test]
    fn every_mode_has_one_table_per_rules_variant() {
        let rustris = RustrisMode::new();
        let keys = rustris.all_high_score_keys();
        assert_eq!(
            keys.iter().map(|k| k.mode.as_str()).collect::<Vec<_>>(),
            vec!["marathon", "1 level sprint", "theme sprint", "10,000 point sprint"]
        );
        assert!(keys.contains(&rustris.high_score_key()));
        assert_eq!(DrRustarioMode::new().all_high_score_keys().len(), 4);

        let versus = VersusMode::new();
        let keys = versus.all_high_score_keys();
        assert_eq!(
            keys.iter().map(|k| k.mode.as_str()).collect::<Vec<_>>(),
            vec!["theme race", "interleaved", "back to back"]
        );
        assert_eq!(keys[0].ranking, Ranking::LowestTime);
        assert_eq!(keys[1].ranking, Ranking::HighestScore);
        assert_eq!(keys[2].ranking, Ranking::HighestScore);
        assert!(keys.contains(&versus.high_score_key()));
    }

    #[test]
    fn only_the_theme_race_is_a_sprint() {
        assert_eq!(
            Playlist::ThemeRace.rules(8),
            MatchRules::StageSprint { stages: 8 }
        );
        assert_eq!(Playlist::Interleaved.rules(8), MatchRules::Marathon);
        assert_eq!(Playlist::BackToBack.rules(8), MatchRules::Marathon);
    }

    #[test]
    fn the_race_ends_with_the_playlist_and_marathons_cycle_it() {
        assert_eq!(Playlist::ThemeRace.stage_index(7, 8), Some(7));
        assert_eq!(Playlist::ThemeRace.stage_index(8, 8), None);
        assert_eq!(Playlist::BackToBack.stage_index(7, 8), Some(7));
        assert_eq!(Playlist::BackToBack.stage_index(8, 8), Some(0));
        assert_eq!(Playlist::Interleaved.stage_index(13, 8), Some(5));
        assert_eq!(Playlist::Interleaved.stage_index(0, 0), None);
    }

    #[test]
    fn back_to_back_plays_one_game_then_the_other() {
        let stages = Playlist::BackToBack.stages(4);
        assert!(stages[..4].iter().all(|(g, _)| *g == GameKind::Rustris));
        assert!(stages[4..].iter().all(|(g, _)| *g == GameKind::DrRustario));
    }
}
