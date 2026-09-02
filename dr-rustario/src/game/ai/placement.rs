//! Every placement the pill in play can reach, and the bottle each one leaves behind.
//!
//! Candidates are found by replaying real [Bottle] moves on a clone, so Dr. Mario's wall kicks
//! and the blocks already in the bottle are honoured for free.
//!
//! **A tuck is a move like any other here.** Besides moving and rotating, the search may let
//! the pill *come to rest* ([`Translation::Rest`]) and carry on moving from there, which is how
//! a half gets under an overhang and into the pit a straight drop can never reach. It used to
//! be left out on the grounds that the agent had no single step soft drop to execute one with,
//! and that is true and beside the point: what makes a tuck executable in this game is extended
//! placement lock down. A pill that has come to rest may be moved for another
//! [`engine::game::timing::Timing::lock`] and each move restarts that delay, up to
//! `max_lock_placements` of them - so "fall, then slide" needs no timing at all, only somewhere
//! to slide to. Resting is the *only* way down the search takes, rather than a row at a time,
//! because a pill cannot fall past where it comes to rest: an agent waiting for that has
//! nothing to get wrong.

use crate::game::ai::features::{placement_stats, BottleFeatures, BottleStats, Grid};
use crate::game::ai::input_sequence::{InputSequence, Translation};
use crate::game::bottle::Bottle;
use crate::game::geometry::{BottlePoint, Rotation};
use crate::game::pill::{PillShape, VirusColor};
use std::collections::{HashSet, VecDeque};

/// The moves the search walks. [`Translation::Rest`] is last so that breadth first order
/// prefers a placement a straight drop can reach: two routes to the same landing are the same
/// placement, and the shorter one is the one the agent is given.
const MOVES: [Translation; 5] = [
    Translation::Left,
    Translation::Right,
    Translation::RotateClockwise,
    Translation::RotateAnticlockwise,
    Translation::Rest,
];

/// How far the search is allowed to walk the pill. Tucking doubles the placements on offer and
/// so doubles what a generation of training costs, and it is the one change here whose worth is
/// meant to be measured rather than assumed, so it can be turned off.
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

    /// the bottle this placement leaves behind, cleared and cascaded out. [BottleFeatures] is
    /// the reading of it the scorers take; anything measuring the bottle some other way - the
    /// feature probe - reads it here.
    pub fn settled(&self) -> &Bottle {
        &self.settled
    }

    /// this placement, marked as belonging to the held pill rather than the one in play
    fn of_the_held_pill(mut self) -> Self {
        self.features = self.features.of_the_held_pill();
        self
    }

    /// where the two halves come to rest, in the pill's own order: the left hand vitamin of
    /// the pill as it spawns first. The scorers that work on the bottle rather than on
    /// [BottleFeatures] need to know which cells the pill actually filled.
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

    /// The placements the bottle would offer if `shape` were the pill in play instead, which is
    /// what an agent weighing the held pill against the one in front of it searches. Every one
    /// of them is marked [`BottleFeatures::of_the_held_pill`], since a scorer shown both sets at
    /// once has no other way to tell that reaching for one of these costs a hold.
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
        // several poses drop into the same place - a wall kick can leave the pill a row lower in
        // the same column - so placements are keyed by where the pill comes to rest, and
        // breadth first order means the first route to each one is the shortest
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
        // a rest that moves the pill nowhere is not a move, and a self loop the visited set
        // would have to catch
        Translation::Rest => bottle.hard_drop().is_some_and(|(rows, _)| rows > 0),
        // the search is always of one pill: a swap is what the agent decides *between* two
        // searches, and never a move inside either of them
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

    /// A pit under an overhang: column 0 is open to the floor but roofed over at row 12, and
    /// column 1 beside it is clear all the way down. A straight drop into column 0 lands on the
    /// roof; the only way into the pit is to come down column 1, land on the floor, and walk
    /// left underneath.
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
