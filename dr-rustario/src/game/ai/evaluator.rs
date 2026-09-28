//! Scoring a candidate placement with the trained network.
//!
//! Comparative inputs are centred on the mean over one pill's candidates, since a scorer only
//! separates those; the context inputs at the end are the bottle before the pill, the same for
//! every candidate, and tell the network whether it is digging out or finishing.
//!
//! The nineteen inputs were selected by `ga dr screen` (medians over fifty taught clones, with
//! inputs silenced), and each costs something to learn to ignore, so do not add one without a
//! screen. To change the set, edit [`raw_inputs`] and [`SPREAD`], [`COMPARATIVE`] and
//! `BOTTLE_FEATURE_INPUTS`.

use crate::game::ai::features::BottleFeatures;
use crate::game::ai::models::DrNeuralNetwork;
use engine::ai::{Tensor, BOTTLE_FEATURE_INPUTS};

/// how many leading inputs are centred on the pill's own candidates; the rest are context
pub const COMPARATIVE: usize = 16;

/// Roughly how far each input moves between one pill's placements, which it is divided by so
/// that no input saturates the sigmoid first layer.
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

/// Every input in the network's order and its own units, before centring.
pub fn raw_inputs(features: &BottleFeatures) -> [f64; BOTTLE_FEATURE_INPUTS] {
    let delta = features.delta();
    // the same for every candidate, so it gates rather than ranks
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

/// [`raw_inputs`] for every candidate of one pill, the comparative block centred and all scaled
/// by [`SPREAD`].
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

/// How a candidate placement is scored: a hand weighted baseline or the network. Only the
/// `ga dr` modules use it, and they are not built under `cfg(test)`.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(test, allow(dead_code))]
#[allow(clippy::large_enum_variant)]
pub enum Scorer {
    Linear,
    Network(DrNeuralNetwork),
}

#[cfg_attr(test, allow(dead_code))]
impl Scorer {
    /// Score every placement of one pill at once; scores are comparable only within one call.
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

/// the hand weighted baseline over the comparative inputs
#[cfg_attr(test, allow(dead_code))]
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
        // every cell occupied, so the raw counts are at their largest
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

        // context is not centred, so pooled together it still says which is which
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
        // the context block keeps its own level
        assert_eq!(rows[0][COMPARATIVE], rows[1][COMPARATIVE]);
    }
}
