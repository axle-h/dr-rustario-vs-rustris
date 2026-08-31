//! Draws every input of the Dr. Rustario network: the bottle before the pill, and the bottles
//! the placements that score highest and lowest on that input leave behind.
//!
//! The positions are [`dr_rustario::game::ai::explain::scenarios`]'s, so what is drawn and what
//! `ga dr explain` reports are the same thing - the picture cannot drift from the number under
//! it. Only the drawing is here, because it needs a theme and a window and the crate that owns
//! the features has neither.
//!
//! `cargo run -p dr-rustario-vs-rustris --example feature_shots -- out/ [theme]`

use dr_rustario::game::ai::explain::{scenarios, INPUTS};
use dr_rustario::game::bottle::Bottle;
use dr_rustario::game::{Game, GameSpeed};
use engine::app_info::{init, AppInfo};
use engine::config::{Config, VideoConfig, VideoMode};
use engine::render::context::{PlayerTextures, PlayerThemes, TextureMode, ThemeContext};
use engine::render::Theme;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::time::Duration;

/// big enough that one bottle is drawn at a readable size and small enough that a hundred and
/// sixty of them are under a megabyte
const WIDTH: u32 = 640;
const HEIGHT: u32 = 720;

/// How far into the destroy animation a shot is taken. The strip runs for
/// [`engine::animate::destroy`]'s three hundred milliseconds and the animation is *over* after
/// that - a shot taken past the end draws the cells as if nothing had happened, which is how
/// the first pass of this went out with no pops in it at all. Halfway is safely inside it.
const POP_FRAME: Duration = Duration::from_millis(150);

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| ".".to_string());
    let wanted = args.get(2).cloned().unwrap_or_else(|| "nes".to_string());

    init(AppInfo {
        name: "feature-shots",
        version: "0",
        authors: "",
    });

    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let window = video
        .window("feature-shots", WIDTH, HEIGHT)
        .position_centered()
        .hidden()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    // leaked for the life of the process, as `Shell::new` leaks its own: a `Theme` borrows the
    // texture creator, and everything holding a theme has to outlive it
    let texture_creator: &'static TextureCreator<WindowContext> =
        Box::leak(Box::new(canvas.texture_creator()));

    let mut config = Config::default();
    config.video = VideoConfig {
        mode: VideoMode::Window {
            width: WIDTH,
            height: HEIGHT,
        },
        vsync: false,
        disable_screensaver: false,
        ..config.video
    };
    config.audio.music_volume = 0.0;
    config.audio.effects_volume = 0.0;

    let themes: &'static [Theme<'static>] = Box::leak(
        dr_rustario::theme::all_themes(&mut canvas, texture_creator, config)?.into_boxed_slice(),
    );
    let index = themes
        .iter()
        .position(|theme| theme.name() == wanted)
        .ok_or_else(|| {
            format!(
                "no theme called '{}', expected one of {:?}",
                wanted,
                themes.iter().map(Theme::name).collect::<Vec<&str>>()
            )
        })?;

    // Everything a document needs about a scenario, written out beside the pictures.
    //
    // The numbers and the pictures have to come from *one* producer or they drift: the first
    // pass of this had the values read out of `ga dr explain` and the shots rendered here, and
    // a change to which end is drawn first swapped every label against its own picture without
    // anything failing. Both come from this loop now.
    let mut manifest = vec![];
    let mut count = 0;
    for scenario in scenarios() {
        let name = INPUTS[scenario.input].name.replace('.', "-");
        // the bottle, then each placement twice: the instant it lands - with whatever it
        // clears shown going, in the theme's own pop frames - and what is left afterwards
        let shots = [
            ("before", &scenario.before, None, [].as_slice()),
            (
                "a-landed",
                &scenario.landed[0],
                Some(scenario.placed[0]),
                scenario.destroyed[0].as_slice(),
            ),
            ("a-after", &scenario.after[0], None, [].as_slice()),
            (
                "b-landed",
                &scenario.landed[1],
                Some(scenario.placed[1]),
                scenario.destroyed[1].as_slice(),
            ),
            ("b-after", &scenario.after[1], None, [].as_slice()),
        ];
        for (what, bottle, placed, destroyed) in shots {
            let path = format!("{out}/{:02}-{name}-{what}.png", scenario.input);
            shoot(
                &mut canvas,
                texture_creator,
                themes,
                index,
                config,
                bottle,
                placed,
                destroyed,
                &path,
            )?;
            count += 1;
        }
        let viruses_before = scenario.before.virus_count();
        let popped = |at: usize| {
            scenario.destroyed[at]
                .iter()
                .filter(|point| {
                    scenario.landed[at]
                        .block_at(point.x() as u32, point.y() as u32)
                        .is_virus()
                })
                .count()
        };
        manifest.push(format!(
            r#"  {{"input": {}, "name": "{}", "value": [{}, {}],
   "cells_popped": [{}, {}], "viruses_popped": [{}, {}], "rounds": [{}, {}],
   "found": {}, "viruses_before": {}, "viruses_after": [{}, {}]}}"#,
            scenario.input,
            INPUTS[scenario.input].name,
            scenario.value[0],
            scenario.value[1],
            scenario.destroyed[0].len(),
            scenario.destroyed[1].len(),
            popped(0),
            popped(1),
            scenario.rounds[0],
            scenario.rounds[1],
            scenario.found,
            viruses_before,
            scenario.after[0].virus_count(),
            scenario.after[1].virus_count(),
        ));

        println!(
            "{:<26} {:>8.1} ({} popping) vs {:>8.1} ({} popping){}",
            INPUTS[scenario.input].name,
            scenario.value[0],
            scenario.destroyed[0].len(),
            scenario.value[1],
            scenario.destroyed[1].len(),
            if scenario.separates() {
                ""
            } else {
                "   <- the same value twice"
            }
        );
    }
    let path = format!("{out}/manifest.json");
    std::fs::write(&path, format!("[\n{}\n]\n", manifest.join(",\n")))
        .map_err(|e| e.to_string())?;
    println!("\n{count} shots and {path}");
    Ok(())
}

