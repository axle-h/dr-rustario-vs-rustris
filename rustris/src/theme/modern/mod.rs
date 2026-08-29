use crate::game::board::{BOARD_WIDTH, TOTAL_HEIGHT};
use crate::game::tetromino::TetrominoShape;
use crate::game::{HARD_DROP_ROWS_PER_FRAME, VISIBLE_HEIGHT};
use crate::theme::data::{audio, cells, curtain, previews, Sounds, HUD_MAX};
use engine::animate::destroy::DestroyStyle;
use engine::animate::frames::FrameAnimationType;
use engine::config::Config;
use engine::game::geometry::Rotation;
use engine::render::modern::{modern_theme, ModernThemeOptions};
use engine::render::scene::ClearParticles;
use engine::render::sprite_sheet::{BlockSpriteSheetData, GhostStyle};
use engine::render::Theme;
use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::time::Duration;

/// what the modern theme radiates into the background particle field: the seven tetromino
/// colours, read off the sprite sheet
const TETROMINO_PALETTE: [Color; 7] = [
    Color::RGB(0x00, 0xD7, 0xFA), // I
    Color::RGB(0x2C, 0x00, 0xFA), // J
    Color::RGB(0xFF, 0x6C, 0x16), // L
    Color::RGB(0xFF, 0xCE, 0x13), // O
    Color::RGB(0x68, 0xEF, 0x13), // S
    Color::RGB(0xAD, 0x00, 0xFA), // T
    Color::RGB(0xFF, 0x1A, 0x45), // Z
];

const SPRITES: &[u8] = include_bytes!("sprites.png");
const GAME_OVER_SOUND: &[u8] = include_bytes!("game-over.ogg");
const LEVEL_UP_SOUND: &[u8] = include_bytes!("level-up.ogg");
const CLEAR_SINGLE_SOUND: &[u8] = include_bytes!("single.ogg");
const CLEAR_DOUBLE_SOUND: &[u8] = include_bytes!("double.ogg");
const CLEAR_TRIPLE_SOUND: &[u8] = include_bytes!("triple.ogg");
const TETRIS_SOUND: &[u8] = include_bytes!("tetris.ogg");
const HARD_DROP_SOUND: &[u8] = include_bytes!("hard-drop.ogg");
const HOLD_SOUND: &[u8] = include_bytes!("hold.ogg");
const LOCK_SOUND: &[u8] = include_bytes!("lock.ogg");
const MOVE_SOUND: &[u8] = include_bytes!("move.ogg");
const MUSIC: &[u8] = include_bytes!("music.ogg");
const PAUSE_SOUND: &[u8] = include_bytes!("pause.ogg");
const ROTATE_SOUND: &[u8] = include_bytes!("rotate.ogg");
const SEND_GARBAGE_SOUND: &[u8] = include_bytes!("send-garbage.ogg");
const SEND_GARBAGE_ALT_SOUND: &[u8] = include_bytes!("send-garbage-alt.ogg");
const STACK_DROP_SOUND: &[u8] = include_bytes!("stack-drop.ogg");
const VICTORY_SOUND: &[u8] = include_bytes!("victory.ogg");

pub const SRC_BLOCK_SIZE: u32 = 48;
const PARTICLE_FADE_IN: Duration = Duration::from_millis(750);

fn block(row: i32, col: i32) -> Point {
    Point::new(4 + 56 * col, 4 + 56 * row)
}

fn mino(col: i32) -> (Point, Point) {
    // (normal block, stack block)
    (block(0, col), block(1, col))
}

pub fn modern_rustris_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
    block_size: u32,
    top_buffer_rows: u32,
) -> Result<Theme<'a>, String> {
    let spawn = TetrominoShape::I.meta().spawn_point();
    let spawn_cell = {
        let p = TetrominoShape::I.meta().rotated_minos(Rotation::North)[0] + spawn;
        engine::game::geometry::Point::new(p.x, TOTAL_HEIGHT as i32 - 1 - p.y)
    };
    let options = ModernThemeOptions {
        name: "particle",
        sprites: BlockSpriteSheetData {
            file: SPRITES,
            source_block_size: SRC_BLOCK_SIZE,
            cells: cells(
                SRC_BLOCK_SIZE,
                mino(6),
                mino(1),
                mino(3),
                mino(7),
                mino(2),
                mino(4),
                mino(5),
                block(0, 0),
            ),
            animations: vec![],
            ghost_alpha: 0x50,
            previews: previews(),
            mascot: None,
        },
        audio: audio(
            config.audio,
            Sounds {
                music: MUSIC,
                move_piece: MOVE_SOUND,
                rotate: ROTATE_SOUND,
                lock: LOCK_SOUND,
                send_garbage: SEND_GARBAGE_SOUND,
                clear: [
                    CLEAR_SINGLE_SOUND,
                    CLEAR_DOUBLE_SOUND,
                    CLEAR_TRIPLE_SOUND,
                    TETRIS_SOUND,
                ],
                level_up: LEVEL_UP_SOUND,
                game_over: GAME_OVER_SOUND,
                pause: PAUSE_SOUND,
                victory: VICTORY_SOUND,
                stack_drop: Some(STACK_DROP_SOUND),
                hard_drop: Some(HARD_DROP_SOUND),
                hold: Some(HOLD_SOUND),
            },
        )?,
        columns: BOARD_WIDTH,
        rows: TOTAL_HEIGHT,
        visible_rows: VISIBLE_HEIGHT,
        block_size,
        top_buffer_rows,
        metrics: HUD_MAX[1..].to_vec(),
        metrics_left: HUD_MAX[..1].to_vec(),
        mascot: None,
        spawn_cell,
        cell_idle_type: FrameAnimationType::Static,
        queue_max: 5,
        // nothing waits: an attack lands the moment it arrives
        pending_max: 0,
        particle_color: Color::WHITE,
        particle_palette: TETROMINO_PALETTE.to_vec(),
        clear_particles: ClearParticles::Rows {
            fade_in: PARTICLE_FADE_IN,
        },
        destroy_style: Some(DestroyStyle::Vanish {
            hold: PARTICLE_FADE_IN,
        }),
        game_over_style: Some(curtain(false)),
        ghost_style: GhostStyle::Outline {
            color: Color::WHITE,
        },
        hard_drop_rows_per_frame: HARD_DROP_ROWS_PER_FRAME,
        // no art for a caption, and no captions: see `clear_popup`
        popup_sprites: None,
        pop_debris: None,
        nuisance_rumble: None,
        attack_ball: None,
    };
    let _ = SEND_GARBAGE_ALT_SOUND;
    modern_theme(canvas, texture_creator, options)
}
