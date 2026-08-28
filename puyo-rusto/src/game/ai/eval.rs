//! What a field is worth.
//!
//! Fifteen numbers, weighted and added up. The shape of it - and most of the terms - is
//! ama's (`ai/search/beam/eval.cpp`, MIT), the strongest open Puyo Puyo Tsu ai; the
//! measurements are written out here against this game's own board rather than its bitboard,
//! and the two places they differ are noted where they happen.
//!
//! The single most important term is not on the board at all. [`quiet`] asks what chain the
//! field is *holding* - how long it would run if one more puyo were dropped on it, where, and
//! at what cost - and the best answer it finds is most of the score. Everything else is about
//! keeping the field in a state where such an answer keeps existing: flat enough to build on,
//! open over the spawn column, with its pairs and threes still joinable.

use crate::game::ai::field::{Field, SPAWN_COLUMN, WIDTH};
use crate::game::ai::quiet;

/// The weights, one per term. Signs are meant: a term measuring a cost carries a negative
/// weight and is written so that bigger is worse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Weights {
    /// how many steps the chain the field is holding would run to - the whole point
    pub chain: i32,
    /// how high up the column that would set it off stands
    pub trigger_height: i32,
    /// how many puyos it would take to set it off (a cost)
    pub key: i32,
    /// how much room that column has to stretch the chain further into
    pub chi: i32,
    /// groups of two and of three: the material a chain is built from
    pub link_2: i32,
    pub link_3: i32,
    /// how far the field is from a shape that can be built on (a cost)
    pub shape: i32,
    /// columns sunk below both their neighbours (a cost)
    pub well: i32,
    /// columns standing above both their neighbours (a cost)
    pub bump: i32,
    /// cells of the ghost row walled off from the spawn column (a cost)
    pub ghost: i32,
    /// nuisance sitting on the board (a cost)
    pub nuisance: i32,
    /// how much lower the spawn column is than the sides
    pub side: i32,
    /// a pair split across two columns of different heights (a cost, paid once)
    pub tear: i32,
    /// puyos spent popping something (a cost, paid once)
    pub waste: i32,
    /// what it is worth to leave a puyo resting on the death square
    pub death: i32,
}

impl Weights {
    /// The set to read first: it builds, it is not in a hurry, and it does not throw its
    /// board away. Ama's `build`, carried over term for term.
    pub const BUILD: Weights = Weights {
        chain: 1000,
        trigger_height: 289,
        key: -200,
        chi: 200,
        link_2: 150,
        link_3: 250,
        shape: -100,
        well: -100,
        bump: -100,
        ghost: -50,
        nuisance: -250,
        side: 0,
        tear: -250,
        waste: -250,
        death: -1_000_000,
    };

    /// Ama's `fast`: the same game played in less time. It cares much less about taking a
    /// hit and much less about how tall the trigger stands, which is what a player chasing a
    /// quick second chain looks like.
    pub const FAST: Weights = Weights {
        chain: 500,
        trigger_height: 77,
        key: -198,
        chi: 108,
        link_2: 56,
        link_3: 148,
        shape: -8,
        well: -5,
        bump: -5,
        ghost: -20,
        nuisance: -100,
        side: 0,
        tear: -104,
        waste: -98,
        death: -1_000_000,
    };

    /// Ama's `freestyle`: between the two, and flatter about where it builds.
    pub const FREESTYLE: Weights = Weights {
        chain: 500,
        trigger_height: 100,
        key: -200,
        chi: 100,
        link_2: 50,
        link_3: 150,
        shape: -50,
        well: -50,
        bump: -50,
        ghost: -20,
        nuisance: -200,
        side: 0,
        tear: -100,
        waste: -100,
        death: -1_000_000,
    };

    /// A player who has learned that four in a row pops and nothing else. It reads the field
    /// well enough not to stack itself into a wall, but it puts no value at all on a chain it
    /// cannot see yet, which is the difference between clearing puyos and playing Puyo.
    ///
    /// This is roughly what Puyo VS's own cpu does (`Puyolib/AI.cpp`: take the biggest chain
    /// on offer, and otherwise place at random), with the random half replaced by something
    /// that at least keeps the board flat.
    pub const GREEDY: Weights = Weights {
        chain: 0,
        trigger_height: 0,
        key: 0,
        chi: 0,
        link_2: 20,
        link_3: 40,
        shape: -20,
        well: -60,
        bump: -60,
        ghost: -50,
        nuisance: -50,
        side: 0,
        tear: 0,
        waste: 0,
        death: -1_000_000,
    };
}

