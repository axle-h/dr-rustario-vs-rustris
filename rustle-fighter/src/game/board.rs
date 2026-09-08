//! The playfield: the grid, gravity, and the census that reads how buried a player is.
//!
//! Coordinates are the engine's, so `y` grows *downwards* and row 0 is the top row simulated.
//!
//! **The shape is read from the code, not from a guide.** The census and gravity passes scan
//! thirteen rows and the erase pass scans fourteen, one above the visible field, so the board
//! is six wide and thirteen tall with a row of headroom over it. Board pressure is counted out
//! of seventy-eight, which is 6 x 13 and confirms the same shape a second way. See the rules
//! doc's *Board*.

use crate::game::cell::{Corner, Gem, GemColor, PowerCell, PowerGemId, PowerMask};
use engine::game::geometry::Point;
use engine::game::pair::PairBoard;

pub const COLUMNS: u32 = 6;
/// the thirteen rows a player can see
pub const VISIBLE_ROWS: u32 = 13;
/// the row above them, which the erase pass reaches and the census does not
pub const HIDDEN_ROWS: u32 = 1;
pub const ROWS: u32 = VISIBLE_ROWS + HIDDEN_ROWS;
pub const CELLS: usize = (COLUMNS * ROWS) as usize;

/// how many cells the board pressure is read out of: the visible field, 6 x 13
pub const VISIBLE_CELLS: u32 = COLUMNS * VISIBLE_ROWS;

/// The Drop Alley: the fourth column from the left, where pieces enter.
///
/// It is confirmed twice in the executable - it is the column pieces spawn over, and it is
/// **last in all eight** of the counter gem column orderings, which is the code's version of
/// the guides' rule that the alley only fills once every other column has taken a gem. See
/// [`crate::game::counter`].
pub const DROP_ALLEY: i32 = 3;

/// where a pair's pivot appears; its child sits in the headroom row above
pub const SPAWN: Point = Point::new(DROP_ALLEY, HIDDEN_ROWS as i32);

/// Is this the headroom row above the visible field?
///
/// It is not Puyo's ghost row - a gem here is perfectly ordinary and the erase pass reaches it.
/// The one thing it does is refuse an upright rotation, which is the ceiling rule we take from
/// Puyo along with the rest of the rotation; see the rules doc's *Deliberate deviations* and
/// [`engine::game::pair::PairBoard::is_ceiling`].
pub fn is_headroom(point: Point) -> bool {
    point.y < HIDDEN_ROWS as i32
}

/// the four orthogonal neighbours, which is how a break spreads and how a counter gem is
/// caught in one
pub const NEIGHBOURS: [Point; 4] = [
    Point::new(0, -1),
    Point::new(0, 1),
    Point::new(-1, 0),
    Point::new(1, 0),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    cells: [Option<Gem>; CELLS],
}

impl Default for Board {
    fn default() -> Self {
        Board::new()
    }
}

impl Board {
    pub fn new() -> Board {
        Board {
            cells: [None; CELLS],
        }
    }

    fn index(point: Point) -> Option<usize> {
        ((0..COLUMNS as i32).contains(&point.x) && (0..ROWS as i32).contains(&point.y))
            .then(|| (point.y * COLUMNS as i32 + point.x) as usize)
    }

    pub fn contains(point: Point) -> bool {
        Board::index(point).is_some()
    }

    pub fn get(&self, point: Point) -> Option<Gem> {
        Board::index(point).and_then(|i| self.cells[i])
    }

    pub fn set(&mut self, point: Point, gem: Option<Gem>) {
        if let Some(i) = Board::index(point) {
            self.cells[i] = gem;
        }
    }

    /// free *and* on the board; anything off the board counts as occupied, so a piece cannot
    /// be moved into it
    pub fn is_free(&self, point: Point) -> bool {
        Board::index(point).is_some_and(|i| self.cells[i].is_none())
    }

    pub fn points() -> impl Iterator<Item = Point> {
        (0..ROWS as i32).flat_map(|y| (0..COLUMNS as i32).map(move |x| Point::new(x, y)))
    }

