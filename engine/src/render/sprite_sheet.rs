//! Sprites for everything on a board, keyed by the game's own [`CellId`]s and [`PieceId`]s.
//!
//! A theme describes where each cell's sprite is in a source PNG; the sheet rescales them all
//! into one atlas at the theme's block size, pre-rotates cells that need it, keeps alpha
//! variants for ghosts and trails, animated strips for cells that idle or pop, and masks for
//! particle emission.

use crate::animate::destroy::DestroyStyle;
use crate::animate::PlayerAnimations;
use crate::game::geometry::Point as CellPoint;
use crate::game::{Cell, CellId, Game, PieceId, PlacedCell};
use crate::render::animation::{AnimationSpriteSheet, AnimationSpriteSheetData};
use crate::render::block_mask::BlockMask;
use crate::render::geometry::BoardGeometry;
use crate::render::helper::TextureFactory;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::render::{Texture, TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

/// how wide an atlas may get before it wraps onto another row.
///
/// A sheet is one texture with every cell side by side, and a game with a great many cells -
/// Puyo Rusto keys a colour by sixteen link masks for each of fifteen sprite sets - runs off
/// the end of what a driver will allocate in a single dimension. GLES parts commonly stop at
/// 4096, so that is where this stops; laying the surplus on another row costs the same
/// pixels and asks for no more of the driver than any other theme does.
///
/// It is the ceiling on a theme's *source* sheet too, which is loaded whole and in one piece
/// - Puyo Rusto's is laid out against this, and says so.
pub const MAX_ATLAS_WIDTH: u32 = 4096;

/// nothing is faded at all
const OPAQUE: u8 = 0xff;

/// Lays sprites out left to right, onto another shelf once a row is [`MAX_ATLAS_WIDTH`] wide.
///
/// Every shelf is as tall as the tallest sprite, which wastes a little on a sheet of mixed
/// sizes and keeps the arithmetic to one line. Returns where each went and how big the sheet
/// has to be.
fn shelve(sizes: &[(u32, u32)]) -> (Vec<Rect>, u32, u32) {
    let row_height = sizes.iter().map(|(_, h)| *h).max().unwrap_or(1);
    let (mut x, mut y, mut width) = (0, 0, 0);
    let mut rects = Vec::with_capacity(sizes.len());
    for (w, h) in sizes.iter().copied() {
        if x > 0 && x + w > MAX_ATLAS_WIDTH {
            x = 0;
            y += row_height;
        }
        rects.push(Rect::new(x as i32, y as i32, w, h));
        x += w;
        width = width.max(x);
    }
    (rects, width.max(1), (y + row_height).max(1))
}

/// Where a cell's sprite is in the source file.
#[derive(Clone, Copy, Debug)]
pub struct CellSpriteData {
    /// the sprite for the cell while it is part of the active piece (and anywhere else if
    /// `stack` is not given)
    pub snip: Rect,
    /// a different sprite once the cell is locked into the stack
    pub stack: Option<Rect>,
    /// clockwise degrees to rotate the source sprite by
    pub angle: f64,
}

impl CellSpriteData {
    pub fn new(snip: Rect) -> Self {
        Self {
            snip,
            stack: None,
            angle: 0.0,
        }
    }

    pub fn with_stack(self, stack: Rect) -> Self {
        Self {
            stack: Some(stack),
            ..self
        }
    }

    pub fn rotated(self, angle: f64) -> Self {
        Self { angle, ..self }
    }
}

/// Animated strips a group of cells share.
#[derive(Clone, Debug)]
pub struct CellAnimationData {
    /// plays while the cell sits on the board; its first frame replaces the still sprite
    pub idle: Option<AnimationSpriteSheetData>,
    /// plays when the cell is cleared ([`DestroyStyle::Pop`])
    pub pop: Option<AnimationSpriteSheetData>,
}

/// How the queue and hold box show a whole piece.
#[derive(Clone, Debug)]
pub enum PreviewData {
    /// dedicated sprites per piece, every one `size` in the source file; they are scaled by
    /// the same factor as the cells
    Sprites {
        file: &'static [u8],
        pieces: Vec<(PieceId, Rect)>,
        size: (u32, u32),
    },
    /// composed from the piece's cells
    Compose {
        pieces: Vec<(PieceId, Vec<(CellPoint, CellId)>)>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MascotKind {
    Idle,
    Spawn,
    GameOver,
    Victory,
}

#[derive(Clone, Debug)]
pub struct MascotSpriteData {
    pub idle: AnimationSpriteSheetData,
    pub spawn: AnimationSpriteSheetData,
    pub game_over: AnimationSpriteSheetData,
    pub victory: AnimationSpriteSheetData,
    pub scale: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct BlockSpriteSheetData {
    pub file: &'static [u8],
    pub source_block_size: u32,
    pub cells: Vec<(CellId, CellSpriteData)>,
    pub animations: Vec<(Vec<CellId>, CellAnimationData)>,
    pub ghost_alpha: u8,
    pub previews: PreviewData,
    pub mascot: Option<MascotSpriteData>,
}

/// How the landing position of the active piece is shown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GhostStyle {
    /// the piece's own sprites, faded
    Alpha,
    /// an outline around the ghost cells
    Outline {
        color: Color,
    },
    None,
}

#[derive(Clone, Copy, Debug)]
struct CellSnips {
    normal: Rect,
    stack: Rect,
}

struct CellAnimations<'a> {
    idle: Option<AnimationSpriteSheet<'a>>,
    pop: Option<AnimationSpriteSheet<'a>>,
}

pub struct PreviewSpriteSheet<'a> {
    texture: Texture<'a>,
    snips: HashMap<PieceId, Rect>,
}

impl<'a> PreviewSpriteSheet<'a> {
    pub fn texture(&self) -> &Texture<'a> {
        &self.texture
    }

    pub fn snip(&self, piece: PieceId) -> Rect {
        self.snips[&piece]
    }

    pub fn size(&self) -> (u32, u32) {
        let width = self.snips.values().map(|r| r.width()).max().unwrap_or(0);
        let height = self.snips.values().map(|r| r.height()).max().unwrap_or(0);
        (width, height)
    }

    /// every piece this sheet can draw, in a stable order
    pub fn pieces(&self) -> Vec<PieceId> {
        let mut pieces = self.snips.keys().copied().collect::<Vec<PieceId>>();
        pieces.sort();
        pieces
    }

    /// the opaque pixels of one piece, for the particle field's silhouettes
    pub fn block_mask(
        &mut self,
        canvas: &mut WindowCanvas,
        piece: PieceId,
    ) -> Result<BlockMask, String> {
        let snip = self.snip(piece);
        BlockMask::from_texture(canvas, &mut self.texture, snip)
    }

    pub fn draw_piece<A: Into<Option<f64>>, S: Into<Option<f64>>>(
        &self,
        canvas: &mut WindowCanvas,
        piece: PieceId,
        point: Point,
        angle: A,
        scale: S,
    ) -> Result<(), String> {
        let snip = self.snip(piece);
        let mut dest = Rect::new(point.x, point.y, snip.width(), snip.height());
        if let Some(scale) = scale.into() {
            dest.resize(
                (dest.width() as f64 * scale).round() as u32,
                (dest.height() as f64 * scale).round() as u32,
            );
        }
        if let Some(angle) = angle.into() {
            canvas.copy_ex(&self.texture, snip, dest, angle, None, false, false)
        } else {
            canvas.copy(&self.texture, snip, dest)
        }
    }

    pub fn draw_piece_in_center(
        &self,
        canvas: &mut WindowCanvas,
        piece: PieceId,
        center: Point,
    ) -> Result<(), String> {
        let snip = self.snip(piece);
        let dest = Rect::from_center(center, snip.width(), snip.height());
        canvas.copy(&self.texture, snip, dest)
    }

    /// draw the piece as large as fits in `dest` without exceeding `max_scale`
    pub fn draw_piece_fill(
        &self,
        canvas: &mut WindowCanvas,
        piece: PieceId,
        dest: Rect,
        max_scale: f64,
    ) -> Result<(), String> {
        let snip = self.snip(piece);
        let scale_x = dest.width() as f64 / snip.width() as f64;
        let scale_y = dest.height() as f64 / snip.height() as f64;
        let scale = scale_x.min(scale_y).min(max_scale);
        let rect = Rect::from_center(
            dest.center(),
            (scale * snip.width() as f64).round() as u32,
            (scale * snip.height() as f64).round() as u32,
        );
        canvas.copy(&self.texture, snip, rect)
    }

    fn clone<'b>(
        &self,
        canvas: &mut WindowCanvas,
        texture_creator: &'b TextureCreator<WindowContext>,
    ) -> Result<PreviewSpriteSheet<'b>, String> {
        let query = self.texture.query();
        let mut texture = texture_creator
            .create_texture_target_blended(query.width.max(1), query.height.max(1))?;
        canvas
            .with_texture_canvas(&mut texture, |c| {
                c.set_draw_color(Color::RGBA(0, 0, 0, 0));
                c.clear();
                c.copy(&self.texture, None, None).unwrap();
            })
            .map_err(|e| e.to_string())?;
        Ok(PreviewSpriteSheet {
            texture,
            snips: self.snips.clone(),
        })
    }
}

