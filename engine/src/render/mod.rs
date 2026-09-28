//! Everything drawn for one game on one theme. A [`Theme`] is data assembled by a builder
//! ([`retro::retro_theme`] or [`modern::modern_theme`]) from what a game crate declares.

pub mod animation;
pub mod block_mask;
pub mod character;
pub mod context;
pub mod font;
pub mod geometry;
pub mod helper;
pub mod layout;
pub mod metrics_table;
pub mod modern;
pub mod pause;
pub mod retro;
pub mod scene;
pub mod sound;
pub mod sprite_sheet;
pub mod timer;

use crate::animate::character::CharacterFrame;
use crate::animate::game_over::{CurtainPhase, GameOverStyle};
use crate::animate::nuisance::NuisanceFall;
use crate::animate::{AnimationMeta, PlayerAnimations};
use crate::game::{CellId, Game, GameEvent, PieceId, PlacedCell};
use crate::particles::particle::ParticleAnimationType;
use crate::particles::prescribed::RaceTheme;
use crate::render::animation::{AnimationSpriteSheet, AnimationSpriteSheetData};
use crate::render::character::{CharacterLayout, CharacterSet};
use crate::render::font::{FontTheme, PopupFont};
use crate::render::geometry::BoardGeometry;
use crate::render::scene::SceneRender;
use crate::render::sound::AudioTheme;
use crate::render::sprite_sheet::{BlockSpriteSheet, GhostStyle, MascotKind};
use crate::scale::{Scale, ScaleMode};
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{BlendMode, Texture, WindowCanvas};

/// What a game tells the renderer beyond its board: how to grade events for sound and
/// particles. Everything visual comes from theme data.
pub trait GameRender {
    /// what this game is called, for the background field to spell out
    fn name(&self) -> &'static str;

    /// grade a [`GameEvent::Clear`] for [`sound::SfxKey::Clear`]
    fn clear_class(&self, event: &GameEvent) -> u16 {
        let _ = event;
        0
    }

    /// A word for the background field to spell out when this happens, if it deserves one. The
    /// words are the engine's, see [`crate::particles::field::reaction::words`]; a game only says
    /// when.
    fn clear_word(&self, event: &GameEvent) -> Option<&'static str> {
        let _ = event;
        None
    }

    /// A short caption to draw over the cells this event cleared, if it deserves one. Unlike
    /// [`Self::clear_word`] it is local and may fire on every clear, and the game owns the text.
    fn clear_popup(&self, event: &GameEvent) -> Option<String> {
        let _ = event;
        None
    }

    /// the cells a freshly spawned piece occupies, for spawn particles
    fn spawn_cells(&self) -> Vec<crate::game::geometry::Point>;

    /// cells already on the board when a stage starts (Dr. Mario's viruses); they pop in
    fn stage_intro_cells(&self) -> Vec<PlacedCell> {
        vec![]
    }

    /// How an attack falls in from over the top of the board. A game that answers holds its play
    /// while it lands; `None` draws the cells where they land and carries on.
    fn attack_fall(&self) -> Option<NuisanceFall> {
        None
    }
}

/// Where the mascot sits and the piece it holds.
#[derive(Clone, Copy, Debug)]
pub struct MascotLayout {
    /// where the next piece waits in the mascot's hand
    pub hand_point: Point,
    pub spawn_point: Point,
    pub game_over_point: Point,
    pub victory_point: Point,
    /// draw the mascot before the piece in its hand, so the piece overlaps it
    pub draw_first: bool,
}

/// Where queued pieces are drawn.
#[derive(Clone, Debug)]
pub enum PeekLayout {
    /// a column of pieces starting at `point`, each `offset` further down; with a mascot the first
    /// is in its hand
    Column {
        point: Point,
        offset: i32,
        max: u32,
        scale: Option<f64>,
    },
    /// explicit slots, each filled by one piece scaled to fit
    Slots { slots: Vec<Rect>, max_scale: f64 },
}

#[derive(Clone, Debug)]
pub enum HoldLayout {
    Point { point: Point, scale: Option<f64> },
    Slot { slot: Rect, max_scale: f64 },
}

/// Where the attacks queued against a player are drawn, in background source pixels, using the
/// theme's own cell sprites. A theme with no `pending` layout draws no strip.
#[derive(Clone, Debug)]
pub struct PendingLayout {
    /// the top left of the icon nearest the front of the queue
    pub point: Point,
    /// how far the next icon sits from the last; negative fills leftwards or upwards
    pub step: Point,
    /// the side of one icon, in source pixels
    pub size: u32,
    /// how many the strip has room for; anything queued past this is not drawn
    pub max: u32,
}

