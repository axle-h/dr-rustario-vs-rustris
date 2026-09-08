//! The break: what a crash gem sets off, what the rainbow takes, and how power gems form.
//!
//! The verb of the game. A crash gem floods its own colour, counter gems standing beside
//! anything that breaks are taken as collateral, and whatever survives is scanned for
//! rectangles. The rules doc's *The break* and *Power gems* are where all of this is read
//! from; the two places this makes a choice the disassembly does not pin are called out below.

use crate::game::board::{Board, COLUMNS, NEIGHBOURS, ROWS};
use crate::game::cell::{Gem, GemColor, PowerGemId, PowerGemIds};
use engine::game::geometry::Point;
use std::collections::HashSet;

/// the tallest a power gem may grow, from the formation scan's own cap
pub const MAX_POWER_GEM_SIDE: i32 = 10;
/// a power gem is a rectangle of at least this on each side
pub const MIN_POWER_GEM_SIDE: i32 = 2;

/// What one erase step took off the board.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Erased {
    /// every cell that went, and what was in it
    pub cells: Vec<(Point, Gem)>,
    /// A rainbow gem came to rest on the **floor**, which is the Tech Bonus.
    ///
    /// Ten thousand points for dropping it down an empty lane, which is why the guides tell
    /// you not to spend a rainbow on an ordinary break.
    pub tech_bonus: bool,
}

impl Erased {
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty() && !self.tech_bonus
    }

    /// how many cells went
    pub fn count(&self) -> u32 {
        self.cells.len() as u32
    }

    pub fn gems(&self) -> impl Iterator<Item = Gem> + '_ {
        self.cells.iter().map(|(_, gem)| *gem)
    }

    /// the distinct colours a break spread by, which the colour bonus is paid on
    pub fn colors(&self) -> HashSet<GemColor> {
        self.gems().filter_map(|gem| gem.break_color()).collect()
    }
}

/// Which cells this step erases.
///
/// Three rules, in the order the game applies them:
///
/// 1. **Propagation.** A four-neighbour flood from every crash gem over cells of its own
///    colour. A crash gem that reached nothing is put back - one sitting alone is inert.
/// 2. **The rainbow.** It takes every gem of the colour of the cell directly under it, that
///    cell whatever it is, and itself. Over an empty lane it takes nothing and pays the Tech
///    Bonus instead.
/// 3. **Collateral.** Any counter gem orthogonally touching something that is going, goes.
///    Counter gems are never matched *by* colour - they have none - so this is the only way
///    one is ever cleared, and it is what makes them worth breaking beside.
pub fn marked(board: &Board) -> Erased {
    let mut marked: HashSet<Point> = HashSet::new();
    let mut tech_bonus = false;

    for (point, gem) in board.occupied() {
        match gem {
            Gem::Crash(color) => {
                let group = flood(board, point, color);
                if group.len() > 1 {
                    marked.extend(group);
                }
            }
            Gem::Rainbow => {
                let below = point.translate(0, 1);
                marked.insert(point);
                match board.get(below) {
                    Some(under) => {
                        marked.insert(below);
                        if let Some(color) = under.break_color() {
                            marked.extend(
                                board
                                    .occupied()
                                    .filter(|(_, gem)| gem.break_color() == Some(color))
                                    .map(|(point, _)| point),
                            );
                        }
                    }
                    // nothing under it at all means it came to rest on the floor
                    None if !Board::contains(below) => tech_bonus = true,
                    None => {}
                }
            }
            _ => {}
        }
    }

    // counter gems are taken as collateral rather than as matches
    let collateral: Vec<Point> = marked
        .iter()
        .flat_map(|point| NEIGHBOURS.map(|step| *point + step))
        .filter(|point| board.get(*point).is_some_and(|gem| gem.is_counter()))
        .collect();
    marked.extend(collateral);

    let mut cells: Vec<(Point, Gem)> = marked
        .into_iter()
        .filter_map(|point| board.get(point).map(|gem| (point, gem)))
        .collect();
    cells.sort_by_key(|(point, _)| *point);
    Erased { cells, tech_bonus }
}