pub struct MascotSprites<'a> {
    idle: AnimationSpriteSheet<'a>,
    spawn: AnimationSpriteSheet<'a>,
    game_over: AnimationSpriteSheet<'a>,
    victory: AnimationSpriteSheet<'a>,
}

impl<'a> MascotSprites<'a> {
    pub fn sheet(&self, kind: MascotKind) -> &AnimationSpriteSheet<'a> {
        match kind {
            MascotKind::Idle => &self.idle,
            MascotKind::Spawn => &self.spawn,
            MascotKind::GameOver => &self.game_over,
            MascotKind::Victory => &self.victory,
        }
    }

    pub fn sheet_mut(&mut self, kind: MascotKind) -> &mut AnimationSpriteSheet<'a> {
        match kind {
            MascotKind::Idle => &mut self.idle,
            MascotKind::Spawn => &mut self.spawn,
            MascotKind::GameOver => &mut self.game_over,
            MascotKind::Victory => &mut self.victory,
        }
    }

    fn clone<'b>(
        &self,
        canvas: &mut WindowCanvas,
        texture_creator: &'b TextureCreator<WindowContext>,
    ) -> Result<MascotSprites<'b>, String> {
        Ok(MascotSprites {
            idle: self.idle.clone(canvas, texture_creator)?,
            spawn: self.spawn.clone(canvas, texture_creator)?,
            game_over: self.game_over.clone(canvas, texture_creator)?,
            victory: self.victory.clone(canvas, texture_creator)?,
        })
    }
}

