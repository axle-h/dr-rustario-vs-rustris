//! Plays every one of Kirby's routines through the real `snes` theme and writes one png a
//! frame, so a capture can be put next to the recording it was measured off.
//!
//! This is the temporary harness the reimplementation was checked with, and it is deliberately
//! dumb: it drives `ThemeContext` exactly as a match does, steps it at 60 Hz, and reads the
//! arch out of the window every *other* tick - because the source capture is 30 fps and a
//! comparison at two different rates is not a comparison.
//!
//! `character_shot` is the sibling for Mean Bean Machine's faces. That one wants a contact
//! sheet, since a mugshot is one pose a state; this one wants every frame, since a routine is
//! a thing that moves.
//!
//! ```text
//! cargo run -p dr-rustario-vs-rustris --example kirby_shot -- <out> [scale]
//! cargo run -p dr-rustario-vs-rustris --example kirby_shot -- <out> [scale] loop [seconds]
//! ```

use engine::app_info::{init, AppInfo};
use engine::config::{Config, VideoConfig, VideoMode};
use engine::game::Game;
use engine::render::context::{PlayerTextures, PlayerThemes, TextureMode, ThemeContext};
use engine::render::Theme;
use puyo_rusto::game::rules::Difficulty;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use std::time::Duration;

const TICK: Duration = Duration::from_nanos(16_666_667);
/// What the capture is cropped to, in the theme's own source pixels.
///
/// The **whole centre column**, not the arch (`snes::kirby::BOX` is (104, 157) 48x35). Kirby
/// is a sprite: `takeoff` carries him seventy two pixels above the box, which is twice the
/// arch's own height, and the game draws him over every course of stone on the way. A crop
/// that stops at the arch shows a routine walking out of frame, and this is meant to show what
/// every frame of one does. The top is as high as the recording itself goes.
const ARCH: (i32, i32, u32, u32) = (104, 84, 48, 124);

