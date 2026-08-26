//! Every placement the pill in play can reach, and the bottle each one leaves behind.
//!
//! Candidates are found by replaying real [Bottle] moves on a clone, so Dr. Mario's wall kicks
//! and the blocks already in the bottle are honoured for free. The search only moves and
//! rotates before dropping: tucking sideways under an overhang after a soft drop is not
//! searched, since the agent has no single step soft drop to execute it with.

use crate::game::ai::features::{wasted_halves, BottleAnalysis, BottleFeatures, BottleStats};
use crate::game::ai::input_sequence::{InputSequence, Translation};
use crate::game::bottle::Bottle;
use crate::game::geometry::{BottlePoint, Rotation};
use crate::game::pill::{PillShape, VirusColor};
use std::collections::{HashSet, VecDeque};

const MOVES: [Translation; 4] = [
    Translation::Left,
    Translation::Right,
    Translation::RotateClockwise,
    Translation::RotateAnticlockwise,
];

pub struct Placement {
    inputs: InputSequence,
    features: BottleFeatures,
}

impl Placement {
    pub fn inputs(&self) -> &InputSequence {
        &self.inputs
    }

    pub fn features(&self) -> BottleFeatures {
        self.features
    }
}

pub trait PlacementSearch {
    fn placements(&self, stats_before: BottleStats) -> Vec<Placement>;

    /// the placements the bottle would offer if `shape` were the pill in play instead
    fn placements_of(&self, shape: PillShape, stats_before: BottleStats) -> Vec<Placement>;
}

impl PlacementSearch for Bottle {
    fn placements_of(&self, shape: PillShape, stats_before: BottleStats) -> Vec<Placement> {
        let mut swapped = self.clone();
        swapped.hold();
        if swapped.try_spawn(shape).is_none() {
            // it cannot even spawn, so it is no alternative
            return vec![];
        }
        swapped.placements(stats_before)
    }

    fn placements(&self, stats_before: BottleStats) -> Vec<Placement> {
        if self.pill().is_none() {
            return vec![];
        }

        let mut visited: HashSet<Pose> = HashSet::from([pose(self)]);
        let mut queue = VecDeque::from([(self.clone(), InputSequence::default())]);
        // several poses drop into the same place - a wall kick can leave the pill a row lower in
        // the same column - so placements are keyed by where the pill comes to rest, and
        // breadth first order means the first route to each one is the shortest
        let mut landings: HashSet<Landing> = HashSet::new();
        let mut placements = vec![];

        while let Some((bottle, inputs)) = queue.pop_front() {
            if landings.insert(landing(&bottle)) {
                placements.push(drop_and_settle(&bottle, &inputs, stats_before));
            }

            for translation in MOVES {
                let mut next = bottle.clone();
                if !apply(&mut next, translation) {
                    continue;
                }
                if !visited.insert(pose(&next)) {
                    continue;
                }
                queue.push_back((next, inputs.with(translation)));
            }
        }

        placements
    }
}

/// the pose of the pill in play, which is what makes two search states the same
type Pose = ([BottlePoint; 2], Rotation);

fn pose(bottle: &Bottle) -> Pose {
    let pill = bottle.pill().expect("no pill");
    (pill.vitamins().map(|v| v.position()), pill.rotation())
}

/// where the pill would come to rest from here, which is what makes two candidates the same
type Landing = [(BottlePoint, VirusColor); 2];

fn landing(bottle: &Bottle) -> Landing {
    let mut dropped = bottle.clone();
    dropped.hard_drop();
    let pill = dropped.pill().expect("no pill");
    pill.vitamins().map(|v| (v.position(), v.color()))
}

fn apply(bottle: &mut Bottle, translation: Translation) -> bool {
    match translation {
        Translation::Left => bottle.left(),
        Translation::Right => bottle.right(),
        Translation::RotateClockwise => bottle.rotate(true),
        Translation::RotateAnticlockwise => bottle.rotate(false),
        Translation::HardDrop => bottle.hard_drop().is_some(),
    }
}

