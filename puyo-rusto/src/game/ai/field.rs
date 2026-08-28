//! The board the ai thinks on.
//!
//! [`crate::game::board::Board`] is the board the *game* plays: it carries link masks and a
//! skin so that a renderer can draw it, and its chain loop hands back `Vec`s of placed cells
//! for the animations to consume. A search asks the same questions tens of thousands of times
//! per pair and needs none of that, so it works on this instead: one byte a cell, no skin, no
//! masks, no allocation anywhere in a chain.
//!
//! It is the same *rules*, though, and deliberately so - the chain loop below is
//! [`crate::game::board::Board::settle`] and [`pop`](crate::game::board::Board::pop) written
//! out flat, and it scores with the game's own [`step_score`]. A search that resolved chains
//! its own way would rank placements the game would not play out.
//!
//! Coordinates are the board's: `y` grows downwards, row 0 is the hidden thirteenth and
//! nothing there pops or counts towards a group - see
//! [`is_ghost`](crate::game::board::is_ghost).

use crate::game::board::{Board, CELLS, COLUMNS, DEATH_SQUARE, HIDDEN_ROWS, ROWS, VISIBLE_ROWS};
use crate::game::cell::{PuyoCell, PuyoColor};
use crate::game::score::{step_score, PoppedGroup, PUYOS_TO_POP};
use engine::game::geometry::Point;

pub const WIDTH: usize = COLUMNS as usize;
pub const HEIGHT: usize = ROWS as usize;
/// the rows a puyo can pop in; the one above them is the ghost row
pub const VISIBLE: usize = VISIBLE_ROWS as usize;
/// the first row anything can group in
pub const FLOOR_OF_PLAY: usize = HIDDEN_ROWS as usize;
/// the column a pair spawns over, which is the one that has to stay passable
pub const SPAWN_COLUMN: usize = DEATH_SQUARE.x as usize;

/// nothing here
pub const EMPTY: u8 = 0;
/// nuisance: it fills a cell and clears beside a group, but it has no colour to group with
pub const NUISANCE: u8 = PuyoColor::N as u8 + 1;

/// the most groups one chain step can pop: a board of 78 cells, four to a group
const MAX_GROUPS: usize = CELLS / PUYOS_TO_POP as usize;

/// a colour as this field stores it. Zero is reserved for [`EMPTY`], so a colour is its index
/// plus one and the whole cell fits in a byte with nuisance above it.
pub const fn of_color(color: PuyoColor) -> u8 {
    color as u8 + 1
}

/// the colour back out of a cell; `None` for empty and for nuisance
pub fn to_color(cell: u8) -> Option<PuyoColor> {
    if cell == EMPTY || cell == NUISANCE {
        None
    } else {
        Some(PuyoColor::from_index(cell as usize - 1))
    }
}

/// What a chain did: how many steps it ran to, what it scored and how many puyos it spent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Chain {
    /// steps, so 0 is "nothing popped" and 1 is a single pop rather than a chain
    pub count: u32,
    /// the game's own score for it, [`step_score`] summed over the steps
    pub score: u32,
    /// every puyo it took off the board, nuisance included
    pub popped: u32,
}

/// A board, and how tall each of its columns is.
///
/// The heights are carried rather than counted because everything asks for them - the
/// evaluation, the move generator, the quiescence search and the tear - and counting them is a
/// scan of the whole board. They are kept true by the two operations that can change them:
/// [`Field::drop_into`], which adds one to a column, and [`Field::settle`], which is where
/// they are worked out from scratch.
#[derive(Clone, Copy)]
pub struct Field {
    cells: [u8; CELLS],
    heights: [u8; WIDTH],
}

/// two fields are the same field when they hold the same puyos; the heights are a cache of
/// exactly that and cannot disagree
impl PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        self.cells == other.cells
    }
}

impl Eq for Field {}

impl Default for Field {
    fn default() -> Self {
        Self::new()
    }
}

/// The up to four orthogonal neighbours of every cell, worked out once at compile time.
///
/// Entry zero of a row is how many there are and the rest are the cells. It is a table rather
/// than arithmetic because the chain loop asks for it three hundred times per scan and the
/// scan runs tens of thousands of times per pair, and because the answer never changes.
///
/// **The ghost row is not a neighbour of anything.** Nothing there groups, and nothing there
/// is dragged out by a group beside it, so leaving it out of the table is the whole of the
/// ghost rule as far as popping is concerned.
const NEIGHBOURS: [[u8; 5]; CELLS] = {
    let mut table = [[0u8; 5]; CELLS];
    let mut index = 0;
    while index < CELLS {
        let x = index % WIDTH;
        let y = index / WIDTH;
        let mut n = 0;
        if y > FLOOR_OF_PLAY {
            n += 1;
            table[index][n] = (index - WIDTH) as u8;
        }
        if y + 1 < HEIGHT {
            n += 1;
            table[index][n] = (index + WIDTH) as u8;
        }
        if x > 0 {
            n += 1;
            table[index][n] = (index - 1) as u8;
        }
        if x + 1 < WIDTH {
            n += 1;
            table[index][n] = (index + 1) as u8;
        }
        table[index][0] = n as u8;
        index += 1;
    }
    table
};

