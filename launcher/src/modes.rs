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
use std::cell::Cell;
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
        for (player, key_delay, network) in self.options.ai_players() {
            let mut agent =
                rustris::game::ai::agent::AiAgent::neural(network).with_key_delay(key_delay);
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
/// fair: the random playlists are dealt once per match, and every player faces the same
/// sequence. The theme race and the random sprints race to the end of the playlist. The
/// other playlists cycle endlessly as marathons: the highest score when everyone is out
/// wins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Playlist {
    /// every theme of each game, the games taking turns; a race to the end of the playlist
    ThemeRace,
    /// the games take turns, each carrying on through its themes; a marathon
    Interleaved,
    /// all of one game, then all of the other; a marathon
    BackToBack,
    /// a random game and theme each stage; a race over this many stages
    RandomSprint { stages: u32 },
    /// a random game and theme each stage, forever; a marathon
    RandomMarathon,
}

impl Playlist {
    pub const ALL: [Playlist; 7] = [
        Playlist::ThemeRace,
        Playlist::Interleaved,
        Playlist::BackToBack,
        Playlist::RandomSprint { stages: 3 },
        Playlist::RandomSprint { stages: 5 },
        Playlist::RandomSprint { stages: 10 },
        Playlist::RandomMarathon,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Playlist::ThemeRace => "theme race",
            Playlist::Interleaved => "interleaved marathon",
            Playlist::BackToBack => "back to back marathon",
            Playlist::RandomSprint { stages: 3 } => "3 level random sprint",
            Playlist::RandomSprint { stages: 5 } => "5 level random sprint",
            Playlist::RandomSprint { stages: 10 } => "10 level random sprint",
            Playlist::RandomSprint { .. } => "random sprint",
            Playlist::RandomMarathon => "random marathon",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.name() == name)
    }

    /// a race to the end of the playlist rather than an endless marathon around it
    pub fn is_race(&self) -> bool {
        matches!(self, Playlist::ThemeRace | Playlist::RandomSprint { .. })
    }

    /// the races rank best times, the marathons rank highest scores
    pub fn ranking(&self) -> Ranking {
        if self.is_race() {
            Ranking::LowestTime
        } else {
            Ranking::HighestScore
        }
    }

    /// how many stages the playlist deals, or `None` for the endless random marathon
    fn stage_count(&self, themes_per_game: usize) -> Option<usize> {
        match self {
            Playlist::RandomSprint { stages } => Some(*stages as usize),
            Playlist::RandomMarathon => None,
            _ => Some(2 * themes_per_game),
        }
    }

    pub fn rules(&self, themes_per_game: usize) -> MatchRules {
        if self.is_race() {
            MatchRules::StageSprint {
                stages: self.stage_count(themes_per_game).unwrap_or(0) as u32,
            }
        } else {
            MatchRules::Marathon
        }
    }

    /// which stage of the playlist a player is on after `completed` stages: the races end
    /// with the playlist, the fixed marathons cycle it forever and the random marathon
    /// never repeats
    fn stage_index(&self, completed: usize, themes_per_game: usize) -> Option<usize> {
        match self.stage_count(themes_per_game) {
            Some(0) => None,
            Some(count) if self.is_race() => (completed < count).then_some(completed),
            Some(count) => Some(completed % count),
            None => Some(completed),
        }
    }

    /// the game and theme a player is on after `completed` stages, or `None` when a race
    /// has been run
    pub fn stage(
        &self,
        seed: u64,
        completed: usize,
        themes_per_game: usize,
    ) -> Option<(GameKind, ThemeMode)> {
        let index = self.stage_index(completed, themes_per_game)?;
        Some(match self {
            Playlist::RandomSprint { .. } | Playlist::RandomMarathon => {
                random_stage(seed, index, themes_per_game)
            }
            _ => self.fixed_stages(themes_per_game)[index],
        })
    }

    /// which game the playlist opens with; the seed only matters to the random playlists
    pub fn first_game(&self, seed: u64) -> GameKind {
        match self {
            Playlist::RandomSprint { .. } | Playlist::RandomMarathon => random_game(seed, 0),
            // every fixed playlist starts on Rustris
            _ => GameKind::Rustris,
        }
    }

    /// the stages of the fixed playlists, as the game and theme of each; the random
    /// playlists are dealt by [`random_stage`] instead
    fn fixed_stages(&self, themes_per_game: usize) -> Vec<(GameKind, ThemeMode)> {
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
            Playlist::RandomSprint { .. } | Playlist::RandomMarathon => vec![],
        }
    }
}

/// splitmix64: spreads a seed and stage index into independent random rolls
fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// one roll of a random playlist's deal: `salt` 0 picks the game, 1 the theme
fn stage_roll(seed: u64, index: usize, salt: u64) -> u64 {
    splitmix64(seed ^ splitmix64(2 * index as u64 + salt))
}

/// the game dealt to a random playlist's stage; independent of the theme count, so the
/// opening game is known before the themes are
fn random_game(seed: u64, index: usize) -> GameKind {
    if stage_roll(seed, index, 0) & 1 == 0 {
        GameKind::Rustris
    } else {
        GameKind::DrRustario
    }
}

