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
use crate::game::cell::{LinkMask, PuyoCell, PuyoColor, PuyoSkin};
use crate::game::rules::{MAX_LEVEL, MAX_SCORE};
use crate::theme::data::{audio, cells, panel_shadow, previews, Sounds, CLEAR_CLASSES};
use crate::theme::GAME_MUSIC_TRACKS;
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
    /// the wash the panels stand on, cut by `rip_retro.py`'s `vignette`
    pub const SCENE: &[u8] = include_bytes!("scene.png");
    /// every strip that plays over a cell: a pop per colour, the refugee bean's pop, and
    /// the refugee bean's blink - one strip per row, cut by `rip_retro.py`'s
    /// `genesis_animations`
    pub const ANIMATIONS: &[u8] = include_bytes!("animations.png");
    /// the four attack balls - see `rip_retro.py`'s `genesis_attack_balls`
    pub const ATTACK: &[u8] = include_bytes!("attack.png");
    /// the bold face, in the first player's red - what the game sets its score in
    pub const FONT: &[u8] = include_bytes!("font.png");
    /// ... and the plain white one it prints its stage number in, which is smaller
    pub const FONT_SMALL: &[u8] = include_bytes!("font-small.png");
}

/// Mean Bean Machine's own soundtrack and sound effects, cut by
/// `puyo-rusto/art/retro_audio.py genesis`.
///
/// The game writes each stage's lead-in as a track of its own, which is exactly the pair the
/// mixer takes: the intro plays once and the stage tune loops behind it forever. The rip
/// peak-normalised every one of its effects to the same level, so the script puts each of them
/// back to the peak the particle theme's sound for the same slot has - see its doc comment,
/// which is the one place this game levels a rip rather than taking it as it came.
mod sound {
    pub const MOVE: &[u8] = include_bytes!("move.ogg");
    pub const ROTATE: &[u8] = include_bytes!("rotate.ogg");
    pub const LOCK: &[u8] = include_bytes!("lock.ogg");
    pub const SETTLE: &[u8] = include_bytes!("settle.ogg");
    /// this game has no hard drop, so this is the nearest noise it owns - see the script
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
    /// there is no track called game over: what this game plays over a burial is the music of
    /// the continue screen it puts you on
    pub const GAME_OVER: &[u8] = include_bytes!("game-over.ogg");

    pub const STAGES_1_4: (&[u8], &[u8]) = (
        include_bytes!("stages-1-4-intro.ogg"),
        include_bytes!("stages-1-4-repeat.ogg"),
    );
    pub const STAGES_5_8: (&[u8], &[u8]) = (
        include_bytes!("stages-5-8-intro.ogg"),
        include_bytes!("stages-5-8-repeat.ogg"),
    );
    pub const STAGES_9_12: (&[u8], &[u8]) = (
        include_bytes!("stages-9-12-intro.ogg"),
        include_bytes!("stages-9-12-repeat.ogg"),
    );
    pub const STAGE_13: (&[u8], &[u8]) = (
        include_bytes!("stage-13-intro.ogg"),
        include_bytes!("stage-13-repeat.ogg"),
    );
}

/// the tracks a match on this theme may be dealt, in the game's own order
///
/// Mean Bean Machine has exactly [`GAME_MUSIC_TRACKS`] stage tunes and deals them by stage,
/// four stages at a time, so the order is the game's own and the count is not a coincidence:
/// the games these themes are cut from all wrote four.
pub const GAME_MUSIC: [(&[u8], &[u8]); GAME_MUSIC_TRACKS] = [
    sound::STAGES_1_4,
    sound::STAGES_5_8,
    sound::STAGES_9_12,
    sound::STAGE_13,
];

/// the Genesis's own bean, and `rip_retro.py`'s grid
pub const SRC_BLOCK_SIZE: u32 = 16;
const PAD: i32 = 4;
const PITCH: i32 = SRC_BLOCK_SIZE as i32 + 2 * PAD;

/// the row under the five colours, holding the refugee bean and the tray's three symbols
const EXTRAS_ROW: i32 = PuyoColor::N as i32;

