//! Walks Mean Bean Machine's whole cast through all four of their states and writes a contact
//! sheet, without opening a visible window.
//!
//! These are *animations* in a box eighty pixels wide, and nothing but looking at them will
//! catch a face holding its action pose instead of its rest one, a strip cut in the wrong
//! order, or an overlay landing off a nose. `animation_shot` is the sibling for the board's
//! moving parts; this one is for the faces.
//!
//! Two players, so the **mirroring** is in the picture too: a character faces the other
//! player's board, which means the one on the left of the window is drawn flipped.
//!
//! ```text
//! cargo run -p dr-rustario-vs-rustris --example character_shot -- [out] [scale]
//! ```

use engine::animate::character::MIN_DWELL;
use engine::app_info::{init, AppInfo};
use engine::config::{Config, VideoConfig, VideoMode};
use engine::game::Game;
use engine::render::context::{PlayerTextures, PlayerThemes, TextureMode, ThemeContext};
use engine::render::Theme;
use puyo_rusto::game::rules::Difficulty;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use std::time::Duration;

const TICK: Duration = Duration::from_micros(16_667);
const PLAYERS: u32 = 2;
/// the MUGSHOT box on the genesis panel, in that theme's own source pixels
const MUGSHOT: (i32, i32, u32, u32) = (120, 96, 80, 56);

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| ".".to_string());
    let scale: u32 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(3);

    init(AppInfo {
        name: "character-shot",
        version: "0",
        authors: "",
    });

    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let (width, height) = (960u32, 720u32);
    let window = video
        .window("character-shot", width, height)
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
        .position(|t| t.name() == "genesis")
        .ok_or("no genesis theme")?;

    let mut themes = ThemeContext::new(
        all,
        texture_creator,
        (0..PLAYERS)
            .map(|_| PlayerThemes::new(0..all.len(), index))
            .collect(),
        (width, height),
        config.video,
    )?;
    let mut games = (0..PLAYERS).map(|_| quiet_board()).collect::<Vec<_>>();
    for game in games.iter_mut() {
        Game::drain_events(game);
    }
    let mut textures = (0..PLAYERS)
        .map(|_| {
            PlayerTextures::new(
                texture_creator,
                themes.max_background_size(),
                themes.max_board_size(),
            )
            .unwrap()
        })
        .collect::<Vec<PlayerTextures>>();

    let cast = themes.character_count(0);
    assert!(cast > 0, "the genesis theme has no cast");
    // the four states, and how to get the character into each one
    let states = ["idle", "winning", "losing", "defeat"];
    let cell = (MUGSHOT.2 * scale, MUGSHOT.3 * scale);
    let mut sheet = image::RgbaImage::new(cell.0 * states.len() as u32, cell.1 * cast as u32);

    for character in 0..cast {
        // player 0 is on the left of the window and so is drawn mirrored; player 1 is native.
        // Both are dealt the same face here so one shot shows both orientations.
        themes.deal_character(0, character, true);
        themes.deal_character(1, character, false);
        for (column, state) in states.iter().enumerate() {
            // hold long enough that MIN_DWELL never refuses the change, then settle on a
            // frame partway through the row rather than always on its first
            for player in 0..PLAYERS {
                match *state {
                    "winning" => themes.animate_character_chain(player),
                    "losing" => themes.character_danger(player, 0.95, true),
                    "defeat" => themes.animate_game_over(player),
                    _ => {}
                }
            }
            let mut held = Duration::ZERO;
            while held < MIN_DWELL * 2 {
                themes.update_animations(TICK);
                held += TICK;
            }
            draw(&mut canvas, &mut themes, &games, &mut textures)?;
            let pixels = canvas.read_pixels(None, PixelFormatEnum::ABGR8888)?;
            let frame =
                image::RgbaImage::from_raw(width, height, pixels).ok_or("bad pixel buffer")?;
            // crop player 1's mugshot box out of the window, unmirrored, at `scale`
            let box_rect =
                themes.player_source_rect(1, Rect::new(MUGSHOT.0, MUGSHOT.1, MUGSHOT.2, MUGSHOT.3));
            let crop = image::imageops::resize(
                &image::imageops::crop_imm(
                    &frame,
                    box_rect.x().max(0) as u32,
                    box_rect.y().max(0) as u32,
                    box_rect.width(),
                    box_rect.height(),
                )
                .to_image(),
                cell.0,
                cell.1,
                image::imageops::FilterType::Nearest,
            );
            image::imageops::overlay(
                &mut sheet,
                &crop,
                (column as u32 * cell.0) as i64,
                (character as u32 * cell.1) as i64,
            );
        }
        // and one whole-window shot per character, which is where placement and mirroring show
        let path = format!("{out}/character-{character:02}.png");
        let pixels = canvas.read_pixels(None, PixelFormatEnum::ABGR8888)?;
        image::RgbaImage::from_raw(width, height, pixels)
            .ok_or("bad pixel buffer")?
            .save(&path)
            .map_err(|e| e.to_string())?;
        println!(
            "  {:2} {:<18} -> {path}",
            character,
            themes.character_name(1).unwrap_or("?")
        );
        // reset the boards, since a game over leaves them finished
        games = (0..PLAYERS).map(|_| quiet_board()).collect();
        for game in games.iter_mut() {
            Game::drain_events(game);
        }
    }
    let path = format!("{out}/cast.png");
    sheet.save(&path).map_err(|e| e.to_string())?;
    println!("\n{cast} characters x {} states -> {path}", states.len());
    Ok(())
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

/// an empty board that is not going anywhere, so the shot is of the face and nothing else
fn quiet_board() -> puyo_rusto::game::Game {
    use puyo_rusto::game::{cell::PuyoSkin, random};
    let difficulty = Difficulty::VeryEasy;
    let seed = random::Seed::from_u64(7);
    let gen = random::from_seed(seed, 1, difficulty.colors())
        .pop()
        .unwrap();
    puyo_rusto::game::Game::new(difficulty, 2, gen, PuyoSkin::deal(seed, 1)[0])
}
