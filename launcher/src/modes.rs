//! The three things the launcher can run: each game on its own, exactly as it was standalone,
//! and a versus mode where every player plays the same playlist of both games.

use crate::games::{AnyGame, GameKind};
use dr_rustario::game::ai::DrAiKind;
use engine::app::{MatchSettings, PlayerSettings, StageChange, ThemeMode};
use engine::high_score::table::Ranking;
use engine::high_score::HighScoreKey;
use engine::menu::sound::MenuSounds;
use engine::menu::MenuItem;
use engine::particles::prescribed::RaceTheme;
use engine::render::{Theme, ThemeFamily};
use engine::session::MatchRules;
use rustris::game::ai::models::TetrisNeuralNetwork;
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
        PlaylistThemes::new(
            self.family(GameKind::DrRustario, family),
            self.family(GameKind::Rustris, family),
        )
    }

    pub fn race_all(&self) -> Vec<RaceTheme> {
        let mut race = self.race(GameKind::DrRustario);
        race.extend(self.race(GameKind::Rustris));
        race
    }
}

/// The themes a playlist deals, as indices within each game's own set. A playlist deals
/// *slots*: at slot `n` each game plays its `n`th theme, so a playlist is only as long as
/// the shorter of the two lists.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlaylistThemes {
    dr_rustario: Vec<usize>,
    rustris: Vec<usize>,
}

impl PlaylistThemes {
    pub fn new(dr_rustario: Vec<usize>, rustris: Vec<usize>) -> Self {
        Self {
            dr_rustario,
            rustris,
        }
    }

    /// how many slots the playlist has: both games play every slot, so it is the shorter
    /// of the two lists
    pub fn slots(&self) -> usize {
        self.dr_rustario.len().min(self.rustris.len())
    }

