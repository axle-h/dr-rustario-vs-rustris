//! Helpers that describe Puyo Rusto's sprites and sounds to the engine's theme builders.

use crate::game::cell::{LinkMask, NuisanceIcon, PuyoCell, PuyoColor, PuyoPiece, PuyoSkin};
use crate::game::rules::{MAX_LEVEL, MAX_SCORE};
use engine::config::AudioConfig;
use engine::game::geometry::Point as CellPoint;
use engine::game::{CellId, MetricKind};
use engine::render::font::MetricSnips;
use engine::render::sound::{AudioTheme, SfxKey};
use engine::render::sprite_sheet::{CellSpriteData, PreviewData};
use engine::render::PanelShadow;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};

/// A chain step's grade, which a theme has a sound per: one per step up to the third, then
/// everything longer (`clear_class` in [`crate::render`]).
pub const CLEAR_CLASSES: usize = 4;

/// Where every cell of a theme's sheet is, asked once per skin slot since a `CellId` carries
/// colour, link mask and skin.
pub fn cells(
    block_size: u32,
    puyo: impl Fn(PuyoSkin, PuyoColor, LinkMask) -> Point,
    nuisance: impl Fn(PuyoSkin) -> Point,
    tray: impl Fn(PuyoSkin) -> [Point; 3],
) -> Vec<(CellId, CellSpriteData)> {
    let snip = |p: Point| CellSpriteData::new(Rect::new(p.x, p.y, block_size, block_size));
    let mut cells = vec![];
    for skin in PuyoSkin::all() {
        cells.push((PuyoCell::Nuisance.id(skin), snip(nuisance(skin))));
        for color in PuyoColor::ALL {
            for bits in 0..LinkMask::COUNT as u8 {
                let links = LinkMask::from_bits(bits);
                cells.push((
                    PuyoCell::puyo(color, links).id(skin),
                    snip(puyo(skin, color, links)),
                ));
            }
        }
        for (icon, point) in [NuisanceIcon::Small, NuisanceIcon::Large, NuisanceIcon::Rock]
            .into_iter()
            .zip(tray(skin))
        {
            cells.push((PuyoCell::Tray(icon).id(skin), snip(point)));
        }
    }
    cells
}

/// The queue's twenty five pairs per skin slot, composed from the cells with the pivot as the
/// lower half, as it spawns.
pub fn previews() -> PreviewData {
    PreviewData::Compose {
        pieces: PuyoSkin::all()
            .flat_map(|skin| {
                PuyoPiece::all().into_iter().map(move |piece| {
                    (
                        piece.id(skin),
                        vec![
                            (CellPoint::new(0, 0), PuyoCell::loose(piece.child).id(skin)),
                            (CellPoint::new(0, 1), PuyoCell::loose(piece.pivot).id(skin)),
                        ],
                    )
                })
            })
            .collect(),
    }
}

/// The shadow a retro panel casts down and to the right on the wash behind it. `margin` is
/// the transparent padding and trim round the panel inside its box, `(left, top, right, bottom)`.
pub fn panel_shadow(margin: (u32, u32, u32, u32)) -> PanelShadow {
    PanelShadow {
        offset: (3, 3),
        spread: 5,
        color: Color::BLACK,
        alpha: 0xa0,
        margin,
    }
}

/// A track a match may be dealt: an optional one-shot lead-in, then the part that loops.
pub type MusicTrack = (Option<&'static [u8]>, &'static [u8]);

