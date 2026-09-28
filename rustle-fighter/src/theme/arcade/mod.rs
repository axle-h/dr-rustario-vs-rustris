//! The arcade theme: Super Puzzle Fighter II Turbo's own art, cut by `rustle-fighter/art/`,
//! with the arcade's music and the PlayStation port's effects.
//!
//! The board is exact, six columns by thirteen rows at sixteen pixels off the frame, but the
//! panel layout around it is composed rather than measured against the emulated game.

use crate::game::board::{COLUMNS, HIDDEN_ROWS, ROWS};
use crate::game::cell::{GemSprite, PowerMask};
use crate::game::rules::{MAX_LEVEL, MAX_SCORE};
use crate::theme::data::{
    audio, cells, panel_shadow, previews, MusicTrack, Sounds, ARCADE_GAIN, CLEAR_CLASSES,
};
use engine::animate::game_over::GameOverStyle;
use engine::config::Config;
use engine::game::MetricKind;
use engine::render::font::{FontRenderOptions, FontThemeOptions, MetricSnips, ThemedNumeric};
use engine::render::geometry::BoardGeometry;
use engine::render::retro::{retro_theme, RetroThemeOptions};
use engine::render::scene::SceneType;
use engine::render::sprite_sheet::{BlockSpriteSheetData, GhostStyle};
use engine::render::{PeekLayout, PendingLayout, Theme};
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;

mod sprites {
    pub const GEMS: &[u8] = include_bytes!("gems.png");
    pub const BACKGROUND: &[u8] = include_bytes!("background.png");
    pub const BOARD: &[u8] = include_bytes!("board.png");
    pub const SCENE: &[u8] = include_bytes!("scene.png");
    /// the score face the plate's baked zeros are set in
    pub const FONT: &[u8] = include_bytes!("font.png");
}

/// Rendered by `art/music.py`; each stage track is an intro and a repeat, split at the VGM
/// loop point, and a match is dealt one from the pool.
mod sound {
    pub const MOVE: &[u8] = include_bytes!("move.ogg");
    pub const ROTATE: &[u8] = include_bytes!("rotate.ogg");
    pub const LOCK: &[u8] = include_bytes!("lock.ogg");
    pub const SETTLE: &[u8] = include_bytes!("settle.ogg");
    pub const HARD_DROP: &[u8] = include_bytes!("hard-drop.ogg");
    pub const POP: [&[u8]; super::CLEAR_CLASSES] = [
        include_bytes!("pop-1.ogg"),
        include_bytes!("pop-2.ogg"),
        include_bytes!("pop-3.ogg"),
        include_bytes!("pop-4.ogg"),
    ];
    pub const ATTACK: &[u8] = include_bytes!("attack.ogg");
    pub const GARBAGE: &[u8] = include_bytes!("garbage.ogg");
    pub const SPEED_UP: &[u8] = include_bytes!("speed-up.ogg");
    pub const PAUSE: &[u8] = include_bytes!("pause.ogg");
    pub const VICTORY: &[u8] = include_bytes!("victory.ogg");
    pub const GAME_OVER: &[u8] = include_bytes!("game-over.ogg");

    /// the character select tune, played over the menus
    pub const MENU: (&[u8], &[u8]) = (
        include_bytes!("menu-intro.ogg"),
        include_bytes!("menu-repeat.ogg"),
    );

    pub const STAGES: [(&[u8], &[u8]); 7] = [
        (
            include_bytes!("stage-morrigan-intro.ogg"),
            include_bytes!("stage-morrigan-repeat.ogg"),
        ),
        (
            include_bytes!("stage-chun-li-intro.ogg"),
            include_bytes!("stage-chun-li-repeat.ogg"),
        ),
        (
            include_bytes!("stage-ryu-intro.ogg"),
            include_bytes!("stage-ryu-repeat.ogg"),
        ),
        (
            include_bytes!("stage-ken-intro.ogg"),
            include_bytes!("stage-ken-repeat.ogg"),
        ),
        (
            include_bytes!("stage-hsien-ko-intro.ogg"),
            include_bytes!("stage-hsien-ko-repeat.ogg"),
        ),
        (
            include_bytes!("stage-felicia-intro.ogg"),
            include_bytes!("stage-felicia-repeat.ogg"),
        ),
        (
            include_bytes!("stage-sakura-intro.ogg"),
            include_bytes!("stage-sakura-repeat.ogg"),
        ),
    ];
}

pub const MENU_MUSIC: (&[u8], &[u8]) = sound::MENU;

