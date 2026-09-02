//! What the network sees, derived from the rules of Dr. Mario rather than from what another
//! program pays attention to.
//!
//! Two measurements do nearly all of it, and both come out of the same scan.
//!
//! **Work** is how many blocks a settled cell still needs for a line of four through it. Take
//! the four windows of four that contain the cell, on each axis; a window is *live* when no
//! cell in it is another colour and every empty cell in it is one a pill could still be dropped
//! into; the work is the fewest empty cells any live window has. It is 1, 2 or 3 - a window
//! holds three cells besides the subject, and 0 would already have cleared.
//!
//! **Buried** is the same scan finding no live window at all, on either axis. So buried is
//! simply work with no answer, which is why [`BURIED`] is a work value rather than a separate
//! question, and why no cell can ever report a cost it cannot pay: a run of three whose only
//! gap is under an overhang is junk, not a threat.
//!
//! Everything else is counting. The bottle is summarised six ways ([`BottleStats`]) and fed
//! twice over - as it stood before the pill, which says whether the agent is digging a full
//! bottle out or finishing one, and as the change the placement made to it, which is the only
//! part that can rank one candidate against another.

use crate::game::block::Block;
use crate::game::bottle::{Bottle, BOTTLE_HEIGHT, BOTTLE_WIDTH};
use crate::game::geometry::BottlePoint;
use crate::game::pill::VirusColor;
use std::ops::Sub;
use std::sync::OnceLock;

/// a run this long or longer clears, so a run one short is a threat
const MATCH_LENGTH: usize = 4;

/// What a cell's work comes to when no line through it can ever complete. It is the largest
/// value a work can take, so `min` over the two axes picks a live answer over this one without
/// having to ask which is which.
pub const BURIED: u8 = u8::MAX;

/// What a block in the top three rows counts against you, by row from the top and then column.
/// The N64's own `BadLineRate`, in the bottle's coordinates: it is steeply weighted towards the
/// middle, because that is where a pill has to come in. Nothing in the model reads it, and it
/// is here for [`crate::game::ai::probe`]'s control group.
pub const TOP_ROW_RATE: [[i32; BOTTLE_WIDTH as usize]; 3] = [
    [6, 7, 8, 9, 9, 8, 7, 6],
    [2, 2, 4, 7, 7, 4, 2, 2],
    [1, 1, 2, 4, 4, 2, 1, 1],
];

/// The columns a pill spawns over. A bottle is lost when nothing can be dealt into these, which
/// is why how high they stand is not the same question as how high the bottle stands. Nothing
/// in the model reads this either - it is measured as a control, in the probe's `EXTRA` group.
pub const ENTRANCE: [usize; 2] = [3, 4];

// ---------------------------------------------------------------------------------------
// the scan
// ---------------------------------------------------------------------------------------

/// How one cell of a window reads to the subject in the middle of it. Two bits, because six of
/// these are packed into the key of [`work_table`].
const SAME: u8 = 0;
/// a block of another colour: it has to clear before any line can pass through here
const FOREIGN: u8 = 1;
/// empty, and a pill could still drop a half into it: this is one block of work
const OPEN: u8 = 2;
/// empty but under an overhang, or off the board altogether: no line is coming through here
const SHUT: u8 = 3;

/// how many cells either side of the subject a window can reach
const REACH: usize = MATCH_LENGTH - 1;

/// The whole of the scan, worked out once for every arrangement a subject's neighbours can be
/// in. Six neighbours - three either side - in one of four states each is `4^6` keys, so the
/// four-window loop becomes one index. That is what buys the room to measure *every* occupied
/// cell rather than only the viruses.
fn work_table() -> &'static [u8; 4096] {
    static TABLE: OnceLock<[u8; 4096]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [BURIED; 4096];
        for (key, entry) in table.iter_mut().enumerate() {
            // the subject sits at REACH with its neighbours either side of it
            let mut line = [SAME; 2 * REACH + 1];
            for slot in 0..2 * REACH {
                let cell = if slot < REACH { slot } else { slot + 1 };
                line[cell] = ((key >> (slot * 2)) & 0b11) as u8;
            }

            let mut fewest = BURIED;
            for start in 0..MATCH_LENGTH {
                let window = &line[start..start + MATCH_LENGTH];
                // a foreign block cannot be moved and an unreachable gap cannot be filled, so
                // either one takes this window out of the running altogether
                if window.iter().any(|cell| *cell == FOREIGN || *cell == SHUT) {
                    continue;
                }
                fewest = fewest.min(window.iter().filter(|cell| **cell == OPEN).count() as u8);
            }
            *entry = fewest;
        }
        table
    })
}

