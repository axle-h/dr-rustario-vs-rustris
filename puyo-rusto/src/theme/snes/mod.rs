//! The SNES theme: Kirby's Avalanche. The blobs are from the "Blobs & Boulders" rip; the panel
//! is rendered from the game's background layers by `puyo-rusto/art/rip_retro.py`.
//!
//! Positions are measured from the SNES game in an emulator at 256x224, in its own pixels.

use crate::game::board::{COLUMNS, HIDDEN_ROWS, ROWS, VISIBLE_ROWS};
use crate::game::cell::{LinkMask, PuyoCell, PuyoColor, PuyoSkin};
use crate::game::rules::{MAX_LEVEL, MAX_SCORE};
use crate::theme::data::{
    audio, cells, hud, panel_shadow, previews, MusicTrack, Sounds, CLEAR_CLASSES, SNES_GAIN,
};
use engine::animate::destroy::DestroyStyle;
use engine::animate::frames::FrameAnimationType;
use engine::animate::game_over::GameOverStyle;
use engine::animate::PopDebris;
use engine::config::Config;
use engine::game::CellId;
use engine::render::animation::AnimationSpriteSheetData;
use engine::render::character::CharacterLayout;
use engine::render::font::{FontRenderOptions, FontThemeOptions, MetricSnips};
use engine::render::geometry::BoardGeometry;
use engine::render::retro::{retro_theme, RetroThemeOptions};
use engine::render::scene::SceneType;
use engine::render::sprite_sheet::{BlockSpriteSheetData, CellAnimationData, GhostStyle};
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
    /// the wash the panels stand on
    pub const SCENE: &[u8] = include_bytes!("scene.png");
    /// every strip that plays over a cell, one per row
    pub const ANIMATIONS: &[u8] = include_bytes!("animations.png");
    pub const FONT: &[u8] = include_bytes!("font.png");
}

/// Kirby's Avalanche's sound, cut by `puyo-rusto/art/retro_audio.py snes`: music from SPC
/// dumps, effects from the game's sound test.
///
/// Each tune's intro is only the moment the echo buffer fills, so the repeat carries the echo.
mod sound {
    pub const MOVE: &[u8] = include_bytes!("move.ogg");
    pub const ROTATE: &[u8] = include_bytes!("rotate.ogg");
    /// also `Sounds::settle`, since the game plays one sound for both
    pub const LOCK: &[u8] = include_bytes!("lock.ogg");
    pub const HARD_DROP: &[u8] = include_bytes!("hard-drop.ogg");
    pub const POP: [&[u8]; super::CLEAR_CLASSES] = [
        include_bytes!("pop-1.ogg"),
        include_bytes!("pop-2.ogg"),
        include_bytes!("pop-3.ogg"),
        include_bytes!("pop-4.ogg"),
    ];
    pub const ATTACK: &[u8] = include_bytes!("attack.ogg");
    /// centred, since the mixer pans effects itself
    pub const GARBAGE: &[u8] = include_bytes!("garbage.ogg");
    pub const SPEED_UP: &[u8] = include_bytes!("speed-up.ogg");
    pub const PAUSE: &[u8] = include_bytes!("pause.ogg");

    pub const STAGE_1: (&[u8], &[u8]) = (
        include_bytes!("stage-1-intro.ogg"),
        include_bytes!("stage-1-repeat.ogg"),
    );
    pub const STAGE_2: (&[u8], &[u8]) = (
        include_bytes!("stage-2-intro.ogg"),
        include_bytes!("stage-2-repeat.ogg"),
    );
    pub const STAGE_3: (&[u8], &[u8]) = (
        include_bytes!("stage-3-intro.ogg"),
        include_bytes!("stage-3-repeat.ogg"),
    );

    /// the dump's two win SPCs, joined
    pub const VICTORY: &[u8] = include_bytes!("victory.ogg");
    pub const GAME_OVER: &[u8] = include_bytes!("game-over.ogg");
}

