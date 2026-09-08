//! Puyo Puyo's rotation, for the two-cell pieces that use it.
//!
//! This is the geometry only - a pivot, a child orbiting it, and the rules for sliding,
//! falling and turning over a board that says which cells are free. What the two halves *are*
//! is the game's, so a game wraps a [`PairMotion`] in its own piece type rather than the other
//! way round: see `puyo-rusto/src/game/pair.rs`, which is that wrapper.
//!
//! It rhymes with Dr. Rustario's pill - two halves, a pivot, kicks, splitting once it lands -
//! but the kick rules are Puyo's own, so `pill.rs` stays where it is rather than being merged
//! into this. The rules here are Puyo Nexus's [Rotation](https://puyonexus.com/wiki/Rotation),
//! read 2026-08-27:
//!
//! * a **floor kick** pushes the whole pair *up* when the cell a puyo is rotating down into is
//!   taken;
//! * a **wall kick** pushes it sideways when the cell it is rotating into is a wall or a puyo;
//! * when the kicked-to cell is taken as well, the rotation is refused and the **double
//!   rotate** (quick turn) rule takes over: pressing rotate again flips the pair end over end,
//!   in place, the two halves swapping the cells they already hold.
//!
//! One rule is not on that page and is easy to miss: a pair whose pivot is already under the
//! ceiling may not turn upright at all once the cell it wants is taken - the rotation is
//! refused rather than kicked anywhere. That is the game's own *current row check*, from
//! [Rotation, collision and push
//! back](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back), and it
//! is what stops a player shuffling a pair about up in the ghost rows. Whether a board has
//! such a row at all is the board's to say - [`PairBoard::is_ceiling`] defaults to no.

use crate::game::geometry::{Point, Rotation};

/// The board a [`PairMotion`] moves over, as far as the motion needs to know it.
pub trait PairBoard {
    /// free *and* on the board; anything off the board counts as occupied, so a pair cannot be
    /// moved into it
    fn is_free(&self, point: Point) -> bool;

    /// Is a pair with its pivot here up against the ceiling, where an upright rotation is
    /// refused outright rather than kicked anywhere - and does not even arm the quick turn?
    ///
    /// Puyo Puyo Tsu's *current row check*, which is half of its ceiling above the thirteenth
    /// row; the other half is the board having no fourteenth row to turn into. A board with no
    /// such row says no, which is the default.
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
    /// flipped end over end, the pair being wedged too tightly to turn
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

    /// would the pair fit here, with both halves on the board and on free cells?
    fn fits<B: PairBoard + ?Sized>(board: &B, candidate: &PairMotion) -> bool {
        candidate.points().iter().all(|point| board.is_free(*point))
    }

    fn moved(&self, dx: i32, dy: i32) -> PairMotion {
        PairMotion {
            pivot: self.pivot.translate(dx, dy),
            ..*self
        }
    }

    /// slide sideways, if there is room for both halves
    pub fn shift<B: PairBoard + ?Sized>(&mut self, board: &B, dx: i32) -> bool {
        let candidate = self.moved(dx, 0);
        if Self::fits(board, &candidate) {
            self.pivot = candidate.pivot;
            true
        } else {
            false
        }
    }

    /// step down one row, if there is room
    pub fn fall<B: PairBoard + ?Sized>(&mut self, board: &B) -> bool {
        let candidate = self.moved(0, 1);
        if Self::fits(board, &candidate) {
            self.pivot = candidate.pivot;
            true
        } else {
            false
        }
    }

    /// nothing below either half: the pair is about to lock
    pub fn is_resting<B: PairBoard + ?Sized>(&self, board: &B) -> bool {
        !Self::fits(board, &self.moved(0, 1))
    }

    /// fall as far as the pair will go, returning how many rows it dropped
    pub fn hard_drop<B: PairBoard + ?Sized>(&mut self, board: &B) -> u32 {
        let mut rows = 0;
        while self.fall(board) {
            rows += 1;
        }
        rows
    }

    /// where the pair would come to rest, for the ghost
    pub fn ghost<B: PairBoard + ?Sized>(&self, board: &B) -> PairMotion {
        let mut ghost = *self;
        ghost.hard_drop(board);
        ghost
    }

    /// Turn a quarter, kicking off the floor or a wall if that is what it takes.
    ///
    /// Refusing a rotation arms the quick turn, so a second press flips the pair instead -
    /// which is the only way out when it is wedged between two columns.
    pub fn rotate<B: PairBoard + ?Sized>(&mut self, board: &B, clockwise: bool) -> RotateOutcome {
        let rotation = self.rotation.rotate(clockwise);
        let turned = PairMotion { rotation, ..*self };
        if Self::fits(board, &turned) {
            self.rotation = rotation;
            self.quick_turn_armed = false;
            return RotateOutcome::Turned;
        }

        // A pair whose pivot is under the ceiling may not turn upright at all once the cell it
        // wants is taken: the rotation is refused outright rather than pushed anywhere, and it
        // does not even arm the quick turn. Puyo Nexus, [Rotation, collision and push
        // back](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back)
        // - the current row check, `if(current_row < 2) if(target_cell == bottom || target_cell
        // == top) exit;`. It is what keeps a player from shoving a pair about up in the ghost
        // rows, and it is half of Tsu's ceiling: the other half is the board having no
        // fourteenth row to turn into.
        if board.is_ceiling(self.pivot) && matches!(rotation, Rotation::North | Rotation::South) {
            return RotateOutcome::Blocked;
        }

        // push the pair away from whatever the child was turning into: down into the floor
        // pushes up, into a wall pushes sideways
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

    /// The double rotate: the two halves swap cells, so the pair flips end over end without
    /// moving.
    ///
    /// Puyo Nexus, [Rotation, collision and push
    /// back](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back): "a
    /// rotation pushes the pair's main puyo upwards, with the slave puyo taking its place at
    /// the bottom; or the slave puyo ends up at the top with the main puyo being pushed down
    /// by one cell". Either way the pair ends up on the same two squares with the halves the
    /// other way round - which is why the page can say that by this point "nothing will cancel
    /// the rotation". Those two squares are the ones the pair is already standing on, so there
    /// is nothing left to collide with and this cannot fail.
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

    /// a board of nothing but occupancy, which is all the motion asks of one
    #[derive(Clone, Default)]
    struct Grid {
        taken: Vec<Point>,
        /// row 0 is the ghost row, the way Puyo Rusto's board has one
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

    /// wedged between two columns: the first press is refused and the second flips the pair
    /// end over end, in place
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

    /// under a ceiling an upright rotation is refused outright: no kick, and no arming either
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

    /// A board with no ceiling row says so by saying nothing, and the same pair in the same
    /// place is an ordinary wedged pair again: refused once, then flipped.
    #[test]
    fn a_board_with_no_ceiling_quick_turns_in_its_top_row_instead() {
        let full = (0..COLUMNS).fold(Grid::default(), |grid, x| grid.with(x, 1..ROWS));
        let mut pair = PairMotion::new(Point::new(2, 0));
        assert_eq!(pair.rotate(&full, true), RotateOutcome::Turned);
        assert_eq!(pair.rotate(&full, true), RotateOutcome::Blocked, "armed");
        assert_eq!(pair.rotate(&full, true), RotateOutcome::QuickTurned);
    }
}
