//! Where a pair can be put, and what putting it there does.
//!
//! There are two move generators here and they are not the same generator, deliberately.
//!
//! The **root** one replays real [`Pair`] moves against the real [`Board`], so the wall kicks,
//! the quick turn and the ghost row's rotation ban are honoured for free and the answer comes
//! with the keys to press. It is the one the agent executes, and it runs once per pair.
//!
//! The **search** one works on a [`Field`] and only names the two columns each half comes to
//! rest in. It runs tens of thousands of times per pair and cannot afford a board, a pair or a
//! route; what it loses is the handful of placements only a kick or a quick turn can reach, in
//! a layer of the search that is a guess about a pair nobody has been dealt yet.
//!
//! Both of them end in the same [`Drop`], which is what a placement actually *is* once the
//! keys have been pressed: two halves, each falling to the bottom of its own column.

use crate::game::ai::field::{of_color, Chain, Field, WIDTH};
use crate::game::ai::input_sequence::{InputSequence, Translation};
use crate::game::ai::quiet;
use crate::game::board::Board;
use crate::game::pair::Pair;
use engine::game::geometry::Point;
use std::collections::{HashSet, VecDeque};

/// the most abstract placements one pair has: six columns standing up either way round, and
/// five adjacent pairs of columns lying down either way round
pub const MAX_MOVES: usize = WIDTH * 2 + (WIDTH - 1) * 2;

/// A placement stripped to what it does: two halves, dropped in order.
///
/// The order matters only when both go into the same column - a pair standing up puts its
/// lower half down first - and the columns differing is what a tear is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Drop {
    pub columns: [usize; 2],
    pub cells: [u8; 2],
}

impl Drop {
    /// Two halves in two columns, in the order they land.
    ///
    /// A drop across two columns is put the same way round whichever rotation reached it -
    /// the leftmost column first - because the order only ever means anything when both
    /// halves go down the same column. Without that the two generators here would disagree
    /// about whether "red at 3, blue at 4" and "blue at 4, red at 3" are one placement.
    pub fn new(columns: [usize; 2], cells: [u8; 2]) -> Self {
        if columns[0] > columns[1] {
            Self {
                columns: [columns[1], columns[0]],
                cells: [cells[1], cells[0]],
            }
        } else {
            Self { columns, cells }
        }
    }

    /// Play this drop out on `field`: put both halves down and run the chain.
    ///
    /// `None` when a column had no room, which is a placement that cannot be made at all.
    pub fn apply(&self, field: &mut Field) -> Option<(u32, Chain)> {
        let tear = self.tear(field);
        field.drop_into(self.columns[0], self.cells[0])?;
        field.drop_into(self.columns[1], self.cells[1])?;
        Some((tear, field.resolve()))
    }

    /// How far the two halves come apart on the way down.
    ///
    /// A pair lying across two columns of different heights splits, and the higher half falls
    /// the rest of the way on its own - which costs time on the clock and, in a two player
    /// game, time is what an attack is made of. A pair standing up cannot tear.
    pub fn tear(&self, field: &Field) -> u32 {
        if self.columns[0] == self.columns[1] {
            return 0;
        }
        let a = field.height(self.columns[0]) as i32;
        let b = field.height(self.columns[1]) as i32;
        (a - b).unsigned_abs()
    }
}

/// Every abstract placement of `cells` on `field`, for the layers of the search below the
/// root. Returns how many were written into `out`.
pub fn moves(field: &Field, cells: [u8; 2], out: &mut [Drop; MAX_MOVES]) -> usize {
    let heights = field.heights();
    let (min, max) = quiet::reachable_columns(&heights);
    let doublet = cells[0] == cells[1];
    let mut n = 0;

    let mut push = |columns: [usize; 2], cells: [u8; 2], out: &mut [Drop; MAX_MOVES]| {
        out[n] = Drop::new(columns, cells);
        n += 1;
    };

    for x in min..=max {
        push([x, x], cells, out);
        if !doublet {
            push([x, x], [cells[1], cells[0]], out);
        }
    }
    for x in min..max {
        push([x, x + 1], cells, out);
        if !doublet {
            push([x, x + 1], [cells[1], cells[0]], out);
        }
    }
    n
}

/// A placement of the pair in play, with the keys that reach it.
#[derive(Clone, Debug)]
pub struct RootMove {
    pub inputs: InputSequence,
    pub drop: Drop,
}

const MOVES: [Translation; 4] = [
    Translation::Left,
    Translation::Right,
    Translation::RotateClockwise,
    Translation::RotateAnticlockwise,
];

/// where a pair is, closely enough that two routes to it are the same route. The quick turn is
/// part of it: a refused rotation leaves the pair where it was but arms the next press, and
/// the flip that press performs reaches a placement nothing else can.
type Pose = (i32, i32, i32, i32, bool);

fn pose(pair: &Pair, armed: bool) -> Pose {
    let [pivot, child] = pair.points();
    (pivot.x, pivot.y, child.x, child.y, armed)
}

/// what the pair would come to rest as, keyed so that two routes to one resting place collapse
fn landing(pair: &Pair, board: &Board) -> (Point, Point) {
    let landed = pair.ghost(board);
    let [pivot, child] = landed.points();
    (pivot, child)
}

fn drop_of(pair: &Pair, board: &Board) -> Drop {
    let landed = pair.ghost(board);
    let [pivot, child] = landed.points();
    let [pivot_color, child_color] = landed.colors();
    let (first, second) = if pivot.x == child.x && child.y < pivot.y {
        // standing up with the child above: the pivot is the half that lands first
        ((pivot, pivot_color), (child, child_color))
    } else if pivot.x == child.x {
        ((child, child_color), (pivot, pivot_color))
    } else {
        ((pivot, pivot_color), (child, child_color))
    };
    Drop::new(
        [first.0.x as usize, second.0.x as usize],
        [of_color(first.1), of_color(second.1)],
    )
}

