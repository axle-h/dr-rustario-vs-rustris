//! Scoring a candidate placement with the trained network.
//!
//! A scorer only ever has to separate the placements of *one pill* from each other, and that is
//! what decides how the features are fed in. Everything comparative is centred on the mean over
//! the candidates in front of it, so what reaches the network is what makes this placement
//! different from the others rather than what the bottle happens to look like today. Without
//! that the numbers drift: [`crate::game::ai::probe`] measured a network's output moving three
//! times further from pill to pill than it did between the placements of one pill, which makes
//! most of its choices rounding error.
//!
//! The context inputs at the end are deliberately *not* centred. They are the same for every
//! candidate by construction, so they cannot rank anything; they are there because the N64 runs
//! what amounts to two opposite policies, one while it is digging a full bottle out and another
//! once the end is in sight, and a network with no idea which it is in can only learn the
//! average of the two. Which is also why the context block is the bottle **before** the pill:
//! the after-bottle varies per candidate and [`inputs`] would centre it away.
//!
//! **These were selected, not designed.** [`crate::game::ai::features`] measures more than is
//! fed - every field of `BottleStats` and `PlacementStats` comes out of the same pass and costs
//! almost nothing extra - and what is fed is what `ga dr screen` chose by adding and removing
//! one input at a time, ranked on the median of fifty taught clones. Three of those measurements
//! are worth writing down, because each is the opposite of what it looks like:
//!
//! * **A number is not an indicator.** `place.halves_work` says the better placed half is one
//!   block short; `place.halves_one_short` and `place.halves_two_short` say the same thing as
//!   counts. Feeding the two indicators as well as the number is worth **+763** on the median,
//!   and neither indicator is worth anything without the other - drop either alone and nothing
//!   moves, drop both and it all goes. A sigmoid layer should be able to carve "exactly one"
//!   out of a continuous input and in practice it does not.
//! * **The tallest column is the wrong height.** `delta.max_height` costs 201 to feed;
//!   `delta.landing_height` - the *shortest* column, which is the lowest a pill can still be
//!   put - is worth 337. One virus on the floor makes the tallest column 1 and changes nothing
//!   about the game, because every other column is still open to the floor.
//! * **More inputs are not better.** The same measurements with eight more of their own kind
//!   fed alongside score a median of 3222 against this set's 3749. An input the network has to
//!   learn to ignore is not free.
//!
//! To try a different set, change [`raw_inputs`] and [`SPREAD`], move [`COMPARATIVE`] to how
//! many are centred, and set `BOTTLE_FEATURE_INPUTS`. `ga dr screen`'s silencing does the search
//! without any of that.

use crate::game::ai::features::BottleFeatures;
use crate::game::ai::models::DrNeuralNetwork;
use engine::ai::{Tensor, BOTTLE_FEATURE_INPUTS};

/// How many of the inputs are comparative, and so centred on the pill's own candidates. The
/// rest are the context block at the end.
pub const COMPARATIVE: usize = 16;

/// Roughly how far each input moves between the placements of one pill, which is what it is
/// divided by. The layers are sigmoid activated, so inputs have to arrive at about the same
/// size as each other or the first layer saturates and every placement scores the same.
#[rustfmt::skip]
const SPREAD: [f64; BOTTLE_FEATURE_INPUTS] = [
    // how the bottle moved
    12.0,  // the work its viruses still need
    12.0,  // the same along rows only
    12.0,  // and down columns only
    4.0,   // viruses no line can reach any more
    12.0,  // the work everything else still needs
    4.0,   // blocks no line can reach any more
    // what the placement did
    2.0,   // the work the better of the two placed halves still needs
    4.0,   // the longest run actually touching one of them
    4.0,   // viruses in the line they are working on
    2.0,   // placed halves exactly one short
    2.0,   // placed halves exactly two short
    // what is one block from going, by axis
    2.0,   // viruses one from dying along a row
    2.0,   // and down a column
    2.0,   // blocks one from clearing along a row
    2.0,   // and down a column
    3.0,   // the lowest a pill can still be put
    // what kind of bottle this is, which is not centred
    50.0,  // blocks already one from clearing
    30.0,  // viruses already one from dying
    1.0,   // whether this is the held pill rather than the one in play
];

/// Every measurement, in the network's own order and in its own units, before any centring.
/// The first [`COMPARATIVE`] are about this placement against the others; the rest are context.
pub fn raw_inputs(features: &BottleFeatures) -> [f64; BOTTLE_FEATURE_INPUTS] {
    let delta = features.delta();
    // the bottle as it was before the pill, which is the same for every candidate: context to
    // gate on, with none of the ranking signal its delta already carries
    let before = features.global() - delta;
    let placement = features.placement();

    [
        delta.virus_work() as f64,
        delta.virus_work_row() as f64,
        delta.virus_work_col() as f64,
        delta.viruses_buried() as f64,
        delta.block_work() as f64,
        delta.blocks_buried() as f64,
        placement.halves_work() as f64,
        placement.halves_touching() as f64,
        placement.halves_run_viruses() as f64,
        placement.halves_one_short() as f64,
        placement.halves_two_short() as f64,
        delta.viruses_at_work_1_row() as f64,
        delta.viruses_at_work_1_col() as f64,
        delta.blocks_at_work_1_row() as f64,
        delta.blocks_at_work_1_col() as f64,
        delta.landing_height() as f64,
        before.blocks_at_work_1() as f64,
        before.viruses_at_work_1() as f64,
        features.held() as u8 as f64,
    ]
}

