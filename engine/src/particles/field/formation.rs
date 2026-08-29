//! Where a feature routine wants its particles.
//!
//! Formations are authored in **canvas-normalised** coordinates — 0-1 across the canvas — and
//! the field maps them into particle space on the way out. A routine written once therefore
//! fits whether it owns the whole window or one half of it, with no special cases. `aspect`
//! is the canvas's own width over height in real pixels, so a circle stays a circle and a
//! silhouette is never stretched.

use crate::particles::field::shapes::EdgeShape;
use crate::particles::field::FieldRng;
use crate::particles::geometry::{RectF, Vec2D};
use rand::RngExt;

/// how many placements a formation tries before settling for the least bad one. This runs
/// once when a feature starts, so it can afford to be thorough.
const PLACEMENT_TRIES: usize = 24;

/// how much of `board` a shape centred at `centre` with these half extents covers
fn overlap(board: &RectF, centre: Vec2D, half_x: f64, half_y: f64) -> f64 {
    let width =
        (board.right().min(centre.x() + half_x) - board.x().max(centre.x() - half_x)).max(0.0);
    let height =
        (board.bottom().min(centre.y() + half_y) - board.y().max(centre.y() - half_y)).max(0.0);
    width * height
}

/// A silhouette drifting and tumbling as it holds, plus the grids and ribbons the other
/// feature routines snap to.
#[derive(Clone, Debug, PartialEq)]
pub enum Formation {
    /// the sprite edge morph: an outline placed in the canvas, slowly turning and breathing
    Sprite {
        points: Vec<Vec2D>,
        centre: Vec2D,
        size: f64,
        /// radians per second
        spin: f64,
        drift: Vec2D,
        /// how much room the placed outline takes either side of its centre, canvas
        /// normalised: what the drift turns round at, so a long hold never walks it off
        half: Vec2D,
    },
    /// a rectilinear grid that breathes, then shears and collapses. Structure with no masks
    /// at all
    Lattice {
        cols: usize,
        rows: usize,
        inset: f64,
    },
    /// a waveform across the canvas, its amplitude driven by how energetic the field is
    Ribbon {
        amplitude: f64,
        frequency: f64,
        phase_rate: f64,
    },
    /// concentric rings about a single centre, drifting across the canvas. One set of rings
    /// and not one per board: a circle behind each player reads as decoration bolted to the
    /// boards, where one that wanders reads as something the field itself is doing
    Haloes {
        centre: Vec2D,
        /// canvas-normalised, outermost first
        radii: Vec<f64>,
        spin: f64,
        /// canvas widths per second, reflected off the edges as it goes
        drift: Vec2D,
    },
    /// arms winding out of a centre, turning as they hold
    Spiral {
        centre: Vec2D,
        arms: usize,
        /// how many times round an arm goes
        turns: f64,
        radius: f64,
        spin: f64,
    },
    /// the figure two perpendicular sines trace: the oscilloscope's other trace, closed
    Lissajous {
        centre: Vec2D,
        amplitude: Vec2D,
        /// the two frequencies, which is what decides which figure it is
        ratio: (f64, f64),
        phase_rate: f64,
    },
    /// the routine steers with fields and velocities instead of targets
    Free,
}

impl Formation {
    /// a silhouette placed somewhere sensible in the canvas: never so big it runs off the
    /// edges, never so small it reads as noise
    /// `boards` are the players' playfields in canvas-normalised coordinates: a silhouette
    /// gathers away from them where it can, so it is not drawn over the one thing the player
    /// is reading
    pub fn sprite(shape: &EdgeShape, aspect: f64, rng: &mut FieldRng, boards: &[RectF]) -> Self {
        Self::sprite_sized(
            shape,
            aspect,
            rng,
            boards,
            (0.38, 0.58),
            (-0.5, 0.5),
            (-0.05, 0.05),
        )
    }

    /// text is wide and thin, so it wants a good part of the canvas, and it should sit still
    /// enough to be read. Not the whole of it: a word stretched wall to wall is one the eye
    /// has to track rather than take in, and its letters end up further apart than the
    /// particles that spell them
    pub fn text(shape: &EdgeShape, aspect: f64, rng: &mut FieldRng, boards: &[RectF]) -> Self {
        Self::sprite_sized(
            shape,
            aspect,
            rng,
            boards,
            (0.5, 0.62),
            (0.0, 0.0),
            (-0.008, 0.008),
        )
    }

