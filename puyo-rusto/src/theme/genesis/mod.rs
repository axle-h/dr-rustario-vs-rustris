//! The Genesis theme: Dr. Robotnik's Mean Bean Machine, cut by `puyo-rusto/art/rip_retro.py`.
//!
//! Positions are measured from the Genesis game in an emulator at 320x224, in its own pixels
//! with a 16 pixel bean.

use crate::game::board::{COLUMNS, HIDDEN_ROWS, ROWS, VISIBLE_ROWS};
use crate::game::cell::{LinkMask, PuyoCell, PuyoColor, PuyoSkin};
use crate::game::rules::{MAX_LEVEL, MAX_SCORE};
use crate::theme::data::{
    audio, cells, panel_shadow, previews, MusicTrack, Sounds, CLEAR_CLASSES, GENESIS_GAIN,
};
use engine::animate::destroy::DestroyStyle;
use engine::animate::frames::FrameAnimationType;
use engine::animate::game_over::GameOverStyle;
use engine::animate::PopDebris;
use engine::config::Config;
use engine::game::{CellId, MetricKind};
use engine::render::animation::AnimationSpriteSheetData;
use engine::render::character::CharacterLayout;
use engine::render::font::{FontRenderOptions, FontThemeOptions, MetricSnips, ThemedNumeric};
use engine::render::geometry::BoardGeometry;
use engine::render::retro::{retro_theme, RetroThemeOptions};
use engine::render::scene::SceneType;
use engine::render::sprite_sheet::{BlockSpriteSheetData, CellAnimationData, GhostStyle};
use engine::render::{AttackBallData, PeekLayout, PendingLayout, Theme};
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::time::Duration;

mod mugshots;

mod sprites {
    pub const SPRITES: &[u8] = include_bytes!("sprites.png");
    pub const BACKGROUND: &[u8] = include_bytes!("background.png");
    pub const BOARD: &[u8] = include_bytes!("board.png");
    /// the wash the panels stand on
    pub const SCENE: &[u8] = include_bytes!("scene.png");
    /// every strip that plays over a cell, one per row
    pub const ANIMATIONS: &[u8] = include_bytes!("animations.png");
    /// the four attack balls
    pub const ATTACK: &[u8] = include_bytes!("attack.png");
    /// the bold score face
    pub const FONT: &[u8] = include_bytes!("font.png");
    /// the plain face the stage number is set in
    pub const FONT_SMALL: &[u8] = include_bytes!("font-small.png");
}

/// Mean Bean Machine's soundtrack and effects, cut by `puyo-rusto/art/retro_audio.py genesis`.
mod sound {
    pub const MOVE: &[u8] = include_bytes!("move.ogg");
    pub const ROTATE: &[u8] = include_bytes!("rotate.ogg");
    pub const LOCK: &[u8] = include_bytes!("lock.ogg");
    pub const SETTLE: &[u8] = include_bytes!("settle.ogg");
    /// the game has no hard drop; this is its nearest sound
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
    /// the continue screen's music
    pub const GAME_OVER: &[u8] = include_bytes!("game-over.ogg");

    /// Each tune loops from its first bar, so none has an intro. The rip's `Stages 5-8` is
    /// `Stages 1-4` at 7/6 speed and is not carried.
    pub const STAGES_1_4: &[u8] = include_bytes!("stages-1-4-repeat.ogg");
    pub const STAGES_9_12: &[u8] = include_bytes!("stages-9-12-repeat.ogg");
    pub const STAGE_13: &[u8] = include_bytes!("stage-13-repeat.ogg");
}

/// the tracks a match on this theme may be dealt, in the game's own stage order
pub const GAME_MUSIC: [MusicTrack; 3] = [
    (None, sound::STAGES_1_4),
    (None, sound::STAGES_9_12),
    (None, sound::STAGE_13),
];

/// the Genesis bean, and `rip_retro.py`'s grid
pub const SRC_BLOCK_SIZE: u32 = 16;
const PAD: i32 = 4;
const PITCH: i32 = SRC_BLOCK_SIZE as i32 + 2 * PAD;

/// the row under the five colours, holding the refugee bean and the tray's three symbols
const EXTRAS_ROW: i32 = PuyoColor::N as i32;

/// The transparent spawning row above the well, drawn over the scene with no panel behind it.
const TOP_PADDING: u32 = SRC_BLOCK_SIZE * HIDDEN_ROWS;

/// Transparent rows under the panel so its floor never sits flush with the window's edge.
/// They count towards the fit, so they shrink a single player's cell.
const BOTTOM_PADDING: u32 = 8;

