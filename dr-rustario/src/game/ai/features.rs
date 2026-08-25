//! What the network sees. Dr. Mario has no line clears, so the shape of the stack matters far
//! less than its colour structure: whether viruses are still reachable, and how close the
//! bottle is to a match.

use crate::game::bottle::{Bottle, BOTTLE_HEIGHT, BOTTLE_WIDTH};
use crate::game::geometry::BottlePoint;
use crate::game::pill::VirusColor;
use std::ops::Sub;

/// a run this long or longer clears, so a run one short is a threat
const MATCH_LENGTH: usize = 4;

/// what a virus no line can reach any more counts as, in blocks of work
const UNREACHABLE_VIRUS: i32 = 2 * MATCH_LENGTH as i32;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct BottleStats {
    viruses: i32,
    virus_near_3: i32,
    virus_near_2: i32,
    block_near_3: i32,
    block_near_2: i32,
    buried_viruses: i32,
    buried_blocks: i32,
    max_height: i32,
    virus_work: i32,
    holes: i32,
}

impl BottleStats {
    pub fn viruses(&self) -> i32 {
        self.viruses
    }
    /// same colour runs one short of a match that would take a virus with them
    pub fn virus_near_3(&self) -> i32 {
        self.virus_near_3
    }
    /// same colour runs two short of a match that would take a virus with them
    pub fn virus_near_2(&self) -> i32 {
        self.virus_near_2
    }
    /// the same, for runs with no virus in them: clearing those only tidies the bottle up
    pub fn block_near_3(&self) -> i32 {
        self.block_near_3
    }
    pub fn block_near_2(&self) -> i32 {
        self.block_near_2
    }
    /// differently coloured blocks stacked on top of viruses
    pub fn buried_viruses(&self) -> i32 {
        self.buried_viruses
    }
    /// the same, for everything that is not a virus
    pub fn buried_blocks(&self) -> i32 {
        self.buried_blocks
    }
    pub fn max_height(&self) -> i32 {
        self.max_height
    }
    /// how many more matching blocks the bottle still needs to clear every virus
    pub fn virus_work(&self) -> i32 {
        self.virus_work
    }
    pub fn holes(&self) -> i32 {
        self.holes
    }
}

impl Sub<BottleStats> for BottleStats {
    type Output = BottleStats;

    fn sub(self, rhs: BottleStats) -> Self::Output {
        BottleStats {
            viruses: self.viruses - rhs.viruses,
            virus_near_3: self.virus_near_3 - rhs.virus_near_3,
            virus_near_2: self.virus_near_2 - rhs.virus_near_2,
            block_near_3: self.block_near_3 - rhs.block_near_3,
            block_near_2: self.block_near_2 - rhs.block_near_2,
            buried_viruses: self.buried_viruses - rhs.buried_viruses,
            buried_blocks: self.buried_blocks - rhs.buried_blocks,
            max_height: self.max_height - rhs.max_height,
            virus_work: self.virus_work - rhs.virus_work,
            holes: self.holes - rhs.holes,
        }
    }
}

/// the stats of the settled bottle, how they moved, and what the placement itself did
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct BottleFeatures {
    global: BottleStats,
    delta: BottleStats,
    /// halves of the placed pill that ended up touching nothing of their own colour
    wasted_halves: i32,
    /// runs cleared by the placement, cascades included
    patterns_cleared: i32,
}

impl BottleFeatures {
    pub fn new(
        global: BottleStats,
        before: BottleStats,
        wasted_halves: i32,
        patterns_cleared: i32,
    ) -> Self {
        Self {
            global,
            delta: global - before,
            wasted_halves,
            patterns_cleared,
        }
    }

    pub fn global(&self) -> BottleStats {
        self.global
    }
    pub fn delta(&self) -> BottleStats {
        self.delta
    }
    pub fn wasted_halves(&self) -> i32 {
        self.wasted_halves
    }
    pub fn patterns_cleared(&self) -> i32 {
        self.patterns_cleared
    }
}

pub trait BottleAnalysis {
    fn stats(&self) -> BottleStats;
}