/// The transparent cell above everything, which is the row a pair spawns in.
///
/// A bean resting up there is still in the game, so it is drawn - but nothing is drawn behind
/// it. The panel is cut level with the top of the well and the board art stops there too, the
/// way a retro Rustris board's frame stops at its skyline, so the spawning row is a cell of
/// scene with the panel below it and nothing to either side. Mean Bean Machine's course of
/// stone over the well mouth goes with that cut. Two other arrangements were built and both
/// were wrong: the row drawn *behind* that stone, which hid a bean that mattered as soon as a
/// stack reached the top; and the well grown a course higher so the row sat inside it, which
/// read as a taller well rather than as room above the board.
const TOP_PADDING: u32 = SRC_BLOCK_SIZE * HIDDEN_ROWS;

/// The board within the padded panel, which is the well with [`TOP_PADDING`] over it.
///
/// The panel is cut at the well's top edge and the padding puts exactly that much back, so
/// **a point in the padded background is a point on the Genesis screen**. Not a `Point`
/// constant: `sdl2::rect::Point::new` is not `const`, and every other theme in the repository
/// builds its points at the call site too.
const BOARD: (i32, i32, u32, u32) = (16, 0, COLUMNS * SRC_BLOCK_SIZE, ROWS * SRC_BLOCK_SIZE);

/// Every coordinate below is a point on the Genesis screen, measured off `rip_retro.py`'s own
/// reading of the frame plane - the boxes the game left empty are holes in it, and their rects
/// are exact.
///
/// The two 32x48 boxes under `NEXT`. Mean Bean Machine fills the left one with the player's
/// next pair and the right one with the opponent's, but a panel here belongs to one player,
/// so the queue runs left to right through both: next, then next but one.
const NEXT_BOXES: [(i32, i32); 2] = [(120, 32), (168, 32)];
/// where the pair sits in one of them - centred across, and low, which is where the game
/// draws it
const NEXT_PAIR: (i32, i32) = (8, 12);

/// The box the game keeps Robotnik's mugshot in, which is the one piece of furniture this
/// panel has no use for. It is where the tray used to be, under a comment saying the Genesis
/// never drew one; it does, and the tray has gone to where it draws it, so this hole is free
/// for the character that belongs in it.
const MUGSHOT: (i32, i32, u32, u32) = (120, 96, 80, 56);

/// Where Mean Bean Machine puts the nuisance tray: on the wall **immediately above the
/// board**, in the band that is [`TOP_PADDING`] - so a point here is a point on the Genesis
/// screen and this needs no plumbing beyond the numbers.
///
/// It is anchored at the well's **right** edge and fills leftwards, which is the one thing
/// here the game does not do. The band is the row a pair spawns in and the pair is drawn
/// over it: left anchored, the front of the tray sat under the spawn column and was hidden
/// by every pair that came out of it. Filling the other way puts the icons a player reads
/// first - the heaviest, since a tray is decomposed biggest first - furthest from the spawn.
///
/// What is left is the three columns of the well the spawn column is not, which is 48
/// pixels; [`TRAY_MAX`] icons of [`TRAY_ICON`] at [`TRAY_STEP`] is exactly that, so the
/// strip fills columns 3, 4 and 5 and stops.
const TRAY: (i32, i32) = (BOARD.0 + BOARD.2 as i32 - TRAY_ICON as i32, 2);
/// How big an icon is drawn, and the pitch it is laid on - which are the same number, so
/// nothing overlaps.
///
/// Three quarters of a cell rather than the half the game draws. The three symbols are cut
/// as whole cells like every other sprite here and the art inside one runs 12 to 16 pixels
/// across a 16 pixel cell, so at half a cell a 12 pixel blob comes out at 6 and the black
/// bean's white outline and eyes mush into a smudge. At three quarters they read as the
/// beans they are.
const TRAY_ICON: u32 = SRC_BLOCK_SIZE * 3 / 4;
const TRAY_STEP: u32 = TRAY_ICON;
/// As many as stand in those three columns. Four rocks is 120 nuisance and the well holds
/// 72, so this has never been what a tray runs out of.
const TRAY_MAX: u32 = 4;

