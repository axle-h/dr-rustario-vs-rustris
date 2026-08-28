use crate::game::geometry::Point as CellPoint;
use crate::game::GameEvent;
use crate::particles::prescribed::{
    PlayerParticleTarget, PlayerTargetedParticles, PrescribedParticles,
};
use crate::render::helper::TextureFactory;
use crate::scale::Scale;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum::RGBA8888;
use sdl2::rect::Rect;
use sdl2::render::{ScaleMode as TextureScaleMode, Texture, TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::time::Duration;

/// How a particle scene shows cleared cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClearParticles {
    /// particles in the shape of each cleared cell, sweeping along them
    Masked { fade_in: Duration },
    /// particles over the whole cleared rows, cascading in, growing then bursting
    Rows { fade_in: Duration },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneType {
    Particles {
        base_color: Color,
        clear: ClearParticles,
    },
    /// a flat colour
    Solid(Color),
    Checkerboard {
        width: u32,
        height: u32,
        colors: [Color; 2],
    },
    Tile {
        texture: &'static [u8],
    },
    /// One picture, scaled until it covers the window and centred on it - what a theme whose
    /// backdrop is a painting rather than a tile wants. It is drawn with linear filtering
    /// and nothing else here is: a tiled retro backdrop scales by whole pixels and has to
    /// keep its hard edges, and a painted one has none to keep.
    Cover {
        texture: &'static [u8],
    },
}

impl SceneType {
    pub fn build<'a>(
        &self,
        canvas: &mut WindowCanvas,
        texture_creator: &'a TextureCreator<WindowContext>,
    ) -> Result<SceneRender<'a>, String> {
        SceneRender::new(canvas, texture_creator, self.clone())
    }
}

pub struct SceneRender<'a> {
    scene_type: SceneType,
    texture: Texture<'a>,
    rect_0: Rect,
}

impl<'a> SceneRender<'a> {
    pub fn new(
        canvas: &mut WindowCanvas,
        texture_creator: &'a TextureCreator<WindowContext>,
        scene_type: SceneType,
    ) -> Result<Self, String> {
        let texture = match scene_type {
            SceneType::Checkerboard {
                width,
                height,
                colors: [color1, color2],
            } => {
                let mut texture = texture_creator
                    .create_texture_target(RGBA8888, width * 2, height * 2)
                    .map_err(|e| e.to_string())?;
                canvas
                    .with_texture_canvas(&mut texture, |c| {
                        c.set_draw_color(Color::RGBA(0, 0, 0, 0));
                        c.clear();

                        c.set_draw_color(color1);
                        c.fill_rects(&[
                            Rect::new(0, 0, width, height),
                            Rect::new(width as i32, height as i32, width, height),
                        ])
                        .unwrap();
                        c.set_draw_color(color2);
                        c.fill_rects(&[
                            Rect::new(width as i32, 0, width, height),
                            Rect::new(0, height as i32, width, height),
                        ])
                        .unwrap();
                    })
                    .map_err(|e| e.to_string())?;
                texture
            }
            SceneType::Tile { texture } => texture_creator.load_texture_bytes(texture)?,
            SceneType::Cover { texture } => {
                let mut texture = texture_creator.load_texture_bytes(texture)?;
                texture.set_scale_mode(TextureScaleMode::Linear);
                texture
            }
            SceneType::Solid(_) => texture_creator
                .create_texture_target(None, 1, 1)
                .map_err(|e| e.to_string())?,
            // TODO this is dirty
            SceneType::Particles { .. } => texture_creator
                .create_texture_target(None, 1, 1)
                .map_err(|e| e.to_string())?,
        };

        let query = texture.query();
        Ok(Self {
            scene_type,
            texture,
            rect_0: Rect::new(0, 0, query.width, query.height),
        })
    }

    pub fn is_particles(&self) -> bool {
        matches!(self.scene_type, SceneType::Particles { .. })
    }

