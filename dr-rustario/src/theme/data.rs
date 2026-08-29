//! Helpers that describe Dr. Rustario's sprites and sounds to the engine's theme builders.

use crate::game::cell::DrCell;
use crate::game::geometry::Rotation;
use crate::game::pill::{PillShape, VirusColor, VitaminOrdinal, LEFT_VITAMIN_SPAWN_POINT};
use crate::game::random::MAX_VIRUSES;
use crate::game::rules::MAX_VIRUS_LEVEL;
use crate::game::MAX_SCORE;
use engine::animate::frames::FrameAnimationType;
use engine::animate::mascot::MascotAnimationTypes;
use engine::config::AudioConfig;
use engine::game::geometry::Point as CellPoint;
use engine::game::{CellId, MetricKind, PieceId};
use engine::render::animation::AnimationSpriteSheetData;
use engine::render::font::MetricSnips;
use engine::render::sound::{AudioTheme, SfxKey};
use engine::render::sprite_sheet::{
    CellAnimationData, CellSpriteData, MascotSpriteData, PreviewData,
};
use sdl2::rect::{Point, Rect};
use std::time::Duration;

pub const CLEAR_VITAMIN: u16 = 0;
pub const CLEAR_VITAMIN_COMBO: u16 = 1;
pub const CLEAR_VIRUS: u16 = 2;
pub const CLEAR_VIRUS_COMBO: u16 = 3;

pub const RETRO_THROW: FrameAnimationType = FrameAnimationType::LinearWithPause {
    fps: 10,
    pause_for: Duration::from_millis(200),
    resume_from_frame: 0,
};
pub const NES_SNES_VICTORY: FrameAnimationType = FrameAnimationType::Linear { fps: 4 };
pub const N64_VICTORY: FrameAnimationType = FrameAnimationType::LinearWithPause {
    fps: 7,
    pause_for: Duration::from_millis(2000),
    resume_from_frame: 0,
};
pub const N64_GAME_OVER: FrameAnimationType = FrameAnimationType::LinearWithPause {
    fps: 7,
    pause_for: Duration::from_millis(2000),
    resume_from_frame: 18,
};

pub fn retro_mascot(
    idle: FrameAnimationType,
    victory: FrameAnimationType,
    game_over: FrameAnimationType,
) -> MascotAnimationTypes {
    MascotAnimationTypes {
        idle,
        spawn: RETRO_THROW,
        victory,
        game_over,
    }
}

/// the cell the thrown pill is aimed at
pub fn spawn_cell() -> CellPoint {
    LEFT_VITAMIN_SPAWN_POINT
}

/// Where one colour's sprites are: the left and right vitamin of each rotation, and the
/// orphaned/garbage block.
#[derive(Clone, Copy, Debug)]
pub struct ColorLayout {
    pub north: [Point; 2],
    pub east: [Point; 2],
    pub south: [Point; 2],
    pub west: [Point; 2],
    pub garbage: Point,
}

pub fn vitamin_id(color: VirusColor, rotation: Rotation, ordinal: VitaminOrdinal) -> CellId {
    DrCell::Vitamin {
        color,
        rotation,
        ordinal,
    }
    .into()
}

/// every vitamin cell id of a colour
pub fn vitamin_ids(color: VirusColor) -> Vec<CellId> {
    let mut ids = vec![];
    for rotation in [
        Rotation::North,
        Rotation::East,
        Rotation::South,
        Rotation::West,
    ] {
        for ordinal in [VitaminOrdinal::Left, VitaminOrdinal::Right] {
            ids.push(vitamin_id(color, rotation, ordinal));
        }
    }
    ids
}

pub fn cells(
    block_size: u32,
    layouts: [(VirusColor, ColorLayout); 3],
) -> Vec<(CellId, CellSpriteData)> {
    let snip = |p: Point| Rect::new(p.x, p.y, block_size, block_size);
    let mut cells = vec![];
    for (color, layout) in layouts {
        for (rotation, points) in [
            (Rotation::North, layout.north),
            (Rotation::East, layout.east),
            (Rotation::South, layout.south),
            (Rotation::West, layout.west),
        ] {
            cells.push((
                vitamin_id(color, rotation, VitaminOrdinal::Left),
                CellSpriteData::new(snip(points[0])),
            ));
            cells.push((
                vitamin_id(color, rotation, VitaminOrdinal::Right),
                CellSpriteData::new(snip(points[1])),
            ));
        }
        cells.push((
            DrCell::Garbage(color).into(),
            CellSpriteData::new(snip(layout.garbage)),
        ));
        // the virus still sprite is its first idle frame; this is a placeholder
        cells.push((
            DrCell::Virus(color).into(),
            CellSpriteData::new(snip(layout.garbage)),
        ));
    }
    cells
}

