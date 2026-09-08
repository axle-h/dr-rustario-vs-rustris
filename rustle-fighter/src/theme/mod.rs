//! Super Rustle Fighter's themes: data handed to the engine's theme builders.
//!
//! One, `arcade`, and that is a decision recorded in the plan rather than a stage of one: the
//! arcade look is the point of the exercise. It also means this game has no version built on
//! original art the way the other three do - see `docs/next-game-ideas.md`, which carries the
//! standing note that a new game should be playable on its particle theme alone.

pub mod arcade;
pub mod data;

use crate::game::cell::{GemColor, GemPair, Half};
use engine::config::Config;
use engine::game::PieceId;
use engine::menu::sound::{MenuMusic, MenuSounds};
use engine::particles::prescribed::RaceTheme;
use engine::render::{Theme, ThemeProgress};
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;

/// The menus play the arcade's **character select** tune, which is the one piece of its music
/// that is a menu's rather than a match's. The two clicks are the engine's own: the effect
/// rips are the PlayStation port's and none of them is a menu blip.
pub const MENU_SOUNDS: MenuSounds = MenuSounds {
    chime: MenuSounds::MODERN.chime,
    select: MenuSounds::MODERN.select,
    title: MenuMusic::IntroLoop(arcade::MENU_MUSIC.0, arcade::MENU_MUSIC.1),
    menu: MenuMusic::IntroLoop(arcade::MENU_MUSIC.0, arcade::MENU_MUSIC.1),
    high_score: MenuSounds::MODERN.high_score,
    gain: data::ARCADE_GAIN,
};

/// The gems the title screen's sprite race sends past.
///
/// One pair per colour rather than every combination: the race wants a handful of
/// recognisable shapes going by, not all seventy two. A crash gem of each colour goes with
/// them, since a crash gem is the shape this game is recognised by.
fn race_pieces() -> Vec<PieceId> {
    GemColor::ALL
        .into_iter()
        .flat_map(|color| {
            [
                GemPair::new(Half::Plain(color), Half::Plain(color)),
                GemPair::new(Half::Plain(color), Half::Crash(color)),
            ]
        })
        .map(PieceId::from)
        .collect()
}

/// the themes' contributions to the title screen piece race
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

/// the source block size this game's sprites are scaled relative to
pub const RACE_REFERENCE_BLOCK_SIZE: u32 = arcade::SRC_BLOCK_SIZE;

pub fn all_themes<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
) -> Result<Vec<Theme<'a>>, String> {
    Ok(vec![arcade::arcade_theme(canvas, texture_creator, config)?])
}

pub fn all_themes_with_progress<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
    progress: &mut ThemeProgress,
) -> Result<Vec<Theme<'a>>, String> {
    let themes = vec![arcade::arcade_theme(canvas, texture_creator, config)?];
    progress(canvas)?;
    Ok(themes)
}
