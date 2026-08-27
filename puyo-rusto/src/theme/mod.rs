//! Puyo Rusto's themes: data handed to the engine's theme builders.

pub mod data;
pub mod modern;

use crate::game::cell::{PuyoColor, PuyoPiece, PuyoSkin};
use crate::game::rules::GameMusic;
use engine::config::Config;
use engine::game::PieceId;
use engine::menu::sound::{MenuMusic, MenuSounds};
use engine::particles::prescribed::RaceTheme;
use engine::render::layout::reference_block_size;
use engine::render::Theme;
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;

/// the source block size every theme's race sprites are scaled relative to
pub const RACE_REFERENCE_BLOCK_SIZE: u32 = modern::SRC_BLOCK_SIZE;

/// What the menus and a match are played over: the music cut out of a Puyo Puyo Tetris rip
/// by `puyo-rusto/art/music.py`, and the two menu clicks cut out of a Puyo Puyo Tetris 2 one
/// by `puyo-rusto/art/sfx.py`.
///
/// Every track is a *pair*: the mixer has no loop marker, so one is split at the point it
/// loops back to and the second half is what repeats. All of it sits here rather than in the
/// particle theme's own directory because phase 3's retro themes are the same game's music
/// and walk the same menus, so they will want the same five and the same two clicks.
mod sound {
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

/// the tracks a match may be played on, **in [`GameMusic::ALL`] order** - a track's place in
/// this table is the number the menu's choice turns into
pub const GAME_MUSIC: [(&[u8], &[u8]); GameMusic::ALL.len()] = [
    sound::KOROBEINIKI,
    sound::DECISIVE,
    sound::MAGICAL,
    sound::TETRO_MIX,
];

/// Puyo Rusto's own menu sounds: the one menu track over both menu screens, as Rustris does
/// with its, and the same game's two clicks over the top of it. `MenuSound` plays a track
/// that is already playing rather than restarting it, so walking the title screen into the
/// options menu does not interrupt it. Only the high score music stays the engine's, since
/// the rip has nothing that belongs under a table of names.
pub const MENU_SOUNDS: MenuSounds = MenuSounds {
    chime: sound::CHIME,
    select: Some(sound::SELECT),
    title: MenuMusic::IntroLoop(sound::MENU.0, sound::MENU.1),
    menu: MenuMusic::IntroLoop(sound::MENU.0, sound::MENU.1),
    high_score: MenuSounds::MODERN.high_score,
};

/// every theme, in the order a theme sprint plays them
///
/// Phase 3 of the plan adds the three retro themes; until they exist the particle theme has
/// nothing to size itself against, so it is built once to measure and once to keep. The
/// other two games take their reference off their retro themes, which are built first and
/// render their art at a fixed size - see [`reference_block_size`].
pub fn all_themes<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
) -> Result<Vec<Theme<'a>>, String> {
    let provisional =
        modern::modern_puyo_theme(canvas, texture_creator, config, modern::SRC_BLOCK_SIZE)?;
    let block_size = reference_block_size(&[&provisional], canvas.window().size(), config.video);
    drop(provisional);
    Ok(vec![modern::modern_puyo_theme(
        canvas,
        texture_creator,
        config,
        block_size,
    )?])
}

/// one pair per colour of every set of puyos there is, which is what the race sends past
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
///
/// One pair per colour rather than all twenty five: the race wants a handful of recognisable
/// shapes going past, not every combination of two. Every *skin* though - the race is not a
/// board and owes no player consistency, so it is the one place all fifteen sets of puyos go
/// by together, which is the whole sheet on show before a match picks two out of it.
pub fn race_themes(themes: &[Theme]) -> Vec<RaceTheme> {
    let pieces = race_pieces();
    themes
        .iter()
        .enumerate()
        .map(|(index, theme)| {
            // every theme's sprites are drawn at the same size in the race, whatever cell
            // size the theme itself was built at
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

    /// `puyo-rusto/art/sfx.py` writes the two clicks and nothing else reads them until a
    /// menu is opened, which no test opens - so this is where a rip that came out at the
    /// wrong rate or was never re-cut is caught
    #[test]
    fn the_menus_two_clicks_decode() {
        for bytes in [sound::CHIME, sound::SELECT] {
            Sound::load(bytes, 100).expect("a menu click did not decode");
        }
    }

    /// the race is where the whole sheet is on show, so every set of puyos has to be in it -
    /// and every piece it offers has to be one the theme's preview sheet keys, or it goes
    /// past as nothing at all
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

    /// the menu picks a track by its place in [`GameMusic::ALL`] and the theme plays the
    /// track at that place in [`GAME_MUSIC`], so the two lists have to be the same length -
    /// which the array's own type says, leaving only the order to check by eye
    #[test]
    fn every_track_the_menu_offers_is_a_track_the_theme_has() {
        assert_eq!(GAME_MUSIC.len(), GameMusic::ALL.len());
        for (intro, repeat) in GAME_MUSIC {
            assert!(!intro.is_empty() && !repeat.is_empty());
        }
    }
}
