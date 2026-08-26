//! renders one frame of a match on every theme of a game, straight to a PNG, without opening
//! a visible window.
//!
//! `cargo run -p dr-rustario-vs-rustris --example frame_shot -- 640 480 1 out/ [game]`

use engine::app_info::{init, AppInfo};
use engine::config::{Config, VideoConfig, VideoMode};
use engine::game::Game;
use engine::render::context::{PlayerTextures, PlayerThemes, TextureMode, ThemeContext};
use engine::render::{GameRender, Theme};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::time::Duration;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let width: u32 = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(640);
    let height: u32 = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(480);
    let players: u32 = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(1);
    let out = args.get(4).cloned().unwrap_or_else(|| ".".to_string());
    let game = args
        .get(5)
        .cloned()
        .unwrap_or_else(|| "rustris".to_string());

    init(AppInfo {
        name: "frame-shot",
        version: "0",
        authors: "",
    });

    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let window = video
        .window("frame-shot", width, height)
        .position_centered()
        .hidden()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    let mut config = Config::default();
    config.video = VideoConfig {
        mode: VideoMode::Window { width, height },
        vsync: false,
        disable_screensaver: false,
        ..config.video
    };
    config.audio.music_volume = 0.0;
    config.audio.effects_volume = 0.0;

    match game.as_str() {
        "rustris" => {
            let themes = rustris::theme::all_themes(&mut canvas, &texture_creator, config)?;
            shoot(
                &mut canvas,
                &texture_creator,
                &themes,
                config,
                (width, height),
                players,
                &out,
                &game,
                rustris_game,
            )
        }
        "dr-rustario" => {
            let themes = dr_rustario::theme::all_themes(&mut canvas, &texture_creator, config)?;
            shoot(
                &mut canvas,
                &texture_creator,
                &themes,
                config,
                (width, height),
                players,
                &out,
                &game,
                dr_rustario_game,
            )
        }
        other => Err(format!(
            "unknown game '{other}', expected rustris or dr-rustario"
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn shoot<'a, G: Game + GameRender>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    all: &'a [Theme<'a>],
    config: Config,
    (width, height): (u32, u32),
    players: u32,
    out: &str,
    game_name: &str,
    new_game: fn() -> G,
) -> Result<(), String> {
    for (index, theme) in all.iter().enumerate() {
        let mut themes = ThemeContext::new(
            all,
            texture_creator,
            (0..players)
                .map(|_| PlayerThemes::new(0..all.len(), index))
                .collect(),
            (width, height),
            config.video,
        )?;
        let games = (0..players).map(|_| new_game()).collect::<Vec<G>>();
        println!(
            "{game_name:12} {:8} board={:?}",
            theme.name(),
            themes.player_board_snip(0)
        );

        let mut textures = (0..players)
            .map(|_| {
                PlayerTextures::new(
                    texture_creator,
                    themes.max_background_size(),
                    themes.max_board_size(),
                )
                .unwrap()
            })
            .collect::<Vec<PlayerTextures>>();
        let mut texture_refs = vec![];
        for (player, t) in textures.iter_mut().enumerate() {
            texture_refs.push((&mut t.board, TextureMode::Board(player as u32)));
            texture_refs.push((&mut t.background, TextureMode::Background(player as u32)));
        }

        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        let refs = games.iter().collect::<Vec<&G>>();
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

        let pixels = canvas.read_pixels(None, PixelFormatEnum::ABGR8888)?;
        let path = format!("{out}/{width}x{height}-{game_name}-{}.png", theme.name());
        image::RgbaImage::from_raw(width, height, pixels)
            .ok_or("bad pixel buffer")?
            .save(&path)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// a game part way through: a stack, a piece in flight and something on hold
fn rustris_game() -> rustris::game::Game {
    let random =
        rustris::game::random::random_tetrominos(rustris::game::random::RandomMode::Bag, 1)
            .pop()
            .unwrap();
    let mut game = rustris::game::Game::new(3, random);
    game.send_garbage(4);
    // hold is only allowed once a piece is falling, so keep nudging until it takes
    for _ in 0..40 {
        game.update(Duration::from_millis(50));
        if game.hold() {
            break;
        }
    }
    game.update(Duration::from_millis(600));
    for i in 0..7 {
        for _ in 0..(i % 4) {
            if i % 2 == 0 {
                game.left();
            } else {
                game.right();
            }
        }
        game.rotate(true);
        game.hard_drop();
        game.update(Duration::from_millis(200));
    }
    game.left();
    game.update(Duration::from_millis(400));
    game
}

fn dr_rustario_game() -> dr_rustario::game::Game {
    let random = dr_rustario::game::random::random(1, dr_rustario::game::random::RandomMode::Bag)
        .pop()
        .unwrap();
    let mut game =
        dr_rustario::game::Game::new(9, dr_rustario::game::GameSpeed::Medium, random).unwrap();
    for i in 0..6 {
        for _ in 0..(i % 3) {
            if i % 2 == 0 {
                game.left();
            } else {
                game.right();
            }
        }
        game.rotate(true);
        game.hard_drop();
        game.update(Duration::from_millis(200));
    }
    game.update(Duration::from_millis(400));
    game
}
