//! The 3DS theme: Puyo Puyo Chronicle, cut out of the spriters-resource rips by
//! `puyo-rusto/art/rip_retro.py`.
//!
//! The odd one out of the three retro themes, and the one that is not retro by any ordinary
//! reading of the word - what makes it one here is [`engine::render::ThemeFamily`], which
//! sorts art-based themes from the particle one and knows nothing about the calendar. What it
//! is, is the only one of the four themes that is actually *Puyo Puyo*: the other two retro
//! themes are Compile's game with somebody else's cast painted over it.
//!
//! Chronicle draws at a size nothing else here does, so two things are scaled rather than
//! taken as they were cut. The field frame's interior is 100 by 180 in the rip where twelve
//! rows of an eighteen pixel puyo is 216, so the frame is scaled up uniformly until its
//! interior is exactly twelve rows tall and the six columns sit centred in the width that
//! leaves; and the background is one of the game's own 3DS top screens, scaled to cover a
//! player's panel and cropped from the middle of it.

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
    pub const BOARD: &[u8] = include_bytes!("board.png");
    pub const FONT: &[u8] = include_bytes!("font.png");
    /// one of the game's own top screens, blown up by `rip_retro.py` to stand being
    /// stretched over a whole window - see `THREE_DS_BG_SCALE` there
    pub const SCENE: &[u8] = include_bytes!("scene.png");
}

/// the rip's own cell, squared off from its 18 by 17 by `rip_retro.py`
pub const SRC_BLOCK_SIZE: u32 = 18;
const PAD: i32 = 4;
const PITCH: i32 = SRC_BLOCK_SIZE as i32 + 2 * PAD;

/// the row under the five colours, holding the nuisance puyo and the tray's three symbols
const EXTRAS_ROW: i32 = PuyoColor::N as i32;

/// one transparent cell above everything, for the thirteenth row to float in
const TOP_PADDING: u32 = SRC_BLOCK_SIZE * HIDDEN_ROWS;

/// the field frame, four of them side by side in `board.png` - one per speed band, so the
/// field changes colour as the pairs come down faster
const FRAME: (u32, u32) = (134, 230);
const FRAMES: u32 = 4;

/// where the six by twelve grid starts inside a frame, which is its border plus the slack the
/// columns are centred in. Both are `rip_retro.py`'s and printed by it.
const CELLS_AT: (i32, i32) = (13, 7);

/// where the frame is cut out of the panel, and so where the board is drawn back into it
const FRAME_AT: (i32, i32) = (7, 11);

/// The column beside the field, which is everything the frame leaves. The queue and the
/// nuisance tray both live in it, against the field rather than centred across it: in a two
/// player game the far side of this column is the gap between the boards, and anything out
/// there reads as belonging to neither player. Chronicle floats both beside the field with
/// no furniture round either - there is none in the rip to give them - so the only things to
/// get right are that they hug the field and start level with its first row.
const SIDE: (i32, u32) = (
    FRAME_AT.0 + FRAME.0 as i32,
    PANEL.0 - FRAME_AT.0 as u32 - FRAME.0,
);
/// the air between the field's frame and the column's contents
const SIDE_GAP: i32 = 6;
/// the first row of the field, which the queue starts level with
const FIRST_ROW: i32 = FRAME_AT.1 + CELLS_AT.1;
/// pair to pair down the queue: a pair, and a little air
const PEEK_STEP: i32 = SRC_BLOCK_SIZE as i32 * 2 + 8;
/// the tray goes under the queue, at a little under a cell so all six fit the column
const TRAY_ICON: u32 = 15;

/// one player's panel, which is `rip_retro.py`'s `THREE_DS_PANEL` - transparent, since the
/// scene is the whole of the backdrop
const PANEL: (u32, u32) = (240, 272);

/// how long a group holds before it goes, under [`crate::game::rules::POP_DELAY`] so a chain
/// step never waits on the animation
const POP_HOLD: Duration = Duration::from_millis(200);