/// Sprites copied out of a [`BlockSpriteSheet`] for use by the particle renderer, which
/// outlives any single theme.
pub struct FlatSpriteSheet<'a> {
    pub previews: PreviewSpriteSheet<'a>,
    pub idle_cells: HashMap<CellId, AnimationSpriteSheet<'a>>,
    pub mascot: Option<MascotSprites<'a>>,
}

/// SDL packs `ARGB8888` into a 32 bit word and writes it out in the machine's byte order, so
/// the bytes come back reversed on a little endian machine - see [`BlockMask::from_pixels`],
/// which learned this the hard way.
#[cfg(target_endian = "little")]
const CHANNELS: [usize; 4] = [3, 2, 1, 0]; // a, r, g, b
#[cfg(target_endian = "big")]
const CHANNELS: [usize; 4] = [0, 1, 2, 3];

/// a pixel below this alpha is not part of the sprite
const OPAQUE_ENOUGH: u8 = 0x40;

/// The colour each cell is drawn in, read off the built atlas in one pass.
///
/// Pixels are weighted by saturation times brightness, so the outline a sprite is drawn with
/// and the white of a puyo's eyes do not wash the answer out - what comes back is the colour
/// somebody would name if you pointed at the sprite. A sprite with no saturation anywhere
/// (nuisance, a monochrome theme) falls back to the plain average of what is there.
fn cell_colors(
    pixels: &[u8],
    atlas_width: u32,
    cells: &HashMap<CellId, CellSnips>,
) -> HashMap<CellId, Color> {
    let mut colors = HashMap::new();
    for (id, snips) in cells.iter() {
        if let Some(color) = snip_color(pixels, atlas_width, snips.normal) {
            colors.insert(*id, color);
        }
    }
    colors
}

/// every pixel of a texture, `ARGB8888`, in one readback
fn read_texture(
    canvas: &mut WindowCanvas,
    texture: &mut Texture,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    let mut pixels = vec![];
    canvas
        .with_texture_canvas(texture, |c| {
            pixels = c
                .read_pixels(Rect::new(0, 0, width, height), PixelFormatEnum::ARGB8888)
                .unwrap_or_default()
        })
        .map_err(|e| e.to_string())?;
    Ok(pixels)
}

