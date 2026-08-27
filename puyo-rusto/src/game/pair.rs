//! The two-puyo piece the player controls.
//!
//! It rhymes with Dr. Rustario's pill - two halves, a pivot, kicks, splitting once it lands -
//! but the kick rules are Puyo's own, so this is a sibling of `pill.rs` rather than anything
//! shared with it. The rules here are Puyo Nexus's
//! [Rotation](https://puyonexus.com/wiki/Rotation), read 2026-08-27:
//!
//! * a **floor kick** pushes the whole pair *up* when the cell a puyo is rotating down into is
//!   taken;
//! * a **wall kick** pushes it sideways when the cell it is rotating into is a wall or a puyo;
//! * when the kicked-to cell is taken as well, the rotation is refused and the **double
//!   rotate** (quick turn) rule takes over: pressing rotate again flips the pair end over end,
//!   in place, the two halves swapping the cells they already hold.
//!
//! One rule is not on that page and is easy to miss: a pair whose pivot is already in the
//! ghost row may not turn upright at all once the cell it wants is taken - the rotation is
//! refused rather than kicked anywhere. That is the game's own *current row check*, from
//! [Rotation, collision and push
//! back](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back), and it
//! is what stops a player shuffling a pair about up in the ghost rows.

use crate::game::board::{self, Board};
use crate::game::cell::{PuyoCell, PuyoColor, PuyoPiece};
use engine::game::geometry::{Point, Rotation};
use engine::game::PlacedCell;

/// where the child sits relative to the pivot, in a `y`-grows-down grid
fn child_offset(rotation: Rotation) -> Point {
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

/// The pair in play: a pivot, a child orbiting it, and the two colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pair {
    pivot: Point,
    rotation: Rotation,
    piece: PuyoPiece,
    /// a rotation has already been refused, so the next one may flip the pair instead
    quick_turn_armed: bool,
}

impl Pair {
    pub fn new(pivot: Point, piece: PuyoPiece) -> Self {
        Self {
            pivot,
            // a pair enters standing up, the child above the pivot
            rotation: Rotation::North,
            piece,
            quick_turn_armed: false,
        }
    }

    pub fn pivot(&self) -> Point {
        self.pivot
    }

    pub fn rotation(&self) -> Rotation {
        self.rotation
    }

    pub fn piece(&self) -> PuyoPiece {
        self.piece
    }

    pub fn child(&self) -> Point {
        self.pivot + child_offset(self.rotation)
    }

    pub fn points(&self) -> [Point; 2] {
        [self.pivot, self.child()]
    }

    /// the two halves and their colours, pivot first. They always draw **unlinked**: a pair
    /// joins to what it lands next to on lock, never before.
    pub fn cells(&self) -> Vec<PlacedCell> {
        vec![
            (self.pivot, PuyoCell::loose(self.piece.pivot).into()),
            (self.child(), PuyoCell::loose(self.piece.child).into()),
        ]
    }

    /// the colour of each half, pivot first
    pub fn colors(&self) -> [PuyoColor; 2] {
        [self.piece.pivot, self.piece.child]
    }

    /// would the pair fit here, with both halves on the board and on free cells?
    fn fits(&self, board: &Board, candidate: &Pair) -> bool {
        candidate.points().iter().all(|point| board.is_free(*point))
    }

    fn moved(&self, dx: i32, dy: i32) -> Pair {
        Pair {
            pivot: self.pivot.translate(dx, dy),
            ..*self
        }
    }

    /// slide sideways, if there is room for both halves
    pub fn shift(&mut self, board: &Board, dx: i32) -> bool {
        let candidate = self.moved(dx, 0);
        if self.fits(board, &candidate) {
            self.pivot = candidate.pivot;
            true
        } else {
            false
        }
    }

    /// step down one row, if there is room
    pub fn fall(&mut self, board: &Board) -> bool {
        let candidate = self.moved(0, 1);
        if self.fits(board, &candidate) {
            self.pivot = candidate.pivot;
            true
        } else {
            false
        }
    }

    /// nothing below either half: the pair is about to lock
    pub fn is_resting(&self, board: &Board) -> bool {
        !self.fits(board, &self.moved(0, 1))
    }

