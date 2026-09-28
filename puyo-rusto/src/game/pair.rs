//! The two-puyo piece the player controls: its colours, sprites and laying down. Movement and
//! rotation are [`engine::game::pair`]'s.

use crate::game::board::Board;
use crate::game::cell::{PuyoCell, PuyoColor, PuyoPiece, PuyoSkin};
use engine::game::geometry::{Point, Rotation};
use engine::game::pair::PairMotion;
use engine::game::PlacedCell;

pub use engine::game::pair::RotateOutcome;

/// The pair in play: where it is, and the two colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pair {
    motion: PairMotion,
    piece: PuyoPiece,
}

impl Pair {
    pub fn new(pivot: Point, piece: PuyoPiece) -> Self {
        Self {
            motion: PairMotion::new(pivot),
            piece,
        }
    }

    pub fn pivot(&self) -> Point {
        self.motion.pivot()
    }

    pub fn rotation(&self) -> Rotation {
        self.motion.rotation()
    }

    pub fn piece(&self) -> PuyoPiece {
        self.piece
    }

    pub fn child(&self) -> Point {
        self.motion.child()
    }

    pub fn points(&self) -> [Point; 2] {
        self.motion.points()
    }

    /// the two halves, pivot first, out of `skin`'s sprites; unlinked until the pair locks
    pub fn cells(&self, skin: PuyoSkin) -> Vec<PlacedCell> {
        vec![
            (self.pivot(), PuyoCell::loose(self.piece.pivot).id(skin)),
            (self.child(), PuyoCell::loose(self.piece.child).id(skin)),
        ]
    }

    /// the colour of each half, pivot first
    pub fn colors(&self) -> [PuyoColor; 2] {
        [self.piece.pivot, self.piece.child]
    }

    /// slide sideways, if there is room for both halves
    pub fn shift(&mut self, board: &Board, dx: i32) -> bool {
        self.motion.shift(board, dx)
    }

    /// step down one row, if there is room
    pub fn fall(&mut self, board: &Board) -> bool {
        self.motion.fall(board)
    }

    /// nothing below either half: the pair is about to lock
    pub fn is_resting(&self, board: &Board) -> bool {
        self.motion.is_resting(board)
    }

    /// fall as far as the pair will go, returning how many rows it dropped
    pub fn hard_drop(&mut self, board: &Board) -> u32 {
        self.motion.hard_drop(board)
    }

    /// where the pair would come to rest, for the ghost
    pub fn ghost(&self, board: &Board) -> Pair {
        Pair {
            motion: self.motion.ghost(board),
            ..*self
        }
    }

    /// Turn a quarter, kicking off the floor or a wall if it must. A refused rotation arms the
    /// quick turn, so a second press flips the pair.
    pub fn rotate(&mut self, board: &Board, clockwise: bool) -> RotateOutcome {
        self.motion.rotate(board, clockwise)
    }

    /// Put both halves on the board where they lie, for [`Board::settle`] to split them. It
    /// recomputes the link masks itself, since a pair landing flat settles and pops nothing.
    pub fn lock(&self, board: &mut Board) {
        board.set(self.pivot(), Some(PuyoCell::loose(self.piece.pivot)));
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
        let empty = Board::new(PuyoSkin::FIRST);
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
        let empty = Board::new(PuyoSkin::FIRST);
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
        let empty = Board::new(PuyoSkin::FIRST);
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
        let empty = Board::new(PuyoSkin::FIRST);
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
        let mut wall = Board::new(PuyoSkin::FIRST);
        for y in 0..ROWS as i32 {
            wall.set(Point::new(3, y), Some(PuyoCell::loose(PuyoColor::Green)));
        }
        let mut pair = pair_at(2, 5);
        assert_eq!(pair.rotate(&wall, true), RotateOutcome::Kicked);
        assert_eq!(pair.pivot().x, 1);
        assert_eq!(pair.child().x, 2);
    }