/// What one settled bottle looks like. Two of these - the bottle before the pill and the bottle
/// after it - are the whole of what the network is shown about the stack.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct BottleStats {
    viruses: i32,
    virus_work: i32,
    virus_work_row: i32,
    virus_work_col: i32,
    viruses_at_work_1: i32,
    viruses_at_work_1_row: i32,
    viruses_at_work_1_col: i32,
    viruses_buried: i32,
    block_work: i32,
    blocks_at_work_1: i32,
    blocks_at_work_1_row: i32,
    blocks_at_work_1_col: i32,
    blocks_buried: i32,
    max_height: i32,
    landing_height: i32,
    flat_landing_height: i32,
    entrance_height: i32,
    holes: i32,
}

impl BottleStats {
    pub fn viruses(&self) -> i32 {
        self.viruses
    }
    /// The work every virus still needs, added up. Buried viruses are not in it - they are
    /// counted in [`Self::viruses_buried`] instead, since a cost nothing can pay is not a cost.
    pub fn virus_work(&self) -> i32 {
        self.virus_work
    }
    /// The same work, kept apart by the axis it is on. `min` over the two hides which way a
    /// virus has to be finished, and the two are not the same job: finishing a column means
    /// building upward, which is the only direction that can end a game.
    pub fn virus_work_row(&self) -> i32 {
        self.virus_work_row
    }
    pub fn virus_work_col(&self) -> i32 {
        self.virus_work_col
    }
    /// How many viruses are one block from dying. A pill delivers two cells a turn, so this is
    /// the currency: a sum of work cannot tell four viruses at {3,3,1,1} from four at {2,2,2,2},
    /// and the first is two kills away where the second is none.
    pub fn viruses_at_work_1(&self) -> i32 {
        self.viruses_at_work_1
    }
    /// the same, kept apart by which axis the one block is wanted on
    pub fn viruses_at_work_1_row(&self) -> i32 {
        self.viruses_at_work_1_row
    }
    pub fn viruses_at_work_1_col(&self) -> i32 {
        self.viruses_at_work_1_col
    }
    pub fn blocks_at_work_1_row(&self) -> i32 {
        self.blocks_at_work_1_row
    }
    pub fn blocks_at_work_1_col(&self) -> i32 {
        self.blocks_at_work_1_col
    }
    /// viruses no line through them can ever complete, on either axis
    pub fn viruses_buried(&self) -> i32 {
        self.viruses_buried
    }
    /// the same work count, over everything that is not a virus
    pub fn block_work(&self) -> i32 {
        self.block_work
    }
    /// the same, over everything that is not a virus: a line one block from tidying itself up
    pub fn blocks_at_work_1(&self) -> i32 {
        self.blocks_at_work_1
    }
    /// blocks no line through them can ever complete: dead weight, and the thing a placement is
    /// most easily talked into making
    pub fn blocks_buried(&self) -> i32 {
        self.blocks_buried
    }
    pub fn max_height(&self) -> i32 {
        self.max_height
    }
    /// How high the next pill has to come to rest at best: the shortest column in the bottle.
    ///
    /// This is the height that actually constrains play, where [`Self::max_height`] is the one
    /// that looks like it does. A lone virus on the floor makes the tallest column 1 and
    /// changes nothing at all - every other column is still open to the floor, so a pill can
    /// still be put as low as it ever could, and this stays 0. A spike in one corner is not a
    /// full bottle.
    pub fn landing_height(&self) -> i32 {
        self.landing_height
    }
    /// The same for a pill laid **flat**, which needs two neighbouring columns rather than one:
    /// the lowest a horizontal half can land anywhere. A bottle of alternating towers has a low
    /// [`Self::landing_height`] and a high one of these.
    pub fn flat_landing_height(&self) -> i32 {
        self.flat_landing_height
    }
    /// How high the two columns a pill spawns over stand. Not the tallest column and not the
    /// average one: the only height that can actually end a game.
    pub fn entrance_height(&self) -> i32 {
        self.entrance_height
    }
    /// empty cells with something above them in the same column
    pub fn holes(&self) -> i32 {
        self.holes
    }
}