    /// every occupied cell and what is in it, top row first
    pub fn occupied(&self) -> impl Iterator<Item = (Point, Gem)> + '_ {
        Board::points().filter_map(|point| self.get(point).map(|gem| (point, gem)))
    }

    /// Is the board empty? This is what an All Clear is worth six counter gems for.
    pub fn is_empty(&self) -> bool {
        self.cells.iter().all(Option::is_none)
    }

    /// How many of the visible field's seventy-eight cells are taken, `+0x13e`.
    ///
    /// The headroom row is not counted, which is what makes the pressure thresholds read out
    /// of 78 rather than 84.
    pub fn occupied_count(&self) -> u32 {
        self.occupied()
            .filter(|(point, _)| !is_headroom(*point))
            .count() as u32
    }

    /// **Gravity.** Every gem falls as far as its column allows, keeping its order.
    ///
    /// Returns whether anything moved, which is what the chain loop asks: a settle that moved
    /// nothing cannot have created a new break.
    ///
    /// A power gem falls as a unit in the original because it is a solid rectangle sitting on
    /// a solid rectangle - column-wise compaction gives the same answer while the rectangle is
    /// intact, and a rectangle that has been partly broken is no longer a power gem. The
    /// ordering the original settles in is [open] in the rules doc; this settles every column
    /// independently, which cannot disagree with it about where a gem ends up.
    pub fn settle(&mut self) -> bool {
        let mut moved = false;
        for x in 0..COLUMNS as i32 {
            let mut write = ROWS as i32 - 1;
            for y in (0..ROWS as i32).rev() {
                let from = Point::new(x, y);
                if let Some(gem) = self.get(from) {
                    let to = Point::new(x, write);
                    if to != from {
                        self.set(from, None);
                        self.set(to, Some(gem));
                        moved = true;
                    }
                    write -= 1;
                }
            }
        }
        if moved {
            // a rectangle that fell past a hole is not a rectangle any more, and the array
            // that says which cells are in which power gem has to agree with the board
            self.recheck_power_gems();
        }
        moved
    }

    /// One tick of every counter gem's countdown, run when the receiver drops a piece.
    pub fn tick_countdowns(&mut self) {
        for point in Board::points() {
            if let Some(gem) = self.get(point) {
                self.set(point, Some(gem.tick_countdown()));
            }
        }
    }

    /// **Which of this cell's edges are interior to its power gem**, for the sheet.
    ///
    /// Computed on demand rather than stored, so it cannot go stale: every settle, break and
    /// formation would otherwise have to remember to refresh it, which is a whole class of
    /// bug the board simply does not have this way.
    pub fn power_mask(&self, point: Point) -> PowerMask {
        let Some(id) = self.get(point).and_then(|gem| gem.power()).map(|p| p.id) else {
            return PowerMask::NONE;
        };
        let joined = |step: Point, bit: PowerMask| {
            let shares = self
                .get(point + step)
                .and_then(|gem| gem.power())
                .is_some_and(|power| power.id == id);
            if shares {
                bit
            } else {
                PowerMask::NONE
            }
        };
        joined(Point::new(0, -1), PowerMask::UP)
            .with(joined(Point::new(0, 1), PowerMask::DOWN))
            .with(joined(Point::new(-1, 0), PowerMask::LEFT))
            .with(joined(Point::new(1, 0), PowerMask::RIGHT))
    }

    /// the cells of one power gem, wherever they have ended up
    pub fn power_gem_cells(&self, id: PowerGemId) -> Vec<Point> {
        self.occupied()
            .filter(|(_, gem)| gem.power().is_some_and(|power| power.id == id))
            .map(|(point, _)| point)
            .collect()
    }

    /// Stamp `rect` as one power gem, clearing whatever membership was there before.
    ///
    /// `rect` is `(top_left, bottom_right)` inclusive, and every cell in it is expected to be
    /// a plain gem of one colour - [`crate::game::gems`] is what establishes that.
    pub fn stamp_power_gem(&mut self, id: PowerGemId, (top_left, bottom_right): (Point, Point)) {
        for y in top_left.y..=bottom_right.y {
            for x in top_left.x..=bottom_right.x {
                let point = Point::new(x, y);
                let (left, right) = (x == top_left.x, x == bottom_right.x);
                let (top, bottom) = (y == top_left.y, y == bottom_right.y);
                let corner = match (top, bottom, left, right) {
                    (true, _, true, _) => Some(Corner::TopLeft),
                    (true, _, _, true) => Some(Corner::TopRight),
                    (_, true, true, _) => Some(Corner::BottomLeft),
                    (_, true, _, true) => Some(Corner::BottomRight),
                    _ => None,
                };
                if let Some(gem) = self.get(point) {
                    self.set(point, Some(gem.with_power(Some(PowerCell { id, corner }))));
                }
            }
        }
    }

    /// **The census**: any power gem that is no longer a solid rectangle stops being one.
    ///
    /// The original does this by clearing the power gem array entry of every cell whose `0x80`
    /// bit is not set and rebuilding from what survives; the effect is the same, and stating it
    /// as "a power gem is a rectangle or it is nothing" is what the rest of the code relies on.
    pub fn recheck_power_gems(&mut self) {
        let mut ids: Vec<PowerGemId> = self
            .occupied()
            .filter_map(|(_, gem)| gem.power().map(|power| power.id))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        for id in ids {
            let cells = self.power_gem_cells(id);
            match rectangle(&cells) {
                Some(rect) if self.is_solid_one_colour(rect) => self.stamp_power_gem(id, rect),
                _ => {
                    for point in cells {
                        if let Some(gem) = self.get(point) {
                            self.set(point, Some(gem.with_power(None)));
                        }
                    }
                }
            }
        }
    }

    /// is every cell of this rectangle a plain gem of one colour?
    pub fn is_solid_one_colour(&self, (top_left, bottom_right): (Point, Point)) -> bool {
        let mut color: Option<GemColor> = None;
        for y in top_left.y..=bottom_right.y {
            for x in top_left.x..=bottom_right.x {
                match self.get(Point::new(x, y)) {
                    Some(gem) if gem.can_join_power_gem() => {
                        let gem_color = gem.color();
                        if color.is_some() && color != gem_color {
                            return false;
                        }
                        color = gem_color;
                    }
                    _ => return false,
                }
            }
        }
        true
    }
}