/// The ball an attack crosses the screen as, which Mean Bean Machine draws as a sprite of
/// its own rather than as one of the puyos that paid for it.
///
/// It is **wider than a cell** - 22 source pixels against a bean's 16, measured off both the
/// sheet and the capture - and its colour is the *sending player's* palette, red for player
/// one and blue for player two, which is the same rule the score font follows. The strip is
/// player one's pair then player two's, big first, and it wraps.
const BALL_CELL: u32 = 24;
const BALL_FRAMES: u32 = 4;
/// how much of a block the ball is drawn at: 22 of a 16 pixel cell, out of a 24 pixel cut
const BALL_SCALE: f64 = BALL_CELL as f64 / SRC_BLOCK_SIZE as f64;
/// an attack of a whole row or more gets the big ball; anything under it the small one
const BALL_BIG_ATTACK: u32 = crate::game::nuisance::ROW;

/// Where the score goes: the first of the two rows of digits the game keeps under `SCORE`,
/// which is the player's own. The game zero fills eight digits from 120 and this game's score
/// is seven, so it starts a cell later and its units digit lands where the game's does.
const SCORE_AT: (i32, i32) = (128, 176);

/// Where the level goes: where Mean Bean Machine prints the number after `STAGE`, which is
/// the same number under another name - right aligned on the cell the game's own sits in, and
/// in the plain face it sets that number in rather than the bold one it scores in.
const LEVEL_AT: (i32, i32) = (184, 80);

/// How long a bean takes to go, over the [`POP_FRAMES`] of its strip.
///
/// It **adds** to [`crate::game::rules::POP_DELAY`] rather than fitting inside it, whatever
/// the comment here used to say: the match screen skips `game.update` outright while an
/// animation blocks the tick, so a chain step costs the strip *and* the delay. That is why
/// the delay is short and this is long - the beat of a genesis chain is set here, and it is
/// the beat Mean Bean Machine plays at.
const POP_HOLD: Duration = Duration::from_millis(430);

/// How long the bean **holds** its surprised face before the ball frames run.
///
/// It is the beat of a Mean Bean Machine pop and by far the longest frame of the strip:
/// measured off the capture, the face is held for a quarter of a second and the two ball
/// frames go by in under a tenth each. Split evenly, three frames over [`POP_HOLD`] would
/// give the face a seventh of a second and read as a flicker on the way to the balls.
const POP_FACE_HOLD: Duration = Duration::from_millis(260);

/// The tell before the strip: the group flashes where it stands, drawn exactly as it sits on
/// the board - joined to its neighbours and all - and only then pulls a face and goes.
///
/// It is what makes a chain readable: a step announces which group is going before it goes,
/// so the eye is already in the right place when the next one starts. Measured off the
/// original at about three flashes over three tenths of a second, and it starts **lit** -
/// the group is shown and then taken away, not the other way round.
const POP_BLINK: Duration = Duration::from_millis(300);
const POP_BLINKS: u32 = 3;

/// The frames of one pop: the bean pulls a face and **holds** it ([`POP_FACE_HOLD`]), then
/// curls into a ball and the ball shrinks until there is nothing of it left. What it bursts into is not on the strip at all - see
/// [`POP_DEBRIS`], which throws droplets that leave the cell the way the original's do.
///
/// It is the widest strip on the sheet, so it is also the sheet's own width; every other
/// strip is asserted against it below.
const POP_FRAMES: usize = 3;

/// The refugee bean's blink, which is the sheet's shrunken one between two of the still one.
/// It holds still far longer than it blinks, which is what the pause is: the strip runs
/// once, waits on its last frame - the bean with its eyes open - and starts again.
const BLINK_FRAMES: usize = 3;

/// The squash a bean plays where it lands: it hits and flattens, springs back past its own
/// height, and the strip runs out into the still sprite it was going to draw anyway.
///
/// Two frames, and short. It is decoration - it holds nothing and the board carries on
/// underneath it - so a bean that outstays its landing reads as a puyo drawn wrong rather
/// than as a bounce. Neither frame carries a neck, which is Mean Bean Machine's own art: a
/// bean is briefly unlinked from its neighbours where it lands, and joins them as it settles.
const BOUNCE_FRAMES: usize = 2;