    /// wedged between two columns, the first press is refused and the second swaps the halves
    /// in place
    #[test]
    fn a_wedged_pair_quick_turns_on_the_second_press() {
        let mut wedge = Board::new(PuyoSkin::FIRST);
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

    /// a quick turn cannot be refused or move the pair, since the halves only swap cells
    #[test]
    fn a_quick_turn_never_moves_the_pair_off_the_cells_it_holds() {
        // boxed in on all four sides: left, right, above and below
        let mut boxed_in = Board::new(PuyoSkin::FIRST);
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
        let mut wedge = Board::new(PuyoSkin::FIRST);
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

    /// a quick turn spends the arming, so flipping again takes two more presses
    #[test]
    fn a_quick_turn_spends_its_arming() {
        let mut wedge = Board::new(PuyoSkin::FIRST);
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

    /// ... and a rotation that turns disarms it
    #[test]
    fn turning_freely_disarms_the_quick_turn() {
        let mut wall = Board::new(PuyoSkin::FIRST);
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
        let mut pair = pair_at(pair.pivot().x, pair.pivot().y);
        assert_eq!(pair.rotate(&wedge, true), RotateOutcome::Blocked);
    }

    #[test]
    fn locking_lays_both_halves_down_where_they_are() {
        let mut board = Board::new(PuyoSkin::FIRST);
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

    /// the halves settle independently, so a horizontal pair over a hole comes apart
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
        let board = board(&["rrrr.."]);
        let mut pair = pair_at(0, ROWS as i32 - 2);
        pair.rotate(&board, true);
        // resting right on top of a row of matching reds, and still joined to nothing
        for (_, id) in pair.cells(PuyoSkin::FIRST) {
            assert_eq!(PuyoCell::from(id).links(), LinkMask::NONE);
        }
        assert_eq!(
            pair.cells(PuyoSkin::FIRST)[0].1,
            PuyoCell::loose(PuyoColor::Red).id(PuyoSkin::FIRST)
        );
    }

    /// Tsu has a ceiling above the thirteenth row: an upright rotation with the pivot in the
    /// ghost row is refused outright, with no kick. Puyo Nexus,
    /// [Rotation, collision and push back](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back).
    #[test]
    fn there_is_a_ceiling_above_the_ghost_row() {
        let empty = Board::new(PuyoSkin::FIRST);
        // a pair lying in the ghost row, child to its right
        let mut pair = pair_at(2, 0);
        assert_eq!(pair.rotate(&empty, true), RotateOutcome::Turned);
        assert_eq!(pair.rotation(), Rotation::East);
        assert_eq!(pair.child(), Point::new(3, 0));

        // turning the child back up is refused: no kick, and the pair stays put
        assert_eq!(pair.rotate(&empty, false), RotateOutcome::Blocked);
        assert_eq!(pair.rotation(), Rotation::East, "it did not turn");
        assert_eq!(pair.pivot(), Point::new(2, 0), "and it did not move");
    }

    /// ... and downwards too: the check is on the pivot's row and an upright target
    #[test]
    fn a_pair_in_the_ghost_row_is_refused_an_upright_rotation_either_way() {
        // the ghost row is free but everything below it is not
        let mut full = Board::new(PuyoSkin::FIRST);
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
        // a refusal up here does not arm the quick turn either
        assert_eq!(pair.rotate(&full, true), RotateOutcome::Blocked);
        assert_eq!(pair.rotation(), Rotation::East);
    }

    /// a pair one row lower is an ordinary pair again, and kicks as one
    #[test]
    fn the_row_below_the_ghost_row_still_kicks_normally() {
        let mut full = Board::new(PuyoSkin::FIRST);
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

    /// the halves join up to what they land beside the moment they are laid down
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

    /// the arming survives being nudged about between presses
    #[test]
    fn the_quick_turn_arming_survives_moving_and_falling() {
        let mut wedge = Board::new(PuyoSkin::FIRST);
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
