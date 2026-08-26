use crate::config::ParticleDensity;
use crate::particles::field::context::SceneContext;
use crate::particles::field::director::Feature;
use crate::particles::field::reaction::{words, FieldEvent};
use crate::particles::field::shapes::ShapeBank;
use crate::particles::field::{FieldBus, ParticleField, SharedBus};
use crate::particles::geometry::RectF;
use crate::particles::meta::ParticleSprite;
use crate::particles::pool::ParticleLink;
use crate::particles::scale::Scale;
use crate::particles::source::ParticleSource;
use crate::particles::Particles;

use crate::font::FontType;
use crate::render::font::FontRender;
use crate::render::sprite_sheet::FlatSpriteSheet;
use crate::render::Theme;

use crate::particles::particle::Particle;
use crate::render::animation::AnimationSpriteSheet;
use crate::render::helper::TextureFactory;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{BlendMode, Texture, TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

const SPRITES: &[u8] = include_bytes!("sprites.png");
const BASE_SCALE: f64 = 0.05;

/// the procedural streak a constellation link is drawn with: a horizontal gradient that falls
/// off at both ends, stretched and rotated to each segment. Soft and thick, and it takes a
/// colour mod like every other sprite - unlike `draw_line`, which is one aliased pixel.
const LINK_TEXTURE_WIDTH: u32 = 64;
const LINK_TEXTURE_HEIGHT: u32 = 8;

pub struct ParticleRender<'a> {
    scale: Scale,
    sprites: Texture<'a>,
    sprite_snips: HashMap<ParticleSprite, Rect>,
    link_texture: Texture<'a>,
    particles: Particles,
    theme_sprites: Vec<FlatSpriteSheet<'a>>,
    /// the outlines the field draws and the events it reacts to
    bus: SharedBus,
    density: ParticleDensity,
    texture_creator: &'a TextureCreator<WindowContext>,
    /// built the first time the field asks for a text morph
    field_font: Option<FontRender<'a>>,
}

/// the em height text morphs are rendered at: big enough that the outline of a letter has
/// some shape to it, small enough to stay a cheap one-off render
const FIELD_FONT_SIZE: u32 = 192;

impl<'a> ParticleRender<'a> {
    pub fn new(
        canvas: &mut WindowCanvas,
        particles: Particles,
        texture_creator: &'a TextureCreator<WindowContext>,
        scale: Scale,
        all_themes: Vec<&Theme<'a>>,
        density: ParticleDensity,
    ) -> Result<Self, String> {
        let mut sprites = texture_creator.load_texture_bytes(SPRITES)?;
        sprites.set_blend_mode(BlendMode::Blend);

        let sprite_snips = ParticleSprite::ATLAS
            .into_iter()
            .filter(|s| s.snip().is_some())
            .map(|s| (s, s.snip().unwrap()))
            .collect();

        let mut theme_sprites = vec![];
        for theme in all_themes {
            theme_sprites.push(theme.sprites().flatten(canvas, texture_creator)?);
        }

        let link_texture = Self::build_link_texture(canvas, texture_creator)?;

        Ok(Self {
            scale,
            particles,
            sprites,
            sprite_snips,
            link_texture,
            theme_sprites,
            bus: Rc::new(RefCell::new(FieldBus {
                shapes: ShapeBank::new(density),
                events: vec![],
            })),
            density,
            texture_creator,
            field_font: None,
        })
    }

