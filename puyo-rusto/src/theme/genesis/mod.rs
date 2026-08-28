//! The Genesis theme: Dr. Robotnik's Mean Bean Machine, cut out of the spriters-resource
//! rips by `puyo-rusto/art/rip_retro.py`.
//!
//! Mean Bean Machine is Compile's Puyo Puyo with Robotnik's cast painted over it, so its
//! board is this game's board exactly: six columns, twelve rows, a sixteen pixel bean, and
//! beans of a colour that touch drawn joined. That last part is why the rip could be used at
//! all - the sheet carries every one of the sixteen link variants, and the script reads which
//! is which off the arrangement rather than being told.
//!
//! Everything here is in the Genesis's own pixels: the panel is cut straight out of the
//! game's 320x224 board and the block is 16, so `reference_block_size` scales the lot up
//! together and the theme is drawn at whatever size the window allows.

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

/// the Genesis's own bean, and `rip_retro.py`'s grid
pub const SRC_BLOCK_SIZE: u32 = 16;
const PAD: i32 = 4;
const PITCH: i32 = SRC_BLOCK_SIZE as i32 + 2 * PAD;

/// the row under the five colours, holding the refugee bean and the tray's three symbols
const EXTRAS_ROW: i32 = PuyoColor::N as i32;

/// The panel is cut at the well's top edge rather than the screen's, so the thirteenth row
/// has somewhere to float: `top_padding` puts exactly one transparent cell above everything
/// and the board frame starts under it.
const TOP_PADDING: u32 = SRC_BLOCK_SIZE * HIDDEN_ROWS;

/// where the well sits in the panel, which is where it sits on the Genesis screen less the
/// sixteen rows cut off the top. Not a `Point` constant: `sdl2::rect::Point::new` is not
/// `const`, and every other theme in the repository builds its points at the call site too.
const WELL: (i32, i32) = (16, 0);

/// The panel is cut at the well's top edge and [`TOP_PADDING`] puts exactly that much back,
/// so **a point in the padded background is a point on the Genesis screen**. Every
/// coordinate below is one, measured off `rip_retro.py`'s own reading of the frame plane -
/// the boxes the game left empty are holes in it, and their rects are exact.
///
/// The two 32x48 boxes under `NEXT`. Mean Bean Machine fills the left one with the player's
/// next pair and the right one with the opponent's, but a panel here belongs to one player,
/// so the queue runs left to right through both: next, then next but one.
const NEXT_BOXES: [(i32, i32); 2] = [(120, 32), (168, 32)];
/// where the pair sits in one of them - centred across, and low, which is where the game
/// draws it
const NEXT_PAIR: (i32, i32) = (8, 12);

/// the box the game keeps Robotnik's mugshot in, which is the one piece of furniture this
/// panel has no use for and the only hole big enough for the tray
const MUGSHOT: (i32, i32, u32, u32) = (120, 96, 80, 56);

/// where the score goes: the first of the two rows of digits the game keeps under `SCORE`,
/// which is the player's own
const SCORE_AT: (i32, i32) = (120, 176);

/// how long the beans hold before they go. Under [`crate::game::rules::POP_DELAY`], so a
/// chain step never waits on the animation - the same bound the particle theme keeps.
const POP_HOLD: Duration = Duration::from_millis(200);

/// the dungeon wall the boards stand against, read off the board art
const WALL: Color = Color::RGB(0x42, 0x45, 0x00);

fn block(col: i32, row: i32) -> Point {
    Point::new(PAD + PITCH * col, PAD + PITCH * row)
}

/// A colour's sixteen link variants run along its own row, indexed by the mask's bits.
///
/// The skin is ignored, which is the whole difference between a retro theme and the particle
/// one: `PuyoSkin::deal` still hands each player one of fourteen, and Mean Bean Machine drew
/// one set of beans, so every slot is keyed to the same art and both players see the same
/// beans - as they did on a Genesis.
fn puyo(_: PuyoSkin, color: PuyoColor, links: LinkMask) -> Point {
    block(links.bits() as i32, color as i32)
}