/// The rows a network is actually shown: [`raw_inputs`] for every candidate of one pill, with
/// the comparative block centred on the mean over them and everything brought to about the same
/// size.
pub fn inputs(candidates: &[BottleFeatures]) -> Vec<[f64; BOTTLE_FEATURE_INPUTS]> {
    let mut rows: Vec<[f64; BOTTLE_FEATURE_INPUTS]> = candidates.iter().map(raw_inputs).collect();
    if rows.is_empty() {
        return rows;
    }

    for input in 0..COMPARATIVE {
        let mean = rows.iter().map(|row| row[input]).sum::<f64>() / rows.len() as f64;
        for row in rows.iter_mut() {
            row[input] -= mean;
        }
    }
    for row in rows.iter_mut() {
        for (value, spread) in row.iter_mut().zip(SPREAD.iter()) {
            *value /= spread;
        }
    }
    rows
}

/// How a candidate placement is scored. The linear scorer is a hand written baseline: it is
/// what the features say if you just weight them by hand, and it is the yardstick a trained
/// model has to beat.
#[derive(Clone, Copy, Debug)]
pub enum Scorer {
    Linear,
    Network(DrNeuralNetwork),
}

impl Scorer {
    /// Score every placement of one pill at once, which is the only way the network can be
    /// shown what separates them. The scores are comparable within the call and nowhere else.
    pub fn rank(&self, candidates: &[BottleFeatures]) -> Vec<f64> {
        match self {
            Scorer::Linear => candidates.iter().map(linear).collect(),
            Scorer::Network(network) => inputs(candidates)
                .into_iter()
                .map(|row| network.forward(&Tensor::vector(row)).value())
                .collect(),
        }
    }
}

/// The hand written baseline, in the order the game itself would put these things: bringing a
/// virus nearer to killable is what the game is, walling one in where no line can ever reach it
/// is the worst, and a half left one block from a clear is what a placement is for. Nothing here
/// says a virus actually *died*: the selection left that input out, because the work counts
/// already say it.
fn linear(features: &BottleFeatures) -> f64 {
    let delta = features.delta();
    let placement = features.placement();

    -40.0 * delta.virus_work() as f64
        - 20.0 * delta.virus_work_row() as f64
        - 10.0 * delta.virus_work_col() as f64
        - 120.0 * delta.viruses_buried() as f64
        - 8.0 * delta.block_work() as f64
        - 30.0 * delta.blocks_buried() as f64
        - 25.0 * placement.halves_work() as f64
        + 6.0 * placement.halves_touching() as f64
        + 8.0 * placement.halves_run_viruses() as f64
        + 40.0 * placement.halves_one_short() as f64
        + 12.0 * placement.halves_two_short() as f64
        + 30.0 * delta.viruses_at_work_1_row() as f64
        + 20.0 * delta.viruses_at_work_1_col() as f64
        + 4.0 * delta.blocks_at_work_1_row() as f64
        + 2.0 * delta.blocks_at_work_1_col() as f64
        - 15.0 * delta.landing_height() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ai::features::{BottleAnalysis, BottleStats, PlacementStats};
    use crate::game::block::Block;
    use crate::game::bottle::{Bottle, BOTTLE_HEIGHT, BOTTLE_WIDTH};
    use crate::game::pill::VirusColor;

    fn features(bottle: &Bottle) -> BottleFeatures {
        BottleFeatures::new(
            bottle.stats(),
            BottleStats::default(),
            PlacementStats::default(),
        )
    }

    #[test]
    fn every_input_stays_in_range_even_for_a_full_bottle() {
        // the worst case for the raw counts: every cell occupied, so the work count, holes and
        // virus count are all at their largest
        let mut bottle = Bottle::new();
        for y in 0..BOTTLE_HEIGHT {
            for x in 0..BOTTLE_WIDTH {
                bottle.place(x, y, Block::Virus(VirusColor::Red));
            }
        }

        let rows = inputs(&[features(&bottle)]);
        for (i, value) in rows[0].iter().enumerate() {
            assert!(
                value.abs() <= 1.5,
                "input {} is {}, far enough from zero to saturate a sigmoid layer",
                i,
                value
            );
        }
    }

    #[test]
    fn the_held_flag_is_the_last_input_and_is_never_centred_away() {
        let bottle = Bottle::new();
        let in_play = features(&bottle);
        let swapped = features(&bottle).of_the_held_pill();

        assert_eq!(raw_inputs(&in_play)[BOTTLE_FEATURE_INPUTS - 1], 0.0);
        assert_eq!(raw_inputs(&swapped)[BOTTLE_FEATURE_INPUTS - 1], 1.0);

        // pooled into one call it still says which is which, since it is context and not
        // centred: the whole point of it is that it survives the comparison
        let rows = inputs(&[in_play, swapped]);
        assert_eq!(rows[0][BOTTLE_FEATURE_INPUTS - 1], 0.0);
        assert_eq!(rows[1][BOTTLE_FEATURE_INPUTS - 1], 1.0);
    }

    #[test]
    fn a_comparative_input_is_centred_on_the_candidates_and_the_context_is_not() {
        let empty = Bottle::new();
        let mut stacked = Bottle::new();
        stacked.place(0, BOTTLE_HEIGHT - 1, Block::Virus(VirusColor::Red));

        let rows = inputs(&[features(&empty), features(&stacked)]);
        // one virus between two candidates: the comparative reading of it is half either side
        assert!(rows[0][0] < 0.0 && rows[1][0] > 0.0);
        assert_eq!(rows[0][0], -rows[1][0]);
        // the context block is the bottle before the pill, so it is the same for both and
        // keeps its own level rather than being centred away
        assert_eq!(rows[0][COMPARATIVE], rows[1][COMPARATIVE]);
    }
}
