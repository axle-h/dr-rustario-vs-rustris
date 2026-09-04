use crate::game::board::{BOARD_WIDTH, TOTAL_HEIGHT};
use crate::game::cell::Mino;
use crate::game::{HARD_DROP_ROWS_PER_FRAME, VISIBLE_BUFFER, VISIBLE_HEIGHT};
use crate::theme::data::{audio, cells, curtain, hud, previews, retro_font, right, Sounds};
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
const STACK_DROP_SOUND: &[u8] = include_bytes!("stack-drop.ogg");
const TETRIS_SOUND: &[u8] = include_bytes!("tetris.ogg");
const VICTORY_SOUND: &[u8] = include_bytes!("victory.ogg");
const ALPHA_PIXELS: u32 = 6;

fn char_snip(row: i32, col: i32) -> Rect {
    // characters are in row x col with 8 pixels between columns and 7 pixels between rows
    let point = Point::new(1 + col * 8, 45 + row * 7);
    Rect::new(point.x(), point.y(), ALPHA_PIXELS, ALPHA_PIXELS)
}

const BACKGROUND_COLOR: Color = Color::WHITE;
const BLOCK_PIXELS: u32 = 8;
const BUFFER_PIXELS: u32 = VISIBLE_BUFFER * BLOCK_PIXELS;

pub fn game_boy_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
) -> Result<Theme<'a>, String> {
    let buffer = BUFFER_PIXELS as i32;
    let options = RetroThemeOptions {
        name: "gameboy",
        scenes: vec![SceneType::Tile {
            texture: BACKGROUND_TILE,
        }],
        sprites: BlockSpriteSheetData {
            file: SPRITES,
            source_block_size: BLOCK_PIXELS,
            cells: cells(
                BLOCK_PIXELS,
                [
                    Point::new(1, 35),
                    Point::new(9, 35),
                    Point::new(17, 35),
                    Point::new(25, 35),
                ],
                Point::new(51, 26),
                Point::new(26, 26),
                Point::new(1, 1),
                Point::new(51, 1),
                Point::new(1, 26),
                Point::new(18, 1),
                Point::new(34, 35),
            ),
            animations: vec![],
            ghost_alpha: 0x30,
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
                effects: 100,
            },
        )?,
        font: FontThemeOptions::simple(
            retro_font(
                SPRITES,
                2,
                |i| char_snip(3, i),
                |i| char_snip(i / 10, i % 10),
            ),
            hud(
                buffer,
                right((50, 25), 6),
                right((39, 52), 3),
                right((39, 78), 4),
            ),
        ),
        board_file: BOARD_FILE,
        board_alpha: 0xbb,
        board_snips: vec![],
        top_padding: BUFFER_PIXELS,
        bottom_padding: 0,
        shadow: None,
        board_point: Point::new(55, 0),
        background_file: BACKGROUND_FILE,
        background_color: BACKGROUND_COLOR,
        match_end_file: Some(GAME_OVER_FILE),
        game_over_points: vec![Point::new(0, 0)],
        interstitial_points: vec![],
        overlay_size: Some((80, 176)),
        hold: Some(HoldLayout::Slot {
            slot: Rect::new(12, 101 + buffer, 32, 32),
            max_scale: 1.0,
        }),
        peek: PeekLayout::Slots {
            slots: vec![
                Rect::new(162, 11 + buffer, 32, 32),
                Rect::new(162, 48 + buffer, 32, 32),
                Rect::new(162, 72 + buffer, 32, 32),
                Rect::new(162, 96 + buffer, 32, 32),
                Rect::new(162, 120 + buffer, 32, 32),
            ],
            max_scale: 1.0,
        },
        // no cast on this theme; see `engine::render::character`
        characters: None,
        mascot: None,
        mascot_animations: None,
        spawn_arc: None,
        cell_idle_type: FrameAnimationType::Static,
        destroy_style: Some(DestroyStyle::Flash),
        game_over_style: Some(curtain(false)),
        curtain_cell: Some(Mino::garbage()),
        // Dr. Rustario and Rustris take a hit as it arrives, so nothing ever waits
        pending: None,
        ghost_style: GhostStyle::Alpha,
        pop_debris: None,
        nuisance_rumble: None,
        attack_ball: None,
        hard_drop_rows_per_frame: HARD_DROP_ROWS_PER_FRAME,
    };
    retro_theme(canvas, texture_creator, options)
}