/// the stage a random playlist deals: a random game and theme, never repeating the exact
/// game and theme of the stage before, so every stage change changes something on screen
fn random_stage(seed: u64, index: usize, themes_per_game: usize) -> (GameKind, ThemeMode) {
    let mut previous: Option<(GameKind, usize)> = None;
    for i in 0..=index {
        let game = random_game(seed, i);
        let mut theme = if themes_per_game == 0 {
            0
        } else {
            (stage_roll(seed, i, 1) % themes_per_game as u64) as usize
        };
        if themes_per_game > 1 && previous == Some((game, theme)) {
            theme = (theme + 1) % themes_per_game;
        }
        previous = Some((game, theme));
    }
    let (game, theme) = previous.unwrap();
    (game, ThemeMode::Fixed(theme))
}

/// One 0-10 dial for both games, shared by every playlist: it sets Dr. Rustario's virus
/// level and fall speed and Rustris's starting level together. 0 is the gentlest start
/// (no viruses, level 0, low fall speed) and each step up adds a virus level and a
/// Rustris level, with the fall speed stepping up along the way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Difficulty(u32);

impl Difficulty {
    pub const MAX: u32 = 10;

    pub fn new(difficulty: u32) -> Self {
        Self(difficulty.min(Self::MAX))
    }

    pub fn names() -> Vec<String> {
        (0..=Self::MAX).map(|d| d.to_string()).collect()
    }

    pub fn from_name(name: &str) -> Option<Self> {
        name.parse::<u32>()
            .ok()
            .filter(|d| *d <= Self::MAX)
            .map(Self::new)
    }

    /// one Rustris starting level per step (the guideline fall speed curve runs to level
    /// 14, so even 10 leaves headroom)
    fn rustris_level(&self) -> u32 {
        self.0
    }

    /// one virus level per step
    fn dr_rustario_level(&self) -> u32 {
        self.0
    }

    /// low up to 3, medium up to 7, high from 8
    fn dr_rustario_speed(&self) -> dr_rustario::game::GameSpeed {
        match self.0 {
            0..=3 => dr_rustario::game::GameSpeed::Low,
            4..=7 => dr_rustario::game::GameSpeed::Medium,
            _ => dr_rustario::game::GameSpeed::High,
        }
    }
}

pub struct VersusMode {
    players: u32,
    playlist: Playlist,
    difficulty: Difficulty,
    /// what the random playlists are dealt from: re-rolled as each match starts, so every
    /// player of one match faces the same random sequence
    seed: Cell<u64>,
}

impl VersusMode {
    pub fn new() -> Self {
        Self {
            players: 1,
            playlist: Playlist::ThemeRace,
            difficulty: Difficulty::default(),
            seed: Cell::new(0),
        }
    }