impl BottleAnalysis for Bottle {
    fn stats(&self) -> BottleStats {
        let colors = self.color_grid();

        let mut column_heights = [0i32; BOTTLE_WIDTH as usize];
        let mut holes = 0;
        let mut viruses = 0;
        let mut buried_viruses = 0;
        let mut buried_blocks = 0;

        for x in 0..BOTTLE_WIDTH {
            let mut stacked = false;
            let mut above: Vec<VirusColor> = vec![];
            for y in 0..BOTTLE_HEIGHT {
                match colors[index(x, y)] {
                    Some(color) => {
                        if !stacked {
                            stacked = true;
                            column_heights[x as usize] = (BOTTLE_HEIGHT - y) as i32;
                        }
                        // everything of another colour above a cell has to clear before that
                        // cell can be reached from above
                        let buried = above.iter().filter(|c| **c != color).count() as i32;
                        if self.block_at(x, y).is_virus() {
                            viruses += 1;
                            buried_viruses += buried;
                        } else {
                            buried_blocks += buried;
                        }
                        above.push(color);
                    }
                    None => {
                        if stacked {
                            holes += 1;
                        }
                    }
                }
            }
        }

        let (virus_near_3, virus_near_2, block_near_3, block_near_2) =
            self.near_matches(&colors);

        BottleStats {
            viruses,
            virus_near_3,
            virus_near_2,
            block_near_3,
            block_near_2,
            buried_viruses,
            buried_blocks,
            max_height: column_heights.iter().copied().max().unwrap_or(0),
            virus_work: self.virus_work(&colors),
            holes,
        }
    }
}

trait ColorGrid {
    fn color_grid(&self) -> Vec<Option<VirusColor>>;
    fn near_matches(&self, colors: &[Option<VirusColor>]) -> (i32, i32, i32, i32);
    fn virus_work(&self, colors: &[Option<VirusColor>]) -> i32;
}

impl ColorGrid for Bottle {
    /// the colour of every settled block, ignoring the pill in play exactly as the matcher does
    fn color_grid(&self) -> Vec<Option<VirusColor>> {
        (0..BOTTLE_HEIGHT)
            .flat_map(|y| (0..BOTTLE_WIDTH).map(move |x| (x, y)))
            .map(|(x, y)| self.block_at(x, y).destructible_color())
            .collect()
    }

    /// Runs one and two short of a match that have somewhere to finish - a run walled in at
    /// both ends is never going to clear, so counting it would only teach the model to build
    /// junk. Runs that would take a virus with them are counted apart from runs that would only
    /// tidy the bottle up, since the two are worth very different things.
    ///
    /// Returns `(virus 3, virus 2, block 3, block 2)`.
    fn near_matches(&self, colors: &[Option<VirusColor>]) -> (i32, i32, i32, i32) {
        let mut virus_three = 0;
        let mut virus_two = 0;
        let mut block_three = 0;
        let mut block_two = 0;

        let rows = (0..BOTTLE_HEIGHT)
            .map(|y| (0..BOTTLE_WIDTH).map(|x| (x, y)).collect::<Vec<_>>());
        let cols = (0..BOTTLE_WIDTH)
            .map(|x| (0..BOTTLE_HEIGHT).map(|y| (x, y)).collect::<Vec<_>>());

        for line in rows.chain(cols) {
            let color_at = |i: usize| colors[index(line[i].0, line[i].1)];
            let mut i = 0;
            while i < line.len() {
                let Some(color) = color_at(i) else {
                    i += 1;
                    continue;
                };

                let mut end = i;
                while end + 1 < line.len() && color_at(end + 1) == Some(color) {
                    end += 1;
                }
                let length = end - i + 1;

                if length == MATCH_LENGTH - 1 || length == MATCH_LENGTH - 2 {
                    let has_virus =
                        (i..=end).any(|k| self.block_at(line[k].0, line[k].1).is_virus());
                    let mut room = 0;
                    for k in (0..i).rev() {
                        if color_at(k).is_some() {
                            break;
                        }
                        room += 1;
                    }
                    for k in end + 1..line.len() {
                        if color_at(k).is_some() {
                            break;
                        }
                        room += 1;
                    }

                    if room >= MATCH_LENGTH - length {
                        match (has_virus, length == MATCH_LENGTH - 1) {
                            (true, true) => virus_three += 1,
                            (true, false) => virus_two += 1,
                            (false, true) => block_three += 1,
                            (false, false) => block_two += 1,
                        }
                    }
                }

                i = end + 1;
            }
        }

        (virus_three, virus_two, block_three, block_two)
    }

