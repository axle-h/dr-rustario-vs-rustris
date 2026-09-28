//! Puyo Puyo's rotation for two-cell pieces, shared by Puyo Rusto and Super Rustle Fighter,
//! from Puyo Nexus's [Rotation](https://puyonexus.com/wiki/Rotation). Geometry only: a game
//! wraps a [`PairMotion`] in its own piece type.

use crate::game::geometry::{Point, Rotation};

/// The board a [`PairMotion`] moves over, as far as the motion needs to know it.
pub trait PairBoard {
    /// free and on the board; off the board counts as occupied
    fn is_free(&self, point: Point) -> bool;

    /// Whether a pivot here is under the ceiling, where an upright rotation into a taken cell
    /// is refused without a kick or arming the quick turn (Puyo Puyo Tsu's current row check).
    fn is_ceiling(&self, _pivot: Point) -> bool {
        false
    }
}

/// where the child sits relative to the pivot, in a `y`-grows-down grid
pub fn child_offset(rotation: Rotation) -> Point {
    match rotation {
        Rotation::North => Point::new(0, -1),
        Rotation::East => Point::new(1, 0),
        Rotation::South => Point::new(0, 1),
        Rotation::West => Point::new(-1, 0),
    }
}

/// What a rotation attempt did, so the game can tell a refused press from a taken one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotateOutcome {
    /// turned on the spot
    Turned,
    /// turned after the pair was pushed out of the way
    Kicked,
    /// flipped end over end in place
    QuickTurned,
    /// nothing was possible; a second press will try the quick turn
    Blocked,
}

/// Where a pair is and which way it faces: a pivot, a child orbiting it, and how it moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PairMotion {
    pivot: Point,
    rotation: Rotation,
    /// a rotation has already been refused, so the next one may flip the pair instead
    quick_turn_armed: bool,
}

impl PairMotion {
    /// a pair enters standing up, the child above the pivot
    pub fn new(pivot: Point) -> Self {
        Self {
            pivot,
            rotation: Rotation::North,
            quick_turn_armed: false,
        }
    }

    pub fn pivot(&self) -> Point {
        self.pivot
    }

    pub fn rotation(&self) -> Rotation {
        self.rotation
    }

    pub fn child(&self) -> Point {
        self.pivot + child_offset(self.rotation)
    }

    pub fn points(&self) -> [Point; 2] {
        [self.pivot, self.child()]
    }

    fn fits<B: PairBoard + ?Sized>(board: &B, candidate: &PairMotion) -> bool {
        candidate.points().iter().all(|point| board.is_free(*point))
    }

    fn moved(&self, dx: i32, dy: i32) -> PairMotion {
        PairMotion {
            pivot: self.pivot.translate(dx, dy),
            ..*self
        }
    }

    pub fn shift<B: PairBoard + ?Sized>(&mut self, board: &B, dx: i32) -> bool {
        let candidate = self.moved(dx, 0);
        if Self::fits(board, &candidate) {
            self.pivot = candidate.pivot;
            true
        } else {
            false
        }
    }

    pub fn fall<B: PairBoard + ?Sized>(&mut self, board: &B) -> bool {
        let candidate = self.moved(0, 1);
        if Self::fits(board, &candidate) {
            self.pivot = candidate.pivot;
            true
        } else {
            false
        }
    }

    pub fn is_resting<B: PairBoard + ?Sized>(&self, board: &B) -> bool {
        !Self::fits(board, &self.moved(0, 1))
    }

    /// returns the rows dropped
    pub fn hard_drop<B: PairBoard + ?Sized>(&mut self, board: &B) -> u32 {
        let mut rows = 0;
        while self.fall(board) {
            rows += 1;
        }
        rows
    }

    pub fn ghost<B: PairBoard + ?Sized>(&self, board: &B) -> PairMotion {
        let mut ghost = *self;
        ghost.hard_drop(board);
        ghost
    }

    /// Turn a quarter, kicking off the floor or a wall if needed. A refused rotation arms the
    /// quick turn, so the next press flips the pair instead.
    pub fn rotate<B: PairBoard + ?Sized>(&mut self, board: &B, clockwise: bool) -> RotateOutcome {
        let rotation = self.rotation.rotate(clockwise);
        let turned = PairMotion { rotation, ..*self };
        if Self::fits(board, &turned) {
            self.rotation = rotation;
            self.quick_turn_armed = false;
            return RotateOutcome::Turned;
        }

        // the current row check: under the ceiling an upright turn is refused without arming
        if board.is_ceiling(self.pivot) && matches!(rotation, Rotation::North | Rotation::South) {
            return RotateOutcome::Blocked;
        }

        // kick away from whatever the child turned into: up off the floor, sideways off a wall
        let away = -child_offset(rotation);
        let kicked = PairMotion {
            rotation,
            pivot: self.pivot + away,
            ..*self
        };
        if Self::fits(board, &kicked) {
            self.pivot = kicked.pivot;
            self.rotation = rotation;
            self.quick_turn_armed = false;
            return RotateOutcome::Kicked;
        }

        if self.quick_turn_armed {
            *self = self.quick_turn();
            return RotateOutcome::QuickTurned;
        }
        self.quick_turn_armed = true;
        RotateOutcome::Blocked
    }

