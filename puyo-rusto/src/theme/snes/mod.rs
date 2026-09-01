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
use crate::game::cell::{LinkMask, PuyoCell, PuyoColor, PuyoSkin};
use crate::game::rules::{MAX_LEVEL, MAX_SCORE};
use crate::theme::data::{audio, cells, hud, panel_shadow, previews, Sounds};
use crate::theme::sound;
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
    /// the wash the panels stand on, cut by `rip_retro.py`'s `vignette`
    pub const SCENE: &[u8] = include_bytes!("scene.png");
    /// every strip that plays over a cell: a pop per colour and the boulder's, a landing
    /// squash per colour, and what each of them bursts into - one strip per row, cut by
    /// `rip_retro.py`'s `snes_animations`
    pub const ANIMATIONS: &[u8] = include_bytes!("animations.png");
    pub const FONT: &[u8] = include_bytes!("font.png");
}

/// Kirby's Avalanche's own music, cut by `puyo-rusto/art/retro_audio.py snes`.
///
/// The music and nothing else: the source is a set of SPC dumps, which carry no sound effects
/// at all, so this theme goes on playing the game's own out of [`crate::theme::sound`] the way
/// it played everything before there was a rip to cut.
///
/// Every track is a *pair*, the mixer having no loop marker. What the first half of each pair
/// holds is **not** a lead-in the way Mean Bean Machine's is - these tunes loop from their
/// first bar, and the `Stage Intro` tracks in that dump are the pre-stage screen's own looping
/// music and lead into nothing. It is the fraction of a second in which the SNES's echo buffer
/// fills, which is the only part of the render that does not repeat, and it is cut off here so
/// that the loop carries the echo the way the hardware does. `retro_audio.py`'s docstring is
/// the long version.
mod music {
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

    /// the dump splits the win music over two SPCs, because the game changes song halfway
    /// through the flourish; the theme gets the pair of them joined
    pub const VICTORY: &[u8] = include_bytes!("victory.ogg");
    pub const GAME_OVER: &[u8] = include_bytes!("game-over.ogg");
}

/// the tracks a match on this theme may be dealt, in the order the dump numbers them
///
/// Kirby's Avalanche wrote **three** stage tunes where Mean Bean Machine wrote four, which is
/// why nothing says how many a theme must have. Nothing picks between them either - the engine
/// deals one when a match opens on this theme - so the order is only the dump's own.
pub const GAME_MUSIC: [(&[u8], &[u8]); 3] = [music::STAGE_1, music::STAGE_2, music::STAGE_3];

mod kirby;

/// the SNES's own blob, and `rip_retro.py`'s grid
pub const SRC_BLOCK_SIZE: u32 = 16;
const PAD: i32 = 4;
const PITCH: i32 = SRC_BLOCK_SIZE as i32 + 2 * PAD;

/// the row under the five colours, holding the boulder and the tray's three symbols
const EXTRAS_ROW: i32 = PuyoColor::N as i32;

/// Where the game's own field sits in the panel. The panel is the whole SNES screen cut off
/// at the second player's field, so **a point here is a point on the SNES screen**, the
/// engine's included.
const FIELD: (i32, i32) = (8, 16);

/// ... and the transparent cell above everything, which is the row a pair spawns in.
///
/// A blob resting up there is still in the game, so it is drawn - but nothing is drawn behind
/// it. The panel is cut level with the top of the field and the board art stops there too,
/// the way a retro Rustris board's frame stops at its skyline, so the spawning row is a cell
/// of scene with the panel below it and nothing to either side. The hedge the game lays
/// across the top of the screen goes with that cut, over the queue's column as well as over
/// the field: what is left of the panel is level all the way across.
const TOP_PADDING: u32 = SRC_BLOCK_SIZE * HIDDEN_ROWS;

