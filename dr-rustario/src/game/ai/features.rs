//! What the network sees. Dr. Mario has no line clears, so the shape of the stack matters far
//! less than its colour structure: whether viruses are still reachable, and how close the
//! bottle is to a match.
//!
//! Which of those things are worth measuring was settled by [`crate::game::ai::probe`], which
//! asks the N64 ai what it thinks of every placement it is offered and then asks how much of
//! that opinion a feature set can reproduce. Two things it found shape everything here.
//!
//! The first is that the N64 scores *the run the pill landed in*, not the bottle: almost all of
//! its weight is on the two cells the placement filled and the lines through them. So the
//! features are in two halves - what the placement did ([`PlacementStats`]) and what the bottle
//! it left behind looks like ([`BottleStats`]) - and the first half is the one that carries the
//! ranking.
//!
//! The second is that a line is only worth building if a pill can still get to the cell that
//! finishes it. A run of three with a gap under an overhang is not one short of a clear, it is
//! junk, and every "room" measured here means room a half could actually be dropped into.

use crate::game::block::Block;
use crate::game::bottle::{Bottle, BOTTLE_HEIGHT, BOTTLE_WIDTH};
use crate::game::geometry::BottlePoint;
use crate::game::pill::VirusColor;
use std::ops::Sub;

/// a run this long or longer clears, so a run one short is a threat
const MATCH_LENGTH: usize = 4;

/// what a virus no line can reach any more counts as, in blocks of work
const UNREACHABLE_VIRUS: i32 = 2 * MATCH_LENGTH as i32;

/// The columns a pill spawns over. A bottle is lost when nothing can be dealt into these, which
/// is why how high they stand is not the same question as how high the bottle stands.
const ENTRANCE: [usize; 2] = [3, 4];

/// What a block in the top three rows counts against you, by row from the top and then column.
/// The N64's own `BadLineRate`, in the bottle's coordinates: it is steeply weighted towards the
/// middle, because that is where a pill has to come in. Nothing in the model reads it - the
/// probe measured a weighted count of what the stack has put up there as worth nothing once the
/// entrance height is being fed - and it is here for the probe's control group.
pub const TOP_ROW_RATE: [[i32; BOTTLE_WIDTH as usize]; 3] = [
    [6, 7, 8, 9, 9, 8, 7, 6],
    [2, 2, 4, 7, 7, 4, 2, 2],
    [1, 1, 2, 4, 4, 2, 1, 1],
];

/// What one settled bottle looks like. Two of these - the bottle before the pill and the
/// bottle after it - make the half of the features that is about the stack rather than about
/// the placement.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct BottleStats {
    viruses: i32,
    virus_work: i32,
    buried_viruses: i32,
    buried_blocks: i32,
    max_height: i32,
    entrance_height: i32,
    holes: i32,
    virus_3_row: i32,
    virus_3_col: i32,
    virus_2_row: i32,
    virus_2_col: i32,
    block_3_row: i32,
    block_3_col: i32,
}

impl BottleStats {
    pub fn viruses(&self) -> i32 {
        self.viruses
    }
    /// how many more matching blocks the bottle still needs to clear every virus
    pub fn virus_work(&self) -> i32 {
        self.virus_work
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
    /// How high the two columns a pill spawns over stand. This is not the tallest column and it
    /// is not the average one: it is the only height that can actually end a game, and the
    /// probe found it the strongest single thing the model was not being shown - ahead of nine
    /// of the inputs it was.
    pub fn entrance_height(&self) -> i32 {
        self.entrance_height
    }
    pub fn holes(&self) -> i32 {
        self.holes
    }
    /// Runs one short of a match that would take a virus with them, along a row and down a
    /// column. The two are counted apart because the N64 pays for them out of different tables
    /// and, in the situation that covers most of a game, pays for one of them and not the other.
    pub fn virus_3_row(&self) -> i32 {
        self.virus_3_row
    }
    pub fn virus_3_col(&self) -> i32 {
        self.virus_3_col
    }
    /// the same, two short
    pub fn virus_2_row(&self) -> i32 {
        self.virus_2_row
    }
    pub fn virus_2_col(&self) -> i32 {
        self.virus_2_col
    }
    /// runs one short with no virus in them: clearing those only tidies the bottle up
    pub fn block_3_row(&self) -> i32 {
        self.block_3_row
    }
    pub fn block_3_col(&self) -> i32 {
        self.block_3_col
    }
}

impl Sub<BottleStats> for BottleStats {
    type Output = BottleStats;

