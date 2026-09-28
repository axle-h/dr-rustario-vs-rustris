use crate::audio::theme::{LoadSound, StructuredMusic};
use crate::audio::Sound;
use crate::config::AudioConfig;
use std::rc::Rc;

const CHIME: &[u8] = include_bytes!("modern/chime.ogg");
const SELECT: &[u8] = include_bytes!("modern/select.ogg");
const TITLE: &[u8] = include_bytes!("modern/title.ogg");
const MENU: &[u8] = include_bytes!("modern/menu.ogg");
const HIGH_SCORE_INTRO: &[u8] = include_bytes!("modern/high-score-intro.ogg");
const HIGH_SCORE_REPEAT: &[u8] = include_bytes!("modern/high-score-repeat.ogg");

/// A menu track: looped, or a one-shot intro then a loop.
#[derive(Clone, Copy, Debug)]
pub enum MenuMusic {
    Loop(&'static [u8]),
    IntroLoop(&'static [u8], &'static [u8]),
}

impl MenuMusic {
    fn load(&self) -> Result<Rc<StructuredMusic>, String> {
        Ok(match self {
            MenuMusic::Loop(music) => StructuredMusic::repeat(music)?,
            MenuMusic::IntroLoop(intro, repeat) => StructuredMusic::new(intro, repeat)?,
        }
        .into_rc())
    }
}

/// The sounds of a set of menus.
#[derive(Clone, Copy, Debug)]
pub struct MenuSounds {
    pub chime: &'static [u8],
    /// defaults to the chime
    pub select: Option<&'static [u8]>,
    pub title: MenuMusic,
    pub menu: MenuMusic,
    pub high_score: MenuMusic,
    /// this set's level as a percentage, the menu's half of
    /// [`crate::render::sound::AudioTheme::with_gain`]
    pub gain: i32,
}

impl MenuSounds {
    pub const MODERN: MenuSounds = MenuSounds {
        chime: CHIME,
        select: Some(SELECT),
        title: MenuMusic::Loop(TITLE),
        menu: MenuMusic::Loop(MENU),
        high_score: MenuMusic::IntroLoop(HIGH_SCORE_INTRO, HIGH_SCORE_REPEAT),
        gain: 100,
    };
}

pub struct MenuSound {
    chime: Sound,
    select: Sound,
    menu_music: Rc<StructuredMusic>,
    title_music: Rc<StructuredMusic>,
    high_score_music: Rc<StructuredMusic>,
}

impl MenuSound {
    pub fn new(config: AudioConfig, sounds: MenuSounds) -> Result<Self, String> {
        let music = |track: MenuMusic| -> Result<Rc<StructuredMusic>, String> {
            let track = track.load()?;
            track.set_gain(sounds.gain);
            Ok(track)
        };
        Ok(Self {
            chime: config.load_sound(sounds.chime)?.with_gain(sounds.gain),
            select: config
                .load_sound(sounds.select.unwrap_or(sounds.chime))?
                .with_gain(sounds.gain),
            menu_music: music(sounds.menu)?,
            title_music: music(sounds.title)?,
            high_score_music: music(sounds.high_score)?,
        })
    }

    pub fn play_chime(&self) -> Result<(), String> {
        self.chime.play()
    }

    pub fn play_select(&self) -> Result<(), String> {
        self.select.play()
    }

    pub fn play_title_music(&self) -> Result<(), String> {
        StructuredMusic::play_unless_current(&self.title_music)
    }

    pub fn play_menu_music(&self) -> Result<(), String> {
        StructuredMusic::play_unless_current(&self.menu_music)
    }

    pub fn play_high_score_music(&self) -> Result<(), String> {
        StructuredMusic::play_unless_current(&self.high_score_music)
    }
}
