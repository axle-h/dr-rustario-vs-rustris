//! Puyo Rusto's themes: data handed to the engine's theme builders.
//!
//! Cell size is the largest all of a game's themes can hold, so each theme's panel dimensions
//! size the board for the rest.

pub mod data;
pub mod genesis;
pub mod modern;
pub mod snes;

use crate::game::cell::{PuyoColor, PuyoPiece, PuyoSkin};
use crate::theme::data::MusicTrack;
use engine::config::Config;
use engine::game::PieceId;
use engine::menu::sound::{MenuMusic, MenuSounds};
use engine::particles::prescribed::RaceTheme;
use engine::render::layout::reference_block_size;
use engine::render::{Theme, ThemeProgress};
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;

/// the source block size every theme's race sprites are scaled relative to
pub const RACE_REFERENCE_BLOCK_SIZE: u32 = modern::SRC_BLOCK_SIZE;

/// The particle theme's and the menus' sound, cut from Puyo Puyo Tetris rips by
/// `puyo-rusto/art/sfx.py` and `art/music.py`.
///
/// Every track is a pair split at its loop point, since the mixer has no loop marker.
pub(crate) mod sound {
    pub const ATTACK: &[u8] = include_bytes!("sfx/attack.ogg");
    pub const GAME_OVER: &[u8] = include_bytes!("sfx/game-over.ogg");
    pub const GARBAGE: &[u8] = include_bytes!("sfx/garbage.ogg");
    pub const HARD_DROP: &[u8] = include_bytes!("sfx/hard-drop.ogg");
    pub const LOCK: &[u8] = include_bytes!("sfx/lock.ogg");
    pub const MOVE: &[u8] = include_bytes!("sfx/move.ogg");
    pub const PAUSE: &[u8] = include_bytes!("sfx/pause.ogg");
    pub const POP: [&[u8]; super::data::CLEAR_CLASSES] = [
        include_bytes!("sfx/pop-1.ogg"),
        include_bytes!("sfx/pop-2.ogg"),
        include_bytes!("sfx/pop-3.ogg"),
        include_bytes!("sfx/pop-4.ogg"),
    ];
    pub const ROTATE: &[u8] = include_bytes!("sfx/rotate.ogg");
    pub const SETTLE: &[u8] = include_bytes!("sfx/settle.ogg");
    pub const SPEED_UP: &[u8] = include_bytes!("sfx/speed-up.ogg");
    pub const VICTORY: &[u8] = include_bytes!("sfx/victory.ogg");

    pub const CHIME: &[u8] = include_bytes!("menu/chime.ogg");
    pub const SELECT: &[u8] = include_bytes!("menu/select.ogg");
    pub const MENU: (&[u8], &[u8]) = (
        include_bytes!("menu/menu-intro.ogg"),
        include_bytes!("menu/menu-repeat.ogg"),
    );
    pub const KOROBEINIKI: (&[u8], &[u8]) = (
        include_bytes!("music/korobeiniki-intro.ogg"),
        include_bytes!("music/korobeiniki-repeat.ogg"),
    );
    pub const DECISIVE: (&[u8], &[u8]) = (
        include_bytes!("music/decisive-battle-intro.ogg"),
        include_bytes!("music/decisive-battle-repeat.ogg"),
    );
    pub const MAGICAL: (&[u8], &[u8]) = (
        include_bytes!("music/magical-confrontation-intro.ogg"),
        include_bytes!("music/magical-confrontation-repeat.ogg"),
    );
    pub const TETRO_MIX: (&[u8], &[u8]) = (
        include_bytes!("music/tetro-mix-intro.ogg"),
        include_bytes!("music/tetro-mix-repeat.ogg"),
    );
}

/// The tracks a match on the particle theme may be dealt, one per match.
pub const GAME_MUSIC: [MusicTrack; 4] = [
    (Some(sound::KOROBEINIKI.0), sound::KOROBEINIKI.1),
    (Some(sound::DECISIVE.0), sound::DECISIVE.1),
    (Some(sound::MAGICAL.0), sound::MAGICAL.1),
    (Some(sound::TETRO_MIX.0), sound::TETRO_MIX.1),
];

