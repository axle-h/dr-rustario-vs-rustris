use crate::game::board::{BOARD_WIDTH, TOTAL_HEIGHT};
use crate::game::cell::Mino;
use crate::game::{HARD_DROP_ROWS_PER_FRAME, VISIBLE_BUFFER, VISIBLE_HEIGHT};
use crate::theme::data::{audio, cells, curtain, hud, previews, retro_font, zero_fill, Sounds};
use engine::animate::destroy::DestroyStyle;
use engine::animate::frames::FrameAnimationType;
use engine::config::Config;
use engine::render::font::FontThemeOptions;
use engine::render::geometry::BoardGeometry;
use engine::render::retro::{retro_theme, RetroThemeOptions};
use engine::render::scene::SceneType;
use engine::render::sprite_sheet::{BlockSpriteSheetData, GhostStyle};
use engine::render::{HoldLayout, PeekLayout, Theme};
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;

const SPRITES: &[u8] = include_bytes!("sprites.png");
const BACKGROUND_FILE: &[u8] = include_bytes!("background.png");
const BACKGROUND_TILE: &[u8] = include_bytes!("background-tile-gold.png");
const BOARD_FILE: &[u8] = include_bytes!("board.png");
const GAME_OVER_FILE: &[u8] = include_bytes!("game-over.png");
const GAME_OVER_SOUND: &[u8] = include_bytes!("game-over.ogg");
const LEVEL_UP_SOUND: &[u8] = include_bytes!("level-up.ogg");
const CLEAR_SOUND: &[u8] = include_bytes!("line-clear.ogg");
const LOCK_SOUND: &[u8] = include_bytes!("lock.ogg");
const MOVE_SOUND: &[u8] = include_bytes!("move.ogg");
const MUSIC: &[u8] = include_bytes!("music.ogg");
const PAUSE_SOUND: &[u8] = include_bytes!("pause.ogg");
const ROTATE_SOUND: &[u8] = include_bytes!("rotate.ogg");
const SEND_GARBAGE_SOUND: &[u8] = include_bytes!("send-garbage.ogg");
const STACK_DROP_SOUND: &[u8] = include_bytes!("stack-drop.ogg");
const TETRIS_SOUND: &[u8] = include_bytes!("tetris.ogg");
const VICTORY_SOUND: &[u8] = include_bytes!("victory.ogg");
const ALPHA_WIDTH: u32 = 7;
const ALPHA_HEIGHT: u32 = 8;

fn char_snip(row: i32, col: i32) -> Rect {
    let point = Point::new(col * 8, 35 + row * 9);
    Rect::new(point.x(), point.y(), ALPHA_WIDTH, ALPHA_HEIGHT)
}

const BACKGROUND_COLOR: Color = Color::RGB(0x74, 0x74, 0x74);
const BLOCK_PIXELS: u32 = 8;
const BUFFER_PIXELS: u32 = VISIBLE_BUFFER * BLOCK_PIXELS;

fn mino(i: i32, j: i32) -> Point {
    Point::new(i * BLOCK_PIXELS as i32, j * BLOCK_PIXELS as i32)
}

pub fn snes_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
) -> Result<Theme<'a>, String> {
    let buffer = BUFFER_PIXELS as i32;
    let options = RetroThemeOptions {
        name: "snes",
        scenes: vec![SceneType::Tile {
            texture: BACKGROUND_TILE,
        }],
        sprites: BlockSpriteSheetData {
            file: SPRITES,
            source_block_size: BLOCK_PIXELS,
            cells: cells(
                BLOCK_PIXELS,
                (mino(1, 1), mino(1, 0)),
                (mino(3, 1), mino(3, 0)),
                (mino(2, 1), mino(2, 0)),
                (mino(0, 1), mino(0, 0)),
                (mino(2, 1), mino(2, 0)),
                (mino(0, 1), mino(0, 0)),
                (mino(3, 1), mino(3, 0)),
                mino(0, 0),
            ),
            animations: vec![],
            ghost_alpha: 0x50,
            previews: previews(),
            mascot: None,
        },
        geometry: BoardGeometry::new(
            BLOCK_PIXELS,
            0,
            (8, 0),
            BOARD_WIDTH,
            TOTAL_HEIGHT,
            VISIBLE_HEIGHT,
        ),
        audio: audio(
            config.audio,
            Sounds {
                music: MUSIC,
                move_piece: MOVE_SOUND,
                rotate: ROTATE_SOUND,
                lock: LOCK_SOUND,
                send_garbage: SEND_GARBAGE_SOUND,
                clear: [CLEAR_SOUND, CLEAR_SOUND, CLEAR_SOUND, TETRIS_SOUND],
                level_up: LEVEL_UP_SOUND,
                game_over: GAME_OVER_SOUND,
                pause: PAUSE_SOUND,
                victory: VICTORY_SOUND,
                stack_drop: Some(STACK_DROP_SOUND),
                hard_drop: None,
                hold: None,
            },
        )?,
        font: FontThemeOptions::simple(
            retro_font(SPRITES, 1, |i| char_snip(0, i), |i| char_snip(1, i)),
            hud(
                buffer,
                zero_fill((7, 22), 6),
                zero_fill((23, 62), 3),
                zero_fill((23, 98), 4),
            ),
        ),
        board_file: BOARD_FILE,
        board_alpha: 0xff,
        board_snips: vec![],
        top_padding: BUFFER_PIXELS,
        shadow: None,
        board_point: Point::new(62, 0),
        background_file: BACKGROUND_FILE,
        background_color: BACKGROUND_COLOR,
        match_end_file: Some(GAME_OVER_FILE),
        game_over_points: vec![Point::new(0, 0)],
        interstitial_points: vec![],
        overlay_size: Some((31, 32)),
        hold: Some(HoldLayout::Slot {
            slot: Rect::new(19, 133 + buffer, 32, 32),
            max_scale: 1.0,
        }),
        peek: PeekLayout::Slots {
            slots: vec![
                Rect::new(168, 17 + buffer, 32, 32),
                Rect::new(168, 58 + buffer, 32, 32),
                Rect::new(168, 82 + buffer, 32, 32),
                Rect::new(168, 106 + buffer, 32, 32),
                Rect::new(168, 130 + buffer, 32, 32),
            ],
            max_scale: 1.0,
        },
        mascot: None,
        mascot_animations: None,
        spawn_arc: None,
        cell_idle_type: FrameAnimationType::Static,
        destroy_style: Some(DestroyStyle::Sweep),
        game_over_style: Some(curtain(true)),
        curtain_cell: Some(Mino::garbage()),
        // Dr. Rustario and Rustris take a hit as it arrives, so nothing ever waits
        pending: None,
        ghost_style: GhostStyle::Alpha,
        hard_drop_rows_per_frame: HARD_DROP_ROWS_PER_FRAME,
        pop_debris: None,
        nuisance_rumble: None,
        attack_ball: None,
    };
    retro_theme(canvas, texture_creator, options)
}
