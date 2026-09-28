//! The particle theme: puyos cut from the Puyo Puyo Tetris rip by `puyo-rusto/art/rip.py`;
//! everything else the engine draws procedurally.

use crate::game::board::{COLUMNS, HIDDEN_ROWS, ROWS, SPAWN, VISIBLE_ROWS};
use crate::game::cell::{LinkMask, PuyoColor, PuyoSkin};
use crate::theme::data::{audio, cells, previews, Sounds, HUD_MAX, PARTICLE_GAIN};
use crate::theme::{sound, GAME_MUSIC};
use engine::animate::destroy::DestroyStyle;
use engine::animate::frames::FrameAnimationType;
use engine::animate::game_over::GameOverStyle;
use engine::config::Config;
use engine::render::font::PopupSpriteData;
use engine::render::modern::{modern_theme, ModernThemeOptions};
use engine::render::scene::ClearParticles;
use engine::render::sprite_sheet::{BlockSpriteSheetData, GhostStyle};
use engine::render::Theme;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::time::Duration;

/// the sheet's cell and the padding that stops snips bleeding when rescaled, both
/// `puyo-rusto/art/rip.py`'s; a neck runs exactly to the cell's edge
pub const SRC_BLOCK_SIZE: u32 = 72;
const PAD: i32 = 4;
const PITCH: i32 = SRC_BLOCK_SIZE as i32 + 2 * PAD;

/// the row of a skin's band holding the nuisance puyo and the tray symbols
const EXTRAS_ROW: i32 = PuyoColor::N as i32;

/// how many rows of the sheet one skin takes: a row per colour, then the extras
const SKIN_ROWS: i32 = EXTRAS_ROW + 1;

/// how many skins the sheet carries, which is `SKINS` in `puyo-rusto/art/rip.py`; every one
/// is keyed and [`PuyoSkin::deal`] picks a player's
pub const SKINS: usize = PuyoSkin::COUNT;

/// How many skin bands lie side by side, which is `BANDS_ACROSS` in `puyo-rusto/art/rip.py`.
/// The sheet is one texture, so each dimension must stay within `MAX_ATLAS_WIDTH`.
const BANDS_ACROSS: usize = 2;

/// how wide one skin's band is, in cells: a link mask's worth
const BAND_COLUMNS: i32 = LinkMask::COUNT as i32;

const SPRITES: &[u8] = include_bytes!("sprites.png");

/// The chain caption: ten digits along the top row and the word under them, laid out as
/// `rip.py`'s `POPUP_CELL` and `POPUP_WORD_CELL`. Glyphs are cut against their row's baseline,
/// so the whole caption is drawn at one y.
const POPUP: &[u8] = include_bytes!("popup.png");
const POPUP_PAD: i32 = 4;
const POPUP_CELL: (u32, u32) = (64, 100);
const POPUP_WORD_CELL: (u32, u32) = (132, 100);
/// the gap between the number and the word, in the sheet's pixels
const POPUP_SPACE: u32 = 8;

/// The caption sheet as ten fixed-pitch digits and the word as one sprite.
fn popup_sprites() -> PopupSpriteData {
    const DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
    let pitch = POPUP_CELL.0 as i32 + 2 * POPUP_PAD;
    let digits = DIGITS.iter().enumerate().map(|(index, digit)| {
        let at = Rect::new(
            POPUP_PAD + pitch * index as i32,
            POPUP_PAD,
            POPUP_CELL.0,
            POPUP_CELL.1,
        );
        (*digit, at)
    });
    let word = Rect::new(
        POPUP_PAD,
        POPUP_PAD + POPUP_CELL.1 as i32 + 2 * POPUP_PAD,
        POPUP_WORD_CELL.0,
        POPUP_WORD_CELL.1,
    );
    PopupSpriteData {
        file: POPUP,
        cell_height: POPUP_CELL.1,
        space: POPUP_SPACE,
        glyphs: digits.chain(std::iter::once(("chain", word))).collect(),
    }
}

/// The five puyo colours the background particle field radiates, from the glossy Tsu skin.
const PUYO_PALETTE: [Color; PuyoColor::N] = [
    Color::RGB(0xBC, 0x3F, 0x3E), // red
    Color::RGB(0x64, 0xC9, 0x43), // green
    Color::RGB(0x30, 0x69, 0xCF), // blue
    Color::RGB(0xE7, 0xAA, 0x31), // yellow
    Color::RGB(0x98, 0x47, 0xCC), // purple
];

/// How long the popped puyos hold before the particles take over. It adds to
/// [`crate::game::rules::POP_DELAY`], since the match skips `game.update` while it blocks.
const POP_HOLD: Duration = Duration::from_millis(200);

/// How far, in blocks, and how long the board shakes when a slab of nuisance lands. Only this
/// theme shakes; the source games do not.
const NUISANCE_RUMBLE: (f64, Duration) = (1.0 / 12.0, Duration::from_millis(280));

fn block(col: i32, row: i32) -> Point {
    Point::new(PAD + PITCH * col, PAD + PITCH * row)
}

/// where a cell of a skin's own band sits on the sheet, given where it sits in the band
fn skin_block(skin: PuyoSkin, row: i32, col: i32) -> Point {
    let index = skin.index();
    block(
        BAND_COLUMNS * (index % BANDS_ACROSS) as i32 + col,
        SKIN_ROWS * (index / BANDS_ACROSS) as i32 + row,
    )
}