fn snip_color(pixels: &[u8], atlas_width: u32, snip: Rect) -> Option<Color> {
    let [a, r, g, b] = CHANNELS;
    let mut weighted = [0f64; 3];
    let mut plain = [0f64; 3];
    let (mut weight, mut count) = (0f64, 0f64);
    for y in snip.y()..snip.bottom() {
        for x in snip.x()..snip.right() {
            let i = 4 * (y as usize * atlas_width as usize + x as usize);
            let Some(pixel) = pixels.get(i..i + 4) else {
                continue;
            };
            if pixel[a] < OPAQUE_ENOUGH {
                continue;
            }
            let rgb = [pixel[r] as f64, pixel[g] as f64, pixel[b] as f64];
            let high = rgb[0].max(rgb[1]).max(rgb[2]);
            let low = rgb[0].min(rgb[1]).min(rgb[2]);
            let w = (high - low) * high;
            for channel in 0..3 {
                weighted[channel] += rgb[channel] * w;
                plain[channel] += rgb[channel];
            }
            weight += w;
            count += 1.0;
        }
    }
    if count == 0.0 {
        return None;
    }
    let divisor = if weight > 0.0 { weight } else { count };
    let source = if weight > 0.0 { weighted } else { plain };
    Some(Color::RGB(
        (source[0] / divisor).round() as u8,
        (source[1] / divisor).round() as u8,
        (source[2] / divisor).round() as u8,
    ))
}

pub struct BlockSpriteSheet<'a> {
    /// The atlas, every cell of it, drawn from at whatever alpha the caller asks for.
    ///
    /// Fading means [`Texture::set_alpha_mod`], which is a mutation behind a `&self` draw, so
    /// it sits in a `RefCell` the same way the popup font's fill does for its tint. This used
    /// to be a bank of sixty three whole copies of the atlas, one per alpha step, to keep the
    /// draws behind a shared reference; that is a texture the size of every cell a theme has,
    /// sixty three times over, and Puyo Rusto's fifteen sets of puyos would have made it most
    /// of a gigabyte.
    texture: RefCell<Texture<'a>>,
    ghost_alpha_mod: u8,
    cells: HashMap<CellId, CellSnips>,
    animations: Vec<CellAnimations<'a>>,
    animation_index: HashMap<CellId, usize>,
    masks: HashMap<CellId, BlockMask>,
    colors: HashMap<CellId, Color>,
    block_size: u32,
    previews: PreviewSpriteSheet<'a>,
    mascot: Option<MascotSprites<'a>>,
}

