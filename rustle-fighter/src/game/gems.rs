//! The break and power gem formation, from the rules doc's *The break* and *Power gems*.

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
    pub cells: Vec<(Point, Gem)>,
    /// a rainbow gem came to rest on the floor: the Tech Bonus
    pub tech_bonus: bool,
}

impl Erased {
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty() && !self.tech_bonus
    }

    pub fn count(&self) -> u32 {
        self.cells.len() as u32
    }

    pub fn gems(&self) -> impl Iterator<Item = Gem> + '_ {
        self.cells.iter().map(|(_, gem)| *gem)
    }

    /// the distinct colours a break spread by, which the colour bonus pays on
    pub fn colors(&self) -> HashSet<GemColor> {
        self.gems().filter_map(|gem| gem.break_color()).collect()
    }
}

/// Which cells this step erases, in the game's order: each crash gem floods its own colour
/// (alone it is inert), each rainbow takes the cell under it and that cell's colour, then every
/// counter gem touching an erased cell goes as collateral.
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
                    None if !Board::contains(below) => tech_bonus = true,
                    None => {}
                }
            }
            _ => {}
        }
    }

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

pub fn erase(board: &mut Board, erased: &Erased) {
    for (point, _) in &erased.cells {
        board.set(*point, None);
    }
    board.recheck_power_gems();
}

/// Power gem formation and merging, which are one search: repeatedly stamp the largest
/// rectangle `accepts` allows until none is left. The original's growth loop is not
/// transcribed, so where it would settle on a smaller rectangle this finds the bigger.
pub fn form_power_gems(board: &mut Board, ids: &mut PowerGemIds) -> Vec<PowerGemId> {
    let mut formed = vec![];
    while let Some(rect) = largest_new_rectangle(board) {
        let id = ids.allocate();
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

/// The game accepts a rectangle whose corner-code sum (`+0x11e`) is 0, 15 or 30, meaning at
/// most two power gems, each wholly inside. This tests that meaning rather than the sum, which
/// would also accept a rectangle in the middle of a gem, and refuses one that already is a gem.
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

/// the corner-code sum the game itself tests
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

/// How full the board is, as the fighter sprites read it: `+0x79`, out of the visible 78.
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

/// whether the Drop Alley still has room for a piece
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

    /// a crash gem takes its connected group of its own colour
    #[test]
    fn a_crash_gem_breaks_its_own_colour() {
        let erased = broken(&["rrr", "rRr"]);
        assert_eq!(erased.count(), 6);
        assert_eq!(erased.colors(), HashSet::from([Red]));
    }

    /// a crash gem leaves other colours alone
    #[test]
    fn a_crash_gem_leaves_every_other_colour_alone() {
        let erased = broken(&["ggg", "gRg"]);
        assert!(erased.is_empty(), "it reached nothing of its own colour");
    }

    /// a crash gem touching nothing of its colour is inert
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

    /// a counter gem touching a break goes with it, whatever its colour
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

    /// a counter gem touching nothing that broke survives
    #[test]
    fn a_counter_gem_away_from_a_break_survives() {
        let erased = broken(&["1.", "..", "rR"]);
        assert_eq!(erased.count(), 2);
    }

    /// a break does not spread through a counter gem
    #[test]
    fn a_break_does_not_spread_through_a_counter_gem() {
        let erased = broken(&["r1r", "..R"]);
        assert_eq!(
            erased.count(),
            3,
            "the crash gem, the red over it and the counter gem beside that red"
        );
    }

    #[test]
    fn the_rainbow_takes_every_gem_of_the_colour_it_lands_on() {
        let erased = broken(&["*.", "gg", "rg"]);
        assert_eq!(erased.count(), 4, "the rainbow and all three greens");
        assert!(
            !erased.gems().any(|gem| gem.color() == Some(Red)),
            "and the red it was not standing on is untouched"
        );
    }

    /// a rainbow on the floor pays the Tech Bonus
    #[test]
    fn a_rainbow_on_the_floor_is_the_tech_bonus() {
        let erased = broken(&["*....."]);
        assert!(erased.tech_bonus);
        assert!(!erased.is_empty(), "the bonus is the whole of what it did");
    }

    /// a rainbow on a counter gem takes only that gem
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

    /// a solid 2x2 forms one power gem, once
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

    /// a 2x3 forms whole rather than as the 2x2 inside it
    #[test]
    fn the_largest_rectangle_is_the_one_that_forms() {
        let mut b = board(&["rrr", "rrr"]);
        let mut ids = PowerGemIds::default();
        form_power_gems(&mut b, &mut ids);
        assert_eq!(b.power_gem_cells(PowerGemId(1)).len(), 6);
    }

    /// a larger rectangle absorbs a whole power gem and its old id goes
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

    /// a 4x4 swallowing two 2x2 power gems merges them
    #[test]
    fn two_power_gems_merge_into_one() {
        let mut b = board(&["rr..", "rr..", "..rr", "..rr"]);
        let mut ids = PowerGemIds::default();
        assert_eq!(form_power_gems(&mut b, &mut ids).len(), 2, "two of them");
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

    /// every rectangle [`accepts`] takes on a 4x4 has the game's corner sum of 0, 15 or 30
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

    /// a rectangle inside a formed 4x4 power gem forms nothing new
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

    /// a crash gem never joins a power gem
    #[test]
    fn a_crash_gem_is_never_part_of_a_power_gem() {
        let mut b = board(&["rr", "rR"]);
        let mut ids = PowerGemIds::default();
        assert!(form_power_gems(&mut b, &mut ids).is_empty());
    }

    /// nor does a counter gem
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
