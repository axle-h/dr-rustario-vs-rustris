//! Silhouettes for the sprite edge morphs.
//!
//! A shape is the *outline* of a sprite: the lattice points where the sprite's mask is set and
//! a neighbour is not. Because only the outline is ever drawn the source theme's art style is
//! irrelevant — an NES tetromino edge and a modern one are the same shape — so a retro-themed
//! player still contributes their game's shapes without dragging their art into the field.
//!
//! Lattices are built lazily, a few sprites per frame, and cached: themes are all constructed
//! at startup and an eager pass would cost a `read_pixels` per sprite per theme at load time,
//! which matters most on wasm.

use crate::config::ParticleDensity;
use crate::particles::geometry::Vec2D;
use crate::render::block_mask::BlockMask;
use crate::render::font::FontRender;
use crate::render::helper::TextureFactory;
use crate::render::sprite_sheet::{FlatSpriteSheet, MascotKind};
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::collections::HashMap;

/// how many sprites are turned into outlines per frame, so a match never hitches on the
/// first frame of a theme
const BUILD_PER_FRAME: usize = 2;
/// an outline with fewer points than this is not a shape, it is a smudge. Retro cells are a
/// handful of source pixels square, so this has to stay low.
const MIN_POINTS: usize = 8;
/// the grid an outline is reduced to when asking whether two shapes are the same
const SIGNATURE_GRID: usize = 8;

/// A sprite's outline, normalised so its longest side is 1.0 and it is centred on the origin.
/// A morph places it with `centre + point * size`.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeShape {
    points: Vec<Vec2D>,
    /// width over height of the source sprite
    aspect: f64,
}

impl EdgeShape {
    fn new(points: Vec<Point>, width: u32, height: u32) -> Self {
        let longest = width.max(height).max(1) as f64;
        let (half_x, half_y) = (width as f64 / 2.0, height as f64 / 2.0);
        Self {
            points: points
                .into_iter()
                .map(|p| {
                    Vec2D::new(
                        (p.x() as f64 - half_x) / longest,
                        (p.y() as f64 - half_y) / longest,
                    )
                })
                .collect(),
            aspect: width as f64 / height.max(1) as f64,
        }
    }

    pub fn points(&self) -> &[Vec2D] {
        &self.points
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn aspect(&self) -> f64 {
        self.aspect
    }

    /// a bar four times as wide as it is tall, for tests
    #[cfg(test)]
    pub fn wide_bar() -> Self {
        Self::new(
            (0..40)
                .flat_map(|i| [Point::new(i, 0), Point::new(i, 9)])
                .chain((0..10).flat_map(|i| [Point::new(0, i), Point::new(39, i)]))
                .collect(),
            40,
            10,
        )
    }

    /// a plain square outline, for tests
    #[cfg(test)]
    pub fn unit_square() -> Self {
        Self::new(
            (0..10)
                .flat_map(|i| {
                    [
                        Point::new(i, 0),
                        Point::new(i, 9),
                        Point::new(0, i),
                        Point::new(9, i),
                    ]
                })
                .collect(),
            10,
            10,
        )
    }

    /// An occupancy grid over the outline's unit box, as bits. Two sprites that differ only in
    /// colour give outlines that are the same to the eye but not to the pixel - the modern
    /// vitamins' anti-aliasing differs between the red, blue and yellow halves - so telling
    /// shapes apart has to be done at the resolution a viewer sees, not the mask's. The box is
    /// the one the shape was normalised into and not the shape's own extents, so that an I
    /// tetromino and an O, whose outlines each fill their own extents completely, do not come
    /// out as the same shape.
    pub fn signature(&self) -> u64 {
        let cell =
            |value: f64| -> usize { ((value + 0.5) * SIGNATURE_GRID as f64).max(0.0) as usize };
        self.points.iter().fold(0u64, |bits, point| {
            let x = cell(point.x()).min(SIGNATURE_GRID - 1);
            let y = cell(point.y()).min(SIGNATURE_GRID - 1);
            bits | 1 << (y * SIGNATURE_GRID + x)
        })
    }

    /// how far the outline reaches from the origin at all, in unit box units: the radius of
    /// the circle it turns inside, which is what a shape that spins has to be placed by
    pub fn radius(&self) -> f64 {
        self.points
            .iter()
            .fold(0.0f64, |radius, p| radius.max(p.magnitude()))
    }

    /// how far the outline reaches from the origin either way, in unit box units. The box is
    /// normalised on the longest side, so the larger of the two is 0.5.
    pub fn extents(&self) -> (f64, f64) {
        self.points.iter().fold((0.0f64, 0.0f64), |(x, y), p| {
            (x.max(p.x().abs()), y.max(p.y().abs()))
        })
    }
}

/// one sprite of one theme, waiting to be outlined
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShapeSource {
    Piece(usize),
    Cell(usize),
    Mascot,
}

impl ShapeSource {
    fn label(&self) -> String {
        match self {
            ShapeSource::Piece(index) => format!("piece {index}"),
            ShapeSource::Cell(index) => format!("cell {index}"),
            ShapeSource::Mascot => "mascot".to_string(),
        }
    }
}

/// what the bank made of one sprite, see [`ShapeBank::audit`]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// outlined and kept
    Used,
    /// the same outline as one already kept: only the outline is ever drawn, so two sprites
    /// that differ merely in colour are one shape
    Duplicate,
    /// too few points to read as anything
    TooFew,
}

