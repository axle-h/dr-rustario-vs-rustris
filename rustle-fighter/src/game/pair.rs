//! The two-gem piece the player controls.
//!
//! Thin, and deliberately so: the movement is [`engine::game::pair`]'s, which is **Puyo
//! Puyo's** rotation rather than this game's own. That is a decision, not an oversight - the
//! PlayStation game has no wall kick, no quick turn and no escape at all from being wedged
//! between two columns, and the rules doc's *Deliberate deviations* records what we traded
//! away. What is left here is the two halves and laying them down.

use crate::game::board::Board;
use crate::game::cell::{GemPair, Half};
use engine::game::geometry::{Point, Rotation};
use engine::game::pair::PairMotion;

pub use engine::game::pair::RotateOutcome;

/// The pair in play: where it is, and the two halves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pair {
    motion: PairMotion,
    piece: GemPair,
}

impl Pair {
    pub fn new(pivot: Point, piece: GemPair) -> Pair {
        Pair {
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

    pub fn piece(&self) -> GemPair {
        self.piece
    }

    pub fn child(&self) -> Point {
        self.motion.child()
    }

    pub fn points(&self) -> [Point; 2] {
        self.motion.points()
    }

    /// the two halves, pivot first
    pub fn halves(&self) -> [Half; 2] {
        [self.piece.pivot, self.piece.child]
    }

    pub fn shift(&mut self, board: &Board, dx: i32) -> bool {
        self.motion.shift(board, dx)
    }

    pub fn fall(&mut self, board: &Board) -> bool {
        self.motion.fall(board)
    }

    pub fn is_resting(&self, board: &Board) -> bool {
        self.motion.is_resting(board)
    }

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

    pub fn rotate(&mut self, board: &Board, clockwise: bool) -> RotateOutcome {
        self.motion.rotate(board, clockwise)
    }

    /// Put both halves on the board where they lie, and leave them to gravity.
    ///
    /// A horizontal pair over a hole drops one half further than the other, which is the
    /// splitting every game of this shape does; settling is [`Board::settle`]'s job.
    pub fn lock(&self, board: &mut Board) {
        board.set(self.pivot(), Some(self.piece.pivot.gem()));
        board.set(self.child(), Some(self.piece.child.gem()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::board::tests::board;
    use crate::game::board::{is_headroom, ROWS, SPAWN};
    use crate::game::cell::{Gem, GemColor};

    fn pair_at(x: i32, y: i32) -> Pair {
        Pair::new(
            Point::new(x, y),
            GemPair::new(Half::Plain(GemColor::Red), Half::Crash(GemColor::Blue)),
        )
    }

    #[test]
    fn a_pair_enters_standing_up_over_the_drop_alley() {
        let pair = pair_at(SPAWN.x, SPAWN.y);
        assert_eq!(pair.rotation(), Rotation::North);
        assert_eq!(pair.child(), SPAWN.translate(0, -1));
        assert!(is_headroom(pair.child()), "with its child in the headroom");
    }

    /// the rotation is the engine's, and the engine's is Puyo's - so the pair kicks off a wall
    /// where the PlayStation game would simply refuse
    #[test]
    fn the_pair_kicks_off_a_wall_the_way_puyo_does() {
        let empty = Board::new();
        let mut pair = pair_at(crate::game::board::COLUMNS as i32 - 1, 5);
        assert_eq!(pair.rotate(&empty, true), RotateOutcome::Kicked);
        assert_eq!(pair.pivot().x, crate::game::board::COLUMNS as i32 - 2);
    }

    /// ... and quick turns out of a wedge, which the PlayStation game has no escape from at all
    #[test]
    fn a_wedged_pair_quick_turns() {
        let mut wedge = Board::new();
        for y in 0..ROWS as i32 {
            wedge.set(Point::new(2, y), Some(Gem::plain(GemColor::Green)));
            wedge.set(Point::new(4, y), Some(Gem::plain(GemColor::Green)));
        }
        let mut pair = pair_at(3, 5);
        assert_eq!(pair.rotate(&wedge, true), RotateOutcome::Blocked);
        assert_eq!(pair.rotate(&wedge, true), RotateOutcome::QuickTurned);
    }

    /// the headroom row is the ceiling: an upright rotation up there is refused rather than
    /// kicked, and does not arm the quick turn either
    #[test]
    fn the_headroom_row_refuses_an_upright_rotation() {
        let mut full = Board::new();
        for point in Board::points().filter(|p| !is_headroom(*p)) {
            full.set(point, Some(Gem::plain(GemColor::Green)));
        }
        let mut pair = pair_at(3, 0);
        assert_eq!(pair.rotate(&full, true), RotateOutcome::Turned, "sideways");
        assert_eq!(pair.rotate(&full, true), RotateOutcome::Blocked);
        assert_eq!(
            pair.rotate(&full, true),
            RotateOutcome::Blocked,
            "and again"
        );
    }

    /// the halves are laid down where they lie and settle independently, so a flat pair over a
    /// hole comes apart
    #[test]
    fn a_flat_pair_over_a_hole_splits() {
        let mut board = board(&[".r...."]);
        let mut pair = pair_at(0, ROWS as i32 - 2);
        assert_eq!(pair.rotate(&board, true), RotateOutcome::Turned);
        pair.lock(&mut board);
        board.settle();
        let floor = ROWS as i32 - 1;
        assert_eq!(
            board.get(Point::new(0, floor)),
            Some(Gem::plain(GemColor::Red)),
            "the pivot fell to the floor"
        );
        assert_eq!(
            board.get(Point::new(1, floor - 1)),
            Some(Gem::Crash(GemColor::Blue)),
            "and the crash half stayed up on the stack beside it"
        );
    }
}