/// The fifteen, in the order `kirby.rs` lists them: the row each is dealt on, which of that
/// row's routines it is, and **where the recording had him standing** when it played that one.
///
/// The last of those is only for the comparison. A routine's places are relative to whatever
/// the one before it left, so in a match he stands wherever he has walked to; here he is put
/// back where the capture caught him, or the two panes do not line up and the eye reads a
/// displacement that is not a difference.
/// The pool, in the order `kirby.rs` lists it: which of `CHOICES` it is, and **where the
/// recording had him standing** when it played that one.
///
/// The last of those is only for the comparison. A routine's places are relative to whatever
/// the one before it left, so in a match he stands wherever he has walked to; here he is put
/// back where the capture caught him, or the two panes do not line up and the eye reads a
/// displacement that is not a difference.
///
/// The blink is the *filler* - played between routines rather than as one of them - and is
/// asked for here by `usize::MAX` so it can still be put next to the recording.
const ROUTINES: &[(&str, usize, i32)] = &[
    // the filler, which is not in the pool - `usize::MAX` asks for it by name
    ("blink", usize::MAX, 3),
    ("yawn", 0, 3),
    ("walk", 1, 2),
    ("flop", 2, 9),
    ("ricochet", 3, 9),
    ("climb", 4, 6),
    ("float-up", 5, 30),
    ("spring", 6, 32),
    ("takeoff", 7, 32),
    ("jump-right", 8, 6),
    ("jump-left", 9, 23),
    ("spin-right", 10, 9),
    ("spin-left", 11, 30),
    ("tumble-right", 12, 12),
    ("tumble-left", 13, 23),
];

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| ".".to_string());
    let scale: u32 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(4);
    // `loop <seconds>` runs it as a match does instead of walking the routines one by one
    let seconds: Option<u32> = (args.get(3).map(String::as_str) == Some("loop"))
        .then(|| args.get(4).and_then(|a| a.parse().ok()).unwrap_or(120));

    init(AppInfo {
        name: "kirby-shot",
        version: "0",
        authors: "",
    });

    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    // one player, so the panel is drawn as big as it goes and the arch crop is not tiny
    let (width, height) = (1280u32, 720u32);
    let window = video
        .window("kirby-shot", width, height)
        .position_centered()
        .hidden()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = Box::leak(Box::new(canvas.texture_creator()));

    let mut config = Config::default();
    config.video = VideoConfig {
        mode: VideoMode::Window { width, height },
        vsync: false,
        disable_screensaver: false,
        ..config.video
    };
    config.audio.music_volume = 0.0;
    config.audio.effects_volume = 0.0;

    let all: &'static [Theme<'static>] = Box::leak(
        puyo_rusto::theme::all_themes(&mut canvas, texture_creator, config)?.into_boxed_slice(),
    );
    let index = all
        .iter()
        .position(|t| t.name() == "snes")
        .ok_or("no snes theme")?;

    let mut themes = ThemeContext::new(
        all,
        texture_creator,
        vec![PlayerThemes::new(0..all.len(), index)],
        (width, height),
        config.video,
    )?;
    let mut games = vec![quiet_board()];
    Game::drain_events(&mut games[0]);
    let mut textures = vec![PlayerTextures::new(
        texture_creator,
        themes.max_background_size(),
        themes.max_board_size(),
    )
    .unwrap()];

    assert!(themes.character_count(0) > 0, "the snes theme has no cast");
    themes.deal_character(0, 0, false);

    let crop = themes.player_source_rect(0, Rect::new(ARCH.0, ARCH.1, ARCH.2, ARCH.3));
    let cell = (ARCH.2 * scale, ARCH.3 * scale);
    println!(
        "arch {}x{} in the window -> {}x{}",
        crop.width(),
        crop.height(),
        cell.0,
        cell.1
    );

    // ... and one run of the thing as it actually plays: routines dealt, paces varied, one
    // flowing into the next from wherever the last left him. This is the check that the parts
    // add up - a routine that does not chain shows here and nowhere else.
    if let Some(seconds) = seconds {
        let dir = format!("{out}/loop");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        themes.play_character_routine(0, 0, None);
        let mut shot = 0usize;
        for tick in 0..(60 * seconds) {
            // The board here is empty and would leave him idling on the floor for the whole
            // clip, which shows the dealing and none of what drives it. So the match's own
            // intensity is *swept*: a slow ramp from nothing up to a buried board and back,
            // with a chain thrown in every fifteen seconds. Everything else - which routine,
            // which way round, how fast, whether he blinks - is the real thing deciding.
            let through = tick as f64 / (60.0 * seconds as f64);
            themes.character_danger(0, 1.0 - (through * 2.0 - 1.0).abs(), false);
            if tick % (60 * 15) == 60 * 7 {
                themes.animate_character_chain(0);
            }
            if tick % 2 == 0 {
                shoot(
                    &mut canvas,
                    &mut themes,
                    &games,
                    &mut textures,
                    crop,
                    cell,
                    &format!("{dir}/{shot:04}.png"),
                    width,
                    height,
                )?;
                shot += 1;
            }
            themes.update_animations(TICK);
        }
        println!("\n{shot} frames of it running loose -> {dir}");
        return Ok(());
    }

    for (name, routine, home) in ROUTINES {
        let dir = format!("{out}/{name}");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        themes.play_character_routine(0, *routine, Some(*home));
        let mut shot = 0usize;
        let mut tick = 0usize;
        // every routine, then the stand it rests on, and stop the moment it has run out -
        // which is what `character_resting` is for. The cap is a routine that never ends.
        while tick < 60 * 10 {
            if tick % 2 == 0 {
                shoot(
                    &mut canvas,
                    &mut themes,
                    &games,
                    &mut textures,
                    crop,
                    cell,
                    &format!("{dir}/{shot:04}.png"),
                    width,
                    height,
                )?;
                shot += 1;
            }
            themes.update_animations(TICK);
            tick += 1;
            // a dozen frames of the stand it settles on, and then out - the rest between
            // routines is two seconds and none of that is the routine
            if themes.character_resting(0) && tick % 2 == 0 && shot > 4 {
                for _ in 0..6 {
                    shoot(
                        &mut canvas,
                        &mut themes,
                        &games,
                        &mut textures,
                        crop,
                        cell,
                        &format!("{dir}/{shot:04}.png"),
                        width,
                        height,
                    )?;
                    shot += 1;
                    themes.update_animations(TICK * 2);
                }
                break;
            }
        }
        println!("  {name:<13} {shot:3} frames -> {dir}");
    }
    // ... and one whole window, which is where the placement shows: Kirby standing on the
    // course the tray's boulders sit in, and clear of everything else in the column
    themes.play_character_routine(0, 0, None);
    draw(&mut canvas, &mut themes, &games, &mut textures)?;
    let pixels = canvas.read_pixels(None, PixelFormatEnum::ABGR8888)?;
    let path = format!("{out}/panel.png");
    image::RgbaImage::from_raw(width, height, pixels)
        .ok_or("bad pixels")?
        .save(&path)
        .map_err(|e| e.to_string())?;
    println!("\nthe whole window -> {path}");
    Ok(())
}