    /// `fill` is the fraction of the canvas the shape spans on its larger side, measured from
    /// the outline's own extents so a wide shape is wide rather than merely tall enough
    #[allow(clippy::too_many_arguments)]
    fn sprite_sized(
        shape: &EdgeShape,
        aspect: f64,
        rng: &mut FieldRng,
        boards: &[RectF],
        (fill_min, fill_max): (f64, f64),
        spin: (f64, f64),
        drift: (f64, f64),
    ) -> Self {
        let aspect = aspect.max(0.01);
        // a shape that turns as it holds is placed by the circle it turns inside, not by the
        // box it happens to fill at the moment it is made: a tetromino lying on its side is
        // half off the canvas by the time it has swung upright otherwise
        let (bound_x, bound_y) = if spin.0 == 0.0 && spin.1 == 0.0 {
            shape.extents()
        } else {
            (shape.radius(), shape.radius())
        };
        let fill = fill_min + (fill_max - fill_min) * rng.random::<f64>();
        // the largest scale that keeps the outline within `fill` of the canvas either way
        let size = [
            (bound_x > 0.0).then(|| fill * aspect / (2.0 * bound_x)),
            (bound_y > 0.0).then(|| fill / (2.0 * bound_y)),
        ]
        .into_iter()
        .flatten()
        .fold(f64::MAX, f64::min);
        let size = if size.is_finite() { size } else { fill };

        let half_x = bound_x * size / aspect;
        let half_y = bound_y * size;
        // room for the drift and the breathe on top of the outline itself, so a formation
        // that is still moving when the hold ends has not walked off the canvas by then
        let margin_x = (half_x * 1.05 + 0.05).min(0.49);
        let margin_y = (half_y * 1.05 + 0.05).min(0.49);
        // a handful of candidate placements, keeping whichever covers the least board. Text
        // spans most of the canvas and cannot avoid them, so this is a preference, not a rule
        let mut best: Option<(Vec2D, f64)> = None;
        for _ in 0..PLACEMENT_TRIES {
            let centre = Vec2D::new(
                margin_x + (1.0 - 2.0 * margin_x) * rng.random::<f64>(),
                margin_y + (1.0 - 2.0 * margin_y) * rng.random::<f64>(),
            );
            let covered = boards
                .iter()
                .map(|board| overlap(board, centre, half_x, half_y))
                .sum::<f64>();
            if covered <= 0.0 {
                best = Some((centre, 0.0));
                break;
            }
            if best.is_none_or(|(_, worst)| covered < worst) {
                best = Some((centre, covered));
            }
        }
        Self::Sprite {
            points: shape.points().to_vec(),
            centre: best
                .map(|(centre, _)| centre)
                .unwrap_or(Vec2D::new(0.5, 0.5)),
            size,
            half: Vec2D::new(margin_x, margin_y),
            spin: spin.0 + (spin.1 - spin.0) * rng.random::<f64>(),
            drift: Vec2D::new(
                drift.0 + (drift.1 - drift.0) * rng.random::<f64>(),
                drift.0 + (drift.1 - drift.0) * rng.random::<f64>(),
            ),
        }
    }

    pub fn lattice(rng: &mut FieldRng) -> Self {
        let cols = 8 + rng.random_range(0..7);
        Self::Lattice {
            cols,
            rows: (cols * 2 / 3).max(4),
            inset: 0.08,
        }
    }

    pub fn ribbon(rng: &mut FieldRng) -> Self {
        Self::Ribbon {
            amplitude: 0.12 + 0.1 * rng.random::<f64>(),
            frequency: 2.0 + 4.0 * rng.random::<f64>(),
            phase_rate: 1.2 + rng.random::<f64>(),
        }
    }

