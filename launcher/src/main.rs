#![windows_subsystem = "windows"]

mod games;
mod modes;

use crate::modes::{
    DrRustarioMode, Mode, RustrisMode, Themes, VersusMode, BACK, HIGH_SCORES, START,
};
use engine::app::{
    App, MenuExit, MenuMusic, PostGameAction, MAX_BACKGROUND_PARTICLES, MAX_PARTICLES_PER_PLAYER,
};
use engine::menu::sound::MenuSounds;
use engine::menu::MenuItem;
use engine::particles::render::ParticleRender;

mod build_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

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
    const ALL: [ModeChoice; 3] = [ModeChoice::Rustris, ModeChoice::DrRustario, ModeChoice::Versus];

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

/// What happens when a mode's menus are left.
enum Exit {
    ToPreMenu,
    Quit,
}

/// A mode's title screen, main menu and matches, until the player backs out to the pre-menu.
fn run_mode<M: Mode>(
    app: &mut App,
    mode: &mut M,
    themes: &Themes,
    fg_particles: &mut ParticleRender,
    bg_particles: &mut ParticleRender,
) -> Result<Exit, String> {
    app.set_menu_sound(mode.menu_sounds())?;
    let race = mode.race(themes);

    'title: loop {
        let mut items = mode.title_items(MAX_PLAYERS);
        items.push(MenuItem::select(START));
        items.push(MenuItem::select(BACK));
        let exit = app.run_menu::<()>(
            items,
            mode.title(),
            None,
            MenuMusic::Title,
            bg_particles,
            &race,
            |name, value| match name {
                START => Some(MenuExit::Start),
                BACK => Some(MenuExit::Back),
                _ => {
                    mode.title_select(name, value);
                    None
                }
            },
        )?;
        match exit {
            MenuExit::Start => {}
            MenuExit::Back => return Ok(Exit::ToPreMenu),
            MenuExit::Quit => return Ok(Exit::Quit),
            MenuExit::Custom(()) => {}
        }

        'menu: loop {
            let mut items = mode.menu_items();
            items.push(MenuItem::select(START));
            items.push(MenuItem::select(BACK));
            let exit = app.run_menu::<()>(
                items,
                mode.title(),
                Some(mode.subtitle()),
                MenuMusic::Menu,
                bg_particles,
                &race,
                |name, value| match name {
                    START => Some(MenuExit::Start),
                    BACK => Some(MenuExit::Back),
                    _ => {
                        mode.menu_select(name, value);
                        None
                    }
                },
            )?;
            match exit {
                MenuExit::Start => {}
                MenuExit::Back => continue 'title,
                MenuExit::Quit => return Ok(Exit::Quit),
                MenuExit::Custom(()) => {}
            }

            let games = mode.games()?;
            let settings = mode.settings(themes);
            let key = settings.high_score_key.clone();
            let controllers = mode.controllers();
            let mode_ref: &M = mode;
            let next_stage = |player: u32, completed: u32| {
                mode_ref.next_stage(themes, player, completed)
            };
            match app.run_match(
                &themes.all,
                games,
                settings,
                fg_particles,
                bg_particles,
                next_stage,
                controllers,
            )? {
                PostGameAction::NewHighScore(high_score) => {
                    app.new_high_score(&key, high_score, bg_particles)?;
                    continue 'menu;
                }
                PostGameAction::ReturnToMenu => continue 'menu,
                PostGameAction::Quit => return Ok(Exit::Quit),
            }
        }
    }
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("ga") {
        use rustris::game::ai::genetic;
        return match args.get(1).map(String::as_str) {
            None | Some("auto") => genetic::ga_main_auto(),
            Some("survival") => genetic::ga_main_survival(),
            Some("score") => genetic::ga_main_score(),
            Some("diagnose") => genetic::ga_diagnose(),
            Some(other) => Err(format!(
                "unknown ga mode '{}', expected: auto, survival, score or diagnose",
                other
            )),
        };
    }

    engine::app_info::init(engine::app_info::AppInfo {
        name: build_info::PKG_NAME,
        version: build_info::PKG_VERSION,
        authors: build_info::PKG_AUTHORS,
    });

    let mut app = App::new(MAX_PLAYERS, include_bytes!("../icon.png"))?;
    let texture_creator = app.canvas().texture_creator();
    let config = app.config();
    let mut all = dr_rustario::theme::all_themes(app.canvas(), &texture_creator, config)?;
    let dr_range = 0..all.len();
    all.extend(rustris::theme::all_themes(app.canvas(), &texture_creator, config)?);
    let themes = Themes {
        rustris: dr_range.end..all.len(),
        dr_rustario: dr_range,
        all,
    };

    let mut fg_particles = app.particle_render(
        &texture_creator,
        MAX_PARTICLES_PER_PLAYER * MAX_PLAYERS as usize,
        vec![],
    )?;
    let mut bg_particles = app.particle_render(
        &texture_creator,
        MAX_BACKGROUND_PARTICLES,
        themes.all.iter().collect(),
    )?;

    let mut rustris = RustrisMode::new();
    let mut dr_rustario = DrRustarioMode::new();
    let mut versus = VersusMode::new();

    loop {
        app.set_menu_sound(MenuSounds::MODERN)?;
        let items = ModeChoice::ALL
            .iter()
            .map(|m| MenuItem::select(m.name()))
            .chain([MenuItem::select(HIGH_SCORES), MenuItem::select(QUIT)])
            .collect();
        let exit = app.run_menu(
            items,
            "Dr. Rustario vs. Rustris".to_string(),
            None,
            MenuMusic::Title,
            &mut bg_particles,
            &themes.race_all(),
            |name, _| {
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
            },
        )?;
        let choice = match exit {
            MenuExit::Custom(PreMenuAction::Play(mode)) => mode,
            MenuExit::Custom(PreMenuAction::ViewHighScores) => {
                // every table of every game and mode, at its defaults until played
                let keys = [
                    rustris.all_high_score_keys(),
                    dr_rustario.all_high_score_keys(),
                    versus.all_high_score_keys(),
                ]
                .concat();
                app.view_high_scores(&keys, &mut bg_particles)?;
                continue;
            }
            MenuExit::Start => continue,
            MenuExit::Back | MenuExit::Quit => break,
        };

        let exit = match choice {
            ModeChoice::Rustris => {
                run_mode(&mut app, &mut rustris, &themes, &mut fg_particles, &mut bg_particles)?
            }
            ModeChoice::DrRustario => run_mode(
                &mut app,
                &mut dr_rustario,
                &themes,
                &mut fg_particles,
                &mut bg_particles,
            )?,
            ModeChoice::Versus => {
                run_mode(&mut app, &mut versus, &themes, &mut fg_particles, &mut bg_particles)?
            }
        };
        if let Exit::Quit = exit {
            break;
        }
    }

    Ok(())
}
