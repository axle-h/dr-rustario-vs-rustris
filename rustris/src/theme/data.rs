//! Helpers that describe Rustris's sprites and sounds to the engine's theme builders.

use crate::game::board::BOARD_HEIGHT;
use crate::game::cell::Mino;
use crate::game::geometry::Rotation;
use crate::game::tetromino::TetrominoShape;
use crate::game::{MAX_LEVEL, MAX_LINES, MAX_SCORE};
use engine::animate::game_over::GameOverStyle;
use engine::config::AudioConfig;
use engine::game::{CellId, MetricKind, PieceId};
use engine::render::font::{FontAlign, FontRenderOptions, FontSprite, MetricSnips};
use engine::render::sound::{AudioTheme, SfxKey};
use engine::render::sprite_sheet::{CellSpriteData, PreviewData};
use sdl2::rect::{Point, Rect};

pub const HUD_MAX: [(MetricKind, u32); 3] = [
    (MetricKind::Score, MAX_SCORE),
    (MetricKind::Level, MAX_LEVEL),
    (MetricKind::Lines, MAX_LINES),
];

/// Where a shape's minos are in the source file: one sprite shared by all four minos or one
/// per mino, and the same for the locked ("stack") look.
#[derive(Clone, Copy, Debug)]
pub struct ShapeSprites {
    pub normal: [Point; 4],
    pub stack: [Point; 4],
    /// the sprite looks the same however the piece is rotated
    pub symmetrical: bool,
}

impl From<Point> for ShapeSprites {
    fn from(p: Point) -> Self {
        Self {
            normal: [p; 4],
            stack: [p; 4],
            symmetrical: true,
        }
    }
}

impl From<(Point, Point)> for ShapeSprites {
    fn from((normal, stack): (Point, Point)) -> Self {
        Self {
            normal: [normal; 4],
            stack: [stack; 4],
            symmetrical: true,
        }
    }
}

impl From<[Point; 4]> for ShapeSprites {
    fn from(points: [Point; 4]) -> Self {
        Self {
            normal: points,
            stack: points,
            symmetrical: false,
        }
    }
}

/// sprites in the order I, J, L, O, S, T, Z plus garbage
pub fn cells(
    block_size: u32,
    i: impl Into<ShapeSprites>,
    j: impl Into<ShapeSprites>,
    l: impl Into<ShapeSprites>,
    o: impl Into<ShapeSprites>,
    s: impl Into<ShapeSprites>,
    t: impl Into<ShapeSprites>,
    z: impl Into<ShapeSprites>,
    garbage: Point,
) -> Vec<(CellId, CellSpriteData)> {
    let snip = |p: Point| Rect::new(p.x, p.y, block_size, block_size);
    let shapes: [(TetrominoShape, ShapeSprites); 7] = [
        (TetrominoShape::I, i.into()),
        (TetrominoShape::J, j.into()),
        (TetrominoShape::L, l.into()),
        (TetrominoShape::O, o.into()),
        (TetrominoShape::S, s.into()),
        (TetrominoShape::T, t.into()),
        (TetrominoShape::Z, z.into()),
    ];
    let mut cells = vec![];
    for (shape, sprites) in shapes {
        for rotation in [
            Rotation::North,
            Rotation::East,
            Rotation::South,
            Rotation::West,
        ] {
            let angle = if sprites.symmetrical {
                0.0
            } else {
                rotation.angle()
            };
            for mino in 0..4 {
                cells.push((
                    Mino::id(shape, rotation, mino),
                    CellSpriteData::new(snip(sprites.normal[mino as usize]))
                        .with_stack(snip(sprites.stack[mino as usize]))
                        .rotated(angle),
                ));
            }
        }
    }
    cells.push((Mino::garbage(), CellSpriteData::new(snip(garbage))));
    cells
}

/// queue and hold pieces composed from their minos
pub fn previews() -> PreviewData {
    PreviewData::Compose {
        pieces: TetrominoShape::ALL
            .into_iter()
            .map(|shape| {
                let minos = shape.meta().normal_minos();
                (
                    PieceId::from(shape),
                    minos
                        .into_iter()
                        .enumerate()
                        .map(|(i, p)| (p, Mino::id(shape, Rotation::North, i as u32)))
                        .collect(),
                )
            })
            .collect(),
    }
}

