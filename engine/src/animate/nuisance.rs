//! An attack falling in from over the top of the board.
//!
//! Garbage that waited in the tray does not appear where it lands: it drops in from above the
//! board, fast, and the game is held while it does. The rules have already put every cell
//! where it comes to rest - this is only where each one is *drawn* on the way there - so
//! nothing about the game is waiting on the answer, and a headless run never plays it at all.
//!
//! A column falls as one piece, keeping the spacing it lands in, so five rows read as a slab
//! of garbage rather than as a shower; and the bottom of each column starts one row above the
//! first visible one, so nothing is ever seen appearing in mid-board. A column landing on an
//! empty well takes longer to arrive than one landing on a full stack, so the board fills
//! raggedly from the top - which is what falling from a common height means.
//!
//! The fall itself is under **gravity** ([`NuisanceFall`]): it starts slowly, accelerates,
//! and each column is given a small head start of its own so the level row it starts as
//! breaks up on the way down. That head start is a hash of the column index rather than a
//! random number - the same board falls the same way twice, it can be asserted about, and
//! there is no randomness anywhere on the render path.
//!
//! Every cell reports itself as it **lands**, so a theme that squashes a landing cell
//! ([`crate::animate::bounce`]) squashes each refugee bean as it arrives, staggered by
//! column. That is the rumble Mean Bean Machine actually has: nothing shakes, but the whole
//! bottom of the board bounces at once.

use crate::game::geometry::Point as CellPoint;
use crate::game::{CellId, PlacedCell};
use std::collections::HashMap;
use std::time::Duration;

/// How an attack falls in: a shove to start it, gravity to build it up, and a small
/// per-column stagger so the row it starts as does not stay a row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NuisanceFall {
    /// rows a second at the moment it appears
    pub initial_speed: f64,
    /// rows a second squared
    pub acceleration: f64,
    /// rows a second it will not go past
    pub max_speed: f64,
    /// the most any one column is held back by, before it starts falling at all
    pub column_jitter: Duration,
}

impl NuisanceFall {
    /// a fall at one constant speed, which is what every theme had before gravity
    pub fn at(rows_per_second: f64) -> Self {
        Self {
            initial_speed: rows_per_second,
            acceleration: 0.0,
            max_speed: rows_per_second,
            column_jitter: Duration::ZERO,
        }
    }

    pub fn accelerating(initial_speed: f64, acceleration: f64, max_speed: f64) -> Self {
        Self {
            initial_speed,
            acceleration,
            max_speed,
            column_jitter: Duration::ZERO,
        }
    }

    pub fn jittered_by(self, column_jitter: Duration) -> Self {
        Self {
            column_jitter,
            ..self
        }
    }

    /// How long a column is held back before it starts to fall.
    ///
    /// The golden ratio's fractional part, which spreads consecutive integers about as evenly
    /// over 0..1 as anything can: neighbouring columns are given very different offsets, so a
    /// level row visibly breaks up rather than tilting. It is a hash and not a random number,
    /// so the same board falls the same way every time and a test can say so.
    fn delay(&self, column: i32) -> f64 {
        const GOLDEN: f64 = 0.618_033_988_749_895;
        self.column_jitter.as_secs_f64() * ((column as f64 * GOLDEN).fract().abs())
    }

    /// how many rows something that started `seconds` ago has fallen
    fn fallen_in(&self, seconds: f64) -> f64 {
        if seconds <= 0.0 {
            return 0.0;
        }
        if self.acceleration <= 0.0 {
            return self.initial_speed * seconds;
        }
        // ... until it reaches the speed limit, after which it is a straight line again
        let to_limit = (self.max_speed - self.initial_speed).max(0.0) / self.acceleration;
        if seconds <= to_limit {
            self.initial_speed * seconds + 0.5 * self.acceleration * seconds * seconds
        } else {
            let at_limit =
                self.initial_speed * to_limit + 0.5 * self.acceleration * to_limit * to_limit;
            at_limit + self.max_speed * (seconds - to_limit)
        }
    }
}

/// where each falling cell lands, and how many rows above that it starts
#[derive(Clone, Debug)]
pub struct State {
    cells: HashMap<CellPoint, (CellId, f64)>,
    fall: NuisanceFall,
    elapsed: Duration,
}

impl State {
    /// how far a column has fallen, in rows
    fn fallen(&self, column: i32) -> f64 {
        self.fall
            .fallen_in(self.elapsed.as_secs_f64() - self.fall.delay(column))
    }

    fn finished(&self) -> bool {
        self.cells
            .iter()
            .all(|(point, (_, distance))| self.fallen(point.x) >= *distance)
    }