impl Weights {
    /// Does anything this player thinks depend on the chain the field is holding?
    ///
    /// A row that answers no is not merely cheaper to run, it is playing a different game -
    /// it can see what pops and not what *would* pop - and the quiescence search, which is
    /// most of what an evaluation costs, is skipped outright for it.
    fn reads_potential(&self) -> bool {
        self.chain != 0 || self.trigger_height != 0 || self.key != 0 || self.chi != 0
    }
}

/// What a field is worth to a player holding these weights.
pub fn evaluate(field: &Field, w: &Weights) -> i32 {
    let heights = field.heights();
    let mut score = 0i32;

    // the chain the field is holding, which is most of the answer. Only the best one counts:
    // a field with two triggers is not twice the field, it is one chain with a spare
    let mut best: Option<i32> = None;
    quiet::search_if(w.reads_potential(), field, |trigger| {
        let (link_2, link_3) = trigger.remain.link_counts();
        let q = trigger.chain as i32 * w.chain
            + heights[trigger.column] as i32 * w.trigger_height
            + trigger.key as i32 * w.key
            + chi(&heights, trigger.column) * w.chi
            + link_2 as i32 * w.link_2
            + link_3 as i32 * w.link_3;
        best = Some(best.map_or(q, |b: i32| b.max(q)));
    });
    score += best.unwrap_or(0);

    score += shape(&heights) * w.shape;
    score += well(&heights) * w.well;
    score += bump(&heights) * w.bump;

    let (link_2, link_3) = field.link_counts();
    score += link_2 as i32 * w.link_2 + link_3 as i32 * w.link_3;

    score += walled_off_ghost_cells(field.ghost_row()) * w.ghost;
    score += field.nuisance_count() as i32 * w.nuisance;
    score += side_bias(&heights) * w.side;

    score
}

/// What the placement itself cost, as against the field it left behind.
///
/// Kept apart from [`evaluate`] because it is paid once and carried: a search several pairs
/// deep is the sum of what every placement along the way cost, while the field is only ever
/// the field as it stands now.
pub fn action(tear: u32, waste: u32, w: &Weights) -> i32 {
    tear as i32 * w.tear + waste as i32 * w.waste
}

/// How far a chain set off in `column` could still be stretched sideways.
///
/// A trigger with nothing but taller columns beside it is finished; one with a run of
/// shorter columns to grow into is not. Counted twice over in each direction - once for
/// columns no taller, and again for columns strictly shorter - so a step down counts for
/// more than a step level. Ama's `get_chi`.
fn chi(heights: &[u8; WIDTH], column: usize) -> i32 {
    let at = heights[column];
    let mut chi = 0;
    for pass in 0..2 {
        // the second pass stops one column earlier: a column exactly level with the trigger
        // is somewhere to grow, but it is not a step *down* into
        let strictly_shorter = pass == 1;
        for height in heights[column + 1..].iter() {
            if *height > at || (strictly_shorter && *height == at) {
                break;
            }
            chi += 1;
        }
        for height in heights[..column].iter().rev() {
            if *height > at || (strictly_shorter && *height == at) {
                break;
            }
            chi += 1;
        }
    }
    chi
}

/// How far the field is from a shape that can be built on.
///
/// The ideal is a field that leans: three columns a little above the average on the left and
/// three a little below on the right, which is the profile every staircase and every GTR is
/// laid into. Measured as the total deviation from it, so it is a cost.
fn shape(heights: &[u8; WIDTH]) -> i32 {
    const IDEAL: [i32; WIDTH] = [1, 1, 1, -1, -1, -1];
    let average = heights.iter().map(|h| *h as i32).sum::<i32>() / WIDTH as i32;
    (0..WIDTH)
        .map(|x| (heights[x] as i32 - average - IDEAL[x]).abs())
        .sum()
}

/// How deep the field's wells are: a column below both its neighbours can only be filled
/// from directly above, and a deep one can only be filled by a pair standing on end.
fn well(heights: &[u8; WIDTH]) -> i32 {
    let mut well = 0;
    for x in 0..WIDTH {
        let left = if x == 0 { None } else { Some(heights[x - 1]) };
        let right = if x + 1 == WIDTH {
            None
        } else {
            Some(heights[x + 1])
        };
        let bound = match (left, right) {
            (Some(l), Some(r)) => l.min(r),
            (Some(l), None) => l,
            (None, Some(r)) => r,
            (None, None) => heights[x],
        };
        if bound > heights[x] {
            well += (bound - heights[x]) as i32;
        }
    }
    well
}

/// How bumpy it is: a column above both its neighbours is a tower, and a tower is two wells.
fn bump(heights: &[u8; WIDTH]) -> i32 {
    let mut bump = 0;
    for x in 1..WIDTH - 1 {
        if heights[x] > heights[x - 1] && heights[x] > heights[x + 1] {
            bump += (heights[x] - heights[x - 1].max(heights[x + 1])) as i32;
        }
    }
    bump
}

