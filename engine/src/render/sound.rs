use crate::audio::theme::{LoadSound, StructuredMusic};
use crate::audio::{self, Sound};
use crate::config::AudioConfig;
use crate::game::GameEvent;
use rand::{Rng, RngExt};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

/// Which sound effect a theme plays for a game event. `Clear(class)` lets a game grade its clears
/// without the theme knowing the rules; see [`crate::render::GameRender::clear_class`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SfxKey {
    Move,
    Rotate,
    Lock,
    /// loose blocks settled after a clear
    Settle,
    HardDrop,
    Hold,
    Clear(u16),
    AttackSent,
    AttackReceived,
    SpeedUp,
    Paused,
    StageComplete,
}

pub struct AudioTheme {
    sfx: HashMap<SfxKey, Sound>,
    /// every track this theme may play a match on; with several, one is dealt per match by
    /// [`Self::deal_game_music`]
    game_music: Vec<Rc<StructuredMusic>>,
    /// which of them is playing; a `Cell` because themes are shared behind `&self`
    game_music_index: Cell<usize>,
    game_over_music: Option<Rc<StructuredMusic>>,
    next_stage_music: Option<Rc<StructuredMusic>>,
    victory_music: Option<Rc<StructuredMusic>>,
    /// what [`Self::with_gain`] set, so a track added after it is levelled too
    music_gain: i32,
}

impl AudioTheme {
    pub fn new(config: AudioConfig, sfx: &[(SfxKey, &'static [u8])]) -> Result<Self, String> {
        let mut sounds = HashMap::new();
        for (key, bytes) in sfx.iter().copied() {
            let mut sound = config.load_sound(bytes)?;
            if key == SfxKey::StageComplete {
                // jingles play over music
                sound.set_volume(sound.volume() / 2);
            }
            sounds.insert(key, sound);
        }
        Ok(Self {
            sfx: sounds,
            game_music: Vec::new(),
            game_music_index: Cell::new(0),
            game_over_music: None,
            next_stage_music: None,
            victory_music: None,
            music_gain: 100,
        })
    }

    /// `repeating` is the looping part after a one-shot `music` intro; without it `music` plays
    /// through once.
    fn music<R: Into<Option<&'static [u8]>>>(
        music: &'static [u8],
        repeating: R,
    ) -> Result<Rc<StructuredMusic>, String> {
        Ok(match repeating.into() {
            Some(repeating) => StructuredMusic::new(music, repeating)?,
            None => StructuredMusic::once(music)?,
        }
        .into_rc())
    }

    pub fn with_game_music(
        self,
        intro: &'static [u8],
        repeating: &'static [u8],
    ) -> Result<Self, String> {
        self.with_game_music_track(Some(intro), repeating)
    }

    /// one track that loops from the start
    pub fn with_looping_game_music(self, music: &'static [u8]) -> Result<Self, String> {
        self.with_game_music_track(None, music)
    }

    /// another track this theme may be dealt a match on, appended to the ones already offered
    pub fn with_game_music_track(
        mut self,
        intro: Option<&'static [u8]>,
        repeating: &'static [u8],
    ) -> Result<Self, String> {
        let track = match intro {
            Some(intro) => StructuredMusic::new(intro, repeating)?,
            None => StructuredMusic::repeat(repeating)?,
        }
        .into_rc();
        track.set_gain(self.music_gain);
        self.game_music.push(track);
        Ok(self)
    }

    /// Levels this whole theme, music and effects together, against the house baseline of music at
    /// -22 dBFS RMS; `engine/art/audio_levels.py` measures it. Use [`Self::with_effects_at`] when a
    /// theme's effects sit wrong against its own music.
    pub fn with_gain(mut self, percent: i32) -> Self {
        self.sfx = self
            .sfx
            .drain()
            .map(|(key, sound)| (key, sound.with_gain(percent)))
            .collect();
        self.music_gain = percent;
        for music in self.music_tracks() {
            music.set_gain(percent);
        }
        self
    }

    /// Plays this theme's effects at `percent` of what its music plays at. Over 100 scales the
    /// samples, since the config's volume dial has no headroom above it.
    pub fn with_effects_at(mut self, percent: i32) -> Self {
        self.sfx = self
            .sfx
            .drain()
            .map(|(key, sound)| (key, sound.with_gain(percent)))
            .collect();
        self
    }

    /// every track this theme can play, whatever the occasion
    fn music_tracks(&self) -> Vec<&Rc<StructuredMusic>> {
        self.game_music
            .iter()
            .chain(self.game_over_music.iter())
            .chain(self.next_stage_music.iter())
            .chain(self.victory_music.iter())
            .collect()
    }

    /// how many tracks a match may be played on
    pub fn game_music_count(&self) -> usize {
        self.game_music.len()
    }

    /// Deals the track the next match is played on.
    pub fn deal_game_music(&self, rng: &mut impl Rng) {
        self.game_music_index.set(if self.game_music.len() > 1 {
            rng.random_range(0..self.game_music.len())
        } else {
            0
        });
    }

    pub fn with_game_over_music<R: Into<Option<&'static [u8]>>>(
        mut self,
        music: &'static [u8],
        repeating: R,
    ) -> Result<Self, String> {
        let track = Self::music(music, repeating)?;
        track.set_gain(self.music_gain);
        self.game_over_music = Some(track);
        Ok(self)
    }