    /// `spawn_cells` is where a new piece appears, for the `Spawned` burst
    pub fn emit_particles(
        &self,
        player: u32,
        event: &GameEvent,
        spawn_cells: &[CellPoint],
    ) -> Option<PlayerTargetedParticles> {
        let SceneType::Particles { base_color, clear } = self.scene_type else {
            return None;
        };
        let points = |cells: &[(CellPoint, crate::game::CellId)]| {
            PlayerParticleTarget::Cells(cells.iter().map(|(p, _)| *p).collect())
        };
        match event {
            GameEvent::Spawned => {
                let target = PlayerParticleTarget::Cells(spawn_cells.to_vec());
                let particles = PrescribedParticles::LightBurstUpAndOut { color: base_color };
                Some(particles.into_targeted(player, target))
            }
            GameEvent::HardDrop { cells, .. } => {
                let particles = PrescribedParticles::BurstUp { color: base_color };
                Some(particles.into_targeted(player, points(cells)))
            }
            GameEvent::AttackSent(_) => Some(
                PrescribedParticles::PerimeterBurst { color: base_color }
                    .into_targeted(player, PlayerParticleTarget::Board),
            ),
            GameEvent::Lock { cells, dropped } if *dropped => {
                let particles = PrescribedParticles::BurstDown { color: base_color };
                Some(particles.into_targeted(player, points(cells)))
            }
            GameEvent::AttackReceived { cells } => {
                let particles = PrescribedParticles::BurstDown { color: base_color };
                Some(particles.into_targeted(player, points(cells)))
            }
            GameEvent::Clear { cells, .. } => {
                let (target, fade_in) = match clear {
                    ClearParticles::Masked { fade_in } => {
                        (PlayerParticleTarget::MaskedCells(cells.clone()), fade_in)
                    }
                    ClearParticles::Rows { fade_in } => {
                        let mut rows = cells.iter().map(|(p, _)| p.y as u32).collect::<Vec<u32>>();
                        rows.sort();
                        rows.dedup();
                        (PlayerParticleTarget::Rows(rows), fade_in)
                    }
                };
                let particles = PrescribedParticles::FadeInLatticeBurstAndFall {
                    fade_in,
                    color: base_color,
                };
                Some(particles.into_targeted(player, target))
            }
            GameEvent::Victory => Some(
                PrescribedParticles::PerimeterSpray { color: base_color }
                    .into_targeted(player, PlayerParticleTarget::Board),
            ),
            GameEvent::StageComplete => Some(
                PrescribedParticles::PerimeterBurst { color: base_color }
                    .into_targeted(player, PlayerParticleTarget::Board),
            ),
            _ => None,
        }
    }

    pub fn draw(&self, canvas: &mut WindowCanvas, scale: &Scale) -> Result<(), String> {
        if self.is_particles() {
            return Ok(());
        }
        if let SceneType::Solid(color) = self.scene_type {
            let (window_width, window_height) = scale.window_size();
            canvas.set_draw_color(color);
            return canvas.fill_rect(Rect::new(0, 0, window_width, window_height));
        }
        if let SceneType::Cover { .. } = self.scene_type {
            let (window_width, window_height) = scale.window_size();
            return canvas.copy(
                &self.texture,
                None,
                cover(self.rect_0, window_width, window_height),
            );
        }

        let (window_width, window_height) = scale.window_size();
        let mut rect = scale.scale_rect(self.rect_0);

        let (offset_x, repeat_x) = Self::offset_and_repeat(window_width, rect.width());
        let (offset_y, repeat_y) = Self::offset_and_repeat(window_height, rect.height());

        rect.offset(offset_x, offset_y);
        for x in 0..repeat_x {
            for y in 0..repeat_y {
                rect.reposition((
                    x * rect.width() as i32 + offset_x,
                    y * rect.height() as i32 + offset_y,
                ));
                canvas.copy(&self.texture, None, rect)?;
            }
        }
        Ok(())
    }

    fn offset_and_repeat(window_size: u32, tile_size: u32) -> (i32, i32) {
        let remainder = window_size as i32 % tile_size as i32;
        let offset = -remainder / 2;
        let repeat = window_size as i32 / tile_size as i32 + if remainder == 0 { 0 } else { 2 };
        (offset, repeat)
    }
}

/// `source` blown up until it covers `width` by `height`, centred on it - the larger of the
/// two ratios, so the short side fills and the long one hangs over the edge equally at both
/// ends. A window narrower or shorter than the picture crops it rather than letterboxing.
fn cover(source: Rect, width: u32, height: u32) -> Rect {
    let factor = (width as f64 / source.width() as f64).max(height as f64 / source.height() as f64);
    let scaled = (
        (source.width() as f64 * factor).round() as u32,
        (source.height() as f64 * factor).round() as u32,
    );
    Rect::new(
        (width as i32 - scaled.0 as i32) / 2,
        (height as i32 - scaled.1 as i32) / 2,
        scaled.0,
        scaled.1,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// a 5:3 picture on a 16:9 window fills the width and hangs over the top and bottom
    #[test]
    fn a_wider_window_covers_by_the_width() {
        assert_eq!(
            cover(Rect::new(0, 0, 400, 240), 1920, 1080),
            Rect::new(0, -36, 1920, 1152)
        );
    }

    /// ... and a taller one the other way about, with the overhang split evenly
    #[test]
    fn a_taller_window_covers_by_the_height() {
        assert_eq!(
            cover(Rect::new(0, 0, 400, 240), 480, 640),
            Rect::new(-293, 0, 1067, 640)
        );
    }
}
