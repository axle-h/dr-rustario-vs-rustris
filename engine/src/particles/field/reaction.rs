//! What the field does about the match: shockwaves from clears, comets for attacks and a pall
//! over a board that is losing.
//!
//! Every player's events feed the field whatever theme they are on — a retro player's line
//! clear still ripples through the visible modern half, which reads correctly as "something
//! happened over there".

use crate::particles::color::ParticleColor;
use crate::particles::geometry::{RectF, Vec2D};
use crate::particles::particle::Particle;

/// The words the field spells out when the match calls for one. They are the engine's own
/// because they are arcade words rather than either game's vocabulary - a game only says
/// *when*, through [`crate::render::GameRender::clear_word`] - and because the renderer
/// outlines them ahead of time, so one is ready the moment it is called for.
pub mod words {
    pub const TETRIS: &str = "TETRIS";
    pub const COMBO: &str = "COMBO";
    pub const T_SPIN: &str = "T-SPIN";
    pub const PERFECT: &str = "PERFECT";
    pub const GAME_OVER: &str = "GAME OVER";

    pub const ALL: [&str; 5] = [TETRIS, COMBO, T_SPIN, PERFECT, GAME_OVER];
}

/// Something that happened in the match, in the terms the field cares about. Built by the
/// match screen, which is the only place that knows both the events and where the boards are.
#[derive(Clone, Debug, PartialEq)]
pub enum FieldEvent {
    /// `rows` are the cleared rows on screen, so the wave starts where the clear was
    Clear {
        player: u32,
        rows: Vec<RectF>,
        /// the game's own grading of the clear, see `GameRender::clear_class`
        class: u16,
        count: u32,
        is_combo: bool,
    },
    SpeedUp {
        player: u32,
    },
    /// an attack and, unlike `GameEvent::AttackSent`, who it landed on
    Attack {
        from: u32,
        to: u32,
        strength: u32,
    },
    AttackReceived {
        player: u32,
    },
    Victory {
        player: u32,
    },
    StageComplete {
        player: u32,
    },
    GameOver {
        player: u32,
    },
    /// spell one of [`words`] out now, whatever the field had in mind
    Spell {
        word: &'static str,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaveKind {
    /// expanding ring
    Ring,
    /// a band sweeping up and down away from a row
    Horizontal,
}

/// An expanding front that shoves whatever it passes through.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shockwave {
    origin: Vec2D,
    kind: WaveKind,
    radius: f64,
    speed: f64,
    thickness: f64,
    strength: f64,
    elapsed: f64,
    duration: f64,
    color: ParticleColor,
}

impl Shockwave {
    pub fn ring(origin: Vec2D, strength: f64, color: ParticleColor) -> Self {
        Self {
            origin,
            kind: WaveKind::Ring,
            radius: 0.0,
            speed: 0.9,
            thickness: 0.09,
            strength,
            elapsed: 0.0,
            duration: 1.1,
            color,
        }
    }

    pub fn horizontal(origin: Vec2D, strength: f64, color: ParticleColor) -> Self {
        Self {
            origin,
            kind: WaveKind::Horizontal,
            radius: 0.0,
            speed: 0.75,
            thickness: 0.07,
            strength,
            elapsed: 0.0,
            duration: 0.9,
            color,
        }
    }

    pub fn with_speed(mut self, speed: f64) -> Self {
        self.speed = speed;
        self
    }

    pub fn is_spent(&self) -> bool {
        self.elapsed >= self.duration
    }

    pub fn update(&mut self, delta_time: f64) {
        self.elapsed += delta_time;
        self.radius += self.speed * delta_time;
    }

