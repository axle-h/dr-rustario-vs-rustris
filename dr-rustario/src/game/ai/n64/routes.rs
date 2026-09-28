//! How much room the pill has to move (`aiRootP`), for the situation classifier: the average
//! route length from each reachable resting place back to the spawn. Only the classifier reads
//! it, so it need only tell a roomy bottle from a desperate one.

use crate::game::ai::n64::field::{Field, COLS, ROWS};

/// the grid padded with walls either side and below, so the walk never checks bounds;
/// original name: `aiRecurData`
const P_ROWS: usize = ROWS + 1;
const P_COLS: usize = COLS + 2;

const WALL: u8 = 0xFF;
const VISITED: u8 = 0xF;
const EMPTY_CO: u8 = 3;
const EMPTY_ST: u8 = 10;

/// where the pill comes in, in padded coordinates
const GOAL_COL: usize = 4;
const GOAL_ROW: usize = 1;

struct Walk {
    co: [[u8; P_COLS]; P_ROWS],
    st: [[u8; P_COLS]; P_ROWS],
    steps: u32,
    arrived: bool,
}

/// The average number of cells on the route to a resting place, over the places that have one.
/// original name: `aiRootP`
pub fn average_route(field: &Field) -> f32 {
    let base = Walk::of(field);
    let mut routes = 0u32;
    let mut reachable = 0u32;

    for (upright, col, row) in base.places() {
        let mut walk = Walk {
            co: base.co,
            st: base.st,
            steps: 0,
            arrived: false,
        };
        if upright {
            walk.upright(col, row);
        } else {
            walk.flat(col, row);
        }
        if walk.arrived {
            reachable += 1;
            routes += walk.steps;
        }
    }

    if reachable == 0 {
        0.0
    } else {
        routes as f32 / reachable as f32
    }
}

impl Walk {
    fn of(field: &Field) -> Self {
        let mut co = [[WALL; P_COLS]; P_ROWS];
        let mut st = [[WALL; P_COLS]; P_ROWS];
        for row in 0..ROWS {
            for col in 0..COLS {
                co[row][col + 1] = field.at(row, col).co;
                st[row][col + 1] = field.at(row, col).st;
            }
        }
        Self {
            co,
            st,
            steps: 0,
            arrived: false,
        }
    }

    /// every resting place in the bottle, as (upright, column, row) in padded coordinates.
    /// original name: `aifPlaceSearch`
    fn places(&self) -> Vec<(bool, usize, usize)> {
        let mut places = vec![];
        for row in 1..P_ROWS - 1 {
            for col in 1..P_COLS - 1 {
                if self.co[row][col] == EMPTY_CO
                    && self.co[row + 1][col] != EMPTY_CO
                    && self.co[row - 1][col] == EMPTY_CO
                {
                    places.push((true, col, row));
                }
            }
        }
        for row in 1..ROWS {
            for col in 1..COLS {
                if self.co[row][col] != EMPTY_CO || self.co[row][col + 1] != EMPTY_CO {
                    continue;
                }
                if self.co[row + 1][col] == EMPTY_CO && self.co[row + 1][col + 1] == EMPTY_CO {
                    continue;
                }
                places.push((false, col, row));
            }
        }
        places
    }

    /// original name: `aifTRecur`, with the pill upright and (col, row) its lower half
    fn upright(&mut self, col: usize, row: usize) {
        self.co[row][col] = VISITED;

        if col == GOAL_COL && row == GOAL_ROW {
            self.arrived = true;
        }

        if !self.arrived
            && row >= 2
            && self.co[row - 1][col] == EMPTY_CO
            && self.st[row - 2][col] == EMPTY_ST
        {
            self.upright(col, row - 1);
        }
        if !self.arrived
            && self.co[row][col + 1] == EMPTY_CO
            && self.st[row - 1][col + 1] == EMPTY_ST
        {
            self.upright(col + 1, row);
        }
        if !self.arrived
            && self.co[row][col - 1] == EMPTY_CO
            && self.st[row - 1][col - 1] == EMPTY_ST
        {
            self.upright(col - 1, row);
        }

        if self.arrived {
            self.steps += 1;
        }
    }

    /// original name: `aifYRecur`, with the pill flat and (col, row) its left half
    fn flat(&mut self, col: usize, row: usize) {
        self.co[row][col] = VISITED;

        if col == GOAL_COL && row == GOAL_ROW {
            self.arrived = true;
        }

        if !self.arrived
            && row >= 2
            && self.co[row - 1][col] == EMPTY_CO
            && self.st[row - 1][col + 1] == EMPTY_ST
        {
            self.flat(col, row - 1);
        }
        if !self.arrived && self.co[row][col + 1] == EMPTY_CO && self.st[row][col + 2] == EMPTY_ST {
            self.flat(col + 1, row);
        }
        if !self.arrived && self.co[row][col - 1] == EMPTY_CO && self.st[row][col] == EMPTY_ST {
            self.flat(col - 1, row);
        }

        if self.arrived {
            self.steps += 1;
        }
    }
}