/// Draw one bottle and save the board out of it.
///
/// The whole scene is drawn and then cropped to the board, rather than the board texture being
/// saved on its own, because a theme's board is not the whole of what it draws a bottle with -
/// the NES one puts the stack on its own backdrop - and cropping is the only way to get what
/// the player actually sees.
fn shoot(
    canvas: &mut WindowCanvas,
    texture_creator: &'static TextureCreator<WindowContext>,
    themes: &'static [Theme<'static>],
    index: usize,
    config: Config,
    bottle: &Bottle,
    placed: Option<[engine::game::geometry::Point; 2]>,
    destroyed: &[engine::game::geometry::Point],
    path: &str,
) -> Result<(), String> {
    let mut context = ThemeContext::new(
        themes,
        texture_creator,
        vec![PlayerThemes::new(0..themes.len(), index)],
        (WIDTH, HEIGHT),
        config.video,
    )?;

    // a bottle with no pill in it and nothing running: the picture is the stack, and anything
    // moving over it would only be in the way
    let random = dr_rustario::game::random::random(1, dr_rustario::game::random::RandomMode::Bag)
        .pop()
        .expect("no random");
    let game = Game::from_bottle(0, GameSpeed::Medium, random, bottle.clone());

    // Whatever this placement clears, shown *going*, in the theme's own destroy frames rather
    // than as a gap where it used to be. The animation is queued and then wound on a little way,
    // because its first instant is the group drawn exactly as it sits on the board - which is
    // the tell before the pop and looks like nothing at all in a still.
    if !destroyed.is_empty() {
        let cells: Vec<engine::game::PlacedCell> = destroyed
            .iter()
            .filter_map(|point| Some((*point, engine::game::Game::cell(&game, *point).id()?)))
            .collect();
        context.animate_destroy(0, &cells);
        context.update_animations(POP_FRAME);
    }

    let mut textures = PlayerTextures::new(
        texture_creator,
        context.max_background_size(),
        context.max_board_size(),
    )?;
    let mut texture_refs = vec![
        (&mut textures.board, TextureMode::Board(0)),
        (&mut textures.background, TextureMode::Background(0)),
    ];

    canvas.set_draw_color(Color::BLACK);
    canvas.clear();
    context.draw_scene(canvas, &[&game])?;
    canvas
        .with_multiple_texture_canvas(texture_refs.iter(), |target, mode| {
            let animations = context.player_animations(0);
            match mode {
                TextureMode::Background(_) => context
                    .theme(0)
                    .draw_background(target, &game, animations)
                    .unwrap(),
                TextureMode::Board(_) => context
                    .theme(0)
                    .draw_board(target, &game, animations)
                    .unwrap(),
            }
        })
        .map_err(|e| e.to_string())?;
    context.draw_players(canvas, &mut texture_refs, Duration::ZERO)?;

    // Two annotations, and they are annotations rather than anything the game draws: a solid
    // ring where the pill's halves came to rest, and a dashed one round every cell the clear is
    // taking. The pop sprites say what is happening on their own for a virus, whose face goes -
    // but the NES draws a popping *vitamin* with very nearly the sprite it already had, so
    // without the dashes half of a clear is invisible in a still.
    let board = context.player_board_snip(0);
    let cell = (
        board.width() / dr_rustario::game::bottle::BOTTLE_WIDTH,
        board.height() / dr_rustario::game::bottle::BOTTLE_HEIGHT,
    );
    let at = |point: engine::game::geometry::Point| {
        Rect::new(
            board.x() + point.x() * cell.0 as i32,
            board.y() + point.y() * cell.1 as i32,
            cell.0,
            cell.1,
        )
    };
    for point in destroyed {
        if point.x() >= 0 && point.y() >= 0 {
            dashed(canvas, at(*point))?;
        }
    }
    for point in placed.into_iter().flatten() {
        if point.x() >= 0 && point.y() >= 0 {
            ring(canvas, at(point))?;
        }
    }

    let crop = Rect::new(
        board.x(),
        board.y(),
        board.width().min(WIDTH),
        board.height().min(HEIGHT),
    );
    let pixels = canvas.read_pixels(Some(crop), PixelFormatEnum::ABGR8888)?;
    image::RgbaImage::from_raw(crop.width(), crop.height(), pixels)
        .ok_or("bad pixel buffer")?
        .save(path)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// A solid ring: white outside and in, black between, so it reads against a virus of any colour
/// and against the bare backdrop alike.
fn ring(canvas: &mut WindowCanvas, at: Rect) -> Result<(), String> {
    for (inset, colour) in [(0, Color::WHITE), (1, Color::BLACK), (2, Color::WHITE)] {
        canvas.set_draw_color(colour);
        canvas.draw_rect(Rect::new(
            at.x() + inset,
            at.y() + inset,
            at.width() - inset as u32 * 2,
            at.height() - inset as u32 * 2,
        ))?;
    }
    Ok(())
}

/// A dashed ring, for a cell the clear is taking. Dashes rather than another colour because a
/// coloured outline disappears against a cell of that colour, and these sit on every colour.
fn dashed(canvas: &mut WindowCanvas, at: Rect) -> Result<(), String> {
    const DASH: i32 = 5;
    let (x, y, w, h) = (
        at.x() + 1,
        at.y() + 1,
        at.width() as i32 - 3,
        at.height() as i32 - 3,
    );
    canvas.set_draw_color(Color::WHITE);
    let mut step = 0;
    let mut plot = |px: i32, py: i32, canvas: &mut WindowCanvas| -> Result<(), String> {
        if (step / DASH) % 2 == 0 {
            canvas.fill_rect(Rect::new(px, py, 2, 2))?;
        }
        step += 1;
        Ok(())
    };
    for i in 0..w {
        plot(x + i, y, canvas)?;
    }
    for i in 0..h {
        plot(x + w, y + i, canvas)?;
    }
    for i in 0..w {
        plot(x + w - i, y + h, canvas)?;
    }
    for i in 0..h {
        plot(x, y + h - i, canvas)?;
    }
    Ok(())
}
