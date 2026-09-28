//! The squash a cell plays where it lands, keyed by board point and expiring on its own clock. Fed
//! by [`crate::game::GameEvent::Landed`]; a theme draws it with its `CellAnimationData::bounce`
//! strip.

use crate::game::geometry::Point;
use crate::game::PlacedCell;
use std::collections::HashMap;
use std::time::Duration;

/// How long a landing takes; short enough to finish before the cell can be disturbed again.
const BOUNCE_DURATION: Duration = Duration::from_millis(140);

/// A cell that landed and has not finished bouncing.
#[derive(Clone, Debug)]
pub struct BounceAnimation {
    /// how far each point is through its bounce
    cells: HashMap<Point, Duration>,
    duration: Duration,
}

impl BounceAnimation {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            duration: BOUNCE_DURATION,
        }
    }

    /// start or restart a bounce at each of `cells`, so a cell that lands twice bounces twice
    pub fn land(&mut self, cells: &[PlacedCell]) {
        for (point, _) in cells {
            self.cells.insert(*point, Duration::ZERO);
        }
    }

    pub fn update(&mut self, delta: Duration) {
        if self.cells.is_empty() {
            return;
        }
        for elapsed in self.cells.values_mut() {
            *elapsed += delta;
        }
        self.cells.retain(|_, elapsed| *elapsed < self.duration);
    }

    pub fn reset(&mut self) {
        self.cells.clear();
    }

    /// which frame of a `frames` long strip the cell at `point` is on, if it is bouncing
    pub fn frame(&self, point: Point, frames: usize) -> Option<usize> {
        if frames == 0 {
            return None;
        }
        let elapsed = self.cells.get(&point)?;
        let through = elapsed.as_secs_f64() / self.duration.as_secs_f64();
        Some(((through * frames as f64) as usize).min(frames - 1))
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

impl Default for BounceAnimation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::game::CellId;

    fn at(x: i32, y: i32) -> PlacedCell {
        (Point::new(x, y), CellId(0))
    }

    #[test]
    fn a_landed_cell_walks_its_strip_and_then_stops() {
        let mut bounce = BounceAnimation::new();
        bounce.land(&[at(0, 0)]);
        assert_eq!(bounce.frame(Point::new(0, 0), 4), Some(0));
        bounce.update(BOUNCE_DURATION / 2);
        assert_eq!(bounce.frame(Point::new(0, 0), 4), Some(2));
        bounce.update(BOUNCE_DURATION);
        assert_eq!(
            bounce.frame(Point::new(0, 0), 4),
            None,
            "and it is over, not stuck on its last frame"
        );
    }

    /// The two halves of a pair bounce at the moments each landed.
    #[test]
    fn every_point_keeps_its_own_clock() {
        let mut bounce = BounceAnimation::new();
        bounce.land(&[at(0, 0)]);
        bounce.update(BOUNCE_DURATION / 2);
        bounce.land(&[at(1, 0)]);
        assert_eq!(bounce.frame(Point::new(0, 0), 4), Some(2));
        assert_eq!(bounce.frame(Point::new(1, 0), 4), Some(0));
    }

    #[test]
    fn a_point_that_never_landed_never_bounces() {
        let mut bounce = BounceAnimation::new();
        bounce.land(&[at(0, 0)]);
        assert_eq!(bounce.frame(Point::new(5, 5), 4), None);
    }
}