/// the tracks a match on this theme may be dealt, in the order the dump numbers them
pub const GAME_MUSIC: [MusicTrack; 3] = [
    (Some(sound::STAGE_1.0), sound::STAGE_1.1),
    (Some(sound::STAGE_2.0), sound::STAGE_2.1),
    (Some(sound::STAGE_3.0), sound::STAGE_3.1),
];

mod kirby;

/// the SNES blob, and `rip_retro.py`'s grid
pub const SRC_BLOCK_SIZE: u32 = 16;
const PAD: i32 = 4;
const PITCH: i32 = SRC_BLOCK_SIZE as i32 + 2 * PAD;

/// the row under the five colours, holding the boulder and the tray's three symbols
const EXTRAS_ROW: i32 = PuyoColor::N as i32;

/// Where the game's field sits in the panel. The panel is the SNES screen cut off at the
/// second player's field, so a point here is a point on the SNES screen.
const FIELD: (i32, i32) = (8, 16);

/// The transparent spawning row above the field, drawn over the scene with no panel behind it.
const TOP_PADDING: u32 = SRC_BLOCK_SIZE * HIDDEN_ROWS;

/// Transparent rows under the panel so its bottom edge stands clear of the window. The panel
/// stops short of the screen's last row, which is the console's border (`SNES_SCREEN_BOTTOM`).
const BOTTOM_PADDING: u32 = 8;

/// The two boxes between the wooden posts under `NEXT`: next, then next but one.
const NEXT_BOXES: [(i32, i32, u32, u32); 2] = [(108, 32, 16, 47), (130, 32, 18, 47)];

/// The recess under `STAGE` (`rip_retro.py`'s `SNES_STAGE_NUMBER`), which the level is
/// printed in.
const STAGE_BOX: (i32, i32, u32, u32) = (120, 103, 16, 16);

/// The plank across the mouth of the arch, which the tray stands on.
const ARCH_MOUTH: (i32, i32, u32, u32) = (104, 192, 48, 16);
/// Tray icon size and pitch: three quarters of a cell, the largest at which [`TRAY_MAX`] icons
/// fill the plank.
const TRAY_ICON: u32 = SRC_BLOCK_SIZE * 3 / 4;
const TRAY_STEP: u32 = TRAY_ICON;
/// as many as stand on the plank
const TRAY_MAX: u32 = ARCH_MOUTH.2 / TRAY_ICON;

/// A digit of the game's font is two 8x8 tiles stacked, on an eight pixel pitch with no gap.
#[cfg(test)]
const FONT_HEIGHT: u32 = 16;
#[cfg(test)]
const FONT_WIDTH: u32 = 8;

/// where the game right aligns its own score, and the cell it prints it in
const SCORE_AT: (i32, i32) = (104, 207);
/// where the level is right aligned, in the stage recess
const LEVEL_AT: (i32, i32) = (STAGE_BOX.0 + STAGE_BOX.2 as i32, STAGE_BOX.1);

/// How long a blob takes to go. It adds to [`crate::game::rules::POP_DELAY`], since the match
/// skips `game.update` while an animation blocks. This and the two below are the genesis
/// theme's beats shortened, not measurements of Kirby's Avalanche.
const POP_HOLD: Duration = Duration::from_millis(260);

/// how long the blob holds its surprised face before it curls into a ball
const POP_FACE_HOLD: Duration = Duration::from_millis(140);

/// The group flashes where it stands, starting lit, before the pop strip plays.
const POP_BLINK: Duration = Duration::from_millis(200);
const POP_BLINKS: u32 = 2;

/// The frames of one pop: a face, then a shrinking ball. It is the widest strip, so it sets
/// the animation sheet's width.
const POP_FRAMES: usize = 3;

/// The squash a blob plays where it lands. Neither frame is linked to its neighbours, as in
/// the original.
const BOUNCE_FRAMES: usize = 2;

/// one spark, thrown several times over by [`POP_DEBRIS`]
const DEBRIS_FRAMES: usize = 1;

