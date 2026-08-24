//! Audio output: a hand-rolled mixer fed through SDL's core audio callback.
//! Effects are decoded up front; music is streamed from the embedded OGG files.

pub mod theme;
mod decode;
mod mixer;

use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, OnceLock};

use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use sdl2::AudioSubsystem;

use decode::VorbisStream;
use mixer::{Command, Mixer};

/// Volume scale shared with the config (0..=128, as SDL_mixer used).
pub const MAX_VOLUME: i32 = 128;
pub const SAMPLE_RATE: u32 = 44_100;
const BUFFER_FRAMES: u16 = 512;

static HANDLE: OnceLock<Mutex<Sender<Command>>> = OnceLock::new();

struct Callback {
    mixer: Mixer,
}

impl AudioCallback for Callback {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        self.mixer.mix(out);
    }
}

/// The open audio device; dropping it stops playback.
pub struct Audio {
    _device: AudioDevice<Callback>,
}

impl Audio {
    pub fn open(subsystem: &AudioSubsystem, channels: u32, music_volume: i32) -> Result<Self, String> {
        let (sender, receiver) = channel();
        let spec = AudioSpecDesired {
            freq: Some(SAMPLE_RATE as i32),
            channels: Some(2),
            samples: Some(BUFFER_FRAMES),
        };
        let device = subsystem.open_playback(None, &spec, |obtained| {
            if obtained.freq != SAMPLE_RATE as i32 || obtained.channels != 2 {
                eprintln!(
                    "warning: audio device opened at {} Hz x{} rather than {SAMPLE_RATE} Hz stereo",
                    obtained.freq, obtained.channels
                );
            }
            Callback {
                mixer: Mixer::new(receiver, channels as usize, music_volume),
            }
        })?;
        device.resume();
        if HANDLE.set(Mutex::new(sender)).is_err() {
            return Err("audio already opened".to_string());
        }
        Ok(Self { _device: device })
    }
}

fn send(command: Command) -> Result<(), String> {
    let handle = HANDLE
        .get()
        .ok_or_else(|| "audio not opened".to_string())?;
    handle
        .lock()
        .map_err(|_| "audio handle poisoned".to_string())?
        .send(command)
        .map_err(|_| "audio device closed".to_string())
}

/// A fully decoded sound effect.
pub struct Sound {
    pcm: Arc<Vec<i16>>,
    volume: i32,
}

impl Sound {
    pub fn load(bytes: &'static [u8], volume: i32) -> Result<Self, String> {
        Ok(Self {
            pcm: Arc::new(decode::decode_all(bytes)?),
            volume: volume.clamp(0, MAX_VOLUME),
        })
    }

    pub fn volume(&self) -> i32 {
        self.volume
    }

    pub fn set_volume(&mut self, volume: i32) {
        self.volume = volume.clamp(0, MAX_VOLUME);
    }

    pub fn play(&self) -> Result<(), String> {
        send(Command::PlaySound {
            pcm: self.pcm.clone(),
            volume: self.volume,
        })
    }
}

/// An embedded music file, validated as decodable at construction.
#[derive(Copy, Clone)]
pub struct Music(&'static [u8]);

impl Music {
    pub fn load(bytes: &'static [u8]) -> Result<Self, String> {
        VorbisStream::new(bytes)?;
        Ok(Self(bytes))
    }

    fn stream(self) -> Box<dyn mixer::MusicSource> {
        Box::new(VorbisStream::new(self.0).expect("music validated at load"))
    }

    /// identity of the embedded bytes: enough to recognise the same track played again
    fn id(self) -> (usize, usize) {
        (self.0.as_ptr() as usize, self.0.len())
    }
}

/// A piece of music as requested: (intro, repeat, loops), each track identified by its
/// embedded bytes.
type MusicId = (Option<(usize, usize)>, (usize, usize), i32);

/// What `play_music` last started, so an identical endless loop can be recognised and
/// left playing. Never ends by itself (only endless loops are ever matched), so it is
/// invalidated only by `play_music` and `halt_music`.
static CURRENT_MUSIC: Mutex<Option<MusicId>> = Mutex::new(None);

fn music_id(intro: Option<Music>, repeat: Music, loops: i32) -> MusicId {
    (intro.map(Music::id), repeat.id(), loops)
}

fn current_music() -> Result<std::sync::MutexGuard<'static, Option<MusicId>>, String> {
    CURRENT_MUSIC
        .lock()
        .map_err(|_| "current music poisoned".to_string())
}

/// Plays `intro` once (if any) then `repeat` `loops` times (`-1` forever), replacing any current music.
pub fn play_music(intro: Option<Music>, repeat: Music, loops: i32) -> Result<(), String> {
    *current_music()? = Some(music_id(intro, repeat, loops));
    send(Command::PlayMusic {
        intro: intro.map(Music::stream),
        repeat: repeat.stream(),
        loops,
    })
}

/// Like [`play_music`], except that if this exact endlessly-looping music is already
/// playing it is left alone rather than restarted, so menus that share a tune switch
/// seamlessly.
pub fn play_music_unless_current(intro: Option<Music>, repeat: Music, loops: i32) -> Result<(), String> {
    if loops < 0 && *current_music()? == Some(music_id(intro, repeat, loops)) {
        // already playing this loop: just make sure it is not paused
        return resume_music();
    }
    play_music(intro, repeat, loops)
}

pub fn pause_music() -> Result<(), String> {
    send(Command::PauseMusic)
}

pub fn resume_music() -> Result<(), String> {
    send(Command::ResumeMusic)
}

pub fn halt_music() -> Result<(), String> {
    *current_music()? = None;
    send(Command::HaltMusic)
}
