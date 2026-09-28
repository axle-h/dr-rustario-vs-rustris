//! The agent that plays a bottle: it picks a placement for each pill as it spawns and then
//! presses the keys to reach it, at whatever rate its difficulty allows.

use crate::game::ai::evaluator::Scorer;
use crate::game::ai::features::{BottleAnalysis, BottleFeatures};
use crate::game::ai::input_sequence::Translation;
use crate::game::ai::models::DrNeuralNetwork;
use crate::game::ai::placement::{PlacementSearch, Reach};
use crate::game::ai::{DrAiKind, N64Ai};
use crate::game::Game;
use engine::ai::KeyPacer;
use std::time::Duration;

pub struct DrAiAgent {
    brain: DrAiKind,
    keys: KeyPacer<Translation>,
    /// whether the pill in play has already been given a plan
    decided: bool,
    /// The plan has reached a [`Translation::Rest`] and soft drops until the pill lands, then
    /// uses the lock delay to walk it under the overhang; see [`DrAiAgent::act`].
    resting: bool,
    /// Hold has been pressed and the swapped pill has not spawned, so an empty bottle does not
    /// discard the plan.
    swapping: bool,
    /// whether this agent is allowed to reach for the held pill at all
    hold: Hold,
    /// how far the agent's placement search may walk the pill
    reach: Reach,
}

/// Whether an agent weighs the pill it is holding against the one in play. Off by default:
/// no scorer prices giving up the pill in play, so with hold on they swap whenever the other
/// pill's best placement scores a shade higher, and it doubles the search.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Hold {
    #[default]
    Off,
    On,
}

impl DrAiAgent {
    /// Dr. Mario 64's deterministic opponent on its strongest row, as [`DrAiKind::default`].
    pub fn n64() -> Self {
        Self::of(DrAiKind::N64(N64Ai::new()))
    }

    /// one of the N64 ai's six rows of weights, for comparing them against each other
    pub fn n64_with_skill(skill: u8) -> Self {
        Self::of(DrAiKind::N64(N64Ai::with_skill(skill)))
    }

    pub fn new(network: DrNeuralNetwork) -> Self {
        Self::of(DrAiKind::Neural(network))
    }

    /// the hand written baseline, for comparing a trained model against
    pub fn linear() -> Self {
        Self::of(DrAiKind::Linear)
    }

    pub fn of(brain: DrAiKind) -> Self {
        Self {
            brain,
            keys: KeyPacer::new(Duration::ZERO),
            decided: false,
            resting: false,
            swapping: false,
            hold: Hold::default(),
            reach: Reach::default(),
        }
    }

    /// let this agent weigh the pill it is holding against the one in play
    pub fn with_hold(mut self, hold: Hold) -> Self {
        self.hold = hold;
        self
    }

    /// how far this agent's placement search may walk the pill
    pub fn with_reach(mut self, reach: Reach) -> Self {
        self.reach = reach;
        self
    }

    pub fn with_key_delay(mut self, key_delay: Duration) -> Self {
        self.keys = KeyPacer::new(key_delay);
        self
    }

    /// drive `game` for one frame
    pub fn act(&mut self, game: &mut Game, delta: Duration) {
        self.keys.tick(delta);

        if game.bottle().pill().is_none() {
            if self.swapping {
                return;
            }
            // nothing is falling, so nothing should still be held down
            game.set_soft_drop(false);
            // anything still queued belonged to a pill that has already locked
            self.keys.abandon();
            self.decided = false;
            self.resting = false;
            return;
        }
        self.swapping = false;

        if !self.decided {
            self.decide(game);
            self.decided = true;
        }

        // a tuck waits for the pill to land by soft dropping rather than waiting on gravity
        if self.resting {
            if !game.bottle().is_collision() {
                game.set_soft_drop(true);
                return;
            }
            // Soft drop cuts the lock delay from 500 ms to 150, under every speed limited
            // difficulty's key delay, so it must be released before the tuck or the pill locks.
            game.set_soft_drop(false);
            self.resting = false;
        }

        while let Some(translation) = self.keys.next_key() {
            match translation {
                Translation::Left => game.left(),
                Translation::Right => game.right(),
                Translation::RotateClockwise => game.rotate(true),
                Translation::RotateAnticlockwise => game.rotate(false),
                Translation::HardDrop => game.hard_drop(),
                Translation::Rest => {
                    if !game.bottle().is_collision() {
                        // a waypoint is refunded so the tuck is due the moment the pill lands,
                        // rather than a key delay later spent out of the lock delay
                        self.keys.refund();
                        self.resting = true;
                        return;
                    }
                }
                Translation::Hold => {
                    game.hold();
                    // the rest of the plan is for the swapped pill, so wait for it to spawn
                    self.swapping = true;
                    return;
                }
            }
        }
    }

    fn decide(&mut self, game: &mut Game) {
        match self.brain {
            DrAiKind::N64(ai) => self.decide_by_n64(game, ai),
            DrAiKind::Neural(network) => self.decide_by_score(game, Scorer::Network(network)),
            DrAiKind::Linear => self.decide_by_score(game, Scorer::Linear),
        }
    }

    /// Choose with the deterministic ai. With [`Hold::On`] both pills' placements are pooled
    /// into one call, which the N64's absolute priorities can compare.
    fn decide_by_n64(&mut self, game: &mut Game, ai: N64Ai) {
        let bottle = game.bottle();
        let stats = bottle.stats();
        let mut placements = bottle.placements_within(self.reach, stats);
        let own = placements.len();
        if self.hold == Hold::On {
            if let Some(shape) = game.holdable() {
                placements.extend(bottle.placements_of(self.reach, shape, stats));
            }
        }

        let Some(chosen) = ai.choose(bottle, &placements) else {
            return;
        };
        if chosen >= own {
            self.keys.queue([Translation::Hold]);
        }
        self.keys.queue(placements[chosen].inputs().clone());
    }

    /// Choose by scoring every candidate in one call, since scores are centred on the call.
    /// With [`Hold::On`] the held pill's placements join the call, marked by the held input.
    fn decide_by_score(&mut self, game: &mut Game, scorer: Scorer) {
        let bottle = game.bottle();
        let stats = bottle.stats();
        let mut placements = bottle.placements_within(self.reach, stats);
        let own = placements.len();
        if self.hold == Hold::On {
            if let Some(shape) = game.holdable() {
                placements.extend(bottle.placements_of(self.reach, shape, stats));
            }
        }
        if placements.is_empty() {
            return;
        }

        let features: Vec<BottleFeatures> = placements.iter().map(|p| p.features()).collect();
        let scores = scorer.rank(&features);
        let Some(best) = (0..placements.len()).max_by(|a, b| {
            scores[*a]
                .total_cmp(&scores[*b])
                // a tie goes to the simpler sequence, and to not swapping
                .then_with(|| placements[*b].inputs().cmp(placements[*a].inputs()))
                .then_with(|| (*b >= own).cmp(&(*a >= own)))
        }) else {
            return;
        };

        if best >= own {
            self.keys.queue([Translation::Hold]);
        }
        self.keys.queue(placements[best].inputs().clone());
    }

    pub fn reset(&mut self) {
        self.keys.abandon();
        self.decided = false;
        self.resting = false;
        self.swapping = false;
    }
}
