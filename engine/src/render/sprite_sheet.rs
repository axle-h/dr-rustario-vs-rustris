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
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Texture, TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::collections::{HashMap, HashSet};

const ALPHA_STRIDE: u8 = 4;

fn alpha_stride(alpha_mod: u8) -> u8 {
    ALPHA_STRIDE * (alpha_mod as f64 / ALPHA_STRIDE as f64).round() as u8
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
    Outline { color: Color },
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
        let mut texture =
            texture_creator.create_texture_target_blended(query.width.max(1), query.height.max(1))?;
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

pub struct BlockSpriteSheet<'a> {
    texture: Texture<'a>,
    alpha_textures: HashMap<u8, Texture<'a>>,
    ghost_alpha_mod: u8,
    cells: HashMap<CellId, CellSnips>,
    animations: Vec<CellAnimations<'a>>,
    animation_index: HashMap<CellId, usize>,
    masks: HashMap<CellId, BlockMask>,
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

        // one atlas row: normal then stack sprite of every cell
        let mut cells = HashMap::new();
        let mut x = 0;
        let mut copies: Vec<(Rect, Rect, f64)> = vec![];
        for (id, sprite) in data.cells.iter() {
            let normal = Rect::new(x, 0, block_size, block_size);
            x += block_size as i32;
            copies.push((sprite.snip, normal, sprite.angle));
            let stack = match sprite.stack {
                Some(stack_src) => {
                    let stack = Rect::new(x, 0, block_size, block_size);
                    x += block_size as i32;
                    copies.push((stack_src, stack, sprite.angle));
                    stack
                }
                None => normal,
            };
            cells.insert(*id, CellSnips { normal, stack });
        }
        let width = (x as u32).max(1);
        let mut texture = texture_creator.create_texture_target_blended(width, block_size)?;
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

        let mut alpha_textures = HashMap::new();
        for i in 0..0xff / ALPHA_STRIDE {
            let alpha_mod = i * ALPHA_STRIDE;
            let mut alpha_texture =
                texture_creator.create_texture_target_blended(width, block_size)?;
            alpha_texture.set_alpha_mod(alpha_mod);
            canvas
                .with_texture_canvas(&mut alpha_texture, |c| {
                    c.set_draw_color(Color::RGBA(0, 0, 0, 0));
                    c.clear();
                    c.copy(&texture, None, None).unwrap();
                })
                .map_err(|e| e.to_string())?;
            alpha_textures.insert(alpha_mod, alpha_texture);
        }

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

        let mut masks = HashMap::new();
        for (id, snips) in cells.iter() {
            let mask = match animation_index
                .get(id)
                .and_then(|i| animations[*i].idle.as_mut())
            {
                Some(idle) => idle.block_mask(canvas, 0)?,
                None => BlockMask::from_texture(canvas, &mut texture, snips.normal)?,
            };
            masks.insert(*id, mask);
        }

        let previews = match &data.previews {
            PreviewData::Sprites { file, pieces, size } => {
                let src = texture_creator.load_texture_bytes(file)?;
                let scale = block_size as f64 / data.source_block_size as f64;
                let w = (size.0 as f64 * scale).round() as u32;
                let h = (size.1 as f64 * scale).round() as u32;
                let mut snips = HashMap::new();
                let mut copies = vec![];
                let mut x = 0;
                for (piece, src_snip) in pieces.iter() {
                    let dest = Rect::new(x, 0, w, h);
                    x += w as i32;
                    snips.insert(*piece, dest);
                    copies.push((*src_snip, dest));
                }
                let mut preview_texture =
                    texture_creator.create_texture_target_blended((x as u32).max(1), h.max(1))?;
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
                let mut snips = HashMap::new();
                let mut copies = vec![];
                let mut x = 0;
                let mut height = 1;
                for (piece, piece_cells) in pieces.iter() {
                    let min_x = piece_cells.iter().map(|(p, _)| p.x).min().unwrap_or(0);
                    let min_y = piece_cells.iter().map(|(p, _)| p.y).min().unwrap_or(0);
                    let max_x = piece_cells.iter().map(|(p, _)| p.x).max().unwrap_or(0);
                    let max_y = piece_cells.iter().map(|(p, _)| p.y).max().unwrap_or(0);
                    let w = (max_x - min_x + 1) as u32 * block_size;
                    let h = (max_y - min_y + 1) as u32 * block_size;
                    height = height.max(h);
                    snips.insert(*piece, Rect::new(x, 0, w, h));
                    for (p, id) in piece_cells {
                        let dest = Rect::new(
                            x + (p.x - min_x) * block_size as i32,
                            (p.y - min_y) * block_size as i32,
                            block_size,
                            block_size,
                        );
                        copies.push((cells[id].normal, dest));
                    }
                    x += w as i32;
                }
                let mut preview_texture =
                    texture_creator.create_texture_target_blended((x as u32).max(1), height)?;
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
            texture,
            alpha_textures,
            ghost_alpha_mod: alpha_stride(data.ghost_alpha),
            cells,
            animations,
            animation_index,
            masks,
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

    fn alpha_texture(&self, alpha_mod: u8) -> &Texture<'a> {
        self.alpha_textures
            .get(&alpha_mod)
            .or_else(|| self.alpha_textures.get(&alpha_stride(alpha_mod)))
            .unwrap_or(&self.texture)
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
        let texture = match alpha_mod.into() {
            Some(alpha_mod) => self.alpha_texture(alpha_mod),
            None => &self.texture,
        };
        canvas.copy(texture, snip, self.offset_by_block_ratio(dest, offset_y))
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
                        self.draw_cell(canvas, id, false, dest, 0.0, None)?
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
                    Cell::Garbage(id) => self.draw_cell(canvas, id, true, dest, 0.0, None)?,
                    _ => {}
                }
            }
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
