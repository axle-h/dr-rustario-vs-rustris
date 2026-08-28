//! The application shell: window, audio and the per-frame screens. Generic over the game
//! being played; a launcher decides which games and themes to run and ticks the screens.

pub mod loading;
pub mod screens;

use crate::audio;
use crate::config::{Config, VideoMode};
use crate::controller::Controllers;
use crate::high_score::{HighScoreKey, NewHighScore};
use crate::icon::app_icon;
use crate::menu::sound::{MenuSound, MenuSounds};
use crate::particles::prescribed::{prescribed_fireworks, prescribed_piece_race, RaceTheme};
use crate::particles::render::ParticleRender;
use crate::particles::scale::Scale as ParticleScale;
use crate::particles::source::ParticleSource;
use crate::particles::Particles;
use crate::render::context::PlayerThemes;
use crate::render::Theme;
use crate::session::MatchRules;
use sdl2::rect::Rect;
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use sdl2::{EventPump, Sdl};
use std::ops::Range;

/// Simultaneous sound effects per player (SDL_mixer's default channel count).
const EFFECT_CHANNELS_PER_PLAYER: u32 = 8;

pub const MAX_PARTICLES_PER_PLAYER: usize = 100000;
pub const MAX_BACKGROUND_PARTICLES: usize = 100000;

/// How a menu screen ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuExit<R> {
    Start,
    Back,
    Quit,
    Custom(R),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuMusic {
    Title,
    Menu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostGameAction {
    NewHighScore(NewHighScore),
    ReturnToMenu,
    Quit,
}

/// Which theme a player plays on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    /// every theme in order, advancing at each stage
    All,
    /// one theme, by index within the player's set
    Fixed(usize),
}

impl ThemeMode {
    pub fn initial(&self) -> usize {
        match self {
            ThemeMode::All => 0,
            ThemeMode::Fixed(index) => *index,
        }
    }
}

/// One player's game setup within a match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerSettings {
    /// the player's themes: a range of indices into the match's theme list
    pub themes: Range<usize>,
    pub theme_mode: ThemeMode,
}

impl PlayerSettings {
    pub fn player_themes(&self) -> PlayerThemes {
        PlayerThemes::new(
            self.themes.clone(),
            self.themes.start + self.theme_mode.initial(),
        )
    }
}

/// What a playlist does to a player at a stage boundary: a new game, or the same game on
/// different themes.
pub struct StageChange<G> {
    /// `None` carries the current game (its stack and hold) into the next stage
    pub game: Option<G>,
    pub settings: PlayerSettings,
}

