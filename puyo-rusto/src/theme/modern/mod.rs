//! The particle theme: puyos cut out of the Puyo Puyo Tetris rip by `puyo-rusto/art/rip.py`,
//! sound effects synthesised by `puyo-rusto/art/audio.py`, and the game's own music cut out
//! of the same rip by `puyo-rusto/art/music.py` - four tracks, of which a match is dealt one.
//! Everything else - the background, the board frame, the HUD and the cards - the engine
//! draws procedurally.

use crate::game::board::{COLUMNS, HIDDEN_ROWS, ROWS, SPAWN, VISIBLE_ROWS};
use crate::game::cell::{LinkMask, PuyoColor, PuyoSkin};
use crate::theme::data::{audio, cells, previews, Sounds, HUD_MAX};
use crate::theme::GAME_MUSIC;
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
/// next as the sheet is rescaled. Both are `puyo-rusto/art/rip.py`'s, and the cell is the
/// rip's own grid: a neck runs exactly to its edge, so two linked puyos meet flush.
pub const SRC_BLOCK_SIZE: u32 = 72;
const PAD: i32 = 4;
const PITCH: i32 = SRC_BLOCK_SIZE as i32 + 2 * PAD;

/// the row of a skin's band the nuisance puyo and the tray symbols are on, under the five
/// colours
const EXTRAS_ROW: i32 = PuyoColor::N as i32;

/// how many rows of the sheet one skin takes: a row per colour, then the extras
const SKIN_ROWS: i32 = EXTRAS_ROW + 1;

/// how many skins the sheet carries, which is `SKINS` in `puyo-rusto/art/rip.py`.
///
/// The rip is sixteen skins of the same puyos, fifteen of them whole and fourteen of those
/// able to join a puyo below; the sheet is all fourteen, one band under the next, and the
/// theme keys **every** one of them. Which two a
/// match shows is not the theme's to decide: [`PuyoSkin::deal`] hands each player one when the
/// match starts, so the choice can change without rebuilding an atlas.
pub const SKINS: usize = PuyoSkin::COUNT;

const SPRITES: &[u8] = include_bytes!("sprites.png");

mod sound {
    pub const ATTACK: &[u8] = include_bytes!("attack.ogg");
    pub const GAME_OVER: &[u8] = include_bytes!("game-over.ogg");
    pub const GARBAGE: &[u8] = include_bytes!("garbage.ogg");
    pub const HARD_DROP: &[u8] = include_bytes!("hard-drop.ogg");
    pub const LOCK: &[u8] = include_bytes!("lock.ogg");
    pub const MOVE: &[u8] = include_bytes!("move.ogg");
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

/// What the theme radiates into the background particle field: the five puyo colours, read
/// off the sheet.
///
/// The glossy Tsu skin's, and left at that whichever skins are dealt: every skin on the sheet
/// draws the same five hues, and the field is a wash behind both boards rather than a legend
/// for either of them.
const PUYO_PALETTE: [Color; PuyoColor::N] = [
    Color::RGB(0xBC, 0x3F, 0x3E), // red
    Color::RGB(0x64, 0xC9, 0x43), // green
    Color::RGB(0x30, 0x69, 0xCF), // blue
    Color::RGB(0xE7, 0xAA, 0x31), // yellow
    Color::RGB(0x98, 0x47, 0xCC), // purple
];

/// how long the popped puyos hold before the particles take over. Under the game's own
/// [`crate::game::rules::POP_DELAY`], so a chain step is never waiting on the animation
const POP_HOLD: Duration = Duration::from_millis(200);

fn block(col: i32, row: i32) -> Point {
    Point::new(PAD + PITCH * col, PAD + PITCH * row)
}

/// where a skin's band of six rows starts
fn skin_row(skin: PuyoSkin, row: i32) -> i32 {
    SKIN_ROWS * skin.index() as i32 + row
}

/// a colour's sixteen link variants run along its own row, indexed by the mask's bits
fn puyo(skin: PuyoSkin, color: PuyoColor, links: LinkMask) -> Point {
    block(links.bits() as i32, skin_row(skin, color as i32))
}

pub fn modern_puyo_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
    block_size: u32,
) -> Result<Theme<'a>, String> {
    let extras = |skin: PuyoSkin| skin_row(skin, EXTRAS_ROW);
    let options = ModernThemeOptions {
        name: "particle",
        sprites: BlockSpriteSheetData {
            file: SPRITES,
            source_block_size: SRC_BLOCK_SIZE,
            cells: cells(
                SRC_BLOCK_SIZE,
                puyo,
                |skin| block(0, extras(skin)),
                |skin| {
                    [
                        block(1, extras(skin)),
                        block(2, extras(skin)),
                        block(3, extras(skin)),
                    ]
                },
            ),
            animations: vec![],
            ghost_alpha: 0x90,
            previews: previews(),
            mascot: None,
        },
        audio: audio(
            config.audio,
            Sounds {
                music: &GAME_MUSIC,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn extras_row(skin: PuyoSkin) -> i32 {
        skin_row(skin, EXTRAS_ROW)
    }

    /// a PNG's width and height are big endian at a fixed offset, which is enough to check a
    /// sheet without decoding one
    fn png_size(bytes: &[u8]) -> (u32, u32) {
        let word = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
        (word(16), word(20))
    }

    /// [`SKINS`] is `puyo-rusto/art/rip.py`'s and nothing in Rust can check the script, but
    /// the sheet it wrote is right here: a band too many and a player would be dealt a skin
    /// off the bottom of it and draw nothing at all
    #[test]
    fn the_sheet_carries_every_skin_the_theme_deals() {
        let (width, height) = png_size(SPRITES);
        assert_eq!(width, (PITCH * LinkMask::COUNT as i32) as u32);
        assert_eq!(height, (PITCH * SKIN_ROWS) as u32 * SKINS as u32);
    }

    /// ... and every cell of the last skin's band is inside it
    #[test]
    fn the_last_skins_extras_are_on_the_sheet() {
        let (width, height) = png_size(SPRITES);
        let last = block(3, extras_row(PuyoSkin::all().last().unwrap()));
        assert!(last.x + SRC_BLOCK_SIZE as i32 <= width as i32);
        assert!(last.y + SRC_BLOCK_SIZE as i32 <= height as i32);
    }

    /// every skin has to key a *different* band, or two players dealt different sets would
    /// draw the same puyos
    #[test]
    fn every_skin_reads_its_own_band() {
        let mut seen = HashSet::new();
        for skin in PuyoSkin::all() {
            assert!(seen.insert(puyo(skin, PuyoColor::Red, LinkMask::NONE)));
            assert!(seen.insert(block(0, extras_row(skin))));
        }
        assert_eq!(seen.len(), 2 * SKINS);
    }
}
