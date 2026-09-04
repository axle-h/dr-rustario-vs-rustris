//! Rustris's themes: data handed to the engine's theme builders.

pub mod data;
pub mod gb;
pub mod modern;
pub mod nes;
pub mod snes;

use crate::game::tetromino::TetrominoShape;
use crate::game::VISIBLE_BUFFER;
use engine::config::Config;
use engine::game::PieceId;
use engine::menu::sound::{MenuMusic, MenuSounds};
use engine::particles::prescribed::RaceTheme;
use engine::render::layout::reference_block_size;
use engine::render::{Theme, ThemeProgress};
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;

/// every theme, in the order a theme sprint plays them
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
    let mut themes = vec![];
    for build in [gb::game_boy_theme, nes::nes_theme, snes::snes_theme] {
        themes.push(build(canvas, texture_creator, config)?);
        built(canvas)?;
    }
    let block_size = reference_block_size(
        &themes.iter().collect::<Vec<&Theme>>(),
        canvas.window().size(),
        config.video,
    );
    themes.push(modern::modern_rustris_theme(
        canvas,
        texture_creator,
        config,
        block_size,
        VISIBLE_BUFFER,
    )?);
    built(canvas)?;
    Ok(themes)
}

/// the themes' contributions to the title screen piece race
pub fn race_themes(themes: &[Theme]) -> Vec<RaceTheme> {
    let pieces = TetrominoShape::ALL
        .into_iter()
        .map(PieceId::from)
        .collect::<Vec<PieceId>>();
    themes
        .iter()
        .enumerate()
        .map(|(index, theme)| {
            // every theme's sprites are drawn at the same size in the race, whatever cell
            // size the theme itself was built at
            let scale = modern::SRC_BLOCK_SIZE as f64 / theme.sprites().block_size() as f64 / 2.0;
            theme.race_theme(index, pieces.clone(), scale)
        })
        .collect()
}

mod menu_assets {
    pub const CHIME: &[u8] = include_bytes!("menu/chime.ogg");
    pub const MAIN_MENU: &[u8] = include_bytes!("menu/main-menu.ogg");
    pub const HIGH_SCORE: &[u8] = include_bytes!("menu/high-score.ogg");
}

/// Rustris's own menu sounds
pub const MENU_SOUNDS: MenuSounds = MenuSounds {
    chime: menu_assets::CHIME,
    select: None,
    title: MenuMusic::Loop(menu_assets::MAIN_MENU),
    menu: MenuMusic::Loop(menu_assets::MAIN_MENU),
    high_score: MenuMusic::Loop(menu_assets::HIGH_SCORE),
    gain: 100,
};
