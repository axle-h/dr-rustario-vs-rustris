//! Every placement the pill in play can reach, and the bottle each one leaves behind.
//!
//! Candidates are found by replaying real [Bottle] moves on a clone, so wall kicks are honoured.
//!
//! Besides moving and rotating, the search may let the pill come to rest ([`Translation::Rest`])
//! and move on under an overhang, which extended lock down allows for another
//! [`engine::game::timing::Timing::lock`] per move. Resting is the only way down the search
//! takes, since a pill cannot fall past its rest and so nothing has to be timed.

use crate::game::ai::features::{placement_stats, BottleFeatures, BottleStats, Grid};
use crate::game::ai::input_sequence::{InputSequence, Translation};
use crate::game::bottle::Bottle;
use crate::game::geometry::{BottlePoint, Rotation};
use crate::game::pill::{PillShape, VirusColor};
use std::collections::{HashSet, VecDeque};

/// The moves the search walks. [`Translation::Rest`] is last so breadth first order gives the
/// agent the straight-drop route to a landing when one exists.
const MOVES: [Translation; 5] = [
    Translation::Left,
    Translation::Right,
    Translation::RotateClockwise,
    Translation::RotateAnticlockwise,
    Translation::Rest,
];

/// How far the search may walk the pill. Tucking doubles the placements and so the cost of a
/// training generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Reach {
    /// move and rotate, then drop: every placement is a straight fall from where the pill spawns
    Drop,
    /// the same, and the pill may come to rest and be walked on from there
    #[default]
    Tuck,
}

impl Reach {
    fn moves(&self) -> &'static [Translation] {
        match self {
            Reach::Drop => &MOVES[..4],
            Reach::Tuck => &MOVES,
        }
    }
}

pub struct Placement {
    inputs: InputSequence,
    features: BottleFeatures,
    landing: Landing,
    settled: Bottle,
}

impl Placement {
    pub fn inputs(&self) -> &InputSequence {
        &self.inputs
    }

    pub fn features(&self) -> BottleFeatures {
        self.features
    }

    /// the bottle this placement leaves behind, cleared and cascaded out
    pub fn settled(&self) -> &Bottle {
        &self.settled
    }

    /// this placement, marked as belonging to the held pill rather than the one in play
    fn of_the_held_pill(mut self) -> Self {
        self.features = self.features.of_the_held_pill();
        self
    }

    /// where the two halves come to rest, the spawn's left hand vitamin first
    pub fn landing(&self) -> Landing {
        self.landing
    }
}

pub trait PlacementSearch {
    /// every placement the pill in play can reach, tucks included
    fn placements(&self, stats_before: BottleStats) -> Vec<Placement> {
        self.placements_within(Reach::default(), stats_before)
    }

    fn placements_within(&self, reach: Reach, stats_before: BottleStats) -> Vec<Placement>;

    /// The placements `shape` would have as the pill in play, each marked
    /// [`BottleFeatures::of_the_held_pill`] so a scorer can tell them apart.
    fn placements_of(
        &self,
        reach: Reach,
        shape: PillShape,
        stats_before: BottleStats,
    ) -> Vec<Placement>;
}

impl PlacementSearch for Bottle {
    fn placements_of(
        &self,
        reach: Reach,
        shape: PillShape,
        stats_before: BottleStats,
    ) -> Vec<Placement> {
        let mut swapped = self.clone();
        swapped.hold();
        if swapped.try_spawn(shape).is_none() {
            // it cannot even spawn, so it is no alternative
            return vec![];
        }
        swapped
            .placements_within(reach, stats_before)
            .into_iter()
            .map(Placement::of_the_held_pill)
            .collect()
    }

