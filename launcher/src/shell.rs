//! The launcher's per-frame state machine: which screen is showing, and how one screen
//! hands over to the next. [`Shell::tick`] runs one frame; `engine::main_loop` drives it.

use crate::games::AnyGame;
use crate::modes::{
    DrRustarioMode, Mode, RustrisMode, Themes, VersusMode, BACK, HIGH_SCORES, START,
};
use engine::app::screens::{
    HighScoreViewScreen, MatchScreen, MenuScreen, NameEntryExit, NameEntryScreen,
};
use engine::app::{
    App, MenuExit, MenuMusic, PostGameAction, MAX_BACKGROUND_PARTICLES, MAX_PARTICLES_PER_PLAYER,
};
use engine::high_score::{HighScoreKey, NewHighScore};
use engine::main_loop::LoopControl;
use engine::menu::sound::MenuSounds;
use engine::menu::MenuItem;
use engine::particles::render::ParticleRender;
use sdl2::render::TextureCreator;
use sdl2::video::WindowContext;

const MAX_PLAYERS: u32 = 2;

const QUIT: &str = "quit";

/// The pre-menu: which of the three modes to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModeChoice {
    Rustris,
    DrRustario,
    Versus,
}

impl ModeChoice {
    const ALL: [ModeChoice; 3] = [
        ModeChoice::Rustris,
        ModeChoice::DrRustario,
        ModeChoice::Versus,
    ];

    fn name(&self) -> &'static str {
        match self {
            ModeChoice::Rustris => "rustris",
            ModeChoice::DrRustario => "dr. rustario",
            ModeChoice::Versus => "dr. rustario vs. rustris",
        }
    }
}

/// A pick from the pre-menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PreMenuAction {
    Play(ModeChoice),
    ViewHighScores,
}