/// Transparent rows under the panel, so the course the score is printed on - the panel's own
/// bottom edge - stands clear of the window rather than running off it. The same band the
/// genesis panel gets, since both are one panel standing on one vignette.
///
/// This theme has no side trim to go with it: at 152 wide it has never been the theme that
/// binds the cell size, and it has 129 pixels of scene either side of it in a two player
/// game already. It is only ever the bottom that runs off.
///
/// The panel had to lose a row for it. The SNES screen's last row is one flat blue-grey run
/// right across it - the console's own border, under both players' fields alike, and none of
/// the game's art - and `rip_retro.py` used to cut the panel through it, which never showed
/// while the panel ran off the bottom of the window. See `SNES_SCREEN_BOTTOM` there, and
/// `SCREEN_BORDER_ROW` in the tests below, which is what the panel's height is measured
/// against now.
const BOTTOM_PADDING: u32 = 8;

/// The two boxes under `NEXT`: the gaps between the three wooden posts that run down the
/// column, which is what the game frames its queues with. Kirby's Avalanche puts the player's
/// next pair in one and the opponent's in the other and names them over the top; a panel here
/// belongs to one player with both boxes to itself, so `rip_retro.py` paints the names out
/// and the queue runs left to right through both - next, then next but one.
const NEXT_BOXES: [(i32, i32, u32, u32); 2] = [(108, 32, 16, 47), (130, 32, 18, 47)];

/// The recess under `STAGE`, which is `rip_retro.py`'s `SNES_STAGE_NUMBER` - the game prints
/// its stage number in it and the script fills it flat, because that number is the game's and
/// this one has its own. The level goes back in, right aligned where the original's single
/// digit sat.
const STAGE_BOX: (i32, i32, u32, u32) = (120, 103, 16, 16);

/// The course of plank across the mouth of the arch, which `rip_retro.py` lays where the
/// game stands Kirby and this one stands nothing. Forty eight pixels across, and the only
/// run this column has that is as wide as a tray needs - so the tray stands on it.
const ARCH_MOUTH: (i32, i32, u32, u32) = (104, 192, 48, 16);
/// How big a tray icon is drawn, and the pitch it is laid on - which are the same number,
/// so nothing overlaps.
///
/// Three quarters of a cell, not the half it was. The three symbols are the boulder at three
/// weights and each is cut as a whole cell with its art to the edges, so at half a cell a
/// sixteen pixel rock came out at eight - eight pixels *on woodgrain*, which is where they
/// stopped reading as rocks at all. Three quarters is as big as the plank will take, and
/// [`TRAY_MAX`] of them fill it exactly.
const TRAY_ICON: u32 = SRC_BLOCK_SIZE * 3 / 4;
const TRAY_STEP: u32 = TRAY_ICON;
/// As many as stand on the plank at that size. Four thirty-rocks is 120 nuisance and the
/// field holds 72, so this has never been what a tray runs out of.
const TRAY_MAX: u32 = ARCH_MOUTH.2 / TRAY_ICON;

/// The game's own digits are two 8x8 tiles stacked - see `snes_font` in `rip_retro.py`, which
/// found the pair by matching the two digits the layer render happens to carry against a
/// decode of every tile in VRAM. They are drawn on an eight pixel pitch with no gap, which
/// is what the font's own spacing is set to.
const FONT_HEIGHT: u32 = 16;
const FONT_WIDTH: u32 = 8;

/// where the game right aligns its own score, and the cell it prints it in
const SCORE_AT: (i32, i32) = (104, 207);
/// ... and where it prints its stage number, in the recess: one cell, right aligned in it
const LEVEL_AT: (i32, i32) = (STAGE_BOX.0 + STAGE_BOX.2 as i32, STAGE_BOX.1);