impl PendingLayout {
    /// where each of `count` queued attacks is drawn, front of the queue first
    pub fn slots(&self, count: usize) -> Vec<Rect> {
        (0..count.min(self.max as usize))
            .map(|i| {
                Rect::new(
                    self.point.x() + self.step.x() * i as i32,
                    self.point.y() + self.step.y() * i as i32,
                    self.size,
                    self.size,
                )
            })
            .collect()
    }

    /// Where an arriving attack belongs: the middle of the strip, in background pixels.
    /// `Theme::draw_pending` slides every new icon out of here, so the ball bursts here.
    pub fn origin(&self) -> Point {
        let middle = self.max as f64 / 2.0;
        Point::new(
            self.point.x() + (self.step.x() as f64 * middle).round() as i32 + self.size as i32 / 2,
            self.point.y() + (self.step.y() as f64 * middle).round() as i32 + self.size as i32 / 2,
        )
    }
}

/// The art an attack crosses the window as: a sprite of its own in the sending player's colour,
/// bigger than a cell. A theme with none falls back to the popped cell's sprite.
#[derive(Clone, Debug)]
pub struct AttackBallData {
    pub sheet: AnimationSpriteSheetData,
    /// how many blocks across the ball is drawn at its full size
    pub scale: f64,
    /// how big an attack, in the receiver's own units, counts as a big one
    pub big_attack: u32,
}

pub(crate) struct AttackBallSprites<'a> {
    sheet: AnimationSpriteSheet<'a>,
    scale: f64,
    big_attack: u32,
}

impl<'a> AttackBallSprites<'a> {
    pub(crate) fn new(sheet: AnimationSpriteSheet<'a>, scale: f64, big_attack: u32) -> Self {
        Self {
            sheet,
            scale,
            big_attack,
        }
    }

    /// The frame for an attack of `strength` sent by `player`. The strip is player-major, big
    /// first, and wraps, so one cut pair serves every player.
    fn frame(&self, player: u32, strength: u32) -> usize {
        let pairs = (self.sheet.frame_count() / 2).max(1);
        let small = usize::from(strength < self.big_attack);
        (player as usize % pairs) * 2 + small
    }
}

/// How a match-end overlay is placed on the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayFit {
    /// stretched over the whole board
    Stretch,
    /// drawn at its own size in the middle of the board
    Center,
}

/// Full-board overlays for the end of a match or stage.
pub struct MatchEndSprites<'a> {
    pub texture: Texture<'a>,
    pub game_over_snips: Vec<Rect>,
    pub interstitial_snips: Vec<Rect>,
    pub fit: OverlayFit,
}

impl<'a> MatchEndSprites<'a> {
    fn dest(&self, snip: Rect, game_snip: Rect) -> Rect {
        match self.fit {
            OverlayFit::Stretch => game_snip,
            OverlayFit::Center => {
                Rect::from_center(game_snip.center(), snip.width(), snip.height())
            }
        }
    }
}

/// Which family a theme belongs to: a retro console rebuild, or the engine's own modern look with
/// a particle field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeFamily {
    Retro,
    Particle,
}

/// how much of the board's width a popup may take, in sixteenths
const POPUP_MAX_BOARD_WIDTH: u32 = 15;

/// Called as each of a game's themes finishes building, so a caller can draw progress; see
/// [`crate::app::loading`].
pub type ThemeProgress<'a> = dyn FnMut(&mut WindowCanvas) -> Result<(), String> + 'a;

