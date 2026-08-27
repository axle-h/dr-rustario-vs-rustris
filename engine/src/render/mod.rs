//! Everything drawn for one game on one theme. A [`Theme`] is data assembled by a builder
//! ([`retro::retro_theme`] or [`modern::modern_theme`]) from what a game crate declares.

pub mod animation;
pub mod block_mask;
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

use crate::animate::game_over::{CurtainPhase, GameOverStyle};
use crate::animate::{AnimationMeta, PlayerAnimations};
use crate::game::{CellId, Game, GameEvent, PieceId, PlacedCell};
use crate::particles::particle::ParticleAnimationType;
use crate::particles::prescribed::RaceTheme;
use crate::render::font::{FontTheme, PopupFont};
use crate::render::geometry::BoardGeometry;
use crate::render::scene::SceneRender;
use crate::render::sound::AudioTheme;
use crate::render::sprite_sheet::{BlockSpriteSheet, GhostStyle, MascotKind};
use crate::scale::{Scale, ScaleMode};
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Texture, WindowCanvas};

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

    /// A word for the background field to spell out when this happens, if it deserves one:
    /// a Rustris tetris, a Dr. Rustario combo. The words themselves are the engine's, see
    /// [`crate::particles::field::reaction::words`]; a game only says when.
    fn clear_word(&self, event: &GameEvent) -> Option<&'static str> {
        let _ = event;
        None
    }

    /// A short caption to draw over the cells this event cleared, if it deserves one.
    ///
    /// Unlike [`Self::clear_word`], which writes across the whole window in particles and is
    /// for the once-a-match moments, this is small, local and may fire on every clear - a Puyo
    /// chain counting itself up step by step. The game owns the words: they are drawn as text
    /// rather than picked from a list the field knows how to spell.
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
}

/// Where the mascot sits and the piece it holds.
#[derive(Clone, Copy, Debug)]
pub struct MascotLayout {
    /// where the next piece waits in the mascot's hand
    pub hand_point: Point,
    pub spawn_point: Point,
    pub game_over_point: Point,
    pub victory_point: Point,
    /// draw the mascot before the piece in its hand (so the piece overlaps it)
    pub draw_first: bool,
}

/// Where queued pieces are drawn.
#[derive(Clone, Debug)]
pub enum PeekLayout {
    /// a column of pieces starting at `point`, each `offset` further down. With a mascot the
    /// first queued piece is in its hand and the column shows the rest.
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

/// Where the attacks queued against a player are drawn: the strip a game with an answerable
/// attack needs, so a player can see what is hanging over them and decide whether to chain
/// back at it or take it.
///
/// The game says *what* is queued, through [`crate::game::Game::pending_attacks`], as its own
/// [`CellId`]s; the theme says where the icons go and how big, in its background's own source
/// pixels, and they are drawn from that theme's own cell sprites - so a theme owes the strip
/// no art it does not already have. A theme with no `pending` layout draws no strip, which is
/// every theme of a game that takes its hits immediately.
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

/// Which family a theme belongs to: the retro themes rebuild a console's look, the
/// particle themes are the engine's own modern look with a background particle field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeFamily {
    Retro,
    Particle,
}

/// how much of the board's width a popup may take, in sixteenths
const POPUP_MAX_BOARD_WIDTH: u32 = 15;

pub struct Theme<'a> {
    pub(crate) name: &'static str,
    pub(crate) family: ThemeFamily,
    pub(crate) scenes: Vec<SceneRender<'a>>,
    pub(crate) sprites: BlockSpriteSheet<'a>,
    pub(crate) geometry: BoardGeometry,
    pub(crate) audio: AudioTheme,
    pub(crate) font: FontTheme<'a>,
    /// draws [`crate::animate::popup::Popup`]s over the board. Every theme has one, sized to
    /// its own cell, because whether there are any popups at all is the *game*'s decision -
    /// see [`GameRender::clear_popup`] - and a theme should not have to opt into a game's
    /// feedback
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
    pub(crate) animation_meta: AnimationMeta,
    pub(crate) match_end: Option<MatchEndSprites<'a>>,
    /// the cell drawn by a curtain game over
    pub(crate) curtain_cell: Option<CellId>,
    pub(crate) hold: Option<HoldLayout>,
    pub(crate) peek: PeekLayout,
    /// where attacks queued against the player are drawn, for a game that holds them
    pub(crate) pending: Option<PendingLayout>,
    pub(crate) ghost_style: GhostStyle,
    /// themes that emit particles do so in this colour
    pub(crate) particle_color: Option<Color>,
    /// the colours this theme radiates into the background particle field. Empty for a theme
    /// with no field of its own, which then falls back to another theme of the same game
    pub(crate) particle_palette: Vec<Color>,
    /// how this theme's art may be resized to the window
    pub(crate) scale_mode: ScaleMode,
    /// source pixels at the top of the background that nothing is ever drawn into, so they
    /// may fall outside the window rather than cost the board a whole step
    pub(crate) top_slack: u32,
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

    /// the strip of attacks waiting to land, drawn from this theme's own cell sprites
    fn draw_pending<G: Game>(&self, canvas: &mut WindowCanvas, game: &G) -> Result<(), String> {
        let Some(layout) = &self.pending else {
            return Ok(());
        };
        let pending = game.pending_attacks();
        for (dest, id) in layout.slots(pending.len()).into_iter().zip(pending) {
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
        self.draw_pending(canvas, game)?;

        self.font.render_all(canvas, game)
    }

    /// The captions a game asked for over the cells they are about, drifting up off them.
    ///
    /// Drawn straight onto the window rather than into the board texture, **after** the
    /// foreground particles - a caption that a clear's own particle burst is drawn over is a
    /// caption nobody reads, and the burst is exactly what is happening when one appears. That
    /// costs the clipping the board texture used to give it for free, so the caption is held
    /// inside the board's own width here instead.
    ///
    /// `origin` is where the board texture sits in the window and `scale` is what it is drawn
    /// at, so everything below is worked out in the theme's own source pixels and mapped out
    /// at the end. Nothing is drawn at all unless a game returned a caption from
    /// [`GameRender::clear_popup`], which neither Dr. Rustario nor Rustris does.
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
        // a caption wider than the board would run over the HUD, or over the other player,
        // so however long a game's words are they are held to the board
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
            // ... and held inside it, so a caption over the first column is not half cut off
            let width = self.popup_font.width(popup.text(), size) as i32;
            let x = (left + (column * block).round() as i32 + half).clamp(
                left + width / 2,
                left + self.geometry.width() as i32 - width / 2,
            );
            let center = Point::new(x, anchor.y() + half - (popup.rise() * block).round() as i32);
            // the colour this theme draws the cells that popped in, so the caption belongs to
            // the burst rather than floating over it
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
            self.sprites
                .draw_board(canvas, game, &self.geometry, animations, self.ghost_style)?;
        }

        if let (Some(rows), Some(height)) = (
            animations.game_over().curtain_rows(),
            animations.game_over().curtain_height(),
        ) {
            if let Some(cell) = self.curtain_cell {
                // the curtain closes over the board itself, from its floor up: a board
                // drawn with a buffer zone showing above the skyline keeps it clear
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

    /// a queue longer than the strip draws what it has room for rather than running off the
    /// side of the background
    #[test]
    fn the_pending_strip_stops_when_it_runs_out_of_room() {
        assert_eq!(strip().slots(99).len(), 4);
    }

    /// ... and a negative step fills the other way, for a theme whose room is to the left
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
