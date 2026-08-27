//! renders one frame of a match on every theme of a game, straight to a PNG, without opening
//! a visible window.
//!
//! `cargo run -p dr-rustario-vs-rustris --example frame_shot -- 640 480 1 out/ [game]`

use engine::animate::popup::POPUP_DURATION;
use engine::app_info::{init, AppInfo};
use engine::config::{Config, VideoConfig, VideoMode};
use engine::game::{Game, GameEvent};
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
    // leaked for the life of the process, as `Shell::new` leaks its own: a `Theme` borrows
    // the texture creator, and everything holding a theme has to outlive it
    let texture_creator: &'static TextureCreator<WindowContext> =
        Box::leak(Box::new(canvas.texture_creator()));

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
            let themes = leak(rustris::theme::all_themes(
                &mut canvas,
                texture_creator,
                config,
            )?);
            shoot(
                &mut canvas,
                texture_creator,
                themes,
                config,
                (width, height),
                players,
                &out,
                &game,
                rustris_game,
            )
        }
        "dr-rustario" => {
            let themes = leak(dr_rustario::theme::all_themes(
                &mut canvas,
                texture_creator,
                config,
            )?);
            shoot(
                &mut canvas,
                texture_creator,
                themes,
                config,
                (width, height),
                players,
                &out,
                &game,
                dr_rustario_game,
            )
        }
        "puyo" => {
            let themes = leak(puyo_rusto::theme::all_themes(
                &mut canvas,
                texture_creator,
                config,
            )?);
            shoot(
                &mut canvas,
                texture_creator,
                themes,
                config,
                (width, height),
                players,
                &out,
                &game,
                puyo_game,
            )
        }
        other => Err(format!(
            "unknown game '{other}', expected rustris, dr-rustario or puyo"
        )),
    }
}

/// A `Theme` borrows the texture creator, which is leaked for the life of the process - so
/// the themes are leaked with it rather than threading two lifetimes through everything that
/// holds one. This is a shot tool that renders a handful of frames and exits.
fn leak(themes: Vec<Theme<'static>>) -> &'static [Theme<'static>] {
    Box::leak(themes.into_boxed_slice())
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
        let mut games = (0..players).map(|_| new_game()).collect::<Vec<G>>();
        // Replay the caption a game asked for over its last clear, exactly as the match
        // screen does - it is part of the frame and this is the only way to see one without
        // a display. Games that ask for none (Dr. Rustario and Rustris) are untouched, down
        // to the animation clock, which is why the shot only moves when there is a popup.
        let mut has_popup = false;
        for (player, game) in games.iter_mut().enumerate() {
            let events = Game::drain_events(game);
            let last_clear = events
                .iter()
                .rev()
                .find(|event| matches!(event, GameEvent::Clear { .. }));
            if let Some(event @ GameEvent::Clear { cells, .. }) = last_clear {
                if let Some(text) = GameRender::clear_popup(&*game, event) {
                    themes.animate_popup(player as u32, text, cells);
                    has_popup = true;
                }
            }
        }
        if has_popup {
            // a popup grows into place, so a frame taken the instant it is queued would draw
            // it at nothing: hold the shot a quarter of the way through its life
            themes.update_animations(POPUP_DURATION / 4);
        }
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
        // last of all, as the match screen draws them: over the board and over the particles
        themes.draw_popups(canvas)?;

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

/// a board part way through: a stack with groups linked up in it, an attack in the tray and,
/// so the shot carries a clear to replay, play stopped the moment something popped
fn puyo_game() -> puyo_rusto::game::Game {
    use engine::game::Game as _;
    // three colours make a group turn up in a handful of placements, which is what this shot
    // wants; a five colour board would usually be a tidy heap and nothing more
    let difficulty = puyo_rusto::game::rules::Difficulty::VeryEasy;
    let random = puyo_rusto::game::random::random(1, difficulty.colors())
        .pop()
        .unwrap();
    let mut game = puyo_rusto::game::Game::new(difficulty, 2, random);
    // round the columns from the left, which stacks the board up without tidying it - the
    // point of the shot is the link masks, so it wants colours landing beside each other
    for column in (0..puyo_rusto::game::board::COLUMNS as i32)
        .cycle()
        .take(48)
    {
        for _ in 0..puyo_rusto::game::board::COLUMNS {
            game.left();
        }
        for _ in 0..column {
            game.right();
        }
        game.hard_drop();
        game.update(Duration::from_millis(1200));
        // the score only moves when something clears; stop on the first one so the clear the
        // shot replays is the one the board is still settling out of
        if game.score() > 0 {
            break;
        }
    }
    // an attack arriving after the last placement is still in the tray, which is where the
    // pending strip is drawn from
    game.receive_attack(
        engine::game::Attack::new(puyo_rusto::game::GAME_ID, 9)
            .with_foreign_for(puyo_rusto::game::GAME_ID, 9),
    );
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