/// The outer rock `rip_retro.py` clears from the panel, `(left, right)`, which the shadow skips.
/// Must equal the script's `GENESIS_PANEL_TRIM`, pinned by
/// `the_panel_art_stops_where_the_trim_says_it_does`.
const SIDE_TRIM: (u32, u32) = (8, 4);

/// The board within the padded panel, which is the well with [`TOP_PADDING`] over it, so a
/// point in the padded background is a point on the Genesis screen.
const BOARD: (i32, i32, u32, u32) = (16, 0, COLUMNS * SRC_BLOCK_SIZE, ROWS * SRC_BLOCK_SIZE);

/// The two 32x48 boxes under `NEXT`: next, then next but one.
const NEXT_BOXES: [(i32, i32); 2] = [(120, 32), (168, 32)];
/// where the pair sits in one of them
const NEXT_PAIR: (i32, i32) = (8, 12);

/// The box the game keeps Robotnik's mugshot in, which holds the player's own character.
const MUGSHOT: (i32, i32, u32, u32) = (120, 96, 80, 56);

/// The nuisance tray, in the [`TOP_PADDING`] band above the board. It fills leftwards from the
/// well's right edge so a spawning pair never covers it.
const TRAY: (i32, i32) = (BOARD.0 + BOARD.2 as i32 - TRAY_ICON as i32, 2);
/// Tray icon size and pitch: three quarters of a cell, since the game's half cell blurs the
/// icons at this scale.
const TRAY_ICON: u32 = SRC_BLOCK_SIZE * 3 / 4;
const TRAY_STEP: u32 = TRAY_ICON;
/// as many as fit in the three columns right of the spawn column
const TRAY_MAX: u32 = 4;

/// The attack ball cell. The strip is player one's pair then player two's, big first, coloured
/// by the sending player.
const BALL_CELL: u32 = 24;
const BALL_FRAMES: u32 = 4;
/// how much of a block the ball is drawn at: 22 of a 16 pixel cell, out of a 24 pixel cut
const BALL_SCALE: f64 = BALL_CELL as f64 / SRC_BLOCK_SIZE as f64;
/// an attack of a whole row or more gets the big ball; anything under it the small one
const BALL_BIG_ATTACK: u32 = crate::game::nuisance::ROW;

/// The first score row under `SCORE`, a cell in from the game's own since this score has seven
/// digits to its eight.
const SCORE_AT: (i32, i32) = (128, 176);

/// the stage number's cell after `STAGE`, right aligned
const LEVEL_AT: (i32, i32) = (184, 80);

/// How long a bean takes to go. It adds to [`crate::game::rules::POP_DELAY`], since the match
/// skips `game.update` while an animation blocks, so this sets the beat of a chain.
const POP_HOLD: Duration = Duration::from_millis(430);

/// how long the bean holds its surprised face before the ball frames run
const POP_FACE_HOLD: Duration = Duration::from_millis(260);

/// The group flashes where it stands, starting lit, before the pop strip plays.
const POP_BLINK: Duration = Duration::from_millis(300);
const POP_BLINKS: u32 = 3;

/// The frames of one pop: a face, then a shrinking ball. It is the widest strip, so it sets
/// the animation sheet's width.
const POP_FRAMES: usize = 3;

/// the refugee bean's blink, which ends on the eyes-open frame it pauses on
const BLINK_FRAMES: usize = 3;

/// The squash a bean plays where it lands. Neither frame is linked to its neighbours, as in
/// the original.
const BOUNCE_FRAMES: usize = 2;

/// one droplet, thrown several times over by [`POP_DEBRIS`]
const DEBRIS_FRAMES: usize = 1;

/// The droplets a bean throws on its last pop frame, which outlive the clear.
const POP_DEBRIS: PopDebris = PopDebris {
    at_frame: POP_FRAMES - 1,
    pieces: 4,
    // drawn on the window, not the board, so a harder throw lands on the panel's stonework
    speed: (2.0, 5.0),
    gravity: 16.0,
    life: Duration::from_millis(380),
    // of the whole cell the droplet is centred in, so the droplet is about half a block
    size: 0.9,
};
const BLINK_FPS: u32 = 6;
const BLINK_EVERY: Duration = Duration::from_millis(2000);

/// The scene behind the panels: the dungeon wall's mean colour at half brightness, untextured
/// so the spawning row does not read as part of the panel.
const WALL: Color = Color::RGB(0x26, 0x2c, 0x16);

fn block(col: i32, row: i32) -> Point {
    Point::new(PAD + PITCH * col, PAD + PITCH * row)
}

