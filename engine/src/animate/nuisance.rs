//! An attack falling in from over the top of the board.
//!
//! Garbage that waited in the tray does not appear where it lands: it drops in from above the
//! board, fast, and the game is held while it does. The rules have already put every cell
//! where it comes to rest - this is only where each one is *drawn* on the way there - so
//! nothing about the game is waiting on the answer, and a headless run never plays it at all.
//!
//! A column falls as one piece, keeping the spacing it lands in, so five rows read as a slab
//! of garbage rather than as a shower; and the bottom of each column starts one row above the
//! first visible one, so nothing is ever seen appearing in mid-board. The fall is at a
//! constant speed, so a column landing on an empty well takes longer to arrive than one
//! landing on a full stack and the board fills raggedly from the top - which is what falling
//! from a common height means.

use crate::game::geometry::Point as CellPoint;
use crate::game::{CellId, PlacedCell};
use std::collections::HashMap;
use std::time::Duration;

/// where each falling cell lands, and how many rows above that it starts
#[derive(Clone, Debug)]
pub struct State {
    cells: HashMap<CellPoint, (CellId, f64)>,
    rows_per_second: f64,
    elapsed: Duration,
}

impl State {
    /// how far everything has fallen, in rows
    fn fallen(&self) -> f64 {
        self.rows_per_second * self.elapsed.as_secs_f64()
    }

    fn finished(&self) -> bool {
        let fallen = self.fallen();
        self.cells.values().all(|(_, distance)| fallen >= *distance)
    }

    /// whether this cell is still in the air, and so must not be drawn where it lands
    pub fn is_falling(&self, point: CellPoint) -> bool {
        let fallen = self.fallen();
        self.cells
            .get(&point)
            .is_some_and(|(_, distance)| fallen < *distance)
    }

    /// every cell still in the air, with how far above its landing place to draw it, in rows.
    ///
    /// Negative, the way every other animation's `offset_y` is: up the board. Two of them can
    /// never overlap - a column keeps its spacing and columns do not move sideways - so the
    /// order they come out in does not matter.
    pub fn frames(&self) -> Vec<(CellPoint, CellId, f64)> {
        let fallen = self.fallen();
        self.cells
            .iter()
            .filter(|(_, (_, distance))| fallen < *distance)
            .map(|(point, (id, distance))| (*point, *id, fallen - distance))
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

    pub fn update(&mut self, delta: Duration) {
        if let Some(state) = self.state.as_mut() {
            state.elapsed += delta;
            if state.finished() {
                self.state = None;
            }
        }
    }

    pub fn reset(&mut self) {
        self.state = None;
    }

    /// Start `cells` falling in, `hidden_rows` being how many rows sit above the visible board.
    ///
    /// The distance is worked out per column off its *lowest* cell, which is the one that
    /// stops one row above the board's first visible row; everything above it in that column
    /// starts higher again and is clipped by the board until it arrives.
    pub fn drop_in(&mut self, cells: &[PlacedCell], hidden_rows: u32, rows_per_second: f64) {
        // a drop always replaces whatever was in the air: only one of them can be landing
        self.state = None;
        if cells.is_empty() || rows_per_second <= 0.0 {
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
            rows_per_second,
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
    /// one row a second, so a duration reads as a distance
    const SLOW: f64 = 1.0;

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

    #[test]
    fn the_animation_ends_when_the_last_cell_lands() {
        let mut animation = NuisanceAnimation::new();
        animation.drop_in(&[cell(0, 12)], 1, SLOW);
        animation.update(Duration::from_millis(11_999));
        assert!(animation.state().is_some());
        animation.update(Duration::from_millis(1));
        assert!(animation.state().is_none());
    }
}
