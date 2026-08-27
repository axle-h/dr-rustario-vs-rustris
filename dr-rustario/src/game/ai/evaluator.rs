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
//! The last few inputs are deliberately *not* centred. They say what kind of bottle this is -
//! how much is left in it, how buried it is, how high it stands - and they are the same for
//! every candidate by construction, so they cannot rank anything. They are there because the
//! N64 runs what amounts to two opposite policies, one while it is digging a full bottle out
//! and another once the end is in sight, and a network with no idea which it is in can only
//! learn the average of the two.

use crate::game::ai::features::BottleFeatures;
use crate::game::ai::models::DrNeuralNetwork;
use engine::ai::{Tensor, BOTTLE_FEATURE_INPUTS};

/// How many of the inputs are comparative, and so centred on the pill's own candidates. The
/// rest are the context block at the end.
pub const COMPARATIVE: usize = 25;

/// Roughly how far each input moves between the placements of one pill, which is what it is
/// divided by. The layers are sigmoid activated, so inputs have to arrive at about the same
/// size as each other or the first layer saturates and every placement scores the same.
#[rustfmt::skip]
const SPREAD: [f64; BOTTLE_FEATURE_INPUTS] = [
    // the bottle the placement leaves behind, as a change
    4.0,   // viruses
    12.0,  // virus work
    8.0,   // buried viruses
    12.0,  // buried blocks
    4.0,   // max height
    6.0,   // holes
    2.0, 2.0, 2.0, 2.0, 2.0, 2.0, // runs one and two short, by axis
    // what the placement did
    2.0,   // patterns cleared
    4.0,   // touching
    4.0,   // reach
    2.0,   // open 3
    2.0,   // open 2
    4.0,   // viruses in the run
    2.0,   // stranded
    2.0,   // stranded on a virus
    2.0,   // covers a virus
    2.0,   // buries a virus
    4.0,   // one away
    2.0,   // one away, taking a virus
    1.0,   // chains
    // what kind of bottle this is, which is not centred
    100.0, // viruses left
    200.0, // work left
    16.0,  // how high it stands
    32.0,  // holes in it
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
        delta.viruses() as f64,
        delta.virus_work() as f64,
        delta.buried_viruses() as f64,
        delta.buried_blocks() as f64,
        delta.max_height() as f64,
        delta.holes() as f64,
        delta.virus_3_row() as f64,
        delta.virus_3_col() as f64,
        delta.virus_2_row() as f64,
        delta.virus_2_col() as f64,
        delta.block_3_row() as f64,
        delta.block_3_col() as f64,
        placement.patterns_cleared() as f64,
        placement.touching() as f64,
        placement.reach() as f64,
        placement.open_3() as f64,
        placement.open_2() as f64,
        placement.run_viruses() as f64,
        placement.stranded() as f64,
        placement.stranded_on_virus() as f64,
        placement.covers_virus() as f64,
        placement.buries_virus() as f64,
        placement.one_away() as f64,
        placement.one_away_virus() as f64,
        placement.chains() as f64,
        before.viruses() as f64,
        before.virus_work() as f64,
        before.max_height() as f64,
        before.holes() as f64,
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

/// The hand written baseline, weighted the way the N64 turned out to weigh things rather than
/// the way it looks like it ought to: getting a virus out matters, but leaving a half where no
/// line can ever join it costs more, and a run one short of a match with room to finish is
/// worth more than most clears.
fn linear(features: &BottleFeatures) -> f64 {
    let delta = features.delta();
    let placement = features.placement();

    // delta.viruses() is negative when the placement killed some
    -300.0 * delta.viruses() as f64
        - 40.0 * delta.virus_work() as f64
        - 25.0 * delta.buried_viruses() as f64
        - 5.0 * delta.buried_blocks() as f64
        - 8.0 * delta.max_height() as f64
        - 15.0 * delta.holes() as f64
        + 30.0 * delta.virus_3_row() as f64
        + 20.0 * delta.virus_3_col() as f64
        + 8.0 * delta.virus_2_row() as f64
        + 5.0 * delta.virus_2_col() as f64
        + 4.0 * delta.block_3_row() as f64
        + 2.0 * delta.block_3_col() as f64
        + 40.0 * placement.open_3() as f64
        + 12.0 * placement.open_2() as f64
        + 6.0 * placement.reach() as f64
        + 8.0 * placement.run_viruses() as f64
        - 90.0 * placement.stranded() as f64
        - 60.0 * placement.stranded_on_virus() as f64
        - 30.0 * placement.buries_virus() as f64
        + 6.0 * placement.one_away() as f64
        + 20.0 * placement.one_away_virus() as f64
        + 25.0 * placement.chains() as f64
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
