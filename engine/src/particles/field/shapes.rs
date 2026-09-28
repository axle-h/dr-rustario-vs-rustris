//! Silhouettes for the sprite edge morphs: a shape is the lattice points where a sprite's mask
//! is set and a neighbour is not, so a theme's art style never reaches the field. Outlines are
//! built lazily a few a frame and cached, since an eager pass costs a `read_pixels` per sprite.

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

const BUILD_PER_FRAME: usize = 2;
/// fewer points than this is a smudge; kept low because retro cells are a few pixels square
const MIN_POINTS: usize = 8;
/// the grid an outline is reduced to when comparing shapes
const SIGNATURE_GRID: usize = 8;

/// A sprite's outline, normalised so its longest side is 1.0 and centred on the origin; a
/// morph places it with `centre + point * size`.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeShape {
    points: Vec<Vec2D>,
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

    /// An occupancy grid over the outline's unit box, as bits, so outlines that differ only in
    /// anti-aliasing compare equal. It covers the normalised box rather than the shape's own
    /// extents, so an I and an O tetromino differ.
    pub fn signature(&self) -> u64 {
        let cell =
            |value: f64| -> usize { ((value + 0.5) * SIGNATURE_GRID as f64).max(0.0) as usize };
        self.points.iter().fold(0u64, |bits, point| {
            let x = cell(point.x()).min(SIGNATURE_GRID - 1);
            let y = cell(point.y()).min(SIGNATURE_GRID - 1);
            bits | 1 << (y * SIGNATURE_GRID + x)
        })
    }

    /// the radius of the circle the outline turns inside, in unit box units
    pub fn radius(&self) -> f64 {
        self.points
            .iter()
            .fold(0.0f64, |radius, p| radius.max(p.magnitude()))
    }

    /// the outline's reach from the origin either way; the larger of the two is 0.5
    pub fn extents(&self) -> (f64, f64) {
        self.points.iter().fold((0.0f64, 0.0f64), |(x, y), p| {
            (x.max(p.x().abs()), y.max(p.y().abs()))
        })
    }
}

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
    Used,
    /// the same outline as one already kept
    Duplicate,
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
    todo: Vec<ShapeSource>,
}

/// Every outline built so far, keyed by theme. The renderer fills it; the field reads it.
#[derive(Default)]
pub struct ShapeBank {
    themes: HashMap<usize, ThemeShapes>,
    /// outlines of rendered text, keyed by the string
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

    /// The cached outline of one rendered string.
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
        let steps = (self.density.edge_steps() * 2).max(16);
        let spacing = (mask.width().max(mask.height()) / steps).max(2);
        let points = mask.edges(Point::new(0, 0), spacing);
        self.text
            .insert(value.to_string(), EdgeShape::new(points, width, height));
        Ok(())
    }

    /// empty until built
    pub fn shapes(&self, theme: usize) -> &[EdgeShape] {
        match self.themes.get(&theme) {
            Some(theme) => &theme.shapes,
            None => &[],
        }
    }

    pub fn has_shapes(&self, theme: usize) -> bool {
        !self.shapes(theme).is_empty()
    }

    /// Build a few more outlines for `themes`, the ones the players are on. Called once a frame.
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
                // only the outline is drawn, so outlines that differ only in colour are kept once
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

    /// Every sprite of one theme outlined, kept or not, for the `field_preview sheet` diagnostic.
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
        // spacing scales with the sprite, so every theme's outline has about the same point count;
        // its floor is one pixel because a retro sprite is only a few pixels square
        let steps = density.edge_steps().max(4);
        let spacing = (mask.width().max(mask.height()) / steps).max(1);
        let points = mask.edges(Point::new(0, 0), spacing);
        Ok(Some(EdgeShape::new(points, mask.width(), mask.height())))
    }
}