/// Which of a theme's game tracks a match is played on.
///
/// It belongs to the match rather than to a player because only one track ever plays:
/// [`crate::render::ThemeContext`] keeps the music on one player's theme. `Track` indexes the
/// tracks that theme was given, in the order the game's own menu lists them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MusicChoice {
    /// dealt afresh every match, and again whenever the theme changes under it
    #[default]
    Random,
    Track(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchSettings {
    pub rules: MatchRules,
    pub players: Vec<PlayerSettings>,
    /// which high score table the match competes for
    pub high_score_key: HighScoreKey,
    /// stages may switch games, so every stage boundary shows the stage card
    pub playlist: bool,
    /// which track of the theme in play the match is played on
    pub music: MusicChoice,
}

pub struct App {
    config: Config,
    _sdl: Sdl,
    canvas: WindowCanvas,
    event_pump: EventPump,
    controllers: Controllers,
    _audio: audio::Audio,
    menu_sound: MenuSound,
    particle_scale: ParticleScale,
    max_players: u32,
}

impl App {
    pub fn new(max_players: u32, icon: &[u8]) -> Result<Self, String> {
        let config = Config::load()?;
        let sdl = sdl2::init()?;
        let video = sdl.video()?;

        if config.video.disable_screensaver && video.is_screen_saver_enabled() {
            video.disable_screen_saver();
        }

        let (width, height) = match config.video.mode {
            VideoMode::Window { width, height } => (width, height),
            VideoMode::FullScreen { width, height } => (width, height),
            _ => (1, 1),
        };

        let mut window_builder = video.window(crate::app_info::get().name, width, height);
        match config.video.mode {
            VideoMode::FullScreen { .. } => {
                window_builder.fullscreen();
            }
            VideoMode::FullScreenDesktop => {
                window_builder.fullscreen_desktop();
            }
            _ => {}
        };

        let mut window = window_builder
            .position_centered()
            .opengl()
            .build()
            .map_err(|e| e.to_string())?;

        window.set_icon(app_icon(icon)?);

        let canvas_builder = window.into_canvas().target_texture().accelerated();

        let mut canvas = if config.video.vsync {
            canvas_builder.present_vsync()
        } else {
            canvas_builder
        }
        .build()
        .map_err(|e| e.to_string())?;

        // Frame zero, before a single theme is built. A Wayland toplevel is not mapped until
        // the client commits a buffer, so until something is presented the window does not
        // exist as far as the compositor is concerned - which is what a PortMaster session's
        // `swaymsg [app_id=...] fullscreen enable` helper spends the whole load failing to
        // find. See `crate::app::loading`, which draws the bar over the top of this.
        canvas.set_draw_color(loading::BACKGROUND);
        canvas.clear();
        canvas.present();

        // the window built under FullScreenDesktop (or any mode the WM adjusts) only knows
        // its real size now: particle space is normalised to it, so it must be the real one
        let (window_width, window_height) = canvas.window().size();

        let event_pump = sdl.event_pump()?;
        // SDL_GAMECONTROLLERCONFIG (set by e.g. PortMaster) is read by the subsystem on init;
        // already-attached pads arrive as ControllerDeviceAdded events on the first poll
        let controllers = Controllers::new(sdl.game_controller()?, max_players);

        let audio = audio::Audio::open(
            &sdl.audio()?,
            max_players * EFFECT_CHANNELS_PER_PLAYER,
            config.audio.music_volume(),
        )?;
        let menu_sound = MenuSound::new(config.audio, MenuSounds::MODERN)?;

        Ok(Self {
            config,
            _sdl: sdl,
            canvas,
            event_pump,
            controllers,
            _audio: audio,
            menu_sound,
            particle_scale: ParticleScale::new((window_width, window_height)),
            max_players,
        })
    }

    pub fn config(&self) -> Config {
        self.config
    }

    /// the menu sounds and music from here on
    pub fn set_menu_sound(&mut self, sounds: MenuSounds) -> Result<(), String> {
        self.menu_sound = MenuSound::new(self.config.audio, sounds)?;
        Ok(())
    }

    pub fn canvas(&mut self) -> &mut WindowCanvas {
        &mut self.canvas
    }

    /// The canvas and the event pump at once, for a long job that runs before the main loop.
    ///
    /// Building the themes draws with one and has to keep pumping the other, and it holds
    /// both for the length of the load - which one `&mut self` at a time cannot give it. See
    /// [`crate::app::loading`] for why a load that never pumps shows nothing.
    pub fn canvas_and_events(&mut self) -> (&mut WindowCanvas, &mut EventPump) {
        (&mut self.canvas, &mut self.event_pump)
    }

    pub fn max_players(&self) -> u32 {
        self.max_players
    }

    pub fn particle_render<'a>(
        &mut self,
        texture_creator: &'a TextureCreator<WindowContext>,
        max_particles: usize,
        themes: Vec<&Theme<'a>>,
    ) -> Result<ParticleRender<'a>, String> {
        ParticleRender::new(
            &mut self.canvas,
            Particles::new(max_particles),
            texture_creator,
            self.particle_scale,
            themes,
            self.config.video.particle_density,
        )
    }

    fn race_particle_source(&self, race: &[RaceTheme]) -> Box<dyn ParticleSource> {
        let (window_width, window_height) = self.canvas.window().size();
        prescribed_piece_race(
            Rect::new(0, 0, window_width, window_height),
            &self.particle_scale,
            race,
        )
    }

    fn fireworks_particle_source(&self) -> Box<dyn ParticleSource> {
        let (window_width, window_height) = self.canvas.window().size();
        prescribed_fireworks(
            Rect::new(0, 0, window_width, window_height),
            &self.particle_scale,
        )
    }
}