pub struct Theme<'a> {
    pub(crate) name: &'static str,
    pub(crate) family: ThemeFamily,
    pub(crate) scenes: Vec<SceneRender<'a>>,
    pub(crate) sprites: BlockSpriteSheet<'a>,
    pub(crate) geometry: BoardGeometry,
    pub(crate) audio: AudioTheme,
    pub(crate) font: FontTheme<'a>,
    /// draws [`crate::animate::popup::Popup`]s over the board; every theme has one, since whether
    /// there are popups is the game's decision
    pub(crate) popup_font: PopupFont<'a>,
    /// the board frame per speed band, drawn under the cells
    pub(crate) board_texture: Texture<'a>,
    pub(crate) board_snips: Vec<Rect>,
    pub(crate) background_texture: Texture<'a>,
    /// where the board texture sits within the background
    pub(crate) board_bg_snip: Rect,
    pub(crate) background_size: (u32, u32),
    pub(crate) background_color: Color,
    pub(crate) mascot: Option<MascotLayout>,
    /// the cast this theme draws beside the board, and where it puts one
    pub(crate) characters: Option<(CharacterSet<'a>, CharacterLayout)>,
    pub(crate) animation_meta: AnimationMeta,
    pub(crate) match_end: Option<MatchEndSprites<'a>>,
    /// the cell drawn by a curtain game over
    pub(crate) curtain_cell: Option<CellId>,
    pub(crate) hold: Option<HoldLayout>,
    pub(crate) peek: PeekLayout,
    /// where attacks queued against the player are drawn, for a game that holds them
    pub(crate) pending: Option<PendingLayout>,
    /// what an attack crossing the window is drawn as, for a theme that cut art for it
    pub(crate) attack_ball: Option<AttackBallSprites<'a>>,
    pub(crate) ghost_style: GhostStyle,
    /// themes that emit particles do so in this colour
    pub(crate) particle_color: Option<Color>,
    /// the colours this theme radiates into the background particle field; empty falls back to
    /// another theme of the same game
    pub(crate) particle_palette: Vec<Color>,
    /// how this theme's art may be resized to the window
    pub(crate) scale_mode: ScaleMode,
    /// source pixels at the top of the background that nothing is ever drawn into, so they may fall
    /// outside the window rather than cost the board a whole step
    pub(crate) top_slack: u32,
    /// what the panel casts on the scene behind it, for a theme that wants lifting off one
    pub(crate) shadow: Option<PanelShadow>,
}

/// A shadow under a theme's panel, drawn on the scene behind it at composite time. Painting it into
/// the panel art would shrink the board, since cell size is set by the panel's source pixels.
#[derive(Clone, Copy, Debug)]
pub struct PanelShadow {
    /// how far down and to the right of the panel it falls, in source pixels
    pub offset: (i32, i32),
    /// how far past the panel it fades out, in source pixels, down and to the right only
    pub spread: u32,
    pub color: Color,
    /// how dark it is at the panel's edge, fading to nothing at the spread
    pub alpha: u8,
    /// Source pixels round the edge of the background that the panel's art does not fill, and which
    /// therefore cast nothing: `(left, top, right, bottom)`.
    pub margin: (u32, u32, u32, u32),
}

impl PanelShadow {
    /// Draw it under `panel`, which is where the panel goes in the window. It only grows down and
    /// to the right, and fades in one outline per window pixel so it is smooth at any scale.
    pub fn draw(
        &self,
        canvas: &mut WindowCanvas,
        panel: Rect,
        scale: &Scale,
    ) -> Result<(), String> {
        let px = |value: i32| scale.scale_coordinate(value);
        let (left, top, right, bottom) = self.margin;
        let (left, top) = (px(left as i32), px(top as i32));
        let (right, bottom) = (px(right as i32), px(bottom as i32));
        let body = Rect::new(
            panel.x() + px(self.offset.0) + left,
            panel.y() + px(self.offset.1) + top,
            panel.width().saturating_sub((left + right) as u32).max(1),
            panel.height().saturating_sub((top + bottom) as u32).max(1),
        );
        let blend = canvas.blend_mode();
        canvas.set_blend_mode(BlendMode::Blend);
        let color = |alpha: u8| Color::RGBA(self.color.r, self.color.g, self.color.b, alpha);
        canvas.set_draw_color(color(self.alpha));
        canvas.fill_rect(body)?;
        let spread = px(self.spread as i32).max(1);
        for step in 1..=spread {
            let fade = self.alpha as i32 * (spread - step) / spread;
            canvas.set_draw_color(color(fade as u8));
            canvas.draw_rect(Rect::new(
                body.x(),
                body.y(),
                body.width() + step as u32,
                body.height() + step as u32,
            ))?;
        }
        canvas.set_blend_mode(blend);
        Ok(())
    }
}

impl<'a> Theme<'a> {
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn family(&self) -> ThemeFamily {
        self.family
    }