    fn sub(self, rhs: BottleStats) -> Self::Output {
        BottleStats {
            viruses: self.viruses - rhs.viruses,
            virus_work: self.virus_work - rhs.virus_work,
            buried_viruses: self.buried_viruses - rhs.buried_viruses,
            buried_blocks: self.buried_blocks - rhs.buried_blocks,
            max_height: self.max_height - rhs.max_height,
            entrance_height: self.entrance_height - rhs.entrance_height,
            holes: self.holes - rhs.holes,
            virus_3_row: self.virus_3_row - rhs.virus_3_row,
            virus_3_col: self.virus_3_col - rhs.virus_3_col,
            virus_2_row: self.virus_2_row - rhs.virus_2_row,
            virus_2_col: self.virus_2_col - rhs.virus_2_col,
            block_3_row: self.block_3_row - rhs.block_3_row,
            block_3_col: self.block_3_col - rhs.block_3_col,
        }
    }
}

/// What the placement itself did, which is where the N64 keeps almost all of its weight: the
/// run each half of the pill came to rest in, what it did to the virus underneath it, and what
/// the next pill could do with the bottle it leaves behind.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PlacementStats {
    patterns_cleared: i32,
    touching: i32,
    reach: i32,
    open_3: i32,
    open_2: i32,
    run_viruses: i32,
    stranded: i32,
    stranded_on_virus: i32,
    covers_virus: i32,
    buries_virus: i32,
    one_away: i32,
    one_away_virus: i32,
    chains: i32,
}

impl PlacementStats {
    /// runs cleared by the placement, cascades included
    pub fn patterns_cleared(&self) -> i32 {
        self.patterns_cleared
    }
    /// the longest run of one colour actually touching a half of the pill
    pub fn touching(&self) -> i32 {
        self.touching
    }
    /// how long that run could still become, counting the gaps a pill could drop a half into
    pub fn reach(&self) -> i32 {
        self.reach
    }
    /// halves left one short of a match with room to finish, which is what the N64 pays most for
    pub fn open_3(&self) -> i32 {
        self.open_3
    }
    /// the same, two short
    pub fn open_2(&self) -> i32 {
        self.open_2
    }
    /// viruses in the runs the halves landed in, which is what makes building one worth doing
    pub fn run_viruses(&self) -> i32 {
        self.run_viruses
    }
    /// Halves left where no line through them can ever reach four. Taking this out of the N64
    /// changes its mind on 44% of pills, three times more than anything else it measures: it is
    /// a veto rather than a preference, which is why it looks weak on its own.
    pub fn stranded(&self) -> i32 {
        self.stranded
    }
    /// stranded, and sitting on a virus, which is worse again
    pub fn stranded_on_virus(&self) -> i32 {
        self.stranded_on_virus
    }
    /// halves that came to rest on top of a virus with nothing else of the sort above them
    pub fn covers_virus(&self) -> i32 {
        self.covers_virus
    }
    /// the same, where the half is also stranded: a virus buried for nothing
    pub fn buries_virus(&self) -> i32 {
        self.buries_virus
    }
    /// how many ways the next pill could clear something in the bottle this leaves behind
    pub fn one_away(&self) -> i32 {
        self.one_away
    }
    /// how many of those would take a virus with them
    pub fn one_away_virus(&self) -> i32 {
        self.one_away_virus
    }
    /// whether one of them would cascade into a second clear
    pub fn chains(&self) -> i32 {
        self.chains
    }
}