    pub fn with_next_stage_music<R: Into<Option<&'static [u8]>>>(
        mut self,
        music: &'static [u8],
        repeating: R,
    ) -> Result<Self, String> {
        let track = Self::music(music, repeating)?;
        track.set_gain(self.music_gain);
        self.next_stage_music = Some(track);
        Ok(self)
    }

    pub fn with_victory_music<R: Into<Option<&'static [u8]>>>(
        mut self,
        music: &'static [u8],
        repeating: R,
    ) -> Result<Self, String> {
        let track = Self::music(music, repeating)?;
        track.set_gain(self.music_gain);
        self.victory_music = Some(track);
        Ok(self)
    }

    /// one track that loops from the start
    pub fn with_looping_victory_music(mut self, music: &'static [u8]) -> Result<Self, String> {
        let track = StructuredMusic::repeat(music)?.into_rc();
        track.set_gain(self.music_gain);
        self.victory_music = Some(track);
        Ok(self)
    }

    pub fn play_game_music(&self) -> Result<(), String> {
        StructuredMusic::maybe_play(self.game_music.get(self.game_music_index.get()))
    }

    pub fn play_game_over_music(&self) -> Result<(), String> {
        StructuredMusic::maybe_play(self.game_over_music.as_ref())
    }

    pub fn play_next_stage_music(&self) -> Result<(), String> {
        StructuredMusic::maybe_play(self.next_stage_music.as_ref())
    }

    pub fn play_victory_music(&self) -> Result<(), String> {
        StructuredMusic::maybe_play(self.victory_music.as_ref())
    }

    pub fn play_stage_complete_jingle(&self) -> Result<(), String> {
        self.play(SfxKey::StageComplete)
    }

    pub fn pause_music(&self) -> Result<(), String> {
        audio::pause_music()
    }

    fn play(&self, key: SfxKey) -> Result<(), String> {
        match self.sfx.get(&key) {
            Some(sound) => sound.play(),
            None => Ok(()),
        }
    }

    /// `clear_class` grades a `Clear` event for this game, see [`SfxKey::Clear`]
    pub fn receive_event(&self, event: &GameEvent, clear_class: u16) -> Result<(), String> {
        match event {
            GameEvent::Move => self.play(SfxKey::Move),
            GameEvent::Rotate => self.play(SfxKey::Rotate),
            GameEvent::Hold => self.play(SfxKey::Hold),
            GameEvent::Lock { .. } => self.play(SfxKey::Lock),
            GameEvent::Settle => self.play(SfxKey::Settle),
            GameEvent::HardDrop { .. } => self.play(SfxKey::HardDrop),
            GameEvent::Clear { .. } => self.play(SfxKey::Clear(clear_class)),
            GameEvent::AttackSent(_) => self.play(SfxKey::AttackSent),
            GameEvent::AttackReceived { .. } => self.play(SfxKey::AttackReceived),
            GameEvent::SpeedUp => self.play(SfxKey::SpeedUp),
            GameEvent::Paused => {
                audio::pause_music()?;
                self.play(SfxKey::Paused)
            }
            GameEvent::UnPaused => audio::resume_music(),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    const ONE: &[u8] = include_bytes!("../audio/test/stereo.ogg");
    const TWO: &[u8] = include_bytes!("../audio/test/mono.ogg");

    fn theme(tracks: usize) -> AudioTheme {
        let config = AudioConfig {
            music_volume: 0.0,
            effects_volume: 0.0,
        };
        let mut audio = AudioTheme::new(config, &[]).unwrap();
        for track in 0..tracks {
            let bytes = if track % 2 == 0 { ONE } else { TWO };
            audio = audio.with_game_music_track(None, bytes).unwrap();
        }
        audio
    }

    /// Over enough matches every track of a soundtrack is dealt.
    #[test]
    fn a_random_choice_deals_every_track_and_only_those() {
        let audio = theme(4);
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let mut seen = vec![false; 4];
        for _ in 0..100 {
            audio.deal_game_music(&mut rng);
            seen[audio.game_music_index.get()] = true;
        }
        assert!(seen.into_iter().all(|s| s));
    }

    /// A theme with one tune is always dealt it.
    #[test]
    fn one_track_is_the_one_that_is_dealt() {
        let audio = theme(1);
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        for _ in 0..8 {
            audio.deal_game_music(&mut rng);
            assert_eq!(audio.game_music_index.get(), 0);
        }
    }
}