impl Verdict {
    pub fn label(&self) -> &'static str {
        match self {
            Verdict::Used => "used",
            Verdict::Duplicate => "duplicate",
            Verdict::TooFew => "too few",
        }
    }
}

/// one sprite, outlined, and what became of it
pub struct ShapeAudit {
    pub label: String,
    pub shape: EdgeShape,
    pub verdict: Verdict,
}

#[derive(Default)]
struct ThemeShapes {
    shapes: Vec<EdgeShape>,
    /// one per held shape, see [`EdgeShape::signature`]
    signatures: Vec<u64>,
    /// what is left to build for this theme
    todo: Vec<ShapeSource>,
}

/// Every outline built so far, keyed by the theme it came from. The renderer fills it; the
/// field reads it.
#[derive(Default)]
pub struct ShapeBank {
    themes: HashMap<usize, ThemeShapes>,
    /// outlines of rendered text, keyed by the string they spell
    text: HashMap<String, EdgeShape>,
    density: ParticleDensity,
}

impl ShapeBank {
    pub fn new(density: ParticleDensity) -> Self {
        Self {
            themes: HashMap::new(),
            text: HashMap::new(),
            density,
        }
    }

    pub fn set_density(&mut self, density: ParticleDensity) {
        if density != self.density {
            self.themes.clear();
            self.text.clear();
            self.density = density;
        }
    }

    /// stand in for the renderer, which needs a canvas and a font to outline a word
    #[cfg(test)]
    pub fn insert_text(&mut self, value: &str, shape: EdgeShape) {
        self.text.insert(value.to_string(), shape);
    }

    pub fn text(&self, value: &str) -> Option<&EdgeShape> {
        self.text.get(value)
    }

    pub fn any_text(&self) -> Vec<&String> {
        self.text.keys().collect()
    }

    /// the outline of one rendered string, cached. Same machinery as a sprite: the text is
    /// drawn into a target texture and the edges of its opaque pixels are the shape.
    pub fn build_text(
        &mut self,
        canvas: &mut WindowCanvas,
        texture_creator: &TextureCreator<WindowContext>,
        font: &FontRender,
        value: &str,
    ) -> Result<(), String> {
        if self.text.contains_key(value) {
            return Ok(());
        }
        let (width, height) = font.string_size(value);
        if width == 0 || height == 0 {
            return Ok(());
        }
        let mut texture = texture_creator.create_texture_target_blended(width, height)?;
        let mut result = Ok(());
        canvas
            .with_texture_canvas(&mut texture, |c| {
                c.set_draw_color(Color::RGBA(0, 0, 0, 0));
                c.clear();
                result = font.render_string(c, Point::new(0, 0), value);
            })
            .map_err(|e| e.to_string())?;
        result?;

        let mask = BlockMask::from_texture(canvas, &mut texture, Rect::new(0, 0, width, height))?;
        // a string is far wider than it is tall, and it is the letters that have to be
        // legible, so it takes more steps across than a piece does
        let steps = (self.density.edge_steps() * 2).max(16);
        let spacing = (mask.width().max(mask.height()) / steps).max(2);
        let points = mask.edges(Point::new(0, 0), spacing);
        self.text
            .insert(value.to_string(), EdgeShape::new(points, width, height));
        Ok(())
    }

    /// the outlines of a theme's sprites, empty until they have been built
    pub fn shapes(&self, theme: usize) -> &[EdgeShape] {
        match self.themes.get(&theme) {
            Some(theme) => &theme.shapes,
            None => &[],
        }
    }

    pub fn has_shapes(&self, theme: usize) -> bool {
        !self.shapes(theme).is_empty()
    }

