//! The three things the launcher can run: each game on its own, exactly as it was standalone,
//! and a versus mode where every player plays the same playlist over all of them.

use crate::games::{AiBrain, AnyGame, GameKind, PerGame};
use engine::app::{MatchSettings, PlayerSettings, StageChange, ThemeMode};
use engine::high_score::table::Ranking;
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
        // each game numbers the race themes within its own set; the race is over the whole
        // list, so every game's are shifted along by where its slice starts
        let mut race = match game {
            GameKind::DrRustario => dr_rustario::theme::race_themes(themes),
            GameKind::Rustris => rustris::theme::race_themes(themes),
            GameKind::Puyo => puyo_rusto::theme::race_themes(themes),
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

    /// what a playlist over this family of themes has to deal
    pub fn playlist(&self, family: Option<ThemeFamily>) -> PlaylistThemes {
        PlaylistThemes::new(PerGame::new(|game| self.family(game, family)))
    }

    pub fn race_all(&self) -> Vec<RaceTheme> {
        GameKind::ALL
            .into_iter()
            .flat_map(|game| self.race(game))
            .collect()
    }
}

/// The themes a playlist deals, as indices within each game's own set. A playlist deals
/// *slots*: at slot `n` every game plays its `n`th theme, so a playlist is only as long as
/// the shortest of the lists.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlaylistThemes(PerGame<Vec<usize>>);

impl PlaylistThemes {
    pub fn new(themes: PerGame<Vec<usize>>) -> Self {
        Self(themes)
    }

    /// how many slots the playlist has: every game the playlist deals plays every slot, so
    /// it is the shortest of *their* lists. A game on the pre-menu but not yet in
    /// [`GameKind::PLAYLIST_ORDER`] does not shorten anybody's playlist.
    pub fn slots(&self) -> usize {
        GameKind::PLAYLIST_ORDER
            .iter()
            .map(|game| self.0.get(*game).len())
            .min()
            .unwrap_or(0)
    }

