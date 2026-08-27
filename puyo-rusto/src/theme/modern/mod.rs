//! The particle theme: original art, drawn by `puyo-rusto/art/sprites.py`, and original
//! audio, synthesised by `puyo-rusto/art/audio.py`. Everything else - the background, the
//! board frame, the HUD and the cards - the engine draws procedurally.

use crate::game::board::{COLUMNS, HIDDEN_ROWS, ROWS, SPAWN, VISIBLE_ROWS};
use crate::game::cell::{LinkMask, PuyoColor};
use crate::theme::data::{audio, cells, previews, Sounds, HUD_MAX};
use engine::animate::destroy::DestroyStyle;
use engine::animate::frames::FrameAnimationType;
use engine::animate::game_over::GameOverStyle;
use engine::config::Config;
use engine::render::modern::{modern_theme, ModernThemeOptions};
use engine::render::scene::ClearParticles;
use engine::render::sprite_sheet::{BlockSpriteSheetData, GhostStyle};
use engine::render::Theme;
use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::time::Duration;

/// the sheet's cell, and the padding around it that keeps one snip from bleeding into the
/// next as the sheet is rescaled. Both are `puyo-rusto/art/sprites.py`'s.
pub const SRC_BLOCK_SIZE: u32 = 64;
const PAD: i32 = 4;
const PITCH: i32 = SRC_BLOCK_SIZE as i32 + 2 * PAD;

/// the row of the sheet the nuisance puyo and the tray symbols are on, under the five
/// colours
const EXTRAS_ROW: i32 = PuyoColor::N as i32;

const SPRITES: &[u8] = include_bytes!("sprites.png");

mod sound {
    pub const ATTACK: &[u8] = include_bytes!("attack.ogg");
    pub const GAME_OVER: &[u8] = include_bytes!("game-over.ogg");
    pub const GARBAGE: &[u8] = include_bytes!("garbage.ogg");
    pub const HARD_DROP: &[u8] = include_bytes!("hard-drop.ogg");
    pub const LOCK: &[u8] = include_bytes!("lock.ogg");
    pub const MOVE: &[u8] = include_bytes!("move.ogg");
    pub const MUSIC: &[u8] = include_bytes!("music.ogg");
    pub const PAUSE: &[u8] = include_bytes!("pause.ogg");
    pub const POP: [&[u8]; 4] = [
        include_bytes!("pop-1.ogg"),
        include_bytes!("pop-2.ogg"),
        include_bytes!("pop-3.ogg"),
        include_bytes!("pop-4.ogg"),
    ];
    pub const ROTATE: &[u8] = include_bytes!("rotate.ogg");
    pub const SETTLE: &[u8] = include_bytes!("settle.ogg");
    pub const SPEED_UP: &[u8] = include_bytes!("speed-up.ogg");
    pub const VICTORY: &[u8] = include_bytes!("victory.ogg");
}

/// what the theme radiates into the background particle field: the five puyo colours, read
/// off the sheet
const PUYO_PALETTE: [Color; PuyoColor::N] = [
    Color::RGB(0xF0, 0x44, 0x2E), // red
    Color::RGB(0x3C, 0xC6, 0x3C), // green
    Color::RGB(0x2E, 0x6B, 0xF0), // blue
    Color::RGB(0xF0, 0xC4, 0x1E), // yellow
    Color::RGB(0xB8, 0x45, 0xE0), // purple
];

/// how long the popped puyos hold before the particles take over. Under the game's own
/// [`crate::game::rules::POP_DELAY`], so a chain step is never waiting on the animation
const POP_HOLD: Duration = Duration::from_millis(200);

fn block(col: i32, row: i32) -> Point {
    Point::new(PAD + PITCH * col, PAD + PITCH * row)
}

/// a colour's sixteen link variants run along its own row, indexed by the mask's bits
fn puyo(color: PuyoColor, links: LinkMask) -> Point {
    block(links.bits() as i32, color as i32)
}

pub fn modern_puyo_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
    block_size: u32,
) -> Result<Theme<'a>, String> {
    let options = ModernThemeOptions {
        name: "particle",
        sprites: BlockSpriteSheetData {
            file: SPRITES,
            source_block_size: SRC_BLOCK_SIZE,
            cells: cells(
                SRC_BLOCK_SIZE,
                puyo,
                block(0, EXTRAS_ROW),
                [
                    block(1, EXTRAS_ROW),
                    block(2, EXTRAS_ROW),
                    block(3, EXTRAS_ROW),
                ],
            ),
            animations: vec![],
            ghost_alpha: 0x90,
            previews: previews(),
            mascot: None,
        },
        audio: audio(
            config.audio,
            Sounds {
                music: sound::MUSIC,
                move_pair: sound::MOVE,
                rotate: sound::ROTATE,
                lock: sound::LOCK,
                settle: sound::SETTLE,
                hard_drop: sound::HARD_DROP,
                pop: sound::POP,
                attack_sent: sound::ATTACK,
                receive_nuisance: sound::GARBAGE,
                speed_up: sound::SPEED_UP,
                paused: sound::PAUSE,
                victory: sound::VICTORY,
                game_over: sound::GAME_OVER,
            },
        )?,
        columns: COLUMNS,
        rows: ROWS,
        // every row is drawn, the hidden thirteenth included: `visible_rows` counts the
        // buffer in and `top_buffer_rows` is how many of them float above the frame, so the
        // board frame ends up around the twelve rows that are played in
        visible_rows: ROWS,
        block_size,
        // the thirteenth row floats above the frame the way Rustris's buffer does. It is
        // worth showing: a puyo up there cannot pop, so a chain with a foot in it is held
        // back, and a player who cannot see it cannot plan around it
        top_buffer_rows: HIDDEN_ROWS,
        // the score goes in the side column under the queue; the left column is empty, since
        // a Puyo board has no hold box either
        metrics: HUD_MAX.to_vec(),
        metrics_left: vec![],
        mascot: None,
        spawn_cell: SPAWN,
        cell_idle_type: FrameAnimationType::Static,
        queue_max: 2,
        // an attack waits in the tray until a chain answers it, which is the whole game -
        // one icon per column of the board
        pending_max: COLUMNS,
        particle_color: Color::WHITE,
        particle_palette: PUYO_PALETTE.to_vec(),
        // particles in the shape of each puyo that went, which is how a group bursts
        clear_particles: ClearParticles::Masked { fade_in: POP_HOLD },
        destroy_style: Some(DestroyStyle::Vanish { hold: POP_HOLD }),
        game_over_style: Some(GameOverStyle::Curtain {
            from_top: false,
            rows: VISIBLE_ROWS,
        }),
        ghost_style: GhostStyle::Alpha,
        hard_drop_rows_per_frame: engine::animate::hard_drop::DEFAULT_ROWS_PER_FRAME,
    };
    modern_theme(canvas, texture_creator, options)
}
