//! What the launcher can run: each game on its own, and a versus mode where every player
//! plays the same playlist over the games.

use crate::games::{AiBrain, AnyGame, GameKind, PerGame};
use engine::app::{MatchSettings, PlayerSettings, StageChange, ThemeMode};
use engine::high_score::HighScoreKey;
use engine::menu::sound::MenuSounds;
use engine::menu::MenuItem;
use engine::particles::prescribed::RaceTheme;
use engine::render::{Theme, ThemeFamily};
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
    ranges: PerGame<Range<usize>>,
}

impl<'a> Themes<'a> {
    /// every game's themes concatenated, each game's slice of the list recorded
    pub fn new(all: Vec<Theme<'a>>, ranges: PerGame<Range<usize>>) -> Self {
        Self { all, ranges }
    }

    pub fn range(&self, game: GameKind) -> Range<usize> {
        self.ranges.get(game).clone()
    }

    pub fn race(&self, game: GameKind) -> Vec<RaceTheme> {
        let range = self.range(game);
        let themes = &self.all[range.clone()];
        // each game numbers its race themes within its own slice of the whole list
        let mut race = match game {
            GameKind::DrRustario => dr_rustario::theme::race_themes(themes),
            GameKind::Rustris => rustris::theme::race_themes(themes),
            GameKind::Puyo => puyo_rusto::theme::race_themes(themes),
            #[cfg(feature = "rustle-fighter")]
            GameKind::RustleFighter => rustle_fighter::theme::race_themes(themes),
        };
        for theme in race.iter_mut() {
            theme.theme += range.start;
        }
        race
    }

    /// the themes of a game in a family, as indices within that game's own set; every
    /// theme when no family is asked for
    pub fn family(&self, game: GameKind, family: Option<ThemeFamily>) -> Vec<usize> {
        self.all[self.range(game)]
            .iter()
            .enumerate()
            .filter(|(_, theme)| family.is_none_or(|f| theme.family() == f))
            .map(|(index, _)| index)
            .collect()
    }

    /// what a playlist over this family of themes has to deal, for the games it deals
    pub fn playlist(&self, family: Option<ThemeFamily>, dealt: &Dealt) -> PlaylistThemes {
        PlaylistThemes::new(
            PerGame::new(|game| self.family(game, family)),
            dealt.clone(),
        )
    }

    pub fn race_all(&self) -> Vec<RaceTheme> {
        GameKind::ALL
            .into_iter()
            .flat_map(|game| self.race(game))
            .collect()
    }
}

/// The games a playlist deals, in order: [`GameKind::PLAYLIST_ORDER`] less the games the vs.
/// menu has unticked. Everything that deals a stage reads this rather than `PLAYLIST_ORDER`, so
/// turn order, theme slots, stage count and random rolls all narrow together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dealt(Vec<GameKind>);

/// every game the playlist can deal, which is where [`GameSelection`] starts
impl Default for Dealt {
    fn default() -> Self {
        Self(GameKind::PLAYLIST_ORDER.to_vec())
    }
}

impl Dealt {
    pub fn new(games: Vec<GameKind>) -> Self {
        Self(games)
    }

    /// how many games take a turn, which is one round of a fixed playlist
    pub fn count(&self) -> usize {
        self.0.len()
    }

    pub fn games(&self) -> impl Iterator<Item = GameKind> + '_ {
        self.0.iter().copied()
    }

    /// the game whose turn it is at `turn` of a round
    fn turn(&self, turn: usize) -> GameKind {
        self.0[turn % self.0.len()]
    }

    /// the game a playlist opens with; the fallback only covers an empty list, which cannot
    /// be started
    fn first(&self) -> GameKind {
        self.0
            .first()
            .copied()
            .unwrap_or(GameKind::PLAYLIST_ORDER[0])
    }
}

/// The themes a playlist deals, as indices within each game's own set, and the games it
/// deals them to. At slot `n` every game plays its `n`th theme; the playlist is as long as the
/// longest list and a game with fewer themes replays its own from the start.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlaylistThemes {
    themes: PerGame<Vec<usize>>,
    dealt: Dealt,
}

impl PlaylistThemes {
    pub fn new(themes: PerGame<Vec<usize>>, dealt: Dealt) -> Self {
        Self { themes, dealt }
    }

    /// the games this playlist takes turns between
    pub fn dealt(&self) -> &Dealt {
        &self.dealt
    }

    /// the longest theme list among the dealt games; an undealt game does not count
    pub fn slots(&self) -> usize {
        self.dealt
            .games()
            .map(|game| self.themes.get(game).len())
            .max()
            .unwrap_or(0)
    }

    /// whether every dealt game has a theme of this family; one with none cannot take its turn
    fn covers_every_game(&self) -> bool {
        self.dealt.count() > 0 && self.dealt.games().all(|g| !self.themes.get(g).is_empty())
    }

    /// The theme a game plays at a slot, within its own set, wrapping round when it has fewer.
    fn theme(&self, game: GameKind, slot: usize) -> usize {
        let themes = self.themes.get(game);
        themes.get(slot % themes.len().max(1)).copied().unwrap_or(0)
    }
}