/// a retro font of digits and 24 letters laid out by `char_snip(row, col)`
pub fn retro_font(
    file: &'static [u8],
    spacing: u32,
    digits: impl Fn(i32) -> Rect,
    letters: impl Fn(i32) -> Rect,
) -> FontRenderOptions {
    let mut sprites = Vec::with_capacity(24 * 2 + 10);
    for i in 0..10 {
        sprites.push(FontSprite::new(
            char::from_u32('0' as u32 + i as u32).unwrap(),
            digits(i),
        ));
    }
    for i in 0..24 {
        let snip = letters(i);
        sprites.push(FontSprite::new(
            char::from_u32('A' as u32 + i as u32).unwrap(),
            snip,
        ));
        sprites.push(FontSprite::new(
            char::from_u32('a' as u32 + i as u32).unwrap(),
            snip,
        ));
    }
    FontRenderOptions::Sprites {
        file_bytes: file,
        sprites,
        spacing,
    }
}

pub fn hud(
    buffer_pixels: i32,
    score: MetricSnips,
    level: MetricSnips,
    lines: MetricSnips,
) -> Vec<(MetricKind, MetricSnips)> {
    vec![
        (MetricKind::Score, score.offset(0, buffer_pixels)),
        (MetricKind::Level, level.offset(0, buffer_pixels)),
        (MetricKind::Lines, lines.offset(0, buffer_pixels)),
    ]
}

pub fn zero_fill<P: Into<Point>>(point: P, chars: u32) -> MetricSnips {
    MetricSnips::chars(FontAlign::Left { zero_fill: true }, point, chars)
}

pub fn right<P: Into<Point>>(point: P, chars: u32) -> MetricSnips {
    MetricSnips::chars(FontAlign::Right, point, chars)
}

/// the game over curtain fills the playfield only: the two rows of buffer zone a Rustris
/// board shows above the skyline are open sky, and stay that way
pub fn curtain(from_top: bool) -> GameOverStyle {
    GameOverStyle::Curtain {
        from_top,
        rows: BOARD_HEIGHT,
    }
}

pub struct Sounds {
    pub music: &'static [u8],
    pub move_piece: &'static [u8],
    pub rotate: &'static [u8],
    pub lock: &'static [u8],
    pub send_garbage: &'static [u8],
    /// single, double, triple, tetris
    pub clear: [&'static [u8]; 4],
    pub level_up: &'static [u8],
    pub game_over: &'static [u8],
    pub pause: &'static [u8],
    pub victory: &'static [u8],
    pub stack_drop: Option<&'static [u8]>,
    pub hard_drop: Option<&'static [u8]>,
    pub hold: Option<&'static [u8]>,
    /// how loud this theme's effects play against its own music, as a percentage - see
    /// [`PARTICLE_EFFECTS`]. 100 for a theme whose rip already balances itself
    pub effects: i32,
}

/// How loud the particle theme's effects play against its own music, as a percentage.
///
/// **Measured by `engine/art/audio_levels.py`**, the compendium's audio meter: take a theme's
/// effects as RMS against the RMS of its music and every other theme in the app lands between
/// -7 and +2 dB, with `rustris/gb` - the balance Alex reads as right - at -2.0. This theme sat
/// at -6.9, the quietest set in the app against the loudest music, which is a set that is
/// *balanced* wrong rather than levelled wrong: nothing it plays is too quiet on its own, they
/// are all too quiet under this particular tune. The lift is +4.9 dB and its bound is the
/// headroom: `stack-drop`, the loudest thing here, peaks at -8.1 dBFS and lands at -3.2.
pub const PARTICLE_EFFECTS: i32 = 176;

pub fn audio(config: AudioConfig, sounds: Sounds) -> Result<AudioTheme, String> {
    let mut sfx = vec![
        (SfxKey::Move, sounds.move_piece),
        (SfxKey::Rotate, sounds.rotate),
        (SfxKey::Lock, sounds.lock),
        (SfxKey::AttackReceived, sounds.send_garbage),
        (SfxKey::Clear(0), sounds.clear[0]),
        (SfxKey::Clear(1), sounds.clear[1]),
        (SfxKey::Clear(2), sounds.clear[2]),
        (SfxKey::Clear(3), sounds.clear[3]),
        (SfxKey::SpeedUp, sounds.level_up),
        (SfxKey::Paused, sounds.pause),
    ];
    if let Some(stack_drop) = sounds.stack_drop {
        sfx.push((SfxKey::Settle, stack_drop));
    }
    if let Some(hard_drop) = sounds.hard_drop {
        sfx.push((SfxKey::HardDrop, hard_drop));
    }
    if let Some(hold) = sounds.hold {
        sfx.push((SfxKey::Hold, hold));
    }
    AudioTheme::new(config, &sfx)?
        .with_effects_at(sounds.effects)
        .with_looping_game_music(sounds.music)?
        .with_game_over_music(sounds.game_over, None)?
        .with_victory_music(sounds.victory, None)
}
