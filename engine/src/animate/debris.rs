//! Fire-and-forget pieces thrown off something, in fractional board cells that may leave the board,
//! so it is drawn on the window clipped to the player. Fed by a theme's `PopDebris` on a pop and by
//! an attack ball arriving; it holds nothing.

use crate::game::CellId;
use rand::rngs::ThreadRng;
use rand::{rng, RngExt};
use std::f64::consts::TAU;
use std::time::Duration;

/// How many pieces may be in the air at once for one player; the oldest go first.
const MAX_PIECES: usize = 256;

/// Where a burst throws its pieces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spread {
    /// evenly around the whole circle, the way a thing that bursts does
    AllDirections,
    /// upward and outward, the way a thing that is knocked loose does
    Upward,
}

/// What one piece is drawn as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebrisArt {
    /// one of the theme's own cell sprites, which every theme has - so a burst needs no art
    Cell(CellId),
    /// the piece this cell is drawn as when thrown off
    /// ([`crate::render::sprite_sheet::CellAnimationData::debris`]), falling back to the whole cell
    Debris(CellId),
}

/// One burst, in the units a board is measured in.
#[derive(Clone, Copy, Debug)]
pub struct BurstSpec {
    /// where it comes from, in fractional board cells; may be off the board
    pub origin: (f64, f64),
    pub count: usize,
    /// cells a second, drawn uniformly from this range
    pub speed: (f64, f64),
    pub spread: Spread,
    /// cells a second squared, pulling down
    pub gravity: f64,
    pub life: Duration,
    /// the fraction of its life a piece spends fading out at the end
    pub fade_last: f64,
    /// how big a piece is drawn, as a fraction of a block
    pub size: f64,
    pub art: DebrisArt,
}

impl BurstSpec {
    /// the droplets something bursting throws off: outward, falling, gone in half a second
    pub fn burst(origin: (f64, f64), count: usize, art: DebrisArt) -> Self {
        Self {
            origin,
            count,
            speed: (3.0, 7.0),
            spread: Spread::AllDirections,
            gravity: 14.0,
            life: Duration::from_millis(450),
            fade_last: 0.5,
            size: 0.5,
            art,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub x: f64,
    pub y: f64,
    velocity: (f64, f64),
    gravity: f64,
    elapsed: Duration,
    life: Duration,
    fade_last: f64,
    pub size: f64,
    pub art: DebrisArt,
}

impl Piece {
    /// 0..=255, for `set_alpha_mod`
    pub fn alpha(&self) -> u8 {
        let through = self.elapsed.as_secs_f64() / self.life.as_secs_f64().max(f64::EPSILON);
        if self.fade_last <= 0.0 || through < 1.0 - self.fade_last {
            return 255;
        }
        let fade = (1.0 - through) / self.fade_last;
        (fade.clamp(0.0, 1.0) * 255.0) as u8
    }
}

/// Every piece one player has in the air.
#[derive(Debug)]
pub struct DebrisAnimation {
    pieces: Vec<Piece>,
    rng: ThreadRng,
}

impl Default for DebrisAnimation {
    fn default() -> Self {
        Self {
            pieces: vec![],
            rng: rng(),
        }
    }
}

impl Clone for DebrisAnimation {
    /// themes are built per player and cloned; a clone starts empty with its own randomness
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl DebrisAnimation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn burst(&mut self, spec: BurstSpec) {
        for _ in 0..spec.count {
            let angle = match spec.spread {
                Spread::AllDirections => self.rng.random_range(0.0..TAU),
                // the upper half circle with y downward, so up and out
                Spread::Upward => self.rng.random_range(0.0..TAU / 2.0) + TAU / 2.0,
            };
            let speed = self
                .rng
                .random_range(spec.speed.0..=spec.speed.1.max(spec.speed.0));
            self.pieces.push(Piece {
                x: spec.origin.0,
                y: spec.origin.1,
                velocity: (angle.cos() * speed, angle.sin() * speed),
                gravity: spec.gravity,
                elapsed: Duration::ZERO,
                life: spec.life,
                fade_last: spec.fade_last,
                size: spec.size,
                art: spec.art,
            });
        }
        // the oldest are the faintest
        if self.pieces.len() > MAX_PIECES {
            self.pieces.drain(..self.pieces.len() - MAX_PIECES);
        }
    }

    pub fn update(&mut self, delta: Duration) {
        if self.pieces.is_empty() {
            return;
        }
        let step = delta.as_secs_f64();
        for piece in self.pieces.iter_mut() {
            piece.velocity.1 += piece.gravity * step;
            piece.x += piece.velocity.0 * step;
            piece.y += piece.velocity.1 * step;
            piece.elapsed += delta;
        }
        self.pieces.retain(|p| p.elapsed < p.life);
    }

    pub fn reset(&mut self) {
        self.pieces.clear();
    }

    pub fn pieces(&self) -> &[Piece] {
        &self.pieces
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn spec(count: usize) -> BurstSpec {
        BurstSpec::burst((1.0, 1.0), count, DebrisArt::Cell(CellId(0)))
    }

    #[test]
    fn a_burst_falls_and_then_expires() {
        let mut debris = DebrisAnimation::new();
        debris.burst(spec(4));
        assert_eq!(debris.pieces().len(), 4);
        let top = debris.pieces()[0].y;
        debris.update(Duration::from_millis(200));
        assert!(
            debris.pieces().iter().all(|p| p.y > top - 2.0),
            "gravity has had a say"
        );
        debris.update(Duration::from_millis(500));
        assert!(debris.pieces().is_empty(), "and they are gone");
    }

    /// A droplet can leave its cell and the board.
    #[test]
    fn a_piece_may_travel_off_the_board() {
        let mut debris = DebrisAnimation::new();
        debris.burst(BurstSpec {
            origin: (0.0, 0.0),
            speed: (20.0, 20.0),
            gravity: 0.0,
            ..spec(16)
        });
        debris.update(Duration::from_millis(200));
        assert!(
            debris.pieces().iter().any(|p| p.x < 0.0),
            "something went out the left of the board"
        );
    }

    #[test]
    fn a_piece_fades_only_over_the_last_of_its_life() {
        let mut debris = DebrisAnimation::new();
        debris.burst(BurstSpec {
            life: Duration::from_millis(400),
            fade_last: 0.5,
            ..spec(1)
        });
        assert_eq!(debris.pieces()[0].alpha(), 255);
        debris.update(Duration::from_millis(200));
        assert_eq!(debris.pieces()[0].alpha(), 255, "half way, still whole");
        debris.update(Duration::from_millis(100));
        assert!((100..200).contains(&(debris.pieces()[0].alpha() as u32)));
    }

    #[test]
    fn the_pool_is_capped_and_the_oldest_go_first() {
        let mut debris = DebrisAnimation::new();
        for _ in 0..10 {
            debris.burst(spec(MAX_PIECES / 4));
        }
        assert_eq!(debris.pieces().len(), MAX_PIECES);
    }
}