    /// whether this cell is still in the air, and so must not be drawn where it lands
    pub fn is_falling(&self, point: CellPoint) -> bool {
        self.cells
            .get(&point)
            .is_some_and(|(_, distance)| self.fallen(point.x) < *distance)
    }

    /// every cell still in the air, with how far above its landing place to draw it, in rows.
    ///
    /// Negative, the way every other animation's `offset_y` is: up the board. Two of them can
    /// never overlap - a column keeps its spacing and columns do not move sideways - so the
    /// order they come out in does not matter.
    pub fn frames(&self) -> Vec<(CellPoint, CellId, f64)> {
        self.cells
            .iter()
            .filter_map(|(point, (id, distance))| {
                let fallen = self.fallen(point.x);
                (fallen < *distance).then_some((*point, *id, fallen - distance))
            })
            .collect()
    }

    /// every cell that crossed its landing place between `before` and now
    fn landed_since(&self, before: Duration) -> Vec<PlacedCell> {
        let was = Self {
            cells: HashMap::new(),
            fall: self.fall,
            elapsed: before,
        };
        self.cells
            .iter()
            .filter(|(point, (_, distance))| {
                self.fallen(point.x) >= *distance && was.fallen(point.x) < *distance
            })
            .map(|(point, (id, _))| (*point, *id))
            .collect()
    }
}

#[derive(Clone, Debug, Default)]
pub struct NuisanceAnimation {
    state: Option<State>,
}

impl NuisanceAnimation {
    pub fn new() -> Self {
        Self::default()
    }

    /// Step the fall, reporting every cell that came to rest this frame.
    ///
    /// The caller passes those to whatever bounces a landing cell, which is how a slab of
    /// nuisance arrives one column at a time rather than all at once.
    pub fn update(&mut self, delta: Duration) -> Vec<PlacedCell> {
        let Some(state) = self.state.as_mut() else {
            return vec![];
        };
        let before = state.elapsed;
        state.elapsed += delta;
        let landed = state.landed_since(before);
        if state.finished() {
            self.state = None;
        }
        landed
    }

    pub fn reset(&mut self) {
        self.state = None;
    }

    /// Start `cells` falling in, `hidden_rows` being how many rows sit above the visible board.
    ///
    /// The distance is worked out per column off its *lowest* cell, which is the one that
    /// stops one row above the board's first visible row; everything above it in that column
    /// starts higher again and is clipped by the board until it arrives.
    pub fn drop_in(&mut self, cells: &[PlacedCell], hidden_rows: u32, fall: NuisanceFall) {
        // a drop always replaces whatever was in the air: only one of them can be landing
        self.state = None;
        if cells.is_empty() || fall.initial_speed <= 0.0 {
            return;
        }
        let mut floor: HashMap<i32, i32> = HashMap::new();
        for (point, _) in cells.iter() {
            let deepest = floor.entry(point.x).or_insert(point.y);
            *deepest = (*deepest).max(point.y);
        }
        let cells = cells
            .iter()
            .filter_map(|(point, id)| {
                let distance = floor[&point.x] - hidden_rows as i32 + 1;
                (distance > 0).then_some((*point, (*id, distance as f64)))
            })
            .collect::<HashMap<CellPoint, (CellId, f64)>>();
        if cells.is_empty() {
            return;
        }
        self.state = Some(State {
            cells,
            fall,
            elapsed: Duration::ZERO,
        });
    }