    /// How much work the bottle still needs: for every virus, the fewest matching blocks that
    /// would have to be added to complete a line of four through it, summed over all of them.
    /// This is the feature that points the agent at the viruses. Counting same colour
    /// neighbours anywhere instead only teaches it to build tidy heaps in the corner.
    fn virus_work(&self, colors: &[Option<VirusColor>]) -> i32 {
        let mut work = 0;

        for y in 0..BOTTLE_HEIGHT as i32 {
            for x in 0..BOTTLE_WIDTH as i32 {
                if !self.block_at(x as u32, y as u32).is_virus() {
                    continue;
                }
                let color = colors[index(x as u32, y as u32)];

                // a virus nothing can reach any more costs more than any reachable one ever
                // could, so walling one in is always the worst thing a placement can do
                let mut fewest = UNREACHABLE_VIRUS;
                for (dx, dy) in [(1, 0), (0, 1)] {
                    for offset in 0..MATCH_LENGTH as i32 {
                        let (sx, sy) = (x - dx * offset, y - dy * offset);
                        let (ex, ey) = (
                            sx + dx * (MATCH_LENGTH as i32 - 1),
                            sy + dy * (MATCH_LENGTH as i32 - 1),
                        );
                        if sx < 0
                            || sy < 0
                            || ex >= BOTTLE_WIDTH as i32
                            || ey >= BOTTLE_HEIGHT as i32
                        {
                            continue;
                        }

                        let mut needed = 0;
                        let mut blocked = false;
                        for step in 0..MATCH_LENGTH as i32 {
                            let cell = colors[index((sx + dx * step) as u32, (sy + dy * step) as u32)];
                            match cell {
                                // already the right colour, nothing to do for this cell
                                c if c == color => (),
                                // an empty cell is one block of work
                                None => needed += 1,
                                // another colour is in the way: this line is not happening
                                Some(_) => {
                                    blocked = true;
                                    break;
                                }
                            }
                        }
                        if !blocked {
                            fewest = fewest.min(needed);
                        }
                    }
                }
                work += fewest;
            }
        }

        work
    }
}

fn index(x: u32, y: u32) -> usize {
    (y * BOTTLE_WIDTH + x) as usize
}

/// Halves of the placement that were thrown away: a half earns its place only if it joins a
/// run of its own colour that a virus of that colour is part of. Dropping a blue half somewhere
/// no blue virus can ever be reached from is a wasted pill, however tidily it stacks - and a
/// double blue while a blue virus is still in the bottle is the clearest waste of all.
pub fn wasted_halves(bottle: &Bottle, placed: &[BottlePoint]) -> i32 {
    placed
        .iter()
        .filter(|point| !joins_a_virus_chain(bottle, **point))
        .count() as i32
}

/// whether the same coloured blocks connected to `point` include a virus
fn joins_a_virus_chain(bottle: &Bottle, point: BottlePoint) -> bool {
    let Some(color) = bottle.block(point).destructible_color() else {
        // it cleared, which is the opposite of wasted
        return true;
    };

    let mut seen = vec![point];
    let mut to_visit = vec![point];
    while let Some(point) = to_visit.pop() {
        if bottle.block(point).is_virus() {
            return true;
        }
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let neighbour = point.translate(dx, dy);
            if !in_bottle(neighbour)
                || seen.contains(&neighbour)
                || bottle.block(neighbour).destructible_color() != Some(color)
            {
                continue;
            }
            seen.push(neighbour);
            to_visit.push(neighbour);
        }
    }

    false
}