impl Sub<BottleStats> for BottleStats {
    type Output = BottleStats;

    fn sub(self, rhs: BottleStats) -> Self::Output {
        BottleStats {
            viruses: self.viruses - rhs.viruses,
            virus_work: self.virus_work - rhs.virus_work,
            virus_work_row: self.virus_work_row - rhs.virus_work_row,
            virus_work_col: self.virus_work_col - rhs.virus_work_col,
            viruses_at_work_1: self.viruses_at_work_1 - rhs.viruses_at_work_1,
            viruses_at_work_1_row: self.viruses_at_work_1_row - rhs.viruses_at_work_1_row,
            viruses_at_work_1_col: self.viruses_at_work_1_col - rhs.viruses_at_work_1_col,
            viruses_buried: self.viruses_buried - rhs.viruses_buried,
            block_work: self.block_work - rhs.block_work,
            blocks_at_work_1: self.blocks_at_work_1 - rhs.blocks_at_work_1,
            blocks_at_work_1_row: self.blocks_at_work_1_row - rhs.blocks_at_work_1_row,
            blocks_at_work_1_col: self.blocks_at_work_1_col - rhs.blocks_at_work_1_col,
            blocks_buried: self.blocks_buried - rhs.blocks_buried,
            max_height: self.max_height - rhs.max_height,
            landing_height: self.landing_height - rhs.landing_height,
            flat_landing_height: self.flat_landing_height - rhs.flat_landing_height,
            entrance_height: self.entrance_height - rhs.entrance_height,
            holes: self.holes - rhs.holes,
        }
    }
}

/// What the placement itself did: what it cleared, and the same work scan read on the two
/// cells it put down rather than summed over the whole bottle.
///
/// The bottle-wide sums cannot say this. [`BottleStats::block_work`] adds up forty-odd blocks,
/// so the two the decision is actually about are a twentieth of the number and are conflated
/// with every other block the placement happened to touch: two halves landing one block from a
/// clear and two halves landing as dead weight beside a tidied corner move it by the same
/// amount. Taking the equivalent term out of the N64 ai changes its mind on 47% of pills, more
/// than three times anything else it measures.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PlacementStats {
    patterns_cleared: i32,
    halves_work: i32,
    halves_buried: i32,
    halves_run_viruses: i32,
    halves_over_virus: i32,
    halves_touching: i32,
    halves_one_short: i32,
    halves_two_short: i32,
}

impl PlacementStats {
    /// runs cleared by the placement, cascades included
    pub fn patterns_cleared(&self) -> i32 {
        self.patterns_cleared
    }
    /// How much work the better of the two cells the pill put down still needs: 1, 2 or 3, or
    /// [`HALF_BURIED`] where no line through either of them can ever complete. Zero when the
    /// placement cleared, since then the halves have gone or moved and
    /// [`Self::patterns_cleared`] is what says so.
    pub fn halves_work(&self) -> i32 {
        self.halves_work
    }
    /// how many of the two, 0 to 2, no line can ever reach
    pub fn halves_buried(&self) -> i32 {
        self.halves_buried
    }
    /// viruses in the line the placed halves are working on, which is what makes building one
    /// worth doing at all rather than tidying a corner
    pub fn halves_run_viruses(&self) -> i32 {
        self.halves_run_viruses
    }
    /// halves resting in a column with a virus below them and none above. A virus never falls,
    /// so everything on top of one is frozen until it goes.
    pub fn halves_over_virus(&self) -> i32 {
        self.halves_over_virus
    }
    /// how many of the two placed cells are exactly one block from a clear, and exactly two.
    /// The same measurement as [`Self::halves_work`], told as an indicator rather than a
    /// number: "exactly one short" is the thing a scorer wants and a sigmoid has to carve it
    /// out of a continuous input.
    pub fn halves_one_short(&self) -> i32 {
        self.halves_one_short
    }
    pub fn halves_two_short(&self) -> i32 {
        self.halves_two_short
    }
    /// the longest contiguous run of one colour through either placed half
    pub fn halves_touching(&self) -> i32 {
        self.halves_touching
    }
}