    fn themes_per_game(themes: &Themes) -> usize {
        themes.dr_rustario.len().min(themes.rustris.len())
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
            MenuItem::select_list(DIFFICULTY, Difficulty::names(), self.difficulty.0 as usize),
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
        let themes_per_game = Self::themes_per_game(themes);
        let (first, theme_mode) = self
            .playlist
            .stage(self.seed.get(), 0, themes_per_game)
            .expect("every playlist opens with a stage");
        MatchSettings {
            rules: self.playlist.rules(themes_per_game),
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
        let seed = engine::game::random::Seed::random();
        self.seed
            .set(u64::from_le_bytes(seed.bytes()[..8].try_into().unwrap()));
        self.new_games(
            self.playlist.first_game(self.seed.get()),
            self.players as usize,
        )
    }

    fn next_stage(
        &self,
        themes: &Themes,
        _player: u32,
        completed: u32,
    ) -> Option<StageChange<AnyGame>> {
        let themes_per_game = Self::themes_per_game(themes);
        let seed = self.seed.get();
        let (kind, theme_mode) = self
            .playlist
            .stage(seed, completed as usize, themes_per_game)?;
        let previous = completed
            .checked_sub(1)
            .and_then(|i| self.playlist.stage(seed, i as usize, themes_per_game))
            .map(|(kind, _)| kind);
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

    fn stages(playlist: Playlist, seed: u64, count: usize) -> Vec<(GameKind, ThemeMode)> {
        (0..count)
            .map(|i| playlist.stage(seed, i, 4).unwrap())
            .collect()
    }

    #[test]
    fn theme_race_alternates_games_through_every_theme() {
        let stages = stages(Playlist::ThemeRace, 0, 8);
        assert_eq!(stages[0], (GameKind::Rustris, ThemeMode::Fixed(0)));
        assert_eq!(stages[1], (GameKind::DrRustario, ThemeMode::Fixed(0)));
        assert_eq!(stages[7], (GameKind::DrRustario, ThemeMode::Fixed(3)));
        assert_eq!(Playlist::ThemeRace.stage(0, 8, 4), None);
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
            vec![
                "theme race",
                "interleaved",
                "back to back",
                "3 level random sprint",
                "5 level random sprint",
                "10 level random sprint",
                "random marathon"
            ]
        );
        assert_eq!(
            keys.iter().map(|k| k.ranking).collect::<Vec<_>>(),
            vec![
                Ranking::LowestTime,
                Ranking::HighestScore,
                Ranking::HighestScore,
                Ranking::LowestTime,
                Ranking::LowestTime,
                Ranking::LowestTime,
                Ranking::HighestScore
            ]
        );
        assert!(keys.contains(&versus.high_score_key()));
    }

    #[test]
    fn only_the_races_are_sprints() {
        assert_eq!(
            Playlist::ThemeRace.rules(4),
            MatchRules::StageSprint { stages: 8 }
        );
        assert_eq!(
            Playlist::RandomSprint { stages: 5 }.rules(4),
            MatchRules::StageSprint { stages: 5 }
        );
        assert_eq!(Playlist::Interleaved.rules(4), MatchRules::Marathon);
        assert_eq!(Playlist::BackToBack.rules(4), MatchRules::Marathon);
        assert_eq!(Playlist::RandomMarathon.rules(4), MatchRules::Marathon);
    }

    #[test]
    fn the_races_end_with_the_playlist_and_marathons_cycle_it() {
        assert!(Playlist::ThemeRace.stage(0, 7, 4).is_some());
        assert_eq!(Playlist::ThemeRace.stage(0, 8, 4), None);
        assert_eq!(Playlist::RandomSprint { stages: 3 }.stage(0, 3, 4), None);
        assert_eq!(
            Playlist::BackToBack.stage(0, 8, 4),
            Playlist::BackToBack.stage(0, 0, 4)
        );
        assert_eq!(
            Playlist::Interleaved.stage(0, 13, 4),
            Playlist::Interleaved.stage(0, 5, 4)
        );
        assert!(Playlist::RandomMarathon.stage(0, 10_000, 4).is_some());
        assert_eq!(Playlist::ThemeRace.stage(0, 0, 0), None);
    }

    #[test]
    fn back_to_back_plays_one_game_then_the_other() {
        let stages = stages(Playlist::BackToBack, 0, 8);
        assert!(stages[..4].iter().all(|(g, _)| *g == GameKind::Rustris));
        assert!(stages[4..].iter().all(|(g, _)| *g == GameKind::DrRustario));
    }

    #[test]
    fn random_playlists_deal_the_same_stages_for_one_seed() {
        let first = stages(Playlist::RandomMarathon, 42, 32);
        assert_eq!(first, stages(Playlist::RandomMarathon, 42, 32));
        assert_ne!(first, stages(Playlist::RandomMarathon, 43, 32));
        // a sprint over the same seed deals the same opening stages
        assert_eq!(first[..3], stages(Playlist::RandomSprint { stages: 3 }, 42, 3));
    }

    #[test]
    fn random_playlists_open_with_the_game_they_deal_first() {
        for seed in 0..32 {
            let (game, _) = Playlist::RandomMarathon.stage(seed, 0, 4).unwrap();
            assert_eq!(Playlist::RandomMarathon.first_game(seed), game);
            assert_eq!(Playlist::RandomSprint { stages: 3 }.first_game(seed), game);
        }
    }

    #[test]
    fn random_playlists_pick_both_games_and_every_theme() {
        let stages = stages(Playlist::RandomMarathon, 7, 256);
        for game in [GameKind::Rustris, GameKind::DrRustario] {
            for theme in 0..4 {
                assert!(stages.contains(&(game, ThemeMode::Fixed(theme))));
            }
        }
    }

    #[test]
    fn random_playlists_never_deal_the_same_stage_twice_in_a_row() {
        for seed in 0..32 {
            let stages = stages(Playlist::RandomMarathon, seed, 64);
            for pair in stages.windows(2) {
                assert_ne!(pair[0], pair[1], "seed {}", seed);
            }
        }
    }

    #[test]
    fn difficulty_ramps_both_games_together() {
        let easiest = Difficulty::new(0);
        assert_eq!(easiest.rustris_level(), 0);
        assert_eq!(easiest.dr_rustario_level(), 0);
        assert_eq!(
            easiest.dr_rustario_speed(),
            dr_rustario::game::GameSpeed::Low
        );
        assert_eq!(easiest, Difficulty::default());

        let hardest = Difficulty::new(10);
        assert_eq!(hardest.rustris_level(), 10);
        assert_eq!(hardest.dr_rustario_level(), 10);
        assert_eq!(
            hardest.dr_rustario_speed(),
            dr_rustario::game::GameSpeed::High
        );

        assert_eq!(
            Difficulty::new(5).dr_rustario_speed(),
            dr_rustario::game::GameSpeed::Medium
        );
        // the dial stops at 10
        assert_eq!(Difficulty::new(99), hardest);
        assert_eq!(Difficulty::from_name("11"), None);
        assert_eq!(Difficulty::from_name("7"), Some(Difficulty::new(7)));
        assert_eq!(Difficulty::names().len(), 11);
    }
}