    /// The halves swap cells. The pair stays on the squares it already holds, so this cannot
    /// fail.
    fn quick_turn(&self) -> PairMotion {
        PairMotion {
            pivot: self.child(),
            rotation: self.rotation.rotate(true).rotate(true),
            quick_turn_armed: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLUMNS: i32 = 6;
    const ROWS: i32 = 13;

    #[derive(Clone, Default)]
    struct Grid {
        taken: Vec<Point>,
        /// row 0 is a ceiling row
        ceiling: bool,
    }

    impl Grid {
        fn with(mut self, x: i32, ys: std::ops::Range<i32>) -> Self {
            self.taken.extend(ys.map(|y| Point::new(x, y)));
            self
        }

        fn under_a_ceiling(mut self) -> Self {
            self.ceiling = true;
            self
        }
    }

    impl PairBoard for Grid {
        fn is_free(&self, point: Point) -> bool {
            (0..COLUMNS).contains(&point.x)
                && (0..ROWS).contains(&point.y)
                && !self.taken.contains(&point)
        }

        fn is_ceiling(&self, pivot: Point) -> bool {
            self.ceiling && pivot.y < 1
        }
    }

    /// up, right, down, left is clockwise when `y` grows downwards
    #[test]
    fn the_child_orbits_the_pivot_clockwise_on_screen() {
        let empty = Grid::default();
        let mut pair = PairMotion::new(Point::new(2, 5));
        for expected in [
            Point::new(1, 0),
            Point::new(0, 1),
            Point::new(-1, 0),
            Point::new(0, -1),
        ] {
            assert_eq!(pair.rotate(&empty, true), RotateOutcome::Turned);
            assert_eq!(pair.child() - pair.pivot(), expected);
        }
    }

    #[test]
    fn a_wall_kick_pushes_the_pair_off_the_wall_and_a_floor_kick_lifts_it() {
        let empty = Grid::default();
        let mut pair = PairMotion::new(Point::new(COLUMNS - 1, 5));
        assert_eq!(pair.rotate(&empty, true), RotateOutcome::Kicked);
        assert_eq!(pair.pivot().x, COLUMNS - 2, "pushed off the right wall");

        let mut pair = PairMotion::new(Point::new(2, ROWS - 1));
        assert_eq!(pair.rotate(&empty, true), RotateOutcome::Turned);
        assert_eq!(pair.rotate(&empty, true), RotateOutcome::Kicked);
        assert_eq!(pair.pivot().y, ROWS - 2, "lifted off the floor");
        assert_eq!(pair.child().y, ROWS - 1);
    }

    /// wedged between two columns, the first press is refused and the second flips in place
    #[test]
    fn a_wedged_pair_quick_turns_on_the_second_press() {
        let wedge = Grid::default().with(1, 0..ROWS).with(3, 0..ROWS);
        let mut pair = PairMotion::new(Point::new(2, 5));
        let held = pair.points();
        assert_eq!(pair.rotate(&wedge, true), RotateOutcome::Blocked);
        assert_eq!(pair.rotation(), Rotation::North, "and it did not turn");
        assert_eq!(pair.rotate(&wedge, true), RotateOutcome::QuickTurned);
        assert_eq!(pair.pivot(), held[1], "the halves swapped cells");
        assert_eq!(pair.child(), held[0]);
        assert_eq!(
            pair.rotate(&wedge, true),
            RotateOutcome::Blocked,
            "the flip spent the arming rather than leaving it flipping back"
        );
    }

    #[test]
    fn a_pair_falls_until_something_is_under_it() {
        let stack = Grid::default().with(2, ROWS - 1..ROWS);
        let mut pair = PairMotion::new(Point::new(2, 1));
        assert_eq!(pair.hard_drop(&stack), ROWS as u32 - 3);
        assert!(pair.is_resting(&stack));
        assert_eq!(pair.ghost(&stack).pivot(), pair.pivot());
    }

    /// under a ceiling an upright rotation is refused without a kick or arming
    #[test]
    fn a_ceiling_refuses_an_upright_rotation_rather_than_kicking_it() {
        let full = Grid::default().under_a_ceiling();
        let full = (0..COLUMNS).fold(full, |grid, x| grid.with(x, 1..ROWS));
        let mut pair = PairMotion::new(Point::new(2, 0));
        assert_eq!(pair.rotate(&full, true), RotateOutcome::Turned, "sideways");
        assert_eq!(pair.rotate(&full, true), RotateOutcome::Blocked);
        assert_eq!(pair.pivot(), Point::new(2, 0), "no floor kick up here");
        assert_eq!(
            pair.rotate(&full, true),
            RotateOutcome::Blocked,
            "and pressing again is refused too rather than flipping the pair"
        );
        assert_eq!(pair.rotation(), Rotation::East, "still lying flat");
    }

    /// without a ceiling the same top-row pair is refused once, then flipped
    #[test]
    fn a_board_with_no_ceiling_quick_turns_in_its_top_row_instead() {
        let full = (0..COLUMNS).fold(Grid::default(), |grid, x| grid.with(x, 1..ROWS));
        let mut pair = PairMotion::new(Point::new(2, 0));
        assert_eq!(pair.rotate(&full, true), RotateOutcome::Turned);
        assert_eq!(pair.rotate(&full, true), RotateOutcome::Blocked, "armed");
        assert_eq!(pair.rotate(&full, true), RotateOutcome::QuickTurned);
    }
}