/// the dark the field is printed on, averaged off the fill itself. Nothing is drawn in it
/// now that the scene is a picture; it is what the window would be were the scene missing.
const FIELD: Color = Color::RGB(0x27, 0x3A, 0x56);

fn block(col: i32, row: i32) -> Point {
    Point::new(PAD + PITCH * col, PAD + PITCH * row)
}

/// a colour's sixteen link variants run along its own row, indexed by the mask's bits. The
/// skin is ignored: Chronicle's Gummy set is one set of puyos, so both players draw the same.
fn puyo(_: PuyoSkin, color: PuyoColor, links: LinkMask) -> Point {
    block(links.bits() as i32, color as i32)
}

pub fn three_ds_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
) -> Result<Theme<'a>, String> {
    let options = RetroThemeOptions {
        name: "3ds",
        // Chronicle stands both fields on one painted scene rather than giving each player a
        // panel, and so does this: the panel is transparent and the backdrop is the picture,
        // covering the window however big it is. It is the one theme in the repository whose
        // background is a painting rather than a tile, which is why `Cover` exists.
        scenes: vec![SceneType::Cover {
            texture: sprites::SCENE,
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
        // the grid starts inside the frame's border; the thirteenth row is the one drawn
        // above it, in the transparent cell `top_padding` leaves
        geometry: BoardGeometry::new(SRC_BLOCK_SIZE, 0, CELLS_AT, COLUMNS, ROWS, ROWS),
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
                .map(|(kind, max)| (*kind, MetricSnips::zero_fill((14, 258), *max)))
                .collect(),
        ),
        board_file: sprites::BOARD,
        board_alpha: 0xff,
        // the snips are into the *padded* board texture, so they are a whole cell taller
        // than the frame art: cut them at the frame's own height and the bottom row of the
        // board is left outside the copy, which is exactly what it looks like
        board_snips: (0..FRAMES)
            .map(|i| Rect::new((FRAME.0 * i) as i32, 0, FRAME.0, FRAME.1 + TOP_PADDING))
            .collect(),
        top_padding: TOP_PADDING,
        board_point: Point::new(FRAME_AT.0, FRAME_AT.1),
        background_file: sprites::BACKGROUND,
        background_color: FIELD,
        match_end_file: None,
        game_over_points: vec![],
        interstitial_points: vec![],
        overlay_size: None,
        hold: None,
        peek: PeekLayout::Column {
            point: Point::new(SIDE.0 + SIDE_GAP, FIRST_ROW + TOP_PADDING as i32),
            offset: PEEK_STEP,
            max: 2,
            scale: None,
        },
        // ... and the tray under the queue, filling rightwards
        pending: Some(PendingLayout {
            point: Point::new(
                SIDE.0 + SIDE_GAP,
                FIRST_ROW + TOP_PADDING as i32 + PEEK_STEP * 2 + 8,
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

    /// four frames side by side, and the six by twelve grid inside each one
    #[test]
    fn every_speed_band_has_a_frame_with_the_board_inside_it() {
        let (width, height) = png_size(sprites::BOARD);
        assert_eq!(width, FRAME.0 * FRAMES);
        assert_eq!(height, FRAME.1);
        assert!(CELLS_AT.0 as u32 + COLUMNS * SRC_BLOCK_SIZE <= FRAME.0);
        assert!(CELLS_AT.1 as u32 + VISIBLE_ROWS * SRC_BLOCK_SIZE <= FRAME.1);
    }

    /// the frame is drawn back into the hole the panel was cut with, so the two have to agree
    #[test]
    fn the_frame_fits_the_hole_it_is_drawn_into() {
        let (width, height) = png_size(sprites::BACKGROUND);
        assert!(FRAME_AT.0 as u32 + FRAME.0 <= width);
        assert!(FRAME_AT.1 as u32 + FRAME.1 <= height);
    }

    #[test]
    fn the_font_is_ten_digits_wide() {
        let (width, _) = png_size(sprites::FONT);
        assert_eq!(width % 10, 0);
    }
}
