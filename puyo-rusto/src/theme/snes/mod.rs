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
use crate::game::rules::{MAX_LEVEL, MAX_SCORE};
use crate::theme::data::{audio, cells, hud, previews, Sounds};
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
use sdl2::rect::{Point, Rect};
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

/// Where the field sits in the panel. The panel is the whole SNES screen cut off at the
/// second player's field, so **a point here is a point on the SNES screen** - and a point
/// handed to the *engine* is that plus [`TOP_PADDING`], since the padding sits above
/// everything and the HUD is measured from the top of that. `board_point` is the exception:
/// the board texture carries the same padding, so it is given the panel point as it stands.
const FIELD: (i32, i32) = (8, 16);

/// The two boxes under `NEXT`: the gaps between the three wooden posts that run down the
/// column, which is what the game frames its queues with. Kirby's Avalanche puts the player's
/// next pair in one and the opponent's in the other and names them over the top; a panel here
/// belongs to one player with both boxes to itself, so `rip_retro.py` paints the names out
/// and the queue runs left to right through both - next, then next but one.
const NEXT_BOXES: [(i32, i32, u32, u32); 2] = [(108, 39, 16, 40), (130, 39, 18, 40)];

/// The recess under `STAGE`, which is `rip_retro.py`'s `SNES_STAGE_NUMBER` - the game prints
/// its stage number in it and the script fills it flat, because that number is the game's and
/// this one has its own. The level goes back in, right aligned where the original's single
/// digit sat.
const STAGE_BOX: (i32, i32, u32, u32) = (120, 103, 16, 16);

/// the mouth of the arch at the foot of the centre column, where Kirby stands in the
/// original: forty eight pixels across, which is exactly six tray icons at half size, and
/// the only clear run this column has that is as wide as the tray needs
const ARCH_MOUTH: (i32, i32, u32, u32) = (104, 186, 48, 14);
/// ... at which the tray's icons are drawn, half the cell so six of them fit
const TRAY_ICON: u32 = SRC_BLOCK_SIZE / 2;

/// the game's own digits are 8x8 tiles, which is what [`STAGE_BOX`] centres one against
const FONT_HEIGHT: u32 = 8;

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
            hud(
                MetricSnips::right((100, 225), MAX_SCORE),
                MetricSnips::right(
                    (
                        STAGE_BOX.0 + STAGE_BOX.2 as i32,
                        STAGE_BOX.1
                            + TOP_PADDING as i32
                            + (STAGE_BOX.3 as i32 - FONT_HEIGHT as i32) / 2,
                    ),
                    MAX_LEVEL,
                ),
            ),
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
        // one pair per box under the game's own `NEXT`, at the size the game drew them
        peek: PeekLayout::Slots {
            slots: NEXT_BOXES
                .iter()
                .map(|(x, y, w, h)| {
                    Rect::from_center(
                        Point::new(x + *w as i32 / 2, y + TOP_PADDING as i32 + *h as i32 / 2),
                        SRC_BLOCK_SIZE,
                        SRC_BLOCK_SIZE * 2,
                    )
                })
                .collect(),
            max_scale: 1.0,
        },
        // the tray goes across the mouth of the arch: Kirby's Avalanche takes its hits as
        // they arrive and drew nothing waiting anywhere, and this column is too narrow to
        // carry six cells at their own size anywhere else
        pending: Some(PendingLayout {
            point: Point::new(
                ARCH_MOUTH.0,
                ARCH_MOUTH.1 + TOP_PADDING as i32 + (ARCH_MOUTH.3 as i32 - TRAY_ICON as i32) / 2,
            ),
            step: Point::new(TRAY_ICON as i32, 0),
            size: TRAY_ICON,
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
        // the flower border above the field and the grass under it are a cell each, which is
        // what puts the field at 16 rather than the 15 the layer render read
        assert_eq!(FIELD.1 as u32, SRC_BLOCK_SIZE);
        assert_eq!(FIELD.1 as u32 + board_height + SRC_BLOCK_SIZE, height);
    }

    /// the boxes are the game's own furniture, measured off the layer render by
    /// `rip_retro.py`; what this checks is that what is put in them fits and lands on the panel
    #[test]
    fn everything_the_panel_is_told_to_draw_lands_on_it() {
        let (width, height) = png_size(sprites::BACKGROUND);
        for (x, y, w, h) in NEXT_BOXES {
            assert!(x as u32 + w <= width, "a next box runs off the panel");
            assert!(y as u32 + h <= height);
            assert!(
                w >= SRC_BLOCK_SIZE && h >= SRC_BLOCK_SIZE * 2,
                "a pair does not fit"
            );
        }
        // the level goes in the recess the game printed its stage number in, right aligned
        // where that number sat, and a digit of the game's own face has to fit the box
        assert!(STAGE_BOX.2 >= 8 && STAGE_BOX.3 >= FONT_HEIGHT);
        assert!(STAGE_BOX.0 as u32 + STAGE_BOX.2 <= width);
        assert!(STAGE_BOX.1 as u32 + STAGE_BOX.3 <= height);
        assert!(ARCH_MOUTH.0 as u32 + ARCH_MOUTH.2 <= width);
        assert!(ARCH_MOUTH.1 as u32 + ARCH_MOUTH.3 <= height);
        // six icons across the arch, at whole pixels, which is what half a cell buys
        assert_eq!(TRAY_ICON * COLUMNS, ARCH_MOUTH.2);
    }

    #[test]
    fn the_font_is_ten_digits_wide() {
        let (width, _) = png_size(sprites::FONT);
        assert_eq!(width % 10, 0);
    }
}