/// One frame: draw it, read the window back and save the crop.
#[allow(clippy::too_many_arguments)]
fn shoot(
    canvas: &mut sdl2::render::WindowCanvas,
    themes: &mut ThemeContext,
    games: &[puyo_rusto::game::Game],
    textures: &mut [PlayerTextures],
    crop: Rect,
    cell: (u32, u32),
    path: &str,
    width: u32,
    height: u32,
) -> Result<(), String> {
    draw(canvas, themes, games, textures)?;
    let pixels = canvas.read_pixels(None, PixelFormatEnum::ABGR8888)?;
    let frame = image::RgbaImage::from_raw(width, height, pixels).ok_or("bad pixels")?;
    image::imageops::resize(
        &image::imageops::crop_imm(
            &frame,
            crop.x().max(0) as u32,
            crop.y().max(0) as u32,
            crop.width(),
            crop.height(),
        )
        .to_image(),
        cell.0,
        cell.1,
        image::imageops::FilterType::Nearest,
    )
    .save(path)
    .map_err(|e| e.to_string())
}

fn draw(
    canvas: &mut sdl2::render::WindowCanvas,
    themes: &mut ThemeContext,
    games: &[puyo_rusto::game::Game],
    textures: &mut [PlayerTextures],
) -> Result<(), String> {
    let mut texture_refs = vec![];
    for (player, t) in textures.iter_mut().enumerate() {
        texture_refs.push((&mut t.board, TextureMode::Board(player as u32)));
        texture_refs.push((&mut t.background, TextureMode::Background(player as u32)));
    }
    canvas.set_draw_color(Color::BLACK);
    canvas.clear();
    let refs = games.iter().collect::<Vec<_>>();
    themes.draw_scene(canvas, &refs)?;
    canvas
        .with_multiple_texture_canvas(texture_refs.iter(), |target, mode| {
            let player = match mode {
                TextureMode::Background(p) | TextureMode::Board(p) => *p,
            };
            let animations = themes.player_animations(player);
            let game = &games[player as usize];
            match mode {
                TextureMode::Background(_) => themes
                    .theme(player)
                    .draw_background(target, game, animations)
                    .unwrap(),
                TextureMode::Board(_) => themes
                    .theme(player)
                    .draw_board(target, game, animations)
                    .unwrap(),
            }
        })
        .map_err(|e| e.to_string())?;
    themes.draw_players(canvas, &mut texture_refs, Duration::ZERO)?;
    themes.draw_debris(canvas)?;
    themes.draw_character_particles(canvas)?;
    themes.draw_attack_balls(canvas)
}

/// an empty board that is not going anywhere, so the shot is of Kirby and nothing else
fn quiet_board() -> puyo_rusto::game::Game {
    use puyo_rusto::game::{cell::PuyoSkin, random};
    let difficulty = Difficulty::VeryEasy;
    let seed = random::Seed::from_u64(7);
    let gen = random::from_seed(seed, 1, difficulty.colors())
        .pop()
        .unwrap();
    puyo_rusto::game::Game::new(difficulty, 2, gen, PuyoSkin::deal(seed, 1)[0])
}