fn in_bottle(point: BottlePoint) -> bool {
    point.x() >= 0
        && point.x() < BOTTLE_WIDTH as i32
        && point.y() >= 0
        && point.y() < BOTTLE_HEIGHT as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::block::Block;
    use crate::game::geometry::Rotation;
    use crate::game::pill::VitaminOrdinal;
    use VirusColor::{Blue, Red, Yellow};

    fn bottle(blocks: &[(u32, u32, Block)]) -> Bottle {
        let mut bottle = Bottle::new();
        for (x, y, block) in blocks {
            bottle.place(*x, *y, *block);
        }
        bottle
    }

    fn virus(x: u32, y: u32, color: VirusColor) -> (u32, u32, Block) {
        (x, y, Block::Virus(color))
    }

    fn garbage(x: u32, y: u32, color: VirusColor) -> (u32, u32, Block) {
        (x, y, Block::Garbage(color))
    }

    fn stack(x: u32, y: u32, color: VirusColor) -> (u32, u32, Block) {
        (
            x,
            y,
            Block::Stack(color, Rotation::North, VitaminOrdinal::Left),
        )
    }

    #[test]
    fn empty_bottle_has_nothing_in_it() {
        assert_eq!(bottle(&[]).stats(), BottleStats::default());
    }

    #[test]
    fn counts_viruses() {
        let stats = bottle(&[
            virus(0, 15, Red),
            virus(1, 15, Blue),
            garbage(2, 15, Yellow),
        ])
        .stats();
        assert_eq!(stats.viruses(), 2);
    }

    #[test]
    fn a_virus_is_buried_only_by_other_colours() {
        // a red virus under one red and two blue blocks: only the blues have to clear first
        let stats = bottle(&[
            stack(0, 12, Blue),
            stack(0, 13, Blue),
            stack(0, 14, Red),
            virus(0, 15, Red),
        ])
        .stats();
        assert_eq!(stats.buried_viruses(), 2);
    }

    #[test]
    fn near_matches_are_counted_apart_by_whether_a_virus_is_in_them() {
        // three reds in a row with no virus: clearing it would only tidy the bottle up
        let no_virus = bottle(&[stack(0, 15, Red), stack(1, 15, Red), stack(2, 15, Red)]).stats();
        assert_eq!(no_virus.block_near_3(), 1);
        assert_eq!(no_virus.virus_near_3(), 0);

        // the same run with a virus in it is one pill from a kill
        let with_virus = bottle(&[stack(0, 15, Red), stack(1, 15, Red), virus(2, 15, Red)]).stats();
        assert_eq!(with_virus.virus_near_3(), 1);
        assert_eq!(with_virus.block_near_3(), 0);
        assert_eq!(with_virus.virus_near_2(), 0);
    }

    #[test]
    fn a_run_with_nowhere_to_finish_is_not_a_near_match() {
        // three reds with a blue at one end and the bottle wall at the other: it can never clear
        let walled = bottle(&[
            stack(0, 15, Red),
            stack(1, 15, Red),
            virus(2, 15, Red),
            stack(3, 15, Blue),
        ])
        .stats();
        assert_eq!(walled.virus_near_3(), 0);

        // move the blue along and the same run has a cell to finish in
        let open = bottle(&[
            stack(0, 15, Red),
            stack(1, 15, Red),
            virus(2, 15, Red),
            stack(4, 15, Blue),
        ])
        .stats();
        assert_eq!(open.virus_near_3(), 1);
    }

    #[test]
    fn counts_runs_of_two_separately() {
        let stats = bottle(&[stack(0, 15, Red), virus(1, 15, Red)]).stats();
        assert_eq!(stats.virus_near_2(), 1);
        assert_eq!(stats.virus_near_3(), 0);
    }

    #[test]
    fn a_column_run_counts_too() {
        let stats = bottle(&[
            stack(0, 13, Red),
            stack(0, 14, Red),
            virus(0, 15, Red),
        ])
        .stats();
        assert_eq!(stats.virus_near_3(), 1);
    }

    #[test]
    fn heights_are_measured_from_the_floor() {
        let stats = bottle(&[stack(0, 15, Red), stack(3, 14, Blue), stack(3, 15, Blue)]).stats();
        assert_eq!(stats.max_height(), 2);
    }

    #[test]
    fn blocks_are_buried_by_other_colours_just_as_viruses_are() {
        // a red virus and a red block, each under two blues and a red
        let stats = bottle(&[
            stack(0, 12, Blue),
            stack(0, 13, Blue),
            stack(0, 14, Red),
            virus(0, 15, Red),
            stack(4, 12, Blue),
            stack(4, 13, Blue),
            stack(4, 14, Red),
            stack(4, 15, Red),
        ])
        .stats();
        assert_eq!(stats.buried_viruses(), 2);
        // three red blocks with two blues over each: (0,14), (4,14) and (4,15)
        assert_eq!(stats.buried_blocks(), 6);
    }

    #[test]
    fn an_empty_cell_under_the_stack_is_a_hole() {
        let stats = bottle(&[stack(0, 13, Red), stack(0, 15, Red)]).stats();
        assert_eq!(stats.holes(), 1);
    }

    #[test]
    fn a_lone_virus_needs_three_more_blocks() {
        assert_eq!(bottle(&[virus(0, 15, Red)]).stats().virus_work(), 3);
    }

    #[test]
    fn work_falls_as_the_line_fills_in() {
        let one = bottle(&[virus(0, 15, Red), stack(1, 15, Red)]).stats();
        assert_eq!(one.virus_work(), 2);
        let two = bottle(&[virus(0, 15, Red), stack(1, 15, Red), stack(2, 15, Red)]).stats();
        assert_eq!(two.virus_work(), 1);
    }

    #[test]
    fn a_walled_in_virus_costs_a_whole_line() {
        // reds either side of a blue virus in the only row it has, and the column above is
        // capped too, so nothing can complete a line through it
        let stats = bottle(&[
            stack(0, 15, Red),
            stack(1, 15, Red),
            virus(2, 15, Blue),
            stack(3, 15, Red),
            stack(4, 15, Red),
            stack(5, 15, Red),
            stack(6, 15, Red),
            stack(7, 15, Red),
            stack(2, 14, Red),
        ])
        .stats();
        assert_eq!(stats.virus_work(), UNREACHABLE_VIRUS);
    }

    #[test]
    fn work_counts_every_virus() {
        let stats = bottle(&[virus(0, 15, Red), virus(4, 15, Blue)]).stats();
        assert_eq!(stats.virus_work(), 6);
    }

    #[test]
    fn a_half_that_reaches_no_virus_of_its_colour_is_wasted() {
        // a blue virus sits in the bottle and a blue half is dropped nowhere near it
        let far = bottle(&[virus(0, 15, Blue), stack(7, 15, Blue)]);
        assert_eq!(wasted_halves(&far, &[BottlePoint::new(7, 15)]), 1);

        // the same half placed onto the blue virus's chain is doing work
        let joined = bottle(&[virus(0, 15, Blue), stack(1, 15, Blue)]);
        assert_eq!(wasted_halves(&joined, &[BottlePoint::new(1, 15)]), 0);
    }

    #[test]
    fn a_half_joining_a_chain_of_its_own_colour_with_no_virus_is_still_wasted() {
        // a tidy blue heap with no blue virus in it is not progress
        let bottle = bottle(&[
            virus(0, 15, Red),
            stack(5, 15, Blue),
            stack(6, 15, Blue),
            stack(7, 15, Blue),
        ]);
        assert_eq!(wasted_halves(&bottle, &[BottlePoint::new(7, 15)]), 1);
    }

    #[test]
    fn a_chain_reaching_the_virus_through_other_blocks_counts() {
        let bottle = bottle(&[
            virus(0, 15, Blue),
            stack(1, 15, Blue),
            stack(2, 15, Blue),
            stack(3, 15, Blue),
        ]);
        assert_eq!(wasted_halves(&bottle, &[BottlePoint::new(3, 15)]), 0);
    }

    #[test]
    fn deltas_subtract_the_bottle_before_the_placement() {
        let before = bottle(&[virus(0, 15, Red), virus(1, 15, Blue)]).stats();
        let after = bottle(&[virus(0, 15, Red)]).stats();
        let features = BottleFeatures::new(after, before, 0, 1);
        assert_eq!(features.delta().viruses(), -1);
        assert_eq!(features.global().viruses(), 1);
        assert_eq!(features.patterns_cleared(), 1);
    }
}
