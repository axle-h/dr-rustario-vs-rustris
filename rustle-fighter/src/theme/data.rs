//! Helpers that describe Super Rustle Fighter's sprites and sounds to the engine's theme
//! builders.

use crate::game::cell::{GemSprite, PowerMask};
use engine::config::AudioConfig;
use engine::game::CellId;
use engine::render::sound::{AudioTheme, SfxKey};
use engine::render::sprite_sheet::{CellSpriteData, PreviewData};
use engine::render::PanelShadow;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};

/// A break's grade, which is what a theme has a sound per.
///
/// One per pass of a chain up to the third, and then everything longer with the biggest break
/// there is - see `clear_class` in [`crate::render`], which reserves the last for the particle
/// field's silhouette interrupt.
pub const CLEAR_CLASSES: usize = 4;

pub type MusicTrack = (Option<&'static [u8]>, &'static [u8]);

/// Where every cell of a theme's sheet is.
///
/// A gem's [`CellId`] is a colour and either a power gem edge mask or a countdown digit, so a
/// theme's sheet is a row per colour and a column per sprite - and there is no way round
/// authoring all of them, because a power gem has to be drawn joined to the rest of its
/// rectangle. `rustle-fighter/art/rip.py` cuts exactly the set [`GemSprite::all`] names, in
/// exactly that order.
pub fn cells(block_size: u32, at: impl Fn(GemSprite) -> Point) -> Vec<(CellId, CellSpriteData)> {
    GemSprite::all()
        .into_iter()
        .map(|sprite| {
            let point = at(sprite);
            (
                sprite.id(),
                CellSpriteData::new(Rect::new(point.x, point.y, block_size, block_size)),
            )
        })
        .collect()
}

/// The NEXT box's pairs, composed from the cells rather than drawn again.
///
/// A pair is two halves out of nine, so dedicated preview sprites would be seventy two of
/// them - and every one would be the two cell sprites stacked, which is what
/// [`PreviewData::Compose`] does for nothing. The pivot is the **lower** half, the way it sits
/// on the board when the pair spawns, and both halves draw unjoined, because a gem joins a
/// power gem only once it has landed.
pub fn previews() -> PreviewData {
    PreviewData::Compose {
        pieces: crate::game::cell::GemPair::all()
            .into_iter()
            .map(|piece| {
                (
                    piece.into(),
                    vec![
                        (
                            engine::game::geometry::Point::new(0, 0),
                            piece.child.gem().id(PowerMask::NONE),
                        ),
                        (
                            engine::game::geometry::Point::new(0, 1),
                            piece.pivot.gem().id(PowerMask::NONE),
                        ),
                    ],
                )
            })
            .collect(),
    }
}

/// What the panel casts on the brick wall behind it, which is what lifts it off one.
///
/// Down and to the right, which is where every shadow in this compendium falls. `margin` is
/// the transparent air round the panel inside its own box, since none of that is art and none
/// of it casts.
pub fn panel_shadow(margin: (u32, u32, u32, u32)) -> PanelShadow {
    PanelShadow {
        offset: (3, 3),
        spread: 5,
        color: Color::BLACK,
        alpha: 0xa0,
        margin,
    }
}

pub struct Sounds {
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
    pub receive_counter: &'static [u8],
    pub speed_up: &'static [u8],
    pub paused: &'static [u8],
    pub victory: &'static [u8],
    pub game_over: &'static [u8],
}

/// **This theme is levelled by its own scripts rather than here.**
///
/// `art/music.py` puts every track at the house baseline of -22 dBFS RMS and `art/sfx.py` puts
/// every effect within four decibels of it, so nothing needs correcting at build time and the
/// gain is a plain hundred. Puyo Rusto's rips needed eight decibels off because they came
/// mastered hot and were taken as they came; these were cut here, so they were cut level.
/// `engine/art/audio_levels.py` is what checks that claim across the whole app.
pub const ARCADE_GAIN: i32 = 100;

pub fn audio(config: AudioConfig, sounds: Sounds) -> Result<AudioTheme, String> {
    let mut sfx = vec![
        (SfxKey::Move, sounds.move_pair),
        (SfxKey::Rotate, sounds.rotate),
        (SfxKey::Lock, sounds.lock),
        (SfxKey::Settle, sounds.settle),
        (SfxKey::HardDrop, sounds.hard_drop),
        (SfxKey::AttackSent, sounds.attack_sent),
        (SfxKey::AttackReceived, sounds.receive_counter),
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
    let mut audio = AudioTheme::new(config, &sfx)?.with_gain(sounds.gain);
    for (intro, repeat) in sounds.music {
        audio = audio.with_game_music_track(*intro, repeat)?;
    }
    audio
        .with_game_over_music(sounds.game_over, None)?
        .with_victory_music(sounds.victory, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the sheet is `rip.py`'s and no Rust can check the script, but the theme and the script
    /// have to agree about how many sprites there are or the board draws the wrong gem
    #[test]
    fn every_sprite_the_game_can_report_is_keyed_once() {
        let keyed = cells(16, |_| Point::new(0, 0));
        assert_eq!(keyed.len(), GemSprite::all().len());
        let ids: std::collections::HashSet<CellId> = keyed.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids.len(), keyed.len(), "and no two share a key");
    }
}