/// the virus idles and pops with its own strips, vitamins and garbage pop with another
pub fn animations(
    color: VirusColor,
    virus_idle: AnimationSpriteSheetData,
    virus_pop: AnimationSpriteSheetData,
    vitamin_pop: AnimationSpriteSheetData,
) -> Vec<(Vec<CellId>, CellAnimationData)> {
    let mut vitamins = vitamin_ids(color);
    vitamins.push(DrCell::Garbage(color).into());
    vec![
        (
            vec![DrCell::Virus(color).into()],
            CellAnimationData {
                idle: Some(virus_idle),
                pop: Some(virus_pop),
                ..Default::default()
            },
        ),
        (
            vitamins,
            CellAnimationData {
                pop: Some(vitamin_pop),
                ..Default::default()
            },
        ),
    ]
}

/// a strip of `frames` same-sized frames starting at `start` in `file`
pub fn strip(
    file: &'static [u8],
    start: Point,
    frames: u32,
    block_size: u32,
) -> AnimationSpriteSheetData {
    AnimationSpriteSheetData::non_exclusive_linear(file, start, frames, block_size, block_size)
}

/// pill sprites per shape: yy yb yr bb by br rr ry rb
pub fn previews(file: &'static [u8], size: (u32, u32), points: [Point; 9]) -> PreviewData {
    let (w, h) = size;
    PreviewData::Sprites {
        file,
        pieces: PillShape::ALL
            .into_iter()
            .zip(points)
            .map(|(shape, p)| (PieceId::from(shape), Rect::new(p.x, p.y, w, h)))
            .collect(),
        size,
    }
}

pub fn mascot(
    throw: AnimationSpriteSheetData,
    game_over: AnimationSpriteSheetData,
    victory: AnimationSpriteSheetData,
    idle: AnimationSpriteSheetData,
) -> MascotSpriteData {
    MascotSpriteData {
        idle,
        spawn: throw,
        game_over,
        victory,
        scale: None,
    }
}

pub struct Sounds {
    pub move_pill: &'static [u8],
    pub rotate: &'static [u8],
    pub drop: &'static [u8],
    pub destroy_virus: &'static [u8],
    pub destroy_virus_combo: &'static [u8],
    pub destroy_vitamin: &'static [u8],
    pub destroy_vitamin_combo: &'static [u8],
    pub paused: &'static [u8],
    pub speed_level_up: &'static [u8],
    pub receive_garbage: &'static [u8],
    pub next_level_jingle: &'static [u8],
    pub hard_drop: Option<&'static [u8]>,
}

pub fn audio(config: AudioConfig, sounds: Sounds) -> Result<AudioTheme, String> {
    let mut sfx = vec![
        (SfxKey::Move, sounds.move_pill),
        (SfxKey::Rotate, sounds.rotate),
        (SfxKey::Lock, sounds.drop),
        (SfxKey::Settle, sounds.drop),
        (SfxKey::Clear(CLEAR_VIRUS), sounds.destroy_virus),
        (SfxKey::Clear(CLEAR_VIRUS_COMBO), sounds.destroy_virus_combo),
        (SfxKey::Clear(CLEAR_VITAMIN), sounds.destroy_vitamin),
        (
            SfxKey::Clear(CLEAR_VITAMIN_COMBO),
            sounds.destroy_vitamin_combo,
        ),
        (SfxKey::Paused, sounds.paused),
        (SfxKey::SpeedUp, sounds.speed_level_up),
        (SfxKey::AttackReceived, sounds.receive_garbage),
        (SfxKey::StageComplete, sounds.next_level_jingle),
    ];
    if let Some(hard_drop) = sounds.hard_drop {
        sfx.push((SfxKey::HardDrop, hard_drop));
    }
    AudioTheme::new(config, &sfx)
}

pub fn hud(
    score: MetricSnips,
    level: MetricSnips,
    viruses: MetricSnips,
) -> Vec<(MetricKind, MetricSnips)> {
    vec![
        (MetricKind::Score, score),
        (MetricKind::Level, level),
        (MetricKind::Viruses, viruses),
    ]
}

pub const HUD_MAX: [(MetricKind, u32); 3] = [
    (MetricKind::Score, MAX_SCORE),
    (MetricKind::Level, MAX_VIRUS_LEVEL),
    (MetricKind::Viruses, MAX_VIRUSES),
];
