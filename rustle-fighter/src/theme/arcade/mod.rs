//! The arcade theme: Super Puzzle Fighter II Turbo's own art, cut by `rustle-fighter/art/`.
//!
//! Everything drawn here is the arcade game's, off the Spriters Resource rips in Alex's drop -
//! the gems, the playfield frame, the NEXT box, the score plate, the brick wall behind the
//! panels. The music is the arcade's QSound, rendered out of the VGM logs; the **effects are
//! the PlayStation port's**, because no arcade effect rip exists and the effects are the half
//! tied to gameplay events anyway.
//!
//! **The panel layout is composed, not measured**, and that is a departure from the house rule
//! worth stating plainly. Every other retro theme here has its geometry measured against the
//! emulated game; this one could not be, so what is below arranges the arcade's own
//! furniture - the frame, the NEXT box, the score plate - rather than reproducing where the
//! arcade puts them. The *board* is exact: the frame's interior is six columns and thirteen
//! rows at sixteen pixels to the cell, straight off the rip.
//!
//! One thing the art settled that the disassembly had already said: the frame's top row is
//! hatched tabs over every column **except column 3**, which is open. That is the Drop Alley,
//! and the frame confirms it independently of the code.
//!
//! **This theme is about nine megabytes**, nearly all of it the seven stage tunes, and every
//! theme in the app is built at startup and kept - so it is the biggest single addition to
//! that bill the compendium has taken. It is left whole: `dr-rustario/build.rs` halves its
//! sheets for the `portmaster` and `browser` builds and this could do the same with its music
//! if either one ever complains, but neither has been measured yet and cutting art nobody has
//! found too heavy would be guessing.

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
    /// the score face, the one the plate's own baked zeros are set in
    pub const FONT: &[u8] = include_bytes!("font.png");
}

/// The arcade's QSound, rendered out of the VGM logs by `art/music.py`.
///
/// One stage theme per fighter we field. Puzzle Fighter plays the *opponent's* stage theme,
/// which is not something the engine's music dealing can express - it deals one when a match
/// opens - so these are simply the pool a match is dealt from. Each is a pair: the intro, and
/// then the part that repeats, split at the loop point the VGM header carries.
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

    /// the character select screen's tune, which this game plays over its menus
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

/// how many sprites `art/rip.py` writes per colour: the plain gem, the crash gem, the nine
/// power gem masks and the ten counter gem digits
#[cfg(test)]
const SPRITES_PER_COLOR: i32 = 21;

/// The transparent row above the panel, which is the row a pair spawns in.
///
/// The frame is thirteen rows tall and the board is fourteen, the extra being the headroom the
/// erase pass reaches into - so it is drawn above the frame's mouth, against the brick wall,
/// exactly as Puyo Rusto draws its ghost row.
const TOP_PADDING: u32 = SRC_BLOCK_SIZE * HIDDEN_ROWS;
const BOTTOM_PADDING: u32 = 6;

/// where the board sits inside the panel: three pixels of the frame's wall to its left, and
/// its top row level with the frame's own
const BOARD: (i32, i32) = (3, 0);

/// **Every point below is in the *padded* background's coordinates**, which is the panel art
/// shifted down by [`TOP_PADDING`] - the band the headroom row is drawn in. The panel's own
/// top left is therefore `(0, TOP_PADDING)`, and a constant measured off the art has the
/// padding added to it here rather than at the point of use.
const fn padded(y: i32) -> i32 {
    y + TOP_PADDING as i32
}

/// where the pair waits: centred in the NEXT box's black interior, which the art puts at
/// x 110-135, y 12-55
const NEXT_PAIR: (i32, i32) = (115, padded(17));

/// The score plate's digit bed, which `rip.py` paints the plate's own zeros out of. The plate
/// sits at (106, 70) in the art and its zeros run from (2, 15) inside it.
const SCORE_AT: (i32, i32) = (108, padded(85));
/// The speed step, right aligned under the plate.
///
/// Under it rather than on it: the plate has one row of digits and it belongs to the score,
/// which is the number this game's own HUD gives it.
const LEVEL_AT: (i32, i32) = (165, padded(110));

/// Where the counter gems still to fall are shown: down the right hand column, under the
/// score plate.
///
/// The arcade shows this as a CAUTION / WARNING / DANGER plate rather than as a count, and
/// those plates are on the sheet - but the engine's widget is a row of icons and it says the
/// same thing more precisely, so the plates are left for the fighter layer to use.
const TRAY: (i32, i32) = (112, padded(130));
const TRAY_ICON: u32 = SRC_BLOCK_SIZE * 3 / 4;
const TRAY_MAX: u32 = 6;

/// the brick wall's own colour, which is what shows where the tile does not reach
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
        // the rainbow has no art of its own on the gem sheets - it is drawn as the crash gem
        // of the first colour until one is cut for it
        GemSprite::Rainbow => (0, 1),
    };
    Point::new(column * PITCH + PAD, row * PITCH + PAD)
}

/// The nine masks in the order `art/rip.py` writes them, which is `POWER_MASKS` there.
///
/// A mask that is not one of the nine cannot occur on a board - see
/// [`PowerMask::REACHABLE`] - so it falls back to the fully joined middle, which is the one
/// that looks least wrong anywhere.
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
        // every row is drawn, the headroom fourteenth included
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
        // Puzzle Fighter has no hold box, and neither does this
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

    /// the sheet is `art/rip.py`'s and nothing in Rust can check the script - but a sheet that
    /// had drifted from the layout this reads it as would draw the wrong gem rather than fail
    #[test]
    fn the_gem_sheet_is_the_shape_the_layout_reads_it_as() {
        let (width, height) = png_size(sprites::GEMS);
        assert_eq!(width, (PITCH * SPRITES_PER_COLOR) as u32);
        assert_eq!(
            height,
            (PITCH * crate::game::cell::GemColor::N as i32) as u32
        );
    }

    /// the board's own backdrop is exactly the playfield: six columns and thirteen rows, which
    /// is the frame's interior off the rip
    #[test]
    fn the_board_backdrop_is_the_frames_interior() {
        let (width, height) = png_size(sprites::BOARD);
        assert_eq!(width, COLUMNS * SRC_BLOCK_SIZE);
        assert_eq!(height, (ROWS - HIDDEN_ROWS) * SRC_BLOCK_SIZE);
    }

    /// every mask a rectangle can produce has a column of its own, and no two share one
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

    /// **The theme and the script agree about which cell is which sprite.**
    ///
    /// Nothing in Rust can check `art/rip.py`, and a sheet that had drifted from the layout
    /// this module reads it as would draw the wrong gem rather than fail - so the sheet is
    /// decoded here and every sprite the game can report is looked up in it. A cell that came
    /// out empty is a cut that landed on the sheet's background; two sprites that came out
    /// identical are two cuts that landed on the same art.
    #[test]
    fn every_sprite_lands_on_art_of_its_own() {
        let sheet = image::load_from_memory(sprites::GEMS)
            .expect("the gem sheet decodes")
            .to_rgba8();
        let mut seen: std::collections::HashMap<Vec<u8>, GemSprite> =
            std::collections::HashMap::new();
        for sprite in GemSprite::all() {
            // the rainbow has no art of its own yet and deliberately shares the crash gem's
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