    fn placements_within(&self, reach: Reach, stats_before: BottleStats) -> Vec<Placement> {
        if self.pill().is_none() {
            return vec![];
        }

        let mut visited: HashSet<Pose> = HashSet::from([pose(self)]);
        let mut queue = VecDeque::from([(self.clone(), InputSequence::default())]);
        // keyed by where the pill rests, so the first route found to each is the shortest
        let mut landings: HashSet<Landing> = HashSet::new();
        let mut placements = vec![];

        while let Some((bottle, inputs)) = queue.pop_front() {
            if landings.insert(landing(&bottle)) {
                placements.push(drop_and_settle(&bottle, &inputs, stats_before));
            }

            for translation in reach.moves().iter().copied() {
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
pub type Landing = [(BottlePoint, VirusColor); 2];

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
        // a rest that moves the pill nowhere is not a move
        Translation::Rest => bottle.hard_drop().is_some_and(|(rows, _)| rows > 0),
        // a swap is decided between two searches, never inside one
        Translation::Hold => false,
    }
}

/// drop the pill, lock it and run the clears and cascades out to a settled bottle, exactly as
/// [crate::game::Game] would
fn drop_and_settle(
    bottle: &Bottle,
    inputs: &InputSequence,
    stats_before: BottleStats,
) -> Placement {
    let landing = landing(bottle);
    let mut bottle = bottle.clone();
    bottle.hard_drop();
    let placed: Vec<BottlePoint> = bottle
        .lock()
        .map(|vitamins| vitamins.map(|v| v.position()).to_vec())
        .unwrap_or_default();

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

    // the settled bottle is read once and then asked everything
    let grid = Grid::of(&bottle);
    let stats = grid.stats();
    let placement = placement_stats(&grid, &placed, patterns_cleared);

    Placement {
        inputs: inputs.with(Translation::HardDrop),
        features: BottleFeatures::new(stats, stats_before, placement),
        landing,
        settled: bottle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ai::features::BottleAnalysis;
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
            .filter(|p| p.features().placement().patterns_cleared() > 0)
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
            assert!(
                placement.settled().pattern().0.is_empty(),
                "a settled bottle still has a match in it"
            );
        }
    }

    /// A pit in column 0 roofed at row 12 beside a clear column 1, reachable only by a tuck.
    fn a_roofed_pit() -> Bottle {
        with_pill(
            PillShape::new(Red, Blue),
            &[(0, BOTTLE_FLOOR - 3, Block::Garbage(Blue))],
        )
    }

    #[test]
    fn a_tuck_reaches_under_an_overhang_and_a_straight_drop_does_not() {
        let bottle = a_roofed_pit();
        let before = bottle.stats();
        let floor_of_the_pit = BottlePoint::new(0, BOTTLE_FLOOR as i32);

        let landed_in_the_pit = |reach: Reach| {
            bottle
                .placements_within(reach, before)
                .into_iter()
                .any(|p| p.landing().iter().any(|(at, _)| *at == floor_of_the_pit))
        };

        assert!(
            landed_in_the_pit(Reach::Tuck),
            "no placement got a half onto the floor of the pit"
        );
        assert!(
            !landed_in_the_pit(Reach::Drop),
            "a straight drop cannot get under the overhang"
        );
    }

    #[test]
    fn a_tuck_is_a_rest_and_then_a_move() {
        let bottle = a_roofed_pit();
        let before = bottle.stats();
        let floor_of_the_pit = BottlePoint::new(0, BOTTLE_FLOOR as i32);

        let tuck = bottle
            .placements_within(Reach::Tuck, before)
            .into_iter()
            .find(|p| p.landing().iter().any(|(at, _)| *at == floor_of_the_pit))
            .expect("no placement got a half onto the floor of the pit");

        let keys = tuck.inputs().translations();
        let rest = keys
            .iter()
            .position(|t| *t == Translation::Rest)
            .expect("a tuck comes to rest first");
        // what follows the rest is what walks it under the overhang, and it still ends in a drop
        assert!(keys[rest + 1..].contains(&Translation::Left));
        assert_eq!(keys.last(), Some(&Translation::HardDrop));
    }

    #[test]
    fn tucking_only_ever_adds_placements() {
        let bottle = a_roofed_pit();
        let before = bottle.stats();
        let dropped: HashSet<Landing> = bottle
            .placements_within(Reach::Drop, before)
            .iter()
            .map(|p| p.landing())
            .collect();
        let tucked: HashSet<Landing> = bottle
            .placements_within(Reach::Tuck, before)
            .iter()
            .map(|p| p.landing())
            .collect();
        assert!(dropped.is_subset(&tucked));
        assert!(tucked.len() > dropped.len());
    }

    #[test]
    fn the_placements_of_the_held_pill_all_say_so() {
        let bottle = with_pill(PillShape::new(Red, Blue), &[]);
        let before = bottle.stats();

        assert!(bottle
            .placements(before)
            .iter()
            .all(|p| !p.features().held()));
        let swapped = bottle.placements_of(Reach::Tuck, PillShape::new(Blue, Blue), before);
        assert!(!swapped.is_empty());
        assert!(swapped.iter().all(|p| p.features().held()));
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
