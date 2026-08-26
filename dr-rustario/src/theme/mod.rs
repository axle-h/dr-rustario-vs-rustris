//! Dr. Rustario's themes: data handed to the engine's theme builders.

pub mod data;
pub mod modern;
pub mod n64;
pub mod nes;
pub mod snes;

use crate::game::pill::PillShape;
use engine::config::Config;
use engine::game::PieceId;
use engine::particles::prescribed::RaceTheme;
use engine::render::layout::reference_block_size;
use engine::render::Theme;
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;

/// every theme, in the order a theme sprint plays them
pub fn all_themes<'a>(
    canvas: &mut WindowCanvas,
    texture_creator: &'a TextureCreator<WindowContext>,
    config: Config,
) -> Result<Vec<Theme<'a>>, String> {
    let mut themes = vec![
        nes::nes_theme(canvas, texture_creator, config)?,
        snes::snes_theme(canvas, texture_creator, config)?,
        n64::n64_theme(canvas, texture_creator, config)?,
    ];
    let block_size = reference_block_size(
        &themes.iter().collect::<Vec<&Theme>>(),
        canvas.window().size(),
        config.video,
    );
    themes.push(modern::modern_dr_theme(
        canvas,
        texture_creator,
        config,
        block_size,
    )?);
    Ok(themes)
}

/// the source block size every theme's race sprites are scaled relative to
pub const RACE_REFERENCE_BLOCK_SIZE: u32 = modern::sprites::SRC_BLOCK_SIZE;

/// the themes' contributions to the title screen piece race
pub fn race_themes(themes: &[Theme]) -> Vec<RaceTheme> {
    let pieces = PillShape::ALL
        .into_iter()
        .map(PieceId::from)
        .collect::<Vec<PieceId>>();
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
