//! The SNES theme: Kirby's Avalanche, which is Compile's Puyo Puyo with Kirby's cast painted
//! over it - so its board is this game's board exactly, six columns and twelve rows of a
//! sixteen pixel blob, joined when they touch.
//!
//! The blobs come out of the "Blobs & Boulders" rip. **Everything else comes out of the game**,
//! because that sheet is the only playfield art there is: no board, no background, no font. It
//! is not screenshotted either - `puyo-rusto/art/rip_retro.py` drives the emulator, pokes the
//! SNES's own main-screen register in a savestate and renders the background layers on their
//! own, without the blobs, without Kirby and without either player's HUD. The long version is
//! in the script, next to `SNES_LAYERS_BOTH`.
//!
//! What the game leaves in the panel is its own furniture: the flower border, the wooden centre
//! column, `NEXT`, and the `SC` label the score is drawn after. Its own score and its own stage
//! number are painted out, since this game prints neither in that place.

use crate::game::board::{COLUMNS, HIDDEN_ROWS, ROWS, VISIBLE_ROWS};
use crate::game::cell::{LinkMask, PuyoColor, PuyoSkin};
use crate::theme::data::{audio, cells, previews, Sounds, HUD_MAX};
use crate::theme::{sound, GAME_MUSIC};
use engine::animate::destroy::DestroyStyle;
use engine::animate::frames::FrameAnimationType;
use engine::animate::game_over::GameOverStyle;
use engine::config::Config;
use engine::render::font::{FontRenderOptions, FontThemeOptions, MetricSnips};
use engine::render::geometry::BoardGeometry;
use engine::render::retro::{retro_theme, RetroThemeOptions};
use engine::render::scene::SceneType;
use engine::render::sprite_sheet::{BlockSpriteSheetData, GhostStyle};
use engine::render::{PeekLayout, PendingLayout, Theme};
use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::time::Duration;

mod sprites {
    pub const SPRITES: &[u8] = include_bytes!("sprites.png");
    pub const BACKGROUND: &[u8] = include_bytes!("background.png");
    pub const BACKGROUND_TILE: &[u8] = include_bytes!("background-tile.png");
    pub const BOARD: &[u8] = include_bytes!("board.png");
    pub const FONT: &[u8] = include_bytes!("font.png");
}

/// the SNES's own blob, and `rip_retro.py`'s grid
pub const SRC_BLOCK_SIZE: u32 = 16;
const PAD: i32 = 4;
const PITCH: i32 = SRC_BLOCK_SIZE as i32 + 2 * PAD;

/// the row under the five colours, holding the boulder and the tray's three symbols
const EXTRAS_ROW: i32 = PuyoColor::N as i32;

/// one transparent cell above everything, for the thirteenth row to float in
const TOP_PADDING: u32 = SRC_BLOCK_SIZE * HIDDEN_ROWS;

/// where the field sits in the panel, measured off the BG1 render: it is the one flat run of
/// the backdrop colour, six columns and twelve rows of sixteen
const FIELD: (i32, i32) = (8, 15);

/// how long a group holds before it goes, under [`crate::game::rules::POP_DELAY`]
const POP_HOLD: Duration = Duration::from_millis(200);

/// the canopy the boards stand against, read off the forest layer
const FOREST: Color = Color::RGB(0x08, 0x28, 0x10);

fn block(col: i32, row: i32) -> Point {
    Point::new(PAD + PITCH * col, PAD + PITCH * row)
}

/// a colour's sixteen link variants run along its own row, indexed by the mask's bits. The
/// skin is ignored: Kirby's Avalanche drew one set of blobs, so both players see the same.
fn puyo(_: PuyoSkin, color: PuyoColor, links: LinkMask) -> Point {
    block(links.bits() as i32, color as i32)
}

