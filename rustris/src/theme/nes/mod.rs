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
const BACKGROUND_TILE: &[u8] = include_bytes!("background-tile.png");
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
const TETRIS_SOUND: &[u8] = include_bytes!("tetris.ogg");
const VICTORY_SOUND: &[u8] = include_bytes!("victory.ogg");
const ALPHA_PIXELS: u32 = 7;

fn char_snip(row: i32, col: i32) -> Rect {
    let point = Point::new(col * 8, 111 + row * 8);
    Rect::new(point.x(), point.y(), ALPHA_PIXELS, ALPHA_PIXELS)
}

const BACKGROUND_COLOR: Color = Color::RGB(0x74, 0x74, 0x74);
const BLOCK_PIXELS: u32 = 8;
const BUFFER_PIXELS: u32 = VISIBLE_BUFFER * BLOCK_PIXELS;

fn mino(i: i32, j: i32) -> Point {
    Point::new(i * BLOCK_PIXELS as i32, j * BLOCK_PIXELS as i32)
}

pub fn nes_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
) -> Result<Theme<'a>, String> {
    let buffer = BUFFER_PIXELS as i32;
    let options = RetroThemeOptions {
        name: "nes",
        scenes: vec![SceneType::Tile {
            texture: BACKGROUND_TILE,
        }],
        sprites: BlockSpriteSheetData {
            file: SPRITES,
            source_block_size: BLOCK_PIXELS,
            cells: cells(
                BLOCK_PIXELS,
                mino(0, 0),
                mino(2, 0),
                mino(1, 0),
                mino(0, 0),
                mino(2, 0),
                mino(0, 0),
                mino(1, 0),
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
            (7, 0),
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
                stack_drop: None,
                hard_drop: None,
                hold: None,
            },
        )?,
        font: FontThemeOptions::simple(
            retro_font(SPRITES, 1, |i| char_snip(0, i), |i| char_snip(1, i)),
            hud(
                buffer,
                zero_fill((8, 24), 6),
                zero_fill((20, 72), 3),
                zero_fill((20, 91), 4),
            ),
        ),
        board_file: BOARD_FILE,
        board_alpha: 0xff,
        board_snips: vec![],
        top_padding: BUFFER_PIXELS,
        board_point: Point::new(66, 0),
        background_file: BACKGROUND_FILE,
        background_color: BACKGROUND_COLOR,
        match_end_file: Some(GAME_OVER_FILE),
        game_over_points: vec![Point::new(0, 0)],
        interstitial_points: vec![],
        overlay_size: Some((80, 176)),
        hold: Some(HoldLayout::Slot {
            slot: Rect::new(16, 127 + buffer, 32, 32),
            max_scale: 1.0,
        }),
        peek: PeekLayout::Slots {
            slots: vec![
                Rect::new(170, 16 + buffer, 32, 32),
                Rect::new(170, 56 + buffer, 32, 32),
                Rect::new(170, 80 + buffer, 32, 32),
                Rect::new(170, 104 + buffer, 32, 32),
                Rect::new(170, 128 + buffer, 32, 32),
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
        ghost_style: GhostStyle::Alpha,
        hard_drop_rows_per_frame: HARD_DROP_ROWS_PER_FRAME,
    };
    retro_theme(canvas, texture_creator, options)
}
