//! What one placement is worth, ported from `aifSearchLineMS` and the handful of routines it
//! leans on: `aifSearchLineCore`, `aifEraseLineCore`, `aifMiniPointK3` and the two
//! `aifMiniAloneCapNumber` counts.
//!
//! The shape of it is: drop the two halves into a copy of the bottle, measure the run each one
//! lands in, take away anything that clears, measure what is left, and turn those measurements
//! into a number with the weights [`Params`] holds. A "run" is measured twice over - once as
//! the cells actually touching, which is what clears, and once as the cells within reach
//! counting the gaps, which is what a line could still become - and it is the second that makes
//! this ai build towards clears rather than only taking the ones in front of it.

use crate::game::ai::n64::field::{
    Cell, Field, BAD_LINE_RATE, COLS, CO_EMPTY, ROWS, ST_SINGLE, ST_VIRUS,
};
use crate::game::ai::n64::params::{Params, BAD_POINT, BAD_POINT2, WALL_RATE};
use crate::game::ai::n64::Candidate;

/// One pass of [`search_line_core`]: what the run through a cell looks like along the column
/// (`hei`) and along the row (`wid`). Original names: `hei_data` and `wid_data`.
///
/// The entries the score reads are `[0]` how many lines this makes, `[1]` how many viruses the
/// clear takes, `[2]` how many cells are actually touching, `[3]` how many are within reach,
/// `[4]` how many of those are viruses, `[5]` how long the line could become, and `[9]` set
/// when the other axis made the line instead. `[7]` and `[8]` weigh a run in the top rows, and
/// are counted the way the original counts them even though it never spends them.
#[derive(Clone, Copy, Default)]
pub struct LineData {
    pub hei: [i32; 10],
    pub wid: [i32; 10],
}

/// The bottle's own reading of one candidate. Original name: `struct_aiFlag`.
pub struct Flag {
    pub pri: i32,
    pub tory: u8,
    hei: [[i32; 10]; 2],
    wid: [[i32; 10]; 2],
    elin: [i32; 2],
    only: [usize; 2],
    wonly: [usize; 2],
    /// the second half had to fall before it could be measured, so what it found counts for less
    sub: bool,
}

impl Flag {
    pub fn new(tory: u8) -> Self {
        Self {
            pri: 0,
            tory,
            hei: [[0; 10]; 2],
            wid: [[0; 10]; 2],
            elin: [0; 2],
            only: [0; 2],
            wonly: [0; 2],
            sub: false,
        }
    }
}

