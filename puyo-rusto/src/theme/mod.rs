//! Puyo Rusto's themes: data handed to the engine's theme builders.

pub mod data;
pub mod modern;

use crate::game::cell::{PuyoColor, PuyoPiece};
use engine::config::Config;
use engine::game::PieceId;
use engine::particles::prescribed::RaceTheme;
use engine::render::layout::reference_block_size;
use engine::render::Theme;
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;

/// the source block size every theme's race sprites are scaled relative to
pub const RACE_REFERENCE_BLOCK_SIZE: u32 = modern::SRC_BLOCK_SIZE;

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

/// the themes' contributions to the title screen piece race
///
/// One pair per colour rather than all twenty five: the race wants a handful of recognisable
/// shapes going past, not every combination of two.
pub fn race_themes(themes: &[Theme]) -> Vec<RaceTheme> {
    let pieces = PuyoColor::ALL
        .into_iter()
        .map(|color| PieceId::from(PuyoPiece::new(color, color)))
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