/// the stats of the settled bottle, how they moved, what the placement itself did, and whether
/// it is a placement of the pill in play at all
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct BottleFeatures {
    global: BottleStats,
    delta: BottleStats,
    placement: PlacementStats,
    held: bool,
}

impl BottleFeatures {
    pub fn new(global: BottleStats, before: BottleStats, placement: PlacementStats) -> Self {
        Self {
            global,
            delta: global - before,
            placement,
            held: false,
        }
    }

    /// Mark this as a placement of the *held* pill rather than the one in play, which is the
    /// one thing about a candidate that nothing else here can see: the bottle it leaves behind
    /// and what it did to get there read exactly the same either way.
    pub fn of_the_held_pill(mut self) -> Self {
        self.held = true;
        self
    }

    /// whether this is a placement of the held pill, which reaching for costs the pill in play
    pub fn held(&self) -> bool {
        self.held
    }

    pub fn global(&self) -> BottleStats {
        self.global
    }
    pub fn delta(&self) -> BottleStats {
        self.delta
    }
    pub fn placement(&self) -> PlacementStats {
        self.placement
    }
}

pub trait BottleAnalysis {
    fn stats(&self) -> BottleStats;
}

impl BottleAnalysis for Bottle {
    fn stats(&self) -> BottleStats {
        Grid::of(self).stats()
    }
}

/// The bottle as the features read it: the colour of every settled block, which of them are
/// viruses, and how high each column stands. Built once and then asked everything, since every
/// measurement here walks the same grid.
pub struct Grid {
    colours: Vec<Option<VirusColor>>,
    viruses: Vec<bool>,
    heights: [i32; BOTTLE_WIDTH as usize],
}

impl Grid {
    pub fn of(bottle: &Bottle) -> Self {
        let cells = (BOTTLE_WIDTH * BOTTLE_HEIGHT) as usize;
        let mut colours = vec![None; cells];
        let mut viruses = vec![false; cells];
        let mut heights = [0; BOTTLE_WIDTH as usize];
        for x in 0..BOTTLE_WIDTH {
            for y in 0..BOTTLE_HEIGHT {
                // the pill in play is ignored exactly as the matcher ignores it
                let block = bottle.block_at(x, y);
                if let Some(colour) = block.destructible_color() {
                    colours[index(x, y)] = Some(colour);
                    viruses[index(x, y)] = block.is_virus();
                    if heights[x as usize] == 0 {
                        heights[x as usize] = (BOTTLE_HEIGHT - y) as i32;
                    }
                }
            }
        }
        Self {
            colours,
            viruses,
            heights,
        }
    }

    pub fn colour(&self, x: u32, y: u32) -> Option<VirusColor> {
        self.colours[index(x, y)]
    }

    pub fn is_virus(&self, x: u32, y: u32) -> bool {
        self.viruses[index(x, y)]
    }

    pub fn heights(&self) -> &[i32; BOTTLE_WIDTH as usize] {
        &self.heights
    }

    /// Whether a pill could still put a half in this cell: it is empty, and so is everything
    /// above it. A gap under an overhang is not room, however empty it is.
    ///
    /// **The placement search can now tuck and this rule was not widened to match**, which
    /// looks wrong and is measured. A cell under an overhang with a clear column beside it is
    /// reachable in the sense that the search can get a half into it, so the obvious change is
    /// to count a neighbouring column that is clear down to this row as a way in. Doing that
    /// costs the model a fifth of everything: the hand written baseline over five whole games
    /// went from 1431 viruses and 24 bottles to 1110 and 18, a clone fitted to the features
    /// from 2082 and 33 to 1703 and 27. Being able to reach a cell and being able to *rely* on
    /// reaching it are not the same thing - a tuck needs the column beside it filled to exactly
    /// the row below, which is not something the pill in play can arrange - and a model told
    /// that the cells under an overhang are still live stops minding whether it makes overhangs.
    /// What it costs to bury a run is worth more than what tucking one out is worth.
    pub fn reachable(&self, x: u32, y: u32) -> bool {
        self.colour(x, y).is_none() && (0..y).all(|above| self.colour(x, above).is_none())
    }

