//! What the field could fire if it were asked to.
//!
//! This is the one idea that separates a Puyo bot that plays from one that only tidies. A
//! placement's own chain is easy to see - drop the pair and run the loop - but a *building*
//! player almost never fires anything, so scoring the chain a placement makes says nothing
//! about nearly every placement on offer. What matters is the chain the field is holding: how
//! long it would run if one more puyo were dropped on it, where that puyo would have to go,
//! and what it would cost to get there.
//!
//! So: for every column a pair can still be moved over, and every colour already on the board,
//! drop puyos of that colour one at a time until a group of four forms, then run the chain out
//! and report it. Nothing is committed - each probe is thrown away - and the field itself is
//! never touched.
//!
//! The technique is ama's (`ai/search/beam/quiet.cpp`), which calls it a quiescence search
//! after the chess idea it is named for: do not evaluate a position while something is still
//! about to happen in it.

use crate::game::ai::field::{Field, EMPTY, NUISANCE, VISIBLE, WIDTH};
use crate::game::score::PUYOS_TO_POP;

/// How many puyos a probe may drop before giving up on a column and colour.
///
/// Three is what it takes to complete a group of four onto a single puyo. Asking for four
/// would be asking what an empty column could hold, which is true of every empty column and
/// therefore says nothing.
pub const MAX_KEY_PUYOS: u32 = PUYOS_TO_POP - 1;

/// The shortest chain worth reporting.
///
/// A single pop is not a chain, it is a clear, and rewarding one would have the ai spend its
/// board on fours as fast as it can build them. Two steps is where building starts to pay.
pub const MIN_CHAIN: u32 = 2;

/// A chain the field is holding: what it would run to, and what it would take to set off.
#[derive(Clone, Copy)]
pub struct Trigger {
    /// how many steps it would run to
    pub chain: u32,
    /// what the game would score it
    pub score: u32,
    /// the column the key puyos would have to be dropped into
    pub column: usize,
    /// how many puyos of one colour it takes to set it off
    pub key: u32,
    /// the field left standing once it has fired
    pub remain: Field,
}

/// The columns a pair can still be brought over, outwards from the spawn column.
///
/// A pair moves sideways across the top of the board, so a column stacked into the ghost row
/// is a wall: everything beyond it is out of reach however empty it looks. Counting from the
/// spawn column outwards in both directions is what turns that into a range.
pub fn reachable_columns(heights: &[u8; WIDTH]) -> (usize, usize) {
    let spawn = crate::game::ai::field::SPAWN_COLUMN;
    let mut min = spawn;
    let mut max = spawn;
    for (x, height) in heights.iter().enumerate().skip(spawn + 1) {
        if *height as usize >= VISIBLE {
            break;
        }
        max = x;
    }
    for (x, height) in heights[..spawn].iter().enumerate().rev() {
        if *height as usize >= VISIBLE {
            break;
        }
        min = x;
    }
    (min, max)
}

/// which colours are on the field at all, as one bit each
fn colors_present(field: &Field) -> u8 {
    let mut bits = 0u8;
    for y in 0..crate::game::ai::field::HEIGHT {
        for x in 0..WIDTH {
            let cell = field.get(x, y);
            if cell != EMPTY && cell != NUISANCE {
                bits |= 1 << (cell - 1);
            }
        }
    }
    bits
}

/// Every chain the field is holding, handed to `f` one at a time.
///
/// Colours are taken from the field rather than from the match's palette, because a colour
/// with nothing on the board cannot reach four inside [`MAX_KEY_PUYOS`] drops anyway - so
/// probing for it is work that can only ever come back empty.
pub fn search(field: &Field, f: impl FnMut(&Trigger)) {
    search_if(true, field, f)
}

/// [`search`], skipped entirely when the caller has already decided it does not care what
/// comes back - which is not the same as calling it and ignoring the answer, since this is
/// the most expensive thing an evaluation does.
pub fn search_if(wanted: bool, field: &Field, mut f: impl FnMut(&Trigger)) {
    if !wanted {
        return;
    }
    let heights = field.heights();
    let (min, max) = reachable_columns(&heights);
    let colors = colors_present(field);

    for (column, height) in heights.iter().enumerate().take(max + 1).skip(min) {
        // the ghost row cannot be built in: a puyo up there does not group, so a key puyo
        // that lands in it is a wasted probe rather than a trigger
        let room = VISIBLE.saturating_sub(*height as usize) as u32;
        let drops = MAX_KEY_PUYOS.min(room);
        if drops == 0 {
            continue;
        }

        for index in 0..crate::game::cell::PuyoColor::N {
            if colors & (1 << index) == 0 {
                continue;
            }
            let cell = index as u8 + 1;
            let mut probe = *field;
            for key in 1..=drops {
                let Some(y) = probe.drop_into(column, cell) else {
                    break;
                };
                if !probe.has_group_of(column, y, PUYOS_TO_POP) {
                    continue;
                }
                // it goes: run the chain out on the probe, which is now exactly the field
                // those key puyos would have left
                let chain = probe.resolve();
                if chain.count >= MIN_CHAIN {
                    f(&Trigger {
                        chain: chain.count,
                        score: chain.score,
                        column,
                        key,
                        remain: probe,
                    });
                }
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ai::field::Field;
    use crate::game::board::tests::{board, board_rows};

    fn triggers(rows: &[&str]) -> Vec<(u32, usize, u32)> {
        let field = Field::from_board(&board(rows));
        let mut found = vec![];
        search(&field, |t| found.push((t.chain, t.column, t.key)));
        found
    }

    /// a two step chain waiting on one red in column 0, which is exactly what a builder wants
    /// to be told and what nothing on the board says by itself
    #[test]
    fn a_chain_waiting_on_one_puyo_is_found_and_priced() {
        let found = triggers(&[".g....", "rg....", "rrgg.."]);
        assert!(
            found
                .iter()
                .any(|(chain, column, key)| *chain >= 2 && *column == 0 && *key == 1),
            "expected a one-puyo trigger in column 0, got {found:?}"
        );
    }

    /// nothing is committed: the field the search was handed is the field it leaves behind
    #[test]
    fn probing_leaves_the_field_alone() {
        let field = Field::from_board(&board(&[".g....", "rg....", "rrgg.."]));
        let before = field;
        search(&field, |_| {});
        assert!(before == field);
    }

    /// a single pop is not a chain and is not reported, or the ai would spend the board on
    /// fours as fast as it could build them
    #[test]
    fn a_lone_group_of_four_is_not_a_chain() {
        let found = triggers(&["rrr..."]);
        assert!(found.is_empty(), "got {found:?}");
    }

    /// a column stacked into the ghost row is a wall, and everything past it is out of reach
    #[test]
    fn a_column_full_to_the_top_walls_off_what_is_behind_it() {
        let mut heights = [0u8; WIDTH];
        heights[4] = VISIBLE as u8;
        assert_eq!(reachable_columns(&heights), (0, 3));
        heights[1] = VISIBLE as u8;
        assert_eq!(reachable_columns(&heights), (2, 3));
    }

    /// the empty board holds nothing, and says so without probing every colour in the game
    #[test]
    fn an_empty_field_is_holding_nothing() {
        let field = Field::from_board(&board_rows(&[]));
        let mut found = 0;
        search(&field, |_| found += 1);
        assert_eq!(found, 0);
    }
}