/// Score `flag`'s placement, which has already been written into `field`. `original` is the
/// bottle before the pill, `(mx, my, mco)` the half that has something under it and
/// `(sx, sy, sco)` the other one, `ec` says the two halves are the same colour and `wall` which
/// side of the bottle is stacked up.
///
/// Returns 1 if the first half made a line, 2 if the second did and 0 if neither, which is what
/// the chain check upstream needs to know. Original name: `aifSearchLineMS`.
#[allow(clippy::too_many_arguments)]
pub fn search_line_ms(
    flag: &mut Flag,
    field: &mut Field,
    original: &Field,
    cands: &[Candidate],
    p: &Params,
    (mx, my, mco): (usize, usize, u8),
    (sx, sy, sco): (usize, usize, u8),
    ec: bool,
    wall: usize,
) -> u8 {
    // where the halves started, which the height penalties below are still measured from
    let main_row = my;
    let second_start = sy;
    let mut sy = sy;

    let mut data = LineData::default();
    let made_main = search_line_core(field, &mut data, mx, my, 0, cands);
    let mut made_second = false;

    erase_line_core(field, &mut data, mx, my);

    if !made_main {
        flag.only[0] = alone_cap_number(field, &data, mx, my, false, ec);
        flag.wonly[0] = alone_cap_number_w(field, &data, mx, my, false, ec);
    }
    flag.hei[0] = data.hei;
    flag.wid[0] = data.wid;

    // the second half is only worth measuring if the first half's clear did not take it
    if !field.at(sy, sx).is_empty() {
        // two halves of one colour lie in the same line, and it has already been counted
        let skip = if ec {
            if flag.tory == 0 {
                2
            } else {
                1
            }
        } else {
            0
        };
        let mut fell = false;
        made_second = search_line_core(field, &mut data, sx, sy, skip, cands);

        if !made_second {
            if made_main {
                // the clear left a hole under this column: whatever is loose settles into it
                for row in (sy + 1..ROWS).rev() {
                    if field.at(row, sx).st != ST_SINGLE {
                        continue;
                    }
                    let mut landing = row + 1;
                    while landing < ROWS && field.at(landing, sx).is_empty() {
                        landing += 1;
                    }
                    if landing != row + 1 {
                        let co = field.at(row, sx).co;
                        field.set(landing - 1, sx, Cell::new(ST_SINGLE, co));
                        field.clear(row, sx);
                    }
                }
            }

            let mut landing = sy + 1;
            while landing < ROWS && field.at(landing, sx).is_empty() {
                landing += 1;
            }
            if landing != sy + 1 {
                let co = field.at(sy, sx).co;
                field.set(landing - 1, sx, Cell::new(ST_SINGLE, co));
                field.clear(sy, sx);
                fell = true;
                sy = landing - 1;
                if !made_main {
                    flag.sub = true;
                }
                made_second = search_line_core(field, &mut data, sx, sy, 0, cands);
            }
        }

        erase_line_core(field, &mut data, sx, sy);

        if !made_second {
            flag.only[1] = alone_cap_number(field, &data, sx, sy, fell, ec);
            flag.wonly[1] = alone_cap_number_w(field, &data, sx, sy, fell, ec);
        }
        flag.hei[1] = data.hei;
        flag.wid[1] = data.wid;
    }

    let sub = flag.sub;
    let other = sub as usize;
    let (points, elin) = mini_point_k3(&flag.hei[0], false, true, flag.tory, ec, p);
    flag.pri += points;
    flag.elin[0] += elin;
    let (points, elin) = mini_point_k3(&flag.hei[1], sub, true, flag.tory, ec, p);
    flag.pri += points;
    flag.elin[other] += elin;
    let (points, elin) = mini_point_k3(&flag.wid[0], false, false, flag.tory, ec, p);
    flag.pri += points;
    flag.elin[0] += elin;
    let (points, elin) = mini_point_k3(&flag.wid[1], sub, false, flag.tory, ec, p);
    flag.pri += points;
    flag.elin[other] += elin;

    if p.alone_cap_p[flag.only[0]] != 0 {
        flag.pri += p.alone_cap_p[flag.only[0]];
    }
    if p.alone_cap_p[flag.only[1]] != 0 {
        flag.pri += p.alone_cap_p[flag.only[1]];
    }
    if p.alone_cap_p[flag.only[0]] != 0 && p.alone_cap_p[flag.only[1]] != 0 {
        // stranding both halves is worse the higher up it happens
        flag.pri -= (0x11 - main_row as i32) * p.l_pri_p;
    }
    // the original tests one table and spends the other's entry, which is what it plays like
    if p.alone_cap_wp[flag.wonly[0]] != 0 {
        flag.pri += p.alone_cap_wp[flag.only[0]];
    }
    if p.alone_cap_wp[flag.wonly[1]] != 0 {
        flag.pri += p.alone_cap_wp[flag.only[1]];
    }

    let lines = (flag.hei[0][0] + flag.hei[1][0]) as usize;
    flag.pri += (p.erase_lin_p[lines.min(8)] as f32 * p.hei_erase_lin_rate) as i32;
    let lines = (flag.wid[0][0] + flag.wid[1][0]) as usize;
    flag.pri += (p.erase_lin_p[lines.min(8)] as f32 * p.wid_erase_lin_rate) as i32;

    if p.on_virus_p != 0 && main_row < 0x10 {
        on_virus(
            flag,
            original,
            p,
            (mx, main_row, mco),
            (sx, second_start, sco),
            ec,
            made_main,
        );
    }

    // how high the stack reaches in each column, as the row above the topmost block. A column
    // with nothing in it never gets a value in the original; 17 is what it means.
    let mut surface = [0x11usize; COLS];
    for (col, top) in surface.iter_mut().enumerate() {
        for row in 1..ROWS {
            if !field.at(row, col).is_empty() {
                *top = row - 1;
                break;
            }
        }
    }

    let mut dead = 0;
    let mut second_is_high = false;
    if surface[sx] < 4
        && sy < 5
        && sy > 0
        && !field.at(sy, sx).is_empty()
        && flag.hei[1][2] + (surface[sx] as i32) < 4
    {
        dead += BAD_POINT[sx] / (sy as i32 * 2 - 1);
        second_is_high = true;
    }
    if surface[mx] < 4
        && main_row < 4
        && main_row > 0
        && (!ec || flag.tory != 0)
        && !field.at(main_row, mx).is_empty()
        && flag.hei[0][2] + (surface[mx] as i32) < 4
    {
        if second_is_high {
            dead += BAD_POINT2[mx] + BAD_POINT2[sx] - BAD_POINT[sx] / (sy as i32 * 2 - 1);
        } else {
            dead += BAD_POINT[mx] / (main_row as i32 * 2 - 1);
        }
    }
    flag.pri += dead;

    if wall != 0 {
        let rate = WALL_RATE[wall][mx].max(WALL_RATE[wall][sx]);
        flag.pri = flag.pri * rate / 10;
    }

    if made_main {
        1
    } else if made_second {
        2
    } else {
        0
    }
}

