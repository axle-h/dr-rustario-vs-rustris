//! What chain the field is holding: for every reachable column and every colour on the board,
//! drop puyos of that colour until a group of four forms, then run the chain out on a probe.
//! Ama's quiescence search (`ai/search/beam/quiet.cpp`); the field is never touched.

use crate::game::ai::field::{Field, EMPTY, NUISANCE, VISIBLE, WIDTH};
use crate::game::score::PUYOS_TO_POP;

/// How many puyos a probe may drop before giving up on a column and colour: enough to complete
/// a four onto a single puyo, since four would say only that the column is empty.
pub const MAX_KEY_PUYOS: u32 = PUYOS_TO_POP - 1;

/// The shortest chain worth reporting; rewarding single pops would have the ai spend its board
/// on fours as fast as it builds them.
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

/// The columns a pair can still be brought over, outwards from the spawn column; a column
/// stacked into the ghost row walls off everything beyond it.
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

/// Every chain the field is holding, handed to `f` one at a time. Only colours already on the
/// field are probed.
pub fn search(field: &Field, f: impl FnMut(&Trigger)) {
    search_if(true, field, f)
}

/// [`search`], skipped entirely when `wanted` is false, since it is the most expensive part of
/// an evaluation.
pub fn search_if(wanted: bool, field: &Field, mut f: impl FnMut(&Trigger)) {
    if !wanted {
        return;
    }
    let heights = field.heights();
    let (min, max) = reachable_columns(&heights);
    let colors = colors_present(field);

    for (column, height) in heights.iter().enumerate().take(max + 1).skip(min) {
        // a puyo in the ghost row does not group, so only visible rows are room for keys
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
                // the probe is now the field those key puyos would leave; run the chain out
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

    /// a two step chain waiting on one red in column 0 is found
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

    /// the field the search was handed is left untouched
    #[test]
    fn probing_leaves_the_field_alone() {
        let field = Field::from_board(&board(&[".g....", "rg....", "rrgg.."]));
        let before = field;
        search(&field, |_| {});
        assert!(before == field);
    }

    /// a single pop is not reported as a chain
    #[test]
    fn a_lone_group_of_four_is_not_a_chain() {
        let found = triggers(&["rrr..."]);
        assert!(found.is_empty(), "got {found:?}");
    }

    /// a column stacked into the ghost row walls off everything past it
    #[test]
    fn a_column_full_to_the_top_walls_off_what_is_behind_it() {
        let mut heights = [0u8; WIDTH];
        heights[4] = VISIBLE as u8;
        assert_eq!(reachable_columns(&heights), (0, 3));
        heights[1] = VISIBLE as u8;
        assert_eq!(reachable_columns(&heights), (2, 3));
    }

    /// the empty board holds nothing
    #[test]
    fn an_empty_field_is_holding_nothing() {
        let field = Field::from_board(&board_rows(&[]));
        let mut found = 0;
        search(&field, |_| found += 1);
        assert_eq!(found, 0);
    }
}