    pub fn sprites(&self) -> &BlockSpriteSheet<'a> {
        &self.sprites
    }

    fn band(&self, speed_index: u32) -> usize {
        (speed_index as usize).min(self.scenes.len().saturating_sub(1))
    }

    pub fn scene(&self, speed_index: u32) -> &SceneRender<'a> {
        &self.scenes[self.band(speed_index)]
    }

    pub fn animation_meta(&self) -> &AnimationMeta {
        &self.animation_meta
    }

    pub fn geometry(&self) -> &BoardGeometry {
        &self.geometry
    }

    pub fn background_size(&self) -> (u32, u32) {
        self.background_size
    }

    pub fn background_color(&self) -> Color {
        self.background_color
    }

    pub fn board_snip(&self) -> Rect {
        self.board_bg_snip
    }

    pub fn audio(&self) -> &AudioTheme {
        &self.audio
    }

    pub fn particle_color(&self) -> Option<Color> {
        self.particle_color
    }

    pub fn particle_palette(&self) -> &[Color] {
        &self.particle_palette
    }

    pub fn scale_mode(&self) -> ScaleMode {
        self.scale_mode
    }

    pub fn top_slack(&self) -> u32 {
        self.top_slack
    }

    pub fn shadow(&self) -> Option<PanelShadow> {
        self.shadow
    }

    /// the playfield within the background, in source pixels: the board frame's place in the
    /// background plus the board's own offset within that frame
    pub fn playfield_snip(&self) -> Rect {
        let snip = self.geometry.game_snip();
        Rect::new(
            self.board_bg_snip.x() + snip.x(),
            self.board_bg_snip.y() + snip.y(),
            snip.width(),
            snip.height(),
        )
    }

    /// what this theme contributes to the menu's piece race, see
    /// [`crate::particles::prescribed::prescribed_piece_race`]
    pub fn race_theme(&self, index: usize, pieces: Vec<PieceId>, scale: f64) -> RaceTheme {
        let meta = &self.animation_meta;
        RaceTheme {
            theme: index,
            pieces,
            cells: meta
                .cell_idle
                .iter()
                .map(|(id, frames)| {
                    (
                        *id,
                        ParticleAnimationType::from_frames(meta.cell_idle_type, *frames),
                    )
                })
                .collect(),
            mascot: meta
                .mascot
                .map(|m| ParticleAnimationType::from_frames(m.idle_type, m.idle_frames)),
            scale,
        }
    }

    fn draw_mascot(
        &self,
        canvas: &mut WindowCanvas,
        kind: MascotKind,
        frame: Option<usize>,
    ) -> Result<(), String> {
        let Some(layout) = self.mascot else {
            return Ok(());
        };
        let point = match kind {
            MascotKind::Idle | MascotKind::Spawn => layout.spawn_point,
            MascotKind::GameOver => layout.game_over_point,
            MascotKind::Victory => layout.victory_point,
        };
        self.sprites
            .draw_mascot(canvas, kind, point, frame.unwrap_or(0))
    }

    fn draw_hold(&self, canvas: &mut WindowCanvas, piece: PieceId) -> Result<(), String> {
        match &self.hold {
            Some(HoldLayout::Point { point, scale }) => self
                .sprites
                .previews()
                .draw_piece(canvas, piece, *point, None, *scale),
            Some(HoldLayout::Slot { slot, max_scale }) => self
                .sprites
                .previews()
                .draw_piece_fill(canvas, piece, *slot, *max_scale),
            None => Ok(()),
        }
    }

    /// The player's character, drawn into the panel texture, which clips it to its box. A mugshot
    /// fills `layout.rect`; a routine pose is drawn on the window by
    /// `ThemeContext::draw_placed_characters`.
    fn draw_character(
        &self,
        canvas: &mut WindowCanvas,
        animations: &PlayerAnimations,
    ) -> Result<(), String> {
        let Some((set, layout)) = self.characters.as_ref() else {
            return Ok(());
        };
        let character = animations.character();
        let Some(index) = character.character() else {
            return Ok(());
        };
        let mirrored = character.mirrored();
        let box_width = layout.rect.width() as i32;
        let drawing = character.drawing();
        set.with(index, |sprites| {
            match drawing {
                CharacterFrame::Hidden => {}
                CharacterFrame::Whole(frame) => {
                    sprites.draw(canvas, layout.rect, character.state(), frame, mirrored)?;
                }
                // drawn on the window by `ThemeContext::draw_placed_characters`, over the stone
                // above the arch
                CharacterFrame::Placed(..) => {}
            }
            // ... and every layer over it, in the order the theme declared them
            for (layer, frame, anchor) in character.layers() {
                let Some((width, height)) = set.layer_size(index, layer) else {
                    continue;
                };
                // a mirrored anchor is measured from the other edge of the box
                let x = if mirrored {
                    box_width - anchor.0 - width as i32
                } else {
                    anchor.0
                };
                let dest = Rect::new(
                    layout.rect.x() + x,
                    layout.rect.y() + anchor.1,
                    width,
                    height,
                );
                sprites.draw_layer(canvas, dest, layer, character.state(), frame, mirrored)?;
            }
            Ok(())
        })?;
        Ok(())
    }

    /// The character's current pose on the window at `at`, unclipped, for `kirby_shot`.
    pub(crate) fn draw_character_unclipped(
        &self,
        canvas: &mut WindowCanvas,
        animations: &PlayerAnimations,
        at: Rect,
    ) -> Result<(), String> {
        let Some((set, layout)) = self.characters.as_ref() else {
            return Ok(());
        };
        let character = animations.character();
        let Some(index) = character.character() else {
            return Ok(());
        };
        let CharacterFrame::Placed(pose, place) = character.drawing() else {
            return Ok(());
        };
        let Some((width, height)) = set.pose_size(index, pose) else {
            return Ok(());
        };
        let mirrored = character.mirrored();
        let scale = at.width() as f64 / layout.rect.width() as f64;
        let x = if mirrored {
            layout.rect.width() as i32 - place.0 - width as i32
        } else {
            place.0
        };
        let dest = Rect::new(
            at.x() + (x as f64 * scale).round() as i32,
            at.y() + (place.1 as f64 * scale).round() as i32,
            (width as f64 * scale).round() as u32,
            (height as f64 * scale).round() as u32,
        );
        set.with(index, |sprites| {
            sprites.draw_pose(canvas, dest, pose, mirrored)
        })?;
        Ok(())
    }

    /// The particles a character has thrown, on the window and unclipped. `origin` is the panel's
    /// top left, since the box is panel furniture.
    pub(crate) fn draw_character_particles(
        &self,
        canvas: &mut WindowCanvas,
        animations: &PlayerAnimations,
        scale: &Scale,
        origin: Point,
    ) -> Result<(), String> {
        let Some((set, layout)) = self.characters.as_ref() else {
            return Ok(());
        };
        let character = animations.character();
        let Some(index) = character.character() else {
            return Ok(());
        };
        if character.particles().is_empty() {
            return Ok(());
        }
        let mirrored = character.mirrored();
        let box_width = layout.rect.width() as f64;
        set.with(index, |sprites| {
            for particle in character.particles() {
                let Some((width, height)) = set.particle_size(index, particle.emitter) else {
                    continue;
                };
                // box coordinates, so one flip about the box's middle turns the whole spray
                let x = if mirrored {
                    box_width - particle.x
                } else {
                    particle.x
                };
                let at = Rect::new(
                    layout.rect.x() + x.round() as i32 - width as i32 / 2,
                    layout.rect.y() + particle.y.round() as i32 - height as i32 / 2,
                    width,
                    height,
                );
                let dest = scale.scale_and_offset_rect(at, origin.x(), origin.y());
                sprites.draw_particle(
                    canvas,
                    dest,
                    particle.emitter,
                    particle.frame,
                    mirrored,
                    particle.alpha(),
                )?;
            }
            Ok(())
        })?;
        Ok(())
    }

    /// The strip of attacks waiting to land. An icon whose attack is still crossing the window is
    /// not drawn, and one that has just landed slides in from the middle; see
    /// [`crate::animate::tray`].
    fn draw_pending<G: Game>(
        &self,
        canvas: &mut WindowCanvas,
        game: &G,
        animations: &PlayerAnimations,
    ) -> Result<(), String> {
        let Some(layout) = &self.pending else {
            return Ok(());
        };
        let pending = game.pending_attacks();
        let tray = animations.tray();
        let shown = tray.visible(pending.len());
        for (index, (dest, id)) in layout.slots(shown).into_iter().zip(pending).enumerate() {
            let dest = match tray.slide(index) {
                // it comes in from over the middle of the strip, which is over the board
                Some(t) => {
                    let back = (layout.max as f64 / 2.0 - index as f64) * (1.0 - t);
                    Rect::new(
                        dest.x() + (layout.step.x() as f64 * back).round() as i32,
                        dest.y() + (layout.step.y() as f64 * back).round() as i32,
                        dest.width(),
                        dest.height(),
                    )
                }
                None => dest,
            };
            self.sprites.draw_cell(canvas, id, false, dest, 0.0, None)?;
        }
        Ok(())
    }

    fn draw_queue(
        &self,
        canvas: &mut WindowCanvas,
        queue: &[PieceId],
        spawn_peek_offset: Option<f64>,
    ) -> Result<(), String> {
        match &self.peek {
            PeekLayout::Column {
                point,
                offset,
                max,
                scale,
            } => {
                let skip = if self.mascot.is_some() { 1 } else { 0 };
                let shift = spawn_peek_offset
                    .map(|o| *offset - (o * *offset as f64).round() as i32)
                    .unwrap_or(0);
                for (i, piece) in queue.iter().skip(skip).take(*max as usize).enumerate() {
                    let dest = point.offset(0, shift + i as i32 * *offset);
                    self.sprites
                        .previews()
                        .draw_piece(canvas, *piece, dest, None, *scale)?;
                }
                Ok(())
            }
            PeekLayout::Slots { slots, max_scale } => {
                let first = slots.first().map(|s| s.width()).unwrap_or(1).max(1) as f64;
                for (slot, piece) in slots.iter().zip(queue.iter()) {
                    // smaller slots scale their pieces down in proportion
                    let scale = max_scale * slot.width() as f64 / first;
                    self.sprites
                        .previews()
                        .draw_piece_fill(canvas, *piece, *slot, scale)?;
                }
                Ok(())
            }
        }
    }

    pub fn draw_background<G: Game>(
        &self,
        canvas: &mut WindowCanvas,
        game: &G,
        animations: &PlayerAnimations,
    ) -> Result<(), String> {
        canvas.set_draw_color(Color::RGBA(0, 0, 0, 0));
        canvas.clear();
        let (width, height) = self.background_size;
        canvas.copy(
            &self.background_texture,
            None,
            Rect::new(0, 0, width, height),
        )?;

        let queue = game.queue();
        let mut spawn_peek_offset = None;
        let mut draw_previews = true;

        if let Some(game_over) = animations.game_over().state() {
            if self.mascot.is_some() {
                self.draw_mascot(canvas, MascotKind::GameOver, game_over.mascot_frame())?;
                draw_previews = false;
            }
        } else if let Some(victory) = animations.victory().state() {
            if self.mascot.is_some() {
                self.draw_mascot(canvas, MascotKind::Victory, victory.mascot_frame())?;
                draw_previews = false;
            }
        } else if let Some(interstitial) = animations.interstitial().state() {
            if self.mascot.is_some() {
                self.draw_mascot(canvas, MascotKind::Victory, interstitial.mascot_frame())?;
                draw_previews = false;
            }
        } else if let Some(spawn) = animations.spawn().state() {
            spawn_peek_offset = spawn.peek_offset();
            if let Some(layout) = self.mascot {
                let draw_piece = |canvas: &mut WindowCanvas| {
                    self.sprites.previews().draw_piece(
                        canvas,
                        spawn.piece(),
                        spawn.throw_position(),
                        spawn.piece_rotate_angle_degrees(),
                        None,
                    )
                };
                if layout.draw_first {
                    self.draw_mascot(canvas, MascotKind::Spawn, spawn.mascot_frame())?;
                    draw_piece(canvas)?;
                } else {
                    draw_piece(canvas)?;
                    self.draw_mascot(canvas, MascotKind::Spawn, spawn.mascot_frame())?;
                }
            }
        } else if let Some(layout) = self.mascot {
            let frame = animations.mascot_idle_frame();
            let draw_hand = |canvas: &mut WindowCanvas| match queue.first() {
                Some(piece) => self.sprites.previews().draw_piece(
                    canvas,
                    *piece,
                    layout.hand_point,
                    None,
                    None,
                ),
                None => Ok(()),
            };
            if layout.draw_first {
                self.draw_mascot(canvas, MascotKind::Idle, frame)?;
                draw_hand(canvas)?;
            } else {
                draw_hand(canvas)?;
                self.draw_mascot(canvas, MascotKind::Idle, frame)?;
            }
        }

        if draw_previews {
            if let Some(hold) = game.held() {
                self.draw_hold(canvas, hold)?;
            }
            self.draw_queue(canvas, &queue, spawn_peek_offset)?;
        }
        self.draw_pending(canvas, game, animations)?;
        self.draw_character(canvas, animations)?;

        self.font.render_all(canvas, game)
    }

    /// The captions a game asked for, drawn on the window after the foreground particles so a
    /// clear's burst does not cover them. `origin` and `scale` are where and how big the board
    /// texture is drawn.
    pub(crate) fn draw_popups(
        &self,
        canvas: &mut WindowCanvas,
        animations: &PlayerAnimations,
        scale: &Scale,
        origin: Point,
    ) -> Result<(), String> {
        let block = self.geometry.block_size() as f64;
        let half = self.geometry.block_size() as i32 / 2;
        let left = self
            .geometry
            .point(crate::game::geometry::Point::new(0, 0))
            .x();
        // a caption is held to the board's width so it cannot run over the HUD or the other player
        let widest = self.geometry.width() * POPUP_MAX_BOARD_WIDTH / 16;
        for popup in animations.popup().active() {
            let (column, row) = popup.at();
            let anchor = self
                .geometry
                .point(crate::game::geometry::Point::new(0, row.round() as i32));
            let mut size = popup.scale();
            let natural = self.popup_font.width(popup.text(), size);
            if natural > widest {
                size *= widest as f64 / natural as f64;
            }
            // ... and inside it, so a caption over the first column is not cut off
            let width = self.popup_font.width(popup.text(), size) as i32;
            let x = (left + (column * block).round() as i32 + half).clamp(
                left + width / 2,
                left + self.geometry.width() as i32 - width / 2,
            );
            let center = Point::new(x, anchor.y() + half - (popup.rise() * block).round() as i32);
            // the colour of the cells that popped
            let color = popup
                .cell()
                .and_then(|id| self.sprites.cell_color(id))
                .unwrap_or(Color::WHITE);
            self.popup_font.draw(
                canvas,
                scale.scale_and_offset_point(center, origin.x(), origin.y()),
                popup.text(),
                size * scale.factor(),
                color,
            )?;
        }
        Ok(())
    }

    /// one of this theme's cells at any size and place, for something drawn off the board
    pub(crate) fn draw_loose_cell(
        &self,
        canvas: &mut WindowCanvas,
        id: CellId,
        dest: Rect,
    ) -> Result<(), String> {
        self.sprites.draw_cell(canvas, id, false, dest, 0.0, None)
    }

    /// Where an attack arriving at this theme bursts, in background pixels; see
    /// [`PendingLayout::origin`]. `None` on a theme with no tray.
    pub(crate) fn pending_origin(&self) -> Option<Point> {
        self.pending.as_ref().map(PendingLayout::origin)
    }

    /// [`Theme::pending_origin`] in this board's cells, the unit [`crate::animate::debris`] uses. A
    /// theme with no tray bursts over the middle of its top row.
    pub(crate) fn attack_arrival_cell(&self) -> (f64, f64) {
        let hidden = self.geometry.hidden_rows() as f64;
        let Some(at) = self.pending_origin() else {
            return (self.geometry.columns() as f64 / 2.0, hidden);
        };
        let origin = self
            .geometry
            .point(crate::game::geometry::Point::new(0, hidden as i32));
        let block = self.geometry.block_size() as f64;
        (
            (at.x() - self.board_bg_snip.x() - origin.x()) as f64 / block - 0.5,
            (at.y() - self.board_bg_snip.y() - origin.y()) as f64 / block - 0.5 + hidden,
        )
    }

    /// how many blocks across an attack ball is drawn: the theme's own scale, or one cell
    pub(crate) fn attack_ball_scale(&self) -> f64 {
        self.attack_ball.as_ref().map_or(1.0, |b| b.scale)
    }

    /// An attack crossing the window in this theme's art; `false` when it has none, so the caller
    /// can fall back to the popped cell.
    pub(crate) fn draw_attack_ball(
        &self,
        canvas: &mut WindowCanvas,
        player: u32,
        strength: u32,
        dest: Rect,
    ) -> Result<bool, String> {
        let Some(ball) = self.attack_ball.as_ref() else {
            return Ok(false);
        };
        ball.sheet
            .draw_frame_scaled(canvas, dest, ball.frame(player, strength))?;
        Ok(true)
    }

    /// Every piece one player has in the air, on the window so it is not clipped at the board's
    /// edge. `origin` and `scale` are as for `draw_popups`.
    pub(crate) fn draw_debris(
        &self,
        canvas: &mut WindowCanvas,
        animations: &PlayerAnimations,
        scale: &Scale,
        origin: Point,
    ) -> Result<(), String> {
        let block = self.geometry.block_size() as f64;
        for piece in animations.debris().pieces() {
            let size = (block * piece.size).round().max(1.0) as u32;
            // a piece is placed by its middle, so a burst is centred on the cell it came from
            let at = self.geometry.point(crate::game::geometry::Point::new(0, 0));
            let x = at.x() + (piece.x * block).round() as i32 - size as i32 / 2;
            let y = at.y()
                + ((piece.y - self.geometry.hidden_rows() as f64) * block).round() as i32
                - size as i32 / 2;
            let dest =
                scale.scale_and_offset_rect(Rect::new(x, y, size, size), origin.x(), origin.y());
            self.sprites
                .draw_debris_piece(canvas, piece.art, dest, piece.alpha())?;
        }
        Ok(())
    }

    pub fn draw_board<G: Game>(
        &self,
        canvas: &mut WindowCanvas,
        game: &G,
        animations: &PlayerAnimations,
    ) -> Result<(), String> {
        canvas.set_draw_color(Color::RGBA(0, 0, 0, 0));
        canvas.clear();

        let board_snip = self.board_snips
            [(game.speed_index() as usize).min(self.board_snips.len().saturating_sub(1))];
        let board_dest = Rect::new(0, 0, board_snip.width(), board_snip.height());
        canvas.copy(&self.board_texture, board_snip, board_dest)?;

        let curtain_phase = animations.game_over().curtain_phase();
        if curtain_phase == Some(CurtainPhase::Opening) {
            // the board is gone behind the curtain, the game over card takes its place
            if let Some(match_end) = &self.match_end {
                if let Some(snip) = match_end.game_over_snips.first() {
                    canvas.copy(
                        &match_end.texture,
                        *snip,
                        match_end.dest(*snip, self.geometry.game_snip()),
                    )?;
                }
            }
        } else {
            // a draining board is clipped to the well, so the puyos slide under the frame
            let draining = animations.game_over().drain().is_some();
            let clip = canvas.clip_rect();
            if draining {
                canvas.set_clip_rect(self.geometry.game_snip());
            }
            let result =
                self.sprites
                    .draw_board(canvas, game, &self.geometry, animations, self.ghost_style);
            if draining {
                canvas.set_clip_rect(clip);
            }
            result?;
        }

        if let (Some(rows), Some(height)) = (
            animations.game_over().curtain_rows(),
            animations.game_over().curtain_height(),
        ) {
            if let Some(cell) = self.curtain_cell {
                // the curtain closes from the board's floor up, leaving any buffer zone above the
                // skyline clear
                let floor = self.geometry.rows().saturating_sub(height);
                for j in rows {
                    for i in 0..self.geometry.columns() {
                        let point = crate::game::geometry::Point::from_u32(i, floor + j);
                        self.sprites.draw_cell(
                            canvas,
                            cell,
                            true,
                            self.geometry.raw_block(point),
                            0.0,
                            None,
                        )?;
                    }
                }
            }
        }

        if let Some(match_end) = &self.match_end {
            if let Some(frame) = animations
                .game_over()
                .state()
                .and_then(|s| s.screen_frame())
            {
                if let GameOverStyle::Screen { .. } = animations.game_over().style() {
                    if let Some(snip) = match_end.game_over_snips.get(frame) {
                        let dest = match_end.dest(*snip, self.geometry.game_snip());
                        canvas.copy(&match_end.texture, *snip, dest)?;
                    }
                }
            } else if let Some(frame) = animations
                .interstitial()
                .state()
                .map(|s| s.interstitial_frame())
            {
                if let Some(snip) = match_end.interstitial_snips.get(frame) {
                    let dest = match_end.dest(*snip, self.geometry.game_snip());
                    canvas.copy(&match_end.texture, *snip, dest)?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip() -> PendingLayout {
        PendingLayout {
            point: Point::new(20, 8),
            step: Point::new(16, 0),
            size: 16,
            max: 4,
        }
    }

    #[test]
    fn the_pending_strip_fills_from_the_front_of_the_queue() {
        assert_eq!(strip().slots(0), vec![]);
        assert_eq!(
            strip().slots(2),
            vec![Rect::new(20, 8, 16, 16), Rect::new(36, 8, 16, 16)]
        );
    }

    /// A queue longer than the strip draws only what it has room for.
    #[test]
    fn the_pending_strip_stops_when_it_runs_out_of_room() {
        assert_eq!(strip().slots(99).len(), 4);
    }

    /// Every arriving icon starts at `origin`, where the ball bursts.
    #[test]
    fn an_arriving_icon_starts_where_the_ball_burst() {
        for layout in [
            strip(),
            PendingLayout {
                step: Point::new(0, 16),
                ..strip()
            },
        ] {
            let middle = layout.max as usize / 2;
            for index in 0..layout.max as usize {
                let slot = layout.slots(layout.max as usize)[index];
                // the offset `draw_pending` applies at t = 0
                let back = layout.max as f64 / 2.0 - index as f64;
                let at = Point::new(
                    slot.x()
                        + (layout.step.x() as f64 * back).round() as i32
                        + slot.width() as i32 / 2,
                    slot.y()
                        + (layout.step.y() as f64 * back).round() as i32
                        + slot.height() as i32 / 2,
                );
                assert_eq!(at, layout.origin(), "icon {index} of {middle}");
            }
        }
    }

    /// A negative step fills the other way.
    #[test]
    fn a_pending_strip_may_fill_backwards() {
        let layout = PendingLayout {
            step: Point::new(-16, 0),
            ..strip()
        };
        assert_eq!(
            layout.slots(2),
            vec![Rect::new(20, 8, 16, 16), Rect::new(4, 8, 16, 16)]
        );
    }
}