    pub fn state(&self) -> Option<&State> {
        self.state.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::CellId;

    const ID: CellId = CellId(1);
    /// one row a second at a constant speed, so a duration reads as a distance
    const SLOW: NuisanceFall = NuisanceFall {
        initial_speed: 1.0,
        acceleration: 0.0,
        max_speed: 1.0,
        column_jitter: Duration::ZERO,
    };

    fn cell(x: i32, y: i32) -> PlacedCell {
        (CellPoint::new(x, y), ID)
    }

    #[test]
    fn nothing_to_drop_starts_nothing() {
        let mut animation = NuisanceAnimation::new();
        animation.drop_in(&[], 1, SLOW);
        assert!(animation.state().is_none());
    }

    /// the lowest cell of a column stops one row above the first visible one, so a well that
    /// is empty to the floor is the longest fall on the board
    #[test]
    fn a_column_falls_the_depth_of_where_its_lowest_cell_lands() {
        let mut animation = NuisanceAnimation::new();
        animation.drop_in(&[cell(0, 12)], 1, SLOW);
        let state = animation.state().expect("falling");
        assert!(state.is_falling(CellPoint::new(0, 12)));
        assert_eq!(state.frames().len(), 1);
        assert_eq!(
            state.frames()[0].2,
            -12.0,
            "twelve rows above where it lands"
        );
    }

    /// a slab keeps its shape: every cell of a column shares the fall, so none of them is ever
    /// seen appearing in mid-board
    #[test]
    fn a_column_falls_as_one_piece() {
        let mut animation = NuisanceAnimation::new();
        animation.drop_in(&[cell(0, 10), cell(0, 11), cell(0, 12)], 1, SLOW);
        let state = animation.state().expect("falling");
        let offsets: Vec<f64> = state
            .frames()
            .iter()
            .map(|(_, _, offset)| *offset)
            .collect();
        assert_eq!(
            offsets,
            vec![-12.0; 3],
            "all three, the depth of the lowest"
        );
    }

    /// a puyo landing on the row below the top has nowhere to fall from and so does not
    #[test]
    fn a_cell_landing_at_the_top_of_the_board_does_not_animate() {
        let mut animation = NuisanceAnimation::new();
        // hidden row 0, first visible row 1: a cell landing there starts one row up
        animation.drop_in(&[cell(0, 1)], 1, SLOW);
        assert_eq!(animation.state().expect("falling").frames()[0].2, -1.0);
        // ... and one in the hidden row itself has no room above it at all
        animation.drop_in(&[cell(0, 0)], 1, SLOW);
        assert!(animation.state().is_none());
    }

    #[test]
    fn a_shallow_column_lands_while_a_deep_one_is_still_falling() {
        let mut animation = NuisanceAnimation::new();
        animation.drop_in(&[cell(0, 3), cell(1, 12)], 1, SLOW);
        animation.update(Duration::from_secs(4));
        let state = animation.state().expect("still falling");
        assert!(
            !state.is_falling(CellPoint::new(0, 3)),
            "the shallow one is down"
        );
        assert!(
            state.is_falling(CellPoint::new(1, 12)),
            "the deep one is not"
        );
        assert_eq!(state.frames().len(), 1);
    }

    /// it starts slowly and speeds up, which is what falling looks like and what a constant
    /// speed never did
    #[test]
    fn a_fall_under_gravity_accelerates() {
        let fall = NuisanceFall::accelerating(4.0, 20.0, 100.0);
        let first = fall.fallen_in(0.1);
        let second = fall.fallen_in(0.2) - first;
        let third = fall.fallen_in(0.3) - fall.fallen_in(0.2);
        assert!(
            first < second && second < third,
            "each tenth of a second covers more ground than the last: {first} {second} {third}"
        );
    }

    /// ... and never past its limit, so a deep well is a fall rather than a teleport
    #[test]
    fn a_fall_stops_speeding_up_at_its_limit() {
        let fall = NuisanceFall::accelerating(4.0, 100.0, 10.0);
        let late = fall.fallen_in(1.0) - fall.fallen_in(0.9);
        assert!(
            (late - 1.0).abs() < 0.01,
            "ten rows a second, no more: {late}"
        );
    }

    /// the level row an attack starts as has to break up on the way down, or a slab lands
    /// like a wall rather than like rain
    #[test]
    fn two_columns_falling_the_same_distance_land_at_different_times() {
        let mut animation = NuisanceAnimation::new();
        let fall =
            NuisanceFall::accelerating(4.0, 20.0, 100.0).jittered_by(Duration::from_millis(60));
        animation.drop_in(&[cell(0, 12), cell(1, 12), cell(2, 12)], 1, fall);
        let mut landings = vec![];
        for step in 0..200 {
            if !animation.update(Duration::from_millis(5)).is_empty() {
                landings.push(step);
            }
        }
        assert_eq!(landings.len(), 3, "each column arrives on its own");
        assert!(
            landings.windows(2).all(|w| w[0] != w[1]),
            "and not all in the same frame: {landings:?}"
        );
    }

    /// ... but the same board falls the same way every time, because the stagger is a hash of
    /// the column and not a random number
    #[test]
    fn the_stagger_is_the_same_every_time() {
        let fall = NuisanceFall::at(1.0).jittered_by(Duration::from_millis(60));
        assert_eq!(fall.delay(3), fall.delay(3));
        assert_ne!(fall.delay(3), fall.delay(4));
    }

    #[test]
    fn the_animation_ends_when_the_last_cell_lands() {
        let mut animation = NuisanceAnimation::new();
        animation.drop_in(&[cell(0, 12)], 1, SLOW);
        animation.update(Duration::from_millis(11_999));
        assert!(animation.state().is_some());
        let landed = animation.update(Duration::from_millis(1));
        assert_eq!(landed.len(), 1, "and it says so as it arrives");
        assert!(animation.state().is_none());
    }
}
