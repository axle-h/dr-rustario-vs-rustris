use std::rc::Rc;

use crate::audio::{self, Music, Sound};
use crate::config::AudioConfig;

/// A piece of music with an optional one-shot intro before the looping part.
pub struct StructuredMusic {
    intro: Option<Music>,
    repeating: Music,
    loops: i32,
}

impl StructuredMusic {
    pub fn new(intro: &'static [u8], repeating: &'static [u8]) -> Result<Self, String> {
        Ok(Self {
            intro: Some(Music::load(intro)?),
            repeating: Music::load(repeating)?,
            loops: -1,
        })
    }

    pub fn once(repeating: &'static [u8]) -> Result<Self, String> {
        Ok(Self {
            intro: None,
            repeating: Music::load(repeating)?,
            loops: 1,
        })
    }

    pub fn repeat(repeating: &'static [u8]) -> Result<Self, String> {
        Ok(Self {
            intro: None,
            repeating: Music::load(repeating)?,
            loops: -1,
        })
    }

    pub fn into_rc(self) -> Rc<Self> {
        Rc::new(self)
    }

    pub fn play(music: &Rc<StructuredMusic>) -> Result<(), String> {
        audio::play_music(music.intro, music.repeating, music.loops)
    }

    /// Plays, unless an endless loop of this same music is already playing, which is left
    /// alone: menus that share a tune switch without restarting it.
    pub fn play_unless_current(music: &Rc<StructuredMusic>) -> Result<(), String> {
        audio::play_music_unless_current(music.intro, music.repeating, music.loops)
    }

    pub fn maybe_play(music: Option<&Rc<StructuredMusic>>) -> Result<(), String> {
        if let Some(music) = music {
            StructuredMusic::play(music)
        } else {
            audio::halt_music()
        }
    }
}

pub trait LoadSound {
    fn load_sound(&self, buffer: &'static [u8]) -> Result<Sound, String>;
}

impl LoadSound for AudioConfig {
    fn load_sound(&self, buffer: &'static [u8]) -> Result<Sound, String> {
        Sound::load(buffer, self.effects_volume())
    }
}