/// How much of the ghost row has been walled off from the spawn column.
///
/// A pair moves sideways with one half in the ghost row, so a puyo resting up there is a
/// door closed: everything past it can no longer be reached even if the column under it is
/// empty. Counted as the cells of that row no longer reachable, which is ama's `waste_14`
/// against the row above its ghost row - the same row, on a board with one fewer.
fn walled_off_ghost_cells(row: u8) -> i32 {
    let mut reachable = 1;
    for x in SPAWN_COLUMN + 1..WIDTH {
        if row & (1 << x) != 0 {
            break;
        }
        reachable += 1;
    }
    for x in (0..SPAWN_COLUMN).rev() {
        if row & (1 << x) != 0 {
            break;
        }
        reachable += 1;
    }
    WIDTH as i32 - reachable
}

/// How much lower the spawn column stands than the taller of the two sides.
///
/// Building over the middle is how a field kills itself: the death square is there, and so is
/// the only way across. Ama measures it as the taller wing against the middle column.
fn side_bias(heights: &[u8; WIDTH]) -> i32 {
    let left: i32 = heights[..SPAWN_COLUMN].iter().map(|h| *h as i32).sum();
    let right: i32 = heights[SPAWN_COLUMN + 1..].iter().map(|h| *h as i32).sum();
    left.max(right) - heights[SPAWN_COLUMN] as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::board::tests::board;

    fn field(rows: &[&str]) -> Field {
        Field::from_board(&board(rows))
    }

    /// the whole point: a field holding a chain is worth more than the same puyos in a heap
    #[test]
    fn a_field_holding_a_chain_beats_a_field_that_is_only_tidy() {
        let holding = field(&[".g....", "rg....", "rrgg.."]);
        let heap = field(&["......", "rg....", "brgb.."]);
        let w = Weights::BUILD;
        assert!(
            evaluate(&holding, &w) > evaluate(&heap, &w),
            "holding {} vs heap {}",
            evaluate(&holding, &w),
            evaluate(&heap, &w)
        );
    }

    /// the chain weight is the quiescence search's, and a field holding nothing is not
    /// touched by it however heavily it is set - which is why the greedy row, whose chain
    /// weight is zero, plays a different game rather than a quieter one
    #[test]
    fn the_chain_weight_only_moves_a_field_that_is_holding_one() {
        let holding = field(&[".g....", "rg....", "rrgg.."]);
        let nothing = field(&["rgby.."]);
        let mut heavier = Weights::BUILD;
        heavier.chain *= 2;
        assert_ne!(
            evaluate(&holding, &Weights::BUILD),
            evaluate(&holding, &heavier)
        );
        assert_eq!(
            evaluate(&nothing, &Weights::BUILD),
            evaluate(&nothing, &heavier)
        );
    }

    #[test]
    fn a_flat_field_has_no_wells_and_no_bumps() {
        let flat = [4u8; WIDTH];
        assert_eq!(well(&flat), 0);
        assert_eq!(bump(&flat), 0);
    }

    #[test]
    fn a_sunken_column_is_a_well_and_a_tower_is_a_bump() {
        assert_eq!(well(&[4, 1, 4, 4, 4, 4]), 3);
        assert_eq!(bump(&[4, 8, 4, 4, 4, 4]), 4);
        // the edges have one neighbour, and a column sunk beside it is still a well
        assert_eq!(well(&[1, 4, 4, 4, 4, 4]), 3);
        assert_eq!(well(&[4, 4, 4, 4, 4, 1]), 3);
    }

    /// a puyo in the ghost row is a door closed, and how much it shuts off depends where it is
    #[test]
    fn a_ghost_row_puyo_walls_off_everything_past_it() {
        assert_eq!(walled_off_ghost_cells(0b000000), 0);
        assert_eq!(walled_off_ghost_cells(0b100000), 1, "the far right column");
        assert_eq!(walled_off_ghost_cells(0b010000), 2, "and the one past it");
        assert_eq!(walled_off_ghost_cells(0b000001), 1, "the far left column");
    }

    /// the deviation is from the *ideal* profile, not from flat: a field that leans the way
    /// a chain is built scores better than one that does not lean at all
    #[test]
    fn the_ideal_shape_leans() {
        assert!(shape(&[5, 5, 5, 3, 3, 3]) < shape(&[4, 4, 4, 4, 4, 4]));
        assert!(shape(&[3, 3, 3, 5, 5, 5]) > shape(&[4, 4, 4, 4, 4, 4]));
    }
}