pub struct Sounds {
    /// this theme's level against the rest of the compendium, one of the gains below
    pub gain: i32,
    /// the tracks a match on this theme may be dealt
    pub music: &'static [MusicTrack],
    pub move_pair: &'static [u8],
    pub rotate: &'static [u8],
    pub lock: &'static [u8],
    pub settle: &'static [u8],
    pub hard_drop: &'static [u8],
    /// one per [`CLEAR_CLASSES`], so a chain is heard climbing
    pub pop: [&'static [u8]; CLEAR_CLASSES],
    pub attack_sent: &'static [u8],
    pub receive_nuisance: &'static [u8],
    pub speed_up: &'static [u8],
    pub paused: &'static [u8],
    pub victory: &'static [u8],
    pub game_over: &'static [u8],
}

/// Each theme's level as a percentage: Puyo Rusto's rips are mastered about eight decibels
/// hotter than the app's -22 dBFS RMS baseline, so each is trimmed here with
/// [`AudioTheme::with_gain`], which scales music and effects together so the source mix survives.
/// `engine/art/audio_levels.py` reads these constants back out of this file.
pub const GENESIS_GAIN: i32 = 45;
pub const SNES_GAIN: i32 = 44;
pub const PARTICLE_GAIN: i32 = 39;
/// the menu screens, whose music comes off the same rip as the particle theme's
pub const MENU_GAIN: i32 = 38;

/// How loud this game's effects play against its own music, as a percentage: about three
/// decibels down, which puts them within the house band of their music.
const EFFECTS_TRIM: i32 = 71;

pub fn audio(config: AudioConfig, sounds: Sounds) -> Result<AudioTheme, String> {
    let mut sfx = vec![
        (SfxKey::Move, sounds.move_pair),
        (SfxKey::Rotate, sounds.rotate),
        (SfxKey::Lock, sounds.lock),
        (SfxKey::Settle, sounds.settle),
        (SfxKey::HardDrop, sounds.hard_drop),
        (SfxKey::AttackSent, sounds.attack_sent),
        (SfxKey::AttackReceived, sounds.receive_nuisance),
        (SfxKey::SpeedUp, sounds.speed_up),
        (SfxKey::Paused, sounds.paused),
    ];
    sfx.extend(
        sounds
            .pop
            .iter()
            .enumerate()
            .map(|(class, sound)| (SfxKey::Clear(class as u16), *sound)),
    );
    let mut audio = AudioTheme::new(config, &sfx)?
        .with_gain(sounds.gain)
        .with_effects_at(EFFECTS_TRIM);
    for (intro, repeat) in sounds.music {
        audio = audio.with_game_music_track(*intro, repeat)?;
    }
    audio
        .with_game_over_music(sounds.game_over, None)?
        .with_victory_music(sounds.victory, None)
}

/// The HUD rows and the largest value each has to show. The chain is not a row: it pops up
/// over the cleared puyos instead (`clear_popup` in [`crate::render`]).
pub const HUD_MAX: [(MetricKind, u32); 2] = [
    (MetricKind::Score, MAX_SCORE),
    (MetricKind::Level, MAX_LEVEL),
];

/// where a theme puts each of them, in the order [`HUD_MAX`] names them
pub fn hud(score: MetricSnips, level: MetricSnips) -> Vec<(MetricKind, MetricSnips)> {
    vec![(MetricKind::Score, score), (MetricKind::Level, level)]
}

#[cfg(test)]
mod tests {
    /// [`hud`] places the same rows, in the same order, as [`HUD_MAX`] sizes.
    #[test]
    fn every_row_the_hud_sizes_is_a_row_it_places() {
        let placed = super::hud(
            super::MetricSnips::zero_fill((0, 0), 1),
            super::MetricSnips::zero_fill((0, 1), 1),
        );
        assert_eq!(placed.len(), super::HUD_MAX.len());
        for ((kind, _), (placed_kind, _)) in super::HUD_MAX.iter().zip(placed.iter()) {
            assert_eq!(kind, placed_kind, "the two lists are in different orders");
        }
    }

    use super::*;

    /// a sheet keyed off a grid, the way a real theme's is
    fn test_cells() -> Vec<(CellId, CellSpriteData)> {
        cells(
            8,
            |skin, color, links| {
                Point::new(
                    color as i32,
                    (links.bits() as usize + 16 * skin.index()) as i32,
                )
            },
            |skin| Point::new(6, skin.index() as i32),
            |skin| [Point::new(7, skin.index() as i32); 3],
        )
    }

    /// every cell the board can draw is keyed once in the sheet, in every skin slot
    #[test]
    fn the_sheet_keys_every_cell_the_board_can_draw() {
        let cells = test_cells();
        assert_eq!(
            cells.len(),
            PuyoSkin::COUNT * (PuyoColor::N * LinkMask::COUNT + 1 + 3)
        );
        let ids: Vec<CellId> = cells.iter().map(|(id, _)| *id).collect();
        let mut unique = ids.clone();
        unique.sort_by_key(|id| id.0);
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "two cells share a key");
    }

    /// all twenty five pairs per skin have a preview, composed of keyed cells
    #[test]
    fn the_queue_knows_every_pair_that_can_be_dealt() {
        let PreviewData::Compose { pieces } = previews() else {
            panic!("the previews are composed from the cells");
        };
        assert_eq!(pieces.len(), PuyoSkin::COUNT * PuyoColor::N * PuyoColor::N);
        let sheet = test_cells();
        for (_, cells) in pieces {
            for (_, id) in cells {
                assert!(
                    sheet.iter().any(|(key, _)| *key == id),
                    "{id:?} is not keyed"
                );
            }
        }
    }

    /// a pair's preview is composed from cells of its own skin
    #[test]
    fn a_pair_previews_in_its_own_players_sprites() {
        let PreviewData::Compose { pieces } = previews() else {
            panic!("the previews are composed from the cells");
        };
        for (piece, cells) in pieces {
            let skin = PuyoSkin::from(piece);
            for (_, id) in cells {
                assert_eq!(PuyoSkin::from(id), skin, "{piece:?} borrowed a skin");
            }
        }
    }
}