pub const GAME_MUSIC: [MusicTrack; 7] = [
    (Some(sound::STAGES[0].0), sound::STAGES[0].1),
    (Some(sound::STAGES[1].0), sound::STAGES[1].1),
    (Some(sound::STAGES[2].0), sound::STAGES[2].1),
    (Some(sound::STAGES[3].0), sound::STAGES[3].1),
    (Some(sound::STAGES[4].0), sound::STAGES[4].1),
    (Some(sound::STAGES[5].0), sound::STAGES[5].1),
    (Some(sound::STAGES[6].0), sound::STAGES[6].1),
];

/// the arcade's own cell, and `rip.py`'s grid
pub const SRC_BLOCK_SIZE: u32 = 16;
const PAD: i32 = 4;
const PITCH: i32 = SRC_BLOCK_SIZE as i32 + 2 * PAD;

/// per colour: plain, crash, nine power gem masks and ten counter digits
#[cfg(test)]
const SPRITES_PER_COLOR: i32 = 21;

/// The transparent band above the frame for the board's fourteenth, headroom row, which the
/// thirteen row frame does not cover.
const TOP_PADDING: u32 = SRC_BLOCK_SIZE * HIDDEN_ROWS;
const BOTTOM_PADDING: u32 = 6;

/// where the board sits inside the panel
const BOARD: (i32, i32) = (3, 0);

/// A y off the panel art in the padded background's coordinates, shifted down by
/// [`TOP_PADDING`]; every point below goes through it.
const fn padded(y: i32) -> i32 {
    y + TOP_PADDING as i32
}

/// centred in the NEXT box's interior, x 110-135 and y 12-55 on the art
const NEXT_PAIR: (i32, i32) = (115, padded(17));

/// the score plate's digit bed, whose baked zeros `rip.py` paints out
const SCORE_AT: (i32, i32) = (108, padded(85));
/// the speed step, right aligned under the plate, whose one row of digits is the score's
const LEVEL_AT: (i32, i32) = (165, padded(110));

/// the pending counter gem tray, under the score plate
const TRAY: (i32, i32) = (112, padded(130));
const TRAY_ICON: u32 = SRC_BLOCK_SIZE * 3 / 4;
const TRAY_MAX: u32 = 6;

/// the brick wall's colour, shown where the tile does not reach
const WALL: Color = Color::RGB(0x3a, 0x18, 0x14);

/// where a sprite sits on `rip.py`'s grid: a row per colour, in `GemColor`'s own order
fn gem(sprite: GemSprite) -> Point {
    let (row, column) = match sprite {
        GemSprite::Plain { color, mask } => {
            let column = if mask == PowerMask::NONE {
                0
            } else {
                2 + power_column(mask)
            };
            (color.index() as i32, column)
        }
        GemSprite::Crash(color) => (color.index() as i32, 1),
        GemSprite::Counter { color, countdown } => {
            (color.index() as i32, 2 + 9 + countdown.min(9) as i32)
        }
        // no rainbow art on the sheets, so the first colour's crash gem stands in
        GemSprite::Rainbow => (0, 1),
    };
    Point::new(column * PITCH + PAD, row * PITCH + PAD)
}

/// The nine masks in `art/rip.py`'s `POWER_MASKS` order; an unreachable mask falls back to
/// the fully joined middle.
fn power_column(mask: PowerMask) -> i32 {
    PowerMask::REACHABLE
        .iter()
        .position(|reachable| *reachable == mask)
        .unwrap_or(4) as i32
}