impl<'a> BlockSpriteSheet<'a> {
    pub fn new<B: Into<Option<u32>>>(
        canvas: &mut WindowCanvas,
        texture_creator: &'a TextureCreator<WindowContext>,
        data: &BlockSpriteSheetData,
        block_size: B,
    ) -> Result<Self, String> {
        let block_size = block_size.into().unwrap_or(data.source_block_size);
        let sprite_src = texture_creator.load_texture_bytes(data.file)?;

        // the atlas: normal then stack sprite of every cell, along a row and onto the next
        // once the row is as wide as a driver is asked to go
        let per_row = (MAX_ATLAS_WIDTH / block_size.max(1)).max(1);
        let mut cells = HashMap::new();
        let mut slot = 0u32;
        let mut copies: Vec<(Rect, Rect, f64)> = vec![];
        let place = |slot: &mut u32| {
            let rect = Rect::new(
                (*slot % per_row * block_size) as i32,
                (*slot / per_row * block_size) as i32,
                block_size,
                block_size,
            );
            *slot += 1;
            rect
        };
        for (id, sprite) in data.cells.iter() {
            let normal = place(&mut slot);
            copies.push((sprite.snip, normal, sprite.angle));
            let stack = match sprite.stack {
                Some(stack_src) => {
                    let stack = place(&mut slot);
                    copies.push((stack_src, stack, sprite.angle));
                    stack
                }
                None => normal,
            };
            cells.insert(*id, CellSnips { normal, stack });
        }
        let width = (slot.min(per_row) * block_size).max(1);
        let height = (slot.div_ceil(per_row) * block_size).max(1);
        let mut texture = texture_creator.create_texture_target_blended(width, height)?;
        canvas
            .with_texture_canvas(&mut texture, |c| {
                c.set_draw_color(Color::RGBA(0, 0, 0, 0));
                c.clear();
                for (src, dest, angle) in copies.iter().copied() {
                    if angle == 0.0 {
                        c.copy(&sprite_src, src, dest).unwrap();
                    } else {
                        c.copy_ex(&sprite_src, src, dest, angle, None, false, false)
                            .unwrap();
                    }
                }
            })
            .map_err(|e| e.to_string())?;

        let mut animations = vec![];
        let mut animation_index = HashMap::new();
        for (ids, animation) in data.animations.iter() {
            let mut build = |sheet: &Option<AnimationSpriteSheetData>| -> Result<_, String> {
                match sheet {
                    Some(sheet) => Ok(Some(sheet.sprite_sheet(texture_creator)?.scale(
                        canvas,
                        texture_creator,
                        block_size,
                        block_size,
                    )?)),
                    None => Ok(None),
                }
            };
            let idle = build(&animation.idle)?;
            let pop = build(&animation.pop)?;
            for id in ids {
                animation_index.insert(*id, animations.len());
            }
            animations.push(CellAnimations { idle, pop });
        }

        // one readback for the whole atlas, which the masks and the colours are both cut out
        // of. Reading a cell at a time is a pipeline stall per cell, and a theme with fifteen
        // sprite sets has twelve hundred of them
        let pixels = read_texture(canvas, &mut texture, width, height)?;
        let mut masks = HashMap::new();
        for (id, snips) in cells.iter() {
            let mask = match animation_index
                .get(id)
                .and_then(|i| animations[*i].idle.as_mut())
            {
                Some(idle) => idle.block_mask(canvas, 0)?,
                None => BlockMask::from_region(&pixels, width, snips.normal),
            };
            masks.insert(*id, mask);
        }

        let colors = cell_colors(&pixels, width, &cells);

        let previews = match &data.previews {
            PreviewData::Sprites { file, pieces, size } => {
                let src = texture_creator.load_texture_bytes(file)?;
                let scale = block_size as f64 / data.source_block_size as f64;
                let w = (size.0 as f64 * scale).round() as u32;
                let h = (size.1 as f64 * scale).round() as u32;
                let (dests, sheet_width, sheet_height) = shelve(&vec![(w, h); pieces.len()]);
                let mut snips = HashMap::new();
                let mut copies = vec![];
                for ((piece, src_snip), dest) in pieces.iter().zip(dests) {
                    snips.insert(*piece, dest);
                    copies.push((*src_snip, dest));
                }
                let mut preview_texture =
                    texture_creator.create_texture_target_blended(sheet_width, sheet_height)?;
                canvas
                    .with_texture_canvas(&mut preview_texture, |c| {
                        c.set_draw_color(Color::RGBA(0, 0, 0, 0));
                        c.clear();
                        for (src_snip, dest) in copies.iter().copied() {
                            c.copy(&src, src_snip, dest).unwrap();
                        }
                    })
                    .map_err(|e| e.to_string())?;
                PreviewSpriteSheet {
                    texture: preview_texture,
                    snips,
                }
            }
            PreviewData::Compose { pieces } => {
                // where each piece's cells sit relative to its own top left, and how big that
                // makes it
                let bounds = |piece_cells: &Vec<(CellPoint, CellId)>| {
                    let min_x = piece_cells.iter().map(|(p, _)| p.x).min().unwrap_or(0);
                    let min_y = piece_cells.iter().map(|(p, _)| p.y).min().unwrap_or(0);
                    let max_x = piece_cells.iter().map(|(p, _)| p.x).max().unwrap_or(0);
                    let max_y = piece_cells.iter().map(|(p, _)| p.y).max().unwrap_or(0);
                    (
                        (min_x, min_y),
                        (
                            (max_x - min_x + 1) as u32 * block_size,
                            (max_y - min_y + 1) as u32 * block_size,
                        ),
                    )
                };
                let sizes: Vec<(u32, u32)> =
                    pieces.iter().map(|(_, c)| bounds(c).1).collect::<Vec<_>>();
                let (dests, sheet_width, sheet_height) = shelve(&sizes);
                let mut snips = HashMap::new();
                let mut copies = vec![];
                for ((piece, piece_cells), at) in pieces.iter().zip(dests) {
                    let ((min_x, min_y), _) = bounds(piece_cells);
                    snips.insert(*piece, at);
                    for (p, id) in piece_cells {
                        let dest = Rect::new(
                            at.x() + (p.x - min_x) * block_size as i32,
                            at.y() + (p.y - min_y) * block_size as i32,
                            block_size,
                            block_size,
                        );
                        copies.push((cells[id].normal, dest));
                    }
                }
                let mut preview_texture =
                    texture_creator.create_texture_target_blended(sheet_width, sheet_height)?;
                canvas
                    .with_texture_canvas(&mut preview_texture, |c| {
                        c.set_draw_color(Color::RGBA(0, 0, 0, 0));
                        c.clear();
                        for (src_snip, dest) in copies.iter().copied() {
                            c.copy(&texture, src_snip, dest).unwrap();
                        }
                    })
                    .map_err(|e| e.to_string())?;
                PreviewSpriteSheet {
                    texture: preview_texture,
                    snips,
                }
            }
        };

        let mascot = match &data.mascot {
            Some(mascot) => {
                let mut build = |sheet: &AnimationSpriteSheetData| -> Result<_, String> {
                    let sheet = sheet.sprite_sheet(texture_creator)?;
                    match mascot.scale {
                        Some(scale) => sheet.scale_f64(canvas, texture_creator, scale),
                        None => Ok(sheet),
                    }
                };
                Some(MascotSprites {
                    idle: build(&mascot.idle)?,
                    spawn: build(&mascot.spawn)?,
                    game_over: build(&mascot.game_over)?,
                    victory: build(&mascot.victory)?,
                })
            }
            None => None,
        };

        Ok(Self {
            texture: RefCell::new(texture),
            ghost_alpha_mod: data.ghost_alpha,
            cells,
            animations,
            animation_index,
            masks,
            colors,
            block_size,
            previews,
            mascot,
        })
    }