/// a colour's sixteen link variants run along its own row of the band, indexed by the bits
fn puyo(skin: PuyoSkin, color: PuyoColor, links: LinkMask) -> Point {
    skin_block(skin, color as i32, links.bits() as i32)
}

pub fn modern_puyo_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
    block_size: u32,
) -> Result<Theme<'a>, String> {
    let extras = |skin: PuyoSkin, col: i32| skin_block(skin, EXTRAS_ROW, col);
    let options = ModernThemeOptions {
        name: "particle",
        sprites: BlockSpriteSheetData {
            file: SPRITES,
            source_block_size: SRC_BLOCK_SIZE,
            cells: cells(
                SRC_BLOCK_SIZE,
                puyo,
                |skin| extras(skin, 0),
                |skin| [extras(skin, 1), extras(skin, 2), extras(skin, 3)],
            ),
            animations: vec![],
            ghost_alpha: 0x90,
            previews: previews(),
            mascot: None,
        },
        audio: audio(
            config.audio,
            Sounds {
                gain: PARTICLE_GAIN,
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
        // every row is drawn, the hidden thirteenth included
        visible_rows: ROWS,
        block_size,
        // the thirteenth row floats above the frame; a puyo there cannot pop
        top_buffer_rows: HIDDEN_ROWS,
        metrics: HUD_MAX.to_vec(),
        metrics_left: vec![],
        mascot: None,
        spawn_cell: SPAWN,
        cell_idle_type: FrameAnimationType::Static,
        queue_max: 2,
        // one tray icon per column
        pending_max: COLUMNS,
        particle_color: Color::WHITE,
        particle_palette: PUYO_PALETTE.to_vec(),
        clear_particles: ClearParticles::Masked { fade_in: POP_HOLD },
        destroy_style: Some(DestroyStyle::Vanish { hold: POP_HOLD }),
        game_over_style: Some(GameOverStyle::drain(VISIBLE_ROWS)),
        ghost_style: GhostStyle::Alpha,
        hard_drop_rows_per_frame: engine::animate::hard_drop::DEFAULT_ROWS_PER_FRAME,
        pop_debris: None,
        nuisance_rumble: Some(NUISANCE_RUMBLE),
        attack_ball: None,
        popup_sprites: Some(popup_sprites()),
    };
    modern_theme(canvas, texture_creator, options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::render::sprite_sheet::MAX_ATLAS_WIDTH;
    use std::collections::HashSet;

    /// a PNG's width and height, big endian at a fixed offset
    fn png_size(bytes: &[u8]) -> (u32, u32) {
        let word = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
        (word(16), word(20))
    }

    /// the sprite png is exactly [`SKINS`] bands laid [`BANDS_ACROSS`]
    #[test]
    fn the_sheet_carries_every_skin_the_theme_deals() {
        let (width, height) = png_size(SPRITES);
        assert_eq!(width, (PITCH * BAND_COLUMNS) as u32 * BANDS_ACROSS as u32);
        let band_rows = SKINS.div_ceil(BANDS_ACROSS);
        assert_eq!(height, (PITCH * SKIN_ROWS) as u32 * band_rows as u32);
    }

    /// the sprite png fits within [`MAX_ATLAS_WIDTH`] both ways
    #[test]
    fn the_sheet_fits_a_texture() {
        let (width, height) = png_size(SPRITES);
        assert!(width <= MAX_ATLAS_WIDTH, "{width} wide");
        assert!(height <= MAX_ATLAS_WIDTH, "{height} tall");
    }

    /// the last skin's extras lie inside the sprite png
    #[test]
    fn the_last_skins_extras_are_on_the_sheet() {
        let (width, height) = png_size(SPRITES);
        let last = skin_block(PuyoSkin::all().last().unwrap(), EXTRAS_ROW, 3);
        assert!(last.x + SRC_BLOCK_SIZE as i32 <= width as i32);
        assert!(last.y + SRC_BLOCK_SIZE as i32 <= height as i32);
    }

    /// the caption png's size matches the layout `popup_sprites` reads it as
    #[test]
    fn the_caption_sheet_is_the_shape_the_layout_reads_it_as() {
        let (width, height) = png_size(POPUP);
        assert_eq!(width, (POPUP_CELL.0 as i32 + 2 * POPUP_PAD) as u32 * 10);
        assert_eq!(height, (POPUP_CELL.1 as i32 + 2 * POPUP_PAD) as u32 * 2);
        for (_, at) in popup_sprites().glyphs {
            assert!(at.right() <= width as i32, "{at:?} runs off the sheet");
            assert!(at.bottom() <= height as i32, "{at:?} runs off the sheet");
        }
    }

    /// the caption sheet spells every chain from 1 to 99
    #[test]
    fn the_sheet_spells_every_chain_a_game_can_count_to() {
        let sprites = popup_sprites();
        for chain in 1..100 {
            assert!(sprites.spells(&format!("{chain} chain")), "{chain} chain");
        }
    }

    /// every skin reads a different band
    #[test]
    fn every_skin_reads_its_own_band() {
        let mut seen = HashSet::new();
        for skin in PuyoSkin::all() {
            assert!(seen.insert(puyo(skin, PuyoColor::Red, LinkMask::NONE)));
            assert!(seen.insert(skin_block(skin, EXTRAS_ROW, 0)));
        }
        assert_eq!(seen.len(), 2 * SKINS);
    }
}