/// What covering a virus is worth. The ai will happily bury one to complete a line, but hates
/// dropping a half on top of a virus for nothing, and the tangle of cases below is it working
/// out which of the two this is. Split out of `aifSearchLineMS`, whose locals it keeps.
#[allow(clippy::too_many_arguments)]
fn on_virus(
    flag: &mut Flag,
    original: &Field,
    p: &Params,
    (mx, my, mco): (usize, usize, u8),
    (sx, sy, sco): (usize, usize, u8),
    ec: bool,
    made_main: bool,
) {
    // does this column have a virus under the half and none above it? Only then is the half
    // sitting *on* something worth clearing rather than in the middle of a heap.
    let over_virus = |col: usize, row: usize| {
        for above in (4..row).rev() {
            if original.at(above, col).is_virus() {
                return false;
            }
        }
        (row + 1..ROWS).any(|below| original.at(below, col).is_virus())
    };
    let main_over = over_virus(mx, my);
    let second_over = over_virus(sx, sy);

    let hei = flag.hei;
    let reward = p.on_virus_p;
    let cost = p.on_virus_p * 2;

    if original.below(my, mx).st < 8 {
        if main_over {
            let touching = if ec && flag.tory == 0 { 3 } else { 2 };
            if hei[0][2] >= touching && hei[0][5] >= 4 {
                if flag.tory == 0 {
                    if ec {
                        flag.pri += reward * 2;
                    } else if !made_main {
                        flag.pri -= cost;
                    }
                } else if !second_over || (hei[1][3] >= 2 && hei[1][5] >= 4) {
                    flag.pri += reward;
                } else {
                    flag.pri -= cost;
                }
            } else {
                flag.pri -= cost;
            }
        } else if second_over {
            if hei[1][3] >= 2 && hei[1][5] >= 4 && hei[0][2] >= 3 && hei[0][5] >= 4 {
                flag.pri += reward;
            } else {
                flag.pri -= cost;
            }
        }
    } else if second_over {
        if hei[1][2] >= 2 && hei[1][5] >= 4 {
            if !main_over || (hei[0][3] >= 2 && hei[0][5] >= 4) {
                flag.pri += reward;
            } else {
                flag.pri -= cost;
            }
        } else {
            flag.pri -= cost;
        }
    } else if main_over {
        if hei[0][3] >= 2 && hei[0][5] >= 4 && hei[1][2] >= 3 && hei[1][5] >= 4 {
            flag.pri += reward;
        } else {
            flag.pri -= cost;
        }
    }

    // an upright pill of two colours that made a line: whatever is left underneath had better
    // match the half that stays behind
    if made_main && !ec && flag.tory == 0 && my < 0x10 {
        let mut different = ROWS;
        for row in my + 1..ROWS {
            let cell = original.at(row, sx);
            if cell.st >= 8 || cell.co != mco {
                different = row;
                break;
            }
        }
        if different < ROWS && (different..ROWS).any(|row| original.at(row, sx).is_virus()) {
            for row in different..ROWS {
                let cell = original.at(row, sx);
                if cell.st < 8 {
                    if cell.co == sco {
                        flag.pri += reward * 4;
                    } else {
                        flag.pri -= cost;
                    }
                    break;
                }
            }
        }
    }
}