/// every cell reachable from `from` through gems of `color`, the starting cell included
fn flood(board: &Board, from: Point, color: GemColor) -> HashSet<Point> {
    let mut seen = HashSet::from([from]);
    let mut queue = vec![from];
    while let Some(point) = queue.pop() {
        for step in NEIGHBOURS {
            let next = point + step;
            if seen.contains(&next) {
                continue;
            }
            if board.get(next).and_then(|gem| gem.break_color()) == Some(color) {
                seen.insert(next);
                queue.push(next);
            }
        }
    }
    seen
}

/// take everything [`marked`] found off the board
pub fn erase(board: &mut Board, erased: &Erased) {
    for (point, _) in &erased.cells {
        board.set(*point, None);
    }
    board.recheck_power_gems();
}

/// **Power gem formation, and merging, which are one search.**
///
/// The original runs six functions - three formation scans from three corners and three merge
/// scans - but they differ only in what the corner-code sum inside the candidate rectangle is
/// allowed to be, and that rule is the whole of it:
///
/// * **0** - no existing power gem inside; a fresh rectangle of plain gems.
/// * **15** - exactly one existing gem (`1 + 2 + 4 + 8`) wholly inside it; it is absorbed.
/// * **30** - exactly two, which is what the merge pass is.
///
/// Any other sum means a power gem is *partly* overlapped, and the candidate is rejected. That
/// is why a power gem gets harder to extend as it grows: you are not adding a row to a gem,
/// you are finding a larger rectangle that swallows it whole.
///
/// **[open], and read here as the largest rectangle wins.** The rules doc has the acceptance
/// rule and the caps as certain and the column-by-column growth loop as untranscribed - about
/// 1,700 bytes of pointer arithmetic per function. This takes the largest acceptable rectangle
/// at each anchor and repeats to a fixed point, which is what three passes from three corners
/// are *for*; where the original would settle on a smaller one, this will find the bigger.
pub fn form_power_gems(board: &mut Board, ids: &mut PowerGemIds) -> Vec<PowerGemId> {
    let mut formed = vec![];
    while let Some(rect) = largest_new_rectangle(board) {
        let id = ids.allocate();
        // whatever was inside is swallowed: the region is cleared and restamped as one gem
        for point in cells_of(rect) {
            if let Some(gem) = board.get(point) {
                board.set(point, Some(gem.with_power(None)));
            }
        }
        board.stamp_power_gem(id, rect);
        formed.push(id);
    }
    formed
}

fn cells_of((top_left, bottom_right): (Point, Point)) -> impl Iterator<Item = Point> {
    (top_left.y..=bottom_right.y)
        .flat_map(move |y| (top_left.x..=bottom_right.x).map(move |x| Point::new(x, y)))
}

/// the biggest rectangle anywhere on the board that would be a *new* power gem
fn largest_new_rectangle(board: &Board) -> Option<(Point, Point)> {
    let mut best: Option<(Point, Point)> = None;
    let mut best_area = 0;
    for bottom_left in Board::points() {
        for height in MIN_POWER_GEM_SIDE..=MAX_POWER_GEM_SIDE {
            let top = bottom_left.y - height + 1;
            if top < 0 {
                break;
            }
            let mut width = MIN_POWER_GEM_SIDE;
            while bottom_left.x + width - 1 < COLUMNS as i32 && width <= MAX_POWER_GEM_SIDE {
                let rect = (
                    Point::new(bottom_left.x, top),
                    Point::new(bottom_left.x + width - 1, bottom_left.y),
                );
                if !board.is_solid_one_colour(rect) {
                    break;
                }
                let area = width * height;
                if area > best_area && accepts(board, rect) {
                    best_area = area;
                    best = Some(rect);
                }
                width += 1;
            }
        }
    }
    best
}