    /// the theme a game plays at a slot, as an index within that game's own set
    fn theme(&self, game: GameKind, slot: usize) -> usize {
        match game {
            GameKind::DrRustario => self.dr_rustario[slot],
            GameKind::Rustris => self.rustris[slot],
        }
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

// ---------------------------------------------------------------- Versus

const PLAYLIST: &str = "playlist";
const DIFFICULTY: &str = "difficulty";
const VS_AI_PREFIX: &str = "vs ";
const VS_AI_SUFFIX: &str = " ai";
const AI_DEMO_1P: &str = "1-player ai demo";
const AI_DEMO_2P: &str = "2-player ai demo";

/// A versus ai difficulty is each game's own difficulty of the same name, so an ai opponent is
/// exactly as strong, and as speed limited, as it would be in that game on its own. The two
/// games declare the same four names, which is what [`ai_difficulties_agree`] holds them to.
pub type AiDifficulty = dr_rustario::game::rules::AiDifficulty;

/// Who is playing a versus match. A playlist deals both games, so every ai player is a pair of
/// brains - a Dr. Rustario one and a Rustris one - and both are simply that game's own ai for
/// the mode chosen: the modes, the difficulties and the demo pairings are the games' own.
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
                    .expect("both games offer the same ai difficulties"),
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

    /// each ai player, the key delay it plays at and the brain it thinks with in each game,
    /// taken from the games themselves
    fn brains(&self) -> Vec<(u32, Duration, DrAiKind, TetrisNeuralNetwork)> {
        let mut dr_config = dr_rustario::game::rules::GameConfig::default();
        dr_config.set_ai(self.dr_rustario());
        let rustris_config = rustris::game::rules::GameConfig {
            ai: self.rustris(),
            ..Default::default()
        };
        dr_config
            .ai_players()
            .into_iter()
            .zip(rustris_config.ai_players())
            .map(|((player, key_delay, brain), (_, _, network))| {
                (player, key_delay, brain, network)
            })
            .collect()
    }
}

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
            _ => Some(2 * themes.slots()),
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
            // every fixed playlist starts on Rustris
            _ => GameKind::Rustris,
        }
    }

    /// the stages of the fixed playlists, as the game and theme of each; the random
    /// playlists are dealt by [`random_stage`] instead
    fn fixed_stages(&self, themes: &PlaylistThemes) -> Vec<(GameKind, ThemeMode)> {
        let order = [GameKind::Rustris, GameKind::DrRustario];
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
                        order.map(|game| (game, ThemeMode::Fixed(themes.theme(game, slot))))
                    })
                    .collect()
            }
            Playlist::BackToBack => order
                .into_iter()
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
        let salt = match kind {
            GameKind::DrRustario => 1,
            GameKind::Rustris => 2,
        };
        engine::game::random::Seed::from_u64(splitmix64(self.seed.get() ^ splitmix64(salt)))
    }

    /// `count` games of a kind at this difficulty, sharing a seed
    fn new_games(&self, kind: GameKind, count: usize) -> Result<Vec<AnyGame>, String> {
        let seed = self.game_seed(kind);
        Ok(match kind {
            GameKind::DrRustario => {
                let mode = dr_rustario::game::random::RandomMode::Bag;
                dr_rustario::game::random::from_seed(seed, count, mode)
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
                rustris::game::random::from_seed(seed.into(), mode, count)
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
        )
    }

    fn next_stage(
        &self,
        themes: &Themes,
        _player: u32,
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
        self.ai
            .brains()
            .into_iter()
            .map(|(player, key_delay, brain, network)| {
                let mut dr_agent =
                    dr_rustario::game::ai::agent::DrAiAgent::of(brain).with_key_delay(key_delay);
                let mut rustris_agent =
                    rustris::game::ai::agent::AiAgent::neural(network).with_key_delay(key_delay);
                // the playlist swaps the board out from under whichever agent was playing, so
                // both forget what they had queued as the game changes
                let mut playing: Option<GameKind> = None;
                let controller = move |game: &mut AnyGame, delta: Duration| {
                    if playing != Some(game.kind()) {
                        dr_agent.reset();
                        rustris_agent.reset();
                        playing = Some(game.kind());
                    }
                    match game {
                        AnyGame::DrRustario(game) => dr_agent.act(game, delta),
                        AnyGame::Rustris(game) => rustris_agent.act(game, delta),
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

    /// four themes each, the last of them the particle theme, as both games are built
    fn all_themes() -> PlaylistThemes {
        PlaylistThemes::new(vec![0, 1, 2, 3], vec![0, 1, 2, 3])
    }

    fn stages(playlist: Playlist, seed: u64, count: usize) -> Vec<(GameKind, ThemeMode)> {
        let themes = all_themes();
        (0..count)
            .map(|i| playlist.stage(seed, i, &themes).unwrap())
            .collect()
    }

    #[test]
    fn theme_race_alternates_games_through_every_theme() {
        let stages = stages(Playlist::ThemeRace, 0, 8);
        assert_eq!(stages[0], (GameKind::Rustris, ThemeMode::Fixed(0)));
        assert_eq!(stages[1], (GameKind::DrRustario, ThemeMode::Fixed(0)));
        assert_eq!(stages[7], (GameKind::DrRustario, ThemeMode::Fixed(3)));
        assert_eq!(Playlist::ThemeRace.stage(0, 8, &all_themes()), None);
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
        for items in [
            DrRustarioMode::new().title_items(2),
            RustrisMode::new().title_items(2),
            VersusMode::new().title_items(2),
        ] {
            assert_eq!(
                items,
                vec![MenuItem::select_list(PLAYERS, ai_players_list(), 0)]
            );
        }
    }

    /// the versus mode names an ai difficulty once and asks both games for it, so the two have
    /// to agree on what they are called
    #[test]
    fn ai_difficulties_agree() {
        let versus: Vec<&str> = AiDifficulty::ALL.iter().map(|d| d.name()).collect();
        let rustris: Vec<&str> = rustris::game::rules::AiDifficulty::ALL
            .iter()
            .map(|d| d.name())
            .collect();
        assert_eq!(versus, rustris);
        assert_eq!(versus, vec!["easy", "normal", "hard", "impossible"]);
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
    /// board is swapped for the other game's
    #[test]
    fn a_versus_ai_plays_both_games() {
        let mut mode = VersusMode::new();
        mode.title_select(PLAYERS, AI_DEMO_1P);
        let mut controllers = mode.controllers();
        let (_, controller) = &mut controllers[0];

        for kind in [GameKind::Rustris, GameKind::DrRustario, GameKind::Rustris] {
            let mut played = mode.new_games(kind, 1).unwrap().pop().unwrap();
            let mut alone = mode.new_games(kind, 1).unwrap().pop().unwrap();
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

    #[test]
    fn picking_an_ai_opponent_puts_the_agent_on_player_two() {
        let mut mode = DrRustarioMode::new();
        assert!(mode.controllers().is_empty());

        mode.title_select(PLAYERS, "vs normal ai");
        let controllers = mode.controllers();
        assert_eq!(controllers.len(), 1);
        assert_eq!(controllers[0].0, 1, "the ai should play as player 2");

        // and the demo plays the first board instead
        mode.title_select(PLAYERS, AI_DEMO_1P);
        let controllers = mode.controllers();
        assert_eq!(controllers.len(), 1);
        assert_eq!(controllers[0].0, 0);

        // the 2-player demo plays both, one row of the N64 ai's weights against another
        mode.title_select(PLAYERS, AI_DEMO_2P);
        let controllers = mode.controllers();
        assert_eq!(
            controllers.iter().map(|(p, _)| *p).collect::<Vec<u32>>(),
            vec![0, 1]
        );

        // back to humans
        mode.title_select(PLAYERS, "2");
        assert!(mode.controllers().is_empty());
    }

    #[test]
    fn every_mode_has_one_table_per_rules_variant() {
        let rustris = RustrisMode::new();
        let keys = rustris.all_high_score_keys();
        assert_eq!(
            keys.iter().map(|k| k.mode.as_str()).collect::<Vec<_>>(),
            vec![
                "marathon",
                "1 level sprint",
                "theme sprint",
                "10,000 point sprint"
            ]
        );
        assert!(keys.contains(&rustris.high_score_key()));
        assert_eq!(DrRustarioMode::new().all_high_score_keys().len(), 4);

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
            MatchRules::StageSprint { stages: 8 }
        );
        assert_eq!(
            Playlist::RandomSprint { stages: 5 }.rules(&all_themes()),
            MatchRules::StageSprint { stages: 5 }
        );
        assert_eq!(
            Playlist::Interleaved.rules(&all_themes()),
            MatchRules::Marathon
        );
        assert_eq!(
            Playlist::BackToBack.rules(&all_themes()),
            MatchRules::Marathon
        );
        assert_eq!(
            Playlist::RandomMarathon.rules(&all_themes()),
            MatchRules::Marathon
        );
    }

    #[test]
    fn the_races_end_with_the_playlist_and_marathons_cycle_it() {
        assert!(Playlist::ThemeRace.stage(0, 7, &all_themes()).is_some());
        assert_eq!(Playlist::ThemeRace.stage(0, 8, &all_themes()), None);
        assert_eq!(
            Playlist::RandomSprint { stages: 3 }.stage(0, 3, &all_themes()),
            None
        );
        assert_eq!(
            Playlist::BackToBack.stage(0, 8, &all_themes()),
            Playlist::BackToBack.stage(0, 0, &all_themes())
        );
        assert_eq!(
            Playlist::Interleaved.stage(0, 13, &all_themes()),
            Playlist::Interleaved.stage(0, 5, &all_themes())
        );
        assert!(Playlist::RandomMarathon
            .stage(0, 10_000, &all_themes())
            .is_some());
        assert_eq!(
            Playlist::ThemeRace.stage(0, 0, &PlaylistThemes::default()),
            None
        );
    }

    #[test]
    fn the_interleaved_marathon_advances_each_games_themes() {
        let stages: Vec<(GameKind, ThemeMode)> = (0..10)
            .map(|i| Playlist::Interleaved.stage(0, i, &all_themes()).unwrap())
            .collect();
        // a pair of stages per theme: each game carries on through its own themes as its
        // turn comes round again, rather than starting over on the first every time
        assert_eq!(stages[0], (GameKind::Rustris, ThemeMode::Fixed(0)));
        assert_eq!(stages[1], (GameKind::DrRustario, ThemeMode::Fixed(0)));
        assert_eq!(stages[2], (GameKind::Rustris, ThemeMode::Fixed(1)));
        assert_eq!(stages[3], (GameKind::DrRustario, ThemeMode::Fixed(1)));
        assert_eq!(stages[7], (GameKind::DrRustario, ThemeMode::Fixed(3)));
        // ... and then it cycles
        assert_eq!(stages[8], stages[0]);
        assert_eq!(stages[9], stages[1]);
    }

    #[test]
    fn the_retro_marathon_cycles_the_retro_themes_of_both_games() {
        // as the themes are built: three retro themes each, then the particle theme
        let retro = PlaylistThemes::new(vec![0, 1, 2], vec![0, 1, 2]);
        assert_eq!(Playlist::Retro.theme_family(), Some(ThemeFamily::Retro));
        assert_eq!(Playlist::Retro.rules(&retro), MatchRules::Marathon);

        let stages: Vec<(GameKind, ThemeMode)> = (0..8)
            .map(|i| Playlist::Retro.stage(0, i, &retro).unwrap())
            .collect();
        assert_eq!(stages[0], (GameKind::Rustris, ThemeMode::Fixed(0)));
        assert_eq!(stages[1], (GameKind::DrRustario, ThemeMode::Fixed(0)));
        assert_eq!(stages[5], (GameKind::DrRustario, ThemeMode::Fixed(2)));
        // never the particle theme, and it cycles rather than ending
        assert!(stages
            .iter()
            .all(|(_, theme)| *theme != ThemeMode::Fixed(3)));
        assert_eq!(stages[6], stages[0]);
    }

    #[test]
    fn the_particle_marathon_plays_the_two_games_particle_themes() {
        let particle = PlaylistThemes::new(vec![3], vec![3]);
        assert_eq!(
            Playlist::Particle.theme_family(),
            Some(ThemeFamily::Particle)
        );
        assert_eq!(Playlist::Particle.rules(&particle), MatchRules::Marathon);

        let stages: Vec<(GameKind, ThemeMode)> = (0..4)
            .map(|i| Playlist::Particle.stage(0, i, &particle).unwrap())
            .collect();
        assert_eq!(
            stages,
            vec![
                (GameKind::Rustris, ThemeMode::Fixed(3)),
                (GameKind::DrRustario, ThemeMode::Fixed(3)),
                (GameKind::Rustris, ThemeMode::Fixed(3)),
                (GameKind::DrRustario, ThemeMode::Fixed(3)),
            ]
        );
    }

    #[test]
    fn a_playlist_over_a_family_deals_that_familys_theme_indices() {
        // the two games need not number their themes alike: slot 1 is each game's own
        let mixed = PlaylistThemes::new(vec![2, 5], vec![0, 1]);
        assert_eq!(mixed.slots(), 2);
        assert_eq!(
            Playlist::Retro.stage(0, 2, &mixed),
            Some((GameKind::Rustris, ThemeMode::Fixed(1)))
        );
        assert_eq!(
            Playlist::Retro.stage(0, 3, &mixed),
            Some((GameKind::DrRustario, ThemeMode::Fixed(5)))
        );
        // and a playlist is only as long as the shorter of the two lists
        assert_eq!(PlaylistThemes::new(vec![0], vec![0, 1, 2]).slots(), 1);
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

    /// a game's board and queue, for comparing two players' copies of it
    fn board_of(game: &AnyGame) -> (Vec<engine::game::Cell>, Vec<engine::game::PieceId>) {
        use engine::game::geometry::Point;
        use engine::game::Game;
        let cells = (0..game.board_height())
            .flat_map(|y| (0..game.board_width()).map(move |x| Point::new(x as i32, y as i32)))
            .map(|p| game.cell(p))
            .collect();
        (cells, game.queue())
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
        for kind in [GameKind::DrRustario, GameKind::Rustris] {
            let mode = versus_at(12345, 10);
            // both at once, as a match starts
            let together = mode.new_games(kind, 2).unwrap();
            assert_eq!(board_of(&together[0]), board_of(&together[1]), "{:?}", kind);
            // and one at a time, as a playlist swaps a board over
            let apart = mode.new_games(kind, 1).unwrap();
            assert_eq!(board_of(&together[0]), board_of(&apart[0]), "{:?}", kind);
        }
    }

    /// ... and it is a seed doing that, not every match dealing the same thing
    #[test]
    fn another_match_is_dealt_another_game() {
        for kind in [GameKind::DrRustario, GameKind::Rustris] {
            let one = versus_at(1, 10).new_games(kind, 1).unwrap();
            let two = versus_at(2, 10).new_games(kind, 1).unwrap();
            assert_ne!(board_of(&one[0]), board_of(&two[0]), "{:?}", kind);
        }
    }

    /// the two games are dealt from the same match seed but must not shadow each other
    #[test]
    fn the_two_games_are_dealt_from_different_seeds() {
        let mode = versus_at(7, 10);
        assert_ne!(
            mode.game_seed(GameKind::DrRustario),
            mode.game_seed(GameKind::Rustris)
        );
    }
}
