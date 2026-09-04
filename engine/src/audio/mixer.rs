//! A small software mixer: N one-shot effect voices plus one streamed music track
//! with intro→loop chaining. Pure code (no SDL) so it can be unit tested.

use std::sync::mpsc::Receiver;
use std::sync::Arc;

use super::MAX_VOLUME;

/// Anything that can produce interleaved stereo samples, e.g. a Vorbis stream.
pub trait MusicSource: Send {
    /// Fills `out` and returns the number of samples written; fewer than `out.len()` means end of stream.
    fn read(&mut self, out: &mut [i16]) -> usize;
    /// Rewinds to the start.
    fn reset(&mut self);
}

pub enum Command {
    PlaySound {
        pcm: Arc<Vec<i16>>,
        volume: i32,
    },
    /// Play `intro` once (if given) then `repeat` `loops` times (`-1` = forever, `0`/`1` = once).
    /// `gain` is the theme's own level for this track, as a percentage of the volume the
    /// config asks for - see [`crate::render::sound::AudioTheme::with_gain`].
    PlayMusic {
        intro: Option<Box<dyn MusicSource>>,
        repeat: Box<dyn MusicSource>,
        loops: i32,
        gain: i32,
    },
    PauseMusic,
    ResumeMusic,
    HaltMusic,
}

struct Voice {
    pcm: Arc<Vec<i16>>,
    position: usize,
    volume: i32,
    /// Monotonic start order, used to steal the oldest voice when all are busy.
    started: u64,
}

struct Music {
    source: Box<dyn MusicSource>,
    /// the theme's level for this track, as a percentage
    gain: i32,
    /// Plays remaining after the current one; `None` = forever.
    remaining: Option<u32>,
    /// Track to chain to when the current one ends (after an intro).
    next: Option<(Box<dyn MusicSource>, i32)>,
    paused: bool,
}

pub struct Mixer {
    commands: Receiver<Command>,
    voices: Vec<Option<Voice>>,
    started: u64,
    music: Option<Music>,
    music_volume: i32,
    scratch: Vec<i16>,
}

fn remaining_plays(loops: i32) -> Option<u32> {
    if loops < 0 {
        None
    } else {
        Some(loops.max(1) as u32 - 1)
    }
}

impl Mixer {
    pub fn new(commands: Receiver<Command>, channels: usize, music_volume: i32) -> Self {
        Self {
            commands,
            voices: (0..channels.max(1)).map(|_| None).collect(),
            started: 0,
            music: None,
            music_volume: music_volume.clamp(0, MAX_VOLUME),
            scratch: Vec::new(),
        }
    }