    pub fn stats(&self) -> BottleStats {
        let mut viruses = 0;
        let mut buried_viruses = 0;
        let mut buried_blocks = 0;
        let mut holes = 0;

        for x in 0..BOTTLE_WIDTH {
            let mut stacked = false;
            let mut above: Vec<VirusColor> = vec![];
            for y in 0..BOTTLE_HEIGHT {
                match self.colour(x, y) {
                    Some(colour) => {
                        stacked = true;
                        // everything of another colour above a cell has to clear before that
                        // cell can be reached from above
                        let buried = above.iter().filter(|c| **c != colour).count() as i32;
                        if self.is_virus(x, y) {
                            viruses += 1;
                            buried_viruses += buried;
                        } else {
                            buried_blocks += buried;
                        }
                        above.push(colour);
                    }
                    None => {
                        if stacked {
                            holes += 1;
                        }
                    }
                }
            }
        }

        let near = self.near_matches();
        BottleStats {
            viruses,
            virus_work: self.virus_work(),
            buried_viruses,
            buried_blocks,
            max_height: self.heights.iter().copied().max().unwrap_or(0),
            entrance_height: ENTRANCE.iter().map(|x| self.heights[*x]).max().unwrap_or(0),
            holes,
            virus_3_row: near[0],
            virus_3_col: near[1],
            virus_2_row: near[2],
            virus_2_col: near[3],
            block_3_row: near[4],
            block_3_col: near[5],
        }
    }

    /// Runs one and two short of a match that have somewhere to finish, along a row and then
    /// down a column. A run walled in at both ends is never going to clear, and neither is one
    /// whose only gap is buried under an overhang, so neither is counted: doing so would only
    /// teach the model to build junk.
    ///
    /// Returns `[virus 3 row, virus 3 column, virus 2 row, virus 2 column, block 3 row,
    /// block 3 column]`.
    fn near_matches(&self) -> [i32; 6] {
        let mut counts = [0; 6];

        for horizontal in [true, false] {
            let lines: Vec<Vec<(u32, u32)>> = if horizontal {
                (0..BOTTLE_HEIGHT)
                    .map(|y| (0..BOTTLE_WIDTH).map(|x| (x, y)).collect())
                    .collect()
            } else {
                (0..BOTTLE_WIDTH)
                    .map(|x| (0..BOTTLE_HEIGHT).map(|y| (x, y)).collect())
                    .collect()
            };

            for line in lines {
                let colour_at = |i: usize| self.colour(line[i].0, line[i].1);
                let mut i = 0;
                while i < line.len() {
                    let Some(colour) = colour_at(i) else {
                        i += 1;
                        continue;
                    };
                    let mut end = i;
                    while end + 1 < line.len() && colour_at(end + 1) == Some(colour) {
                        end += 1;
                    }
                    let length = end - i + 1;

                    if length == MATCH_LENGTH - 1 || length == MATCH_LENGTH - 2 {
                        let virus = (i..=end).any(|k| self.is_virus(line[k].0, line[k].1));
                        let mut room = 0;
                        for k in (0..i).rev() {
                            if !self.reachable(line[k].0, line[k].1) {
                                break;
                            }
                            room += 1;
                        }
                        for k in end + 1..line.len() {
                            if !self.reachable(line[k].0, line[k].1) {
                                break;
                            }
                            room += 1;
                        }

                        if room >= MATCH_LENGTH - length {
                            let at = match (virus, length == MATCH_LENGTH - 1, horizontal) {
                                (true, true, true) => Some(0),
                                (true, true, false) => Some(1),
                                (true, false, true) => Some(2),
                                (true, false, false) => Some(3),
                                (false, true, true) => Some(4),
                                (false, true, false) => Some(5),
                                // a run of two with no virus in it is not worth a feature
                                (false, false, _) => None,
                            };
                            if let Some(at) = at {
                                counts[at] += 1;
                            }
                        }
                    }
                    i = end + 1;
                }
            }
        }

        counts
    }

