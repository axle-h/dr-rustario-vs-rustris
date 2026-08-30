//! Steps a scripted Puyo Rusto match and writes one PNG every so many milliseconds, without
//! opening a visible window - so an animation can be *seen* rather than reasoned about.
//!
//! `frame_shot` renders one frame and replays only a clear's caption, which is enough to
//! check where things are drawn and nothing at all about how they move. This is its sibling
//! for the moving parts: the pop's blink and strip, the landing squash, the droplets thrown
//! off a burst, the ball crossing to the other board, the tray sliding in and the ragged
//! fall of a slab of nuisance.
//!
//! It reimplements the small part of `match_screen` that matters here - drain the events,
//! fan them out to the animations, and **skip the game's own update while an animation holds
//! the tick**, which is the rule the whole timing of a chain rests on.
//!
//! ```text
//! cargo run -p dr-rustario-vs-rustris --example animation_shot -- [width] [height] [out] [theme] [every_ms] [frames]
//! ```

use engine::app_info::{init, AppInfo};
use engine::config::{Config, VideoConfig, VideoMode};
use engine::game::{Attack, Game, GameEvent};
use engine::render::context::{PlayerTextures, PlayerThemes, TextureMode, ThemeContext};
use engine::render::{GameRender, Theme};
use puyo_rusto::game::rules::Difficulty;
use sdl2::pixels::{Color, PixelFormatEnum};
use std::time::Duration;

/// one simulated frame; the shot cadence is a multiple of it
const TICK: Duration = Duration::from_micros(16_667);
const PLAYERS: u32 = 2;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let arg = |i: usize, or: &str| args.get(i).cloned().unwrap_or_else(|| or.to_string());
    let width: u32 = arg(1, "960").parse().unwrap();
    let height: u32 = arg(2, "720").parse().unwrap();
    let out = arg(3, ".");
    let wanted = arg(4, "genesis");
    let every: u64 = arg(5, "50").parse().unwrap();
    let shots: usize = arg(6, "40").parse().unwrap();

    init(AppInfo {
        name: "animation-shot",
        version: "0",
        authors: "",
    });

    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let window = video
        .window("animation-shot", width, height)
        .position_centered()
        .hidden()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    // leaked for the life of the process, as `frame_shot` and `Shell::new` both leak theirs
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
    let index = all.iter().position(|t| t.name() == wanted).ok_or_else(|| {
        format!(
            "unknown theme '{wanted}', expected one of {:?}",
            all.iter().map(|t| t.name()).collect::<Vec<_>>()
        )
    })?;

    let mut themes = ThemeContext::new(
        all,
        texture_creator,
        (0..PLAYERS)
            .map(|_| PlayerThemes::new(0..all.len(), index))
            .collect(),
        (width, height),
        config.video,
    )?;
    let mut games = (0..PLAYERS)
        .map(|p| about_to_chain(p as usize))
        .collect::<Vec<_>>();
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

    let per_shot = (every * 1000 / TICK.as_micros() as u64).max(1);
    println!(
        "{} on {}: {shots} shots, one every {every}ms ({per_shot} ticks)",
        "puyo",
        all[index].name()
    );
    let mut tick = 0usize;
    for shot in 0..shots {
        for _ in 0..per_shot {
            step(&mut themes, &mut games, tick);
            tick += 1;
        }
        draw(&mut canvas, &mut themes, &games, &mut textures)?;
        let pixels = canvas.read_pixels(None, PixelFormatEnum::ABGR8888)?;
        let path = format!("{out}/{}-{shot:03}.png", all[index].name());
        image::RgbaImage::from_raw(width, height, pixels)
            .ok_or("bad pixel buffer")?
            .save(&path)
            .map_err(|e| e.to_string())?;
    }
    println!("wrote {shots} pngs to {out}/");
    Ok(())
}

