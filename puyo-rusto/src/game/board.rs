//! The playfield: the grid, what is connected to what, popping, settling and the chain loop.
//!
//! Coordinates are the engine's, so `y` grows *downwards* and row 0 is the top row simulated.
//! That top row is the game's hidden thirteenth, and it is not merely invisible - see
//! [`is_ghost`].

use crate::game::cell::{LinkMask, PuyoCell};
use crate::game::score::{PoppedGroup, PUYOS_TO_POP};
use engine::game::geometry::Point;
use engine::game::{CellId, PlacedCell};

pub const COLUMNS: u32 = 6;
/// the twelve rows a player can see
pub const VISIBLE_ROWS: u32 = 12;
/// the thirteenth row, above the visible board: puyos rest there but are not drawn
pub const HIDDEN_ROWS: u32 = 1;
pub const ROWS: u32 = VISIBLE_ROWS + HIDDEN_ROWS;
pub const CELLS: usize = (COLUMNS * ROWS) as usize;

/// The square that ends the game.
///
/// Puyo Nexus, *Basic rules*: "the game acts as if there was a red X in the square on the
/// first row, third column". It is the top *visible* row, and it is also where a pair spawns -
/// so the losing condition is a puyo coming to **rest** here, which is not the same rule as
/// the new pair having nowhere to go.
pub const DEATH_SQUARE: Point = Point::new(2, HIDDEN_ROWS as i32);

/// where a pair's pivot appears; its child sits in the hidden row above
pub const SPAWN: Point = DEATH_SQUARE;

/// Is this the hidden thirteenth row, where a puyo becomes a **ghost puyo**?
///
/// Puyo Nexus, [Special Maneuvers and
/// Mechanics](https://puyonexus.com/wiki/Special_Maneuvers_and_Mechanics#The_13th_Row_and_Beyond):
/// "Puyo in the 13th row can't be cleared even if they 'connect' in a group of four... You can
/// use the 13th row's properties to make chains that won't pop until the Puyo in the 13th row
/// drops down."
///
/// So a ghost puyo is inert rather than invisible. It does not pop, and it does not count
/// towards the four that a group needs - which is the whole technique: a chain with a foot in
/// the ghost row is *held back* until that puyo falls into row 12 and joins the game properly.
/// Reading it the other way round, with the three visible ones popping and the ghost left
/// behind, would make the chain fire immediately and there would be nothing to hold back.
///
/// Nothing in this row is ever cleared, nuisance included, and nothing here draws itself
/// joined to anything - the link mask is the game telling a player what will pop together, and
/// a ghost will not.
pub fn is_ghost(point: Point) -> bool {
    point.y < HIDDEN_ROWS as i32
}

/// the four orthogonal neighbours, paired with the link bit each one sets
const NEIGHBOURS: [(Point, LinkMask); 4] = [
    (Point::new(0, -1), LinkMask::UP),
    (Point::new(0, 1), LinkMask::DOWN),
    (Point::new(-1, 0), LinkMask::LEFT),
    (Point::new(1, 0), LinkMask::RIGHT),
];

/// What one step of the chain loop took off the board.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChainStep {
    /// every cell that went, coloured and nuisance alike, as the renderer wants them
    pub cells: Vec<PlacedCell>,
    /// the coloured groups, which is what the score is worked out from
    pub groups: Vec<PoppedGroup>,
    /// how many nuisance puyos were taken out alongside them
    pub nuisance: u32,
}