    /// How much work the bottle still needs: for every virus, the fewest matching blocks that
    /// would have to be added to complete a line of four through it, summed over all of them.
    /// This is the feature that points the agent at the viruses, and the strongest single input
    /// the probe measured. Counting same colour neighbours anywhere instead only teaches it to
    /// build tidy heaps in the corner.
    fn virus_work(&self) -> i32 {
        let mut work = 0;

        for y in 0..BOTTLE_HEIGHT as i32 {
            for x in 0..BOTTLE_WIDTH as i32 {
                if !self.is_virus(x as u32, y as u32) {
                    continue;
                }
                let colour = self.colour(x as u32, y as u32);

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
                            let cell =
                                self.colour((sx + dx * step) as u32, (sy + dy * step) as u32);
                            match cell {
                                // already the right colour, nothing to do for this cell
                                c if c == colour => (),
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

    /// The run of one colour through a cell, along a row or down a column: how much of it is
    /// touching, how long it could still become counting the gaps a pill could reach, and how
    /// many viruses are in it. This is the N64's own reading of a placement, and the thing that
    /// makes it build towards a clear rather than only taking the ones in front of it.
    fn run(&self, x: u32, y: u32, horizontal: bool) -> Run {
        let Some(colour) = self.colour(x, y) else {
            return Run::default();
        };
        let (dx, dy) = if horizontal { (1i32, 0i32) } else { (0, 1) };
        let mut run = Run {
            reach: 1,
            touching: 1,
            viruses: self.is_virus(x, y) as i32,
        };

        for direction in [-1i32, 1] {
            let mut touching = true;
            for step in 1..MATCH_LENGTH as i32 {
                let (nx, ny) = (
                    x as i32 + dx * step * direction,
                    y as i32 + dy * step * direction,
                );
                if nx < 0 || ny < 0 || nx >= BOTTLE_WIDTH as i32 || ny >= BOTTLE_HEIGHT as i32 {
                    break;
                }
                let (nx, ny) = (nx as u32, ny as u32);
                match self.colour(nx, ny) {
                    Some(other) if other == colour => {
                        run.reach += 1;
                        run.viruses += self.is_virus(nx, ny) as i32;
                        if touching {
                            run.touching += 1;
                        }
                    }
                    // a gap only counts towards what the line could become if a pill can
                    // actually get a half into it
                    None if self.reachable(nx, ny) => {
                        touching = false;
                        run.reach += 1;
                    }
                    _ => break,
                }
            }
        }
        run
    }

    /// Would dropping a half of `colour` into the empty cell at `(x, y)` clear a line? If so,
    /// whether that line takes a virus with it. This is [`Self::run`] counting only the cells
    /// actually touching, since a gap does not clear.
    fn would_clear(&self, x: u32, y: u32, colour: VirusColor) -> Option<bool> {
        for (dx, dy) in [(1i32, 0i32), (0, 1)] {
            let mut length = 1;
            let mut virus = false;
            for direction in [-1i32, 1] {
                for step in 1..MATCH_LENGTH as i32 {
                    let (nx, ny) = (
                        x as i32 + dx * step * direction,
                        y as i32 + dy * step * direction,
                    );
                    if nx < 0 || ny < 0 || nx >= BOTTLE_WIDTH as i32 || ny >= BOTTLE_HEIGHT as i32 {
                        break;
                    }
                    if self.colour(nx as u32, ny as u32) != Some(colour) {
                        break;
                    }
                    length += 1;
                    virus |= self.is_virus(nx as u32, ny as u32);
                }
            }
            if length >= MATCH_LENGTH {
                return Some(virus);
            }
        }
        None
    }

    /// the cells a half of the next pill could come to rest in: the top of each column and the
    /// one above it, which is where the upper half of an upright pill lands
    fn landing_cells(&self) -> Vec<(u32, u32)> {
        let mut cells = vec![];
        for x in 0..BOTTLE_WIDTH {
            let top = BOTTLE_HEIGHT as i32 - self.heights[x as usize];
            for step in 1..=2 {
                if top - step >= 0 {
                    cells.push((x, (top - step) as u32));
                }
            }
        }
        cells
    }
}

/// a run of one colour through a cell, ordered so that the longer reach wins
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
struct Run {
    reach: i32,
    touching: i32,
    viruses: i32,
}

/// What the placement did. `before` is the bottle the pill was dropped into, `after` the one
/// it left behind once everything cleared and cascaded - read once as a [`Grid`] - and `landed`
/// where the two halves came to rest.
pub fn placement_stats(
    before: &Bottle,
    after: &Bottle,
    grid: &Grid,
    landed: &[BottlePoint],
    patterns_cleared: i32,
) -> PlacementStats {
    let mut stats = PlacementStats {
        patterns_cleared,
        ..Default::default()
    };

    for point in landed {
        let (x, y) = (point.x(), point.y());
        if x < 0 || y < 0 || x >= BOTTLE_WIDTH as i32 || y >= BOTTLE_HEIGHT as i32 {
            continue;
        }
        let (x, y) = (x as u32, y as u32);

        // is this half sitting on a virus it has just covered up? Only a virus below it in the
        // column with none above counts: anywhere else it is in the middle of a heap.
        let over_virus = (0..y).all(|above| !before.block_at(x, above).is_virus())
            && (y + 1..BOTTLE_HEIGHT).any(|below| before.block_at(x, below).is_virus());

        // the clear took the half with it, which is the opposite of stranding it
        if grid.colour(x, y).is_none() {
            stats.covers_virus += over_virus as i32;
            continue;
        }

        let best = grid.run(x, y, false).max(grid.run(x, y, true));
        stats.touching = stats.touching.max(best.touching);
        stats.reach = stats.reach.max(best.reach);
        stats.run_viruses += best.viruses;
        if best.reach >= MATCH_LENGTH as i32 {
            match best.touching {
                3 => stats.open_3 += 1,
                2 => stats.open_2 += 1,
                _ => (),
            }
        } else {
            stats.stranded += 1;
            if y + 1 < BOTTLE_HEIGHT && before.block_at(x, y + 1).is_virus() {
                stats.stranded_on_virus += 1;
            }
        }
        if over_virus {
            stats.covers_virus += 1;
            if best.reach < MATCH_LENGTH as i32 {
                stats.buries_virus += 1;
            }
        }
    }

    let (one_away, one_away_virus, chains) = lookahead(after, grid);
    stats.one_away = one_away;
    stats.one_away_virus = one_away_virus;
    stats.chains = chains;
    stats
}

/// One ply of lookahead: how many ways the next pill could clear something in the bottle this
/// placement leaves behind, how many of those would take a virus, and whether any of them
/// cascades into a second clear. The N64's chain term carries its largest single weight, and
/// the features had no lookahead of any kind before this.
///
/// Whether a half of a given colour completes a line is answered off the grid, since a line
/// forms exactly when the run through the cell it lands in reaches four. Only the cascade needs
/// the bottle itself, and only for the placements that have a clear to cascade from.
fn lookahead(settled: &Bottle, grid: &Grid) -> (i32, i32, i32) {
    let mut one_away = 0;
    let mut one_away_virus = 0;
    let mut chains = 0;

    for (x, y) in grid.landing_cells() {
        for colour in [VirusColor::Red, VirusColor::Blue, VirusColor::Yellow] {
            let Some(takes_a_virus) = grid.would_clear(x, y, colour) else {
                continue;
            };
            one_away += 1;
            one_away_virus += takes_a_virus as i32;

            if chains == 0 {
                let mut next = settled.clone();
                next.place(x, y, Block::Garbage(colour));
                let (blocks, _) = next.pattern();
                if !blocks.is_empty() {
                    next.destroy(blocks);
                    while next.step_down_garbage() {}
                    chains = !next.pattern().0.is_empty() as i32;
                }
            }
        }
    }
    (one_away, one_away_virus, chains)
}

fn index(x: u32, y: u32) -> usize {
    (y * BOTTLE_WIDTH + x) as usize
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

    /// What `landed` did, dropping the pill into `before` to make `after`. The blocks are
    /// [`Block::Garbage`] rather than [`Block::Stack`] because the lookahead settles the bottle
    /// it tries a half in, and half of a locked pill with no partner in the bottle is not a
    /// state the real game can be in.
    fn placed(before: &Bottle, after: &Bottle, landed: &[(u32, u32)]) -> PlacementStats {
        let points: Vec<BottlePoint> = landed
            .iter()
            .map(|(x, y)| BottlePoint::new(*x as i32, *y as i32))
            .collect();
        placement_stats(before, after, &Grid::of(after), &points, 0)
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
    fn blocks_are_buried_by_other_colours_just_as_viruses_are() {
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
    fn near_matches_are_counted_apart_by_axis_and_by_whether_a_virus_is_in_them() {
        // three reds along a row with no virus: clearing it would only tidy the bottle up
        let no_virus = bottle(&[stack(0, 15, Red), stack(1, 15, Red), stack(2, 15, Red)]).stats();
        assert_eq!(no_virus.block_3_row(), 1);
        assert_eq!(no_virus.virus_3_row(), 0);

        // the same run with a virus in it is one pill from a kill
        let with_virus = bottle(&[stack(0, 15, Red), stack(1, 15, Red), virus(2, 15, Red)]).stats();
        assert_eq!(with_virus.virus_3_row(), 1);
        assert_eq!(with_virus.block_3_row(), 0);
        assert_eq!(with_virus.virus_2_row(), 0);

        // and the same run down a column is counted in its own place
        let column = bottle(&[stack(0, 13, Red), stack(0, 14, Red), virus(0, 15, Red)]).stats();
        assert_eq!(column.virus_3_col(), 1);
        assert_eq!(column.virus_3_row(), 0);
    }

    #[test]
    fn counts_runs_of_two_separately() {
        let stats = bottle(&[stack(0, 15, Red), virus(1, 15, Red)]).stats();
        assert_eq!(stats.virus_2_row(), 1);
        assert_eq!(stats.virus_3_row(), 0);
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
        assert_eq!(walled.virus_3_row(), 0);

        // move the blue along and the same run has a cell to finish in
        let open = bottle(&[
            stack(0, 15, Red),
            stack(1, 15, Red),
            virus(2, 15, Red),
            stack(4, 15, Blue),
        ])
        .stats();
        assert_eq!(open.virus_3_row(), 1);
    }

    #[test]
    fn a_gap_under_an_overhang_is_not_room() {
        // three reds one short of a match, with the only cell that would finish them buried
        // under a blue: no pill can ever get a half in there, so this is junk and not a threat
        let covered = bottle(&[
            stack(0, 15, Red),
            stack(1, 15, Red),
            virus(2, 15, Red),
            stack(3, 14, Blue),
        ])
        .stats();
        assert_eq!(covered.virus_3_row(), 0);

        // lift the lid off and it is one pill away again
        let open = bottle(&[
            stack(0, 15, Red),
            stack(1, 15, Red),
            virus(2, 15, Red),
            stack(4, 14, Blue),
        ])
        .stats();
        assert_eq!(open.virus_3_row(), 1);
    }

    #[test]
    fn heights_are_measured_from_the_floor() {
        let stats = bottle(&[stack(0, 15, Red), stack(3, 14, Blue), stack(3, 15, Blue)]).stats();
        assert_eq!(stats.max_height(), 2);
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
    fn a_half_in_a_run_that_can_still_reach_four_is_not_stranded() {
        // a blue half landing on the end of two blues with room to the right
        let before = bottle(&[virus(0, 15, Blue), garbage(1, 15, Blue)]);
        let after = bottle(&[
            virus(0, 15, Blue),
            garbage(1, 15, Blue),
            garbage(2, 15, Blue),
        ]);
        let stats = placed(&before, &after, &[(2, 15)]);
        assert_eq!(stats.stranded(), 0);
        assert_eq!(stats.open_3(), 1);
        assert_eq!(stats.run_viruses(), 1);
    }

    #[test]
    fn a_half_no_line_can_ever_join_is_stranded() {
        // a lone blue walled in by reds along its row and capped by reds up its column, so no
        // line of blues can ever reach it
        let mut blocks = vec![];
        for x in 0..BOTTLE_WIDTH {
            blocks.push(garbage(x, 15, Red));
            if x != 3 {
                blocks.push(garbage(x, 14, Red));
            }
        }
        for y in 11..14 {
            blocks.push(garbage(3, y, Red));
        }
        let before = bottle(&blocks);
        blocks.push(garbage(3, 14, Blue));
        let after = bottle(&blocks);
        assert_eq!(placed(&before, &after, &[(3, 14)]).stranded(), 1);
    }

    #[test]
    fn a_half_with_room_above_it_is_not_stranded_however_boxed_in_its_row_is() {
        // the same blue, with the column above it left open: a line of blues can still come
        // down onto it, so it is not dead weight
        let mut blocks = vec![];
        for x in 0..BOTTLE_WIDTH {
            blocks.push(garbage(x, 15, Red));
            if x != 3 {
                blocks.push(garbage(x, 14, Red));
            }
        }
        let before = bottle(&blocks);
        blocks.push(garbage(3, 14, Blue));
        let after = bottle(&blocks);
        assert_eq!(placed(&before, &after, &[(3, 14)]).stranded(), 0);
    }

    #[test]
    fn covering_a_virus_for_nothing_is_burying_it() {
        // a blue half dropped straight onto a red virus, in a row and a column that can never
        // take a line of blues
        let mut blocks = vec![virus(3, 15, Red)];
        for x in 0..BOTTLE_WIDTH {
            if x != 3 {
                blocks.push(garbage(x, 14, Red));
                blocks.push(garbage(x, 15, Red));
            }
        }
        for y in 11..14 {
            blocks.push(garbage(3, y, Red));
        }
        let before = bottle(&blocks);
        blocks.push(garbage(3, 14, Blue));
        let after = bottle(&blocks);
        let stats = placed(&before, &after, &[(3, 14)]);
        assert_eq!(stats.covers_virus(), 1);
        assert_eq!(stats.buries_virus(), 1);
    }

    #[test]
    fn a_bottle_one_half_from_a_clear_is_one_away() {
        // three reds on the floor with the fourth cell open: any red half finishes it
        let after = bottle(&[virus(0, 15, Red), garbage(1, 15, Red), garbage(2, 15, Red)]);
        let stats = placed(&bottle(&[]), &after, &[]);
        assert_eq!(stats.one_away(), 1);
        assert_eq!(stats.one_away_virus(), 1);
        assert_eq!(stats.chains(), 0);
    }

    #[test]
    fn a_clear_that_drops_a_line_onto_another_chains() {
        // a red half in the gap clears the whole red row along the floor, and the blue left
        // hanging over it drops onto the end of the blues to make a second line
        let after = bottle(&[
            garbage(0, 15, Red),
            garbage(1, 15, Red),
            garbage(2, 15, Red),
            garbage(4, 15, Red),
            garbage(4, 14, Blue),
            garbage(5, 15, Blue),
            garbage(6, 15, Blue),
            garbage(7, 15, Blue),
        ]);
        assert_eq!(placed(&bottle(&[]), &after, &[]).chains(), 1);
    }
}
