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
use sdl2::render::{Texture, TextureCreator, WindowCanvas};
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