/// The screen currently being ticked.
enum Screen {
    PreMenu(MenuScreen<'static>),
    Title {
        mode: ModeChoice,
        menu: MenuScreen<'static>,
    },
    ModeMenu {
        mode: ModeChoice,
        menu: MenuScreen<'static>,
    },
    Playing {
        mode: ModeChoice,
        /// the high score table the match competes for
        key: HighScoreKey,
        screen: Box<MatchScreen<'static, AnyGame>>,
    },
    HighScores(HighScoreViewScreen<'static>),
    NameEntry {
        mode: ModeChoice,
        screen: NameEntryScreen<'static>,
    },
}

/// A screen handing over: what to show next.
enum Transition {
    ToPreMenu,
    ToHighScores,
    ToTitle(ModeChoice),
    ToModeMenu(ModeChoice),
    StartMatch(ModeChoice),
    ToNameEntry {
        mode: ModeChoice,
        key: HighScoreKey,
        high_score: NewHighScore,
    },
    /// leave name entry, saving the entered name or discarding it
    FinishNameEntry {
        mode: ModeChoice,
        save: bool,
    },
    Exit,
}

pub struct Shell {
    app: App,
    tc: &'static TextureCreator<WindowContext>,
    themes: &'static Themes<'static>,
    fg_particles: ParticleRender<'static>,
    bg_particles: ParticleRender<'static>,
    rustris: RustrisMode,
    dr_rustario: DrRustarioMode,
    versus: VersusMode,
    screen: Screen,
}

impl Shell {
    pub fn new() -> Result<Self, String> {
        let mut app = App::new(MAX_PLAYERS, include_bytes!("../icon.png"))?;
        // the texture creator, and everything borrowing it (themes, screens), lives for
        // the whole process: leak one so the per-frame state machine has no self-references
        let tc: &'static TextureCreator<WindowContext> =
            Box::leak(Box::new(app.canvas().texture_creator()));
        let config = app.config();
        let mut all = dr_rustario::theme::all_themes(app.canvas(), tc, config)?;
        let dr_range = 0..all.len();
        all.extend(rustris::theme::all_themes(app.canvas(), tc, config)?);
        let themes: &'static Themes<'static> = Box::leak(Box::new(Themes {
            rustris: dr_range.end..all.len(),
            dr_rustario: dr_range,
            all,
        }));

        let fg_particles =
            app.particle_render(tc, MAX_PARTICLES_PER_PLAYER * MAX_PLAYERS as usize, vec![])?;
        let mut bg_particles =
            app.particle_render(tc, MAX_BACKGROUND_PARTICLES, themes.all.iter().collect())?;

        let screen = pre_menu(&mut app, tc, themes, &mut bg_particles)?;
        Ok(Self {
            app,
            tc,
            themes,
            fg_particles,
            bg_particles,
            rustris: RustrisMode::new(),
            dr_rustario: DrRustarioMode::new(),
            versus: VersusMode::new(),
            screen,
        })
    }

    /// one frame: tick the current screen and make any handover it asks for
    pub fn tick(&mut self) -> Result<LoopControl, String> {
        match self.update_screen()? {
            None => Ok(LoopControl::Continue),
            Some(Transition::Exit) => Ok(LoopControl::Exit),
            Some(transition) => {
                self.apply(transition)?;
                Ok(LoopControl::Continue)
            }
        }
    }

    fn update_screen(&mut self) -> Result<Option<Transition>, String> {
        let Self {
            app,
            themes,
            fg_particles,
            bg_particles,
            rustris,
            dr_rustario,
            versus,
            screen,
            ..
        } = self;
        let themes = *themes;
        match screen {
            Screen::PreMenu(menu) => {
                let exit = menu.update(app, bg_particles, |name, _| {
                    if name == QUIT {
                        return Some(MenuExit::Back);
                    }
                    if name == HIGH_SCORES {
                        return Some(MenuExit::Custom(PreMenuAction::ViewHighScores));
                    }
                    ModeChoice::ALL
                        .iter()
                        .find(|m| m.name() == name)
                        .map(|m| MenuExit::Custom(PreMenuAction::Play(*m)))
                })?;
                Ok(exit.map(|exit| match exit {
                    MenuExit::Custom(PreMenuAction::Play(mode)) => Transition::ToTitle(mode),
                    MenuExit::Custom(PreMenuAction::ViewHighScores) => Transition::ToHighScores,
                    // Start just re-enters the pre-menu, as the old loop's `continue` did
                    MenuExit::Start => Transition::ToPreMenu,
                    MenuExit::Back | MenuExit::Quit => Transition::Exit,
                }))
            }
            Screen::Title { mode, menu } => {
                let m = choose_mode(rustris, dr_rustario, versus, *mode);
                let exit = menu.update::<()>(app, bg_particles, |name, value| match name {
                    START => Some(MenuExit::Start),
                    BACK => Some(MenuExit::Back),
                    _ => {
                        m.title_select(name, value);
                        None
                    }
                })?;
                Ok(exit.map(|exit| match exit {
                    MenuExit::Start | MenuExit::Custom(()) => Transition::ToModeMenu(*mode),
                    MenuExit::Back => Transition::ToPreMenu,
                    MenuExit::Quit => Transition::Exit,
                }))
            }
            Screen::ModeMenu { mode, menu } => {
                let m = choose_mode(rustris, dr_rustario, versus, *mode);
                let mut selected = false;
                let exit = menu.update::<()>(app, bg_particles, |name, value| match name {
                    START => Some(MenuExit::Start),
                    BACK => Some(MenuExit::Back),
                    _ => {
                        m.menu_select(name, value);
                        selected = true;
                        None
                    }
                })?;
                // one option can change what another offers: a single theme takes the
                // theme sprint off the mode list
                if selected {
                    menu.set_items(app, &m.menu_items())?;
                }
                Ok(exit.map(|exit| match exit {
                    MenuExit::Start | MenuExit::Custom(()) => Transition::StartMatch(*mode),
                    MenuExit::Back => Transition::ToTitle(*mode),
                    MenuExit::Quit => Transition::Exit,
                }))
            }
            Screen::Playing { mode, key, screen } => {
                let m: &dyn Mode = choose_mode(rustris, dr_rustario, versus, *mode);
                let exit =
                    screen.update(app, fg_particles, bg_particles, |player, completed| {
                        m.next_stage(themes, player, completed)
                    })?;
                Ok(exit.map(|exit| match exit {
                    PostGameAction::NewHighScore(high_score) => Transition::ToNameEntry {
                        mode: *mode,
                        key: key.clone(),
                        high_score,
                    },
                    PostGameAction::ReturnToMenu => Transition::ToModeMenu(*mode),
                    PostGameAction::Quit => Transition::Exit,
                }))
            }
            Screen::HighScores(view) => Ok(view
                .update(app, bg_particles)?
                .map(|()| Transition::ToPreMenu)),
            Screen::NameEntry { mode, screen } => {
                Ok(screen
                    .update(app, bg_particles)?
                    .map(|exit| Transition::FinishNameEntry {
                        mode: *mode,
                        save: matches!(exit, NameEntryExit::Done),
                    }))
            }
        }
    }

    fn apply(&mut self, transition: Transition) -> Result<(), String> {
        self.screen = match transition {
            Transition::ToPreMenu => {
                pre_menu(&mut self.app, self.tc, self.themes, &mut self.bg_particles)?
            }
            Transition::ToHighScores => {
                // every table of every game and mode, at its defaults until played
                let keys = [
                    self.rustris.all_high_score_keys(),
                    self.dr_rustario.all_high_score_keys(),
                    self.versus.all_high_score_keys(),
                ]
                .concat();
                match HighScoreViewScreen::new(
                    &mut self.app,
                    self.tc,
                    &keys,
                    &mut self.bg_particles,
                )? {
                    Some(view) => Screen::HighScores(view),
                    // nothing to show: straight back, as the old code's early return
                    None => pre_menu(&mut self.app, self.tc, self.themes, &mut self.bg_particles)?,
                }
            }
            Transition::ToTitle(mode) => {
                self.app.set_menu_sound(self.mode(mode).menu_sounds())?;
                self.title_screen(mode)?
            }
            Transition::ToModeMenu(mode) => self.mode_menu_screen(mode)?,
            Transition::StartMatch(mode) => {
                let themes = self.themes;
                let m = self.mode(mode);
                let games = m.games()?;
                let settings = m.settings(themes);
                let key = settings.high_score_key.clone();
                let controllers = m.controllers();
                let screen = MatchScreen::new(
                    &mut self.app,
                    self.tc,
                    &themes.all,
                    games,
                    settings,
                    &mut self.fg_particles,
                    &mut self.bg_particles,
                    controllers,
                )?;
                Screen::Playing {
                    mode,
                    key,
                    screen: Box::new(screen),
                }
            }
            Transition::ToNameEntry {
                mode,
                key,
                high_score,
            } => {
                let screen = NameEntryScreen::new(
                    &mut self.app,
                    self.tc,
                    &key,
                    high_score,
                    &mut self.bg_particles,
                )?;
                Screen::NameEntry { mode, screen }
            }
            Transition::FinishNameEntry { mode, save } => {
                let next = self.mode_menu_screen(mode)?;
                let old = std::mem::replace(&mut self.screen, next);
                if save {
                    if let Screen::NameEntry { screen, .. } = old {
                        screen.finish()?;
                    }
                }
                return Ok(());
            }
            Transition::Exit => unreachable!("Exit is handled by tick"),
        };
        Ok(())
    }

    fn mode(&self, choice: ModeChoice) -> &dyn Mode {
        match choice {
            ModeChoice::Rustris => &self.rustris,
            ModeChoice::DrRustario => &self.dr_rustario,
            ModeChoice::Versus => &self.versus,
        }
    }

    fn title_screen(&mut self, mode: ModeChoice) -> Result<Screen, String> {
        let m = self.mode(mode);
        let mut items = m.title_items(MAX_PLAYERS);
        items.push(MenuItem::select(START));
        items.push(MenuItem::select(BACK));
        let title = m.title();
        let race = m.race(self.themes);
        let menu = MenuScreen::new(
            &mut self.app,
            self.tc,
            items,
            title,
            None,
            MenuMusic::Title,
            &mut self.bg_particles,
            &race,
        )?;
        Ok(Screen::Title { mode, menu })
    }

    fn mode_menu_screen(&mut self, mode: ModeChoice) -> Result<Screen, String> {
        let m = self.mode(mode);
        let mut items = m.menu_items();
        items.push(MenuItem::select(START));
        items.push(MenuItem::select(BACK));
        let title = m.title();
        let subtitle = Some(m.subtitle());
        let race = m.race(self.themes);
        let menu = MenuScreen::new(
            &mut self.app,
            self.tc,
            items,
            title,
            subtitle,
            MenuMusic::Menu,
            &mut self.bg_particles,
            &race,
        )?;
        Ok(Screen::ModeMenu { mode, menu })
    }
}

fn pre_menu(
    app: &mut App,
    tc: &'static TextureCreator<WindowContext>,
    themes: &Themes,
    bg_particles: &mut ParticleRender,
) -> Result<Screen, String> {
    app.set_menu_sound(MenuSounds::MODERN)?;
    let items = ModeChoice::ALL
        .iter()
        .map(|m| MenuItem::select(m.name()))
        .chain([MenuItem::select(HIGH_SCORES), MenuItem::select(QUIT)])
        .collect();
    let menu = MenuScreen::new(
        app,
        tc,
        items,
        "Dr. Rustario vs. Rustris".to_string(),
        None,
        MenuMusic::Title,
        bg_particles,
        &themes.race_all(),
    )?;
    Ok(Screen::PreMenu(menu))
}

fn choose_mode<'m>(
    rustris: &'m mut RustrisMode,
    dr_rustario: &'m mut DrRustarioMode,
    versus: &'m mut VersusMode,
    choice: ModeChoice,
) -> &'m mut dyn Mode {
    match choice {
        ModeChoice::Rustris => rustris,
        ModeChoice::DrRustario => dr_rustario,
        ModeChoice::Versus => versus,
    }
}
