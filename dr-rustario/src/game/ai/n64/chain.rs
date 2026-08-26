//! Whether a placement leaves a chain behind: play the pill, let everything it clears go, let
//! what is loose fall, and look for a line that was not there before. Ported from
//! `aifRensaCheckCore`.
//!
//! Upstream runs this once for each colour the second half could have been, which is how the ai
//! decides between a placement that only pays off if the right colour turns up and one that
//! pays off whatever comes next.

use crate::game::ai::n64::field::{Field, BAD_LINE_RATE, COLS, ROWS, ST_CLEARING, ST_EMPTY};
use crate::game::ai::n64::Half;

/// Nothing, a chain, or a chain off a placement that cleared two lines at once and left
/// everything where it was.
pub struct Chain {
    /// 0 none, 1 a chain, 2 a double clear that nothing had to fall for
    pub strength: u8,
    /// how much weight this takes out of the top four rows. Original name: `aiHiEraseCtr`
    pub relieved: i32,
}

pub fn rensa_check_core(original: &Field, main: Half, second: Half) -> Chain {
    let mut field = *original;

    let mut relieved = top_weight(&field);

    if main.row != 0 {
        field.set(main.row, main.col, main.cell());
    }
    if second.row != 0 {
        field.set(second.row, second.col, second.cell());
    }

    let mut lines = 0;

    for row in 1..ROWS {
        let mut col = 0;
        while col < COLS - 3 {
            if field.at(row, col).is_empty() {
                col += 1;
                continue;
            }
            let run = run_right(&field, row, col);
            if run >= 4 {
                lines += run / 4;
                for cell in 0..run {
                    field.orphan_partner(row, col + cell);
                    field.set_st(row, col + cell, ST_CLEARING);
                }
            }
            col += run;
        }
    }

    for col in 0..COLS {
        let mut row = 1;
        while row < ROWS - 3 {
            if field.at(row, col).is_empty() {
                row += 1;
                continue;
            }
            let run = run_down(&field, row, col);
            if run >= 4 {
                lines += run / 4;
                for cell in 0..run {
                    field.orphan_partner(row + cell, col);
                    field.set_st(row + cell, col, ST_CLEARING);
                }
            }
            row += run;
        }
    }

    let settled = settle(&mut field);

    relieved -= top_weight(&field);

    if lines >= 2 {
        return Chain {
            strength: if settled { 2 } else { 1 },
            relieved,
        };
    }

    let chained = (1..ROWS).any(|row| (0..COLS - 3).any(|col| run_right(&field, row, col) >= 4))
        || (0..COLS).any(|col| (1..ROWS - 3).any(|row| run_down(&field, row, col) >= 4));

    Chain {
        strength: chained as u8,
        relieved,
    }
}

/// how much of the top four rows is filled, weighted by how much being there matters
fn top_weight(field: &Field) -> i32 {
    let mut weight = 0;
    for (row, rates) in BAD_LINE_RATE.iter().enumerate().skip(1) {
        for (col, rate) in rates.iter().enumerate() {
            if field.at(row, col).st < 8 {
                weight += rate;
            }
        }
    }
    weight
}

fn run_right(field: &Field, row: usize, col: usize) -> usize {
    if field.at(row, col).is_empty() {
        return 0;
    }
    let colour = field.at(row, col).co;
    let mut run = 1;
    while col + run < COLS
        && !field.at(row, col + run).is_empty()
        && field.at(row, col + run).co == colour
    {
        run += 1;
    }
    run
}

fn run_down(field: &Field, row: usize, col: usize) -> usize {
    if field.at(row, col).is_empty() {
        return 0;
    }
    let colour = field.at(row, col).co;
    let mut run = 1;
    while row + run < ROWS
        && !field.at(row + run, col).is_empty()
        && field.at(row + run, col).co == colour
    {
        run += 1;
    }
    run
}

/// Clear what was marked and drop what that left hanging. Returns whether everything stayed
/// put, which upstream reads as "this clear was self contained".
fn settle(field: &mut Field) -> bool {
    let mut moved = false;

    for row in (1..ROWS).rev() {
        for col in (0..COLS).rev() {
            match field.at(row, col).st {
                ST_CLEARING => field.set_st(row, col, ST_EMPTY),
                // a lone half falls on its own
                4 if row + 1 < ROWS && field.at(row + 1, col).is_empty() => {
                    moved = true;
                    let landing = drop_to(field, row, col);
                    field.set(landing, col, field.at(row, col));
                    field.set_st(row, col, ST_EMPTY);
                }
                // the bottom of an upright pill takes its top half with it
                1 if row + 1 < ROWS && field.at(row + 1, col).is_empty() => {
                    moved = true;
                    let landing = drop_to(field, row, col);
                    field.set(landing, col, field.at(row, col));
                    field.set_st(row, col, ST_EMPTY);
                    field.set(landing - 1, col, field.at(row - 1, col));
                    field.set_st(row - 1, col, ST_EMPTY);
                }
                // the left of a flat pill falls only if both columns are clear beneath it
                2 if row + 1 < ROWS
                    && col + 1 < COLS
                    && field.at(row + 1, col).is_empty()
                    && field.at(row + 1, col + 1).is_empty() =>
                {
                    moved = true;
                    let mut landing = row + 1;
                    while landing < ROWS
                        && field.at(landing, col).is_empty()
                        && field.at(landing, col + 1).is_empty()
                    {
                        landing += 1;
                    }
                    landing -= 1;
                    field.set(landing, col, field.at(row, col));
                    field.set_st(row, col, ST_EMPTY);
                    field.set(landing, col + 1, field.at(row, col + 1));
                    field.set_st(row, col + 1, ST_EMPTY);
                }
                _ => {}
            }
        }
    }

    !moved
}

fn drop_to(field: &Field, row: usize, col: usize) -> usize {
    let mut landing = row + 1;
    while landing < ROWS && field.at(landing, col).is_empty() {
        landing += 1;
    }
    landing - 1
}