/// Measure the run of one colour through `(mx, my)`, up and down the column into `hei` and left
/// and right along the row into `wid`. `skip` leaves out one of the two axes - 1 the column, 2
/// the row - when the other half of the pill has already counted it.
///
/// Returns whether that run is four long and about to go. Original name: `aifSearchLineCore`.
fn search_line_core(
    field: &Field,
    data: &mut LineData,
    mx: usize,
    my: usize,
    skip: u8,
    cands: &[Candidate],
) -> bool {
    let tc = field.at(my, mx).co;
    *data = LineData::default();

    if field.at(my, mx).is_empty() {
        return false;
    }

    if skip != 2 {
        let mut touching = true;
        let mut row = my as i32 - 1;
        let stop_at = my as i32 - 4;
        while row > 0 && row > stop_at {
            let cell = field.at(row as usize, mx);
            if cell.co != tc {
                if cell.co != CO_EMPTY {
                    break;
                }
                touching = false;
            } else {
                data.hei[3] += 1;
                if cell.is_virus() {
                    data.hei[4] += 1;
                    if row < 4 {
                        data.hei[8] += BAD_LINE_RATE[row as usize][mx];
                    }
                } else if row < 4 {
                    data.hei[7] += BAD_LINE_RATE[row as usize][mx];
                }
                if touching {
                    data.hei[2] += 1;
                }
            }
            data.hei[5] += 1;
            row -= 1;
        }

        let mut touching = true;
        for (seen, row) in (my + 1..ROWS).enumerate() {
            let cell = field.at(row, mx);
            if cell.co != tc {
                if cell.co != CO_EMPTY {
                    break;
                }
                touching = false;
            } else {
                data.hei[3] += 1;
                if cell.is_virus() {
                    // only the first three count: further down they are out of reach
                    if seen < 3 {
                        data.hei[4] += 1;
                        if row < 4 {
                            data.hei[8] += BAD_LINE_RATE[row][mx];
                        }
                    }
                } else if row < 4 {
                    data.hei[7] += BAD_LINE_RATE[row][mx];
                }
                if touching {
                    data.hei[2] += 1;
                }
                data.hei[5] += 1;
            }
        }

        data.hei[2] += 1;
        data.hei[3] += 1;
        data.hei[5] += 1;
    }

    if skip != 1 {
        let mut fillable = true;
        let mut touching = true;
        let mut col = mx as i32 - 1;
        let stop_at = mx as i32 - 4;
        while col >= 0 && col > stop_at {
            let cell = field.at(my, col as usize);
            if cell.co != tc {
                if cell.co != CO_EMPTY {
                    break;
                }
                touching = false;
                if fillable {
                    if can_fill(cands, col as usize, my) {
                        data.wid[5] += 1;
                    } else {
                        fillable = false;
                    }
                }
            } else {
                data.wid[3] += 1;
                if cell.is_virus() {
                    data.wid[4] += 1;
                    if my < 4 {
                        data.wid[8] += BAD_LINE_RATE[my][col as usize];
                    }
                } else if my < 4 {
                    data.wid[7] += BAD_LINE_RATE[my][col as usize];
                }
                if touching {
                    data.wid[2] += 1;
                }
                if fillable {
                    data.wid[5] += 1;
                }
            }
            col -= 1;
        }

        let mut fillable = true;
        let mut touching = true;
        let mut col = mx + 1;
        while col < (mx + 4).min(COLS) {
            let cell = field.at(my, col);
            if cell.co != tc {
                if cell.co != CO_EMPTY {
                    break;
                }
                touching = false;
                if fillable {
                    if can_fill(cands, col, my) {
                        data.wid[5] += 1;
                    } else {
                        fillable = false;
                    }
                }
            } else {
                data.wid[3] += 1;
                if cell.is_virus() {
                    data.wid[4] += 1;
                    if my < 4 {
                        data.wid[8] += BAD_LINE_RATE[my][col];
                    }
                } else if my < 4 {
                    data.wid[7] += BAD_LINE_RATE[my][col];
                }
                if touching {
                    data.wid[2] += 1;
                }
                if fillable {
                    data.wid[5] += 1;
                }
            }
            col += 1;
        }

        data.wid[2] += 1;
        data.wid[3] += 1;
        data.wid[5] += 1;
    }

    let mut made = false;
    if data.hei[2] >= 4 {
        made = true;
        data.hei[0] = if data.hei[2] == 8 { 2 } else { 1 };
    }
    if data.wid[2] >= 4 {
        made = true;
        data.wid[0] = if data.wid[2] == 8 { 2 } else { 1 };
    }
    if made {
        if data.hei[0] != 0 {
            if data.wid[0] == 0 {
                data.wid[9] = 1;
            }
        } else {
            data.hei[9] = 1;
        }
    }

    made
}

/// Could an upright pill still drop a half into the gap at `(col, row)`? That is what turns a
/// row of three with a hole in it into something worth building towards.
fn can_fill(cands: &[Candidate], col: usize, row: usize) -> bool {
    cands
        .iter()
        .any(|c| c.tory == 0 && c.col == col && (c.row == row || c.row == row + 1))
}

