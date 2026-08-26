//! The bottle as Dr. Mario 64's own ai sees it.
//!
//! The N64 ai works on its own copy of the playfield - `aiFieldData` in the decompilation - a
//! 17 by 8 grid of `(st, co)` pairs, where `co` is the colour and `st` says what the block is
//! and, for half a pill, which way its partner lies. Row 0 is a phantom row above the bottle
//! that is always empty; rows 1 to 16 are the bottle's own 16 rows. Every routine ported from
//! `aiset.c` is written in those coordinates, so the translation happens once, here.

use crate::game::block::{block_partner_offset, Block};
use crate::game::bottle::{Bottle, BOTTLE_HEIGHT, BOTTLE_WIDTH};
use crate::game::pill::VirusColor;

pub const ROWS: usize = BOTTLE_HEIGHT as usize + 1;
pub const COLS: usize = BOTTLE_WIDTH as usize;

/// the top half of a vertical pill: its partner is below
pub const ST_VERTICAL_TOP: u8 = 0;
/// the bottom half of a vertical pill: its partner is above
pub const ST_VERTICAL_BOTTOM: u8 = 1;
/// the left half of a horizontal pill: its partner is to the right
pub const ST_HORIZONTAL_LEFT: u8 = 2;
/// the right half of a horizontal pill: its partner is to the left
pub const ST_HORIZONTAL_RIGHT: u8 = 3;
/// a half whose partner has gone, which falls on its own
pub const ST_SINGLE: u8 = 4;
/// a virus. The original uses 5, 6 and 7 for the three virus sprites; every test is a range,
/// so one value does for all of them
pub const ST_VIRUS: u8 = 5;
/// marked by the chain check as part of a line that is about to go
pub const ST_CLEARING: u8 = 8;
pub const ST_EMPTY: u8 = 10;
/// the colour of an empty cell, which is why `co == 3` reads as "nothing here"
pub const CO_EMPTY: u8 = 3;

/// how much a block in the top four rows counts against you, by row and column: the middle
/// columns, where the pills come in, matter most. Original name: `BadLineRate`
pub const BAD_LINE_RATE: [[i32; COLS]; 4] = [
    [6, 7, 8, 9, 9, 8, 7, 6],
    [6, 7, 8, 9, 9, 8, 7, 6],
    [2, 2, 4, 7, 7, 4, 2, 2],
    [1, 1, 2, 4, 4, 2, 1, 1],
];

/// where a half's partner lies, indexed by `st`. Original name: `srh_466`
const PARTNER: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

pub fn is_virus_st(st: u8) -> bool {
    (5..8).contains(&st)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub st: u8,
    pub co: u8,
}

impl Cell {
    pub const EMPTY: Cell = Cell {
        st: ST_EMPTY,
        co: CO_EMPTY,
    };
    /// what the routines read when they run off the bottom of the bottle: the original indexes
    /// one row past the end and picks up whatever follows in memory, but every such read is
    /// asking "is there something under this?", and under the last row there is the floor
    pub const FLOOR: Cell = Cell {
        st: ST_SINGLE,
        co: CO_EMPTY,
    };

    pub fn new(st: u8, co: u8) -> Self {
        Self { st, co }
    }

    pub fn is_empty(&self) -> bool {
        self.st == ST_EMPTY
    }

    pub fn is_virus(&self) -> bool {
        is_virus_st(self.st)
    }
}

/// the ai's colour numbering, which is what `co` holds
pub fn colour(color: VirusColor) -> u8 {
    match color {
        VirusColor::Red => 0,
        VirusColor::Yellow => 1,
        VirusColor::Blue => 2,
    }
}

#[derive(Clone, Copy)]
pub struct Field {
    cells: [[Cell; COLS]; ROWS],
}

impl Field {
    pub fn empty() -> Self {
        Self {
            cells: [[Cell::EMPTY; COLS]; ROWS],
        }
    }

    /// the settled bottle, with the pill in play left out: the ai places that itself
    pub fn of(bottle: &Bottle) -> Self {
        let mut field = Self::empty();
        for y in 0..BOTTLE_HEIGHT {
            for x in 0..BOTTLE_WIDTH {
                field.cells[y as usize + 1][x as usize] = cell_of(bottle.block_at(x, y));
            }
        }
        field
    }

    pub fn at(&self, row: usize, col: usize) -> Cell {
        self.cells[row][col]
    }

    /// as [Self::at], but the row below the last one reads as the floor rather than running off
    /// the end of the grid the way the original does
    pub fn below(&self, row: usize, col: usize) -> Cell {
        if row + 1 < ROWS {
            self.cells[row + 1][col]
        } else {
            Cell::FLOOR
        }
    }

    pub fn set(&mut self, row: usize, col: usize, cell: Cell) {
        self.cells[row][col] = cell;
    }

    /// mark a cell as gone the way the chain check does: it leaves the colour behind, and
    /// every test that follows asks about `st`
    pub fn set_st(&mut self, row: usize, col: usize, st: u8) {
        self.cells[row][col].st = st;
    }

    pub fn clear(&mut self, row: usize, col: usize) {
        self.cells[row][col] = Cell::EMPTY;
    }

    /// A half of a pill has just gone, so whatever it was joined to is on its own from now on.
    /// Original name: `aif_MiniChangeBall`
    pub fn orphan_partner(&mut self, row: usize, col: usize) {
        let st = self.cells[row][col].st;
        if st as usize >= PARTNER.len() {
            // a single, a virus, or a block already marked for clearing: nothing to orphan
            return;
        }
        let (dy, dx) = PARTNER[st as usize];
        let y = row as i32 + dy;
        let x = col as i32 + dx;
        if y > 0 && (y as usize) < ROWS && x >= 0 && (x as usize) < COLS {
            self.cells[y as usize][x as usize].st = ST_SINGLE;
        }
    }
}

fn cell_of(block: Block) -> Cell {
    match block {
        Block::Virus(color) => Cell::new(ST_VIRUS, colour(color)),
        Block::Garbage(color) => Cell::new(ST_SINGLE, colour(color)),
        Block::Stack(color, rotation, ordinal) => {
            let offset = block_partner_offset(rotation, ordinal);
            let st = match (offset.x(), offset.y()) {
                (0, 1) => ST_VERTICAL_TOP,
                (0, -1) => ST_VERTICAL_BOTTOM,
                (1, 0) => ST_HORIZONTAL_LEFT,
                _ => ST_HORIZONTAL_RIGHT,
            };
            Cell::new(st, colour(color))
        }
        // the pill in play and its ghost are not part of the stack
        Block::Empty | Block::Vitamin(_, _, _) | Block::Ghost(_, _, _) => Cell::EMPTY,
    }
}