/// the neighbours of a cell, as a slice
#[inline(always)]
fn neighbours(index: usize) -> &'static [u8] {
    let row = &NEIGHBOURS[index];
    &row[1..1 + row[0] as usize]
}

impl Field {
    pub const fn new() -> Self {
        Self {
            cells: [EMPTY; CELLS],
            heights: [0; WIDTH],
        }
    }

    /// Read a game board in.
    ///
    /// The link masks come off here and are not missed: a mask says what a *renderer* should
    /// join up, and connectivity for popping is worked out from the colours themselves - see
    /// [`crate::game::board::Board::recompute_links`]. Comparing masked cells would see
    /// sixteen different reds and find no chains at all.
    pub fn from_board(board: &Board) -> Self {
        let mut field = Self::new();
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let cell = board.get(Point::new(x as i32, y as i32));
                field.cells[y * WIDTH + x] = match cell {
                    None => EMPTY,
                    Some(PuyoCell::Puyo { color, .. }) => of_color(color),
                    Some(_) => NUISANCE,
                };
            }
        }
        // settling here is what makes "always settled" true whatever this was handed. A board
        // with a pair in play is settled already, so on the path that matters it moves
        // nothing; a board caught in the middle of a chain is read as it is about to be
        field.settle();
        field
    }

    pub fn get(&self, x: usize, y: usize) -> u8 {
        self.cells[y * WIDTH + x]
    }

    pub fn set(&mut self, x: usize, y: usize, cell: u8) {
        self.cells[y * WIDTH + x] = cell;
        self.recount_heights();
    }

    /// how tall the stack in a column is, the ghost row included: 13 is a column full to the
    /// top of the board and 12 is one that has reached the ghost row
    pub fn height(&self, x: usize) -> u8 {
        self.heights[x]
    }

    pub fn heights(&self) -> [u8; WIDTH] {
        self.heights
    }

    /// A column is as tall as its topmost puyo stands, which counts a hole under the stack as
    /// filled - that is what a height is for, since a pair dropped on the column lands on top
    /// of the hole and not in it.
    fn recount_heights(&mut self) {
        for x in 0..WIDTH {
            self.heights[x] = 0;
            for y in 0..HEIGHT {
                if self.cells[y * WIDTH + x] != EMPTY {
                    self.heights[x] = (HEIGHT - y) as u8;
                    break;
                }
            }
        }
    }

    /// drop one cell down a column, returning the row it came to rest in
    pub fn drop_into(&mut self, x: usize, cell: u8) -> Option<usize> {
        let height = self.heights[x] as usize;
        if height >= HEIGHT {
            return None;
        }
        let y = HEIGHT - 1 - height;
        self.cells[y * WIDTH + x] = cell;
        self.heights[x] = (height + 1) as u8;
        Some(y)
    }

    /// a puyo is resting on the death square, which is the one thing that ends a game
    pub fn is_dead(&self) -> bool {
        self.get(DEATH_SQUARE.x as usize, DEATH_SQUARE.y as usize) != EMPTY
    }

    pub fn is_empty(&self) -> bool {
        self.cells.iter().all(|cell| *cell == EMPTY)
    }

    pub fn nuisance_count(&self) -> u32 {
        self.cells.iter().filter(|cell| **cell == NUISANCE).count() as u32
    }

    /// the occupancy of the ghost row as one bit per column, low bit column 0
    pub fn ghost_row(&self) -> u8 {
        let mut bits = 0u8;
        for x in 0..WIDTH {
            if self.cells[x] != EMPTY {
                bits |= 1 << x;
            }
        }
        bits
    }

    /// Let everything floating fall, and report whether anything moved.
    pub fn settle(&mut self) -> bool {
        let mut moved = false;
        for x in 0..WIDTH {
            let mut write = HEIGHT as i32 - 1;
            for y in (0..HEIGHT as i32).rev() {
                let index = y as usize * WIDTH + x;
                if self.cells[index] != EMPTY {
                    if y != write {
                        self.cells[write as usize * WIDTH + x] = self.cells[index];
                        self.cells[index] = EMPTY;
                        moved = true;
                    }
                    write -= 1;
                }
            }
            self.heights[x] = (HEIGHT as i32 - 1 - write) as u8;
        }
        moved
    }

    /// every orthogonally connected run of one colour, whatever its size, ghost row excluded
    fn for_each_group(&self, mut f: impl FnMut(u8, &[u8])) {
        let mut seen = [false; CELLS];
        let mut group = [0u8; CELLS];
        for index in FLOOR_OF_PLAY * WIDTH..CELLS {
            let color = self.cells[index];
            if color == EMPTY || color == NUISANCE || seen[index] {
                continue;
            }
            seen[index] = true;
            group[0] = index as u8;
            let mut len = 1usize;
            let mut head = 0usize;
            while head < len {
                let at = group[head] as usize;
                head += 1;
                for next in neighbours(at) {
                    let next = *next as usize;
                    if !seen[next] && self.cells[next] == color {
                        seen[next] = true;
                        group[len] = next as u8;
                        len += 1;
                    }
                }
            }
            f(color, &group[..len]);
        }
    }

    /// How many groups of exactly two and of exactly three there are.
    ///
    /// This is the chain-building material on the board: a pair wants to join something, and a
    /// three wants one more. ama counts *connections* off its bitboard rather than groups;
    /// counting whole groups says the same thing about the same board and says it in the units
    /// the pop rule is written in, so a three is a three rather than three overlapping pairs.
    pub fn link_counts(&self) -> (u32, u32) {
        let (mut twos, mut threes) = (0, 0);
        self.for_each_group(|_, group| match group.len() {
            2 => twos += 1,
            3 => threes += 1,
            _ => {}
        });
        (twos, threes)
    }

    /// Does the group containing `(x, y)` reach `needed` puyos?
    ///
    /// Asked once per key puyo of every probe the quiescence search makes, which is tens of
    /// thousands of times a pair, so it is written to answer rather than to count: it walks
    /// outwards and stops the moment it has seen enough, and it remembers where it has been in
    /// a list `needed` long rather than in a map of the board. Nothing here groups in the
    /// ghost row.
    pub fn has_group_of(&self, x: usize, y: usize, needed: u32) -> bool {
        debug_assert!(needed as usize <= PUYOS_TO_POP as usize);
        if y < FLOOR_OF_PLAY || needed == 0 {
            return needed == 0;
        }
        let start = y * WIDTH + x;
        let color = self.cells[start];
        if color == EMPTY || color == NUISANCE {
            return false;
        }
        if needed == 1 {
            return true;
        }

        let mut seen = [0u8; PUYOS_TO_POP as usize];
        seen[0] = start as u8;
        let mut count = 1usize;
        let mut head = 0usize;
        while head < count {
            let at = seen[head] as usize;
            head += 1;
            for next in neighbours(at) {
                if self.cells[*next as usize] != color || seen[..count].contains(next) {
                    continue;
                }
                seen[count] = *next;
                count += 1;
                if count >= needed as usize {
                    return true;
                }
            }
        }
        false
    }

    /// One step of the chain loop: pop everything ready to go, nuisance beside it included.
    ///
    /// Written around the list of what is going rather than around a map of the board,
    /// because a chain step takes a dozen cells off a board of seventy eight and every pass
    /// over the other sixty six is time the quiescence search pays for a dozen times over per
    /// placement.
    fn pop_step(&mut self, chain: u32) -> Option<(u32, u32)> {
        let mut groups = [PoppedGroup {
            color: PuyoColor::Red,
            size: 0,
        }; MAX_GROUPS];
        let mut count = 0usize;
        let mut going = [0u8; CELLS];
        let mut len = 0usize;

        self.for_each_group(|color, group| {
            if group.len() as u32 >= PUYOS_TO_POP {
                groups[count] = PoppedGroup {
                    // a group is one colour and it is not nuisance, so this cannot fail
                    color: to_color(color).expect("a colour group has a colour"),
                    size: group.len() as u32,
                };
                count += 1;
                going[len..len + group.len()].copy_from_slice(group);
                len += group.len();
            }
        });

        if count == 0 {
            return None;
        }

        // nuisance touching anything that goes, goes with it - but never a ghost, which
        // nothing clears, and never twice however many groups it is beside
        let colored = len;
        for at in 0..colored {
            for next in neighbours(going[at] as usize) {
                if self.cells[*next as usize] == NUISANCE && !going[colored..len].contains(next) {
                    going[len] = *next;
                    len += 1;
                }
            }
        }

        for cell in &going[..len] {
            self.cells[*cell as usize] = EMPTY;
        }

        Some((step_score(chain, &groups[..count]), len as u32))
    }

    /// the cells of the board, for a test that wants to look at one
    #[cfg(test)]
    pub fn row(&self, y: usize) -> &[u8] {
        &self.cells[y * WIDTH..(y + 1) * WIDTH]
    }

    /// Run the chain out: pop, settle, pop, until nothing is left to go.
    ///
    /// It pops before it settles, where [`crate::game::board::Board`]'s loop settles first,
    /// because the two are handed different boards. The game's chain starts from a pair that
    /// has just locked with one half still in the air; a [`Field`] is **always settled** - it
    /// is built by settling and the only thing that adds to it is
    /// [`drop_into`](Self::drop_into), which lands on top of a column - so settling to open
    /// with would be a scan of the whole board to move nothing, a dozen times over per
    /// placement.
    pub fn resolve(&mut self) -> Chain {
        let mut chain = Chain::default();
        while let Some((score, popped)) = self.pop_step(chain.count + 1) {
            chain.count += 1;
            chain.score = chain.score.saturating_add(score);
            chain.popped += popped;
            self.settle();
        }
        chain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::board::tests::{board, board_rows};

    fn field(rows: &[&str]) -> Field {
        Field::from_board(&board(rows))
    }

    #[test]
    fn a_field_reads_a_board_back_the_way_it_found_it() {
        let field = field(&["......", "..rr..", "..rr.o"]);
        assert_eq!(field.height(2), 2);
        assert_eq!(field.height(0), 0);
        assert_eq!(field.get(5, HEIGHT - 1), NUISANCE);
        assert_eq!(to_color(field.get(2, HEIGHT - 1)), Some(PuyoColor::Red));
    }

    /// the same square of four the board pops, popped the same way and scored the same
    #[test]
    fn a_square_of_four_pops_for_what_the_game_would_score_it() {
        let mut field = field(&["..rr..", "..rr.."]);
        let chain = field.resolve();
        assert_eq!(chain.count, 1);
        assert_eq!(chain.popped, 4);
        assert_eq!(
            chain.score,
            step_score(
                1,
                &[PoppedGroup {
                    color: PuyoColor::Red,
                    size: 4
                }]
            )
        );
        assert!(field.is_empty());
    }

    /// nuisance has no colour of its own, so it only ever leaves beside a group that pops
    #[test]
    fn nuisance_beside_a_group_goes_with_it() {
        let mut field = field(&["o.....", "r....o", "rrr..."]);
        let chain = field.resolve();
        assert_eq!(chain.count, 1);
        // four reds and the nuisance sitting on them; the one across the board is untouched
        assert_eq!(chain.popped, 5);
        assert_eq!(field.nuisance_count(), 1);
    }

    /// A chain is steps, and a step only happens because the one before it moved something.
    ///
    /// Three reds waiting on a fourth, with two greens sitting on the red that is holding
    /// them apart from two more: the reds go, the greens fall into each other, and the greens
    /// go. Nothing about the board before the drop is ready to pop, which is the point - a
    /// chain is built, and then set off.
    #[test]
    fn a_chain_counts_its_steps() {
        let mut field = field(&[".g....", "rg....", "rrgg.."]);
        assert_eq!(field.resolve().count, 0, "nothing is ready to go yet");
        field.drop_into(0, of_color(PuyoColor::Red));
        let chain = field.resolve();
        assert_eq!(chain.count, 2, "the reds go, then the greens fall together");
        assert!(field.is_empty());
    }

    /// the whole of the ghost rule: a group with a foot in the hidden row is held back, and
    /// the three visible ones do **not** pop and leave it behind
    #[test]
    fn a_group_of_four_with_a_ghost_in_it_does_not_pop() {
        // a column full to the very top, so nothing settles and the fourth red is a ghost
        let full = [
            "r.....", "r.....", "r.....", "r.....", "b.....", "y.....", "b.....", "y.....",
            "b.....", "y.....", "b.....", "y.....", "b.....",
        ];
        let mut field = Field::from_board(&board_rows(&full));
        assert_eq!(field.height(0), HEIGHT as u8);
        assert_eq!(field.resolve().count, 0, "three visible reds and a ghost");

        // take one cell out from under them and the ghost drops into view, and the same four go
        let mut dropped = Field::from_board(&board_rows(&full[..full.len() - 1]));
        assert_eq!(
            dropped.resolve().count,
            1,
            "the ghost fell in and the four popped"
        );
    }

    #[test]
    fn a_group_of_two_and_a_group_of_three_are_counted_apart() {
        let field = field(&["g.....", "g.....", "grr.bb"]);
        assert_eq!(
            field.link_counts(),
            (2, 1),
            "rr and bb are twos, ggg is the three"
        );
    }
}