pub fn genesis_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
) -> Result<Theme<'a>, String> {
    let options = RetroThemeOptions {
        name: "genesis",
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
        // every row is drawn, the hidden thirteenth included, so `visible_rows` is ROWS and
        // the buffer floats above the frame in `top_padding`
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
        font: FontThemeOptions::simple(
            FontRenderOptions::numeric_sprites(sprites::FONT, texture_creator, 1)?,
            HUD_MAX
                .iter()
                .map(|(kind, max)| (*kind, MetricSnips::zero_fill(SCORE_AT, *max)))
                .collect(),
        ),
        board_file: sprites::BOARD,
        board_alpha: 0xff,
        board_snips: vec![],
        top_padding: TOP_PADDING,
        board_point: Point::new(WELL.0, WELL.1),
        background_file: sprites::BACKGROUND,
        background_color: WALL,
        // Mean Bean Machine ends a match on its cutscenes rather than on a card over the
        // board, and none of those is a board-sized overlay - so the curtain below does the
        // whole of it, exactly as the particle theme's does
        match_end_file: None,
        game_over_points: vec![],
        interstitial_points: vec![],
        overlay_size: None,
        // Tsu has no hold and neither does this game
        hold: None,
        // one pair per box rather than a column of them: the boxes are where the game puts
        // its previews and they are side by side, which no `Column` can say
        peek: PeekLayout::Slots {
            slots: NEXT_BOXES
                .iter()
                .map(|(x, y)| {
                    Rect::new(
                        x + NEXT_PAIR.0,
                        y + NEXT_PAIR.1,
                        SRC_BLOCK_SIZE,
                        SRC_BLOCK_SIZE * 2,
                    )
                })
                .collect(),
            max_scale: 1.0,
        },
        // the tray, which is the one thing on this panel the Genesis never drew: Mean Bean
        // Machine lands an attack the moment it is sent and has nothing waiting to show. It
        // goes in the mugshot box, filling leftwards from the right edge so a long queue
        // grows towards the board rather than off the panel. Five fit across eighty pixels
        // at the cell size and a sixth would have to be drawn small, so a queue longer than
        // that says five and no more.
        pending: Some(PendingLayout {
            point: Point::new(
                MUGSHOT.0 + MUGSHOT.2 as i32 - SRC_BLOCK_SIZE as i32,
                MUGSHOT.1 + (MUGSHOT.3 as i32 - SRC_BLOCK_SIZE as i32) / 2,
            ),
            step: Point::new(-(SRC_BLOCK_SIZE as i32), 0),
            size: SRC_BLOCK_SIZE,
            max: MUGSHOT.2 / SRC_BLOCK_SIZE,
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

    /// a PNG's width and height are big endian at a fixed offset, which is enough to check a
    /// sheet without decoding one
    fn png_size(bytes: &[u8]) -> (u32, u32) {
        let word = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
        (word(16), word(20))
    }

    /// the sheet is `rip_retro.py`'s and nothing in Rust can check the script - but a sheet
    /// that has drifted from the layout this reads it as would draw the wrong half of a bean
    /// rather than fail
    #[test]
    fn the_sheet_is_the_shape_the_layout_reads_it_as() {
        let (width, height) = png_size(sprites::SPRITES);
        assert_eq!(width, (PITCH * LinkMask::COUNT as i32) as u32);
        assert_eq!(height, (PITCH * (EXTRAS_ROW + 1)) as u32);
    }

    /// the panel has to be exactly as tall as the twelve played rows plus whatever furniture
    /// stands under them, and the well has to start at its top left corner - the thirteenth
    /// row lives in `top_padding` above it and nowhere else
    ///
    /// The row under the well is the point. The sheet keeps the screen as the two planes the
    /// Genesis drew it on and the well's *floor* is on the front one, so a panel cut from the
    /// back plane alone had open well where the floor should be and the last row of beans
    /// looked like it had stopped short. One cell, and it is what the panel is one cell
    /// taller than the well for.
    #[test]
    fn the_well_fills_the_panel_from_its_own_top() {
        let (width, height) = png_size(sprites::BACKGROUND);
        let (board_width, board_height) = png_size(sprites::BOARD);
        assert_eq!(board_width, COLUMNS * SRC_BLOCK_SIZE);
        assert_eq!(board_height, VISIBLE_ROWS * SRC_BLOCK_SIZE);
        assert!(WELL.0 as u32 + board_width <= width);
        assert_eq!(
            WELL.1 as u32 + board_height + SRC_BLOCK_SIZE,
            height,
            "the panel has to carry the well's floor under it"
        );
        assert_eq!(TOP_PADDING, SRC_BLOCK_SIZE);
    }

    /// the boxes are holes in the frame plane and their rects are `rip_retro.py`'s reading of
    /// it, so what this can check is that what goes in them fits and lands on the panel
    #[test]
    fn everything_the_panel_is_told_to_draw_lands_on_it() {
        let (width, height) = png_size(sprites::BACKGROUND);
        let panel = |x: i32, y: i32, w: u32, h: u32| {
            assert!(x >= 0 && y >= 0);
            assert!(x as u32 + w <= width, "{x}+{w} runs off the panel");
            assert!(
                y as u32 + h <= height + TOP_PADDING,
                "{y}+{h} runs off the panel"
            );
        };
        for (x, y) in NEXT_BOXES {
            panel(
                x + NEXT_PAIR.0,
                y + NEXT_PAIR.1,
                SRC_BLOCK_SIZE,
                SRC_BLOCK_SIZE * 2,
            );
        }
        panel(MUGSHOT.0, MUGSHOT.1, MUGSHOT.2, MUGSHOT.3);
        panel(SCORE_AT.0, SCORE_AT.1, 0, 0);
        // and the tray fills the mugshot box across, one whole cell per icon
        assert_eq!(MUGSHOT.2 % SRC_BLOCK_SIZE, 0);
    }

    /// `numeric_sprites` divides the sheet by ten and takes its whole height, so a font that
    /// is not exactly ten cells wide draws sliced digits rather than failing
    #[test]
    fn the_font_is_ten_digits_wide() {
        let (width, _) = png_size(sprites::FONT);
        assert_eq!(width % 10, 0);
    }
}