pub fn arcade_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
) -> Result<Theme<'a>, String> {
    let options = RetroThemeOptions {
        name: "arcade",
        scenes: vec![SceneType::Tile {
            texture: sprites::SCENE,
        }],
        sprites: BlockSpriteSheetData {
            file: sprites::GEMS,
            source_block_size: SRC_BLOCK_SIZE,
            cells: cells(SRC_BLOCK_SIZE, gem),
            animations: vec![],
            ghost_alpha: 0x60,
            previews: previews(),
            mascot: None,
        },
        // the headroom row is drawn too
        geometry: BoardGeometry::new(SRC_BLOCK_SIZE, 0, (0, 0), COLUMNS, ROWS, ROWS),
        audio: audio(
            config.audio,
            Sounds {
                gain: ARCADE_GAIN,
                music: &GAME_MUSIC,
                move_pair: sound::MOVE,
                rotate: sound::ROTATE,
                lock: sound::LOCK,
                settle: sound::SETTLE,
                hard_drop: sound::HARD_DROP,
                pop: sound::POP,
                attack_sent: sound::ATTACK,
                receive_counter: sound::GARBAGE,
                speed_up: sound::SPEED_UP,
                paused: sound::PAUSE,
                victory: sound::VICTORY,
                game_over: sound::GAME_OVER,
            },
        )?,
        font: FontThemeOptions::new(
            vec![FontRenderOptions::numeric_sprites(
                sprites::FONT,
                texture_creator,
                0,
            )?],
            vec![
                (
                    MetricKind::Score,
                    ThemedNumeric::new(0, MetricSnips::zero_fill(SCORE_AT, MAX_SCORE)),
                ),
                (
                    MetricKind::Level,
                    ThemedNumeric::new(0, MetricSnips::right(LEVEL_AT, MAX_LEVEL)),
                ),
            ],
        ),
        board_file: sprites::BOARD,
        board_alpha: 0xff,
        board_snips: vec![],
        top_padding: TOP_PADDING,
        bottom_padding: BOTTOM_PADDING,
        shadow: Some(panel_shadow((0, TOP_PADDING, 0, BOTTOM_PADDING))),
        board_point: Point::new(BOARD.0, BOARD.1),
        background_file: sprites::BACKGROUND,
        background_color: WALL,
        match_end_file: None,
        game_over_points: vec![],
        interstitial_points: vec![],
        overlay_size: None,
        hold: None,
        peek: PeekLayout::Slots {
            slots: vec![Rect::new(
                NEXT_PAIR.0,
                NEXT_PAIR.1,
                SRC_BLOCK_SIZE,
                SRC_BLOCK_SIZE * 2,
            )],
            max_scale: 1.0,
        },
        pending: Some(PendingLayout {
            point: Point::new(TRAY.0, TRAY.1),
            step: Point::new(0, TRAY_ICON as i32),
            size: TRAY_ICON,
            max: TRAY_MAX,
        }),
        attack_ball: None,
        characters: None,
        mascot: None,
        mascot_animations: None,
        spawn_arc: None,
        cell_idle_type: engine::animate::frames::FrameAnimationType::Linear { fps: 0 },
        destroy_style: None,
        game_over_style: Some(GameOverStyle::drain(ROWS)),
        curtain_cell: None,
        ghost_style: GhostStyle::Alpha,
        hard_drop_rows_per_frame: engine::animate::hard_drop::DEFAULT_ROWS_PER_FRAME,
        pop_debris: None,
        nuisance_rumble: None,
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

    /// the gem sheet's dimensions match the grid `gem` reads
    #[test]
    fn the_gem_sheet_is_the_shape_the_layout_reads_it_as() {
        let (width, height) = png_size(sprites::GEMS);
        assert_eq!(width, (PITCH * SPRITES_PER_COLOR) as u32);
        assert_eq!(
            height,
            (PITCH * crate::game::cell::GemColor::N as i32) as u32
        );
    }

    /// the board backdrop is exactly the visible playfield
    #[test]
    fn the_board_backdrop_is_the_frames_interior() {
        let (width, height) = png_size(sprites::BOARD);
        assert_eq!(width, COLUMNS * SRC_BLOCK_SIZE);
        assert_eq!(height, (ROWS - HIDDEN_ROWS) * SRC_BLOCK_SIZE);
    }

    /// every reachable mask has a column of its own
    #[test]
    fn the_nine_power_gem_masks_each_have_their_own_sprite() {
        let columns: std::collections::HashSet<i32> = PowerMask::REACHABLE
            .iter()
            .map(|mask| power_column(*mask))
            .collect();
        assert_eq!(columns.len(), PowerMask::REACHABLE.len());
        assert_eq!(
            columns.into_iter().max(),
            Some(PowerMask::REACHABLE.len() as i32 - 1),
            "and they are the first nine columns of the power gem run"
        );
    }

    /// Every sprite's cell on the decoded sheet is non-empty and unique, so the layout agrees
    /// with `art/rip.py`.
    #[test]
    fn every_sprite_lands_on_art_of_its_own() {
        let sheet = image::load_from_memory(sprites::GEMS)
            .expect("the gem sheet decodes")
            .to_rgba8();
        let mut seen: std::collections::HashMap<Vec<u8>, GemSprite> =
            std::collections::HashMap::new();
        for sprite in GemSprite::all() {
            // the rainbow shares the crash gem's art
            if sprite == GemSprite::Rainbow {
                continue;
            }
            let at = gem(sprite);
            let cell: Vec<u8> = (0..SRC_BLOCK_SIZE)
                .flat_map(|y| (0..SRC_BLOCK_SIZE).map(move |x| (x, y)))
                .flat_map(|(x, y)| {
                    sheet
                        .get_pixel(at.x as u32 + x, at.y as u32 + y)
                        .0
                        .into_iter()
                })
                .collect();
            assert!(
                cell.chunks(4).any(|pixel| pixel[3] > 0),
                "{sprite:?} is cut from an empty part of the sheet"
            );
            if let Some(other) = seen.insert(cell, sprite) {
                panic!("{sprite:?} and {other:?} are cut from the same art");
            }
        }
    }
}