    /// the theme a game plays at a slot, as an index within that game's own set
    fn theme(&self, game: GameKind, slot: usize) -> usize {
        self.0.get(game)[slot]
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
    fn next_stage(
        &self,
        themes: &Themes,
        player: u32,
        completed: u32,
    ) -> Option<StageChange<AnyGame>>;
    fn controllers(&self) -> Vec<Controller>;
}

/// The mode that plays one game on its own. Every game has one, and this is the only place
/// that names them, so the pre-menu, the high score screen and the tests below all offer
/// exactly the same set.
pub fn game_mode(game: GameKind) -> Box<dyn Mode> {
    match game {
        GameKind::DrRustario => Box::new(DrRustarioMode::new()),
        GameKind::Rustris => Box::new(RustrisMode::new()),
        GameKind::Puyo => Box::new(PuyoMode::new()),
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
        MenuSounds::MODERN
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

    fn high_score_key(&self) -> HighScoreKey {
        let rules = self.options.rules();
        HighScoreKey::new(
            self.title(),
            rules.name(puyo_rusto::options::STAGE_NOUN),
            rules.ranking(),
        )
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

// ---------------------------------------------------------------- Versus

const PLAYLIST: &str = "playlist";
const DIFFICULTY: &str = "difficulty";
const VS_AI_PREFIX: &str = "vs ";
const VS_AI_SUFFIX: &str = " ai";
const AI_DEMO_1P: &str = "1-player ai demo";
const AI_DEMO_2P: &str = "2-player ai demo";

/// A versus ai difficulty is each game's own difficulty of the same name, so an ai opponent is
/// exactly as strong, and as speed limited, as it would be in that game on its own. Every game
/// declares the same four names, which is what [`ai_difficulties_agree`] holds them to.
pub type AiDifficulty = dr_rustario::game::rules::AiDifficulty;

/// Who is playing a versus match. A playlist deals every game, so an ai player is a brain per
/// game - and each of them is simply that game's own ai for the mode chosen: the modes, the
/// difficulties and the demo pairings are the games' own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VersusAi {
    /// no ai players
    #[default]
    Off,
    /// one board played by the ai at full speed
    Demo,
    /// two boards played by the ai at full speed, each game fielding the two models it puts
    /// against each other in its own 2-player demo
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

    /// the ai players one game fields for this mode: which board each of them plays, and the
    /// brain they think with there - the game's own, at the game's own key rate
    fn ai_players(&self, game: GameKind) -> Vec<(u32, Box<dyn AiBrain>)> {
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
        }
    }

    /// every ai player of this mode, and the brain each of them thinks with in every game.
    ///
    /// The games agree on what a mode means - the same boards played by the ai, under the same
    /// four difficulty names, which [`every_mode_offers_the_same_ai_opponents_and_demos`] holds
    /// them to - so an ai player is one brain from each game's list. A game joining the
    /// compendium adds a brain to each player rather than a dimension to this.
    fn brains(&self) -> Vec<(u32, Vec<Box<dyn AiBrain>>)> {
        let mut per_game: Vec<std::vec::IntoIter<(u32, Box<dyn AiBrain>)>> = GameKind::ALL
            .into_iter()
            .map(|game| self.ai_players(game).into_iter())
            .collect();
        let players = per_game.iter().map(|game| game.len()).min().unwrap_or(0);
        (0..players)
            .map(|_| {
                let mut board = None;
                let brains = per_game
                    .iter_mut()
                    .map(|game| {
                        let (plays, brain) = game.next().expect("a brain per game per player");
                        // taken by position, so the games have to agree on which board each
                        // of their ai players takes - as they do, offering the same modes
                        debug_assert_eq!(*board.get_or_insert(plays), plays);
                        brain
                    })
                    .collect();
                (board.expect("a game to field the player"), brains)
            })
            .collect()
    }
}

/// How the games are sequenced. Every player plays the same playlist, so it is always
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
            Playlist::ThemeRace => "theme race",
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

    /// the races rank best times, the marathons rank highest scores
    pub fn ranking(&self) -> Ranking {
        if self.is_race() {
            Ranking::LowestTime
        } else {
            Ranking::HighestScore
        }
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
            // every game takes a turn at every slot
            _ => Some(GameKind::PLAYLIST_COUNT * themes.slots()),
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

    /// which stage of the playlist a player is on after `completed` stages: the races end
    /// with the playlist, the fixed marathons cycle it forever and the random marathon
    /// never repeats
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
    pub fn first_game(&self, seed: u64) -> GameKind {
        match self {
            Playlist::RandomSprint { .. } | Playlist::RandomMarathon => random_game(seed, 0),
            // every fixed playlist starts on whichever game the turn order opens with
            _ => GameKind::PLAYLIST_ORDER[0],
        }
    }

    /// the stages of the fixed playlists, as the game and theme of each; the random
    /// playlists are dealt by [`random_stage`] instead
    fn fixed_stages(&self, themes: &PlaylistThemes) -> Vec<(GameKind, ThemeMode)> {
        let order = GameKind::PLAYLIST_ORDER;
        let slots = themes.slots();
        match self {
            // theme by theme, the games taking turns: the race runs the playlist once, the
            // interleaved, retro and particle marathons cycle theirs forever. A stage names
            // the theme it wants rather than asking for the next one, since the game the
            // playlist has just dealt has been away and would otherwise start over on its
            // first theme every time its turn came round again
            Playlist::ThemeRace | Playlist::Interleaved | Playlist::Retro | Playlist::Particle => {
                (0..slots)
                    .flat_map(|slot| {
                        order
                            .iter()
                            .map(move |game| (*game, ThemeMode::Fixed(themes.theme(*game, slot))))
                            .collect::<Vec<(GameKind, ThemeMode)>>()
                    })
                    .collect()
            }
            Playlist::BackToBack => order
                .iter()
                .flat_map(|game| {
                    (0..slots).map(move |slot| (*game, ThemeMode::Fixed(themes.theme(*game, slot))))
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

/// the game dealt to a random playlist's stage; independent of the theme count, so the
/// opening game is known before the themes are
fn random_game(seed: u64, index: usize) -> GameKind {
    let roll = stage_roll(seed, index, 0) % GameKind::PLAYLIST_COUNT as u64;
    GameKind::PLAYLIST_ORDER[roll as usize]
}

/// the stage a random playlist deals: a random game and theme, never repeating the exact
/// game and theme of the stage before, so every stage change changes something on screen
fn random_stage(seed: u64, index: usize, themes: &PlaylistThemes) -> (GameKind, ThemeMode) {
    let slots = themes.slots();
    if slots == 0 {
        return (random_game(seed, index), ThemeMode::Fixed(0));
    }
    let mut previous: Option<(GameKind, usize)> = None;
    for i in 0..=index {
        let game = random_game(seed, i);
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

/// One 0-10 dial for every game, shared by every playlist: it sets Dr. Rustario's virus
/// level and fall speed and Rustris's starting level together. 0 is the gentlest start
/// (no viruses, level 0, low fall speed) and each step up adds a virus level and a
/// Rustris level, with the fall speed stepping up along the way. What it means to one game
/// is [`Difficulty::level`] and that game's own arm of it.
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

    /// The starting level the dial sets in one game, in that game's own terms: a virus level
    /// in Dr. Rustario, a Rustris level in Rustris.
    ///
    /// One arm per game rather than one method per game, so that a game joining the
    /// compendium is a row in this table and the compiler asks for it.
    fn level(&self, game: GameKind) -> u32 {
        match game {
            // one virus level per step
            GameKind::DrRustario => self.0,
            // one Rustris starting level per step (the guideline fall speed curve runs to
            // level 14, so even 10 leaves headroom)
            GameKind::Rustris => self.0,
            // one speed step per dial step. Phase 5 of `docs/puyo-puyo-plan.md` measures
            // what this should really be, along with whether the colour count belongs on
            // the dial too; until then Puyo is not in `GameKind::PLAYLIST_ORDER` and no
            // playlist asks
            GameKind::Puyo => self.0,
        }
    }

    /// How fast pieces fall in Dr. Rustario: low up to 3, medium up to 7, high from 8.
    ///
    /// A dial of its own because Dr. Rustario has one; Rustris does not, since there its
    /// level *is* its fall speed. A game with its own speed dial says so in its own terms
    /// here, the way this one does.
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
    ai: VersusAi,
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
            ai: VersusAi::Off,
            seed: Cell::new(0),
        }
    }

    /// the title screen's players list: humans, then the ai opponents and the ai demos, the
    /// same set each game offers on its own
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

    /// the themes the chosen playlist deals from, falling back to every theme should a
    /// game turn out to have none of the family the playlist wants: a playlist with no
    /// stages could not be played at all
    fn playlist_themes(&self, themes: &Themes) -> PlaylistThemes {
        let family = themes.playlist(self.playlist.theme_family());
        if family.slots() > 0 {
            family
        } else {
            themes.playlist(None)
        }
    }

    /// the seed every copy of `kind` in this match is dealt from, derived from the match
    /// seed: a playlist starts each player's game as their own board reaches it, so a game
    /// dealt to player 2 three stages after player 1 must still be dealt from the same seed
    fn game_seed(&self, kind: GameKind) -> engine::game::random::Seed {
        // one salt per game so the games do not shadow each other; taken from where the game
        // is numbered, so a new one is dealt its own stream without being given a number here
        let salt = kind.index() as u64 + 1;
        engine::game::random::Seed::from_u64(splitmix64(self.seed.get() ^ splitmix64(salt)))
    }

    /// `count` games of a kind at this difficulty, sharing a seed, for the players from
    /// `first_player` on.
    ///
    /// Which player each is for matters only to Puyo Rusto, whose boards are drawn from a
    /// sprite set per player: a playlist swapping one board over mid-match deals a single
    /// game, and it has to be that player's puyos rather than the first player's.
    fn new_games(
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
                // one set of puyos per player, so two Puyo boards are never the same ones -
                // and dealt off the match seed, so a player swapping onto Puyo three stages
                // into a playlist is handed the set they had the last time round
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
        let (players, current) = self.players_list(max_players);
        vec![MenuItem::select_list(PLAYERS, players, current)]
    }

    fn title_select(&mut self, name: &str, value: &str) {
        if name == PLAYERS {
            self.select_players(value);
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
            self.playlist.first_game(self.seed.get()),
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
        let playlist_themes = self.playlist_themes(themes);
        let seed = self.seed.get();
        let (kind, theme_mode) = self
            .playlist
            .stage(seed, completed as usize, &playlist_themes)?;
        let previous = completed
            .checked_sub(1)
            .and_then(|i| self.playlist.stage(seed, i as usize, &playlist_themes))
            .map(|(kind, _)| kind);
        // the same game again keeps its board and hold: only the theme changes
        let game = if previous == Some(kind) {
            None
        } else {
            Some(self.new_games(kind, 1, player as usize).ok()?.pop()?)
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
        self.ai
            .brains()
            .into_iter()
            .map(|(player, mut brains)| {
                // the playlist swaps the board out from under whichever brain was playing, so
                // they all forget what they had queued as the game changes; the one whose
                // game has just been dealt then plays it and the rest do nothing
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
    use std::collections::HashSet;

    /// the same list of themes for every game
    fn same_themes(themes: Vec<usize>) -> PlaylistThemes {
        PlaylistThemes::new(PerGame::new(|_| themes.clone()))
    }

    /// four themes each, the last of them the particle theme, as every game is built
    fn all_themes() -> PlaylistThemes {
        same_themes(vec![0, 1, 2, 3])
    }

    /// how many stages a fixed playlist over `all_themes` deals before it repeats: one turn
    /// per game the playlist deals, per theme slot
    const THEME_SLOTS: usize = 4;
    const FIXED_STAGES: usize = GameKind::PLAYLIST_COUNT * THEME_SLOTS;

    fn stages(playlist: Playlist, seed: u64, count: usize) -> Vec<(GameKind, ThemeMode)> {
        let themes = all_themes();
        (0..count)
            .map(|i| playlist.stage(seed, i, &themes).unwrap())
            .collect()
    }

    /// one turn each, in the order the playlist deals them, all on the same theme slot
    fn turns(slot: usize) -> Vec<(GameKind, ThemeMode)> {
        GameKind::PLAYLIST_ORDER
            .iter()
            .map(|game| (*game, ThemeMode::Fixed(slot)))
            .collect()
    }

    /// every mode the pre-menu offers: each game on its own, then the versus playlist
    fn all_modes() -> Vec<Box<dyn Mode>> {
        GameKind::ALL
            .into_iter()
            .map(game_mode)
            .chain([Box::new(VersusMode::new()) as Box<dyn Mode>])
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
        }
    }

    #[test]
    fn theme_race_alternates_games_through_every_theme() {
        let stages = stages(Playlist::ThemeRace, 0, FIXED_STAGES);
        // every game takes a turn on a theme slot before the playlist moves on to the next
        for (slot, turn) in stages.chunks(GameKind::PLAYLIST_COUNT).enumerate() {
            assert_eq!(turn, turns(slot), "slot {slot}");
        }
        // ... and the race ends with the playlist rather than cycling it
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

    #[test]
    fn every_mode_offers_the_same_ai_opponents_and_demos() {
        for mode in all_modes() {
            assert_eq!(
                mode.title_items(2),
                vec![MenuItem::select_list(PLAYERS, ai_players_list(), 0)],
                "{}",
                mode.title()
            );
        }
    }

    /// the versus mode names an ai difficulty once and asks every game for it, so they all
    /// have to agree on what they are called
    #[test]
    fn ai_difficulties_agree() {
        let versus: Vec<&str> = AiDifficulty::ALL.iter().map(|d| d.name()).collect();
        assert_eq!(versus, vec!["easy", "normal", "hard", "impossible"]);
        for game in GameKind::ALL {
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

    /// an ai player thinks with one brain per game, so that a game joining the compendium is
    /// a brain rather than another dimension
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
        for (player, brains) in brains.iter() {
            assert_eq!(brains.len(), GameKind::COUNT, "player {player}");
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

    /// a versus ai plays whichever game the playlist deals it, and goes on playing after the
    /// board is swapped for another game's
    #[test]
    fn a_versus_ai_plays_every_game() {
        let mut mode = VersusMode::new();
        mode.title_select(PLAYERS, AI_DEMO_1P);
        let mut controllers = mode.controllers();
        let (_, controller) = &mut controllers[0];

        // every game in turn, and then back to the first: a brain has to survive its board
        // being taken away and given back
        let dealt = GameKind::ALL.into_iter().chain([GameKind::ALL[0]]);
        for kind in dealt {
            let mut played = mode.new_games(kind, 1, 0).unwrap().pop().unwrap();
            let mut alone = mode.new_games(kind, 1, 0).unwrap().pop().unwrap();
            // the ai hard drops every piece, so a few seconds of it locks several; the same
            // few seconds of gravity alone at difficulty 0 locks at most one
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

    /// a game played on its own deals boards of *its* game, one per player. It is the one
    /// thing `game_mode` could get wrong that nothing else would notice, since every board
    /// past the first is only ever seen through [`AnyGame`]
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
        for game in GameKind::ALL {
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
        // every game offers the same four modes, whatever it calls its stages
        for game in GameKind::ALL {
            let mode = game_mode(game);
            let keys = mode.all_high_score_keys();
            assert_eq!(keys.len(), 4, "{game:?}");
            assert!(keys.contains(&mode.high_score_key()), "{game:?}");
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
                "1 level sprint",
                "theme sprint",
                "10,000 point sprint"
            ]
        );

        let versus = VersusMode::new();
        let keys = versus.all_high_score_keys();
        assert_eq!(
            keys.iter().map(|k| k.mode.as_str()).collect::<Vec<_>>(),
            vec![
                "theme race",
                "interleaved marathon",
                "back to back marathon",
                "retro marathon",
                "particle marathon",
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
        let stages = stages(
            Playlist::Interleaved,
            0,
            FIXED_STAGES + GameKind::PLAYLIST_COUNT,
        );
        // a turn each per theme: every game carries on through its own themes as its turn
        // comes round again, rather than starting over on the first every time
        for slot in 0..THEME_SLOTS {
            let turn = slot * GameKind::PLAYLIST_COUNT;
            assert_eq!(
                stages[turn..turn + GameKind::PLAYLIST_COUNT],
                turns(slot)[..]
            );
        }
        // ... and then it cycles
        assert_eq!(stages[FIXED_STAGES..], stages[..GameKind::PLAYLIST_COUNT]);
    }

    #[test]
    fn the_retro_marathon_cycles_the_retro_themes_of_every_game() {
        // as the themes are built: three retro themes each, then the particle theme
        let retro = same_themes(vec![0, 1, 2]);
        let slots = 3;
        let cycle = GameKind::PLAYLIST_COUNT * slots;
        assert_eq!(Playlist::Retro.theme_family(), Some(ThemeFamily::Retro));
        assert_eq!(Playlist::Retro.rules(&retro), MatchRules::Marathon);

        let stages: Vec<(GameKind, ThemeMode)> = (0..cycle + GameKind::PLAYLIST_COUNT)
            .map(|i| Playlist::Retro.stage(0, i, &retro).unwrap())
            .collect();
        for slot in 0..slots {
            let turn = slot * GameKind::PLAYLIST_COUNT;
            assert_eq!(
                stages[turn..turn + GameKind::PLAYLIST_COUNT],
                turns(slot)[..]
            );
        }
        // never the particle theme, and it cycles rather than ending
        assert!(stages
            .iter()
            .all(|(_, theme)| *theme != ThemeMode::Fixed(3)));
        assert_eq!(stages[cycle..], stages[..GameKind::PLAYLIST_COUNT]);
    }

    #[test]
    fn the_particle_marathon_plays_every_games_particle_theme() {
        let particle = same_themes(vec![3]);
        assert_eq!(
            Playlist::Particle.theme_family(),
            Some(ThemeFamily::Particle)
        );
        assert_eq!(Playlist::Particle.rules(&particle), MatchRules::Marathon);

        let stages: Vec<(GameKind, ThemeMode)> = (0..2 * GameKind::PLAYLIST_COUNT)
            .map(|i| Playlist::Particle.stage(0, i, &particle).unwrap())
            .collect();
        assert_eq!(stages, [turns(3).as_slice(), turns(3).as_slice()].concat());
    }

    #[test]
    fn a_playlist_over_a_family_deals_that_familys_theme_indices() {
        // the games need not number their themes alike: slot 1 is each game's own
        let mixed = PlaylistThemes::new(PerGame::new(|game| match game {
            GameKind::DrRustario => vec![2, 5],
            GameKind::Rustris => vec![0, 1],
            // a game the playlist does not deal yet, with a list of its own
            GameKind::Puyo => vec![0],
        }));
        assert_eq!(mixed.slots(), 2);
        for game in GameKind::ALL {
            let themes = mixed.0.get(game);
            let Some(turn) = GameKind::PLAYLIST_ORDER.iter().position(|g| *g == game) else {
                continue;
            };
            // slot 1 of the playlist, as that game numbers it
            assert_eq!(
                Playlist::Retro.stage(0, GameKind::PLAYLIST_COUNT + turn, &mixed),
                Some((game, ThemeMode::Fixed(themes[1])))
            );
        }
        // and a playlist is only as long as the shortest of the lists
        let short = PlaylistThemes::new(PerGame::new(|game| match game {
            GameKind::DrRustario => vec![0],
            GameKind::Rustris => vec![0, 1, 2],
            GameKind::Puyo => vec![0, 1, 2, 3],
        }));
        assert_eq!(short.slots(), 1);
    }

    #[test]
    fn back_to_back_plays_one_game_then_the_other() {
        let stages = stages(Playlist::BackToBack, 0, FIXED_STAGES);
        // all of one game's themes, then all of the next's, in the order they are billed
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
            assert_eq!(Playlist::RandomMarathon.first_game(seed), game);
            assert_eq!(Playlist::RandomSprint { stages: 3 }.first_game(seed), game);
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

    /// A game's board and queue, for comparing two players' copies of it.
    ///
    /// Puyo cell and piece ids carry the player's sprite set as well as the puyo, and the two
    /// players are deliberately dealt different sets - so those are read back onto the first
    /// player's before comparing. What is being tested is the game, not the art.
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
        let mut mode = VersusMode::new();
        mode.difficulty = Difficulty::new(difficulty);
        mode.seed.set(seed);
        mode
    }

    /// the playlist deals each player's game as their own board reaches it, so two players
    /// three stages apart must still be dealt the same bottle - and the same pieces
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

    /// Puyo Rusto draws each player's board from its own set of puyos, so the two players of a
    /// match must not be dealt the same one - and a board dealt on its own, which is how a
    /// playlist swaps one player over, has to be that player's rather than the first player's.
    #[test]
    fn every_player_is_dealt_their_own_puyos() {
        let mode = versus_at(12345, 10);
        let together = mode.new_games(GameKind::Puyo, 2, 0).unwrap();
        let first = skins_of(&together[0]);
        let second = skins_of(&together[1]);
        assert_eq!(first.len(), 1, "a board is drawn from one set");
        assert_eq!(second.len(), 1, "a board is drawn from one set");
        assert_ne!(first, second, "both players were dealt the same puyos");

        // ... and one at a time, as a playlist swaps a board over
        for (player, expected) in [(0, &first), (1, &second)] {
            let alone = mode.new_games(GameKind::Puyo, 1, player).unwrap();
            assert_eq!(&skins_of(&alone[0]), expected, "player {player} alone");
        }
    }

    /// ... and another match is another pair of them, or every game would look the same
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

    /// ... and it is a seed doing that, not every match dealing the same thing
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