    fn build_link_texture(
        canvas: &mut WindowCanvas,
        texture_creator: &'a TextureCreator<WindowContext>,
    ) -> Result<Texture<'a>, String> {
        let mut texture =
            texture_creator.create_texture_target_blended(LINK_TEXTURE_WIDTH, LINK_TEXTURE_HEIGHT)?;
        let mut result = Ok(());
        canvas
            .with_texture_canvas(&mut texture, |c| {
                result = (|| -> Result<(), String> {
                    c.set_draw_color(Color::RGBA(0, 0, 0, 0));
                    c.clear();
                    let half = (LINK_TEXTURE_HEIGHT as f64 - 1.0) / 2.0;
                    for x in 0..LINK_TEXTURE_WIDTH {
                        // fade out toward both ends so a segment has no hard cap
                        let along = x as f64 / (LINK_TEXTURE_WIDTH - 1) as f64;
                        let ends = (along * std::f64::consts::PI).sin();
                        for y in 0..LINK_TEXTURE_HEIGHT {
                            // and out from the centre line, so it is a soft streak
                            let across = 1.0 - (y as f64 - half).abs() / (half + 0.5);
                            let alpha = (255.0 * ends * across.powi(2)).round() as u8;
                            c.set_draw_color(Color::RGBA(255, 255, 255, alpha));
                            c.draw_point(Point::new(x as i32, y as i32))?;
                        }
                    }
                    Ok(())
                })();
            })
            .map_err(|e| e.to_string())?;
        texture.set_blend_mode(BlendMode::Blend);
        result.map(|_| texture)
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.bus.borrow_mut().events.clear();
    }

    pub fn add_source(&mut self, source: Box<dyn ParticleSource>) {
        self.particles.sources.push(source);
    }

    /// hand the background over to a scene field spanning `canvas`
    pub fn add_field(&mut self, canvas: RectF, window_size: (u32, u32)) {
        self.bus.borrow_mut().shapes.set_density(self.density);
        self.particles.add_pool(Box::new(ParticleField::new(
            canvas,
            window_size,
            self.density,
            self.bus.clone(),
        )));
    }

    /// the retained pools, for diagnostics
    pub fn pools(&self) -> &[Box<dyn crate::particles::pool::ParticlePool>] {
        self.particles.pools()
    }

    pub fn has_field(&self) -> bool {
        !self.particles.pools().is_empty()
    }

    /// stage one named feature on the field, whatever it had in mind: see
    /// [`crate::particles::field::ParticleField::force`]
    pub fn force_field_feature(&mut self, feature: Feature) {
        for pool in self.particles.pools_mut() {
            pool.force_feature(feature);
        }
    }

    /// something happened in the match that the field should react to
    pub fn push_field_event(&mut self, event: FieldEvent) {
        self.bus.borrow_mut().events.push(event);
    }

    /// the emitters only: menus have no scene to update against
    pub fn update(&mut self, delta: Duration) {
        self.particles.update(delta, None)
    }

    /// a match frame: build any silhouettes the themes in play still owe, then tick
    /// everything, emitters and field alike
    pub fn update_scene(
        &mut self,
        delta: Duration,
        canvas: &mut WindowCanvas,
        scene: &SceneContext,
    ) -> Result<(), String> {
        let themes = scene.regions().map(|r| r.theme).collect::<Vec<usize>>();
        self.bus
            .borrow_mut()
            .shapes
            .build(canvas, &mut self.theme_sprites, &themes)?;
        self.build_captions(canvas, scene)?;
        self.particles.update(delta, Some(scene));
        Ok(())
    }

    /// one caption outline per frame, so a level change never costs a hitch
    fn build_captions(
        &mut self,
        canvas: &mut WindowCanvas,
        scene: &SceneContext,
    ) -> Result<(), String> {
        // the words the match may call for are outlined along with the captions, so one is
        // ready the moment it is needed rather than a second later
        let wanted = scene
            .captions
            .iter()
            .map(|caption| caption.as_str())
            .chain(words::ALL)
            .find(|caption| self.bus.borrow().shapes.text(caption).is_none());
        let Some(wanted) = wanted.map(|caption| caption.to_string()) else {
            return Ok(());
        };
        if self.field_font.is_none() {
            self.field_font = Some(FontRender::from_font(
                canvas,
                self.texture_creator,
                FontType::Bold,
                FIELD_FONT_SIZE,
                Color::WHITE,
            )?);
        }
        let font = self.field_font.as_ref().unwrap();
        self.bus
            .borrow_mut()
            .shapes
            .build_text(canvas, self.texture_creator, font, &wanted)
    }

    pub fn draw(&mut self, canvas: &mut WindowCanvas) -> Result<(), String> {
        let quad_links = self.density.links_are_quads();
        let Self {
            scale,
            sprites,
            sprite_snips,
            link_texture,
            particles,
            theme_sprites,
            ..
        } = self;

        // the emitted groups: fire and forget bursts, and everything the menus draw
        for particle in particles.group_particles() {
            Self::draw_particle(canvas, scale, sprites, sprite_snips, theme_sprites, particle)?;
        }

        // then each pool, once, clipped to the area it owns. A field spanning two players is
        // drawn once with one clip rather than once per player.
        for pool in particles.pools() {
            let clip = pool.clip().map(|rect| Self::to_render_rect(scale, rect));
            canvas.set_clip_rect(clip);
            if !pool.links().is_empty() && !quad_links {
                // a plain line takes the canvas's blend mode, not a texture's
                canvas.set_blend_mode(BlendMode::Blend);
            }
            for link in pool.links() {
                if quad_links {
                    Self::draw_link(canvas, scale, link_texture, link)?;
                } else {
                    Self::draw_thin_link(canvas, scale, link)?;
                }
            }
            if !pool.links().is_empty() && !quad_links {
                canvas.set_blend_mode(BlendMode::None);
            }
            for particle in pool.particles() {
                Self::draw_particle(canvas, scale, sprites, sprite_snips, theme_sprites, particle)?;
            }
        }
        canvas.set_clip_rect(None);
        Ok(())
    }

    fn to_render_rect(scale: &Scale, rect: RectF) -> Rect {
        let top_left = scale.point_to_render_space((rect.x(), rect.y()));
        let bottom_right = scale.point_to_render_space((rect.right(), rect.bottom()));
        Rect::new(
            top_left.x(),
            top_left.y(),
            (bottom_right.x() - top_left.x()).max(1) as u32,
            (bottom_right.y() - top_left.y()).max(1) as u32,
        )
    }

    /// a rotated textured quad, so a link is a soft glowing streak rather than a hairline.
    /// Deliberately not `render_geometry`: that is SDL 2.0.18, and a PortMaster build links
    /// the handheld firmware's own SDL, where a missing symbol is a binary that will not start
    fn draw_link(
        canvas: &mut WindowCanvas,
        scale: &Scale,
        texture: &mut Texture,
        link: &ParticleLink,
    ) -> Result<(), String> {
        let from = scale.point_to_render_space(link.from);
        let to = scale.point_to_render_space(link.to);
        let dx = (to.x() - from.x()) as f64;
        let dy = (to.y() - from.y()) as f64;
        let length = (dx * dx + dy * dy).sqrt();
        if length < 1.0 {
            return Ok(());
        }
        let (_, window_height) = scale.window_size();
        let thickness = (link.width * window_height as f64).round().max(1.0) as u32;
        let (r, g, b): (u8, u8, u8) = link.color.into();
        texture.set_color_mod(r, g, b);
        texture.set_alpha_mod((255.0 * link.alpha.clamp(0.0, 1.0)).round() as u8);
        let dest = Rect::from_center(
            Point::new((from.x() + to.x()) / 2, (from.y() + to.y()) / 2),
            length.round() as u32,
            thickness,
        );
        canvas.copy_ex(
            texture,
            None,
            dest,
            dy.atan2(dx).to_degrees(),
            None,
            false,
            false,
        )
    }

    /// the low density fallback: `SDL_RenderDrawLine`, one call and one physical pixel
    fn draw_thin_link(
        canvas: &mut WindowCanvas,
        scale: &Scale,
        link: &ParticleLink,
    ) -> Result<(), String> {
        canvas.set_draw_color(link.color.to_sdl(link.alpha));
        canvas.draw_line(
            scale.point_to_render_space(link.from),
            scale.point_to_render_space(link.to),
        )
    }

    fn draw_particle(
        canvas: &mut WindowCanvas,
        scale: &Scale,
        sprites: &mut Texture,
        sprite_snips: &HashMap<ParticleSprite, Rect>,
        theme_sprites: &[FlatSpriteSheet],
        particle: &Particle,
    ) -> Result<(), String> {
        let (r, g, b): (u8, u8, u8) = particle.color().into();
        sprites.set_color_mod(r, g, b);
        if particle.alpha() < 1.0 {
            sprites.set_alpha_mod((255.0 * particle.alpha()).round() as u8);
        } else {
            sprites.set_alpha_mod(255);
        }

        let point = scale.point_to_render_space(particle.position());

        match particle.sprite() {
            ParticleSprite::Piece { theme, piece } => {
                let sprite_sheet = &theme_sprites[theme].previews;
                let snip = sprite_sheet.snip(piece);
                let scale = particle.size();
                let rect = Rect::from_center(
                    point,
                    (scale * snip.width() as f64).round() as u32,
                    (scale * snip.height() as f64).round() as u32,
                );

                if particle.rotation() > 0.0 || particle.rotation() < 0.0 {
                    canvas.copy_ex(
                        sprite_sheet.texture(),
                        snip,
                        rect,
                        particle.rotation(),
                        None,
                        false,
                        false,
                    )?;
                } else {
                    canvas.copy(sprite_sheet.texture(), snip, rect)?;
                }
            }
            ParticleSprite::Cell { theme, cell, .. } => {
                if let Some(sheet) = theme_sprites[theme].idle_cells.get(&cell) {
                    Self::draw_animated_particle(canvas, sheet, particle, point)?;
                }
            }
            ParticleSprite::Mascot { theme, kind, .. } => {
                if let Some(mascot) = theme_sprites[theme].mascot.as_ref() {
                    Self::draw_animated_particle(canvas, mascot.sheet(kind), particle, point)?;
                }
            }
            _ => {
                if let Some(snip) = sprite_snips.get(&particle.sprite()) {
                    let scale = BASE_SCALE * particle.size();
                    let rect = Rect::from_center(
                        point,
                        (scale * snip.width() as f64).round() as u32,
                        (scale * snip.height() as f64).round() as u32,
                    );
                    canvas.copy(sprites, *snip, rect)?;
                } else {
                    unreachable!();
                }
            }
        }
        Ok(())
    }

    fn draw_animated_particle(
        canvas: &mut WindowCanvas,
        sprite_sheet: &AnimationSpriteSheet,
        particle: &Particle,
        dest: Point,
    ) -> Result<(), String> {
        let (width, height) = sprite_sheet.frame_size();
        let scale = particle.size();
        let rect = Rect::from_center(
            dest,
            (scale * width as f64).round() as u32,
            (scale * height as f64).round() as u32,
        );

        let frame = particle.animation_frame();
        if particle.rotation() > 0.0 || particle.rotation() < 0.0 {
            sprite_sheet.draw_frame_ex(canvas, rect, particle.rotation(), frame)
        } else {
            sprite_sheet.draw_frame_scaled(canvas, rect, frame)
        }
    }
}