/// How long a blob takes to go, over the [`POP_FRAMES`] of its strip.
///
/// It **adds** to [`crate::game::rules::POP_DELAY`] rather than fitting inside it: the match
/// screen skips `game.update` outright while an animation blocks the tick, so a chain step
/// costs the strip *and* the delay.
///
/// This and the two constants under it are **not measured off Kirby's Avalanche**, unlike
/// every geometric number in this file. They are the genesis theme's own beats, shortened:
/// the two games are the same Compile engine with different art, so the shape of a pop is the
/// same, but genesis is deliberately the slower of the two (see `POP_HOLD` there, and the
/// note in `docs/puyo-puyo-plan.md`) and this theme was 290 ms a chain step before it had a
/// strip at all. The three of them plus `POP_DELAY` come to about 550 ms, against genesis's
/// 820. If a capture of the real thing ever settles it, these are the three numbers to set.
const POP_HOLD: Duration = Duration::from_millis(260);

/// How long the blob **holds** its surprised face before it curls into a ball.
///
/// The pop is not evenly paced in either game: the face is the beat that reads and the balls
/// go by in a moment. Split evenly, three frames over [`POP_HOLD`] would give the face under
/// a tenth of a second and read as a flicker on the way to the ball.
const POP_FACE_HOLD: Duration = Duration::from_millis(140);

/// The tell before the strip: the group flashes where it stands, drawn exactly as it sits on
/// the board - joined to its neighbours and all - and only then pulls a face and goes.
///
/// It is what makes a chain readable: a step announces which group is going before it goes,
/// so the eye is already in the right place when the next one starts. It starts **lit** - the
/// group is shown and then taken away, not the other way round. Two flashes rather than
/// genesis's three, at genesis's own measured rate of one about every hundred milliseconds:
/// the same tell, on the faster of the two games.
const POP_BLINK: Duration = Duration::from_millis(200);
const POP_BLINKS: u32 = 2;

/// The frames of one pop: the blob's eyes go wide, it curls into a ball, and the ball shrinks
/// until there is nothing of it left. What it bursts into is not on the strip at all - see
/// [`POP_DEBRIS`], which throws sparks that leave the cell.
///
/// It is the widest strip on the sheet, so it is also the sheet's own width; every other
/// strip is asserted against it below.
const POP_FRAMES: usize = 3;

/// The squash a blob plays where it lands: it hits and flattens, springs back past its own
/// height, and the strip runs out into the still sprite it was going to draw anyway.
///
/// Two frames, and short. It is decoration - it holds nothing and the board carries on
/// underneath it - so a blob that outstays its landing reads as a puyo drawn wrong rather
/// than as a bounce. Neither frame carries a neck, which is Kirby's Avalanche's own art: a
/// blob is briefly unlinked from its neighbours where it lands, and joins them as it settles.
const BOUNCE_FRAMES: usize = 2;

/// One spark, which is the whole of the burst's art: it is thrown several times over and each
/// piece finds its own way out of the cell.
const DEBRIS_FRAMES: usize = 1;

/// What a blob throws off as it bursts, and when.
///
/// It is thrown on the strip's *last* frame - the blob is a shrinking ball right up to the
/// moment there is nothing left of it - and it **outlives the clear**: the board settles and
/// the next chain step starts blinking while the sparks are still in the air, which is what a
/// sprite stuck inside its own cell could never do.
const POP_DEBRIS: PopDebris = PopDebris {
    at_frame: POP_FRAMES - 1,
    pieces: 4,
    // far enough to leave the cell and cross a couple, and no further: they are drawn on the
    // window rather than into the board texture, so one thrown much harder than this ends up
    // out on the flower border
    speed: (2.0, 5.0),
    gravity: 16.0,
    life: Duration::from_millis(380),
    // `size` measures the **cell**, and this game's spark is four pixels across a sixteen
    // pixel one where Mean Bean Machine's droplet is eight - so the cell is drawn wider than
    // a block to bring the spark itself out at about a third of one, which is the size the
    // genesis droplet leaves its cell at.
    size: 1.5,
};

/// What the panels stand on: the canopy's own colour, flat.
///
/// Kirby's Avalanche tiles a leafy canopy behind its two fields and this theme tiled it too,
/// until the board opened at the top and a blob spawning above the field had that same
/// canopy behind it. Flat, at three quarters of its brightness, it is still the same forest
/// and the panel is the only thing on the screen with a texture - see the note on `genesis`'s
/// wall, which is the same problem and the same answer.
const FOREST: Color = Color::RGB(0x00, 0x15, 0x00);