/// What a placed half's work reads as when no line through it can ever complete. One worse than
/// the worst live answer, so the input stays an ordering rather than needing a second question.
pub const HALF_BURIED: i32 = MATCH_LENGTH as i32;

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

    /// How many blocks the cell at `(x, y)` still needs for a line of four through it along one
    /// axis, or [`BURIED`] when no window on that axis can ever complete.
    fn work_on(&self, x: u32, y: u32, colour: VirusColor, horizontal: bool) -> u8 {
        let (dx, dy) = if horizontal { (1i32, 0i32) } else { (0, 1) };
        let mut key = 0usize;
        for (slot, step) in (-(REACH as i32)..=REACH as i32)
            .filter(|step| *step != 0)
            .enumerate()
        {
            let (nx, ny) = (x as i32 + dx * step, y as i32 + dy * step);
            let cell =
                if nx < 0 || ny < 0 || nx >= BOTTLE_WIDTH as i32 || ny >= BOTTLE_HEIGHT as i32 {
                    SHUT
                } else {
                    let (nx, ny) = (nx as u32, ny as u32);
                    match self.colour(nx, ny) {
                        // a virus and a settled half of the same colour are the same cell here:
                        // both clear, and a line does not care which it took
                        Some(other) if other == colour => SAME,
                        Some(_) => FOREIGN,
                        None if self.reachable(nx, ny) => OPEN,
                        None => SHUT,
                    }
                };
            key |= (cell as usize) << (slot * 2);
        }
        work_table()[key]
    }

    /// The work of a settled cell: the cheaper of its two axes, or [`BURIED`] when neither has
    /// a live window. Empty cells have no work, which is also [`BURIED`].
    pub fn work(&self, x: u32, y: u32) -> u8 {
        match self.colour(x, y) {
            Some(colour) => self
                .work_on(x, y, colour, true)
                .min(self.work_on(x, y, colour, false)),
            None => BURIED,
        }
    }

    /// The cheapest live window through a cell, and how many viruses are already in it. The
    /// lookup table answers the work of every cell in one index but knows nothing about what is
    /// *in* the window it chose, so this walks them - which is affordable because only the two
    /// cells a pill just put down ever ask.
    pub fn best_window(&self, x: u32, y: u32) -> Option<(u8, i32)> {
        let colour = self.colour(x, y)?;
        let mut best: Option<(u8, i32)> = None;
        for (dx, dy) in [(1i32, 0i32), (0, 1)] {
            for offset in 0..MATCH_LENGTH as i32 {
                let (sx, sy) = (x as i32 - dx * offset, y as i32 - dy * offset);
                let (ex, ey) = (
                    sx + dx * (MATCH_LENGTH as i32 - 1),
                    sy + dy * (MATCH_LENGTH as i32 - 1),
                );
                if sx < 0 || sy < 0 || ex >= BOTTLE_WIDTH as i32 || ey >= BOTTLE_HEIGHT as i32 {
                    continue;
                }
                let (mut empties, mut viruses, mut live) = (0, 0, true);
                for step in 0..MATCH_LENGTH as i32 {
                    let (cx, cy) = ((sx + dx * step) as u32, (sy + dy * step) as u32);
                    match self.colour(cx, cy) {
                        Some(other) if other == colour => viruses += self.is_virus(cx, cy) as i32,
                        Some(_) => live = false,
                        None if self.reachable(cx, cy) => empties += 1,
                        None => live = false,
                    }
                    if !live {
                        break;
                    }
                }
                if live && best.is_none_or(|(fewest, _)| empties < fewest) {
                    best = Some((empties, viruses));
                }
            }
        }
        best
    }

    /// The longest **contiguous** run of one colour through a cell, on its better axis.
    ///
    /// This is not [`Self::work`] upside down and the difference is the whole reason it is fed
    /// beside it. Work counts a reachable gap as one block of work, so a run of two with a hole
    /// in the middle and a run of three touching are both "one short"; touching counts only the
    /// cells that are actually joined, which is what would clear this instant if it reached
    /// four. In the thirty two input set that preceded this one they were the first and third
    /// most decisive inputs the model had, and neither could stand in for the other.
    pub fn touching(&self, x: u32, y: u32) -> i32 {
        let Some(colour) = self.colour(x, y) else {
            return 0;
        };
        let mut best = 1;
        for (dx, dy) in [(1i32, 0i32), (0, 1)] {
            let mut run = 1;
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
                    run += 1;
                }
            }
            best = best.max(run);
        }
        best
    }

    /// whether the cell at `(x, y)` rests over a virus with none of its own above it
    pub fn over_virus(&self, x: u32, y: u32) -> bool {
        (0..y).all(|above| !self.is_virus(x, above))
            && (y + 1..BOTTLE_HEIGHT).any(|below| self.is_virus(x, below))
    }

    pub fn stats(&self) -> BottleStats {
        let mut stats = BottleStats {
            max_height: self.heights.iter().copied().max().unwrap_or(0),
            landing_height: self.heights.iter().copied().min().unwrap_or(0),
            flat_landing_height: self
                .heights
                .windows(2)
                .map(|pair| pair[0].max(pair[1]))
                .min()
                .unwrap_or(0),
            entrance_height: ENTRANCE.iter().map(|x| self.heights[*x]).max().unwrap_or(0),
            ..Default::default()
        };

        for x in 0..BOTTLE_WIDTH {
            let mut stacked = false;
            for y in 0..BOTTLE_HEIGHT {
                if self.colour(x, y).is_none() {
                    stats.holes += stacked as i32;
                    continue;
                }
                stacked = true;
                let virus = self.is_virus(x, y);
                if let Some(colour) = self.colour(x, y) {
                    let row = self.work_on(x, y, colour, true);
                    let col = self.work_on(x, y, colour, false);
                    if virus {
                        if let work @ 1..=3 = row {
                            stats.virus_work_row += work as i32;
                        }
                        if let work @ 1..=3 = col {
                            stats.virus_work_col += work as i32;
                        }
                        stats.viruses_at_work_1_row += (row == 1) as i32;
                        stats.viruses_at_work_1_col += (col == 1) as i32;
                    } else {
                        stats.blocks_at_work_1_row += (row == 1) as i32;
                        stats.blocks_at_work_1_col += (col == 1) as i32;
                    }
                }
                stats.viruses += virus as i32;
                match (virus, self.work(x, y)) {
                    (true, BURIED) => stats.viruses_buried += 1,
                    (true, work) => {
                        stats.virus_work += work as i32;
                        stats.viruses_at_work_1 += (work == 1) as i32;
                    }
                    (false, BURIED) => stats.blocks_buried += 1,
                    (false, work) => {
                        stats.block_work += work as i32;
                        stats.blocks_at_work_1 += (work == 1) as i32;
                    }
                }
            }
        }

        stats
    }
}