/// One droplet, which is the whole of the burst's art: it is thrown many times over and each
/// piece finds its own way out of the cell.
const DEBRIS_FRAMES: usize = 1;

/// What a bean throws off as it bursts, and when.
///
/// It is thrown on the strip's *last* frame - the bean is a shrinking ball right up to the
/// moment there is nothing left of it - and it **outlives the clear**: the board settles and
/// the next chain step starts blinking while the droplets are still in the air, which is
/// what Mean Bean Machine does and what the strip alone could never do, a sprite being stuck
/// inside its own cell.
const POP_DEBRIS: PopDebris = PopDebris {
    at_frame: POP_FRAMES - 1,
    pieces: 4,
    // Far enough to leave the cell and cross a couple, which is what the original throws,
    // and no further: they are drawn on the window rather than into the board texture, so
    // one thrown much harder than this ends up out on the panel's stonework.
    speed: (2.0, 5.0),
    gravity: 16.0,
    life: Duration::from_millis(380),
    // The droplet is cut small and centred in a whole cell, so this is the *cell's* size and
    // the droplet inside it comes out at about half a block - which is what the original
    // throws. Measured off the capture: a droplet is a little under half a bean across.
    size: 0.9,
};
const BLINK_FPS: u32 = 6;
const BLINK_EVERY: Duration = Duration::from_millis(2000);

/// What the panels stand on: the dungeon wall's own colour, flat.
///
/// The wall is *tiled* on the Genesis, and this theme tiled it too until the board opened at
/// the top - at which point the spawning row was a bean floating on the same hand scattered
/// stone the panel beside it is made of, and neither read as being in front of the other. So
/// the scene is the wall's mean colour at half its brightness: the same wall, unlit and
/// without its texture, which leaves the panel the only stone on the screen and the board
/// floating on it.
const WALL: Color = Color::RGB(0x26, 0x2c, 0x16);

fn block(col: i32, row: i32) -> Point {
    Point::new(PAD + PITCH * col, PAD + PITCH * row)
}

/// A colour's sixteen link variants run along its own row, indexed by the mask's bits.
///
/// The skin is ignored, which is the whole difference between a retro theme and the particle
/// one: `PuyoSkin::deal` still hands each player one of seven, and Mean Bean Machine drew
/// one set of beans, so every slot is keyed to the same art and both players see the same
/// beans - as they did on a Genesis.
fn puyo(_: PuyoSkin, color: PuyoColor, links: LinkMask) -> Point {
    block(links.bits() as i32, color as i32)
}

/// A strip on the animation sheet: `frames` cells edge to edge, `row` rows down.
///
/// The rows are spaced by [`ANIM_ROW_GAP`] and the frames are not, which is the engine's
/// arrangement rather than a choice: it addresses a frame by counting frame widths from the
/// strip's own start, so a strip has to be contiguous and only the rows can be given air.
const ANIM_ROW_GAP: u32 = 4;

/// Where each strip sits on the sheet, in rows. `rip_retro.py` lays them out in this order
/// and nothing else names it, so a row moved there and not here draws another strip's art
/// rather than failing - which is what [`ANIM_ROWS`] and the test below are for.
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

