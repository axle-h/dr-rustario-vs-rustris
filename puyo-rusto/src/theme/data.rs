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

/// A chain step's grade, which is what a theme has a sound per.
///
/// One per step of a chain up to the third, and then everything longer with the biggest
/// clear there is - see `clear_class` in [`crate::render`], which reserves the last of them
/// for the particle field's silhouette interrupt.
pub const CLEAR_CLASSES: usize = 4;

/// Where every cell of a theme's sheet is.
///
/// A Puyo `CellId` is a colour, a four bit link mask and a skin slot, so the coloured cells
/// are a `colour x 16` grid and there is no way round authoring all eighty of them - see
/// `puyo-rusto/art/sprites.py`, which draws the particle theme's. That grid is then asked
/// for once per slot, because both players' sprites live in one sheet keyed by cell id: a
/// theme with one set of art hands back the same points every time and pays only for the
/// duplicate keys. `puyo` is asked for each cell in turn; `nuisance` and the three `tray`
/// symbols are single sprites.
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

/// The queue's twenty five pairs, per skin slot, composed from the cells rather than drawn
/// again.
///
/// A pair is two colours drawn from five, so dedicated preview sprites would be twenty five
/// of them - and every one would be the two cell sprites stacked, which is exactly what
/// [`PreviewData::Compose`] does for nothing. Since they cost only their keys there is a set
/// per slot as well, so the queue over a board is drawn from the same art as the board. The
/// pivot is the lower half, the way it sits on the board when the pair spawns.
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

/// What a retro panel casts on the wash behind it, which is what lifts it off one.
///
/// Both retro themes take the same shadow, because both are one panel standing on one
/// vignette and nothing about either says it should fall differently. Down and to the right,
/// which is a light over the panel's top left shoulder and is all the direction there is:
/// [`PanelShadow`] grows only that way, so there is none along the top edge, where a spawning
/// pair is the only thing standing on the scene. `margin` is the transparent air round the
/// panel inside its own box - the theme's `top_padding` and `bottom_padding`, and whatever
/// outer rock the rip cut off its sides - since none of that is art and none of it casts.
///
/// It is **not** painted into the panel art, which is where it would naturally go. A margin
/// painted round the art either comes straight off the board - every theme of a game is drawn
/// at the largest cell all of them can hold, and in a two player game the panels are sized by
/// the width they have - or has to be cut out of the panel first to pay for itself, which is
/// what `SIDE_TRIM` does and is a trade only the outermost rock could stand.
/// [`PanelShadow`] draws it at composite time instead, for nothing.
pub fn panel_shadow(margin: (u32, u32, u32, u32)) -> PanelShadow {
    PanelShadow {
        offset: (3, 3),
        spread: 5,
        color: Color::BLACK,
        alpha: 0xa0,
        margin,
    }
}

/// A track a match may be dealt: a one-shot lead-in, where the tune has one, and the part
/// that loops behind it forever.
///
/// The lead-in is optional because a console's own music often has none - Mean Bean Machine's
/// four stage tunes all loop from their first bar. What that game writes beside each of them
/// is the *stage announcement screen's* music, which is not the head of the stage tune and is
/// nine seconds long, so the theme says `None` here rather than open every match on it.
pub type MusicTrack = (Option<&'static [u8]>, &'static [u8]);