/// the bounding rectangle of `cells`, if the cells fill it exactly
fn rectangle(cells: &[Point]) -> Option<(Point, Point)> {
    let (first, rest) = cells.split_first()?;
    let mut top_left = *first;
    let mut bottom_right = *first;
    for point in rest {
        top_left = Point::new(top_left.x.min(point.x), top_left.y.min(point.y));
        bottom_right = Point::new(bottom_right.x.max(point.x), bottom_right.y.max(point.y));
    }
    let area = (bottom_right.x - top_left.x + 1) * (bottom_right.y - top_left.y + 1);
    (area as usize == cells.len()).then_some((top_left, bottom_right))
}

/// The board as the pair's movement sees it.
///
/// The ceiling is the headroom row, which is Puyo's current-row check standing in for the
/// PlayStation game's own spawn-area guard - see [`is_headroom`].
impl PairBoard for Board {
    fn is_free(&self, point: Point) -> bool {
        Board::is_free(self, point)
    }

    fn is_ceiling(&self, pivot: Point) -> bool {
        is_headroom(pivot)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::game::cell::GemColor::*;

    /// A board written out as rows of characters, bottom row **last**.
    ///
    /// `b y g r` are plain gems, `B Y G R` their crash gems, `1-4` a counter gem of that
    /// colour with five on the clock, `*` the rainbow and `.` an empty cell. Rows are laid
    /// against the *floor*, so `board(&["rrrrrr"])` is one row of red on the bottom.
    pub fn board(rows: &[&str]) -> Board {
        let mut board = Board::new();
        let floor = ROWS as i32 - 1;
        for (up, row) in rows.iter().rev().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                let gem = match ch {
                    '.' => None,
                    'b' => Some(Gem::plain(Blue)),
                    'y' => Some(Gem::plain(Yellow)),
                    'g' => Some(Gem::plain(Green)),
                    'r' => Some(Gem::plain(Red)),
                    'B' => Some(Gem::Crash(Blue)),
                    'Y' => Some(Gem::Crash(Yellow)),
                    'G' => Some(Gem::Crash(Green)),
                    'R' => Some(Gem::Crash(Red)),
                    '*' => Some(Gem::Rainbow),
                    '1'..='4' => Some(Gem::counter(
                        GemColor::from_game_index(ch as u8 - b'0').expect("a colour"),
                        crate::game::cell::COUNTER_COUNTDOWN,
                    )),
                    other => panic!("no gem is written `{other}`"),
                };
                board.set(Point::new(x as i32, floor - up as i32), gem);
            }
        }
        board
    }

    #[test]
    fn the_board_is_six_by_thirteen_with_a_row_of_headroom() {
        assert_eq!(COLUMNS, 6);
        assert_eq!(VISIBLE_ROWS, 13);
        assert_eq!(
            VISIBLE_CELLS, 78,
            "which is what board pressure is read out of"
        );
        assert!(is_headroom(Point::new(0, 0)));
        assert!(!is_headroom(Point::new(0, 1)), "the top visible row is not");
    }

    #[test]
    fn pieces_enter_down_the_drop_alley() {
        assert_eq!(SPAWN.x, DROP_ALLEY);
        assert_eq!(DROP_ALLEY, 3, "the fourth column from the left");
    }

    #[test]
    fn gravity_drops_every_gem_as_far_as_its_column_allows() {
        let mut board = board(&["r.g.", "....", "..b."]);
        assert!(board.settle());
        let floor = ROWS as i32 - 1;
        assert_eq!(board.get(Point::new(0, floor)), Some(Gem::plain(Red)));
        assert_eq!(board.get(Point::new(2, floor)), Some(Gem::plain(Blue)));
        assert_eq!(board.get(Point::new(2, floor - 1)), Some(Gem::plain(Green)));
        assert!(!board.settle(), "and a settled board settles no further");
    }

    #[test]
    fn the_headroom_row_is_not_counted_in_board_pressure() {
        let mut board = Board::new();
        board.set(Point::new(0, 0), Some(Gem::plain(Red)));
        assert_eq!(board.occupied_count(), 0);
        board.set(Point::new(0, 1), Some(Gem::plain(Red)));
        assert_eq!(board.occupied_count(), 1);
    }

    /// a power gem is a rectangle or it is nothing: break a hole in one and the rest stops
    /// being a power gem at all
    #[test]
    fn a_power_gem_that_stops_being_a_rectangle_stops_being_a_power_gem() {
        let mut board = board(&["rr", "rr"]);
        let floor = ROWS as i32 - 1;
        let rect = (Point::new(0, floor - 1), Point::new(1, floor));
        board.stamp_power_gem(PowerGemId(1), rect);
        assert!(board.get(Point::new(0, floor)).unwrap().power().is_some());

        board.set(Point::new(1, floor), None);
        board.recheck_power_gems();
        for point in [
            Point::new(0, floor),
            Point::new(0, floor - 1),
            Point::new(1, floor - 1),
        ] {
            assert_eq!(board.get(point).unwrap().power(), None);
        }
    }

    /// ... and the four corners of one that survives still sum to fifteen, which is the whole
    /// of the formation acceptance rule
    #[test]
    fn a_stamped_power_gem_carries_four_corners_summing_to_fifteen() {
        let mut board = board(&["rrr", "rrr"]);
        let floor = ROWS as i32 - 1;
        board.stamp_power_gem(
            PowerGemId(7),
            (Point::new(0, floor - 1), Point::new(2, floor)),
        );
        let sum: u32 = board
            .occupied()
            .filter_map(|(_, gem)| gem.power())
            .filter_map(|power| power.corner)
            .map(|corner| corner.code())
            .sum();
        assert_eq!(sum, 15);
        assert_eq!(board.power_gem_cells(PowerGemId(7)).len(), 6);
    }

    /// the mask says which edges are interior, so a corner of a power gem is joined two ways
    /// and a cell in the middle of one all four
    #[test]
    fn the_power_mask_reads_a_rectangles_own_edges() {
        let mut b = board(&["rrr", "rrr", "rrr"]);
        let floor = ROWS as i32 - 1;
        b.stamp_power_gem(
            PowerGemId(1),
            (Point::new(0, floor - 2), Point::new(2, floor)),
        );
        assert_eq!(
            b.power_mask(Point::new(1, floor - 1)),
            PowerMask::UP
                .with(PowerMask::DOWN)
                .with(PowerMask::LEFT)
                .with(PowerMask::RIGHT),
            "the middle is joined on every side"
        );
        assert_eq!(
            b.power_mask(Point::new(0, floor)),
            PowerMask::UP.with(PowerMask::RIGHT),
            "and the bottom left corner only inwards"
        );
        b.set(Point::new(0, floor), None);
        b.recheck_power_gems();
        assert_eq!(
            b.power_mask(Point::new(1, floor - 1)),
            PowerMask::NONE,
            "and a gem that stopped being a rectangle draws joined to nothing"
        );
    }

    #[test]
    fn every_counter_gem_ticks_when_the_receiver_drops_a_piece() {
        let mut board = board(&["1r"]);
        board.tick_countdowns();
        let floor = ROWS as i32 - 1;
        assert_eq!(
            board.get(Point::new(0, floor)),
            Some(Gem::counter(Blue, 4)),
            "the counter gem counted down"
        );
        assert_eq!(
            board.get(Point::new(1, floor)),
            Some(Gem::plain(Red)),
            "and the ordinary gem did not notice"
        );
    }
}
