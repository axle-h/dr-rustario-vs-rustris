//! renders each game's main menu straight to a PNG, without opening a visible window, as
//! the theme selection is walked: the mode list has to lose the theme sprint as soon as a
//! single theme is picked, and get it back with `all`. One menu per game and player count
//! is kept for the walk, refreshed by `Menu::set_items` exactly as the shell does.
//!
//! `cargo run -p dr-rustario-vs-rustris --example menu_shot -- 960 720 out/`

use dr_rustario::options::Options as DrOptions;
use engine::app_info::{init, AppInfo};
use engine::menu::{Menu, MenuItem};
use puyo_rusto::options::Options as PuyoOptions;
use rustris::options::Options as RustrisOptions;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::WindowCanvas;

/// As much of a game's options as this example drives. The games' `Options` are separate
/// types with no trait between them, so the walk gets one here: a game joining the compendium
/// is an impl and an entry in [`all_games`], not another hand-written pair of menus.
trait MenuOptions {
    fn set_players(&mut self, players: u32);
    fn select(&mut self, name: &str, value: &str);
    fn items(&self) -> Vec<MenuItem>;
    /// the names of this game's own themes, `all` first - the games do not name their themes
    /// alike, so the walk asks each one what it has rather than assuming
    fn theme_names(&self) -> Vec<&'static str>;
}

impl MenuOptions for RustrisOptions {
    fn set_players(&mut self, players: u32) {
        RustrisOptions::set_players(self, players);
    }
    fn select(&mut self, name: &str, value: &str) {
        RustrisOptions::select(self, name, value);
    }
    fn items(&self) -> Vec<MenuItem> {
        self.menu_items(false)
    }
    fn theme_names(&self) -> Vec<&'static str> {
        rustris::game::rules::MatchThemes::names()
    }
}

impl MenuOptions for DrOptions {
    fn set_players(&mut self, players: u32) {
        DrOptions::set_players(self, players);
    }
    fn select(&mut self, name: &str, value: &str) {
        DrOptions::select(self, name, value);
    }
    fn items(&self) -> Vec<MenuItem> {
        self.menu_items(false)
    }
    fn theme_names(&self) -> Vec<&'static str> {
        dr_rustario::game::rules::MatchThemes::names()
    }
}

impl MenuOptions for PuyoOptions {
    fn set_players(&mut self, players: u32) {
        PuyoOptions::set_players(self, players);
    }
    fn select(&mut self, name: &str, value: &str) {
        PuyoOptions::select(self, name, value);
    }
    fn items(&self) -> Vec<MenuItem> {
        self.menu_items(false)
    }
    fn theme_names(&self) -> Vec<&'static str> {
        puyo_rusto::game::rules::MatchThemes::names()
    }
}

/// every game whose menus are walked: the file name it is shot under, its title, and its
/// options
fn all_games() -> Vec<(&'static str, &'static str, Box<dyn MenuOptions>)> {
    vec![
        (
            "rustris",
            "Rustris",
            Box::new(RustrisOptions::default()) as Box<dyn MenuOptions>,
        ),
        (
            "dr-rustario",
            "Dr. Rustario",
            Box::new(DrOptions::default()) as Box<dyn MenuOptions>,
        ),
        (
            "puyo",
            "Puyo Rusto",
            Box::new(PuyoOptions::default()) as Box<dyn MenuOptions>,
        ),
    ]
}

/// the theme picks to walk, as *steps*: `all`, then a single theme of the game's own, then
/// `all` again - which is what takes the theme sprint off the mode list and puts it back
const THEME_STEPS: usize = 3;

/// the theme this game is asked for at each step
fn theme_step(names: &[&'static str], step: usize) -> &'static str {
    // "all" is always the first of them; a single theme is anything after it, and a game with
    // only one theme has nothing to switch to yet
    match step {
        1 => names.get(1).copied().unwrap_or(names[0]),
        _ => names[0],
    }
}

/// how far to walk the mode row: one more than the longest list, so the wrap shows
const MODE_SHOTS: usize = 5;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let width: u32 = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(960);
    let height: u32 = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(720);
    let out = args.get(3).cloned().unwrap_or_else(|| ".".to_string());

    init(AppInfo {
        name: "menu-shot",
        version: "0",
        authors: "",
    });

    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let window = video
        .window("menu-shot", width, height)
        .position_centered()
        .hidden()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    for players in [1, 2] {
        let mut games = all_games();
        let mut menus = vec![];
        for (name, title, options) in games.iter_mut() {
            options.set_players(players);
            menus.push((
                *name,
                Menu::new(
                    options.items(),
                    &mut canvas,
                    &texture_creator,
                    title.to_string(),
                    format!("{players} player"),
                )?,
            ));
        }

        for step in 0..THEME_STEPS {
            for ((game, menu), (_, _, options)) in menus.iter_mut().zip(games.iter_mut()) {
                let theme = theme_step(&options.theme_names(), step);
                options.select("themes", theme);
                menu.set_items(&mut canvas, &options.items())?;
                // park on the mode row and walk it, so every mode on offer is in a shot
                menu.down();
                for mode in 0..MODE_SHOTS {
                    shoot(
                        menu,
                        &mut canvas,
                        (width, height),
                        &format!("{out}/menu-{game}-{players}p-{step}{theme}-mode{mode}.png"),
                    )?;
                    menu.right();
                }
                menu.up();
            }
        }
    }
    Ok(())
}

fn shoot(
    menu: &mut Menu,
    canvas: &mut WindowCanvas,
    (width, height): (u32, u32),
    path: &str,
) -> Result<(), String> {
    canvas.set_draw_color(Color::BLACK);
    canvas.clear();
    menu.draw(canvas)?;
    let pixels = canvas.read_pixels(None, PixelFormatEnum::ABGR8888)?;
    image::RgbaImage::from_raw(width, height, pixels)
        .ok_or("bad pixel buffer")?
        .save(path)
        .map_err(|e| e.to_string())?;
    println!("{path}");
    Ok(())
}