/// A colour's sixteen link variants run along its own row, indexed by the mask's bits. The
/// skin is ignored: every player sees the same beans.
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
const NUISANCE_BLINK_ROW: u32 = NUISANCE_POP_ROW + 1;
const BOUNCE_ROW: u32 = NUISANCE_BLINK_ROW + 1;
const NUISANCE_BOUNCE_ROW: u32 = BOUNCE_ROW + PuyoColor::N as u32;
const DEBRIS_ROW: u32 = NUISANCE_BOUNCE_ROW + 1;
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

/// What plays over a cell, keyed for every link mask and skin of each colour; only the refugee
/// bean idles.
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
            idle: Some(strip(NUISANCE_BLINK_ROW, BLINK_FRAMES as u32)),
            pop: Some(strip(NUISANCE_POP_ROW, POP_FRAMES as u32)),
            bounce: Some(strip(NUISANCE_BOUNCE_ROW, BOUNCE_FRAMES as u32)),
            debris: Some(strip(NUISANCE_DEBRIS_ROW, DEBRIS_FRAMES as u32)),
        },
    ));
    out
}

pub fn genesis_theme<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
) -> Result<Theme<'a>, String> {
    let options = RetroThemeOptions {
        name: "genesis",
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
        // every row is drawn, the spawning thirteenth included
        geometry: BoardGeometry::new(SRC_BLOCK_SIZE, 0, (0, 0), COLUMNS, ROWS, ROWS),
        audio: audio(
            config.audio,
            Sounds {
                gain: GENESIS_GAIN,
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
        // score face and stage face, both on an eight pixel pitch with no gap
        font: FontThemeOptions::new(
            vec![
                FontRenderOptions::numeric_sprites(sprites::FONT, texture_creator, 0)?,
                FontRenderOptions::numeric_sprites(sprites::FONT_SMALL, texture_creator, 0)?,
            ],
            vec![
                (
                    MetricKind::Score,
                    ThemedNumeric::new(0, MetricSnips::zero_fill(SCORE_AT, MAX_SCORE)),
                ),
                (
                    MetricKind::Level,
                    ThemedNumeric::new(1, MetricSnips::right(LEVEL_AT, MAX_LEVEL)),
                ),
            ],
        ),
        board_file: sprites::BOARD,
        board_alpha: 0xff,
        board_snips: vec![],
        top_padding: TOP_PADDING,
        bottom_padding: BOTTOM_PADDING,
        shadow: Some(panel_shadow((
            SIDE_TRIM.0,
            TOP_PADDING,
            SIDE_TRIM.1,
            BOTTOM_PADDING,
        ))),
        board_point: Point::new(BOARD.0, BOARD.1),
        background_file: sprites::BACKGROUND,
        background_color: WALL,
        // the game has no board-sized match end card, so the curtain does it all
        match_end_file: None,
        game_over_points: vec![],
        interstitial_points: vec![],
        overlay_size: None,
        hold: None,
        // the two preview boxes are side by side, which no `Column` can say
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
        attack_ball: Some(AttackBallData {
            sheet: AnimationSpriteSheetData::non_exclusive_linear(
                sprites::ATTACK,
                Point::new(0, 0),
                BALL_FRAMES,
                BALL_CELL,
                BALL_CELL,
            ),
            scale: BALL_SCALE,
            big_attack: BALL_BIG_ATTACK,
        }),
        pending: Some(PendingLayout {
            point: Point::new(TRAY.0, TRAY.1),
            step: Point::new(-(TRAY_STEP as i32), 0),
            size: TRAY_ICON,
            max: TRAY_MAX,
        }),
        characters: Some((
            mugshots::characters(),
            CharacterLayout {
                rect: Rect::new(MUGSHOT.0, MUGSHOT.1, MUGSHOT.2, MUGSHOT.3),
            },
        )),
        mascot: None,
        mascot_animations: None,
        spawn_arc: None,
        cell_idle_type: FrameAnimationType::LinearWithPause {
            fps: BLINK_FPS,
            pause_for: BLINK_EVERY,
            resume_from_frame: 0,
        },
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
    };
    retro_theme(canvas, texture_creator, options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::board::SPAWN;

    /// a PNG's width and height, big endian at a fixed offset
    fn png_size(bytes: &[u8]) -> (u32, u32) {
        let word = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
        (word(16), word(20))
    }

    /// the sprite sheet's size matches the grid `puyo` reads it as
    #[test]
    fn the_sheet_is_the_shape_the_layout_reads_it_as() {
        let (width, height) = png_size(sprites::SPRITES);
        assert_eq!(width, (PITCH * LinkMask::COUNT as i32) as u32);
        assert_eq!(height, (PITCH * (EXTRAS_ROW + 1)) as u32);
    }

    /// The board and panel pngs put the well at the panel's top with its floor under it.
    #[test]
    fn the_well_fills_the_panel_from_its_own_top() {
        let (width, height) = png_size(sprites::BACKGROUND);
        let (board_width, board_height) = png_size(sprites::BOARD);
        assert_eq!(board_width, COLUMNS * SRC_BLOCK_SIZE);
        assert_eq!(board_height, VISIBLE_ROWS * SRC_BLOCK_SIZE);
        assert_eq!(BOARD.2, board_width);
        assert_eq!(
            BOARD.3,
            board_height + TOP_PADDING,
            "the board is the well with the spawning row over it"
        );
        assert!(BOARD.0 as u32 + BOARD.2 <= width);
        assert_eq!(
            BOARD.1, 0,
            "the padded board starts at the top of the padded panel"
        );
        assert_eq!(
            board_height + SRC_BLOCK_SIZE,
            height,
            "the panel has to carry the well's floor under it"
        );
        assert_eq!(TOP_PADDING, SRC_BLOCK_SIZE * HIDDEN_ROWS);
    }

    /// The panel png's opaque columns start and stop exactly at [`SIDE_TRIM`].
    #[test]
    fn the_panel_art_stops_where_the_trim_says_it_does() {
        let panel = image::load_from_memory(sprites::BACKGROUND)
            .expect("the panel is a png")
            .to_rgba8();
        let (left, right) = SIDE_TRIM;
        let opaque = |x: u32| (0..panel.height()).any(|y| panel.get_pixel(x, y)[3] > 0);
        for x in (0..left).chain(panel.width() - right..panel.width()) {
            assert!(
                !opaque(x),
                "column {x} is trimmed but the art still fills it"
            );
        }
        assert!(opaque(left), "the art does not start where the trim ends");
        assert!(
            opaque(panel.width() - right - 1),
            "the art stops short of the trim on the right"
        );
    }

    /// Every tray slot fits the band over the well and stands clear of the spawn column.
    #[test]
    fn the_whole_tray_stands_clear_of_the_spawn_column() {
        let last = TRAY.0 - (TRAY_STEP * (TRAY_MAX - 1)) as i32;
        let spawn_right = BOARD.0 + (SPAWN.x + 1) * SRC_BLOCK_SIZE as i32;
        assert!(
            last >= spawn_right,
            "slot {} of the tray is at {last}, over the spawn column, which ends at \
             {spawn_right}",
            TRAY_MAX - 1
        );
        assert!(
            TRAY.0 + TRAY_ICON as i32 <= BOARD.0 + BOARD.2 as i32,
            "the front of the tray hangs off the well"
        );
        assert!(
            TRAY.1 as u32 + TRAY_ICON <= TOP_PADDING,
            "the tray has to fit the band over the well"
        );
    }

    /// The animation png is the size the strip rows add up to, and every cell claims a strip.
    #[test]
    fn every_strip_is_where_the_theme_counts_it() {
        let (width, height) = png_size(sprites::ANIMATIONS);
        assert_eq!(width, SRC_BLOCK_SIZE * POP_FRAMES as u32);
        assert_eq!(height, (SRC_BLOCK_SIZE + ANIM_ROW_GAP) * ANIM_ROWS);
        const {
            assert!(
                BLINK_FRAMES <= POP_FRAMES
                    && BOUNCE_FRAMES <= POP_FRAMES
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
            "every puyo and every nuisance, in every skin slot"
        );
    }

    /// Both font pngs are ten half-bean digits wide and the same size.
    #[test]
    fn the_font_is_ten_digits_wide() {
        let (width, height) = png_size(sprites::FONT);
        let (small_width, small_height) = png_size(sprites::FONT_SMALL);
        assert_eq!(width % 10, 0);
        assert_eq!(small_width, width);
        assert_eq!(small_height, height);
        assert_eq!(
            width / 10,
            SRC_BLOCK_SIZE / 2,
            "a digit is half a bean wide"
        );
    }

    /// Every sound and tune this theme embeds decodes.
    #[test]
    fn every_sound_this_theme_owns_decodes() {
        let mut sounds = vec![
            sound::MOVE,
            sound::ROTATE,
            sound::LOCK,
            sound::SETTLE,
            sound::HARD_DROP,
            sound::ATTACK,
            sound::GARBAGE,
            sound::SPEED_UP,
            sound::PAUSE,
            sound::VICTORY,
            sound::GAME_OVER,
        ];
        sounds.extend(sound::POP);
        for (intro, repeat) in GAME_MUSIC {
            sounds.extend(intro);
            sounds.push(repeat);
        }
        for bytes in sounds {
            engine::audio::Sound::load(bytes, 100).expect("a genesis sound did not decode");
        }
    }
}
