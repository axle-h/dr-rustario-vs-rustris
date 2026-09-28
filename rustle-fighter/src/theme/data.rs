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

/// A break's grade, one sound each: chain passes one to three, then the long chains.
pub const CLEAR_CLASSES: usize = 4;

pub type MusicTrack = (Option<&'static [u8]>, &'static [u8]);

/// Where every cell of a theme's sheet is: a row per colour and a column per sprite.
/// `art/rip.py` cuts exactly the set [`GemSprite::all`] names, in that order.
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

/// The NEXT box's pairs, composed from the cells with the pivot as the lower half, as it
/// spawns; both halves draw unjoined, since a gem joins a power gem only once landed.
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

/// The panel's shadow on the wall, down and to the right like every shadow in the app;
/// `margin` is the transparent border inside the panel's box, which casts nothing.
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
    /// one per [`CLEAR_CLASSES`]
    pub pop: [&'static [u8]; CLEAR_CLASSES],
    pub attack_sent: &'static [u8],
    pub receive_counter: &'static [u8],
    pub speed_up: &'static [u8],
    pub paused: &'static [u8],
    pub victory: &'static [u8],
    pub game_over: &'static [u8],
}

/// Unity, because `art/music.py` and `art/sfx.py` cut the audio to the house level;
/// `engine/art/audio_levels.py` checks it.
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

    /// every sprite in `GemSprite::all` is keyed exactly once
    #[test]
    fn every_sprite_the_game_can_report_is_keyed_once() {
        let keyed = cells(16, |_| Point::new(0, 0));
        assert_eq!(keyed.len(), GemSprite::all().len());
        let ids: std::collections::HashSet<CellId> = keyed.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids.len(), keyed.len(), "and no two share a key");
    }
}