    /// fall as far as the pair will go, returning how many rows it dropped
    pub fn hard_drop(&mut self, board: &Board) -> u32 {
        let mut rows = 0;
        while self.fall(board) {
            rows += 1;
        }
        rows
    }

    /// where the pair would come to rest, for the ghost
    pub fn ghost(&self, board: &Board) -> Pair {
        let mut ghost = *self;
        ghost.hard_drop(board);
        ghost
    }

    /// Turn a quarter, kicking off the floor or a wall if that is what it takes.
    ///
    /// Refusing a rotation arms the quick turn, so a second press flips the pair instead -
    /// which is the only way out when it is wedged between two columns.
    pub fn rotate(&mut self, board: &Board, clockwise: bool) -> RotateOutcome {
        let rotation = self.rotation.rotate(clockwise);
        let turned = Pair { rotation, ..*self };
        if self.fits(board, &turned) {
            self.rotation = rotation;
            self.quick_turn_armed = false;
            return RotateOutcome::Turned;
        }

        // A pair whose pivot is in the ghost row may not turn upright at all once the cell it
        // wants is taken: the rotation is refused outright rather than pushed anywhere, and it
        // does not even arm the quick turn. Puyo Nexus, [Rotation, collision and push
        // back](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back)
        // - the current row check, `if(current_row < 2) if(target_cell == bottom || target_cell
        // == top) exit;`. It is what keeps a player from shoving a pair about up in the ghost
        // rows, and it is half of Tsu's ceiling: the other half is the board having no
        // fourteenth row to turn into.
        if board::is_ghost(self.pivot) && matches!(rotation, Rotation::North | Rotation::South) {
            return RotateOutcome::Blocked;
        }

        // push the pair away from whatever the child was turning into: down into the floor
        // pushes up, into a wall pushes sideways
        let away = -child_offset(rotation);
        let kicked = Pair {
            rotation,
            pivot: self.pivot + away,
            ..*self
        };
        if self.fits(board, &kicked) {
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
    fn quick_turn(&self) -> Pair {
        Pair {
            pivot: self.child(),
            rotation: self.rotation.rotate(true).rotate(true),
            quick_turn_armed: false,
            ..*self
        }
    }

    /// Put both halves on the board where they lie.
    ///
    /// They are placed and then left to gravity: a horizontal pair over a hole drops one half
    /// further than the other, which is the splitting the game is known for. Settling is
    /// [`Board::settle`]'s job, so this only has to lay them down - but it joins them up as it
    /// does, because a pair that lands on flat ground settles nothing and pops nothing, and
    /// nothing else would ever recompute the masks of what it just landed beside.
    pub fn lock(&self, board: &mut Board) {
        board.set(self.pivot, Some(PuyoCell::loose(self.piece.pivot)));
        board.set(self.child(), Some(PuyoCell::loose(self.piece.child)));
        board.recompute_links();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::board::tests::board;
    use crate::game::board::{COLUMNS, ROWS, SPAWN};

    fn pair_at(x: i32, y: i32) -> Pair {
        Pair::new(
            Point::new(x, y),
            PuyoPiece::new(PuyoColor::Red, PuyoColor::Blue),
        )
    }

    #[test]
    fn a_pair_enters_standing_up_with_the_child_above() {
        let pair = Pair::new(SPAWN, PuyoPiece::new(PuyoColor::Red, PuyoColor::Blue));
        assert_eq!(pair.rotation(), Rotation::North);
        assert_eq!(pair.pivot(), SPAWN);
        assert_eq!(pair.child(), SPAWN.translate(0, -1), "the child is above");
    }

    #[test]
    fn the_child_orbits_the_pivot_clockwise_on_screen() {
        let empty = Board::new();
        let mut pair = pair_at(2, 5);
        // up, right, down, left is clockwise when y grows downwards
        for expected in [
            Point::new(1, 0),
            Point::new(0, 1),
            Point::new(-1, 0),
            Point::new(0, -1),
        ] {
            pair.rotate(&empty, true);
            assert_eq!(pair.child() - pair.pivot(), expected);
        }
    }

    #[test]
    fn a_pair_slides_sideways_until_it_meets_a_wall() {
        let empty = Board::new();
        let mut pair = pair_at(0, 5);
        assert!(!pair.shift(&empty, -1), "the left wall stops it");
        assert!(pair.shift(&empty, 1));
        assert_eq!(pair.pivot().x, 1);
    }

    #[test]
    fn a_pair_falls_until_something_is_under_it() {
        let board = board(&["rrrrrr"]);
        let mut pair = pair_at(0, 1);
        let dropped = pair.hard_drop(&board);
        assert!(pair.is_resting(&board));
        // it comes to rest on the row above the stack
        assert_eq!(pair.pivot().y, ROWS as i32 - 2);
        assert_eq!(dropped, ROWS as i32 as u32 - 3);
    }

    /// rotating a puyo down into the floor pushes the whole pair up
    #[test]
    fn a_floor_kick_pushes_the_pair_up() {
        let empty = Board::new();
        let mut pair = pair_at(2, ROWS as i32 - 1);
        assert!(pair.is_resting(&empty));
        // North -> East is free, East -> South would put the child under the floor
        assert_eq!(pair.rotate(&empty, true), RotateOutcome::Turned);
        assert_eq!(pair.rotate(&empty, true), RotateOutcome::Kicked);
        assert_eq!(pair.rotation(), Rotation::South);
        assert_eq!(pair.pivot().y, ROWS as i32 - 2, "lifted a row");
        assert_eq!(pair.child().y, ROWS as i32 - 1, "the child took the floor");
    }

    /// ... and rotating into a wall pushes it sideways
    #[test]
    fn a_wall_kick_pushes_the_pair_off_the_wall() {
        let empty = Board::new();
        let mut pair = pair_at(5, 5);
        assert_eq!(pair.rotate(&empty, true), RotateOutcome::Kicked);
        assert_eq!(pair.rotation(), Rotation::East);
        assert_eq!(pair.pivot().x, 4, "pushed off the right wall");
        assert_eq!(pair.child().x, 5);

        let mut pair = pair_at(0, 5);
        assert_eq!(pair.rotate(&empty, false), RotateOutcome::Kicked);
        assert_eq!(pair.rotation(), Rotation::West);
        assert_eq!(pair.pivot().x, 1, "pushed off the left wall");
        assert_eq!(pair.child().x, 0);
    }

    /// a puyo in the way kicks exactly as a wall does
    #[test]
    fn a_stack_kicks_the_pair_the_same_way_a_wall_does() {
        let mut wall = Board::new();
        for y in 0..ROWS as i32 {
            wall.set(Point::new(3, y), Some(PuyoCell::loose(PuyoColor::Green)));
        }
        let mut pair = pair_at(2, 5);
        assert_eq!(pair.rotate(&wall, true), RotateOutcome::Kicked);
        assert_eq!(pair.pivot().x, 1);
        assert_eq!(pair.child().x, 2);
    }

    /// Wedged between two columns with no room to turn either way: the first press is
    /// refused and the second flips the pair end over end, **in place** - the two halves
    /// swap cells rather than the pair moving anywhere.
    #[test]
    fn a_wedged_pair_quick_turns_on_the_second_press() {
        let mut wedge = Board::new();
        for y in 0..ROWS as i32 {
            wedge.set(Point::new(1, y), Some(PuyoCell::loose(PuyoColor::Green)));
            wedge.set(Point::new(3, y), Some(PuyoCell::loose(PuyoColor::Green)));
        }
        let mut pair = pair_at(2, 5);
        assert_eq!(pair.rotation(), Rotation::North);
        assert_eq!(
            pair.rotate(&wedge, true),
            RotateOutcome::Blocked,
            "no room either side"
        );
        assert_eq!(pair.rotation(), Rotation::North, "and it did not turn");
        assert_eq!(
            pair.rotate(&wedge, true),
            RotateOutcome::QuickTurned,
            "the second press flips it"
        );
        assert_eq!(pair.rotation(), Rotation::South);
        assert_eq!(
            pair.pivot(),
            Point::new(2, 4),
            "the pivot took the child's cell"
        );
        assert_eq!(
            pair.child(),
            Point::new(2, 5),
            "and the child took the pivot's"
        );
    }

    /// A quick turn cannot be refused, and cannot slide the pair anywhere.
    ///
    /// The two halves only ever swap the cells they are already standing on, so there is
    /// nothing left for it to collide with - which is what lets the game say that once the
    /// double tap has happened, "nothing will cancel the rotation".
    #[test]
    fn a_quick_turn_never_moves_the_pair_off_the_cells_it_holds() {
        // boxed in on all four sides: left, right, above and below
        let mut boxed_in = Board::new();
        for y in 0..ROWS as i32 {
            boxed_in.set(Point::new(1, y), Some(PuyoCell::loose(PuyoColor::Green)));
            boxed_in.set(Point::new(3, y), Some(PuyoCell::loose(PuyoColor::Green)));
        }
        boxed_in.set(Point::new(2, 3), Some(PuyoCell::loose(PuyoColor::Green)));
        boxed_in.set(Point::new(2, 6), Some(PuyoCell::loose(PuyoColor::Green)));

        let mut pair = pair_at(2, 5);
        let held = pair.points();
        assert_eq!(pair.rotate(&boxed_in, true), RotateOutcome::Blocked);
        assert_eq!(pair.rotate(&boxed_in, true), RotateOutcome::QuickTurned);
        let mut after = pair.points();
        after.sort_by_key(|p| p.y);
        let mut before = held;
        before.sort_by_key(|p| p.y);
        assert_eq!(after, before, "the pair holds the same two cells");
        assert_eq!(pair.pivot(), held[1], "with the halves the other way round");
    }

    /// a quick turn against the floor lifts the pair, the way a floor kick does
    #[test]
    fn a_quick_turn_on_the_floor_lifts_the_pair() {
        let mut wedge = Board::new();
        for y in 0..ROWS as i32 {
            wedge.set(Point::new(1, y), Some(PuyoCell::loose(PuyoColor::Green)));
            wedge.set(Point::new(3, y), Some(PuyoCell::loose(PuyoColor::Green)));
        }
        let floor = ROWS as i32 - 1;
        let mut pair = pair_at(2, floor);
        assert_eq!(pair.rotate(&wedge, true), RotateOutcome::Blocked);
        assert_eq!(pair.rotate(&wedge, true), RotateOutcome::QuickTurned);
        assert_eq!(pair.pivot(), Point::new(2, floor - 1), "lifted a row");
        assert_eq!(pair.child(), Point::new(2, floor));
    }

    /// A quick turn spends the arming: flipping again takes another two presses, so a single
    /// press can never flip a pair by surprise.
    #[test]
    fn a_quick_turn_spends_its_arming() {
        let mut wedge = Board::new();
        for y in 0..ROWS as i32 {
            wedge.set(Point::new(1, y), Some(PuyoCell::loose(PuyoColor::Green)));
            wedge.set(Point::new(3, y), Some(PuyoCell::loose(PuyoColor::Green)));
        }
        let mut pair = pair_at(2, 5);
        assert_eq!(pair.rotate(&wedge, true), RotateOutcome::Blocked);
        assert_eq!(pair.rotate(&wedge, true), RotateOutcome::QuickTurned);
        assert_eq!(
            pair.rotate(&wedge, true),
            RotateOutcome::Blocked,
            "the next press is refused again rather than flipping straight back"
        );
        assert_eq!(pair.rotate(&wedge, true), RotateOutcome::QuickTurned);
        assert_eq!(pair.rotation(), Rotation::North, "back where it started");
    }

    /// ... and a rotation that actually turns disarms it, so leaving the wedge and coming
    /// back needs two fresh presses
    #[test]
    fn turning_freely_disarms_the_quick_turn() {
        let mut wall = Board::new();
        for y in 0..ROWS as i32 {
            wall.set(Point::new(3, y), Some(PuyoCell::loose(PuyoColor::Green)));
        }
        let mut wedge = wall.clone();
        for y in 0..ROWS as i32 {
            wedge.set(Point::new(1, y), Some(PuyoCell::loose(PuyoColor::Green)));
        }

        let mut pair = pair_at(2, 5);
        assert_eq!(pair.rotate(&wedge, true), RotateOutcome::Blocked, "armed");
        // the column to the left goes away, and the pair can turn again
        assert_eq!(pair.rotate(&wall, true), RotateOutcome::Kicked);
        // back into the wedge: the first press is refused, not flipped
        let mut pair = Pair {
            rotation: Rotation::North,
            ..pair
        };
        assert_eq!(pair.rotate(&wedge, true), RotateOutcome::Blocked);
    }

    #[test]
    fn locking_lays_both_halves_down_where_they_are() {
        let mut board = Board::new();
        let pair = pair_at(2, 5);
        pair.lock(&mut board);
        assert_eq!(
            board.get(Point::new(2, 5)).unwrap().color(),
            Some(PuyoColor::Red)
        );
        assert_eq!(
            board.get(Point::new(2, 4)).unwrap().color(),
            Some(PuyoColor::Blue)
        );
    }

    /// the halves are laid down loose and settle independently, which is how a horizontal
    /// pair over a hole comes apart
    #[test]
    fn a_horizontal_pair_over_a_hole_splits() {
        let mut board = board(&[".r...."]);
        let mut pair = pair_at(0, ROWS as i32 - 2);
        pair.rotate(&board, true);
        assert_eq!(pair.rotation(), Rotation::East);
        pair.lock(&mut board);
        board.settle();
        let floor = ROWS as i32 - 1;
        // the pivot fell to the floor; the child stayed up on the stack beside it
        assert_eq!(
            board.get(Point::new(0, floor)).unwrap().color(),
            Some(PuyoColor::Red)
        );
        assert_eq!(
            board.get(Point::new(1, floor - 1)).unwrap().color(),
            Some(PuyoColor::Blue)
        );
    }

    #[test]
    fn a_pair_always_draws_unjoined() {
        use crate::game::cell::LinkMask;
        use engine::game::CellId;
        let board = board(&["rrrr.."]);
        let mut pair = pair_at(0, ROWS as i32 - 2);
        pair.rotate(&board, true);
        // resting right on top of a row of matching reds, and still joined to nothing
        for (_, id) in pair.cells() {
            assert_eq!(PuyoCell::from(id).links(), LinkMask::NONE);
        }
        assert_eq!(
            pair.cells()[0].1,
            CellId::from(PuyoCell::loose(PuyoColor::Red))
        );
    }

    /// Tsu has a ceiling above the thirteenth row.
    ///
    /// Puyo Nexus, *Special Maneuvers and Mechanics*: "the vanishing trick is not possible in
    /// games that use traditional Tsu physics because there is a ceiling above the 13th row
    /// that prevents rotation into the 14th row". Half of that is the board having no such row
    /// to turn into; the other half is the *current row check* in
    /// [Rotation, collision and push
    /// back](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back),
    /// which refuses an upright rotation outright when the pivot is in a ghost row rather than
    /// pushing the pair anywhere - so the player gets no free shove out of it either.
    #[test]
    fn there_is_a_ceiling_above_the_ghost_row() {
        let empty = Board::new();
        // a pair lying in the ghost row, child to its right
        let mut pair = pair_at(2, 0);
        assert_eq!(pair.rotate(&empty, true), RotateOutcome::Turned);
        assert_eq!(pair.rotation(), Rotation::East);
        assert_eq!(pair.child(), Point::new(3, 0));

        // turning the child back up would put it above the board, and up here that is simply
        // refused: no kick, and the pair stays where it is
        assert_eq!(pair.rotate(&empty, false), RotateOutcome::Blocked);
        assert_eq!(pair.rotation(), Rotation::East, "it did not turn");
        assert_eq!(pair.pivot(), Point::new(2, 0), "and it did not move");
    }

    /// ... and the same refusal downwards: the check is on the pivot's row and the *target*
    /// being upright, not on which way the push would have gone.
    #[test]
    fn a_pair_in_the_ghost_row_is_refused_an_upright_rotation_either_way() {
        // the ghost row is free but everything below it is not
        let mut full = Board::new();
        for x in 0..COLUMNS as i32 {
            for y in 1..ROWS as i32 {
                full.set(Point::new(x, y), Some(PuyoCell::loose(PuyoColor::Green)));
            }
        }
        // a pair lying flat in the ghost row: turning either half down is refused
        let mut pair = pair_at(2, 0);
        pair.rotate(&full, true);
        assert_eq!(pair.rotation(), Rotation::East, "sideways is still allowed");
        assert_eq!(pair.rotate(&full, true), RotateOutcome::Blocked);
        assert_eq!(pair.pivot(), Point::new(2, 0), "no floor kick up here");
        // and a refusal up here does not even arm the quick turn, so pressing again is
        // refused just the same rather than flipping the pair
        assert_eq!(pair.rotate(&full, true), RotateOutcome::Blocked);
        assert_eq!(pair.rotation(), Rotation::East);
    }

    /// a pair one row lower is an ordinary pair again, and kicks as one
    #[test]
    fn the_row_below_the_ghost_row_still_kicks_normally() {
        let mut full = Board::new();
        for x in 0..COLUMNS as i32 {
            for y in 2..ROWS as i32 {
                full.set(Point::new(x, y), Some(PuyoCell::loose(PuyoColor::Green)));
            }
        }
        let mut pair = pair_at(2, 1);
        pair.rotate(&full, true);
        assert_eq!(pair.rotation(), Rotation::East);
        assert_eq!(pair.rotate(&full, true), RotateOutcome::Kicked);
        assert_eq!(pair.pivot(), Point::new(2, 0), "floor kicked up a row");
        assert_eq!(pair.child(), Point::new(2, 1));
    }

    /// The halves join up to what they land beside the moment they are laid down.
    ///
    /// A pair that lands flat settles nothing and pops nothing, so nothing else in the chain
    /// loop would ever recompute the masks - and the joined look is the whole point of them.
    #[test]
    fn locking_joins_the_halves_to_what_they_land_beside() {
        use crate::game::cell::LinkMask;
        let mut board = board(&["r.....", "r....."]);
        // a red on top of a red pair, dropped into the same column
        let mut pair = Pair::new(
            Point::new(0, 0),
            PuyoPiece::new(PuyoColor::Red, PuyoColor::Blue),
        );
        pair.hard_drop(&board);
        pair.lock(&mut board);
        let floor = ROWS as i32 - 1;
        let links = |y: i32| board.get(Point::new(0, y)).unwrap().links();
        assert_eq!(
            links(floor),
            LinkMask::UP,
            "the red already there joined up"
        );
        assert_eq!(
            links(floor - 1),
            LinkMask::UP.with(LinkMask::DOWN),
            "and the one above it joined both ways"
        );
        assert_eq!(
            links(floor - 2),
            LinkMask::DOWN,
            "the pivot joined the reds it landed on"
        );
        assert_eq!(
            links(floor - 3),
            LinkMask::NONE,
            "and the blue half joined nothing"
        );
    }

    /// the arming is the pair's own, and survives being nudged about between presses
    #[test]
    fn the_quick_turn_arming_survives_moving_and_falling() {
        let mut wedge = Board::new();
        for y in 0..ROWS as i32 {
            wedge.set(Point::new(1, y), Some(PuyoCell::loose(PuyoColor::Green)));
            wedge.set(Point::new(3, y), Some(PuyoCell::loose(PuyoColor::Green)));
        }
        let mut pair = pair_at(2, 5);
        assert_eq!(pair.rotate(&wedge, true), RotateOutcome::Blocked);
        assert!(!pair.shift(&wedge, 1), "the wedge holds it in the column");
        assert!(pair.fall(&wedge), "but it can still drop");
        assert_eq!(
            pair.rotate(&wedge, true),
            RotateOutcome::QuickTurned,
            "and the second press still flips it"
        );
    }

    #[test]
    fn the_ghost_is_where_the_pair_would_land() {
        let board = board(&["rrrrrr"]);
        let pair = pair_at(2, 1);
        let ghost = pair.ghost(&board);
        assert_eq!(ghost.pivot().y, ROWS as i32 - 2);
        assert_eq!(pair.pivot().y, 1, "the pair itself did not move");
    }
}