    /// three or four rings about a point somewhere in the middle of the canvas, set drifting
    pub fn haloes(rng: &mut FieldRng, aspect: f64) -> Self {
        let rings = 3 + rng.random_range(0..2);
        // the rings wander, so they are sized against the tightest they will ever be boxed in
        let outer = (0.3 + 0.1 * rng.random::<f64>()).min(0.45 * aspect.max(0.01));
        Self::Haloes {
            centre: Vec2D::new(
                0.35 + 0.3 * rng.random::<f64>(),
                0.35 + 0.3 * rng.random::<f64>(),
            ),
            // evenly spaced from a quarter of the outer radius out to it
            radii: (0..rings)
                .map(|ring| outer * (0.25 + 0.75 * ring as f64 / (rings - 1).max(1) as f64))
                .rev()
                .collect(),
            spin: 0.35 + 0.35 * rng.random::<f64>(),
            drift: Vec2D::new(Self::signed(rng, 0.02, 0.06), Self::signed(rng, 0.02, 0.06)),
        }
    }

    pub fn spiral(rng: &mut FieldRng, aspect: f64) -> Self {
        let centre = Vec2D::new(
            0.4 + 0.2 * rng.random::<f64>(),
            0.4 + 0.2 * rng.random::<f64>(),
        );
        Self::Spiral {
            centre,
            arms: 1 + rng.random_range(0..3),
            turns: 1.5 + 1.5 * rng.random::<f64>(),
            // as wide as it likes, up to whatever room the centre it was given leaves it
            radius: (0.34 + 0.1 * rng.random::<f64>()).min(Self::head_room(centre, aspect) / 1.07),
            spin: Self::signed(rng, 0.25, 0.6),
        }
    }

    /// how far a circle about `centre` can reach before it leaves the canvas, allowing for
    /// the canvas being wider than it is tall
    fn head_room(centre: Vec2D, aspect: f64) -> f64 {
        let across = centre.x().min(1.0 - centre.x()) * aspect.max(0.01);
        across.min(centre.y().min(1.0 - centre.y())).max(0.0)
    }

    pub fn lissajous(rng: &mut FieldRng, aspect: f64) -> Self {
        // the small integer ratios are the ones that close into a figure rather than a
        // scribble; anything larger is too busy to read at this size
        const RATIOS: [(f64, f64); 6] = [
            (1.0, 2.0),
            (2.0, 3.0),
            (3.0, 4.0),
            (3.0, 2.0),
            (4.0, 5.0),
            (5.0, 4.0),
        ];
        let centre = Vec2D::new(0.5, 0.5);
        // it swells with the field's energy, so it is sized for its widest
        let widest = Self::head_room(centre, aspect) / 1.16;
        Self::Lissajous {
            centre,
            amplitude: Vec2D::new(
                (0.3 + 0.08 * rng.random::<f64>()).min(widest),
                (0.3 + 0.08 * rng.random::<f64>()).min(widest),
            ),
            ratio: RATIOS[rng.random_range(0..RATIOS.len())],
            phase_rate: Self::signed(rng, 0.15, 0.4),
        }
    }

    /// a magnitude between `min` and `max`, either way round
    fn signed(rng: &mut FieldRng, min: f64, max: f64) -> f64 {
        let magnitude = min + (max - min) * rng.random::<f64>();
        if rng.random::<bool>() {
            magnitude
        } else {
            -magnitude
        }
    }

    /// where something drifting at `rate` from `start` has got to, reflected back off `min`
    /// and `max` rather than sailing off the canvas
    fn wander(start: f64, rate: f64, elapsed: f64, min: f64, max: f64) -> f64 {
        let span = max - min;
        if span <= 0.0 {
            return min;
        }
        let travelled = (start - min + rate * elapsed).rem_euclid(2.0 * span);
        min + if travelled <= span {
            travelled
        } else {
            2.0 * span - travelled
        }
    }

    /// how many particles this formation can usefully hold; `None` means as many as there are
    pub fn capacity(&self) -> Option<usize> {
        match self {
            Formation::Sprite { points, .. } => Some(points.len()),
            Formation::Lattice { cols, rows, .. } => Some(cols * rows),
            Formation::Ribbon { .. }
            | Formation::Haloes { .. }
            | Formation::Spiral { .. }
            | Formation::Lissajous { .. }
            | Formation::Free => None,
        }
    }