    pub fn block_size(&self) -> u32 {
        self.block_size
    }

    pub fn previews(&self) -> &PreviewSpriteSheet<'a> {
        &self.previews
    }

    pub fn mascot(&self) -> Option<&MascotSprites<'a>> {
        self.mascot.as_ref()
    }

    pub fn mask(&self, id: CellId) -> Option<&BlockMask> {
        self.masks.get(&id)
    }

    /// The colour this theme draws a cell in, near enough for something else to be drawn to
    /// match it - a caption over the cells that just cleared, say.
    ///
    /// Read off the sheet rather than declared, so it is right on every theme including ones
    /// built from a rip, and costs a game no new contract at all.
    pub fn cell_color(&self, id: CellId) -> Option<Color> {
        self.colors.get(&id).copied()
    }

    fn cell_animations(&self, id: CellId) -> Option<&CellAnimations<'a>> {
        self.animation_index.get(&id).map(|i| &self.animations[*i])
    }

    pub fn idle_sheet(&self, id: CellId) -> Option<&AnimationSpriteSheet<'a>> {
        self.cell_animations(id).and_then(|a| a.idle.as_ref())
    }

    pub fn idle_frames(&self, id: CellId) -> Option<usize> {
        self.idle_sheet(id).map(|s| s.frame_count())
    }

    pub fn pop_frames(&self, id: CellId) -> Option<usize> {
        self.cell_animations(id)
            .and_then(|a| a.pop.as_ref())
            .map(|s| s.frame_count())
    }

    /// a [`DestroyStyle::Pop`] with the frame counts of every cell that has a pop strip
    pub fn pop_style(&self) -> DestroyStyle {
        let mut style = DestroyStyle::pop(1);
        for id in self.cells.keys() {
            if let Some(frames) = self.pop_frames(*id) {
                style = style.with_pop_frames(*id, frames);
            }
        }
        style
    }

    /// every cell with an idle strip and its frame count, for [`crate::animate::AnimationMeta`]
    pub fn idle_cells(&self) -> Vec<(CellId, usize)> {
        let mut result = self
            .cells
            .keys()
            .filter_map(|id| self.idle_frames(*id).map(|frames| (*id, frames)))
            .collect::<Vec<_>>();
        result.sort();
        result
    }

    pub fn draw_mascot(
        &self,
        canvas: &mut WindowCanvas,
        kind: MascotKind,
        point: Point,
        frame: usize,
    ) -> Result<(), String> {
        match &self.mascot {
            Some(mascot) => mascot.sheet(kind).draw_frame(canvas, point, frame),
            None => Ok(()),
        }
    }

    fn offset_by_block_ratio(&self, rect: Rect, offset_y: f64) -> Rect {
        if offset_y == 0.0 {
            return rect;
        }
        Rect::new(
            rect.x,
            (rect.y as f64 + offset_y * self.block_size as f64).round() as i32,
            rect.width(),
            rect.height(),
        )
    }

    /// draw a cell's still sprite
    pub fn draw_cell<A: Into<Option<u8>>>(
        &self,
        canvas: &mut WindowCanvas,
        id: CellId,
        stacked: bool,
        dest: Rect,
        offset_y: f64,
        alpha_mod: A,
    ) -> Result<(), String> {
        let Some(snips) = self.cells.get(&id) else {
            return Ok(());
        };
        let snip = if stacked { snips.stack } else { snips.normal };
        // set every time rather than only when it changes: the atlas is shared by every cell
        // on the board, so a fade left behind by the last draw would tint the next one
        let mut texture = self.texture.borrow_mut();
        texture.set_alpha_mod(alpha_mod.into().unwrap_or(OPAQUE));
        canvas.copy(&texture, snip, self.offset_by_block_ratio(dest, offset_y))
    }

    /// draw a cell as it sits on the stack: its idle strip if it has one, else the still sprite
    fn draw_stack_cell(
        &self,
        canvas: &mut WindowCanvas,
        id: CellId,
        dest: Rect,
        offset_y: f64,
        animations: &PlayerAnimations,
    ) -> Result<(), String> {
        match (self.idle_sheet(id), animations.cell_idle().frame(id)) {
            (Some(sheet), Some(frame)) => {
                sheet.draw_frame_scaled(canvas, self.offset_by_block_ratio(dest, offset_y), frame)
            }
            (Some(sheet), None) => {
                sheet.draw_frame_scaled(canvas, self.offset_by_block_ratio(dest, offset_y), 0)
            }
            _ => self.draw_cell(canvas, id, true, dest, offset_y, None),
        }
    }

    fn draw_outline(
        &self,
        canvas: &mut WindowCanvas,
        color: Color,
        point: CellPoint,
        dest: Rect,
        ghosts: &HashSet<CellPoint>,
    ) -> Result<(), String> {
        canvas.set_draw_color(color);
        let top_left = dest.top_left();
        let top_right = dest.top_right() - Point::new(1, 0);
        let bottom_right = dest.bottom_right() - Point::new(1, 1);
        let bottom_left = dest.bottom_left() - Point::new(0, 1);
        if !ghosts.contains(&point.translate(0, -1)) {
            canvas.draw_line(top_left, top_right)?;
        }
        if !ghosts.contains(&point.translate(1, 0)) {
            canvas.draw_line(top_right, bottom_right)?;
        }
        if !ghosts.contains(&point.translate(0, 1)) {
            canvas.draw_line(bottom_left, bottom_right)?;
        }
        if !ghosts.contains(&point.translate(-1, 0)) {
            canvas.draw_line(top_left, bottom_left)?;
        }
        Ok(())
    }

    /// Draw the board and everything animating on it.
    pub fn draw_board<G: Game>(
        &self,
        canvas: &mut WindowCanvas,
        game: &G,
        geometry: &BoardGeometry,
        animations: &PlayerAnimations,
        ghost_style: GhostStyle,
    ) -> Result<(), String> {
        if let Some(cells) = animations.next_stage().state().map(|s| s.display_cells()) {
            for (point, id) in cells {
                let dest = geometry.raw_block(point);
                self.draw_stack_cell(canvas, id, dest, 0.0, animations)?;
            }
            return Ok(());
        }

        let mut draw_active = animations.spawn().state().is_none();

        if let Some(hard_drop) = animations.hard_drop().state() {
            draw_active = false;
            for frame in hard_drop.frames() {
                for (point, id) in hard_drop.cells().iter().copied() {
                    let dest = geometry.raw_block(point);
                    self.draw_cell(canvas, id, false, dest, frame.offset_y, frame.alpha_mod)?;
                }
            }
        }

        let lock_offset_y = animations
            .lock()
            .state()
            .map(|s| s.offset_y())
            .unwrap_or(0.0);
        let lock_animates = |point: CellPoint| {
            animations
                .lock()
                .state()
                .map(|s| s.animates(point))
                .unwrap_or(false)
        };

        // how far the piece in play is into the row below, so it slides rather than stepping.
        // Zero for a game that does not interpolate, and the ghost never takes it: the ghost
        // marks the cell the piece lands in, which is a grid position whatever the piece is
        // doing between two of them
        let fall_offset = game.fall_progress();

        // a cell that is still falling in from over the top is drawn after the board, on its
        // way down, and not where the rules have already put it
        let falling = animations.nuisance().state();
        let in_the_air = |point: CellPoint| falling.is_some_and(|s| s.is_falling(point));

        let hidden = geometry.hidden_rows();
        let mut ghosts = HashSet::new();
        if let GhostStyle::Outline { .. } = ghost_style {
            for j in hidden..game.board_height() {
                for i in 0..game.board_width() {
                    let point = CellPoint::from_u32(i, j);
                    if matches!(game.cell(point), Cell::Ghost(_)) {
                        ghosts.insert(point);
                    }
                }
            }
        }

        for j in (hidden..game.board_height()).rev() {
            for i in 0..game.board_width() {
                let point = CellPoint::from_u32(i, j);
                let dest = geometry.raw_block(point);
                match game.cell(point) {
                    Cell::Empty => {}
                    Cell::Active(id) if draw_active => {
                        self.draw_cell(canvas, id, false, dest, fall_offset, None)?
                    }
                    Cell::Ghost(id) if draw_active => match ghost_style {
                        GhostStyle::Alpha => {
                            self.draw_cell(canvas, id, false, dest, 0.0, self.ghost_alpha_mod)?
                        }
                        GhostStyle::Outline { color } => {
                            self.draw_outline(canvas, color, point, dest, &ghosts)?
                        }
                        GhostStyle::None => {}
                    },
                    Cell::Stack(id) => {
                        let offset_y = if lock_animates(point) {
                            lock_offset_y
                        } else {
                            0.0
                        };
                        self.draw_stack_cell(canvas, id, dest, offset_y, animations)?
                    }
                    // through the stack path, so garbage that idles gets its strip: a Puyo
                    // nuisance blinks where a Dr. Mario virus wriggles. With no strip this
                    // is the still stacked sprite, which is what every other game gets
                    Cell::Garbage(id) if !in_the_air(point) => {
                        self.draw_stack_cell(canvas, id, dest, 0.0, animations)?
                    }
                    _ => {}
                }
            }
        }

        if let Some(falling) = falling {
            // clipped to the playfield: a theme's board texture may carry room above the top
            // row - a stone border, the sky over a well - and an attack coming in has to
            // appear over the board's own top edge rather than out of the furniture
            let clip = canvas.clip_rect();
            canvas.set_clip_rect(geometry.game_snip());
            let result = falling
                .frames()
                .into_iter()
                .try_for_each(|(point, id, offset_y)| {
                    self.draw_cell(canvas, id, true, geometry.raw_block(point), offset_y, None)
                });
            canvas.set_clip_rect(clip);
            result?;
        }

        if let Some(destroyed) = animations.destroy().state() {
            let destroy = animations.destroy();
            match destroy.style() {
                DestroyStyle::Pop { .. } => {
                    for (point, id) in destroyed.cells().iter().copied() {
                        let dest = geometry.raw_block(point);
                        let frame = destroy.pop_frame(id).unwrap_or(0);
                        match self.cell_animations(id).and_then(|a| a.pop.as_ref()) {
                            Some(sheet) => sheet.draw_frame_scaled(canvas, dest, frame)?,
                            None => self.draw_cell(canvas, id, true, dest, 0.0, None)?,
                        }
                    }
                }
                DestroyStyle::Flash => {
                    if !destroy.is_flashed_off() {
                        for (point, id) in destroyed.cells().iter().copied() {
                            let dest = geometry.raw_block(point);
                            self.draw_cell(canvas, id, true, dest, 0.0, None)?;
                        }
                    }
                }
                DestroyStyle::Vanish { .. } => {}
                DestroyStyle::Sweep => {
                    // a gap opens from the middle of each cleared row outwards, pixel by pixel
                    let progress = destroy.sweep_progress().unwrap_or(1.0);
                    let board = geometry.game_snip();
                    let half_gap = (board.width() as f64 * progress / 2.0).round() as i32;
                    let centre = board.center().x();
                    let left = Rect::new(
                        board.left(),
                        board.top(),
                        (centre - half_gap - board.left()).max(0) as u32,
                        board.height(),
                    );
                    let right = Rect::new(
                        centre + half_gap,
                        board.top(),
                        (board.right() - centre - half_gap).max(0) as u32,
                        board.height(),
                    );
                    for clip in [left, right] {
                        if clip.width() == 0 {
                            continue;
                        }
                        canvas.set_clip_rect(clip);
                        for (point, id) in destroyed.cells().iter().copied() {
                            self.draw_cell(canvas, id, true, geometry.raw_block(point), 0.0, None)?;
                        }
                    }
                    canvas.set_clip_rect(None);
                }
            }
        }

        Ok(())
    }

    /// draw the cells of a piece at their board positions (e.g. a spawning piece)
    pub fn draw_cells(
        &self,
        canvas: &mut WindowCanvas,
        cells: &[PlacedCell],
        geometry: &BoardGeometry,
    ) -> Result<(), String> {
        for (point, id) in cells.iter().copied() {
            self.draw_cell(canvas, id, false, geometry.raw_block(point), 0.0, None)?;
        }
        Ok(())
    }

    pub fn flatten<'b>(
        &self,
        canvas: &mut WindowCanvas,
        texture_creator: &'b TextureCreator<WindowContext>,
    ) -> Result<FlatSpriteSheet<'b>, String> {
        let mut idle_cells = HashMap::new();
        for id in self.cells.keys() {
            if let Some(sheet) = self.idle_sheet(*id) {
                idle_cells.insert(*id, sheet.clone(canvas, texture_creator)?);
            }
        }
        Ok(FlatSpriteSheet {
            previews: self.previews.clone(canvas, texture_creator)?,
            idle_cells,
            mascot: match &self.mascot {
                Some(mascot) => Some(mascot.clone(canvas, texture_creator)?),
                None => None,
            },
        })
    }
}