/// Every placement the pair in play can be walked to, shortest route first.
///
/// Breadth first over the moves a player has, on a clone of the pair against the real board,
/// so what comes back is reachable rather than merely drawable. Several poses come to rest in
/// the same two cells - a kick can leave the pair a row lower in the same column - so
/// placements are keyed by where they land and the first route to each one wins, which is the
/// shortest.
pub fn root_moves(board: &Board, pair: Pair) -> Vec<RootMove> {
    let mut visited: HashSet<Pose> = HashSet::from([pose(&pair, false)]);
    let mut queue: VecDeque<(Pair, bool, InputSequence)> =
        VecDeque::from([(pair, false, InputSequence::default())]);
    let mut landings: HashSet<(Point, Point)> = HashSet::new();
    let mut found = vec![];

    while let Some((pair, armed, inputs)) = queue.pop_front() {
        if landings.insert(landing(&pair, board)) {
            found.push(RootMove {
                inputs: inputs.with(Translation::HardDrop),
                drop: drop_of(&pair, board),
            });
        }

        for translation in MOVES {
            let mut next = pair;
            let mut next_armed = false;
            let moved = match translation {
                Translation::Left => next.shift(board, -1),
                Translation::Right => next.shift(board, 1),
                Translation::RotateClockwise | Translation::RotateAnticlockwise => {
                    let clockwise = translation == Translation::RotateClockwise;
                    match next.rotate(board, clockwise) {
                        crate::game::pair::RotateOutcome::Blocked => {
                            // it did not turn, but the next press will flip it
                            next_armed = true;
                            !armed
                        }
                        _ => true,
                    }
                }
                Translation::HardDrop => unreachable!("not a move the search makes"),
            };
            if !moved {
                continue;
            }
            if visited.insert(pose(&next, next_armed)) {
                queue.push_back((next, next_armed, inputs.with(translation)));
            }
        }
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::board::tests::board;
    use crate::game::board::SPAWN;
    use crate::game::cell::{PuyoColor, PuyoPiece};

    fn pair(pivot: PuyoColor, child: PuyoColor) -> Pair {
        Pair::new(SPAWN, PuyoPiece::new(pivot, child))
    }

    /// an empty board offers every column standing up either way round and every adjacent
    /// pair of columns lying down either way round, and nothing twice
    #[test]
    fn an_open_board_offers_every_placement_once() {
        let board = board(&[]);
        let found = root_moves(&board, pair(PuyoColor::Red, PuyoColor::Blue));
        assert_eq!(found.len(), MAX_MOVES, "{} placements", found.len());
        let mut landings: Vec<_> = found.iter().map(|m| m.drop).collect();
        landings.sort_by_key(|d| (d.columns, d.cells));
        landings.dedup();
        assert_eq!(landings.len(), MAX_MOVES, "two routes to one placement");
    }

    /// a pair of one colour has half as many placements, because turning it round changes
    /// nothing about it
    #[test]
    fn a_doublet_has_half_as_many_placements() {
        let board = board(&[]);
        let found = root_moves(&board, pair(PuyoColor::Red, PuyoColor::Red));
        let mut drops: Vec<_> = found.iter().map(|m| m.drop).collect();
        drops.sort_by_key(|d| (d.columns, d.cells));
        drops.dedup();
        assert_eq!(drops.len(), MAX_MOVES / 2);
    }

    /// the search on a field agrees with the real one about what is on offer
    #[test]
    fn the_search_generator_offers_what_the_root_does() {
        let board = board(&[]);
        let field = Field::from_board(&board);
        let cells = [of_color(PuyoColor::Red), of_color(PuyoColor::Blue)];
        let mut out = [Drop::new([0, 0], cells); MAX_MOVES];
        let n = moves(&field, cells, &mut out);
        assert_eq!(n, MAX_MOVES);

        let mut theirs: Vec<_> = root_moves(&board, pair(PuyoColor::Red, PuyoColor::Blue))
            .iter()
            .map(|m| (m.drop.columns, m.drop.cells))
            .collect();
        let mut ours: Vec<_> = out[..n].iter().map(|d| (d.columns, d.cells)).collect();
        theirs.sort();
        ours.sort();
        assert_eq!(ours, theirs);
    }

    /// a column stacked to the ghost row is a wall the pair cannot be carried over
    #[test]
    fn a_walled_off_column_is_not_offered() {
        let mut rows = vec!["......"; 12];
        for row in rows.iter_mut() {
            *row = "....o.";
        }
        let board = board(&rows);
        let found = root_moves(&board, pair(PuyoColor::Red, PuyoColor::Blue));
        assert!(
            found.iter().all(|m| m.drop.columns.iter().all(|x| *x < 5)),
            "the far column is behind a wall"
        );
    }

    /// a pair lying across two columns of different heights comes apart on the way down
    #[test]
    fn a_pair_across_uneven_ground_tears() {
        let field = Field::from_board(&board(&["r.....", "r....."]));
        assert_eq!(Drop::new([2, 3], [1, 2]).tear(&field), 0);
        assert_eq!(Drop::new([0, 1], [1, 2]).tear(&field), 2);
        assert_eq!(
            Drop::new([0, 0], [1, 2]).tear(&field),
            0,
            "standing up, it cannot tear"
        );
    }
}