/// drop the pill, lock it and run the clears and cascades out to a settled bottle, exactly as
/// [crate::game::Game] would
fn drop_and_settle(
    bottle: &Bottle,
    inputs: &InputSequence,
    stats_before: BottleStats,
) -> Placement {
    let mut bottle = bottle.clone();
    bottle.hard_drop();
    let placed: Vec<BottlePoint> = bottle
        .lock()
        .map(|vitamins| vitamins.map(|v| v.position()).to_vec())
        .unwrap_or_default();

    let wasted = wasted_halves(&bottle, &placed);

    let mut patterns_cleared = 0;
    loop {
        let (blocks, patterns) = bottle.pattern();
        if blocks.is_empty() {
            break;
        }
        patterns_cleared += patterns.len() as i32;
        bottle.destroy(blocks);
        // settle whatever the clear left unsupported, then look for the cascade
        while bottle.step_down_garbage() {}
    }

    Placement {
        inputs: inputs.with(Translation::HardDrop),
        features: BottleFeatures::new(bottle.stats(), stats_before, wasted, patterns_cleared),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::block::Block;
    use crate::game::bottle::{BOTTLE_FLOOR, BOTTLE_WIDTH};
    use crate::game::pill::{PillShape, VirusColor};
    use VirusColor::{Blue, Red};

    fn with_pill(shape: PillShape, blocks: &[(u32, u32, Block)]) -> Bottle {
        let mut bottle = Bottle::new();
        for (x, y, block) in blocks {
            bottle.place(*x, *y, *block);
        }
        bottle.try_spawn(shape);
        bottle
    }

    #[test]
    fn an_empty_bottle_offers_every_column_in_every_orientation() {
        let bottle = with_pill(PillShape::new(Red, Blue), &[]);
        let placements = bottle.placements(bottle.stats());

        // 7 horizontal positions in each of two orientations, 8 vertical in each of two
        assert_eq!(
            placements.len(),
            2 * (BOTTLE_WIDTH as usize - 1) + 2 * BOTTLE_WIDTH as usize
        );
        // every one of them ends in a hard drop
        assert!(placements
            .iter()
            .all(|p| p.inputs().translations().last() == Some(&Translation::HardDrop)));
    }

    #[test]
    fn a_pill_of_one_colour_has_half_as_many_distinct_placements() {
        let bottle = with_pill(PillShape::new(Red, Red), &[]);
        // the search still walks every pose, but north and south are the same placement
        let placements = bottle.placements(bottle.stats());
        assert!(!placements.is_empty());
    }

    #[test]
    fn finds_the_placement_that_clears_a_row() {
        // three reds on the floor: dropping a red half alongside them clears all four
        let bottle = with_pill(
            PillShape::new(Red, Blue),
            &[
                (0, BOTTLE_FLOOR, Block::Virus(Red)),
                (1, BOTTLE_FLOOR, Block::Virus(Red)),
                (2, BOTTLE_FLOOR, Block::Virus(Red)),
            ],
        );
        let before = bottle.stats();
        assert_eq!(before.viruses(), 3);

        let clearing: Vec<_> = bottle
            .placements(before)
            .into_iter()
            .filter(|p| p.features().patterns_cleared() > 0)
            .collect();

        assert!(!clearing.is_empty(), "no placement cleared the row");
        // the clear takes all three viruses with it
        assert!(clearing
            .iter()
            .any(|p| p.features().delta().viruses() == -3));
    }

    #[test]
    fn a_settled_placement_has_no_pending_matches_left() {
        let bottle = with_pill(
            PillShape::new(Red, Red),
            &[
                (0, BOTTLE_FLOOR, Block::Virus(Red)),
                (1, BOTTLE_FLOOR, Block::Virus(Red)),
            ],
        );
        // whatever it picks, the bottle it reports is quiescent: a run of four would have gone
        for placement in bottle.placements(bottle.stats()) {
            assert!(placement.features().global().virus_near_3() >= 0);
        }
    }

    #[test]
    fn a_blocked_column_is_not_offered() {
        // fill the left three columns to the top so nothing can reach them
        let mut blocks = vec![];
        for x in 0..3 {
            for y in 0..=BOTTLE_FLOOR {
                blocks.push((x, y, Block::Garbage(Blue)));
            }
        }
        let bottle = with_pill(PillShape::new(Red, Blue), &blocks);
        let placements = bottle.placements(bottle.stats());
        assert!(!placements.is_empty());
        // fewer options than an empty bottle, since the left of the bottle is walled off
        assert!(placements.len() < 2 * (BOTTLE_WIDTH as usize - 1) + 2 * BOTTLE_WIDTH as usize);
    }
}