pub fn snes_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
) -> Result<Theme<'a>, String> {
    let options = RetroThemeOptions {
        name: "snes",
        scenes: vec![SceneType::Tile {
            texture: sprites::BACKGROUND_TILE,
        }],
        sprites: BlockSpriteSheetData {
            file: sprites::SPRITES,
            source_block_size: SRC_BLOCK_SIZE,
            cells: cells(
                SRC_BLOCK_SIZE,
                puyo,
                |_| block(0, EXTRAS_ROW),
                |_| {
                    [
                        block(1, EXTRAS_ROW),
                        block(2, EXTRAS_ROW),
                        block(3, EXTRAS_ROW),
                    ]
                },
            ),
            animations: vec![],
            ghost_alpha: 0x60,
            previews: previews(),
            mascot: None,
        },
        geometry: BoardGeometry::new(SRC_BLOCK_SIZE, 0, (0, 0), COLUMNS, ROWS, ROWS),
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
        // Right aligned where the game printed its own, just after the `SC` its border keeps.
        // Every point on this panel is in the *padded* background's coordinates, so it is one
        // cell lower than the same place in the art `rip_retro.py` wrote - `top_padding` sits
        // above everything and the HUD is measured from the top of that.
        font: FontThemeOptions::simple(
            FontRenderOptions::numeric_sprites(sprites::FONT, texture_creator, 1)?,
            HUD_MAX
                .iter()
                .map(|(kind, max)| (*kind, MetricSnips::right((100, 225), *max)))
                .collect(),
        ),
        board_file: sprites::BOARD,
        board_alpha: 0xff,
        board_snips: vec![],
        top_padding: TOP_PADDING,
        board_point: Point::new(FIELD.0, FIELD.1),
        background_file: sprites::BACKGROUND,
        background_color: FOREST,
        match_end_file: None,
        game_over_points: vec![],
        interstitial_points: vec![],
        overlay_size: None,
        hold: None,
        // under the game's own `NEXT`, in the framed slot it drew for one
        peek: PeekLayout::Column {
            point: Point::new(114, 62),
            offset: 26,
            max: 2,
            // three quarters, so both pairs fit the slot the game framed for its own two
            // without running down over `STAGE`
            scale: Some(0.75),
        },
        // the tray runs down the wooden column between `STAGE` and the arch, which is the only
        // clear stretch of panel this layout has: Kirby's Avalanche takes its hits as they
        // arrive and drew nothing waiting anywhere
        pending: Some(PendingLayout {
            point: Point::new(119, 138),
            step: Point::new(0, 9),
            size: 9,
            max: COLUMNS,
        }),
        mascot: None,
        mascot_animations: None,
        spawn_arc: None,
        cell_idle_type: FrameAnimationType::Static,
        destroy_style: Some(DestroyStyle::Vanish { hold: POP_HOLD }),
        game_over_style: Some(GameOverStyle::Curtain {
            from_top: false,
            rows: VISIBLE_ROWS,
        }),
        curtain_cell: None,
        ghost_style: GhostStyle::Alpha,
        hard_drop_rows_per_frame: engine::animate::hard_drop::DEFAULT_ROWS_PER_FRAME,
    };
    retro_theme(canvas, texture_creator, options)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_size(bytes: &[u8]) -> (u32, u32) {
        let word = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
        (word(16), word(20))
    }

    #[test]
    fn the_sheet_is_the_shape_the_layout_reads_it_as() {
        let (width, height) = png_size(sprites::SPRITES);
        assert_eq!(width, (PITCH * LinkMask::COUNT as i32) as u32);
        assert_eq!(height, (PITCH * (EXTRAS_ROW + 1)) as u32);
    }

    /// the board is drawn *under* the panel, so the panel needs a hole exactly where the field
    /// is - the one thing about a retro theme that nothing but the art records
    #[test]
    fn the_field_fits_the_hole_it_is_drawn_into() {
        let (width, height) = png_size(sprites::BACKGROUND);
        let (board_width, board_height) = png_size(sprites::BOARD);
        assert_eq!(board_width, COLUMNS * SRC_BLOCK_SIZE);
        assert_eq!(board_height, VISIBLE_ROWS * SRC_BLOCK_SIZE);
        assert!(FIELD.0 as u32 + board_width <= width);
        assert!(FIELD.1 as u32 + board_height <= height);
    }

    #[test]
    fn the_font_is_ten_digits_wide() {
        let (width, _) = png_size(sprites::FONT);
        assert_eq!(width % 10, 0);
    }
}