    /// where the `index`th of `count` members belongs, canvas-normalised. `energy` swells the
    /// routines that react to how hard the match is being played.
    pub fn target(
        &self,
        index: usize,
        count: usize,
        elapsed: f64,
        aspect: f64,
        energy: f64,
    ) -> Option<Vec2D> {
        let count = count.max(1);
        match self {
            Formation::Free => None,
            Formation::Sprite {
                points,
                centre,
                size,
                spin,
                drift,
                half,
            } => {
                if points.is_empty() {
                    return None;
                }
                // a cast smaller than the outline is spread evenly over the whole of it,
                // rather than filling a prefix and leaving the rest of the shape missing
                let point = if count >= points.len() {
                    points[index % points.len()]
                } else {
                    points[index * points.len() / count]
                };
                // it should move as it holds, not sit still
                let breathe = 1.0 + 0.03 * (elapsed * 1.7).sin();
                let angle = spin * elapsed;
                let (sin, cos) = angle.sin_cos();
                let rotated = Vec2D::new(
                    point.x() * cos - point.y() * sin,
                    point.x() * sin + point.y() * cos,
                );
                let scaled = Vec2D::new(
                    rotated.x() * size * breathe / aspect.max(0.01),
                    rotated.y() * size * breathe,
                );
                let centre = Vec2D::new(
                    Self::wander(centre.x(), drift.x(), elapsed, half.x(), 1.0 - half.x()),
                    Self::wander(centre.y(), drift.y(), elapsed, half.y(), 1.0 - half.y()),
                );
                Some(centre + scaled)
            }
            Formation::Lattice { cols, rows, inset } => {
                let (cols, rows) = (*cols, *rows);
                let cell = index % (cols * rows);
                let (col, row) = (cell % cols, cell / cols);
                let span = 1.0 - 2.0 * inset;
                // breathe, then shear over as it comes apart
                let breathe = 1.0 + 0.05 * (elapsed * 1.3).sin();
                let shear = (elapsed * 0.35).powi(2) * 0.25;
                let x = inset + span * col as f64 / (cols.max(2) - 1) as f64;
                let y = inset + span * row as f64 / (rows.max(2) - 1) as f64;
                let centred = Vec2D::new(x - 0.5, y - 0.5) * breathe;
                Some(Vec2D::new(
                    0.5 + centred.x() + centred.y() * shear,
                    0.5 + centred.y(),
                ))
            }
            Formation::Ribbon {
                amplitude,
                frequency,
                phase_rate,
            } => {
                let x = index as f64 / (count - 1).max(1) as f64;
                let amplitude = amplitude * (0.7 + 0.6 * energy);
                let y = 0.5
                    + amplitude * (frequency * x * std::f64::consts::TAU + elapsed * phase_rate).sin()
                    // a slower second harmonic so it is a waveform, not a plain sine
                    + amplitude * 0.35 * (frequency * 0.5 * x * std::f64::consts::TAU - elapsed).sin();
                Some(Vec2D::new(x, y))
            }
            Formation::Haloes {
                centre,
                radii,
                spin,
                drift,
            } => {
                if radii.is_empty() {
                    return None;
                }
                let radius = radii[index % radii.len()];
                let ring_members = (count as f64 / radii.len() as f64).max(1.0);
                let step = (index / radii.len()) as f64 / ring_members;
                // tighter and faster as the field grows agitated
                let radius = radius * (1.0 - 0.25 * energy);
                // the inner rings turn faster, the way an orrery does
                let rate = spin * (1.0 + 0.6 * (radii.len() - 1 - index % radii.len()) as f64);
                let angle = step * std::f64::consts::TAU + elapsed * rate;
                let (sin, cos) = angle.sin_cos();
                let half_x = radii[0] / aspect.max(0.01);
                let half_y = radii[0];
                let centre = Vec2D::new(
                    Self::wander(centre.x(), drift.x(), elapsed, half_x, 1.0 - half_x),
                    Self::wander(centre.y(), drift.y(), elapsed, half_y, 1.0 - half_y),
                );
                Some(Vec2D::new(
                    centre.x() + cos * radius / aspect.max(0.01),
                    centre.y() + sin * radius,
                ))
            }
            Formation::Spiral {
                centre,
                arms,
                turns,
                radius,
                spin,
            } => {
                let arms = (*arms).max(1);
                let arm = index % arms;
                let per_arm = (count as f64 / arms as f64).max(1.0);
                let along = ((index / arms) as f64 / per_arm).min(1.0);
                let angle = along * turns * std::f64::consts::TAU
                    + arm as f64 * std::f64::consts::TAU / arms as f64
                    + elapsed * spin;
                // it winds outward as it holds, so the arms are never a still photograph
                let radius = radius * (0.12 + 0.88 * along) * (1.0 + 0.06 * (elapsed * 0.8).sin());
                let (sin, cos) = angle.sin_cos();
                Some(Vec2D::new(
                    centre.x() + cos * radius / aspect.max(0.01),
                    centre.y() + sin * radius,
                ))
            }
            Formation::Lissajous {
                centre,
                amplitude,
                ratio,
                phase_rate,
            } => {
                let t = index as f64 / count as f64 * std::f64::consts::TAU;
                // the phase creeping round is what makes the figure turn itself inside out
                let phase = elapsed * phase_rate;
                let swell = 1.0 + 0.15 * energy;
                Some(Vec2D::new(
                    centre.x()
                        + amplitude.x() * swell * (ratio.0 * t + phase).sin() / aspect.max(0.01),
                    centre.y() + amplitude.y() * swell * (ratio.1 * t).sin(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particles::field::test_rng;

    #[test]
    fn a_ribbon_spans_the_canvas_left_to_right() {
        let ribbon = Formation::Ribbon {
            amplitude: 0.1,
            frequency: 2.0,
            phase_rate: 1.0,
        };
        let first = ribbon.target(0, 10, 0.0, 1.0, 0.0).unwrap();
        let last = ribbon.target(9, 10, 0.0, 1.0, 0.0).unwrap();
        assert_eq!(first.x(), 0.0);
        assert_eq!(last.x(), 1.0);
    }

    #[test]
    fn a_lattice_stays_inside_its_inset_before_it_shears() {
        let lattice = Formation::Lattice {
            cols: 6,
            rows: 4,
            inset: 0.1,
        };
        for i in 0..24 {
            let point = lattice.target(i, 24, 0.0, 1.0, 0.0).unwrap();
            assert!(point.x() >= 0.09 && point.x() <= 0.91, "{point:?}");
            assert!(point.y() >= 0.09 && point.y() <= 0.91, "{point:?}");
        }
    }

    /// the widest a target ever gets from the canvas over `seconds` of holding
    fn bounds(formation: &Formation, seconds: f64, aspect: f64) -> (f64, f64) {
        let mut worst: (f64, f64) = (1.0, 0.0);
        let mut elapsed = 0.0;
        while elapsed <= seconds {
            for index in 0..200 {
                let Some(point) = formation.target(index, 200, elapsed, aspect, 1.0) else {
                    continue;
                };
                worst.0 = worst.0.min(point.x()).min(point.y());
                worst.1 = worst.1.max(point.x()).max(point.y());
            }
            elapsed += 0.1;
        }
        worst
    }

    #[test]
    fn the_curves_stay_on_the_canvas_for_as_long_as_they_hold() {
        let mut rng = test_rng();
        for _ in 0..40 {
            // a whole 16:9 window, half of one, and a square
            for aspect in [1.78, 0.89, 1.0] {
                for formation in [
                    Formation::haloes(&mut rng, aspect),
                    Formation::spiral(&mut rng, aspect),
                    Formation::lissajous(&mut rng, aspect),
                    Formation::ribbon(&mut rng),
                ] {
                    // longer than any hold, so a drifter has to have turned round by then
                    let (low, high) = bounds(&formation, 12.0, aspect);
                    assert!(
                        low > -0.05 && high < 1.05,
                        "at {aspect}, {formation:?} reaches {low} to {high}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_silhouette_that_spins_stays_on_the_canvas_as_it_turns() {
        // an outline far wider than it is tall, which is the one that swings off the canvas
        // if it is placed by the box it happens to fill rather than the circle it turns in
        let shape = EdgeShape::wide_bar();
        let mut rng = test_rng();
        for _ in 0..40 {
            let formation = Formation::sprite(&shape, 1.78, &mut rng, &[]);
            let (low, high) = bounds(&formation, 4.5, 1.78);
            assert!(
                low > -0.05 && high < 1.05,
                "{formation:?} reaches {low} to {high}"
            );
        }
    }

    #[test]
    fn one_set_of_haloes_however_many_players_there_are() {
        let mut rng = test_rng();
        let Formation::Haloes { radii, .. } = Formation::haloes(&mut rng, 1.78) else {
            panic!("not haloes")
        };
        // rings about one centre: a circle bolted to each board is decoration, not a routine
        assert!((3..=4).contains(&radii.len()), "{radii:?}");
        assert!(
            radii.windows(2).all(|w| w[0] > w[1]),
            "outermost first: {radii:?}"
        );
    }

    #[test]
    fn a_wanderer_turns_round_at_the_edges_rather_than_leaving() {
        for elapsed in 0..400 {
            let value = Formation::wander(0.3, 0.4, elapsed as f64 * 0.1, 0.2, 0.8);
            assert!((0.2..=0.8).contains(&value), "{elapsed} {value}");
        }
        // and it is going somewhere: it does not simply sit at its start
        let start = Formation::wander(0.3, 0.4, 0.0, 0.2, 0.8);
        assert!((start - Formation::wander(0.3, 0.4, 1.0, 0.2, 0.8)).abs() > 0.1);
    }

    #[test]
    fn a_free_formation_has_no_targets() {
        assert!(Formation::Free.target(0, 10, 0.0, 1.0, 0.0).is_none());
    }

    /// how much board the average placement covers, over enough tries to be stable
    fn mean_cover(boards: &[RectF]) -> f64 {
        let shape = EdgeShape::unit_square();
        let mut rng = test_rng();
        let mut total = 0.0;
        const TRIES: usize = 200;
        for _ in 0..TRIES {
            let Formation::Sprite { centre, size, .. } =
                Formation::sprite(&shape, 1.0, &mut rng, boards)
            else {
                panic!("not a sprite")
            };
            let half = size * 0.5;
            total += boards
                .iter()
                .map(|board| overlap(board, centre, half, half))
                .sum::<f64>();
        }
        total / TRIES as f64
    }

    #[test]
    fn a_silhouette_gathers_clear_of_the_boards_where_it_can() {
        let boards = [
            RectF::new(0.0, 0.0, 0.22, 1.0),
            RectF::new(0.78, 0.0, 0.22, 1.0),
        ];
        // the same placement with nothing to avoid, as the yardstick
        let nowhere = [RectF::new(-9.0, -9.0, 0.01, 0.01)];
        let blind = {
            let shape = EdgeShape::unit_square();
            let mut rng = test_rng();
            let mut total = 0.0;
            for _ in 0..200 {
                let Formation::Sprite { centre, size, .. } =
                    Formation::sprite(&shape, 1.0, &mut rng, &nowhere)
                else {
                    panic!("not a sprite")
                };
                let half = size * 0.5;
                total += boards
                    .iter()
                    .map(|board| overlap(board, centre, half, half))
                    .sum::<f64>();
            }
            total / 200.0
        };
        let aware = mean_cover(&boards);
        assert!(
            aware < blind * 0.6,
            "covers {aware:.4} of the boards, against {blind:.4} placing blind"
        );
    }

    #[test]
    fn a_silhouette_still_appears_when_the_boards_leave_nowhere_to_go() {
        let shape = EdgeShape::unit_square();
        let mut rng = test_rng();
        let boards = [RectF::new(0.0, 0.0, 1.0, 1.0)];
        let Formation::Sprite { centre, .. } = Formation::sprite(&shape, 1.0, &mut rng, &boards)
        else {
            panic!("not a sprite")
        };
        assert!(centre.x() > 0.0 && centre.x() < 1.0, "{centre:?}");
    }

    #[test]
    fn a_wide_canvas_does_not_stretch_a_silhouette() {
        let shape = Formation::Sprite {
            points: vec![Vec2D::new(0.5, 0.0), Vec2D::new(0.0, 0.5)],
            centre: Vec2D::new(0.5, 0.5),
            size: 1.0,
            spin: 0.0,
            drift: Vec2D::ZERO,
            half: Vec2D::new(0.0, 0.0),
        };
        // on a canvas twice as wide as it is tall, a point half a unit right is only a
        // quarter of the canvas across, but half a unit down is half of it
        let right = shape.target(0, 2, 0.0, 2.0, 0.0).unwrap();
        let down = shape.target(1, 2, 0.0, 2.0, 0.0).unwrap();
        assert_eq!(right.x() - 0.5, 0.25);
        assert_eq!(down.y() - 0.5, 0.5);
    }
}