/// The sparks a blob throws on its last pop frame, which outlive the clear.
const POP_DEBRIS: PopDebris = PopDebris {
    at_frame: POP_FRAMES - 1,
    pieces: 4,
    // drawn on the window, not the board, so a harder throw lands on the flower border
    speed: (2.0, 5.0),
    gravity: 16.0,
    life: Duration::from_millis(380),
    // of the whole cell; the spark is four pixels of sixteen, so this draws it about a third
    // of a block
    size: 1.5,
};

/// The scene behind the panels: the canopy's colour at three quarters brightness, untextured
/// so the spawning row does not read as part of the panel.
const FOREST: Color = Color::RGB(0x00, 0x15, 0x00);

fn block(col: i32, row: i32) -> Point {
    Point::new(PAD + PITCH * col, PAD + PITCH * row)
}

/// a colour's sixteen link variants run along its own row, indexed by the mask's bits. The
/// skin is ignored: Kirby's Avalanche drew one set of blobs, so both players see the same.
fn puyo(_: PuyoSkin, color: PuyoColor, links: LinkMask) -> Point {
    block(links.bits() as i32, color as i32)
}

/// Space between animation sheet rows. Frames in a strip have none, since the engine counts
/// frame widths from the strip's start.
const ANIM_ROW_GAP: u32 = 4;

/// Where each strip sits on the sheet, in rows, in `rip_retro.py`'s order;
/// `every_strip_is_where_the_theme_counts_it`.
const POP_ROW: u32 = 0;
const NUISANCE_POP_ROW: u32 = PuyoColor::N as u32;
const BOUNCE_ROW: u32 = NUISANCE_POP_ROW + 1;
const DEBRIS_ROW: u32 = BOUNCE_ROW + PuyoColor::N as u32;
const NUISANCE_DEBRIS_ROW: u32 = DEBRIS_ROW + PuyoColor::N as u32;

/// how many rows the sheet has altogether
const ANIM_ROWS: u32 = NUISANCE_DEBRIS_ROW + 1;

fn strip(row: u32, frames: u32) -> AnimationSpriteSheetData {
    debug_assert!(row < ANIM_ROWS, "the sheet has no row {row}");
    AnimationSpriteSheetData::non_exclusive_linear(
        sprites::ANIMATIONS,
        Point::new(0, ((SRC_BLOCK_SIZE + ANIM_ROW_GAP) * row) as i32),
        frames,
        SRC_BLOCK_SIZE,
        SRC_BLOCK_SIZE,
    )
}

/// What plays over a cell, keyed for every link mask and skin of each colour. The boulder has
/// no bounce or idle, since the rip has no art for either.
fn animations() -> Vec<(Vec<CellId>, CellAnimationData)> {
    let mut out = vec![];
    for (row, color) in PuyoColor::ALL.into_iter().enumerate() {
        let ids = PuyoSkin::all()
            .flat_map(|skin| {
                (0..LinkMask::COUNT as u8)
                    .map(move |bits| PuyoCell::puyo(color, LinkMask::from_bits(bits)).id(skin))
            })
            .collect();
        out.push((
            ids,
            CellAnimationData {
                pop: Some(strip(POP_ROW + row as u32, POP_FRAMES as u32)),
                bounce: Some(strip(BOUNCE_ROW + row as u32, BOUNCE_FRAMES as u32)),
                debris: Some(strip(DEBRIS_ROW + row as u32, DEBRIS_FRAMES as u32)),
                ..Default::default()
            },
        ));
    }
    let nuisance = PuyoSkin::all()
        .map(|skin| PuyoCell::Nuisance.id(skin))
        .collect();
    out.push((
        nuisance,
        CellAnimationData {
            pop: Some(strip(NUISANCE_POP_ROW, POP_FRAMES as u32)),
            debris: Some(strip(NUISANCE_DEBRIS_ROW, DEBRIS_FRAMES as u32)),
            ..Default::default()
        },
    ));
    out
}