pub struct Sounds {
    /// this theme's level against the rest of the compendium - one of the gains below
    pub gain: i32,
    /// the tracks a match on this theme may be dealt - as many as this theme's own game
    /// wrote, which is not the same number for all of them
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

/// How loud each theme plays against the rest of the compendium, as a percentage.
///
/// **Puyo Rusto's own assets are mastered some eight decibels hotter than everything else in
/// the app**, music and effects alike: its music sits around -14 dBFS RMS where every theme of
/// Rustris and Dr. Rustario sits at -22, which is the house baseline
/// ([`engine::render::sound::AudioTheme::with_gain`]). Nobody had heard it, because Puyo does
/// not take a playlist turn yet and no other game's tune ever follows one of these; the moment
/// it does, this game is the loud one. Measured by `engine/art/audio_levels.py`, which reads
/// these constants back out of this file so the two cannot disagree.
///
/// It is a gain here rather than a re-cut because most of these files cannot be re-cut at all -
/// `art/sfx.py`'s rip and `art/music.py`'s are not on this machine - and because re-encoding a
/// lossy file to change its level is a worse answer than multiplying it on the way out.
///
/// One number per theme, applied to its music and its effects **together**, so each theme's own
/// internal balance survives being levelled. [`EFFECTS_TRIM`] is the other thing, and is what
/// says the effects sit wrong against the music rather than the whole theme sitting wrong
/// against the app.
pub const GENESIS_GAIN: i32 = 45;
pub const SNES_GAIN: i32 = 44;
pub const PARTICLE_GAIN: i32 = 39;
/// the menu screens, whose music comes off the same rip as the particle theme's
pub const MENU_GAIN: i32 = 38;

/// How loud this game's effects play against **its own music**, as a percentage.
///
/// **Measured, against the balance the other two games already strike.** Take a theme's effects
/// as RMS against the RMS of its music: every theme of Rustris and Dr. Rustario lands between
/// -7 and +2 dB, and Rustris's Game Boy theme - the one that sounds right - is at -2. The three
/// Puyo themes came off one Puyo Puyo Tetris rip whose effects are peakier than its own music,
/// by about the three decibels taken here, which puts all three inside that band: -3.7, -1.7
/// and -2.8 for `genesis`, `snes` and the particle theme.
///
/// It is a number here rather than a gain in the files because `art/sfx.py`'s rip is not on
/// this machine and cannot be re-cut, so the particle theme's own set has to be taken where it
/// stands - and it is that set every other theme is levelled against.
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

/// The HUD rows and the largest value each has to show.
///
/// The score and the speed step, which every one of the three source games prints - Kirby's
/// Avalanche and Mean Bean Machine both call it a *stage* and give it a box of its own, and
/// this game's menu offers it as the `level` to start on, so it is the same number under
/// three names. The chain is not a row: it announces itself over the puyos that just went
/// (`clear_popup` in [`crate::render`]), which is what Tsu does. `MetricKind::Chain` stays
/// on the game and is simply never drawn.
pub const HUD_MAX: [(MetricKind, u32); 2] = [
    (MetricKind::Score, MAX_SCORE),
    (MetricKind::Level, MAX_LEVEL),
];

/// where a theme puts each of them, in the order [`HUD_MAX`] names them
///
/// A retro theme places every row by hand, in its own panel's coordinates, so it needs one
/// snip per metric rather than the same snip for all of them - which is what mapping over
/// [`HUD_MAX`] gave, and why the level would have landed on top of the score.
pub fn hud(score: MetricSnips, level: MetricSnips) -> Vec<(MetricKind, MetricSnips)> {
    vec![(MetricKind::Score, score), (MetricKind::Level, level)]
}

#[cfg(test)]
mod tests {
    /// [`hud`] and [`HUD_MAX`] are two lists of the same rows and a theme uses both - the max
    /// to size the digits, the snip to place them - so a row in one and not the other is a
    /// metric drawn at the wrong width or not at all. They were one list until the level
    /// joined the score, which is when mapping a single snip over `HUD_MAX` started drawing
    /// both numbers on top of each other.
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

    /// every cell the board can draw has to be in the sheet, or it is drawn as nothing at
    /// all - and eighty of them are the link grid, once per player, which is exactly the
    /// mistake this is here to catch
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

    /// a pair is two colours out of five, so there are twenty five of them and not five -
    /// and a set of them per player, since the queue is drawn from the board's own art
    #[test]
    fn the_queue_knows_every_pair_that_can_be_dealt() {
        let PreviewData::Compose { pieces } = previews() else {
            panic!("the previews are composed from the cells");
        };
        assert_eq!(pieces.len(), PuyoSkin::COUNT * PuyoColor::N * PuyoColor::N);
        // ... and every one of them is drawn with cells the sheet has
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

    /// ... and out of the *right* art: a preview keyed for one player must not be composed
    /// from the other's cells, which is the whole point of the slot
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
