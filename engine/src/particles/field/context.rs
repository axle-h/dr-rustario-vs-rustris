//! What the field is told about the match each frame.
//!
//! The field never reads game state directly and never writes it: everything it reacts to
//! arrives here, or as a queued event.

use crate::game::GameId;
use crate::particles::color::ParticleColor;
use crate::particles::geometry::{RectF, Vec2D};
use sdl2::pixels::Color;

/// how much of each palette segment is spent blending into the next
const BLEND: f64 = 0.3;

/// The colours one game radiates into the field.
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    colors: Vec<ParticleColor>,
}

impl Palette {
    pub fn new(colors: Vec<ParticleColor>) -> Self {
        Self {
            colors: if colors.is_empty() {
                vec![ParticleColor::WHITE]
            } else {
                colors
            },
        }
    }

    pub fn from_sdl(colors: &[Color]) -> Self {
        Self::new(
            colors
                .iter()
                .copied()
                .map(ParticleColor::from_sdl)
                .collect(),
        )
    }

    pub fn colors(&self) -> &[ParticleColor] {
        &self.colors
    }

    pub fn len(&self) -> usize {
        self.colors.len()
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn nth(&self, index: usize) -> ParticleColor {
        self.colors[index % self.colors.len()]
    }

    /// a colour from anywhere around the ring. Most of each segment is the palette colour
    /// itself and only the tail of it blends into the next, so the field reads as the theme's
    /// own colours with soft transitions - rather than as every hue between them, which is
    /// what an even blend of three or seven well spaced colours comes out as.
    pub fn pick(&self, t: f64) -> ParticleColor {
        let len = self.colors.len();
        if len == 1 {
            return self.colors[0];
        }
        let t = t.rem_euclid(1.0) * len as f64;
        let index = t.floor() as usize % len;
        let blend = ((t.fract() - (1.0 - BLEND)) / BLEND).clamp(0.0, 1.0);
        if blend <= 0.0 {
            return self.colors[index];
        }
        // smoothstep, so a particle does not visibly step as the phase carries it across
        let blend = blend * blend * (3.0 - 2.0 * blend);
        self.colors[index].lerp_hue(self.colors[(index + 1) % len], blend)
    }

    /// every colour rotated the same way round the hue circle
    pub fn shift_hue(&self, degrees: f64) -> Self {
        Self::new(self.colors.iter().map(|c| c.shift_hue(degrees)).collect())
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::new(vec![ParticleColor::WHITE])
    }
}

/// One player as the field sees them.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerRegion {
    pub player: u32,
    /// this player's vertical slice of the window, particle space
    pub clip: RectF,
    /// their playfield, particle space. Known even for a player the field never draws over,
    /// which is what makes a half-visible attack resolvable
    pub board: RectF,
    /// indexes the particle renderer's theme sprites
    pub theme: usize,
    pub game: GameId,
    pub palette: Palette,
    /// false for a player on a retro theme: nothing is ever drawn over their half
    pub in_canvas: bool,
    /// how close to the top their stack is, 0-1. Read per frame; there is no danger event
    pub danger: f64,
    pub speed_index: u32,
    /// their board is not being played right now (a stage card, a game over)
    pub held_up: bool,
}

/// The state of the match handed to the field each frame.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneContext {
    /// the union of the clips of the players on a particle scene. Because player clips are
    /// vertical slices that tile the window this is always one contiguous rect: the whole
    /// window, the left half or the right half
    pub canvas: RectF,
    pub players: Vec<PlayerRegion>,
    /// the union of the games being played, which picks the sprite set and the palette
    pub games: Vec<GameId>,
    /// short strings the field may morph into, e.g. "LEVEL 8". Built by the match screen,
    /// which is the only place that knows what a game's numbers mean.
    pub captions: Vec<String>,
}

impl SceneContext {
    /// `None` when no player is on a particle scene, in which case there is no field at all
    pub fn new(players: Vec<PlayerRegion>) -> Option<Self> {
        Self::with_captions(players, vec![])
    }

    pub fn with_captions(players: Vec<PlayerRegion>, captions: Vec<String>) -> Option<Self> {
        let canvas = players
            .iter()
            .filter(|p| p.in_canvas)
            .map(|p| p.clip)
            .reduce(|a, b| a.union(&b))?;
        let mut games: Vec<GameId> = players.iter().map(|p| p.game).collect();
        games.dedup();
        Some(Self {
            canvas,
            players,
            games,
            captions,
        })
    }

    pub fn regions(&self) -> impl Iterator<Item = &PlayerRegion> {
        self.players.iter()
    }

    /// the players whose half the field is drawn over
    pub fn visible(&self) -> impl Iterator<Item = &PlayerRegion> {
        self.players.iter().filter(|p| p.in_canvas)
    }

    pub fn region(&self, player: u32) -> Option<&PlayerRegion> {
        self.players.iter().find(|p| p.player == player)
    }

    /// a point given 0-1 across the canvas, in particle space. Routines are authored in
    /// canvas-normalised coordinates so one written once fits the whole window or half of it
    pub fn from_canvas<P: Into<Vec2D>>(&self, point: P) -> Vec2D {
        self.canvas.denormalise(point)
    }

