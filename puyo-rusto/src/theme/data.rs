//! Helpers that describe Puyo Rusto's sprites and sounds to the engine's theme builders.

use crate::game::cell::{LinkMask, NuisanceIcon, PuyoCell, PuyoColor, PuyoPiece};
use crate::game::rules::MAX_SCORE;
use engine::config::AudioConfig;
use engine::game::geometry::Point as CellPoint;
use engine::game::{CellId, MetricKind};
use engine::render::sound::{AudioTheme, SfxKey};
use engine::render::sprite_sheet::{CellSpriteData, PreviewData};
use sdl2::rect::{Point, Rect};

/// A chain step's grade, which is what a theme has a sound per.
///
/// One per step of a chain up to the third, and then everything longer with the biggest
/// clear there is - see `clear_class` in [`crate::render`], which reserves the last of them
/// for the particle field's silhouette interrupt.
pub const CLEAR_CLASSES: usize = 4;

/// Where every cell of a theme's sheet is.
///
/// A Puyo `CellId` is a colour and a four bit link mask, so the coloured cells are a
/// `colour x 16` grid and there is no way round authoring all eighty of them - see
/// `puyo-rusto/art/sprites.py`, which draws the particle theme's. `puyo` is asked for each
/// one in turn; `nuisance` and the three `tray` symbols are single sprites.
pub fn cells(
    block_size: u32,
    puyo: impl Fn(PuyoColor, LinkMask) -> Point,
    nuisance: Point,
    tray: [Point; 3],
) -> Vec<(CellId, CellSpriteData)> {
    let snip = |p: Point| CellSpriteData::new(Rect::new(p.x, p.y, block_size, block_size));
    let mut cells = vec![(CellId::from(PuyoCell::Nuisance), snip(nuisance))];
    for color in PuyoColor::ALL {
        for bits in 0..LinkMask::COUNT as u8 {
            let links = LinkMask::from_bits(bits);
            cells.push((
                CellId::from(PuyoCell::puyo(color, links)),
                snip(puyo(color, links)),
            ));
        }
    }
    for (icon, point) in [NuisanceIcon::Small, NuisanceIcon::Large, NuisanceIcon::Rock]
        .into_iter()
        .zip(tray)
    {
        cells.push((CellId::from(PuyoCell::Tray(icon)), snip(point)));
    }
    cells
}

/// The queue's twenty five pairs, composed from the cells rather than drawn again.
///
/// A pair is two colours drawn from five, so dedicated preview sprites would be twenty five
/// of them - and every one would be the two cell sprites stacked, which is exactly what
/// [`PreviewData::Compose`] does for nothing. The pivot is the lower half, the way it sits
/// on the board when the pair spawns.
pub fn previews() -> PreviewData {
    PreviewData::Compose {
        pieces: PuyoPiece::all()
            .into_iter()
            .map(|piece| {
                (
                    piece.into(),
                    vec![
                        (
                            CellPoint::new(0, 0),
                            CellId::from(PuyoCell::loose(piece.child)),
                        ),
                        (
                            CellPoint::new(0, 1),
                            CellId::from(PuyoCell::loose(piece.pivot)),
                        ),
                    ],
                )
            })
            .collect(),
    }
}

pub struct Sounds {
    pub music: &'static [u8],
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
    AudioTheme::new(config, &sfx)?
        .with_looping_game_music(sounds.music)?
        .with_game_over_music(sounds.game_over, None)?
        .with_victory_music(sounds.victory, None)
}

/// The HUD rows and the largest value each has to show.
///
/// The score, and nothing else - which is Tsu's own HUD, less the nuisance tray, and the tray
/// is drawn from the theme's own sprites rather than as a number. A chain announces itself
/// over the puyos that just went (`clear_popup` in [`crate::render`]) and the speed step is
/// something the player feels rather than reads, so neither is a row here. The engine's
/// `MetricKind::Chain` and `MetricKind::Level` both stay; this game simply shows neither.
pub const HUD_MAX: [(MetricKind, u32); 1] = [(MetricKind::Score, MAX_SCORE)];

#[cfg(test)]
mod tests {
    use super::*;

    /// every cell the board can draw has to be in the sheet, or it is drawn as nothing at
    /// all - and eighty of them are the link grid, which is exactly the mistake this is here
    /// to catch
    #[test]
    fn the_sheet_keys_every_cell_the_board_can_draw() {
        let cells = cells(
            8,
            |color, links| Point::new(color as i32, links.bits() as i32),
            Point::new(0, 0),
            [Point::new(0, 0); 3],
        );
        assert_eq!(cells.len(), PuyoColor::N * LinkMask::COUNT + 1 + 3);
        let ids: Vec<CellId> = cells.iter().map(|(id, _)| *id).collect();
        let mut unique = ids.clone();
        unique.sort_by_key(|id| id.0);
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "two cells share a key");
    }

    /// a pair is two colours out of five, so there are twenty five of them and not five
    #[test]
    fn the_queue_knows_every_pair_that_can_be_dealt() {
        let PreviewData::Compose { pieces } = previews() else {
            panic!("the previews are composed from the cells");
        };
        assert_eq!(pieces.len(), PuyoColor::N * PuyoColor::N);
        // ... and every one of them is drawn with cells the sheet has
        let sheet = cells(
            8,
            |color, links| Point::new(color as i32, links.bits() as i32),
            Point::new(0, 0),
            [Point::new(0, 0); 3],
        );
        for (_, cells) in pieces {
            for (_, id) in cells {
                assert!(
                    sheet.iter().any(|(key, _)| *key == id),
                    "{id:?} is not keyed"
                );
            }
        }
    }
}