    /// build a few more of the outlines these themes will want. Called once a frame with the
    /// themes the players are actually on, so nothing is ever built for a theme nobody plays.
    pub fn build(
        &mut self,
        canvas: &mut WindowCanvas,
        sprites: &mut [FlatSpriteSheet],
        themes: &[usize],
    ) -> Result<(), String> {
        let mut budget = BUILD_PER_FRAME;
        for theme in themes {
            if budget == 0 {
                return Ok(());
            }
            let Some(sheet) = sprites.get_mut(*theme) else {
                continue;
            };
            let entry = self.themes.entry(*theme).or_insert_with(|| ThemeShapes {
                todo: Self::sources(sheet),
                ..ThemeShapes::default()
            });
            while budget > 0 {
                let Some(source) = entry.todo.pop() else {
                    break;
                };
                budget -= 1;
                let Some(shape) = Self::build_one(canvas, sheet, source, self.density)? else {
                    continue;
                };
                // only the outline is ever drawn, so two sprites that differ merely in colour
                // contribute the same shape twice. Dr. Rustario's nine pills come down to a
                // couple of distinct capsules that way, which is what stops its mascot and
                // viruses being crowded out of the playlist.
                let signature = shape.signature();
                if shape.len() < MIN_POINTS || entry.signatures.contains(&signature) {
                    continue;
                }
                entry.signatures.push(signature);
                entry.shapes.push(shape);
            }
        }
        Ok(())
    }

    /// Every sprite of one theme outlined, whether the bank would keep it or not, for the
    /// `field_preview sheet` diagnostic: it is the only way to see what the edge detection
    /// made of a sprite that was dropped for being a duplicate or too small.
    pub fn audit(
        canvas: &mut WindowCanvas,
        sheet: &mut FlatSpriteSheet,
        density: ParticleDensity,
    ) -> Result<Vec<ShapeAudit>, String> {
        let mut signatures: Vec<u64> = vec![];
        let mut audited = vec![];
        for source in Self::sources(sheet) {
            let Some(shape) = Self::build_one(canvas, sheet, source, density)? else {
                continue;
            };
            let signature = shape.signature();
            let verdict = if shape.len() < MIN_POINTS {
                Verdict::TooFew
            } else if signatures.contains(&signature) {
                Verdict::Duplicate
            } else {
                signatures.push(signature);
                Verdict::Used
            };
            audited.push(ShapeAudit {
                label: source.label(),
                shape,
                verdict,
            });
        }
        Ok(audited)
    }

    fn sources(sheet: &FlatSpriteSheet) -> Vec<ShapeSource> {
        let mut sources: Vec<ShapeSource> = (0..sheet.previews.pieces().len())
            .map(ShapeSource::Piece)
            .collect();
        sources.extend((0..sheet.idle_cells.len()).map(ShapeSource::Cell));
        if sheet.mascot.is_some() {
            sources.push(ShapeSource::Mascot);
        }
        sources
    }

    fn build_one(
        canvas: &mut WindowCanvas,
        sheet: &mut FlatSpriteSheet,
        source: ShapeSource,
        density: ParticleDensity,
    ) -> Result<Option<EdgeShape>, String> {
        let mask = match source {
            ShapeSource::Piece(index) => {
                let Some(piece) = sheet.previews.pieces().get(index).copied() else {
                    return Ok(None);
                };
                sheet.previews.block_mask(canvas, piece)?
            }
            ShapeSource::Cell(index) => {
                let mut ids = sheet.idle_cells.keys().copied().collect::<Vec<_>>();
                ids.sort();
                let Some(id) = ids.get(index).copied() else {
                    return Ok(None);
                };
                sheet
                    .idle_cells
                    .get_mut(&id)
                    .unwrap()
                    .block_mask(canvas, 0)?
            }
            ShapeSource::Mascot => match sheet.mascot.as_mut() {
                Some(mascot) => mascot.sheet_mut(MascotKind::Idle).block_mask(canvas, 0)?,
                None => return Ok(None),
            },
        };
        // spacing chosen from the sprite's own size so every theme's outline comes out about
        // the same number of points, whatever cell size it was built at. A retro sprite is a
        // few source pixels square, so the floor has to be one: at two, an eight pixel NES
        // cell is sampled four times across and its outline comes out as a scatter of dots
        // rather than a tetromino
        let steps = density.edge_steps().max(4);
        let spacing = (mask.width().max(mask.height()) / steps).max(1);
        let points = mask.edges(Point::new(0, 0), spacing);
        Ok(Some(EdgeShape::new(points, mask.width(), mask.height())))
    }
}
