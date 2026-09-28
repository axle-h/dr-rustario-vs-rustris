use std::cell::Cell;
use std::rc::Rc;

use crate::audio::{self, Music, Sound};
use crate::config::AudioConfig;

/// A piece of music with an optional one-shot intro before the looping part.
pub struct StructuredMusic {
    intro: Option<Music>,
    repeating: Music,
    loops: i32,
    /// this theme's level for the track, as a percentage of the config's music volume
    gain: Cell<i32>,
}

impl StructuredMusic {
    pub fn new(intro: &'static [u8], repeating: &'static [u8]) -> Result<Self, String> {
        Ok(Self {
            intro: Some(Music::load(intro)?),
            repeating: Music::load(repeating)?,
            loops: -1,
            gain: Cell::new(100),
        })
    }

    pub fn once(repeating: &'static [u8]) -> Result<Self, String> {
        Ok(Self {
            intro: None,
            repeating: Music::load(repeating)?,
            loops: 1,
            gain: Cell::new(100),
        })
    }

    pub fn repeat(repeating: &'static [u8]) -> Result<Self, String> {
        Ok(Self {
            intro: None,
            repeating: Music::load(repeating)?,
            loops: -1,
            gain: Cell::new(100),
        })
    }

    pub fn into_rc(self) -> Rc<Self> {
        Rc::new(self)
    }

    /// A `Cell` because a built theme lives on as a `&'static` and is levelled through its `Rc`.
    pub fn set_gain(&self, percent: i32) {
        self.gain.set(percent);
    }

    pub fn play(music: &Rc<StructuredMusic>) -> Result<(), String> {
        audio::play_music(music.intro, music.repeating, music.loops, music.gain.get())
    }

    /// Plays, unless an endless loop of this same music is already playing.
    pub fn play_unless_current(music: &Rc<StructuredMusic>) -> Result<(), String> {
        audio::play_music_unless_current(
            music.intro,
            music.repeating,
            music.loops,
            music.gain.get(),
        )
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
