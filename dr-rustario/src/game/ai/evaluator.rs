//! Scoring a candidate placement with the trained network.

use crate::game::ai::features::BottleFeatures;
use crate::game::ai::models::DrNeuralNetwork;
use engine::ai::{Tensor, BOTTLE_FEATURE_INPUTS};

/// Roughly the largest each stat gets in a full bottle, used to bring every input into about
/// the same range. The layers are sigmoid activated, so raw counts - a virus count up to 99, an
/// virus count up to 99, a work count into the hundreds - push the first layer into saturation,
/// every candidate placement scores the same and the agent may as well be choosing at random.
mod scale {
    use crate::game::bottle::{BOTTLE_HEIGHT, TOTAL_BLOCKS};

    pub const VIRUSES: f64 = TOTAL_BLOCKS as f64;
    pub const NEAR_MATCHES: f64 = TOTAL_BLOCKS as f64 / 4.0;
    pub const BURIED: f64 = TOTAL_BLOCKS as f64;
    pub const HEIGHT: f64 = BOTTLE_HEIGHT as f64;
    /// every virus walled in at once
    pub const VIRUS_WORK: f64 = TOTAL_BLOCKS as f64;
    pub const HOLES: f64 = TOTAL_BLOCKS as f64;
    /// both halves of the pill
    pub const WASTED_HALVES: f64 = 2.0;
    pub const PATTERNS: f64 = 8.0;
}

/// pack the features into the network's inputs: the ten stats as deltas, then as globals, then
/// what the placement itself did, each scaled into about [-1, 1]
pub fn inputs(features: BottleFeatures) -> [f64; BOTTLE_FEATURE_INPUTS] {
    let delta = features.delta();
    let global = features.global();

    let mut values = [0.0; BOTTLE_FEATURE_INPUTS];

    let mut pack = |at: usize, stats: crate::game::ai::features::BottleStats| {
        values[at] = stats.viruses() as f64 / scale::VIRUSES;
        values[at + 1] = stats.virus_near_3() as f64 / scale::NEAR_MATCHES;
        values[at + 2] = stats.virus_near_2() as f64 / scale::NEAR_MATCHES;
        values[at + 3] = stats.block_near_3() as f64 / scale::NEAR_MATCHES;
        values[at + 4] = stats.block_near_2() as f64 / scale::NEAR_MATCHES;
        values[at + 5] = stats.buried_viruses() as f64 / scale::BURIED;
        values[at + 6] = stats.buried_blocks() as f64 / scale::BURIED;
        values[at + 7] = stats.max_height() as f64 / scale::HEIGHT;
        values[at + 8] = stats.virus_work() as f64 / scale::VIRUS_WORK;
        values[at + 9] = stats.holes() as f64 / scale::HOLES;
    };
    pack(0, delta);
    pack(STATS, global);

    values[2 * STATS] = features.wasted_halves() as f64 / scale::WASTED_HALVES;
    values[2 * STATS + 1] = features.patterns_cleared() as f64 / scale::PATTERNS;

    values
}

/// how many bottle statistics are fed in, once as a change and once as a total
const STATS: usize = 10;

/// How a candidate placement is scored. The linear scorer is a hand written baseline: it is
/// what the features say if you just weight them by hand, and it is the yardstick a trained
/// model has to beat.
#[derive(Clone, Copy, Debug)]
pub enum Scorer {
    Linear,
    Network(DrNeuralNetwork),
}

impl Scorer {
    pub fn evaluate(&self, features: BottleFeatures) -> f64 {
        match self {
            Scorer::Linear => linear(features),
            Scorer::Network(network) => {
                network.forward(&Tensor::vector(inputs(features))).value()
            }
        }
    }
}

/// getting viruses out of the bottle dominates; everything else only breaks ties between
/// placements that kill the same number
fn linear(features: BottleFeatures) -> f64 {
    let delta = features.delta();

    // delta.viruses() is negative when the placement killed some
    -1000.0 * delta.viruses() as f64
        - 40.0 * delta.virus_work() as f64
        + 60.0 * delta.virus_near_3() as f64
        + 12.0 * delta.virus_near_2() as f64
        + 6.0 * delta.block_near_3() as f64
        + 2.0 * delta.block_near_2() as f64
        - 25.0 * delta.buried_viruses() as f64
        - 5.0 * delta.buried_blocks() as f64
        - 8.0 * delta.max_height() as f64
        - 15.0 * delta.holes() as f64
        - 8.0 * features.wasted_halves() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ai::features::{BottleAnalysis, BottleStats};
    use crate::game::block::Block;
    use crate::game::bottle::{Bottle, BOTTLE_HEIGHT, BOTTLE_WIDTH};
    use crate::game::pill::VirusColor;

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

        let features = BottleFeatures::new(bottle.stats(), BottleStats::default(), 2, 4);
        for (i, value) in inputs(features).iter().enumerate() {
            assert!(
                value.abs() <= 1.5,
                "input {} is {}, far enough from zero to saturate a sigmoid layer",
                i,
                value
            );
        }
    }
}