pub fn snes_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
) -> Result<Theme<'a>, String> {
    let options = RetroThemeOptions {
        name: "snes",
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
            animations: animations(),
            ghost_alpha: 0x60,
            previews: previews(),
            mascot: None,
        },
        geometry: BoardGeometry::new(SRC_BLOCK_SIZE, 0, (0, 0), COLUMNS, ROWS, ROWS),
        audio: audio(
            config.audio,
            Sounds {
                gain: SNES_GAIN,
                music: &GAME_MUSIC,
                move_pair: sound::MOVE,
                rotate: sound::ROTATE,
                lock: sound::LOCK,
                settle: sound::LOCK,
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
            FontRenderOptions::numeric_sprites(sprites::FONT, texture_creator, 0)?,
            hud(
                MetricSnips::right(SCORE_AT, MAX_SCORE),
                MetricSnips::right(LEVEL_AT, MAX_LEVEL),
            ),
        ),
        board_file: sprites::BOARD,
        board_alpha: 0xff,
        board_snips: vec![],
        top_padding: TOP_PADDING,
        bottom_padding: BOTTOM_PADDING,
        shadow: Some(panel_shadow((0, TOP_PADDING, 0, BOTTOM_PADDING))),
        board_point: Point::new(FIELD.0, 0),
        background_file: sprites::BACKGROUND,
        background_color: FOREST,
        match_end_file: None,
        game_over_points: vec![],
        interstitial_points: vec![],
        overlay_size: None,
        hold: None,
        peek: PeekLayout::Slots {
            slots: NEXT_BOXES
                .iter()
                .map(|(x, y, w, h)| {
                    Rect::from_center(
                        Point::new(x + *w as i32 / 2, y + *h as i32 / 2),
                        SRC_BLOCK_SIZE,
                        SRC_BLOCK_SIZE * 2,
                    )
                })
                .collect(),
            max_scale: 1.0,
        },
        pending: Some(PendingLayout {
            point: Point::new(
                ARCH_MOUTH.0
                    + (ARCH_MOUTH.2 as i32 - (TRAY_STEP * (TRAY_MAX - 1) + TRAY_ICON) as i32) / 2,
                ARCH_MOUTH.1 + (ARCH_MOUTH.3 as i32 - TRAY_ICON as i32) / 2,
            ),
            step: Point::new(TRAY_STEP as i32, 0),
            size: TRAY_ICON,
            max: TRAY_MAX,
        }),
        // the player's own Kirby, in the arch, declared as routines rather than strips
        characters: Some((
            kirby::cast(),
            CharacterLayout {
                rect: Rect::new(kirby::BOX.0, kirby::BOX.1, kirby::BOX.2, kirby::BOX.3),
            },
        )),
        mascot: None,
        mascot_animations: None,
        spawn_arc: None,
        // no cell declares an idle strip
        cell_idle_type: FrameAnimationType::Static,
        destroy_style: Some(
            DestroyStyle::pop(POP_FRAMES)
                .for_duration(POP_HOLD)
                .blinking_for(POP_BLINK, POP_BLINKS)
                .holding_first(POP_FACE_HOLD),
        ),
        game_over_style: Some(GameOverStyle::drain(VISIBLE_ROWS)),
        curtain_cell: None,
        ghost_style: GhostStyle::Alpha,
        hard_drop_rows_per_frame: engine::animate::hard_drop::DEFAULT_ROWS_PER_FRAME,
        pop_debris: Some(POP_DEBRIS),
        nuisance_rumble: None,
        // no ball art, so `Theme::draw_attack_ball` falls back to the popped blob's cell
        attack_ball: None,
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

    /// the console's border row, which the panel stops short of
    const SCREEN_BORDER_ROW: u32 = 1;

    /// The board png fits the panel's field hole, with the panel starting at the field's top.
    #[test]
    fn the_field_fits_the_hole_it_is_drawn_into() {
        let (width, height) = png_size(sprites::BACKGROUND);
        let (board_width, board_height) = png_size(sprites::BOARD);
        assert_eq!(board_width, COLUMNS * SRC_BLOCK_SIZE);
        assert_eq!(board_height, VISIBLE_ROWS * SRC_BLOCK_SIZE);
        assert!(FIELD.0 as u32 + board_width <= width);
        assert_eq!(FIELD.1 as u32, SRC_BLOCK_SIZE);
        assert_eq!(
            board_height + SRC_BLOCK_SIZE - SCREEN_BORDER_ROW,
            height,
            "the panel is the field and the border under it, less the screen's own last row"
        );
        assert_eq!(TOP_PADDING, SRC_BLOCK_SIZE * HIDDEN_ROWS);
    }

    /// the next boxes, stage recess and tray plank fit their contents and lie on the panel
    #[test]
    fn everything_the_panel_is_told_to_draw_lands_on_it() {
        let (width, panel_height) = png_size(sprites::BACKGROUND);
        // the rects are in the padded background, so they fit the panel plus the spawning row
        let height = panel_height + TOP_PADDING;
        for (x, y, w, h) in NEXT_BOXES {
            assert!(x as u32 + w <= width, "a next box runs off the panel");
            assert!(y as u32 + h <= height);
            assert!(
                w >= SRC_BLOCK_SIZE && h >= SRC_BLOCK_SIZE * 2,
                "a pair does not fit"
            );
        }
        assert!(STAGE_BOX.2 >= FONT_WIDTH && STAGE_BOX.3 >= FONT_HEIGHT);
        assert!(STAGE_BOX.0 as u32 + STAGE_BOX.2 <= width);
        assert!(STAGE_BOX.1 as u32 + STAGE_BOX.3 <= height);
        assert!(ARCH_MOUTH.0 as u32 + ARCH_MOUTH.2 <= width);
        assert!(ARCH_MOUTH.1 as u32 + ARCH_MOUTH.3 <= height);
        // the last icon reaches past its slot, so the whole tray has to fit the plank
        assert!(TRAY_STEP * (TRAY_MAX - 1) + TRAY_ICON <= ARCH_MOUTH.2);
    }

    /// The animation png is the size the strip rows add up to, and every cell claims a strip.
    #[test]
    fn every_strip_is_where_the_theme_counts_it() {
        let (width, height) = png_size(sprites::ANIMATIONS);
        assert_eq!(width, SRC_BLOCK_SIZE * POP_FRAMES as u32);
        assert_eq!(height, (SRC_BLOCK_SIZE + ANIM_ROW_GAP) * ANIM_ROWS);
        const {
            assert!(
                BOUNCE_FRAMES <= POP_FRAMES
                    && DEBRIS_FRAMES <= POP_FRAMES
                    && POP_DEBRIS.at_frame < POP_FRAMES,
                "every other strip shares the sheet's width with the pop, which is its widest"
            );
        }
        let strips = animations();
        assert_eq!(strips.len(), PuyoColor::N + 1);
        let keyed: usize = strips.iter().map(|(ids, _)| ids.len()).sum();
        assert_eq!(
            keyed,
            PuyoSkin::COUNT * (PuyoColor::N * LinkMask::COUNT + 1),
            "every blob and every boulder, in every skin slot"
        );
    }

    #[test]
    fn the_font_is_ten_digits_wide() {
        let (width, _) = png_size(sprites::FONT);
        assert_eq!(width % 10, 0);
    }

    /// Every sound and tune this theme embeds decodes.
    #[test]
    fn every_sound_this_theme_owns_decodes() {
        let mut sounds = vec![
            sound::VICTORY,
            sound::GAME_OVER,
            sound::MOVE,
            sound::ROTATE,
            sound::LOCK,
            sound::HARD_DROP,
            sound::ATTACK,
            sound::GARBAGE,
            sound::SPEED_UP,
            sound::PAUSE,
        ];
        sounds.extend(sound::POP);
        for (intro, repeat) in GAME_MUSIC {
            sounds.extend(intro);
            sounds.push(repeat);
        }
        for bytes in sounds {
            engine::audio::Sound::load(bytes, 100).expect("a snes sound did not decode");
        }
    }
}