impl ChainStep {
    /// how many puyos went in total, nuisance included: the count the engine's `Clear` carries
    pub fn count(&self) -> u32 {
        self.cells.len() as u32
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    cells: [Option<PuyoCell>; CELLS],
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl Board {
    pub fn new() -> Self {
        Self {
            cells: [None; CELLS],
        }
    }

    fn index(point: Point) -> Option<usize> {
        if point.x < 0 || point.y < 0 || point.x >= COLUMNS as i32 || point.y >= ROWS as i32 {
            None
        } else {
            Some((point.y * COLUMNS as i32 + point.x) as usize)
        }
    }

    pub fn in_bounds(point: Point) -> bool {
        Self::index(point).is_some()
    }

    pub fn get(&self, point: Point) -> Option<PuyoCell> {
        Self::index(point).and_then(|i| self.cells[i])
    }

    /// free *and* on the board; anything off the board counts as occupied, so a piece cannot
    /// be moved into it
    pub fn is_free(&self, point: Point) -> bool {
        Self::index(point).is_some_and(|i| self.cells[i].is_none())
    }

    pub fn set(&mut self, point: Point, cell: Option<PuyoCell>) {
        if let Some(i) = Self::index(point) {
            self.cells[i] = cell;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.cells.iter().all(Option::is_none)
    }

    /// nothing left at all, which under Tsu earns the all clear bonus
    pub fn is_all_clear(&self) -> bool {
        self.is_empty()
    }

    /// a puyo has come to rest on the death square
    pub fn is_dead(&self) -> bool {
        self.get(DEATH_SQUARE).is_some()
    }

    pub fn occupied(&self) -> u32 {
        self.cells.iter().filter(|c| c.is_some()).count() as u32
    }

    /// the lowest free cell of a column, or `None` when the column is full to the top
    pub fn landing(&self, column: i32) -> Option<Point> {
        (0..ROWS as i32)
            .rev()
            .map(|y| Point::new(column, y))
            .find(|point| self.is_free(*point))
    }

    /// drop one cell down a column, returning where it came to rest
    pub fn drop_into(&mut self, column: i32, cell: PuyoCell) -> Option<Point> {
        let point = self.landing(column)?;
        self.set(point, Some(cell));
        Some(point)
    }

    /// how tall the stack in a column is
    pub fn height(&self, column: i32) -> u32 {
        (0..ROWS as i32)
            .find(|y| !self.is_free(Point::new(column, *y)))
            .map(|y| ROWS - y as u32)
            .unwrap_or(0)
    }

    /// Let everything floating fall, one column at a time, and report whether anything moved.
    ///
    /// A settle is what happens between chain steps: a pop leaves holes and whatever was
    /// resting on the group comes down into them.
    pub fn settle(&mut self) -> bool {
        let mut moved = false;
        for x in 0..COLUMNS as i32 {
            let mut write = ROWS as i32 - 1;
            for y in (0..ROWS as i32).rev() {
                let point = Point::new(x, y);
                if let Some(cell) = self.get(point) {
                    if y != write {
                        self.set(point, None);
                        self.set(Point::new(x, write), Some(cell));
                        moved = true;
                    }
                    write -= 1;
                }
            }
        }
        if moved {
            self.recompute_links();
        }
        moved
    }

    /// The colour of a cell for the purpose of grouping: a ghost puyo has none.
    ///
    /// This one function is the whole of the ghost rule. A ghost is neither the start of a
    /// group nor reachable from one, so it can neither pop nor make up the numbers.
    fn grouping_color(&self, point: Point) -> Option<crate::game::cell::PuyoColor> {
        if is_ghost(point) {
            return None;
        }
        self.get(point).and_then(|cell| cell.color())
    }

    /// every orthogonally connected run of one colour, whatever its size
    fn colour_groups(&self) -> Vec<Vec<Point>> {
        let mut seen = [false; CELLS];
        let mut groups = vec![];
        // the ghost row is skipped outright: nothing there groups
        for y in HIDDEN_ROWS as i32..ROWS as i32 {
            for x in 0..COLUMNS as i32 {
                let start = Point::new(x, y);
                let index = Self::index(start).expect("in bounds");
                let Some(color) = self.grouping_color(start) else {
                    continue;
                };
                if seen[index] {
                    continue;
                }
                // flood fill this colour outwards from here
                let mut group = vec![];
                let mut stack = vec![start];
                seen[index] = true;
                while let Some(point) = stack.pop() {
                    group.push(point);
                    for (offset, _) in NEIGHBOURS {
                        let next = point + offset;
                        let Some(next_index) = Self::index(next) else {
                            continue;
                        };
                        if seen[next_index] {
                            continue;
                        }
                        if self.grouping_color(next) == Some(color) {
                            seen[next_index] = true;
                            stack.push(next);
                        }
                    }
                }
                groups.push(group);
            }
        }
        groups
    }

    /// the groups that are big enough to pop
    pub fn popping_groups(&self) -> Vec<Vec<Point>> {
        self.colour_groups()
            .into_iter()
            .filter(|group| group.len() as u32 >= PUYOS_TO_POP)
            .collect()
    }

    /// Pop everything that is ready to, and report it; `None` when nothing was.
    ///
    /// Nuisance is not cleared by being grouped - it has no colour to group with - but any
    /// nuisance touching a group that pops goes with it.
    pub fn pop(&mut self) -> Option<ChainStep> {
        let groups = self.popping_groups();
        if groups.is_empty() {
            return None;
        }

        let mut step = ChainStep::default();
        let mut going: Vec<Point> = vec![];
        for group in groups.iter() {
            let color = self
                .get(group[0])
                .and_then(|cell| cell.color())
                .expect("a colour group has a colour");
            step.groups.push(PoppedGroup {
                color,
                size: group.len() as u32,
            });
            going.extend(group.iter().copied());
        }

        // nuisance beside anything that pops goes too, once however many groups touch it -
        // unless it is a ghost, which nothing clears
        let mut nuisance: Vec<Point> = vec![];
        for point in going.iter() {
            for (offset, _) in NEIGHBOURS {
                let next = *point + offset;
                if is_ghost(next) {
                    continue;
                }
                if self.get(next) == Some(PuyoCell::Nuisance) && !nuisance.contains(&next) {
                    nuisance.push(next);
                }
            }
        }
        step.nuisance = nuisance.len() as u32;
        going.extend(nuisance);

        for point in going {
            if let Some(cell) = self.get(point) {
                step.cells.push((point, CellId::from(cell)));
            }
            self.set(point, None);
        }
        self.recompute_links();
        Some(step)
    }

    /// Recompute every puyo's link mask from the colours around it.
    ///
    /// Cheap enough to do wholesale after every lock, pop and settle, and doing it wholesale
    /// is why it cannot drift: a pop changes the mask of every survivor that was touching the
    /// group, and a settle changes the masks of everything the fallen puyo left behind as well
    /// as everything it arrives next to.
    pub fn recompute_links(&mut self) {
        let before = self.cells;
        for y in 0..ROWS as i32 {
            for x in 0..COLUMNS as i32 {
                let point = Point::new(x, y);
                let index = Self::index(point).expect("in bounds");
                let Some(cell) = before[index] else { continue };
                let Some(color) = cell.color() else {
                    // nuisance never joins to anything, including other nuisance
                    continue;
                };
                let mut links = LinkMask::NONE;
                // a ghost puyo joins to nothing, and nothing joins up to it: the mask says
                // what will pop together, and a ghost will not
                if !is_ghost(point) {
                    for (offset, bit) in NEIGHBOURS {
                        let next = point + offset;
                        if is_ghost(next) {
                            continue;
                        }
                        let matches = Self::index(next)
                            .and_then(|i| before[i])
                            .and_then(|c| c.color())
                            == Some(color);
                        if matches {
                            links = links.with(bit);
                        }
                    }
                }
                self.cells[index] = Some(cell.with_links(links));
            }
        }
    }

    /// every occupied cell, for the renderer and for tests
    pub fn placed_cells(&self) -> Vec<PlacedCell> {
        (0..ROWS as i32)
            .flat_map(|y| (0..COLUMNS as i32).map(move |x| Point::new(x, y)))
            .filter_map(|point| self.get(point).map(|cell| (point, CellId::from(cell))))
            .collect()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::game::cell::PuyoColor;

    /// Build a board from rows of characters, bottom row last - the way a Puyo field is drawn.
    ///
    /// `.` is empty, `o` nuisance, and `r g b y p` the five colours. Rows shorter than the
    /// board are padded, and the whole thing is bottom-aligned, so a test only writes the
    /// stack it cares about.
    pub fn board(rows: &[&str]) -> Board {
        let mut board = Board::new();
        let top = ROWS as i32 - rows.len() as i32;
        for (j, row) in rows.iter().enumerate() {
            for (i, c) in row.chars().enumerate() {
                board.set(Point::new(i as i32, top + j as i32), parse(c));
            }
        }
        board.recompute_links();
        board
    }

    fn parse(c: char) -> Option<PuyoCell> {
        match c {
            'r' => Some(PuyoCell::loose(PuyoColor::Red)),
            'g' => Some(PuyoCell::loose(PuyoColor::Green)),
            'b' => Some(PuyoCell::loose(PuyoColor::Blue)),
            'y' => Some(PuyoCell::loose(PuyoColor::Yellow)),
            'p' => Some(PuyoCell::loose(PuyoColor::Purple)),
            'o' => Some(PuyoCell::Nuisance),
            _ => None,
        }
    }

    /// like [`board`], but written from the top of the field down rather than bottom-aligned
    pub fn board_rows(rows: &[&str]) -> Board {
        let mut board = Board::new();
        for (j, row) in rows.iter().enumerate() {
            for (i, c) in row.chars().enumerate() {
                if let Some(cell) = parse(c) {
                    board.set(Point::new(i as i32, j as i32), Some(cell));
                }
            }
        }
        board.recompute_links();
        board
    }

    /// the board as `board` would have spelt it, visible rows only
    pub fn render(board: &Board) -> Vec<String> {
        (0..ROWS as i32)
            .map(|y| {
                (0..COLUMNS as i32)
                    .map(|x| match board.get(Point::new(x, y)) {
                        None => '.',
                        Some(PuyoCell::Nuisance) => 'o',
                        Some(PuyoCell::Tray(_)) => '?',
                        Some(cell) => match cell.color().unwrap() {
                            PuyoColor::Red => 'r',
                            PuyoColor::Green => 'g',
                            PuyoColor::Blue => 'b',
                            PuyoColor::Yellow => 'y',
                            PuyoColor::Purple => 'p',
                        },
                    })
                    .collect()
            })
            .collect()
    }

    fn links_at(board: &Board, x: i32, y: i32) -> LinkMask {
        board.get(Point::new(x, y)).unwrap().links()
    }

    #[test]
    fn the_field_is_six_by_twelve_with_a_hidden_row_above() {
        assert_eq!(COLUMNS, 6);
        assert_eq!(VISIBLE_ROWS, 12);
        assert_eq!(ROWS, 13);
        // the death square is the third column of the top visible row
        assert_eq!(DEATH_SQUARE, Point::new(2, 1));
    }

    #[test]
    fn four_of_a_colour_in_a_row_pop() {
        let mut board = board(&["rrrr.."]);
        let step = board.pop().expect("a group of four pops");
        assert_eq!(step.groups.len(), 1);
        assert_eq!(step.groups[0].size, 4);
        assert_eq!(step.groups[0].color, PuyoColor::Red);
        assert_eq!(step.count(), 4);
        assert!(board.is_empty());
    }

    #[test]
    fn three_of_a_colour_do_not() {
        let mut board = board(&["rrr..."]);
        assert_eq!(board.pop(), None);
        assert_eq!(board.occupied(), 3);
    }

    /// Connectivity is orthogonal only: four puyos touching one another at the corners are
    /// four groups of one, not a group of four.
    #[test]
    fn diagonals_do_not_connect() {
        let mut checker = board(&["r.r...", ".r.r.."]);
        assert_eq!(checker.pop(), None, "corners do not join");
        assert_eq!(checker.occupied(), 4);

        // an S *is* connected, though - its middle two share a column
        let mut ess = board(&["rr....", ".rr..."]);
        assert!(ess.pop().is_some(), "an S of four is one group");

        let mut square = board(&["rr....", "rr...."]);
        assert!(square.pop().is_some(), "a square of four is one group");
    }

    #[test]
    fn nuisance_beside_a_popping_group_goes_with_it() {
        let mut board = board(&["o.....", "rrrr..", "o....."]);
        let step = board.pop().expect("pops");
        assert_eq!(step.groups[0].size, 4, "only the reds are a group");
        assert_eq!(step.nuisance, 2, "the nuisance above and below goes too");
        assert_eq!(step.count(), 6, "the clear is six cells in all");
        assert!(board.is_empty());
    }

    /// a nuisance puyo caught between two groups that both pop goes once, not twice
    #[test]
    fn nuisance_beside_two_groups_at_once_is_only_cleared_once() {
        let mut board = board(&["rrrr..", "o.....", "bbbb.."]);
        let step = board.pop().expect("both rows pop");
        assert_eq!(step.groups.len(), 2);
        assert_eq!(step.nuisance, 1, "the one puyo between them, once");
        assert_eq!(step.count(), 9, "four, four and the nuisance");
        assert!(board.is_empty());
    }

    /// ... but nuisance out of reach of the group stays
    #[test]
    fn nuisance_away_from_the_group_stays() {
        let mut board = board(&["rrrr.o"]);
        let step = board.pop().expect("pops");
        assert_eq!(step.nuisance, 0);
        assert_eq!(board.occupied(), 1);
    }

    /// nuisance has no colour, so four of it touching is not a group
    #[test]
    fn nuisance_never_groups_with_itself() {
        let mut board = board(&["oooo.."]);
        assert_eq!(board.pop(), None);
        assert_eq!(board.occupied(), 4);
    }

    #[test]
    fn two_groups_of_different_colours_pop_together() {
        let mut board = board(&["rrrrbb", "....bb"]);
        let step = board.pop().expect("pops");
        assert_eq!(step.groups.len(), 2);
        assert_eq!(step.count(), 8);
    }

    #[test]
    fn a_pop_leaves_what_was_above_it_to_fall() {
        let mut board = board(&["b.....", "r.....", "r.....", "r.....", "r....."]);
        board.pop().expect("the reds pop");
        assert!(board.settle(), "the blue falls");
        assert_eq!(
            board.get(Point::new(0, ROWS as i32 - 1)).unwrap().color(),
            Some(PuyoColor::Blue)
        );
        assert_eq!(board.occupied(), 1);
    }

    /// the chain loop: pop, settle, pop again
    #[test]
    fn a_settle_can_set_off_the_next_step() {
        // four blues sit on top of four reds in one column; the reds go, the blues land on
        // the blue already waiting beside them and go too
        let mut board = board(&[
            "b.....", "b.....", "b.....", "r.....", "r.....", "r.....", "r....", ".b....",
        ]);
        let first = board.pop().expect("the reds pop");
        assert_eq!(first.groups[0].color, PuyoColor::Red);
        assert!(board.settle());
        let second = board.pop().expect("the blues then reach each other");
        assert_eq!(second.groups[0].color, PuyoColor::Blue);
        assert_eq!(second.groups[0].size, 4);
        assert!(board.is_empty());
    }

    #[test]
    fn settling_compacts_each_column_independently() {
        let mut board = Board::new();
        board.set(Point::new(0, 0), Some(PuyoCell::loose(PuyoColor::Red)));
        board.set(Point::new(3, 5), Some(PuyoCell::Nuisance));
        assert!(board.settle());
        let floor = ROWS as i32 - 1;
        assert_eq!(
            board.get(Point::new(0, floor)).unwrap().color(),
            Some(PuyoColor::Red)
        );
        assert_eq!(board.get(Point::new(3, floor)), Some(PuyoCell::Nuisance));
        assert!(!board.settle(), "a settled board settles no further");
    }

    #[test]
    fn a_puyo_dropped_into_a_column_lands_on_the_stack() {
        let mut board = board(&["r....."]);
        let landed = board.drop_into(0, PuyoCell::Nuisance).unwrap();
        assert_eq!(landed, Point::new(0, ROWS as i32 - 2));
        assert_eq!(board.height(0), 2);
    }

    #[test]
    fn a_full_column_takes_nothing_more() {
        let mut board = Board::new();
        for _ in 0..ROWS {
            assert!(board.drop_into(0, PuyoCell::Nuisance).is_some());
        }
        assert_eq!(board.drop_into(0, PuyoCell::Nuisance), None);
        assert_eq!(board.landing(0), None);
        assert_eq!(board.height(0), ROWS);
    }

    #[test]
    fn matching_neighbours_are_joined_and_others_are_not() {
        let board = board(&["rrb..."]);
        let floor = ROWS as i32 - 1;
        assert_eq!(links_at(&board, 0, floor), LinkMask::RIGHT);
        assert_eq!(links_at(&board, 1, floor), LinkMask::LEFT);
        assert_eq!(
            links_at(&board, 2, floor),
            LinkMask::NONE,
            "a different colour"
        );
    }

    #[test]
    fn a_puyo_joins_in_all_four_directions() {
        let board = board(&[".r....", "rrr...", ".r...."]);
        let middle = links_at(&board, 1, ROWS as i32 - 2);
        assert_eq!(middle.links(), 4);
        for bit in [
            LinkMask::UP,
            LinkMask::DOWN,
            LinkMask::LEFT,
            LinkMask::RIGHT,
        ] {
            assert!(middle.has(bit));
        }
    }

    #[test]
    fn nuisance_is_joined_to_nothing_even_beside_more_nuisance() {
        let board = board(&["oo...."]);
        assert_eq!(links_at(&board, 0, ROWS as i32 - 1), LinkMask::NONE);
        assert_eq!(links_at(&board, 1, ROWS as i32 - 1), LinkMask::NONE);
    }

    /// a pop re-joins whatever was touching the group
    #[test]
    fn links_are_recomputed_after_a_pop() {
        // a lone red beside a group of four reds is joined to it; once the group goes it is
        // joined to nothing
        let floor = ROWS as i32 - 1;
        let mut five = board(&["r.....", "rrrrr."]);
        assert!(links_at(&five, 4, floor).has(LinkMask::LEFT));
        five.pop().expect("five in a row is one group");
        assert!(five.is_empty(), "all five go, being one group");

        // the blue riding on the group is joined to nothing before or after
        let mut riding = board(&["b.....", "rrrr.."]);
        assert_eq!(links_at(&riding, 0, floor - 1), LinkMask::NONE);
        riding.pop().expect("pops");
        riding.settle();
        assert_eq!(links_at(&riding, 0, floor), LinkMask::NONE);
    }

    /// ... and after a settle, on both sides of the move
    #[test]
    fn links_are_recomputed_after_a_settle() {
        let mut board = Board::new();
        let floor = ROWS as i32 - 1;
        // a red resting on the floor and another floating two above it
        board.set(Point::new(0, floor), Some(PuyoCell::loose(PuyoColor::Red)));
        board.set(
            Point::new(0, floor - 2),
            Some(PuyoCell::loose(PuyoColor::Red)),
        );
        board.recompute_links();
        assert_eq!(
            links_at(&board, 0, floor),
            LinkMask::NONE,
            "not touching yet"
        );
        board.settle();
        assert_eq!(links_at(&board, 0, floor), LinkMask::UP, "now they are");
        assert_eq!(links_at(&board, 0, floor - 1), LinkMask::DOWN);
    }

    #[test]
    fn the_death_square_ends_it_only_once_something_rests_there() {
        let mut board = Board::new();
        assert!(!board.is_dead());
        board.set(DEATH_SQUARE, Some(PuyoCell::loose(PuyoColor::Red)));
        assert!(board.is_dead());
    }

    /// The death square is the top *visible* row, not the ghost row above it.
    ///
    /// Puyo Nexus, *Basic rules*: the game acts as if there were a red X "in the square on the
    /// first row, third column", and the first row is the first one a player can see. Nothing
    /// can rest in the ghost row above it without the death square being taken first anyway,
    /// but the two are different squares and the game must not read the wrong one.
    #[test]
    fn the_ghost_row_above_the_death_square_is_not_the_death_square() {
        let mut board = Board::new();
        board.set(
            Point::new(DEATH_SQUARE.x, 0),
            Some(PuyoCell::loose(PuyoColor::Red)),
        );
        assert!(is_ghost(Point::new(DEATH_SQUARE.x, 0)));
        assert!(!board.is_dead(), "the ghost row is not the death square");
        assert!(!is_ghost(DEATH_SQUARE));
    }

    /// a full column that is *not* the third does not end the game
    #[test]
    fn filling_another_column_to_the_top_is_survivable() {
        let mut board = Board::new();
        for _ in 0..ROWS {
            board.drop_into(0, PuyoCell::Nuisance);
        }
        assert_eq!(board.height(0), ROWS);
        assert!(!board.is_dead());
    }

    /// A column with three reds in view and a fourth in the ghost row: it does not pop.
    ///
    /// This is the whole point of the rule - the chain is *held back* until the ghost drops.
    fn ghost_column() -> Board {
        board(&[
            "r.....", "r.....", "r.....", "r.....", "b.....", "g.....", "b.....", "g.....",
            "b.....", "g.....", "b.....", "g.....", "b.....",
        ])
    }

    #[test]
    fn a_group_of_four_with_a_foot_in_the_ghost_row_does_not_pop() {
        let mut board = ghost_column();
        assert!(is_ghost(Point::new(0, 0)));
        assert_eq!(
            board.pop(),
            None,
            "three in view and a ghost is not a group of four"
        );
        assert_eq!(board.popping_groups().len(), 0);
    }

    /// ... and it pops the moment the ghost falls into view, which is the technique
    #[test]
    fn a_held_back_chain_fires_once_the_ghost_drops() {
        let mut board = ghost_column();
        assert_eq!(board.pop(), None);

        // take a puyo out from under the stack: everything shifts down a row and the ghost
        // becomes an ordinary puyo
        board.set(Point::new(0, ROWS as i32 - 1), None);
        assert!(board.settle());
        let step = board.pop().expect("now it is four in view");
        assert_eq!(step.groups.len(), 1);
        assert_eq!(step.groups[0].size, 4);
        assert_eq!(step.groups[0].color, PuyoColor::Red);
    }

    /// a ghost puyo is not taken by a group popping right beneath it
    #[test]
    fn a_ghost_puyo_survives_a_pop_beside_it() {
        let mut board = board_rows(&["r.....", "rrrr.."]);
        let step = board.pop().expect("the row of four pops");
        assert_eq!(step.groups[0].size, 4, "the ghost was not one of them");
        assert_eq!(board.occupied(), 1, "and it is still there");
        assert_eq!(
            board.get(Point::new(0, 0)).unwrap().color(),
            Some(PuyoColor::Red)
        );
    }

    /// nor is ghost nuisance, which nothing clears either
    #[test]
    fn ghost_nuisance_survives_a_pop_beside_it() {
        let mut board = board_rows(&["o.....", "rrrr.."]);
        let step = board.pop().expect("pops");
        assert_eq!(step.nuisance, 0, "the ghost nuisance stayed");
        assert_eq!(board.get(Point::new(0, 0)), Some(PuyoCell::Nuisance));

        // ... whereas the same nuisance one row lower goes with the group
        let mut lower = board_rows(&["......", "o.....", "rrrr.."]);
        let step = lower.pop().expect("pops");
        assert_eq!(step.nuisance, 1);
    }

    /// the mask tells a player what will pop together, so it must not cross into the ghost row
    #[test]
    fn nothing_draws_itself_joined_to_a_ghost() {
        let board = board_rows(&["r.....", "r.....", "r....."]);
        assert_eq!(
            links_at(&board, 0, 0),
            LinkMask::NONE,
            "the ghost joins nothing"
        );
        assert_eq!(
            links_at(&board, 0, 1),
            LinkMask::DOWN,
            "and nothing joins up to it"
        );
        assert_eq!(links_at(&board, 0, 2), LinkMask::UP);
    }

    /// there is no fourteenth row: Tsu has a ceiling above the ghost row
    #[test]
    fn there_is_nothing_above_the_ghost_row() {
        for x in 0..COLUMNS as i32 {
            assert!(!Board::in_bounds(Point::new(x, -1)));
            assert!(!Board::new().is_free(Point::new(x, -1)));
        }
    }

    #[test]
    fn an_emptied_board_is_an_all_clear() {
        let mut board = board(&["rrrr.."]);
        assert!(!board.is_all_clear());
        board.pop();
        assert!(board.is_all_clear());
    }

    /// ... but nuisance left behind is not
    #[test]
    fn nuisance_left_behind_is_not_an_all_clear() {
        let mut board = board(&["rrrr.o"]);
        board.pop();
        assert!(!board.is_all_clear());
    }

    #[test]
    fn the_test_helpers_round_trip() {
        let rows = ["rrb...", "oyyp.."];
        let built = board(&rows);
        let drawn = render(&built);
        assert_eq!(&drawn[ROWS as usize - 2..], &rows[..]);
        assert!(drawn[..ROWS as usize - 2].iter().all(|r| r == "......"));
    }
}