pub type Controller = (u32, Box<dyn FnMut(&mut AnyGame, Duration) + 'static>);

/// A launcher mode: its menus, and how it builds a match.
pub trait Mode {
    /// persisted verbatim as `HighScoreKey`'s game, so renaming a ranked mode orphans its scores
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
    /// The table the current options compete for, one per game and rules mode; start level,
    /// speed and difficulty share it. `None` is a mode that never ranks, the vs. playlist.
    fn high_score_key(&self) -> Option<HighScoreKey>;
    /// every table this mode can compete for, whatever the options; empty for a mode that
    /// does not rank
    fn all_high_score_keys(&self) -> Vec<HighScoreKey>;
    fn settings(&self, themes: &Themes) -> MatchSettings;
    fn games(&self) -> Result<Vec<AnyGame>, String>;
    fn next_stage(
        &self,
        themes: &Themes,
        player: u32,
        completed: u32,
    ) -> Option<StageChange<AnyGame>>;
    fn controllers(&self) -> Vec<Controller>;
}

/// The mode that plays one game on its own; the only place that names them, so the pre-menu,
/// high score screen and tests offer the same set.
pub fn game_mode(game: GameKind) -> Box<dyn Mode> {
    match game {
        GameKind::DrRustario => Box::new(DrRustarioMode::new()),
        GameKind::Rustris => Box::new(RustrisMode::new()),
        GameKind::Puyo => Box::new(PuyoMode::new()),
        #[cfg(feature = "rustle-fighter")]
        GameKind::RustleFighter => Box::new(RustleFighterMode::new()),
    }
}

/// one table per rules variant of a game
fn game_high_score_keys(game: &str, stage_noun: &str) -> Vec<HighScoreKey> {
    MatchRules::MODES
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

    fn high_score_key(&self) -> Option<HighScoreKey> {
        let rules = self.options.rules();
        Some(HighScoreKey::new(
            self.title(),
            rules.name(dr_rustario::options::STAGE_NOUN),
            rules.ranking(),
        ))
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
        let mut controllers: Vec<Controller> = vec![];
        for (player, key_delay, brain) in self.options.ai_players() {
            let mut agent =
                dr_rustario::game::ai::agent::DrAiAgent::of(brain).with_key_delay(key_delay);
            controllers.push((
                player,
                Box::new(move |game: &mut AnyGame, delta| {
                    if let AnyGame::DrRustario(game) = game {
                        agent.act(game, delta);
                    }
                }),
            ));
        }
        controllers
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

    fn high_score_key(&self) -> Option<HighScoreKey> {
        let rules = self.options.rules();
        Some(HighScoreKey::new(
            self.title(),
            rules.name(rustris::options::STAGE_NOUN),
            rules.ranking(),
        ))
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

// ---------------------------------------------------------------- Puyo Rusto

#[derive(Default)]
pub struct PuyoMode {
    options: puyo_rusto::options::Options,
}

impl PuyoMode {
    pub fn new() -> Self {
        let mut mode = Self::default();
        mode.options.set_players(1);
        mode
    }
}

impl Mode for PuyoMode {
    fn title(&self) -> String {
        "Puyo Rusto".to_string()
    }

    fn menu_sounds(&self) -> MenuSounds {
        puyo_rusto::theme::MENU_SOUNDS
    }

    fn race(&self, themes: &Themes) -> Vec<RaceTheme> {
        themes.race(GameKind::Puyo)
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

    fn high_score_key(&self) -> Option<HighScoreKey> {
        let rules = self.options.rules();
        Some(HighScoreKey::new(
            self.title(),
            rules.name(puyo_rusto::options::STAGE_NOUN),
            rules.ranking(),
        ))
    }

    fn all_high_score_keys(&self) -> Vec<HighScoreKey> {
        game_high_score_keys(&self.title(), puyo_rusto::options::STAGE_NOUN)
    }

    fn settings(&self, themes: &Themes) -> MatchSettings {
        MatchSettings {
            rules: self.options.rules(),
            players: (0..self.options.players())
                .map(|_| PlayerSettings {
                    themes: themes.range(GameKind::Puyo),
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
            .map(AnyGame::Puyo)
            .collect())
    }

    fn next_stage(&self, _: &Themes, _: u32, _: u32) -> Option<StageChange<AnyGame>> {
        None
    }

    fn controllers(&self) -> Vec<Controller> {
        let mut controllers: Vec<Controller> = vec![];
        for (player, key_delay, brain) in self.options.ai_players() {
            let mut agent =
                puyo_rusto::game::ai::agent::PuyoAiAgent::of(brain).with_key_delay(key_delay);
            controllers.push((
                player,
                Box::new(move |game: &mut AnyGame, delta| {
                    if let AnyGame::Puyo(game) = game {
                        agent.act(game, delta);
                    }
                }),
            ));
        }
        controllers
    }
}

/// The mode that plays Super Rustle Fighter on its own. It has no ai controllers and no
/// playlist turn.
#[cfg(feature = "rustle-fighter")]
pub struct RustleFighterMode {
    options: rustle_fighter::options::Options,
}

#[cfg(feature = "rustle-fighter")]
impl RustleFighterMode {
    pub fn new() -> Self {
        Self {
            options: rustle_fighter::options::Options::default(),
        }
    }
}

#[cfg(feature = "rustle-fighter")]
impl Default for RustleFighterMode {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "rustle-fighter")]
impl Mode for RustleFighterMode {
    fn title(&self) -> String {
        "Super Rustle Fighter".to_string()
    }

    fn menu_sounds(&self) -> MenuSounds {
        rustle_fighter::theme::MENU_SOUNDS
    }

    fn race(&self, themes: &Themes) -> Vec<RaceTheme> {
        themes.race(GameKind::RustleFighter)
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

    fn high_score_key(&self) -> Option<HighScoreKey> {
        let rules = self.options.rules();
        Some(HighScoreKey::new(
            self.title(),
            rules.name(rustle_fighter::options::STAGE_NOUN),
            rules.ranking(),
        ))
    }

    fn all_high_score_keys(&self) -> Vec<HighScoreKey> {
        game_high_score_keys(&self.title(), rustle_fighter::options::STAGE_NOUN)
    }

    fn settings(&self, themes: &Themes) -> MatchSettings {
        MatchSettings {
            rules: self.options.rules(),
            players: (0..self.options.players())
                .map(|_| PlayerSettings {
                    themes: themes.range(GameKind::RustleFighter),
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
            .map(AnyGame::RustleFighter)
            .collect())
    }

    fn next_stage(&self, _: &Themes, _: u32, _: u32) -> Option<StageChange<AnyGame>> {
        None
    }

    fn controllers(&self) -> Vec<Controller> {
        vec![]
    }
}

// ---------------------------------------------------------------- Versus

const PLAYLIST: &str = "playlist";
const DIFFICULTY: &str = "difficulty";
const VS_AI_PREFIX: &str = "vs ";
const VS_AI_SUFFIX: &str = " ai";
const AI_DEMO_1P: &str = "1-player ai demo";
const AI_DEMO_2P: &str = "2-player ai demo";

/// A versus ai difficulty is each game's own difficulty of the same name; every game declares
/// the same four names, which [`ai_difficulties_agree`] pins.
pub type AiDifficulty = dr_rustario::game::rules::AiDifficulty;

/// Who is playing a versus match. An ai player is a brain per game, each that game's own ai
/// for the mode chosen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VersusAi {
    /// no ai players
    #[default]
    Off,
    /// one board played by the ai at full speed
    Demo,
    /// two boards played by the ai at full speed, each game fielding its own 2-player demo pair
    VsDemo,
    /// player 2 is the ai
    Opponent(AiDifficulty),
}

impl VersusAi {
    /// how many boards the match runs, or `None` when the players dial decides
    fn players(&self) -> Option<u32> {
        match self {
            VersusAi::Off => None,
            VersusAi::Demo => Some(1),
            VersusAi::VsDemo | VersusAi::Opponent(_) => Some(2),
        }
    }

    fn dr_rustario(&self) -> dr_rustario::game::rules::AiMode {
        use dr_rustario::game::rules::AiMode;
        match self {
            VersusAi::Off => AiMode::Off,
            VersusAi::Demo => AiMode::Demo,
            VersusAi::VsDemo => AiMode::VsDemo,
            VersusAi::Opponent(difficulty) => AiMode::Opponent(*difficulty),
        }
    }

    fn rustris(&self) -> rustris::game::rules::AiMode {
        use rustris::game::rules::AiMode;
        match self {
            VersusAi::Off => AiMode::Off,
            VersusAi::Demo => AiMode::Demo,
            VersusAi::VsDemo => AiMode::VsDemo,
            VersusAi::Opponent(difficulty) => AiMode::Opponent(
                rustris::game::rules::AiDifficulty::from_name(difficulty.name())
                    .expect("every game offers the same ai difficulties"),
            ),
        }
    }

    fn puyo(&self) -> puyo_rusto::game::rules::AiMode {
        use puyo_rusto::game::rules::AiMode;
        match self {
            VersusAi::Off => AiMode::Off,
            VersusAi::Demo => AiMode::Demo,
            VersusAi::VsDemo => AiMode::VsDemo,
            VersusAi::Opponent(difficulty) => AiMode::Opponent(
                puyo_rusto::game::rules::AiDifficulty::from_name(difficulty.name())
                    .expect("every game offers the same ai difficulties"),
            ),
        }
    }

    /// the name this mode goes by in the players list
    fn name(&self) -> Option<String> {
        match self {
            VersusAi::Off => None,
            VersusAi::Demo => Some(AI_DEMO_1P.to_string()),
            VersusAi::VsDemo => Some(AI_DEMO_2P.to_string()),
            VersusAi::Opponent(difficulty) => Some(format!(
                "{}{}{}",
                VS_AI_PREFIX,
                difficulty.name(),
                VS_AI_SUFFIX
            )),
        }
    }

    /// a pick from the players list, or `None` for a number of humans
    fn from_name(value: &str) -> Option<Self> {
        if value == AI_DEMO_1P {
            Some(VersusAi::Demo)
        } else if value == AI_DEMO_2P {
            Some(VersusAi::VsDemo)
        } else {
            value
                .strip_prefix(VS_AI_PREFIX)
                .and_then(|s| s.strip_suffix(VS_AI_SUFFIX))
                .and_then(AiDifficulty::from_name)
                .map(VersusAi::Opponent)
        }
    }

    /// the ai players one game fields for this mode: each one's board, and the game's own
    /// brain at the game's own key rate
    pub(crate) fn ai_players(&self, game: GameKind) -> Vec<(u32, Box<dyn AiBrain>)> {
        match game {
            GameKind::DrRustario => {
                let mut config = dr_rustario::game::rules::GameConfig::default();
                config.set_ai(self.dr_rustario());
                config
                    .ai_players()
                    .into_iter()
                    .map(|(player, key_delay, brain)| {
                        (player, crate::games::dr_rustario_brain(brain, key_delay))
                    })
                    .collect()
            }
            GameKind::Rustris => {
                let config = rustris::game::rules::GameConfig {
                    ai: self.rustris(),
                    ..Default::default()
                };
                config
                    .ai_players()
                    .into_iter()
                    .map(|(player, key_delay, network)| {
                        (player, crate::games::rustris_brain(network, key_delay))
                    })
                    .collect()
            }
            GameKind::Puyo => {
                let config = puyo_rusto::game::rules::GameConfig {
                    ai: self.puyo(),
                    ..Default::default()
                };
                config
                    .ai_players()
                    .into_iter()
                    .map(|(player, key_delay, brain)| {
                        (player, crate::games::puyo_brain(brain, key_delay))
                    })
                    .collect()
            }
            // no ai; not on `PLAYLIST_ORDER` either, so no playlist reaches this arm
            #[cfg(feature = "rustle-fighter")]
            GameKind::RustleFighter => vec![],
        }
    }

    /// Every ai player of this mode, with one brain from each game's list. The games agree on
    /// the boards each mode's ai plays, which [`every_mode_offers_the_same_ai_opponents_and_demos`]
    /// pins.
    fn brains(&self) -> Vec<(u32, Vec<Box<dyn AiBrain>>)> {
        // a game with no ai would take the `min` below to zero and drop every ai player
        let mut per_game: Vec<std::vec::IntoIter<(u32, Box<dyn AiBrain>)>> = GameKind::ALL
            .into_iter()
            .filter(|game| game.fields_an_ai())
            .map(|game| self.ai_players(game).into_iter())
            .collect();
        let players = per_game.iter().map(|game| game.len()).min().unwrap_or(0);
        (0..players)
            .map(|_| {
                let dealt = per_game
                    .iter_mut()
                    .map(|game| game.next().expect("a brain per game per player"))
                    .collect::<Vec<(u32, Box<dyn AiBrain>)>>();
                // brains are paired by position, so the games must agree on each one's board;
                // the dealing stays outside the `debug_assert!`, which release builds skip
                let board = dealt[0].0;
                debug_assert!(
                    dealt.iter().all(|(plays, _)| *plays == board),
                    "the games disagree about which board an ai player takes"
                );
                (board, dealt.into_iter().map(|(_, brain)| brain).collect())
            })
            .collect()
    }
}

/// How the games are sequenced. Every player faces the same sequence, the random ones dealt
/// once per match; the races end with the playlist and the marathons cycle it forever.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Playlist {
    /// every theme of each game, the games taking turns; a race, listed as `theme sprint`
    ThemeRace,
    /// the games take turns, each carrying on through its themes; a marathon
    Interleaved,
    /// all of one game, then all of the other; a marathon
    BackToBack,
    /// the retro themes only, the games taking turns theme by theme; a marathon
    Retro,
    /// the particle themes only, the games taking turns; a marathon
    Particle,
    /// a random game and theme each stage; a race over this many stages
    RandomSprint { stages: u32 },
    /// a random game and theme each stage, forever; a marathon
    RandomMarathon,
}

impl Playlist {
    pub const ALL: [Playlist; 9] = [
        Playlist::ThemeRace,
        Playlist::Interleaved,
        Playlist::BackToBack,
        Playlist::Retro,
        Playlist::Particle,
        Playlist::RandomSprint { stages: 3 },
        Playlist::RandomSprint { stages: 5 },
        Playlist::RandomSprint { stages: 10 },
        Playlist::RandomMarathon,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Playlist::ThemeRace => "theme sprint",
            Playlist::Interleaved => "interleaved marathon",
            Playlist::BackToBack => "back to back marathon",
            Playlist::Retro => "retro marathon",
            Playlist::Particle => "particle marathon",
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

    /// which themes this playlist is over: one family of them, or every theme
    pub fn theme_family(&self) -> Option<ThemeFamily> {
        match self {
            Playlist::Retro => Some(ThemeFamily::Retro),
            Playlist::Particle => Some(ThemeFamily::Particle),
            _ => None,
        }
    }

    /// how many stages the playlist deals, or `None` for the endless random marathon
    fn stage_count(&self, themes: &PlaylistThemes) -> Option<usize> {
        match self {
            Playlist::RandomSprint { stages } => Some(*stages as usize),
            Playlist::RandomMarathon => None,
            _ => Some(themes.dealt().count() * themes.slots()),
        }
    }

    pub fn rules(&self, themes: &PlaylistThemes) -> MatchRules {
        if self.is_race() {
            MatchRules::StageSprint {
                stages: self.stage_count(themes).unwrap_or(0) as u32,
            }
        } else {
            MatchRules::Marathon
        }
    }

    /// which stage a player is on after `completed` stages; `None` once a race is run
    fn stage_index(&self, completed: usize, themes: &PlaylistThemes) -> Option<usize> {
        match self.stage_count(themes) {
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
        themes: &PlaylistThemes,
    ) -> Option<(GameKind, ThemeMode)> {
        let index = self.stage_index(completed, themes)?;
        Some(match self {
            Playlist::RandomSprint { .. } | Playlist::RandomMarathon => {
                random_stage(seed, index, themes)
            }
            _ => self.fixed_stages(themes)[index],
        })
    }

    /// which game the playlist opens with; the seed only matters to the random playlists
    pub fn first_game(&self, seed: u64, dealt: &Dealt) -> GameKind {
        match self {
            Playlist::RandomSprint { .. } | Playlist::RandomMarathon => random_game(seed, 0, dealt),
            _ => dealt.first(),
        }
    }

    /// the game and theme of each stage of a fixed playlist; [`random_stage`] deals the rest
    fn fixed_stages(&self, themes: &PlaylistThemes) -> Vec<(GameKind, ThemeMode)> {
        let order = themes.dealt();
        let slots = themes.slots();
        match self {
            // a stage names its theme rather than asking for the next, since a game returning
            // to its turn would otherwise start over on its first theme
            Playlist::ThemeRace | Playlist::Interleaved | Playlist::Retro | Playlist::Particle => {
                (0..slots)
                    .flat_map(|slot| {
                        order
                            .games()
                            .map(move |game| (game, ThemeMode::Fixed(themes.theme(game, slot))))
                            .collect::<Vec<(GameKind, ThemeMode)>>()
                    })
                    .collect()
            }
            Playlist::BackToBack => order
                .games()
                .flat_map(|game| {
                    (0..slots).map(move |slot| (game, ThemeMode::Fixed(themes.theme(game, slot))))
                })
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

/// the game dealt to a random playlist's stage; independent of the themes, so the opening
/// game is known before they are
fn random_game(seed: u64, index: usize, dealt: &Dealt) -> GameKind {
    let roll = stage_roll(seed, index, 0) % dealt.count().max(1) as u64;
    dealt.turn(roll as usize)
}

/// a random game and theme, never the exact pair of the stage before
fn random_stage(seed: u64, index: usize, themes: &PlaylistThemes) -> (GameKind, ThemeMode) {
    let slots = themes.slots();
    let dealt = themes.dealt();
    if slots == 0 {
        return (random_game(seed, index, dealt), ThemeMode::Fixed(0));
    }
    let mut previous: Option<(GameKind, usize)> = None;
    for i in 0..=index {
        let game = random_game(seed, i, dealt);
        let mut slot = if slots == 0 {
            0
        } else {
            (stage_roll(seed, i, 1) % slots as u64) as usize
        };
        if slots > 1 && previous == Some((game, slot)) {
            slot = (slot + 1) % slots;
        }
        previous = Some((game, slot));
    }
    let (game, slot) = previous.unwrap();
    (game, ThemeMode::Fixed(themes.theme(game, slot)))
}

/// Which games the vs. playlist deals, one `on`/`off` select list per game. Every row starts
/// ticked and the last ticked one cannot be turned off.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameSelection(PerGame<bool>);

impl GameSelection {
    const ON: &'static str = "on";
    const OFF: &'static str = "off";

    /// every game the playlist can deal, ticked
    fn new() -> Self {
        Self(PerGame::new(|game| {
            GameKind::PLAYLIST_ORDER.contains(&game)
        }))
    }

    fn is_on(&self, game: GameKind) -> bool {
        *self.0.get(game)
    }

    /// the games a match deals, in the playlist's own turn order
    fn dealt(&self) -> Dealt {
        Dealt::new(
            GameKind::PLAYLIST_ORDER
                .iter()
                .copied()
                .filter(|game| self.is_on(*game))
                .collect(),
        )
    }

    /// one menu row per game the playlist can deal, in the order it deals them
    fn items(&self) -> Vec<MenuItem> {
        GameKind::PLAYLIST_ORDER
            .iter()
            .map(|game| {
                MenuItem::select_list(
                    game.name(),
                    vec![Self::ON.to_string(), Self::OFF.to_string()],
                    usize::from(!self.is_on(*game)),
                )
            })
            .collect()
    }

    /// A pick on one of those rows, answering whether it was one. Unticking the last game is
    /// refused, and the row snaps back when the menu redraws from [`Self::items`].
    fn select(&mut self, name: &str, value: &str) -> bool {
        let Some(game) = GameKind::PLAYLIST_ORDER
            .iter()
            .copied()
            .find(|game| game.name() == name)
        else {
            return false;
        };
        let on = value == Self::ON;
        if on || self.dealt().count() > 1 {
            *self.0.get_mut(game) = on;
        }
        true
    }
}

/// One 0-10 starting difficulty dial shared by every game of a playlist; what it means to one
/// game is that game's arm of [`Difficulty::level`].
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

    /// The starting level the dial sets in one game, in that game's own terms.
    fn level(&self, game: GameKind) -> u32 {
        match game {
            // a virus level
            GameKind::DrRustario => self.0,
            // a starting level, inside the guideline fall curve's 14
            GameKind::Rustris => self.0,
            // a speed step only: colours and starting nuisance stay at the default, so the dial
            // never changes what is dealt. 10 is inside the twelve step fall curve
            GameKind::Puyo => self.0,
            // a speed step, on the same ladder as Puyo's
            #[cfg(feature = "rustle-fighter")]
            GameKind::RustleFighter => self.0,
        }
    }

    /// Dr. Rustario's separate fall speed; the other games' levels are their fall speed.
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
    /// which games the playlist deals
    selection: GameSelection,
    difficulty: Difficulty,
    ai: VersusAi,
    /// what the random playlists are dealt from, re-rolled as each match starts
    seed: Cell<u64>,
}

impl VersusMode {
    pub fn new() -> Self {
        Self {
            players: 1,
            playlist: Playlist::ThemeRace,
            selection: GameSelection::new(),
            difficulty: Difficulty::default(),
            ai: VersusAi::Off,
            seed: Cell::new(0),
        }
    }

    /// the title screen's players list: humans, then the ai opponents and the ai demos
    fn players_list(&self, max_players: u32) -> (Vec<String>, usize) {
        let mut players = (1..=max_players)
            .map(|i| i.to_string())
            .collect::<Vec<String>>();
        if max_players > 1 {
            players.extend(
                AiDifficulty::ALL
                    .iter()
                    .filter_map(|d| VersusAi::Opponent(*d).name()),
            );
        }
        players.push(AI_DEMO_1P.to_string());
        if max_players > 1 {
            players.push(AI_DEMO_2P.to_string());
        }
        let current = match self.ai.name() {
            None => (self.players as usize).clamp(1, max_players as usize) - 1,
            Some(name) => players.iter().position(|p| *p == name).unwrap_or(0),
        };
        (players, current)
    }

    /// a pick from [`Self::players_list`]
    fn select_players(&mut self, value: &str) {
        self.ai = VersusAi::from_name(value).unwrap_or(VersusAi::Off);
        self.players = self
            .ai
            .players()
            .unwrap_or_else(|| value.parse::<u32>().unwrap_or(1));
    }

    /// the themes the chosen playlist deals from, or every theme when a ticked game has none
    /// of the playlist's family
    fn playlist_themes(&self, themes: &Themes) -> PlaylistThemes {
        let dealt = self.selection.dealt();
        let family = themes.playlist(self.playlist.theme_family(), &dealt);
        if family.covers_every_game() {
            family
        } else {
            themes.playlist(None, &dealt)
        }
    }

    /// A mode dealing from a known seed at a known difficulty, for the headless harnesses such
    /// as `ga cross`.
    pub(crate) fn dealing(seed: u64, difficulty: Difficulty) -> Self {
        let mode = Self::new();
        mode.seed.set(seed);
        Self { difficulty, ..mode }
    }

    /// the seed every copy of `kind` in this match is dealt from, so players who reach the
    /// game at different stages are still dealt the same pieces
    fn game_seed(&self, kind: GameKind) -> engine::game::random::Seed {
        // one salt per game so the games do not share a stream
        let salt = kind.index() as u64 + 1;
        engine::game::random::Seed::from_u64(splitmix64(self.seed.get() ^ splitmix64(salt)))
    }

    /// `count` games of a kind at this difficulty, sharing a seed, for the players from
    /// `first_player` on; Puyo Rusto draws each player's board from their own sprite set.
    pub(crate) fn new_games(
        &self,
        kind: GameKind,
        count: usize,
        first_player: usize,
    ) -> Result<Vec<AnyGame>, String> {
        let seed = self.game_seed(kind);
        Ok(match kind {
            GameKind::DrRustario => {
                let mode = dr_rustario::game::random::RandomMode::Bag;
                dr_rustario::game::random::from_seed(seed, count, mode)
                    .into_iter()
                    .map(|rand| {
                        dr_rustario::game::Game::new(
                            self.difficulty.level(kind),
                            self.difficulty.dr_rustario_speed(),
                            rand,
                        )
                        .map(AnyGame::DrRustario)
                    })
                    .collect::<Result<Vec<AnyGame>, String>>()?
            }
            GameKind::Rustris => {
                let mode = rustris::game::random::RandomMode::Bag;
                rustris::game::random::from_seed(seed.into(), mode, count)
                    .into_iter()
                    .map(|rand| {
                        AnyGame::Rustris(rustris::game::Game::new(
                            self.difficulty.level(kind),
                            rand,
                        ))
                    })
                    .collect()
            }
            GameKind::Puyo => {
                let difficulty = puyo_rusto::game::rules::Difficulty::default();
                // one set of puyos per player, dealt off the match seed so a player swapping
                // back onto Puyo gets the same set
                let skins = puyo_rusto::game::cell::PuyoSkin::deal(seed, first_player + count);
                puyo_rusto::game::random::from_seed(seed, count, difficulty.colors())
                    .into_iter()
                    .zip(skins.into_iter().skip(first_player))
                    .map(|(rand, skin)| {
                        AnyGame::Puyo(puyo_rusto::game::Game::new(
                            difficulty,
                            self.difficulty.level(kind),
                            rand,
                            skin,
                        ))
                    })
                    .collect()
            }
            // every board is dealt the same sequence; the fighter only shapes the opponent's
            // garbage
            #[cfg(feature = "rustle-fighter")]
            GameKind::RustleFighter => {
                let fighter = rustle_fighter::game::counter::Fighter::default();
                let difficulty = rustle_fighter::game::rules::Difficulty::default();
                (0..count)
                    .map(|_| {
                        AnyGame::RustleFighter(rustle_fighter::game::Game::new(
                            fighter,
                            difficulty,
                            self.difficulty.level(kind),
                            rustle_fighter::game::random::GameRandom::from_seed(seed),
                        ))
                    })
                    .collect()
            }
        })
    }

    /// The playlist's next turn for this player: its game, theme, and a new board when the game
    /// has changed. [`Mode::next_stage`] without a window, so a playlist can be played headless.
    fn next_turn(
        &self,
        playlist_themes: &PlaylistThemes,
        player: u32,
        completed: u32,
    ) -> Option<(GameKind, ThemeMode, Option<AnyGame>)> {
        let seed = self.seed.get();
        let (kind, theme_mode) = self
            .playlist
            .stage(seed, completed as usize, playlist_themes)?;
        let previous = completed
            .checked_sub(1)
            .and_then(|i| self.playlist.stage(seed, i as usize, playlist_themes))
            .map(|(kind, _)| kind);
        // the same game again keeps its board and hold: only the theme changes
        let game = if previous == Some(kind) {
            None
        } else {
            Some(self.new_games(kind, 1, player as usize).ok()?.pop()?)
        };
        Some((kind, theme_mode, game))
    }
}

impl Mode for VersusMode {
    fn title(&self) -> String {
        "vs. playlist".to_string()
    }

    fn menu_sounds(&self) -> MenuSounds {
        MenuSounds::MODERN
    }

    fn race(&self, themes: &Themes) -> Vec<RaceTheme> {
        themes.race_all()
    }

    fn title_items(&self, max_players: u32) -> Vec<MenuItem> {
        let (players, current) = self.players_list(max_players);
        vec![MenuItem::select_list(PLAYERS, players, current)]
    }

    fn title_select(&mut self, name: &str, value: &str) {
        if name == PLAYERS {
            self.select_players(value);
        }
    }

    /// the playlist, then a tick per game it can deal, then the difficulty dial
    fn menu_items(&self) -> Vec<MenuItem> {
        let mut items = vec![MenuItem::select_list(
            PLAYLIST,
            Playlist::ALL.iter().map(|p| p.name().to_string()).collect(),
            Playlist::ALL
                .iter()
                .position(|p| *p == self.playlist)
                .unwrap_or(0),
        )];
        items.extend(self.selection.items());
        items.push(MenuItem::select_list(
            DIFFICULTY,
            Difficulty::names(),
            self.difficulty.0 as usize,
        ));
        items
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
            _ => {
                self.selection.select(name, value);
            }
        }
    }

    fn subtitle(&self) -> String {
        subtitle("", self.players).trim().to_string()
    }

    /// The vs. playlist does not rank, so it never offers name entry; old versus rows in
    /// `high_scores.yml` are left in place and not shown.
    fn high_score_key(&self) -> Option<HighScoreKey> {
        None
    }

    fn all_high_score_keys(&self) -> Vec<HighScoreKey> {
        vec![]
    }

    fn settings(&self, themes: &Themes) -> MatchSettings {
        let playlist_themes = self.playlist_themes(themes);
        let (first, theme_mode) = self
            .playlist
            .stage(self.seed.get(), 0, &playlist_themes)
            .expect("every playlist opens with a stage");
        MatchSettings {
            rules: self.playlist.rules(&playlist_themes),
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
            self.playlist
                .first_game(self.seed.get(), &self.selection.dealt()),
            self.players as usize,
            0,
        )
    }

    fn next_stage(
        &self,
        themes: &Themes,
        player: u32,
        completed: u32,
    ) -> Option<StageChange<AnyGame>> {
        let (kind, theme_mode, game) =
            self.next_turn(&self.playlist_themes(themes), player, completed)?;
        Some(StageChange {
            game,
            settings: PlayerSettings {
                themes: themes.range(kind),
                theme_mode,
            },
        })
    }

    fn controllers(&self) -> Vec<Controller> {
        self.ai
            .brains()
            .into_iter()
            .map(|(player, mut brains)| {
                // every brain forgets its queue when the game changes; only the one for the
                // dealt game then acts
                let mut playing: Option<GameKind> = None;
                let controller = move |game: &mut AnyGame, delta: Duration| {
                    if playing != Some(game.kind()) {
                        for brain in brains.iter_mut() {
                            brain.reset();
                        }
                        playing = Some(game.kind());
                    }
                    for brain in brains.iter_mut() {
                        brain.act(game, delta);
                    }
                };
                (player, Box::new(controller) as Box<_>)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::game::{Game, GameEvent, StageState};
    use engine::session::Player;
    use std::collections::HashSet;

    /// the same list of themes for every game, for every game the playlist deals
    fn same_themes(themes: Vec<usize>) -> PlaylistThemes {
        PlaylistThemes::new(PerGame::new(|_| themes.clone()), Dealt::default())
    }

    /// four themes each, the last of them the particle theme, as every game is built
    fn all_themes() -> PlaylistThemes {
        same_themes(vec![0, 1, 2, 3])
    }

    /// how many games a playlist deals with every row of its menu ticked
    const DEALT: usize = GameKind::PLAYLIST_ORDER.len();

    /// how many stages a fixed playlist over `all_themes` deals before it repeats: one turn
    /// per game the playlist deals, per theme slot
    const THEME_SLOTS: usize = 4;
    const FIXED_STAGES: usize = DEALT * THEME_SLOTS;

    fn stages(playlist: Playlist, seed: u64, count: usize) -> Vec<(GameKind, ThemeMode)> {
        let themes = all_themes();
        (0..count)
            .map(|i| playlist.stage(seed, i, &themes).unwrap())
            .collect()
    }

    /// one turn each, in the order the playlist deals them, all on the same theme slot
    fn turns(slot: usize) -> Vec<(GameKind, ThemeMode)> {
        Dealt::default()
            .games()
            .map(|game| (game, ThemeMode::Fixed(slot)))
            .collect()
    }

    /// what one game calls its ai difficulties, in its own order
    fn ai_difficulty_names(game: GameKind) -> Vec<&'static str> {
        match game {
            GameKind::DrRustario => dr_rustario::game::rules::AiDifficulty::ALL
                .iter()
                .map(|d| d.name())
                .collect(),
            GameKind::Rustris => rustris::game::rules::AiDifficulty::ALL
                .iter()
                .map(|d| d.name())
                .collect(),
            GameKind::Puyo => puyo_rusto::game::rules::AiDifficulty::ALL
                .iter()
                .map(|d| d.name())
                .collect(),
            // no ai, so no difficulties; `ai_difficulties_agree` skips it
            #[cfg(feature = "rustle-fighter")]
            GameKind::RustleFighter => vec![],
        }
    }

    #[test]
    fn theme_race_alternates_games_through_every_theme() {
        let stages = stages(Playlist::ThemeRace, 0, FIXED_STAGES);
        // every game takes a turn on a slot before the next slot
        for (slot, turn) in stages.chunks(DEALT).enumerate() {
            assert_eq!(turn, turns(slot), "slot {slot}");
        }
        // the race ends with the playlist rather than cycling it
        assert_eq!(
            Playlist::ThemeRace.stage(0, FIXED_STAGES, &all_themes()),
            None
        );
    }

    /// the players list every mode offers: humans, the four ai opponents, then the two demos
    fn ai_players_list() -> Vec<String> {
        [
            "1",
            "2",
            "vs easy ai",
            "vs normal ai",
            "vs hard ai",
            "vs impossible ai",
            "1-player ai demo",
            "2-player ai demo",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    /// every mode that has an ai offers the same opponents and demos under the same names
    #[test]
    fn every_mode_offers_the_same_ai_opponents_and_demos() {
        let modes = GameKind::ALL
            .into_iter()
            .filter(|game| game.fields_an_ai())
            .map(game_mode)
            .chain([Box::new(VersusMode::new()) as Box<dyn Mode>]);
        for mode in modes {
            assert_eq!(
                mode.title_items(2),
                vec![MenuItem::select_list(PLAYERS, ai_players_list(), 0)],
                "{}",
                mode.title()
            );
        }
    }

    /// every game with an ai names its difficulties as the versus mode does
    #[test]
    fn ai_difficulties_agree() {
        let versus: Vec<&str> = AiDifficulty::ALL.iter().map(|d| d.name()).collect();
        assert_eq!(versus, vec!["easy", "normal", "hard", "impossible"]);
        for game in GameKind::ALL.into_iter().filter(|g| g.fields_an_ai()) {
            assert_eq!(ai_difficulty_names(game), versus, "{game:?}");
        }
    }

    #[test]
    fn a_versus_ai_is_each_games_own_ai() {
        let mut mode = VersusMode::new();
        assert!(mode.controllers().is_empty());

        mode.title_select(PLAYERS, "vs hard ai");
        let controllers = mode.controllers();
        assert_eq!(controllers.len(), 1);
        assert_eq!(controllers[0].0, 1, "the ai should play as player 2");
        assert_eq!(mode.players, 2);

        // one board in the single player demo, two in the vs. demo
        mode.title_select(PLAYERS, AI_DEMO_1P);
        let controllers = mode.controllers();
        assert_eq!(
            controllers.iter().map(|(p, _)| *p).collect::<Vec<u32>>(),
            vec![0]
        );
        assert_eq!(mode.players, 1);

        mode.title_select(PLAYERS, AI_DEMO_2P);
        let controllers = mode.controllers();
        assert_eq!(
            controllers.iter().map(|(p, _)| *p).collect::<Vec<u32>>(),
            vec![0, 1]
        );
        assert_eq!(mode.players, 2);

        // back to humans
        mode.title_select(PLAYERS, "2");
        assert!(mode.controllers().is_empty());
        assert_eq!(mode.players, 2);
    }

    /// a versus ai player carries one brain per game with an ai
    #[test]
    fn a_versus_ai_player_carries_a_brain_for_every_game() {
        let mut mode = VersusMode::new();
        mode.title_select(PLAYERS, AI_DEMO_2P);
        let brains = mode.ai.brains();
        assert_eq!(
            brains
                .iter()
                .map(|(player, _)| *player)
                .collect::<Vec<u32>>(),
            vec![0, 1]
        );
        let with_ai = GameKind::ALL.iter().filter(|g| g.fields_an_ai()).count();
        for (player, brains) in brains.iter() {
            assert_eq!(brains.len(), with_ai, "player {player}");
        }
    }

    const STEP: Duration = Duration::from_millis(16);

    /// run `game` for `frames`, returning how many pieces it locked
    fn run(
        game: &mut AnyGame,
        frames: usize,
        mut controller: impl FnMut(&mut AnyGame, Duration),
    ) -> usize {
        use engine::game::{Game, GameEvent};
        let mut locked = 0;
        for _ in 0..frames {
            controller(game, STEP);
            Game::update(game, STEP);
            locked += Game::drain_events(game)
                .iter()
                .filter(|event| matches!(event, GameEvent::Lock { .. }))
                .count();
        }
        locked
    }

    /// a versus ai plays each game it is dealt, and again after its board is swapped back
    #[test]
    fn a_versus_ai_plays_every_game() {
        let mut mode = VersusMode::new();
        mode.title_select(PLAYERS, AI_DEMO_1P);
        let mut controllers = mode.controllers();
        let (_, controller) = &mut controllers[0];

        // every game in turn, then back to the first
        let with_ai: Vec<GameKind> = GameKind::ALL
            .into_iter()
            .filter(|game| game.fields_an_ai())
            .collect();
        let dealt = with_ai.clone().into_iter().chain([with_ai[0]]);
        for kind in dealt {
            let mut played = mode.new_games(kind, 1, 0).unwrap().pop().unwrap();
            let mut alone = mode.new_games(kind, 1, 0).unwrap().pop().unwrap();
            // gravity alone at difficulty 0 locks at most one piece in this time
            let with_ai = run(&mut played, 240, |game, delta| controller(game, delta));
            let without = run(&mut alone, 240, |_, _| {});
            assert!(
                with_ai > without && with_ai > 1,
                "the ai did not play {:?}: {} pieces locked against {} left to fall on their own",
                kind,
                with_ai,
                without
            );
        }
    }

    /// A whole match played headless, touching the games in the order the match screen does:
    /// controllers, garbage crossing, and a playlist swapping boards between stages.
    struct Session<'a> {
        players: Vec<Player<AnyGame>>,
        controllers: Vec<Controller>,
        /// the boards a player has swapped out, resumed when their game's turn comes round
        parked: Vec<Vec<AnyGame>>,
        completed: Vec<u32>,
        /// a player who has been buried plays no further part
        out: Vec<bool>,
        /// the board for a player's next stage, or `None` when the game does not change
        next_turn: Box<dyn FnMut(u32, u32) -> Option<AnyGame> + 'a>,
        locked: usize,
        stages: usize,
        swaps: usize,
        attacks: usize,
    }

    impl<'a> Session<'a> {
        fn new(
            games: Vec<AnyGame>,
            controllers: Vec<Controller>,
            next_turn: impl FnMut(u32, u32) -> Option<AnyGame> + 'a,
        ) -> Self {
            let count = games.len();
            Self {
                players: games
                    .into_iter()
                    .enumerate()
                    .map(|(pid, game)| Player::new(pid as u32, game))
                    .collect(),
                controllers,
                parked: (0..count).map(|_| vec![]).collect(),
                completed: vec![0; count],
                out: vec![false; count],
                next_turn: Box::new(next_turn),
                locked: 0,
                stages: 0,
                swaps: 0,
                attacks: 0,
            }
        }

        /// the board an attack lands on: the next player still playing
        fn victim(&self, from: u32) -> Option<u32> {
            (1..self.players.len() as u32)
                .map(|step| (from + step) % self.players.len() as u32)
                .find(|player| !self.out[*player as usize])
        }

        fn play(&mut self, frames: usize) -> &mut Self {
            for _ in 0..frames {
                for player in 0..self.players.len() as u32 {
                    if self.out[player as usize] {
                        continue;
                    }
                    self.frame(player);
                }
            }
            self
        }

        fn frame(&mut self, player: u32) {
            let index = player as usize;
            let game = self.players[index].game_mut();
            for (controlled, controller) in self.controllers.iter_mut() {
                if *controlled == player {
                    controller(game, STEP);
                }
            }
            let mut events = Game::drain_events(game);
            Game::update(game, STEP);
            events.extend(Game::drain_events(game));

            let mut stage_complete = false;
            for event in events {
                match event {
                    GameEvent::Lock { .. } => self.locked += 1,
                    GameEvent::AttackSent(attack) => {
                        if let Some(victim) = self.victim(player) {
                            let board = self.players[victim as usize].game_mut();
                            // an unpriced attack is worth nothing and drops
                            if attack.strength_for(Game::game_id(board)) > 0 {
                                board.receive_attack(attack);
                                self.attacks += 1;
                            }
                        }
                    }
                    GameEvent::StageComplete => stage_complete = true,
                    GameEvent::GameOver => self.out[index] = true,
                    _ => {}
                }
            }

            if stage_complete && !self.out[index] {
                self.stages += 1;
                self.completed[index] += 1;
                let completed = self.completed[index];
                if let Some(next) = (self.next_turn)(player, completed) {
                    // resume a parked board of this game, else start the new one
                    let wanted = Game::game_id(&next);
                    let resumed = match self.parked[index]
                        .iter()
                        .position(|g| Game::game_id(g) == wanted)
                    {
                        Some(i) => self.parked[index].swap_remove(i),
                        None => next,
                    };
                    let outgoing = self.players[index].replace_game(resumed);
                    self.parked[index].push(outgoing);
                    self.swaps += 1;
                }
                let game = self.players[index].game_mut();
                if Game::stage_state(game) == StageState::StageComplete {
                    Game::next_stage(game).expect("the next stage");
                }
                Game::set_completed_stages(game, completed);
            }
        }
    }

    /// every ai mode the versus playlist offers, by the name it goes by on the title screen
    fn versus_ai_modes() -> Vec<String> {
        AiDifficulty::ALL
            .iter()
            .filter_map(|d| VersusAi::Opponent(*d).name())
            .chain([AI_DEMO_1P.to_string(), AI_DEMO_2P.to_string()])
            .collect()
    }

    /// a versus match under `mode`'s current options, dealt from [`ticked_themes`]
    fn versus_session(mode: &VersusMode) -> Session<'_> {
        let games = mode.games().unwrap();
        let controllers = mode.controllers();
        let themes = ticked_themes(mode);
        Session::new(games, controllers, move |player, completed| {
            mode.next_turn(&themes, player, completed)
                .and_then(|(_, _, game)| game)
        })
    }

    /// Frames per ai mode, scaled per dealt game so a demo is dealt every game more than once.
    /// `Mode::games` re-rolls the seed, so an unlucky draw can fail the swap count without a
    /// broken ai.
    const MATCH_FRAMES: usize = 3_000 * DEALT / 2;

    /// Every ai mode of the versus playlist plays a match out; `cargo test --release` also
    /// catches work hidden inside a `debug_assert!`.
    #[test]
    fn every_versus_ai_mode_plays_a_match_out() {
        for players in versus_ai_modes() {
            let mut mode = VersusMode::new();
            mode.title_select(PLAYERS, &players);
            // so boards are swapped out and later resumed
            mode.menu_select(PLAYLIST, Playlist::Interleaved.name());
            let mut session = versus_session(&mode);
            session.play(MATCH_FRAMES);
            assert!(session.locked > 0, "{players} played nothing");
            // a demo plays at full speed, so it reaches the playlist's swaps
            if matches!(
                VersusAi::from_name(&players),
                Some(VersusAi::Demo) | Some(VersusAi::VsDemo)
            ) {
                assert!(
                    session.swaps >= DEALT,
                    "{players} was never dealt the whole playlist: {} stages, {} boards \
                     dealt, {} attacks crossed",
                    session.stages,
                    session.swaps,
                    session.attacks
                );
            }
        }
    }

    /// frames per single game ai mode: enough for the slowest difficulty to place several pieces
    const GAME_FRAMES: usize = 1_200;

    /// every ai mode of every game played on its own plays a match out
    #[test]
    fn every_ai_mode_of_every_game_plays_a_match_out() {
        for game in GameKind::ALL {
            for players in versus_ai_modes() {
                let mut mode = game_mode(game);
                mode.title_select(PLAYERS, &players);
                let mut session =
                    Session::new(mode.games().unwrap(), mode.controllers(), |_, _| None);
                session.play(GAME_FRAMES);
                assert!(session.locked > 0, "{game:?}: {players} played nothing");
            }
        }
    }

    /// a game played on its own deals one board of its own game per player
    #[test]
    fn a_game_played_on_its_own_deals_its_own_boards() {
        for game in GameKind::ALL {
            let mut mode = game_mode(game);
            mode.title_select(PLAYERS, "2");
            let games = mode.games().unwrap();
            assert_eq!(games.len(), 2, "{game:?}");
            assert!(games.iter().all(|g| g.kind() == game), "{game:?}");
        }
    }

    #[test]
    fn picking_an_ai_opponent_puts_the_agent_on_player_two() {
        for game in GameKind::ALL.into_iter().filter(|g| g.fields_an_ai()) {
            let mut mode = game_mode(game);
            assert!(mode.controllers().is_empty(), "{game:?}");

            mode.title_select(PLAYERS, "vs normal ai");
            let controllers = mode.controllers();
            assert_eq!(controllers.len(), 1, "{game:?}");
            assert_eq!(controllers[0].0, 1, "the ai should play as player 2");

            // and the demo plays the first board instead
            mode.title_select(PLAYERS, AI_DEMO_1P);
            let controllers = mode.controllers();
            assert_eq!(controllers.len(), 1, "{game:?}");
            assert_eq!(controllers[0].0, 0, "{game:?}");

            // the 2-player demo plays both, one of the game's models against another
            mode.title_select(PLAYERS, AI_DEMO_2P);
            let controllers = mode.controllers();
            assert_eq!(
                controllers.iter().map(|(p, _)| *p).collect::<Vec<u32>>(),
                vec![0, 1],
                "{game:?}"
            );

            // back to humans
            mode.title_select(PLAYERS, "2");
            assert!(mode.controllers().is_empty(), "{game:?}");
        }
    }

    #[test]
    fn every_mode_has_one_table_per_rules_variant() {
        for game in GameKind::ALL {
            let mode = game_mode(game);
            let keys = mode.all_high_score_keys();
            assert_eq!(keys.len(), 4, "{game:?}");
            assert!(
                keys.contains(&mode.high_score_key().expect("a game ranks")),
                "{game:?}"
            );
        }

        let rustris = RustrisMode::new();
        assert_eq!(
            rustris
                .all_high_score_keys()
                .iter()
                .map(|k| k.mode.as_str())
                .collect::<Vec<_>>(),
            vec![
                "marathon",
                "theme sprint",
                "1 level sprint",
                "10,000 point sprint"
            ]
        );
    }

    /// four themes each, dealt to the games this mode has ticked
    fn ticked_themes(mode: &VersusMode) -> PlaylistThemes {
        PlaylistThemes::new(PerGame::new(|_| vec![0, 1, 2, 3]), mode.selection.dealt())
    }

    /// the games a playlist deals over its first `count` stages, at this mode's ticks
    fn dealt_games(mode: &VersusMode, count: usize) -> Vec<GameKind> {
        let themes = ticked_themes(mode);
        (0..count)
            .filter_map(|i| mode.playlist.stage(0, i, &themes))
            .map(|(game, _)| game)
            .collect()
    }

    /// the on/off row a game is drawn as
    fn tick_row(game: GameKind, on: bool) -> MenuItem {
        MenuItem::select_list(
            game.name(),
            vec![
                GameSelection::ON.to_string(),
                GameSelection::OFF.to_string(),
            ],
            usize::from(!on),
        )
    }

    /// the versus menu is the playlist, a ticked row per dealable game, then the difficulty
    #[test]
    fn the_versus_menu_ticks_every_game_it_can_deal() {
        let items = VersusMode::new().menu_items();
        assert_eq!(items.len(), DEALT + 2);
        assert_eq!(items[0].name(), PLAYLIST);
        assert_eq!(items[items.len() - 1].name(), DIFFICULTY);
        assert_eq!(
            items[1..=DEALT],
            GameKind::PLAYLIST_ORDER
                .iter()
                .map(|game| tick_row(*game, true))
                .collect::<Vec<MenuItem>>()[..]
        );
    }

    /// every playlist, and the opening game, skips a game unticked on the menu
    #[test]
    fn a_playlist_deals_only_the_games_its_menu_ticks() {
        for off in GameKind::PLAYLIST_ORDER.iter().copied() {
            let mut mode = VersusMode::new();
            mode.menu_select(off.name(), GameSelection::OFF);
            assert!(
                mode.menu_items().contains(&tick_row(off, false)),
                "{off:?} was not unticked"
            );

            let dealt = mode.selection.dealt();
            assert_eq!(dealt.count(), DEALT - 1, "{off:?}");
            assert!(!dealt.games().any(|game| game == off), "{off:?}");

            // every playlist, fixed and random alike, and the game a match opens on
            for playlist in Playlist::ALL {
                mode.playlist = playlist;
                let played = dealt_games(&mode, 64);
                assert!(!played.is_empty(), "{playlist:?} dealt nothing");
                assert!(
                    played.iter().all(|game| *game != off),
                    "{playlist:?} dealt {off:?}"
                );
                let opening = mode.games().unwrap();
                assert_ne!(opening[0].kind(), off, "{playlist:?} opened on {off:?}");
            }
        }
    }

    /// unticking a game shortens a fixed playlist by its turns rather than leaving gaps
    #[test]
    fn unticking_a_game_shortens_the_playlist() {
        let mut mode = VersusMode::new();
        assert_eq!(
            Playlist::ThemeRace.stage_count(&ticked_themes(&mode)),
            Some(DEALT * THEME_SLOTS)
        );
        mode.menu_select(GameKind::Puyo.name(), GameSelection::OFF);
        assert_eq!(
            Playlist::ThemeRace.stage_count(&ticked_themes(&mode)),
            Some((DEALT - 1) * THEME_SLOTS)
        );
    }

    /// unticking every game leaves the last one on
    #[test]
    fn the_last_game_cannot_be_unticked() {
        let mut mode = VersusMode::new();
        for game in GameKind::PLAYLIST_ORDER {
            mode.menu_select(game.name(), GameSelection::OFF);
        }
        let dealt = mode.selection.dealt();
        assert_eq!(dealt.count(), 1);
        let last = dealt.games().next().unwrap();
        assert!(mode.menu_items().contains(&tick_row(last, true)));
        assert_eq!(last, GameKind::PLAYLIST_ORDER[DEALT - 1]);

        // a refused row is not stuck off
        mode.menu_select(GameKind::PLAYLIST_ORDER[0].name(), GameSelection::ON);
        assert_eq!(mode.selection.dealt().count(), 2);
    }

    /// the vs. playlist has no high score table, while every single game mode has one
    #[test]
    fn the_versus_playlist_has_no_high_score_table() {
        let versus = VersusMode::new();
        assert_eq!(versus.high_score_key(), None);
        assert!(versus.all_high_score_keys().is_empty());
        for game in GameKind::ALL {
            assert!(game_mode(game).high_score_key().is_some(), "{game:?}");
        }
    }

    #[test]
    fn only_the_races_are_sprints() {
        assert_eq!(
            Playlist::ThemeRace.rules(&all_themes()),
            MatchRules::StageSprint {
                stages: FIXED_STAGES as u32
            }
        );
        assert_eq!(
            Playlist::RandomSprint { stages: 5 }.rules(&all_themes()),
            MatchRules::StageSprint { stages: 5 }
        );
        for playlist in [
            Playlist::Interleaved,
            Playlist::BackToBack,
            Playlist::RandomMarathon,
        ] {
            assert_eq!(playlist.rules(&all_themes()), MatchRules::Marathon);
        }
    }

    #[test]
    fn the_races_end_with_the_playlist_and_marathons_cycle_it() {
        let themes = all_themes();
        assert!(Playlist::ThemeRace
            .stage(0, FIXED_STAGES - 1, &themes)
            .is_some());
        assert_eq!(Playlist::ThemeRace.stage(0, FIXED_STAGES, &themes), None);
        assert_eq!(
            Playlist::RandomSprint { stages: 3 }.stage(0, 3, &themes),
            None
        );
        for playlist in [Playlist::BackToBack, Playlist::Interleaved] {
            assert_eq!(
                playlist.stage(0, FIXED_STAGES, &themes),
                playlist.stage(0, 0, &themes),
                "{playlist:?}"
            );
            assert_eq!(
                playlist.stage(0, FIXED_STAGES + 5, &themes),
                playlist.stage(0, 5, &themes),
                "{playlist:?}"
            );
        }
        assert!(Playlist::RandomMarathon.stage(0, 10_000, &themes).is_some());
        assert_eq!(
            Playlist::ThemeRace.stage(0, 0, &PlaylistThemes::default()),
            None
        );
    }

    #[test]
    fn the_interleaved_marathon_advances_each_games_themes() {
        let stages = stages(Playlist::Interleaved, 0, FIXED_STAGES + DEALT);
        // each game carries on through its themes rather than restarting on its turn
        for slot in 0..THEME_SLOTS {
            let turn = slot * DEALT;
            assert_eq!(stages[turn..turn + DEALT], turns(slot)[..]);
        }
        assert_eq!(stages[FIXED_STAGES..], stages[..DEALT]);
    }

    #[test]
    fn the_retro_marathon_cycles_the_retro_themes_of_every_game() {
        let retro = same_themes(vec![0, 1, 2]);
        let slots = 3;
        let cycle = DEALT * slots;
        assert_eq!(Playlist::Retro.theme_family(), Some(ThemeFamily::Retro));
        assert_eq!(Playlist::Retro.rules(&retro), MatchRules::Marathon);

        let stages: Vec<(GameKind, ThemeMode)> = (0..cycle + DEALT)
            .map(|i| Playlist::Retro.stage(0, i, &retro).unwrap())
            .collect();
        for slot in 0..slots {
            let turn = slot * DEALT;
            assert_eq!(stages[turn..turn + DEALT], turns(slot)[..]);
        }
        assert!(stages
            .iter()
            .all(|(_, theme)| *theme != ThemeMode::Fixed(3)));
        assert_eq!(stages[cycle..], stages[..DEALT]);
    }

    #[test]
    fn the_particle_marathon_plays_every_games_particle_theme() {
        let particle = same_themes(vec![3]);
        assert_eq!(
            Playlist::Particle.theme_family(),
            Some(ThemeFamily::Particle)
        );
        assert_eq!(Playlist::Particle.rules(&particle), MatchRules::Marathon);

        let stages: Vec<(GameKind, ThemeMode)> = (0..2 * DEALT)
            .map(|i| Playlist::Particle.stage(0, i, &particle).unwrap())
            .collect();
        assert_eq!(stages, [turns(3).as_slice(), turns(3).as_slice()].concat());
    }

    #[test]
    fn a_playlist_over_a_family_deals_that_familys_theme_indices() {
        let mixed = PlaylistThemes::new(
            PerGame::new(|game| match game {
                GameKind::DrRustario => vec![2, 5],
                GameKind::Rustris => vec![0, 1],
                GameKind::Puyo => vec![4, 6],
                #[cfg(feature = "rustle-fighter")]
                GameKind::RustleFighter => vec![0],
            }),
            Dealt::default(),
        );
        assert_eq!(mixed.slots(), 2);
        for (turn, game) in Dealt::default().games().enumerate() {
            let themes = mixed.themes.get(game).clone();
            assert_eq!(
                Playlist::Retro.stage(0, DEALT + turn, &mixed),
                Some((game, ThemeMode::Fixed(themes[1])))
            );
        }
    }

    /// a playlist is as long as the longest theme list, and a shorter one replays from its start
    #[test]
    fn a_game_with_fewer_themes_wraps_rather_than_shortening_the_playlist() {
        let short = PlaylistThemes::new(
            PerGame::new(|game| match game {
                GameKind::DrRustario => vec![0, 1, 2, 3],
                GameKind::Rustris => vec![0, 1, 2, 3],
                GameKind::Puyo => vec![0, 1, 2],
                #[cfg(feature = "rustle-fighter")]
                GameKind::RustleFighter => vec![0],
            }),
            Dealt::default(),
        );
        assert_eq!(short.slots(), 4);
        for game in [GameKind::DrRustario, GameKind::Rustris] {
            assert_eq!(
                (0..4)
                    .map(|slot| short.theme(game, slot))
                    .collect::<Vec<_>>(),
                vec![0, 1, 2, 3],
                "{game:?}"
            );
        }
        assert_eq!(
            (0..4)
                .map(|slot| short.theme(GameKind::Puyo, slot))
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 0]
        );
    }

    /// a playlist covers its dealt games only when each has a theme, ignoring undealt ones
    #[test]
    fn a_playlist_covers_every_game_it_deals() {
        assert!(all_themes().covers_every_game());
        let missing = PlaylistThemes::new(
            PerGame::new(|game| match game {
                GameKind::Puyo => vec![],
                _ => vec![0, 1],
            }),
            Dealt::default(),
        );
        assert!(!missing.covers_every_game());
        let without_puyo = PlaylistThemes::new(
            PerGame::new(|game| match game {
                GameKind::Puyo => vec![],
                _ => vec![0, 1],
            }),
            Dealt::new(vec![GameKind::Rustris, GameKind::DrRustario]),
        );
        assert!(without_puyo.covers_every_game());
        assert_eq!(without_puyo.slots(), 2);
    }

    #[test]
    fn back_to_back_plays_one_game_then_the_other() {
        let stages = stages(Playlist::BackToBack, 0, FIXED_STAGES);
        for (turn, game) in GameKind::PLAYLIST_ORDER.iter().copied().enumerate() {
            let run = &stages[turn * THEME_SLOTS..(turn + 1) * THEME_SLOTS];
            assert!(run.iter().all(|(g, _)| *g == game), "{game:?}");
            assert_eq!(
                run.iter().map(|(_, theme)| *theme).collect::<Vec<_>>(),
                (0..THEME_SLOTS).map(ThemeMode::Fixed).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn random_playlists_deal_the_same_stages_for_one_seed() {
        let first = stages(Playlist::RandomMarathon, 42, 32);
        assert_eq!(first, stages(Playlist::RandomMarathon, 42, 32));
        assert_ne!(first, stages(Playlist::RandomMarathon, 43, 32));
        // a sprint over the same seed deals the same opening stages
        assert_eq!(
            first[..3],
            stages(Playlist::RandomSprint { stages: 3 }, 42, 3)
        );
    }

    #[test]
    fn random_playlists_open_with_the_game_they_deal_first() {
        for seed in 0..32 {
            let (game, _) = Playlist::RandomMarathon
                .stage(seed, 0, &all_themes())
                .unwrap();
            let dealt = Dealt::default();
            assert_eq!(Playlist::RandomMarathon.first_game(seed, &dealt), game);
            assert_eq!(
                Playlist::RandomSprint { stages: 3 }.first_game(seed, &dealt),
                game
            );
        }
    }

    #[test]
    fn random_playlists_pick_every_game_and_every_theme() {
        let stages = stages(Playlist::RandomMarathon, 7, 256);
        for game in GameKind::PLAYLIST_ORDER.iter().copied() {
            for theme in 0..THEME_SLOTS {
                assert!(
                    stages.contains(&(game, ThemeMode::Fixed(theme))),
                    "{game:?} never played theme {theme}"
                );
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
    fn difficulty_ramps_every_game_together() {
        let easiest = Difficulty::new(0);
        let hardest = Difficulty::new(10);
        for game in GameKind::ALL {
            assert_eq!(easiest.level(game), 0, "{game:?}");
            assert_eq!(hardest.level(game), Difficulty::MAX, "{game:?}");
        }
        assert_eq!(easiest, Difficulty::default());

        assert_eq!(
            easiest.dr_rustario_speed(),
            dr_rustario::game::GameSpeed::Low
        );
        assert_eq!(
            Difficulty::new(5).dr_rustario_speed(),
            dr_rustario::game::GameSpeed::Medium
        );
        assert_eq!(
            hardest.dr_rustario_speed(),
            dr_rustario::game::GameSpeed::High
        );

        // the dial stops at 10
        assert_eq!(Difficulty::new(99), hardest);
        assert_eq!(Difficulty::from_name("11"), None);
        assert_eq!(Difficulty::from_name("7"), Some(Difficulty::new(7)));
        assert_eq!(Difficulty::names().len(), 11);
    }

    /// A game's board and queue for comparing players' copies, with Puyo ids mapped onto the
    /// first player's sprite set since each player is dealt their own.
    fn board_of(game: &AnyGame) -> (Vec<engine::game::Cell>, Vec<engine::game::PieceId>) {
        use engine::game::geometry::Point;
        use engine::game::{Cell, CellId, Game, PieceId};
        use puyo_rusto::game::cell::{PuyoCell, PuyoPiece, PuyoSkin};

        let puyo = game.kind() == GameKind::Puyo;
        let cell_id = |id: CellId| match puyo {
            true => PuyoCell::from(id).id(PuyoSkin::FIRST),
            false => id,
        };
        let piece_id = |id: PieceId| match puyo {
            true => PuyoPiece::from(id).id(PuyoSkin::FIRST),
            false => id,
        };
        let cells = (0..game.board_height())
            .flat_map(|y| (0..game.board_width()).map(move |x| Point::new(x as i32, y as i32)))
            .map(|p| match game.cell(p) {
                Cell::Empty => Cell::Empty,
                Cell::Active(id) => Cell::Active(cell_id(id)),
                Cell::Ghost(id) => Cell::Ghost(cell_id(id)),
                Cell::Stack(id) => Cell::Stack(cell_id(id)),
                Cell::Garbage(id) => Cell::Garbage(cell_id(id)),
            })
            .collect();
        (cells, game.queue().into_iter().map(piece_id).collect())
    }

    fn versus_at(seed: u64, difficulty: u32) -> VersusMode {
        VersusMode::dealing(seed, Difficulty::new(difficulty))
    }

    /// two players are dealt the same board and pieces whether dealt together or apart
    #[test]
    fn every_player_is_dealt_the_same_game_whenever_the_playlist_reaches_them() {
        for kind in GameKind::ALL {
            let mode = versus_at(12345, 10);
            // both at once, as a match starts
            let together = mode.new_games(kind, 2, 0).unwrap();
            assert_eq!(board_of(&together[0]), board_of(&together[1]), "{:?}", kind);
            // and one at a time, as a playlist swaps a board over
            let apart = mode.new_games(kind, 1, 0).unwrap();
            assert_eq!(board_of(&together[0]), board_of(&apart[0]), "{:?}", kind);
        }
    }

    /// every skin a Puyo game reports, off its board and out of its queue
    fn skins_of(game: &AnyGame) -> HashSet<puyo_rusto::game::cell::PuyoSkin> {
        use engine::game::Game;
        use puyo_rusto::game::cell::PuyoSkin;
        let mut skins: HashSet<PuyoSkin> = game.queue().into_iter().map(PuyoSkin::from).collect();
        skins.extend(
            (0..game.board_height())
                .flat_map(|y| (0..game.board_width()).map(move |x| (x as i32, y as i32)))
                .filter_map(|(x, y)| game.cell(engine::game::geometry::Point::new(x, y)).id())
                .map(PuyoSkin::from),
        );
        skins
    }

    /// each Puyo player gets their own set of puyos, including a board dealt alone mid-playlist
    #[test]
    fn every_player_is_dealt_their_own_puyos() {
        let mode = versus_at(12345, 10);
        let together = mode.new_games(GameKind::Puyo, 2, 0).unwrap();
        let first = skins_of(&together[0]);
        let second = skins_of(&together[1]);
        assert_eq!(first.len(), 1, "a board is drawn from one set");
        assert_eq!(second.len(), 1, "a board is drawn from one set");
        assert_ne!(first, second, "both players were dealt the same puyos");

        for (player, expected) in [(0, &first), (1, &second)] {
            let alone = mode.new_games(GameKind::Puyo, 1, player).unwrap();
            assert_eq!(&skins_of(&alone[0]), expected, "player {player} alone");
        }
    }

    /// different seeds deal different pairs of puyo sets
    #[test]
    fn another_match_deals_another_pair_of_sets() {
        let deals: HashSet<Vec<_>> = (0..40u64)
            .map(|seed| {
                versus_at(seed, 10)
                    .new_games(GameKind::Puyo, 2, 0)
                    .unwrap()
                    .iter()
                    .map(|game| skins_of(game).into_iter().next().unwrap())
                    .collect()
            })
            .collect();
        assert!(deals.len() > 20, "{} distinct deals in 40", deals.len());
    }

    /// different match seeds deal different games
    #[test]
    fn another_match_is_dealt_another_game() {
        for kind in GameKind::ALL {
            let one = versus_at(1, 10).new_games(kind, 1, 0).unwrap();
            let two = versus_at(2, 10).new_games(kind, 1, 0).unwrap();
            assert_ne!(board_of(&one[0]), board_of(&two[0]), "{:?}", kind);
        }
    }

    /// the games are dealt from the same match seed but must not shadow each other
    #[test]
    fn no_two_games_are_dealt_from_the_same_seed() {
        let mode = versus_at(7, 10);
        let seeds: Vec<engine::game::random::Seed> = GameKind::ALL
            .into_iter()
            .map(|g| mode.game_seed(g))
            .collect();
        for (i, seed) in seeds.iter().enumerate() {
            assert!(
                !seeds[..i].contains(seed),
                "{:?} shares a seed",
                GameKind::ALL[i]
            );
        }
    }
}