/// What the placement did on its way to the bottle it left behind.
///
/// The halves are only read when **nothing cleared**. A clear takes cells out and drops
/// whatever was resting on them, so the points the pill locked at are no longer where its
/// halves are - and reading the work at a stale point measures some other cell's line. Where
/// something did clear, [`PlacementStats::patterns_cleared`] is what carries the placement and
/// the halves report nothing.
pub fn placement_stats(
    grid: &Grid,
    landed: &[BottlePoint],
    patterns_cleared: i32,
) -> PlacementStats {
    let mut stats = PlacementStats {
        patterns_cleared,
        ..Default::default()
    };
    if patterns_cleared > 0 {
        return stats;
    }

    let mut best = HALF_BURIED;
    for point in landed {
        let (x, y) = (point.x(), point.y());
        if x < 0 || y < 0 || x >= BOTTLE_WIDTH as i32 || y >= BOTTLE_HEIGHT as i32 {
            continue;
        }
        let (x, y) = (x as u32, y as u32);
        stats.halves_over_virus += grid.over_virus(x, y) as i32;
        stats.halves_touching = stats.halves_touching.max(grid.touching(x, y));
        match grid.best_window(x, y) {
            None => stats.halves_buried += 1,
            Some((work, viruses)) => {
                best = best.min(work as i32);
                stats.halves_run_viruses += viruses;
                stats.halves_one_short += (work == 1) as i32;
                stats.halves_two_short += (work == 2) as i32;
            }
        }
    }
    stats.halves_work = best;
    stats
}