fn block(col: i32, row: i32) -> Point {
    Point::new(PAD + PITCH * col, PAD + PITCH * row)
}

/// a colour's sixteen link variants run along its own row, indexed by the mask's bits. The
/// skin is ignored: Kirby's Avalanche drew one set of blobs, so both players see the same.
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

/// What plays over a cell: every blob of a colour pops and lands through that colour's own
/// strips, and the boulder pops through its dissolving frames.
///
/// A blob is keyed by colour, link mask *and* skin, and the retro themes draw one set of art
/// for every skin, so a colour's strip is claimed by all sixteen masks of it in each of the
/// [`PuyoSkin::COUNT`] slots.
///
/// The boulder gets **no bounce and no idle**, where the genesis refugee bean has both. The
/// sheet carries neither: its four frames are one rock coming apart, and a rock that came
/// apart where it landed would read as a clear rather than as a landing. Nothing in this file
/// invents art the rip does not have.
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
                victory: music::VICTORY,
                game_over: music::GAME_OVER,
            },
        )?,
        // right aligned where the game printed its own, just after the `SC` its border keeps
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
        // ... and the panel casts on it, which is what lifts it off the wash. Down and to
        // the right, because that is where every shadow in this compendium falls.
        shadow: Some(panel_shadow((0, TOP_PADDING, 0, BOTTOM_PADDING))),
        // the padding is above the panel and the board alike, so the field's art lands back
        // on the field: a point here is a point on the SNES screen
        board_point: Point::new(FIELD.0, 0),
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
                        Point::new(x + *w as i32 / 2, y + *h as i32 / 2),
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
                ARCH_MOUTH.0
                    + (ARCH_MOUTH.2 as i32 - (TRAY_STEP * (TRAY_MAX - 1) + TRAY_ICON) as i32) / 2,
                ARCH_MOUTH.1 + (ARCH_MOUTH.3 as i32 - TRAY_ICON as i32) / 2,
            ),
            step: Point::new(TRAY_STEP as i32, 0),
            size: TRAY_ICON,
            max: TRAY_MAX,
        }),
        // Kirby himself, in the arch at the foot of the centre column where the game stands
        // him. Not a mugshot: this is the *player's own* character and he walks about the
        // arch, changes shape and leaves it altogether, so he is declared as routines rather
        // than as one strip a state - see `kirby.rs` and `engine::render::character`.
        characters: Some((
            kirby::cast(),
            CharacterLayout {
                rect: Rect::new(kirby::BOX.0, kirby::BOX.1, kirby::BOX.2, kirby::BOX.3),
            },
        )),
        mascot: None,
        mascot_animations: None,
        spawn_arc: None,
        // nothing on this theme idles: the boulder has no blink where the genesis refugee bean
        // has one, so no cell declares an idle strip and this is never asked for
        cell_idle_type: FrameAnimationType::Static,
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
        // Kirby's Avalanche draws no ball crossing the screen and the rip carries none, where
        // Mean Bean Machine draws one per player per weight - so an attack sent from this
        // theme falls back to the popped blob's own cell with a white core over it, which is
        // `Theme::draw_attack_ball`'s answer for a theme with nothing cut
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

    /// The console's own border row, which the panel stops a row short of - see
    /// [`BOTTOM_PADDING`], which is why it had to.
    const SCREEN_BORDER_ROW: u32 = 1;

    /// The board is drawn *under* the panel, so the panel needs a hole exactly where the
    /// field is - the one thing about a retro theme that nothing but the art records. The
    /// panel is cut level with the top of the field, so that the spawning row -
    /// [`TOP_PADDING`], above the panel and the board alike - has the scene behind it and
    /// nothing to either side.
    #[test]
    fn the_field_fits_the_hole_it_is_drawn_into() {
        let (width, height) = png_size(sprites::BACKGROUND);
        let (board_width, board_height) = png_size(sprites::BOARD);
        assert_eq!(board_width, COLUMNS * SRC_BLOCK_SIZE);
        assert_eq!(board_height, VISIBLE_ROWS * SRC_BLOCK_SIZE);
        assert!(FIELD.0 as u32 + board_width <= width);
        // the field's own top is where the panel now starts, and the grass under it is a cell
        // - which is what puts the field at 16 on the screen rather than the 15 the layer
        // render read
        assert_eq!(FIELD.1 as u32, SRC_BLOCK_SIZE);
        assert_eq!(
            board_height + SRC_BLOCK_SIZE - SCREEN_BORDER_ROW,
            height,
            "the panel is the field and the border under it, less the screen's own last row"
        );
        assert_eq!(TOP_PADDING, SRC_BLOCK_SIZE * HIDDEN_ROWS);
    }

    /// the boxes are the game's own furniture, measured off the layer render by
    /// `rip_retro.py`; what this checks is that what is put in them fits and lands on the panel
    #[test]
    fn everything_the_panel_is_told_to_draw_lands_on_it() {
        let (width, panel_height) = png_size(sprites::BACKGROUND);
        // every rect below is a point on the SNES screen, which is a point in the *padded*
        // background - so what they have to fit is the panel with the spawning row over it,
        // and not the panel's own png. It passed either way while the panel was the height of
        // the screen; it stopped being that when the console's border row went.
        let height = panel_height + TOP_PADDING;
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
        assert!(STAGE_BOX.2 >= FONT_WIDTH && STAGE_BOX.3 >= FONT_HEIGHT);
        assert!(STAGE_BOX.0 as u32 + STAGE_BOX.2 <= width);
        assert!(STAGE_BOX.1 as u32 + STAGE_BOX.3 <= height);
        assert!(ARCH_MOUTH.0 as u32 + ARCH_MOUTH.2 <= width);
        assert!(ARCH_MOUTH.1 as u32 + ARCH_MOUTH.3 <= height);
        // the whole tray stands on the plank: a rock is drawn bigger than the pitch it is
        // laid on, so the last one reaches past the last slot and can hang off the end
        assert!(TRAY_STEP * (TRAY_MAX - 1) + TRAY_ICON <= ARCH_MOUTH.2);
    }

    /// Every strip on the animation sheet is addressed by counting frames from its own
    /// start, so a sheet a row short or a frame narrow draws another strip's art rather than
    /// failing. One pop per colour, the boulder's, then the squashes and the bursts.
    #[test]
    fn every_strip_is_where_the_theme_counts_it() {
        let (width, height) = png_size(sprites::ANIMATIONS);
        assert_eq!(width, SRC_BLOCK_SIZE * POP_FRAMES as u32);
        assert_eq!(height, (SRC_BLOCK_SIZE + ANIM_ROW_GAP) * ANIM_ROWS);
        assert!(
            BOUNCE_FRAMES <= POP_FRAMES
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
            "every blob and every boulder, in every skin slot"
        );
    }

    #[test]
    fn the_font_is_ten_digits_wide() {
        let (width, _) = png_size(sprites::FONT);
        assert_eq!(width % 10, 0);
    }

    /// The dump this theme's music is cut from is 44.1 kHz already, so `retro_audio.py`
    /// resamples nothing and a rate the decoder refuses would only be caught when a match
    /// opened on this theme. The theme builder decodes them all, but no test builds a theme.
    #[test]
    fn every_sound_this_theme_owns_decodes() {
        let mut sounds = vec![music::VICTORY, music::GAME_OVER];
        for (intro, repeat) in GAME_MUSIC {
            sounds.extend([intro, repeat]);
        }
        for bytes in sounds {
            engine::audio::Sound::load(bytes, 100).expect("a snes sound did not decode");
        }
    }
}
