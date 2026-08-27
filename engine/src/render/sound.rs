use crate::app::MusicChoice;
use crate::audio::theme::{LoadSound, StructuredMusic};
use crate::audio::{self, Sound};
use crate::config::AudioConfig;
use crate::game::GameEvent;
use rand::{Rng, RngExt};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

/// Which sound effect a theme plays for a game event. `Clear(class)` lets a game grade its
/// clears (a tetris vs a single, a virus vs a vitamin) without the theme knowing the rules:
/// see [`crate::render::GameRender::clear_class`].
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
    /// every track this theme will play a match on. Most themes offer one; a theme that
    /// offers several has one of them chosen per match by [`Self::choose_game_music`]
    game_music: Vec<Rc<StructuredMusic>>,
    /// which of them is playing. A `Cell` because themes are built once and lived on as
    /// `&'static`, so every play site holds a `&self` - the same reason the fill font of
    /// [`crate::render::Theme`] sits in a `RefCell`
    game_music_index: Cell<usize>,
    game_over_music: Option<Rc<StructuredMusic>>,
    next_stage_music: Option<Rc<StructuredMusic>>,
    victory_music: Option<Rc<StructuredMusic>>,
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
        self.with_game_music_choice(Some(intro), repeating)
    }

    /// one track that loops from the start
    pub fn with_looping_game_music(self, music: &'static [u8]) -> Result<Self, String> {
        self.with_game_music_choice(None, music)
    }

    /// another track this theme may play a match on, appended to the ones already offered:
    /// a game with a soundtrack rather than a tune calls this once per track, in the order
    /// its own menu lists them, since that order is what [`MusicChoice::Track`] indexes
    pub fn with_game_music_choice(
        mut self,
        intro: Option<&'static [u8]>,
        repeating: &'static [u8],
    ) -> Result<Self, String> {
        self.game_music.push(
            match intro {
                Some(intro) => StructuredMusic::new(intro, repeating)?,
                None => StructuredMusic::repeat(repeating)?,
            }
            .into_rc(),
        );
        Ok(self)
    }

    /// how many tracks a match may be played on
    pub fn game_music_count(&self) -> usize {
        self.game_music.len()
    }

    /// Picks the track the next match is played on. A pinned track that this theme does not
    /// have is taken as no answer at all rather than as silence, so a saved choice outliving
    /// the track it named still plays something.
    pub fn choose_game_music(&self, choice: MusicChoice, rng: &mut impl Rng) {
        let index = match choice {
            MusicChoice::Track(track) if track < self.game_music.len() => track,
            _ if self.game_music.len() > 1 => rng.random_range(0..self.game_music.len()),
            _ => 0,
        };
        self.game_music_index.set(index);
    }

    pub fn with_game_over_music<R: Into<Option<&'static [u8]>>>(
        mut self,
        music: &'static [u8],
        repeating: R,
    ) -> Result<Self, String> {
        self.game_over_music = Some(Self::music(music, repeating)?);
        Ok(self)
    }

    pub fn with_next_stage_music<R: Into<Option<&'static [u8]>>>(
        mut self,
        music: &'static [u8],
        repeating: R,
    ) -> Result<Self, String> {
        self.next_stage_music = Some(Self::music(music, repeating)?);
        Ok(self)
    }

    pub fn with_victory_music<R: Into<Option<&'static [u8]>>>(
        mut self,
        music: &'static [u8],
        repeating: R,
    ) -> Result<Self, String> {
        self.victory_music = Some(Self::music(music, repeating)?);
        Ok(self)
    }

    /// one track that loops from the start
    pub fn with_looping_victory_music(mut self, music: &'static [u8]) -> Result<Self, String> {
        self.victory_music = Some(StructuredMusic::repeat(music)?.into_rc());
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
            audio = audio.with_game_music_choice(None, bytes).unwrap();
        }
        audio
    }

    /// over enough matches a soundtrack is dealt all the way through, rather than the same
    /// track every time
    #[test]
    fn a_random_choice_deals_every_track_and_only_those() {
        let audio = theme(4);
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let mut seen = vec![false; 4];
        for _ in 0..100 {
            audio.choose_game_music(MusicChoice::Random, &mut rng);
            seen[audio.game_music_index.get()] = true;
        }
        assert!(seen.into_iter().all(|s| s));
    }

    #[test]
    fn a_pinned_track_is_the_one_that_plays() {
        let audio = theme(4);
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        for track in 0..audio.game_music_count() {
            audio.choose_game_music(MusicChoice::Track(track), &mut rng);
            assert_eq!(audio.game_music_index.get(), track);
        }
    }

    /// a choice this theme cannot honour - a track a theme with fewer of them does not have -
    /// is taken as no answer rather than as silence
    #[test]
    fn a_track_the_theme_does_not_have_is_dealt_one_it_does() {
        let audio = theme(2);
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        audio.choose_game_music(MusicChoice::Track(7), &mut rng);
        assert!(audio.game_music_index.get() < 2);
    }

    /// the games with a single tune ask for nothing and get it
    #[test]
    fn one_track_is_played_whatever_is_asked_for() {
        let audio = theme(1);
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        for choice in [MusicChoice::Random, MusicChoice::Track(3)] {
            audio.choose_game_music(choice, &mut rng);
            assert_eq!(audio.game_music_index.get(), 0);
        }
    }
}