    /// the highest danger of any player, which is what drives the whole field's agitation
    pub fn danger(&self) -> f64 {
        self.players
            .iter()
            .map(|p| p.danger)
            .fold(0.0, |a: f64, b| a.max(b))
    }

    /// every player is held up (a stage card, a game over): the field idles
    pub fn all_held_up(&self) -> bool {
        self.players.iter().all(|p| p.held_up)
    }

    /// the palettes of the players in the canvas, weighted by how close `point` is to each of
    /// their boards. In two players the middle is a contested gradient that shifts as one
    /// player pressures the other.
    pub fn radiated(&self, point: Vec2D, t: f64) -> ParticleColor {
        let mut result: Option<(ParticleColor, f64)> = None;
        for region in self.visible() {
            let distance = (point - region.board.center()).magnitude().max(0.05);
            let weight = 1.0 / (distance * distance);
            let color = region.palette.pick(t);
            result = Some(match result {
                None => (color, weight),
                // blended round the hue circle rather than averaged in rgb: averaging one
                // game's red with another's cyan comes out grey, and a grey particle is a
                // dead one
                Some((current, total)) => (
                    current.lerp_hue(color, weight / (total + weight)),
                    total + weight,
                ),
            });
        }
        result
            .map(|(color, _)| color)
            .unwrap_or(ParticleColor::WHITE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(player: u32, clip: RectF, in_canvas: bool) -> PlayerRegion {
        PlayerRegion {
            player,
            clip,
            board: clip,
            theme: 0,
            game: GameId(1),
            palette: Palette::default(),
            in_canvas,
            danger: 0.0,
            speed_index: 0,
            held_up: false,
        }
    }

    fn left() -> RectF {
        RectF::new(0.0, 0.0, 0.5, 1.0)
    }
    fn right() -> RectF {
        RectF::new(0.5, 0.0, 0.5, 1.0)
    }
    fn whole() -> RectF {
        RectF::new(0.0, 0.0, 1.0, 1.0)
    }

    #[test]
    fn one_modern_player_owns_the_whole_window() {
        let ctx = SceneContext::new(vec![region(0, whole(), true)]).unwrap();
        assert_eq!(ctx.canvas, whole());
    }

    #[test]
    fn two_modern_players_share_one_contiguous_canvas() {
        let ctx =
            SceneContext::new(vec![region(0, left(), true), region(1, right(), true)]).unwrap();
        assert_eq!(ctx.canvas, whole());
    }

    #[test]
    fn a_retro_player_is_not_drawn_over() {
        let ctx =
            SceneContext::new(vec![region(0, left(), true), region(1, right(), false)]).unwrap();
        assert_eq!(ctx.canvas, left());

        let ctx =
            SceneContext::new(vec![region(0, left(), false), region(1, right(), true)]).unwrap();
        assert_eq!(ctx.canvas, right());
    }

    #[test]
    fn two_retro_players_have_no_field_at_all() {
        assert!(
            SceneContext::new(vec![region(0, left(), false), region(1, right(), false)]).is_none()
        );
    }

    #[test]
    fn a_routine_written_in_canvas_space_fits_whatever_it_is_given() {
        let whole = SceneContext::new(vec![region(0, whole(), true)]).unwrap();
        let half =
            SceneContext::new(vec![region(0, left(), false), region(1, right(), true)]).unwrap();
        // the same authored point lands in the middle of each
        assert_eq!(whole.from_canvas((0.5, 0.5)), Vec2D::new(0.5, 0.5));
        assert_eq!(half.from_canvas((0.5, 0.5)), Vec2D::new(0.75, 0.5));
    }

    #[test]
    fn a_palette_of_one_colour_picks_it_everywhere() {
        let palette = Palette::new(vec![ParticleColor::rgb(1.0, 0.0, 0.0)]);
        assert_eq!(palette.pick(0.0), palette.pick(0.73));
    }

    #[test]
    fn a_palette_blends_between_its_colours() {
        let red = ParticleColor::rgb(1.0, 0.0, 0.0);
        let blue = ParticleColor::rgb(0.0, 0.0, 1.0);
        let palette = Palette::new(vec![red, blue]);
        assert_eq!(palette.pick(0.0), red);
        assert_eq!(palette.pick(0.5), blue);
        // and wraps back round
        assert_eq!(palette.pick(1.0), palette.pick(0.0));
        // in between is a blend, not one or the other
        let between = palette.pick(0.5 - 0.5 * BLEND / 2.0);
        assert_ne!(between, red);
        assert_ne!(between, blue);
    }

    #[test]
    fn most_of_a_palette_is_the_theme_colours_themselves() {
        let colors = vec![
            ParticleColor::rgb(1.0, 0.0, 0.0),
            ParticleColor::rgb(0.0, 0.0, 1.0),
            ParticleColor::rgb(1.0, 0.8, 0.0),
        ];
        let palette = Palette::new(colors.clone());
        let steps = 1000;
        let pure = (0..steps)
            .map(|i| palette.pick(i as f64 / steps as f64))
            .filter(|c| colors.contains(c))
            .count();
        // the blend is a third of each segment, so two thirds should be untouched
        assert!(pure as f64 / steps as f64 > 0.65, "{pure} of {steps}");
    }
}