/// Take the line the run at `(mx, my)` makes out of the field, counting the viruses it takes
/// with it, so the other half is measured against what is left. Original name:
/// `aifEraseLineCore`.
fn erase_line_core(field: &mut Field, data: &mut LineData, mx: usize, my: usize) {
    let tc = field.at(my, mx).co;
    let mut cleared = false;

    if data.hei[2] >= 4 {
        cleared = true;
        let mut row = my as i32 - 1;
        while row > 0 && row > my as i32 - 4 && field.at(row as usize, mx).co == tc {
            take(field, &mut data.hei[1], row as usize, mx);
            row -= 1;
        }
        let mut row = my + 1;
        while row < (my + 4).min(ROWS) && field.at(row, mx).co == tc {
            take(field, &mut data.hei[1], row, mx);
            row += 1;
        }
    }

    if data.wid[2] >= 4 {
        cleared = true;
        let mut col = mx as i32 - 1;
        while col > -1 && col > mx as i32 - 4 && field.at(my, col as usize).co == tc {
            take(field, &mut data.wid[1], my, col as usize);
            col -= 1;
        }
        let mut col = mx + 1;
        while col < (mx + 4).min(COLS) && field.at(my, col).co == tc {
            take(field, &mut data.wid[1], my, col);
            col += 1;
        }
    }

    if cleared {
        field.orphan_partner(my, mx);
        field.clear(my, mx);
    }
}

fn take(field: &mut Field, viruses: &mut i32, row: usize, col: usize) {
    let st = field.at(row, col).st;
    if (ST_VIRUS..8).contains(&st) {
        *viruses += 1;
    } else {
        field.orphan_partner(row, col);
    }
    field.clear(row, col);
}

/// Turn one measured run into points, and say how many lines it is worth. A run that has gone
/// is paid for what it took; one that has not is paid for what it could still become, but only
/// if there is room in the line for it to get there. Original name: `aifMiniPointK3`.
fn mini_point_k3(
    tbl: &[i32; 10],
    sub: bool,
    vertical: bool,
    tory: u8,
    ec: bool,
    p: &Params,
) -> (i32, i32) {
    let mut ex = 0;
    let mut elin = 0;

    if tbl[0] != 0 {
        elin = tbl[0];
        // the original zeroes [7] and [8] before summing, so a cleared line is never charged
        // for having been near the top
        for (count, point) in tbl.iter().zip(p.pri_point.iter()).take(7).skip(1) {
            ex += count * point;
        }
    } else if tbl[9] == 0 && tbl[5] >= 4 {
        let reach = (tbl[3] as usize).min(8);
        if vertical {
            if !ec || tory != 0 || tbl[3] >= 3 {
                ex = p.hei_lines_allp[reach];
            }
        } else if !ec || tory != 1 || tbl[3] >= 3 {
            ex = p.wid_lines_allp[reach];
        }
        ex += tbl[4] * p.pri_point[4];
    }

    if sub {
        ex /= 3;
    }
    (ex, elin)
}

/// How badly this half is stranded: a half in a run nothing can join is dead weight, and worse
/// again if it is sitting on a virus. `fell` says the half had to drop to get here.
/// Original name: `aifMiniAloneCapNumber`.
fn alone_cap_number(
    field: &Field,
    data: &LineData,
    x: usize,
    y: usize,
    fell: bool,
    ec: bool,
) -> usize {
    let hei = &data.hei;
    let wid = &data.wid;
    let alone = (hei[2] == 1 || (hei[2] != 0 && hei[5] < 4))
        && (wid[2] == 1 || (wid[2] != 0 && wid[5] < 4));
    let boxed_in = ec
        && hei[2] != 0
        && wid[2] != 0
        && (if hei[5] >= 4 { hei[2] } else { 0 }) + (if wid[5] >= 4 { wid[2] } else { 0 }) < 4;

    if alone || boxed_in {
        stranded(field, x, y, fell)
    } else {
        0
    }
}

/// As [`alone_cap_number`], but only asking about the row. Original name:
/// `aifMiniAloneCapNumberW`.
fn alone_cap_number_w(
    field: &Field,
    data: &LineData,
    x: usize,
    y: usize,
    fell: bool,
    ec: bool,
) -> usize {
    if data.wid[2] == 1 || (ec && data.wid[2] == 2) {
        stranded(field, x, y, fell)
    } else {
        0
    }
}

fn stranded(field: &Field, x: usize, y: usize, fell: bool) -> usize {
    if y == 0x10 {
        return 1;
    }
    let on_virus = field.below(y, x).st >= 5;
    match (fell, on_virus) {
        (true, false) => 2,
        (true, true) => 3,
        (false, false) => 4,
        (false, true) => 5,
    }
}