    /// how much of the wave is left, for the colour flash it carries
    fn fade(&self) -> f64 {
        (1.0 - self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    /// push a particle if the front is passing it, and return how strongly it was hit so the
    /// colour flash can follow
    pub fn apply(&self, particle: &mut Particle, delta_time: f64) -> f64 {
        let delta = particle.position() - self.origin;
        let (distance, direction) = match self.kind {
            WaveKind::Ring => (delta.magnitude(), delta),
            WaveKind::Horizontal => (
                delta.y().abs(),
                // mostly vertical, with a little outward spread so the row does not become a
                // pair of flat sheets
                Vec2D::new(delta.x() * 0.35, delta.y()),
            ),
        };
        let offset = (distance - self.radius).abs();
        if offset > self.thickness {
            return 0.0;
        }
        let hit = (1.0 - offset / self.thickness) * self.fade();
        let unit = if direction.is_zero() {
            Vec2D::new(0.0, -1.0)
        } else {
            direction.unit_vector()
        };
        particle.add_velocity(unit * (self.strength * hit * delta_time * 6.0));
        hit
    }

    pub fn color(&self) -> ParticleColor {
        self.color
    }
}

/// A trail of particles flying from one board to another. The victim is resolved against the
/// canvas, so an attack aimed at a player the field never draws over still leaves properly.
#[derive(Clone, Debug, PartialEq)]
pub struct Comet {
    from: Vec2D,
    to: Vec2D,
    /// how far the arc bows away from the straight line
    bow: f64,
    elapsed: f64,
    duration: f64,
    color: ParticleColor,
    /// the particles it has conscripted out of the ambient field
    members: Vec<usize>,
    /// where each member sits along the trail, 0 at the head
    offsets: Vec<f64>,
    /// true once the arrival burst has been spawned
    arrived: bool,
}

impl Comet {
    pub fn new(
        from: Vec2D,
        to: Vec2D,
        strength: u32,
        color: ParticleColor,
        members: Vec<usize>,
    ) -> Self {
        let count = members.len().max(1);
        let offsets = (0..members.len())
            .map(|i| i as f64 / count as f64 * 0.35)
            .collect();
        Self {
            from,
            to,
            // a bigger attack takes a wider arc
            bow: 0.08 + 0.02 * strength.min(6) as f64,
            elapsed: 0.0,
            duration: 0.75,
            color,
            members,
            offsets,
            arrived: false,
        }
    }

    pub fn members(&self) -> &[usize] {
        &self.members
    }

    pub fn color(&self) -> ParticleColor {
        self.color
    }

    pub fn target(&self) -> Vec2D {
        self.to
    }

    pub fn is_spent(&self) -> bool {
        self.elapsed >= self.duration
    }

    /// true on the frame the head lands, so the caller can burst
    pub fn update(&mut self, delta_time: f64) -> bool {
        self.elapsed += delta_time;
        if self.elapsed >= self.duration && !self.arrived {
            self.arrived = true;
            return true;
        }
        false
    }

    /// where the `index`th member should be right now
    pub fn position_of(&self, index: usize) -> Vec2D {
        let t = (self.elapsed / self.duration - self.offsets[index]).clamp(0.0, 1.0);
        self.point_at(t)
    }

    fn point_at(&self, t: f64) -> Vec2D {
        let straight = self.from + (self.to - self.from) * t;
        // a quadratic bow perpendicular to the flight, zero at both ends
        let perpendicular = (self.to - self.from).perpendicular().unit_vector();
        straight + perpendicular * (self.bow * 4.0 * t * (1.0 - t))
    }
}

/// A weight settling over one part of the canvas: what an attack lands on its victim, and what
/// the losing half of the screen gets at the end of a match.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pall {
    region: RectF,
    elapsed: f64,
    duration: f64,
    /// pressed downward
    gravity: f64,
    /// 0-1 of the colour drained out
    drain: f64,
}

impl Pall {
    pub fn new(region: RectF, duration: f64, gravity: f64, drain: f64) -> Self {
        Self {
            region,
            elapsed: 0.0,
            duration,
            gravity,
            drain,
        }
    }

    pub fn is_spent(&self) -> bool {
        self.elapsed >= self.duration
    }

    pub fn update(&mut self, delta_time: f64) {
        self.elapsed += delta_time;
    }