/// One frame of the match, animations and all - the same order and the same rules the match
/// screen keeps, cut down to what this shot needs.
fn step(themes: &mut ThemeContext, games: &mut [puyo_rusto::game::Game], tick: usize) {
    let trays_before: Vec<usize> = games
        .iter()
        .map(|g| Game::pending_attacks(g).len())
        .collect();
    let mut sent: Vec<(u32, Attack)> = vec![];
    for (index, game) in games.iter_mut().enumerate() {
        let player = index as u32;
        // the rule the whole pace of a chain rests on: while an animation blocks, the game
        // does not tick at all, so a theme's clear animation *adds* to the rules' own delay
        if themes.is_pause_required_for_animation(player) {
            continue;
        }
        // a hand on the controls: walk to a column and drop, so the shot keeps producing
        // the things it is here to look at rather than watching one pair fall for a second
        if tick % 24 == 0 {
            let column = (tick / 24 + index * 2) % puyo_rusto::game::board::COLUMNS as usize;
            for _ in 0..puyo_rusto::game::board::COLUMNS {
                game.left();
            }
            for _ in 0..column {
                game.right();
            }
            game.hard_drop();
        }
        Game::update(game, TICK);
        for event in Game::drain_events(game) {
            match event {
                GameEvent::Clear { ref cells, .. } => {
                    if let Some(text) = GameRender::clear_popup(&*game, &event) {
                        themes.animate_popup(player, text, cells);
                    }
                    themes.animate_destroy(player, cells);
                }
                GameEvent::Landed { cells } => themes.animate_landed(player, &cells),
                GameEvent::Lock { cells, dropped } => {
                    if dropped {
                        themes.animate_impact(player);
                    }
                    themes.animate_lock(player, &cells);
                }
                GameEvent::AttackReceived { cells } => {
                    if let Some(fall) = GameRender::attack_fall(&*game) {
                        themes.animate_nuisance(player, &cells, fall);
                    }
                }
                GameEvent::AttackSent(attack) => sent.push((player, attack)),
                _ => {}
            }
        }
    }
    // ... and the session's own job: route what was sent to the other board, and throw the
    // ball that carries it
    for (from, attack) in sent {
        let to = (from + 1) % games.len() as u32;
        let strength = attack.strength_for(games[to as usize].game_id());
        games[to as usize].receive_attack(attack);
        themes.send_attack_ball(from, to, trays_before[to as usize], strength);
    }
    themes.update_animations(TICK);
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
    // ... and composited onto the window, which is what actually puts a board on the screen
    themes.draw_players(canvas, &mut texture_refs, Duration::ZERO)?;
    // then the match screen's own order, minus the particles: what the boards threw off,
    // the attacks crossing between them, and the captions over all of it
    themes.draw_debris(canvas)?;
    themes.draw_character_particles(canvas)?;
    themes.draw_attack_balls(canvas)?;
    themes.draw_popups(canvas)
}

/// A board stacked so that the very next placement sets off a chain, so the shot opens on
/// the moment worth watching rather than on forty frames of tidying.
fn about_to_chain(player: usize) -> puyo_rusto::game::Game {
    use puyo_rusto::game::{board, cell::PuyoSkin, random, rules};
    let difficulty = Difficulty::VeryEasy;
    static SEED: std::sync::OnceLock<random::Seed> = std::sync::OnceLock::new();
    let seed = *SEED.get_or_init(random::Seed::random);
    let gen = random::from_seed(seed, 1, difficulty.colors())
        .pop()
        .unwrap();
    let skins = PuyoSkin::deal(seed, player + 1);
    let mut game = puyo_rusto::game::Game::new(difficulty, 2, gen, skins[player]);
    let _ = rules::POP_DELAY;
    // round the columns from the left, which stacks the board without tidying it and leaves
    // colours landing beside each other - which is what makes a chain turn up
    for column in (0..board::COLUMNS as i32).cycle().take(40) {
        for _ in 0..board::COLUMNS {
            game.left();
        }
        for _ in 0..column {
            game.right();
        }
        game.hard_drop();
        Game::update(&mut game, Duration::from_millis(1200));
        if game.score() > 0 {
            break;
        }
    }
    game
}
