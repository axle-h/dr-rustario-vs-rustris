//! Draws every input of the Dr. Rustario network: the bottle before the pill, and the bottles
//! the placements that score highest and lowest on that input leave behind. The positions are
//! [`dr_rustario::game::ai::explain::scenarios`]'s, the same ones `ga dr explain` reports.
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

/// readable per bottle, small enough that the whole set stays under a megabyte
const WIDTH: u32 = 640;
const HEIGHT: u32 = 720;

/// How far into the destroy animation a shot is taken: halfway through
/// [`engine::animate::destroy`]'s strip, since past its end the cells draw as if untouched.
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
    // leaked, as `Shell::new` leaks its own, since every `Theme` borrows it
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

    // the manifest is written from the same loop as the shots, so labels cannot drift from
    // their pictures
    let mut manifest = vec![];
    let mut count = 0;
    for scenario in scenarios() {
        let name = INPUTS[scenario.input].name.replace('.', "-");
        // the bottle, then each placement as it lands (clears popping) and after it settles
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

/// Draw one bottle and save the board out of it, cropped from the whole scene so any backdrop a
/// theme draws behind the stack is kept.
#[allow(clippy::too_many_arguments)]
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

    // no pill and nothing running, so the picture is just the stack
    let random = dr_rustario::game::random::random(1, dr_rustario::game::random::RandomMode::Bag)
        .pop()
        .expect("no random");
    let game = Game::from_bottle(0, GameSpeed::Medium, random, bottle.clone());

    // wound on past the destroy animation's first instant, which draws the group unchanged
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

    // annotations: a solid ring on the pill's halves and a dashed one on every cleared cell,
    // since the NES pops a vitamin with nearly its resting sprite
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

/// A solid ring, white either side of black, so it reads against any colour.
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

/// A dashed white ring, for a cell the clear is taking.
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
