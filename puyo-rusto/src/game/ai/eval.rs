//! What a field is worth: fifteen weighted terms, after ama's `ai/search/beam/eval.cpp` (MIT).
//! The biggest is [`quiet`]'s answer to what chain the field is holding.

use crate::game::ai::field::{Field, SPAWN_COLUMN, WIDTH};
use crate::game::ai::quiet;

/// The weights, one per term. Signs are meant: a term measuring a cost carries a negative
/// weight and is written so that bigger is worse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Weights {
    /// how many steps the chain the field is holding would run to
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
    /// Ama's `build`, term for term.
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

    /// Ama's `fast`: less care about taking a hit and about how tall the trigger stands.
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

    /// Pops four in a row and values no chain it cannot see yet, keeping the board flat. Puyo
    /// VS's cpu (`Puyolib/AI.cpp`) with its random placement replaced.
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
    /// Whether any weight depends on the chain the field is holding; if not, the quiescence
    /// search is skipped.
    fn reads_potential(&self) -> bool {
        self.chain != 0 || self.trigger_height != 0 || self.key != 0 || self.chi != 0
    }
}

/// What a field is worth to a player holding these weights.
pub fn evaluate(field: &Field, w: &Weights) -> i32 {
    let heights = field.heights();
    let mut score = 0i32;

    // only the best chain the field holds counts: a spare trigger does not double it
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

/// What the placement itself cost. Kept apart from [`evaluate`] because a search sums it along
/// the path while the field is scored as it stands.
pub fn action(tear: u32, waste: u32, w: &Weights) -> i32 {
    tear as i32 * w.tear + waste as i32 * w.waste
}

/// How far a chain set off in `column` could still be stretched sideways, counting columns no
/// taller and again columns strictly shorter so a step down weighs more. Ama's `get_chi`.
fn chi(heights: &[u8; WIDTH], column: usize) -> i32 {
    let at = heights[column];
    let mut chi = 0;
    for pass in 0..2 {
        // the second pass skips columns level with the trigger
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

/// Total deviation from the ideal leaning profile, three columns a little above average on the
/// left and three below on the right, which every staircase and GTR is laid into.
fn shape(heights: &[u8; WIDTH]) -> i32 {
    const IDEAL: [i32; WIDTH] = [1, 1, 1, -1, -1, -1];
    let average = heights.iter().map(|h| *h as i32).sum::<i32>() / WIDTH as i32;
    (0..WIDTH)
        .map(|x| (heights[x] as i32 - average - IDEAL[x]).abs())
        .sum()
}

/// How deep the field's wells are; a deep one can only be filled by a pair on end.
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

/// Ghost row cells walled off from the spawn column, since a pair moves sideways with one half
/// in the ghost row. Ama's `waste_14`.
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

/// How much lower the spawn column stands than the taller side, where the death square is.
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

    /// a field holding a chain beats the same puyos in a heap
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

    /// a field holding no chain is untouched by the chain weight however heavy
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

    /// how much a ghost row puyo shuts off depends where it is
    #[test]
    fn a_ghost_row_puyo_walls_off_everything_past_it() {
        assert_eq!(walled_off_ghost_cells(0b000000), 0);
        assert_eq!(walled_off_ghost_cells(0b100000), 1, "the far right column");
        assert_eq!(walled_off_ghost_cells(0b010000), 2, "and the one past it");
        assert_eq!(walled_off_ghost_cells(0b000001), 1, "the far left column");
    }

    /// a field leaning the way a chain is built beats a flat one
    #[test]
    fn the_ideal_shape_leans() {
        assert!(shape(&[5, 5, 5, 3, 3, 3]) < shape(&[4, 4, 4, 4, 4, 4]));
        assert!(shape(&[3, 3, 3, 5, 5, 5]) > shape(&[4, 4, 4, 4, 4, 4]));
    }
}
