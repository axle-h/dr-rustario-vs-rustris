//! Puyo Rusto's themes: data handed to the engine's theme builders.

pub mod data;
pub mod genesis;
pub mod modern;
pub mod snes;
pub mod three_ds;

use crate::game::cell::{PuyoColor, PuyoPiece, PuyoSkin};
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

/// Everything Puyo Rusto is played over: the sound effects and music cut out of Puyo Puyo
/// Tetris rips by `puyo-rusto/art/sfx.py` and `art/music.py`, and the two menu clicks with
/// them.
///
/// All of it sits here rather than in the particle theme's own directory because it belongs to
/// the *game* and not to that theme - a Mean Bean Machine bean settling and a Puyo Puyo Tetris
/// puyo settling are the same event - so the three retro themes play these until phase 3d cuts
/// each of them a rip of its own. `include_bytes!` of one path embeds one copy however many
/// modules name it, so a theme borrowing the set costs nothing but the wrong period sound.
///
/// Every track is a *pair*: the mixer has no loop marker, so one is split at the point it
/// loops back to and the second half is what repeats.
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

/// How many tracks a theme offers a match, which is the same number for every one of them.
///
/// Nothing lets a player pick between them - the theme is dealt one at the start of a match
/// and plays it to the end - so this is not a menu row's length any more. It is still one
/// number rather than each theme's own, because a theme with a *shorter* soundtrack would be
/// heard less often than the others rather than differently, and because a retro theme cutting
/// its own has four to find: the games these are drawn from all wrote four.
pub const GAME_MUSIC_TRACKS: usize = 4;

/// the tracks a match on the particle theme may be dealt
pub const GAME_MUSIC: [(&[u8], &[u8]); GAME_MUSIC_TRACKS] = [
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

/// every theme, in the order a theme sprint plays them: oldest hardware first, with the
/// particle theme last, the way the other two games order theirs
///
/// The retro themes are built first and the particle theme is sized against them, since a
/// retro theme renders its art at a fixed size and the particle one does not. Until phase 3
/// there were no retro themes to measure, so the particle theme was built once to measure
/// and once to keep - that is gone, and this reads like `dr-rustario` and `rustris` now.
pub fn all_themes<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
) -> Result<Vec<Theme<'a>>, String> {
    all_themes_with_progress(canvas, texture_creator, config, &mut |_| Ok(()))
}

/// ... reporting each one as it is built, which is what the loading bar counts
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
    let three_ds = three_ds::three_ds_theme(canvas, texture_creator, config)?;
    built(canvas)?;
    let block_size = reference_block_size(
        &[&genesis, &snes, &three_ds],
        canvas.window().size(),
        config.video,
    );
    let modern = modern::modern_puyo_theme(canvas, texture_creator, config, block_size)?;
    built(canvas)?;
    Ok(vec![genesis, snes, three_ds, modern])
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

    /// every theme is dealt out of a table of [`GAME_MUSIC_TRACKS`], which the arrays' own
    /// types say - so what is left to check is that none of them is empty, which an
    /// `include_bytes!` of a file the rip never wrote would be
    #[test]
    fn every_track_a_theme_deals_has_something_in_it() {
        for (intro, repeat) in GAME_MUSIC.into_iter().chain(genesis::GAME_MUSIC) {
            assert!(!intro.is_empty() && !repeat.is_empty());
        }
    }
}