/// **The acceptance rule.**
///
/// The game states it as a sum: the power gem corner codes inside the candidate rectangle are
/// added up in `+0x11e`, and the rectangle is accepted only when that sum is **0, 15 or 30**.
/// A whole power gem inside contributes all four of its corners and so exactly 15, and a gem
/// the rectangle only *partly* covers contributes some other number - so the sum is a
/// compact way of saying **every power gem this rectangle touches is wholly inside it, and
/// there are at most two of them**. That is what this tests, because a sum read off the
/// corners alone would also accept a rectangle sitting entirely in the middle of a large gem,
/// where there are no corners to count; the original's scan starts at a gem's own corner and
/// never asks that question.
///
/// A rectangle that is already exactly one power gem is refused as well: it is no change, and
/// accepting it would restamp the same gem for ever.
fn accepts(board: &Board, rect: (Point, Point)) -> bool {
    let mut inside: Vec<PowerGemId> = cells_of(rect)
        .filter_map(|point| board.get(point).and_then(|gem| gem.power()).map(|p| p.id))
        .collect();
    inside.sort_unstable();
    inside.dedup();
    if inside.len() > 2 {
        return false;
    }
    let area = cells_of(rect).count();
    for id in &inside {
        let cells = board.power_gem_cells(*id);
        if cells.iter().any(|point| !contains(rect, *point)) {
            return false;
        }
        if inside.len() == 1 && cells.len() == area {
            return false;
        }
    }
    true
}

/// the corner-code sum the game itself tests, for the test that holds the two readings
/// together
#[cfg(test)]
fn corner_sum(board: &Board, rect: (Point, Point)) -> u32 {
    cells_of(rect)
        .filter_map(|point| board.get(point).and_then(|gem| gem.power()))
        .filter_map(|power| power.corner)
        .map(|corner| corner.code())
        .sum()
}

fn contains((top_left, bottom_right): (Point, Point), point: Point) -> bool {
    (top_left.x..=bottom_right.x).contains(&point.x)
        && (top_left.y..=bottom_right.y).contains(&point.y)
}

/// how full the board is, as the fighter sprites read it: `+0x79`, out of the visible 78.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pressure {
    /// below 37 cells
    Clear = 0,
    /// 37 to 54
    Pressed = 1,
    /// above 54
    Buried = 2,
}

impl Pressure {
    pub fn of(board: &Board) -> Pressure {
        match board.occupied_count() {
            0..=36 => Pressure::Clear,
            37..=54 => Pressure::Pressed,
            _ => Pressure::Buried,
        }
    }
}