    fn apply(&mut self, command: Command) {
        match command {
            Command::PlaySound { pcm, volume } => {
                self.started += 1;
                let voice = Voice {
                    pcm,
                    position: 0,
                    volume,
                    started: self.started,
                };
                let slot = match self.voices.iter().position(Option::is_none) {
                    Some(free) => free,
                    None => self
                        .voices
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, v)| v.as_ref().map(|v| v.started))
                        .map(|(i, _)| i)
                        .unwrap_or(0),
                };
                self.voices[slot] = Some(voice);
            }
            Command::PlayMusic {
                intro,
                repeat,
                loops,
                gain,
            } => {
                self.music = Some(match intro {
                    Some(intro) => Music {
                        source: intro,
                        gain,
                        remaining: Some(0),
                        next: Some((repeat, loops)),
                        paused: false,
                    },
                    None => Music {
                        source: repeat,
                        gain,
                        remaining: remaining_plays(loops),
                        next: None,
                        paused: false,
                    },
                });
            }
            Command::PauseMusic => {
                if let Some(music) = self.music.as_mut() {
                    music.paused = true;
                }
            }
            Command::ResumeMusic => {
                if let Some(music) = self.music.as_mut() {
                    music.paused = false;
                }
            }
            Command::HaltMusic => self.music = None,
        }
    }

    /// Fills `out` with the next block of interleaved stereo samples.
    pub fn mix(&mut self, out: &mut [i16]) {
        while let Ok(command) = self.commands.try_recv() {
            self.apply(command);
        }

        let mut accumulator = vec![0i32; out.len()];
        self.mix_music(&mut accumulator);
        for slot in self.voices.iter_mut() {
            if let Some(voice) = slot {
                let samples = &voice.pcm[voice.position.min(voice.pcm.len())..];
                let n = samples.len().min(accumulator.len());
                for (acc, &s) in accumulator.iter_mut().zip(&samples[..n]) {
                    *acc += s as i32 * voice.volume / MAX_VOLUME;
                }
                voice.position += n;
                if voice.position >= voice.pcm.len() {
                    *slot = None;
                }
            }
        }
        for (o, acc) in out.iter_mut().zip(accumulator) {
            *o = acc.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        }
    }

    fn mix_music(&mut self, accumulator: &mut [i32]) {
        let Some(mut music) = self.music.take() else {
            return;
        };
        if music.paused {
            self.music = Some(music);
            return;
        }
        self.scratch.clear();
        self.scratch.resize(accumulator.len(), 0);
        let mut written = 0;
        let mut empty_reads = 0;
        let mut finished = false;
        while written < accumulator.len() && !finished {
            let n = music.source.read(&mut self.scratch[written..]);
            written += n;
            // Guard against a source that yields nothing even after a reset.
            empty_reads = if n == 0 { empty_reads + 1 } else { 0 };
            if written < accumulator.len() {
                // End of the current track: loop, chain to the next one, or stop.
                match music.remaining {
                    None => music.source.reset(),
                    Some(left) if left > 0 => {
                        music.remaining = Some(left - 1);
                        music.source.reset();
                    }
                    Some(_) => match music.next.take() {
                        Some((source, loops)) => {
                            music.source = source;
                            music.remaining = remaining_plays(loops);
                            empty_reads = 0;
                        }
                        None => finished = true,
                    },
                }
                if empty_reads >= 2 {
                    finished = true;
                }
            }
        }
        // the config's volume and the theme's own gain, which is why the two are multiplied
        // rather than one of them winning
        let volume = self.music_volume * music.gain / 100;
        for (acc, &s) in accumulator.iter_mut().zip(&self.scratch[..written]) {
            *acc += s as i32 * volume / MAX_VOLUME;
        }
        if !finished {
            self.music = Some(music);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{channel, Sender};

    /// A track of `len` samples all equal to `value`.
    struct Tone {
        value: i16,
        len: usize,
        pos: usize,
    }

    fn tone(value: i16, len: usize) -> Box<dyn MusicSource> {
        Box::new(Tone { value, len, pos: 0 })
    }

    impl MusicSource for Tone {
        fn read(&mut self, out: &mut [i16]) -> usize {
            let n = out.len().min(self.len - self.pos);
            out[..n].fill(self.value);
            self.pos += n;
            n
        }
        fn reset(&mut self) {
            self.pos = 0;
        }
    }

    fn mixer(channels: usize) -> (Sender<Command>, Mixer) {
        mixer_with_volume(channels, MAX_VOLUME)
    }

    fn mixer_with_volume(channels: usize, music_volume: i32) -> (Sender<Command>, Mixer) {
        let (tx, rx) = channel();
        (tx, Mixer::new(rx, channels, music_volume))
    }

    fn mix(m: &mut Mixer, n: usize) -> Vec<i16> {
        let mut out = vec![0i16; n];
        m.mix(&mut out);
        out
    }

    #[test]
    fn idle_mixer_is_silent() {
        let (_tx, mut m) = mixer(4);
        assert!(mix(&mut m, 64).iter().all(|&s| s == 0));
    }

    #[test]
    fn sound_is_scaled_by_volume_and_stops_at_end() {
        let (tx, mut m) = mixer(4);
        tx.send(Command::PlaySound {
            pcm: Arc::new(vec![1000; 6]),
            volume: MAX_VOLUME / 2,
        })
        .unwrap();
        let out = mix(&mut m, 8);
        assert_eq!(out, vec![500, 500, 500, 500, 500, 500, 0, 0]);
        assert!(mix(&mut m, 8).iter().all(|&s| s == 0));
    }

    #[test]
    fn mixing_saturates_instead_of_wrapping() {
        let (tx, mut m) = mixer(4);
        for _ in 0..3 {
            tx.send(Command::PlaySound {
                pcm: Arc::new(vec![30000; 4]),
                volume: MAX_VOLUME,
            })
            .unwrap();
        }
        assert_eq!(mix(&mut m, 4), vec![i16::MAX; 4]);
    }

    #[test]
    fn oldest_voice_is_stolen_when_full() {
        let (tx, mut m) = mixer(2);
        for v in [1, 2, 3] {
            tx.send(Command::PlaySound {
                pcm: Arc::new(vec![v; 4]),
                volume: MAX_VOLUME,
            })
            .unwrap();
        }
        // voices 2 and 3 survive
        assert_eq!(mix(&mut m, 4), vec![5; 4]);
    }

    #[test]
    fn music_loops_forever_and_honours_volume() {
        let (tx, mut m) = mixer_with_volume(1, MAX_VOLUME / 4);
        tx.send(Command::PlayMusic {
            intro: None,
            repeat: tone(400, 3),
            loops: -1,
            gain: 100,
        })
        .unwrap();
        assert_eq!(mix(&mut m, 8), vec![100; 8]);
        assert_eq!(mix(&mut m, 8), vec![100; 8]);
    }

    /// the config's dial and the theme's own level multiply, so a theme levelled against the
    /// house is still levelled when the player moves the slider
    #[test]
    fn a_themes_music_gain_multiplies_the_configured_volume() {
        let (tx, mut m) = mixer_with_volume(1, MAX_VOLUME / 2);
        tx.send(Command::PlayMusic {
            intro: None,
            repeat: tone(400, 3),
            loops: -1,
            gain: 50,
        })
        .unwrap();
        assert_eq!(mix(&mut m, 4), vec![100; 4]);
    }

    #[test]
    fn music_once_then_silence() {
        let (tx, mut m) = mixer(1);
        tx.send(Command::PlayMusic {
            intro: None,
            repeat: tone(7, 5),
            loops: 1,
            gain: 100,
        })
        .unwrap();
        assert_eq!(mix(&mut m, 8), vec![7, 7, 7, 7, 7, 0, 0, 0]);
        assert_eq!(mix(&mut m, 4), vec![0; 4]);
    }

    #[test]
    fn music_plays_n_times() {
        let (tx, mut m) = mixer(1);
        tx.send(Command::PlayMusic {
            intro: None,
            repeat: tone(1, 2),
            loops: 3,
            gain: 100,
        })
        .unwrap();
        assert_eq!(mix(&mut m, 8), vec![1, 1, 1, 1, 1, 1, 0, 0]);
    }

    #[test]
    fn intro_plays_once_then_repeat_loops() {
        let (tx, mut m) = mixer(1);
        tx.send(Command::PlayMusic {
            intro: Some(tone(9, 3)),
            repeat: tone(2, 2),
            loops: -1,
            gain: 100,
        })
        .unwrap();
        assert_eq!(mix(&mut m, 9), vec![9, 9, 9, 2, 2, 2, 2, 2, 2]);
        assert_eq!(mix(&mut m, 4), vec![2; 4]);
    }

    #[test]
    fn halt_during_intro_cancels_queued_repeat() {
        let (tx, mut m) = mixer(1);
        tx.send(Command::PlayMusic {
            intro: Some(tone(9, 10)),
            repeat: tone(2, 2),
            loops: -1,
            gain: 100,
        })
        .unwrap();
        assert_eq!(mix(&mut m, 2), vec![9, 9]);
        tx.send(Command::HaltMusic).unwrap();
        assert_eq!(mix(&mut m, 4), vec![0; 4]);
    }

    #[test]
    fn pause_and_resume_music_without_losing_position() {
        let (tx, mut m) = mixer(1);
        tx.send(Command::PlayMusic {
            intro: None,
            repeat: tone(5, 4),
            loops: 1,
            gain: 100,
        })
        .unwrap();
        assert_eq!(mix(&mut m, 2), vec![5, 5]);
        tx.send(Command::PauseMusic).unwrap();
        assert_eq!(mix(&mut m, 2), vec![0, 0]);
        tx.send(Command::ResumeMusic).unwrap();
        assert_eq!(mix(&mut m, 4), vec![5, 5, 0, 0]);
    }

    #[test]
    fn new_music_replaces_current() {
        let (tx, mut m) = mixer(1);
        tx.send(Command::PlayMusic {
            intro: None,
            repeat: tone(1, 100),
            loops: -1,
            gain: 100,
        })
        .unwrap();
        mix(&mut m, 4);
        tx.send(Command::PlayMusic {
            intro: None,
            repeat: tone(3, 100),
            loops: -1,
            gain: 100,
        })
        .unwrap();
        assert_eq!(mix(&mut m, 4), vec![3; 4]);
    }
}