/// What plays over a cell: every bean of a colour pops through that colour's own strip, and
/// the refugee bean both pops and - alone among them - blinks where it sits.
///
/// A bean is keyed by colour, link mask *and* skin, and the retro themes draw one set of art
/// for every skin, so a colour's strip is claimed by all sixteen masks of it in each of the
/// [`PuyoSkin::COUNT`] slots.
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
            ..Default::default()
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
        // every row is drawn, the spawning thirteenth included, so `visible_rows` is ROWS
        // and the well is cut that many rows tall
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
        // two faces, because the game uses two: the digits are on an eight pixel pitch with
        // no gap in both of them, which is what the spacing of zero says
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
        // ... and the panel casts on it, which is what lifts it off the wash. Down and to
        // the right, because that is where every shadow in this compendium falls.
        shadow: Some(panel_shadow(TOP_PADDING)),
        board_point: Point::new(BOARD.0, BOARD.1),
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
        // the tray, where the game draws it: on the wall over the board, filling leftwards
        // from its right edge - see [`TRAY`]
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
        // the cast, in the box Mean Bean Machine keeps its opponent's face in - which is
        // free now that the tray has gone to the wall over the board, where the game
        // draws it. A face here is the *player's own*, since a panel belongs to one
        // player and there may be only one of them.
        characters: Some((
            mugshots::characters(),
            CharacterLayout {
                rect: Rect::new(MUGSHOT.0, MUGSHOT.1, MUGSHOT.2, MUGSHOT.3),
            },
        )),
        mascot: None,
        mascot_animations: None,
        spawn_arc: None,
        // the refugee bean's blink, and nothing else on this theme idles: the strip runs
        // once and then holds its last frame - the bean with its eyes open - for as long as
        // the pause, which is what makes a blink a blink rather than a flicker
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
        game_over_style: Some(GameOverStyle::Curtain {
            from_top: false,
            rows: VISIBLE_ROWS,
        }),
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

    /// The well has to fill the panel from its own top, with the panel's floor under it.
    ///
    /// Both ends are the point. The sheet keeps the screen as the two planes the Genesis drew
    /// it on and the well's *floor* is on the front one, so a panel cut from the back plane
    /// alone had open well where the floor should be and the last row of beans looked like it
    /// had stopped a row short. And the panel is cut level with the top of the well, so that
    /// the spawning row - [`TOP_PADDING`], above the panel and the board alike - has the
    /// scene behind it and nothing to either side.
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

    /// The tray shares its band with the row a pair spawns in, and the pair is drawn over
    /// it - so every slot of it has to stand clear of the spawn column, and only a test can
    /// say so: the strip is laid out in numbers that read as a tidy row either way round.
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

    /// Every strip on the animation sheet is addressed by counting frames from its own
    /// start, so a sheet a row short or a frame narrow draws another strip's art rather than
    /// failing. One row per colour, then the refugee bean's pop and its blink.
    #[test]
    fn every_strip_is_where_the_theme_counts_it() {
        let (width, height) = png_size(sprites::ANIMATIONS);
        assert_eq!(width, SRC_BLOCK_SIZE * POP_FRAMES as u32);
        assert_eq!(height, (SRC_BLOCK_SIZE + ANIM_ROW_GAP) * ANIM_ROWS);
        assert!(
            BLINK_FRAMES <= POP_FRAMES
                && BOUNCE_FRAMES <= POP_FRAMES
                && DEBRIS_FRAMES <= POP_FRAMES
                && POP_DEBRIS.at_frame < POP_FRAMES,
            "every other strip shares the sheet's width with the pop, which is its widest"
        );
        // ... and every cell the board can draw claims one of them
        let strips = animations();
        assert_eq!(strips.len(), PuyoColor::N + 1);
        let keyed: usize = strips.iter().map(|(ids, _)| ids.len()).sum();
        assert_eq!(
            keyed,
            PuyoSkin::COUNT * (PuyoColor::N * LinkMask::COUNT + 1),
            "every puyo and every nuisance, in every skin slot"
        );
    }

    /// `numeric_sprites` divides the sheet by ten and takes its whole height, so a font that
    /// is not exactly ten cells wide draws sliced digits rather than failing
    /// `numeric_sprites` divides a sheet by ten and takes its whole height, so a face that is
    /// not exactly ten cells wide draws sliced digits rather than failing. Both of this
    /// theme's are checked, and against each other: the game sets its score and its stage
    /// number in two different faces at the same eight pixel cell.
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

    /// Every one of this theme's sounds is a `retro_audio.py` cut of a rip that was 44.1 kHz
    /// already, so nothing here resamples and a rate the decoder refuses would only be caught
    /// when a match opened. The theme builder decodes them all, but no test builds a theme.
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
            sounds.extend([intro, repeat]);
        }
        for bytes in sounds {
            engine::audio::Sound::load(bytes, 100).expect("a genesis sound did not decode");
        }
    }
}