    fn weight(&self) -> f64 {
        (1.0 - self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    /// press the particle down and return how much of its colour to drain
    pub fn apply(&self, particle: &mut Particle, delta_time: f64) -> f64 {
        if !self.region.contains(particle.position()) {
            return 0.0;
        }
        let weight = self.weight();
        particle.add_velocity(Vec2D::new(0.0, self.gravity * weight * delta_time));
        self.drain * weight
    }
}

/// One end of an attack: where that player's board is, and whether the field draws over it.
/// A board outside the canvas is still known — that is what makes the half-visible cases
/// answerable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackEnd {
    pub board: RectF,
    pub in_canvas: bool,
}

impl AttackEnd {
    pub fn new(board: RectF, in_canvas: bool) -> Self {
        Self { board, in_canvas }
    }
}

/// Where an attack's comet starts and ends, resolved against the canvas. A board outside it
/// is replaced by the canvas edge on that board's side, so the comet leaves or enters rather
/// than being drawn over a half that is not ours.
pub fn attack_endpoints(
    canvas: RectF,
    attacker: AttackEnd,
    victim: AttackEnd,
) -> Option<(Vec2D, Vec2D)> {
    match (attacker.in_canvas, victim.in_canvas) {
        // both boards are ours: the full comet
        (true, true) => Some((attacker.board.center(), victim.board.center())),
        // the attacker is ours; the comet leaves the canvas on the victim's side
        (true, false) => Some((attacker.board.center(), edge_toward(canvas, victim.board))),
        // the victim is ours; the comet enters from the attacker's side
        (false, true) => Some((edge_toward(canvas, attacker.board), victim.board.center())),
        // neither is ours; there is no field over either of them and nothing to draw
        (false, false) => None,
    }
}

/// the point on the canvas edge nearest `board`, at the board's own height. Player clips are
/// vertical slices, so a board outside the canvas is always to one side of it.
fn edge_toward(canvas: RectF, board: RectF) -> Vec2D {
    let centre = board.center();
    let x = if centre.x() < canvas.center().x() {
        canvas.x()
    } else {
        canvas.right()
    };
    Vec2D::new(x, centre.y().clamp(canvas.y(), canvas.bottom()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas() -> RectF {
        RectF::new(0.0, 0.0, 1.0, 1.0)
    }

    fn left_board() -> RectF {
        RectF::new(0.15, 0.3, 0.2, 0.5)
    }

    fn right_board() -> RectF {
        RectF::new(0.65, 0.3, 0.2, 0.5)
    }

    #[test]
    fn both_boards_in_the_canvas_is_a_board_to_board_comet() {
        let (from, to) = attack_endpoints(
            canvas(),
            AttackEnd::new(left_board(), true),
            AttackEnd::new(right_board(), true),
        )
        .unwrap();
        assert_eq!(from, left_board().center());
        assert_eq!(to, right_board().center());
    }

    #[test]
    fn an_attack_on_a_player_we_do_not_draw_leaves_on_their_side() {
        // only the left half is ours; the victim's board is off to the right
        let half = RectF::new(0.0, 0.0, 0.5, 1.0);
        let (from, to) = attack_endpoints(
            half,
            AttackEnd::new(left_board(), true),
            AttackEnd::new(right_board(), false),
        )
        .unwrap();
        assert_eq!(from, left_board().center());
        assert_eq!(to.x(), half.right());
        assert_eq!(to.y(), right_board().center().y());
    }

    #[test]
    fn an_attack_from_a_player_we_do_not_draw_enters_from_their_side() {
        // only the right half is ours; the attacker's board is off to the left
        let half = RectF::new(0.5, 0.0, 0.5, 1.0);
        let (from, to) = attack_endpoints(
            half,
            AttackEnd::new(left_board(), false),
            AttackEnd::new(right_board(), true),
        )
        .unwrap();
        assert_eq!(from.x(), half.x());
        assert_eq!(from.y(), left_board().center().y());
        assert_eq!(to, right_board().center());
    }

    #[test]
    fn an_attack_between_two_players_we_do_not_draw_is_nothing_at_all() {
        assert!(attack_endpoints(
            canvas(),
            AttackEnd::new(left_board(), false),
            AttackEnd::new(right_board(), false)
        )
        .is_none());
    }

    #[test]
    fn a_shockwave_only_pushes_what_its_front_is_passing() {
        use crate::particles::meta::ParticleSprite;
        let build = |x: f64| {
            Particle::new(
                Vec2D::new(x, 0.5),
                Vec2D::ZERO,
                Vec2D::ZERO,
                1.0,
                1.0,
                None,
                ParticleColor::WHITE,
                None,
                ParticleSprite::Circle05,
                1.0,
                0.0,
            )
        };
        let mut wave = Shockwave::ring(Vec2D::new(0.5, 0.5), 1.0, ParticleColor::WHITE);
        wave.update(0.3); // radius 0.27

        let mut on_the_front = build(0.77);
        assert!(wave.apply(&mut on_the_front, 0.016) > 0.0);
        assert!(on_the_front.velocity().x() > 0.0);

        let mut far_ahead = build(0.99);
        assert_eq!(wave.apply(&mut far_ahead, 0.016), 0.0);
        assert_eq!(far_ahead.velocity(), Vec2D::ZERO);
    }
}