/// whether the board still has room for a piece: the Drop Alley is not blocked
pub fn drop_alley_is_clear(board: &Board) -> bool {
    (0..ROWS as i32)
        .take(2)
        .all(|y| board.is_free(Point::new(crate::game::board::DROP_ALLEY, y)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::board::tests::board;
    use crate::game::cell::GemColor::*;

    fn broken(rows: &[&str]) -> Erased {
        marked(&board(rows))
    }

    /// a crash gem takes the whole connected group of its own colour, itself included
    #[test]
    fn a_crash_gem_breaks_its_own_colour() {
        let erased = broken(&["rrr", "rRr"]);
        assert_eq!(erased.count(), 6);
        assert_eq!(erased.colors(), HashSet::from([Red]));
    }

    /// ... and nothing of any other
    #[test]
    fn a_crash_gem_leaves_every_other_colour_alone() {
        let erased = broken(&["ggg", "gRg"]);
        assert!(erased.is_empty(), "it reached nothing of its own colour");
    }

    /// a crash gem alone is inert, which is what makes one worth saving
    #[test]
    fn a_lone_crash_gem_breaks_nothing() {
        assert!(broken(&["R....."]).is_empty());
        assert!(
            broken(&["gR"]).is_empty(),
            "and a colour that is not its own"
        );
    }

    /// two crash gems of a colour set each other off
    #[test]
    fn two_crash_gems_of_a_colour_break_each_other() {
        assert_eq!(broken(&["RR"]).count(), 2);
    }

    /// Counter gems are collateral, never matches.
    ///
    /// StrategyWiki: "if a gem that touches a counter gem is destroyed, even if that gem is a
    /// different colour, the counter gem will be shattered".
    #[test]
    fn a_counter_gem_beside_a_break_is_shattered_whatever_colour_it_is() {
        let erased = broken(&["1r", "rR"]);
        assert_eq!(erased.count(), 4, "three reds and the blue counter gem");
        assert_eq!(
            erased.gems().filter(|gem| gem.is_counter()).count(),
            1,
            "and it went as a counter gem, so it scores as one"
        );
    }

    /// ... and one that touches nothing that broke stays put
    #[test]
    fn a_counter_gem_away_from_a_break_survives() {
        let erased = broken(&["1.", "..", "rR"]);
        assert_eq!(erased.count(), 2);
    }

    /// a counter gem does not spread a break: it is taken, and nothing beyond it is
    #[test]
    fn a_break_does_not_spread_through_a_counter_gem() {
        let erased = broken(&["r1r", "..R"]);
        assert_eq!(
            erased.count(),
            3,
            "the crash gem, the red over it and the counter gem beside that red"
        );
    }

    /// the rainbow takes every gem of the colour it lands on, wherever they are
    #[test]
    fn the_rainbow_takes_every_gem_of_the_colour_it_lands_on() {
        let erased = broken(&["*.", "gg", "rg"]);
        assert_eq!(erased.count(), 4, "the rainbow and all three greens");
        assert!(
            !erased.gems().any(|gem| gem.color() == Some(Red)),
            "and the red it was not standing on is untouched"
        );
    }

    /// down an empty lane it lands on the floor and pays ten thousand points instead
    #[test]
    fn a_rainbow_on_the_floor_is_the_tech_bonus() {
        let erased = broken(&["*....."]);
        assert!(erased.tech_bonus);
        assert!(!erased.is_empty(), "the bonus is the whole of what it did");
    }

    /// a rainbow on a counter gem takes the counter gem and nothing else: a counter gem has no
    /// break colour to spread by
    #[test]
    fn a_rainbow_on_a_counter_gem_takes_only_that_gem() {
        let erased = broken(&["*", "1"]);
        assert_eq!(erased.count(), 2, "itself and the gem it landed on");
    }

    #[test]
    fn erasing_takes_the_marked_cells_off_the_board() {
        let mut b = board(&["rrr", "rRr"]);
        let erased = marked(&b);
        erase(&mut b, &erased);
        assert!(b.is_empty());
    }

    /// a fresh rectangle of one colour is a power gem, corner sum zero
    #[test]
    fn a_solid_rectangle_of_one_colour_becomes_a_power_gem() {
        let mut b = board(&["rr", "rr"]);
        let mut ids = PowerGemIds::default();
        assert_eq!(form_power_gems(&mut b, &mut ids).len(), 1);
        assert_eq!(b.power_gem_cells(PowerGemId(1)).len(), 4);
        assert!(
            form_power_gems(&mut b, &mut ids).is_empty(),
            "and running the scan again forms nothing new"
        );
    }

    /// the biggest rectangle wins, so a 2x3 does not settle for the 2x2 inside it
    #[test]
    fn the_largest_rectangle_is_the_one_that_forms() {
        let mut b = board(&["rrr", "rrr"]);
        let mut ids = PowerGemIds::default();
        form_power_gems(&mut b, &mut ids);
        assert_eq!(b.power_gem_cells(PowerGemId(1)).len(), 6);
    }

    /// a rectangle that swallows one whole power gem is accepted - sum 15 - and the old id
    /// goes with it
    #[test]
    fn a_larger_rectangle_absorbs_a_power_gem_whole() {
        let mut b = board(&["rr", "rr"]);
        let mut ids = PowerGemIds::default();
        form_power_gems(&mut b, &mut ids);
        // a third row of red arrives under it
        let floor = ROWS as i32 - 1;
        for x in 0..2 {
            b.set(Point::new(x, floor - 2), b.get(Point::new(x, floor)));
            b.set(Point::new(x, floor), Some(Gem::plain(Red)));
        }
        b.recheck_power_gems();
        form_power_gems(&mut b, &mut ids);
        assert_eq!(b.power_gem_cells(PowerGemId(2)).len(), 6, "one gem of six");
        assert!(
            b.power_gem_cells(PowerGemId(1)).is_empty(),
            "the old id is gone"
        );
    }

    /// two whole power gems in one rectangle sum to 30, which is the merge
    #[test]
    fn two_power_gems_merge_into_one() {
        let mut b = board(&["rr..", "rr..", "..rr", "..rr"]);
        let mut ids = PowerGemIds::default();
        assert_eq!(form_power_gems(&mut b, &mut ids).len(), 2, "two of them");
        // the two columns between them fill in, and the 4x4 swallows both
        let floor = ROWS as i32 - 1;
        for y in floor - 3..=floor {
            for x in 0..4 {
                if b.get(Point::new(x, y)).is_none() {
                    b.set(Point::new(x, y), Some(Gem::plain(Red)));
                }
            }
        }
        form_power_gems(&mut b, &mut ids);
        assert_eq!(b.power_gem_cells(PowerGemId(3)).len(), 16);
    }

    /// **The two readings of the acceptance rule agree.** Whenever a rectangle is accepted,
    /// the corner-code sum the game itself tests is 0, 15 or 30; whenever it is refused for
    /// overlapping a gem, it is not.
    #[test]
    fn an_accepted_rectangle_always_has_a_corner_sum_of_zero_fifteen_or_thirty() {
        let mut b = board(&["rrrr", "rrrr", "rrrr", "rrrr"]);
        let mut ids = PowerGemIds::default();
        let mut seen = 0;
        for _ in 0..4 {
            for bottom_left in Board::points() {
                for height in MIN_POWER_GEM_SIDE..=4 {
                    for width in MIN_POWER_GEM_SIDE..=4 {
                        let rect = (
                            Point::new(bottom_left.x, bottom_left.y - height + 1),
                            Point::new(bottom_left.x + width - 1, bottom_left.y),
                        );
                        if bottom_left.y - height + 1 < 0 || !b.is_solid_one_colour(rect) {
                            continue;
                        }
                        if accepts(&b, rect) {
                            seen += 1;
                            assert!(
                                matches!(corner_sum(&b, rect), 0 | 15 | 30),
                                "{rect:?} was accepted with a corner sum of {}",
                                corner_sum(&b, rect)
                            );
                        }
                    }
                }
            }
            if largest_new_rectangle(&b).is_none() {
                break;
            }
            form_power_gems(&mut b, &mut ids);
        }
        assert!(seen > 0, "and some rectangle was accepted at all");
    }

    /// a rectangle wholly inside a bigger power gem is not a new one, however few corners it
    /// happens to cover - which is the case a plain corner-code sum reads as zero
    #[test]
    fn a_rectangle_inside_a_power_gem_forms_nothing() {
        let mut b = board(&["rrrr", "rrrr", "rrrr", "rrrr"]);
        let mut ids = PowerGemIds::default();
        assert_eq!(form_power_gems(&mut b, &mut ids).len(), 1);
        assert_eq!(b.power_gem_cells(PowerGemId(1)).len(), 16);
        assert!(
            form_power_gems(&mut b, &mut ids).is_empty(),
            "and the scan settles rather than restamping the middle of it for ever"
        );
    }

    /// a crash gem never joins a power gem: it is a class, not a colour
    #[test]
    fn a_crash_gem_is_never_part_of_a_power_gem() {
        let mut b = board(&["rr", "rR"]);
        let mut ids = PowerGemIds::default();
        assert!(form_power_gems(&mut b, &mut ids).is_empty());
    }

    /// nor does a counter gem, however it got there
    #[test]
    fn a_counter_gem_is_never_part_of_a_power_gem() {
        let mut b = board(&["44", "44"]);
        let mut ids = PowerGemIds::default();
        assert!(form_power_gems(&mut b, &mut ids).is_empty());
    }

    #[test]
    fn board_pressure_is_read_out_of_the_visible_seventy_eight() {
        let mut b = Board::new();
        assert_eq!(Pressure::of(&b), Pressure::Clear);
        for (i, point) in Board::points()
            .filter(|p| !crate::game::board::is_headroom(*p))
            .enumerate()
        {
            b.set(point, Some(Gem::plain(Red)));
            let expected = match i + 1 {
                0..=36 => Pressure::Clear,
                37..=54 => Pressure::Pressed,
                _ => Pressure::Buried,
            };
            assert_eq!(Pressure::of(&b), expected, "at {} cells", i + 1);
        }
    }
}