fn index(x: u32, y: u32) -> usize {
    (y * BOTTLE_WIDTH + x) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::block::Block;
    use crate::game::bottle::BOTTLE_FLOOR;
    use crate::game::geometry::Rotation;
    use crate::game::pill::VitaminOrdinal;
    use VirusColor::{Blue, Red};

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

    fn stack(x: u32, y: u32, color: VirusColor) -> (u32, u32, Block) {
        (
            x,
            y,
            Block::Stack(color, Rotation::North, VitaminOrdinal::Left),
        )
    }

    /// One axis around a subject, written the way the specification is: `S` is the subject,
    /// `_` an empty cell, `1` a block of the same colour and `0` a block of another. It is laid
    /// along the floor with nothing above it, so every empty cell is one a pill can reach.
    fn row_work(pattern: &str) -> u8 {
        let mut blocks = vec![];
        let mut subject = None;
        for (x, character) in pattern.chars().enumerate() {
            let x = x as u32;
            match character {
                '_' => continue,
                'S' => {
                    subject = Some(x);
                    blocks.push(virus(x, BOTTLE_FLOOR, Red));
                }
                '1' => blocks.push(stack(x, BOTTLE_FLOOR, Red)),
                '0' => blocks.push(stack(x, BOTTLE_FLOOR, Blue)),
                other => panic!("unknown cell {} in {}", other, pattern),
            }
        }
        let x = subject.expect("no subject in the pattern");
        Grid::of(&bottle(&blocks)).work_on(x, BOTTLE_FLOOR, Red, true)
    }

    #[test]
    fn the_scan_reproduces_the_specification() {
        for (pattern, work) in [
            ("___S___", 3),
            ("__0S1__", 2),
            ("_11S0__", 1),
            ("__0S11_", 1),
            ("___S11_", 1),
            ("_11S___", 1),
            ("__1S1__", 1),
            ("_1_S1__", 1),
            ("__1S_1_", 1),
            ("11_S___", 1),
            ("1__S___", 2),
            ("___S_11", 1),
            ("___S__1", 2),
        ] {
            assert_eq!(row_work(pattern), work, "{}", pattern);
        }
    }

    #[test]
    fn a_subject_with_a_clear_run_of_four_across_it_is_not_buried() {
        for (pattern, work) in [
            ("___S___", 3),
            ("__0S___", 3),
            ("___S0__", 3),
            ("_0_S__0", 3),
        ] {
            assert_eq!(row_work(pattern), work, "{}", pattern);
        }
    }

    #[test]
    fn a_subject_no_line_can_ever_cross_is_buried() {
        for pattern in ["_0_S_0_", "__0S_0_", "__0S0__"] {
            assert_eq!(row_work(pattern), BURIED, "{}", pattern);
        }
    }

    #[test]
    fn a_gap_under_an_overhang_is_not_work() {
        // a red virus in the corner whose row is otherwise empty, but with the two cells that
        // would finish the line roofed over: they are empty and no pill can ever get to them
        let covered = bottle(&[virus(7, 15, Red), stack(5, 14, Blue), stack(6, 14, Blue)]);
        assert_eq!(
            Grid::of(&covered).work_on(7, BOTTLE_FLOOR, Red, true),
            BURIED
        );

        // lift the roof off and the same three cells are three blocks of work
        let open = bottle(&[virus(7, 15, Red)]);
        assert_eq!(Grid::of(&open).work_on(7, BOTTLE_FLOOR, Red, true), 3);
    }

    #[test]
    fn work_is_the_cheaper_of_the_two_axes() {
        // two reds along the floor beside a red virus: one block finishes the row, where the
        // column above it is still three blocks away
        let grid = Grid::of(&bottle(&[
            virus(2, 15, Red),
            stack(0, 15, Red),
            stack(1, 15, Red),
        ]));
        assert_eq!(grid.work_on(2, 15, Red, true), 1);
        assert_eq!(grid.work_on(2, 15, Red, false), 3);
        assert_eq!(grid.work(2, 15), 1);
    }

    #[test]
    fn empty_bottle_has_nothing_in_it() {
        assert_eq!(bottle(&[]).stats(), BottleStats::default());
    }

    #[test]
    fn counts_viruses_apart_from_blocks() {
        let stats = bottle(&[virus(0, 15, Red), virus(2, 15, Blue), stack(4, 15, Red)]).stats();
        assert_eq!(stats.viruses(), 2);
        // three blocks of work each, and nothing is buried in an otherwise empty bottle
        assert_eq!(stats.virus_work(), 6);
        assert_eq!(stats.block_work(), 3);
        assert_eq!(stats.viruses_buried(), 0);
        assert_eq!(stats.blocks_buried(), 0);
    }

    #[test]
    fn work_falls_as_the_line_fills_in() {
        let one = bottle(&[virus(0, 15, Red), stack(1, 15, Red)]).stats();
        assert_eq!(one.virus_work(), 2);
        let two = bottle(&[virus(0, 15, Red), stack(1, 15, Red), stack(2, 15, Red)]).stats();
        assert_eq!(two.virus_work(), 1);
    }

    #[test]
    fn a_walled_in_virus_is_buried_and_costs_no_work_at_all() {
        // reds either side of a blue virus in the only row it has, and the column above it is
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
        assert_eq!(stats.viruses(), 1);
        assert_eq!(stats.viruses_buried(), 1);
        assert_eq!(stats.virus_work(), 0);
    }

    #[test]
    fn blocks_are_measured_exactly_as_viruses_are() {
        // a lone blue half walled in by reds along its row and capped by reds up its column
        let mut blocks = vec![];
        for x in 0..BOTTLE_WIDTH {
            blocks.push(stack(x, 15, Red));
            if x != 3 {
                blocks.push(stack(x, 14, Red));
            }
        }
        for y in 11..14 {
            blocks.push(stack(3, y, Red));
        }
        blocks.push(stack(3, 14, Blue));

        let stats = bottle(&blocks).stats();
        assert_eq!(stats.blocks_buried(), 1);
        assert_eq!(stats.viruses(), 0);
    }

    #[test]
    fn heights_are_measured_from_the_floor() {
        let stats = bottle(&[stack(0, 15, Red), stack(3, 14, Blue), stack(3, 15, Blue)]).stats();
        assert_eq!(stats.max_height(), 2);
    }

    #[test]
    fn counts_the_subjects_one_block_from_going() {
        // two reds beside a red virus: the virus and both blocks are all one block short
        let one_away = bottle(&[virus(0, 15, Red), stack(1, 15, Red), stack(2, 15, Red)]).stats();
        assert_eq!(one_away.viruses_at_work_1(), 1);
        assert_eq!(one_away.blocks_at_work_1(), 2);

        // take one away and nothing is one short any more, though the work sum is only two
        // larger - which is the whole reason this is counted apart from the sum
        let two_away = bottle(&[virus(0, 15, Red), stack(1, 15, Red)]).stats();
        assert_eq!(two_away.viruses_at_work_1(), 0);
        assert_eq!(two_away.blocks_at_work_1(), 0);
        assert_eq!(one_away.virus_work() + one_away.block_work(), 3);
        assert_eq!(two_away.virus_work() + two_away.block_work(), 4);
    }

    #[test]
    fn a_sum_of_work_cannot_tell_two_bottles_apart_that_the_count_can() {
        // four red viruses in their own quarters of the floor. Two of them have a red beside
        // them and two have nothing: {2, 2, 3, 3}. Ten blocks of work.
        let mixed = bottle(&[
            virus(0, 15, Red),
            stack(1, 15, Red),
            virus(4, 15, Red),
            stack(5, 15, Red),
            virus(0, 11, Red),
            virus(4, 11, Red),
        ])
        .stats();
        // and four with a red *and* a second red beside two of them: {1, 1, 3, 3}, eight
        assert_eq!(mixed.viruses(), 4);
        assert_eq!(mixed.virus_work(), 10);
        assert_eq!(mixed.viruses_at_work_1(), 0);

        let sharp = bottle(&[
            virus(0, 15, Red),
            stack(1, 15, Red),
            stack(2, 15, Red),
            virus(4, 15, Red),
            stack(5, 15, Red),
            stack(6, 15, Red),
            virus(0, 11, Red),
            virus(4, 11, Red),
        ])
        .stats();
        assert_eq!(sharp.viruses(), 4);
        assert_eq!(sharp.viruses_at_work_1(), 2);
    }

    /// the two cells a pill put down, as [`placement_stats`] reads them
    fn placed(blocks: &[(u32, u32, Block)], landed: &[(u32, u32)]) -> PlacementStats {
        let grid = Grid::of(&bottle(blocks));
        let points: Vec<BottlePoint> = landed
            .iter()
            .map(|(x, y)| BottlePoint::new(*x as i32, *y as i32))
            .collect();
        placement_stats(&grid, &points, 0)
    }

    #[test]
    fn the_halves_are_read_where_the_pill_put_them() {
        // a red half landing on the end of two reds is one block from a clear
        let stats = placed(
            &[virus(0, 15, Red), stack(1, 15, Red), stack(2, 15, Red)],
            &[(2, 15)],
        );
        assert_eq!(stats.halves_work(), 1);
        assert_eq!(stats.halves_buried(), 0);

        // the better of the two halves is what is reported: one at work 1, one at work 3
        let stats = placed(
            &[
                virus(0, 15, Red),
                stack(1, 15, Red),
                stack(2, 15, Red),
                stack(7, 15, Red),
            ],
            &[(2, 15), (7, 15)],
        );
        assert_eq!(stats.halves_work(), 1);
    }

    #[test]
    fn a_half_no_line_can_ever_join_is_counted_as_buried_rather_than_as_work() {
        // a lone blue walled in by reds along its row and capped by reds up its column
        let mut blocks = vec![];
        for x in 0..BOTTLE_WIDTH {
            blocks.push(stack(x, 15, Red));
            if x != 3 {
                blocks.push(stack(x, 14, Red));
            }
        }
        for y in 11..14 {
            blocks.push(stack(3, y, Red));
        }
        blocks.push(stack(3, 14, Blue));

        let stats = placed(&blocks, &[(3, 14)]);
        assert_eq!(stats.halves_buried(), 1);
        assert_eq!(stats.halves_work(), HALF_BURIED);
    }

    #[test]
    fn the_delta_is_how_the_bottle_moved() {
        let before = bottle(&[virus(0, 15, Red)]).stats();
        let after = bottle(&[virus(0, 15, Red), stack(1, 15, Red)]).stats();
        let delta = after - before;
        // one block of work came off the virus, and the half that did it brought its own
        assert_eq!(delta.virus_work(), -1);
        assert_eq!(delta.viruses(), 0);
        assert_eq!(delta.block_work(), 2);
    }
}