/// Puyo Rusto's menu sounds: one track over both menu screens, which carries on unbroken
/// between them, and the engine's high score music.
pub const MENU_SOUNDS: MenuSounds = MenuSounds {
    chime: sound::CHIME,
    select: Some(sound::SELECT),
    title: MenuMusic::IntroLoop(sound::MENU.0, sound::MENU.1),
    menu: MenuMusic::IntroLoop(sound::MENU.0, sound::MENU.1),
    high_score: MenuSounds::MODERN.high_score,
    gain: data::MENU_GAIN,
};

/// Every theme, oldest hardware first and the particle theme last. The particle theme is
/// built last because it is sized against the retro themes' fixed art.
pub fn all_themes<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
) -> Result<Vec<Theme<'a>>, String> {
    all_themes_with_progress(canvas, texture_creator, config, &mut |_| Ok(()))
}

/// [`all_themes`], reporting each one to the loading bar as it is built
pub fn all_themes_with_progress<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
    built: &mut ThemeProgress,
) -> Result<Vec<Theme<'a>>, String> {
    let genesis = genesis::genesis_theme(canvas, texture_creator, config)?;
    built(canvas)?;
    let snes = snes::snes_theme(canvas, texture_creator, config)?;
    built(canvas)?;
    let block_size = reference_block_size(&[&genesis, &snes], canvas.window().size(), config.video);
    let modern = modern::modern_puyo_theme(canvas, texture_creator, config, block_size)?;
    built(canvas)?;
    Ok(vec![genesis, snes, modern])
}

/// one single-colour pair per colour of every skin
fn race_pieces() -> Vec<PieceId> {
    PuyoSkin::all()
        .flat_map(|skin| {
            PuyoColor::ALL
                .into_iter()
                .map(move |color| PuyoPiece::new(color, color).id(skin))
        })
        .collect()
}

/// the themes' contributions to the title screen piece race
pub fn race_themes(themes: &[Theme]) -> Vec<RaceTheme> {
    let pieces = race_pieces();
    themes
        .iter()
        .enumerate()
        .map(|(index, theme)| {
            // every theme races at the same size, whatever cell size it was built at
            let scale =
                RACE_REFERENCE_BLOCK_SIZE as f64 / theme.sprites().block_size() as f64 / 2.0;
            theme.race_theme(index, pieces.clone(), scale)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::audio::Sound;
    use engine::render::sprite_sheet::PreviewData;
    use std::collections::HashSet;

    /// the two menu clicks decode
    #[test]
    fn the_menus_two_clicks_decode() {
        for bytes in [sound::CHIME, sound::SELECT] {
            Sound::load(bytes, 100).expect("a menu click did not decode");
        }
    }

    /// the race carries every skin, and every piece it sends is keyed in the previews
    #[test]
    fn the_race_sends_every_set_of_puyos_past() {
        let pieces = race_pieces();
        assert_eq!(pieces.len(), PuyoSkin::COUNT * PuyoColor::N);
        let skins: HashSet<PuyoSkin> = pieces.iter().map(|p| PuyoSkin::from(*p)).collect();
        assert_eq!(
            skins.len(),
            PuyoSkin::COUNT,
            "a set is missing from the race"
        );

        let PreviewData::Compose { pieces: keyed } = data::previews() else {
            panic!("the previews are composed from the cells");
        };
        let keyed: HashSet<PieceId> = keyed.into_iter().map(|(piece, _)| piece).collect();
        for piece in pieces {
            assert!(keyed.contains(&piece), "{piece:?} is not on the sheet");
        }
    }

    /// No track or lead-in any theme deals is an empty file.
    #[test]
    fn every_track_a_theme_deals_has_something_in_it() {
        let tracks = GAME_MUSIC
            .into_iter()
            .chain(genesis::GAME_MUSIC)
            .chain(snes::GAME_MUSIC);
        for (intro, repeat) in tracks {
            assert!(intro.is_none_or(|intro| !intro.is_empty()));
            assert!(!repeat.is_empty());
        }
    }
}
