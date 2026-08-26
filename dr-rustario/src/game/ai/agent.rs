//! The agent that plays a bottle: it picks a placement for each pill as it spawns and then
//! presses the keys to reach it, at whatever rate its difficulty allows.

use crate::game::ai::evaluator::Scorer;
use crate::game::ai::features::BottleAnalysis;
use crate::game::ai::input_sequence::Translation;
use crate::game::ai::models::DrNeuralNetwork;
use crate::game::ai::placement::{Placement, PlacementSearch};
use crate::game::ai::{DrAiKind, N64Ai};
use crate::game::Game;
use engine::ai::KeyPacer;
use std::time::Duration;

pub struct DrAiAgent {
    brain: DrAiKind,
    keys: KeyPacer<Translation>,
    /// whether the pill in play has already been given a plan
    decided: bool,
}

impl DrAiAgent {
    /// Dr. Mario 64's own deterministic opponent, which is what every difficulty plays
    pub fn n64() -> Self {
        Self::of(DrAiKind::default())
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
        }
    }

    pub fn with_key_delay(mut self, key_delay: Duration) -> Self {
        self.keys = KeyPacer::new(key_delay);
        self
    }

    /// drive `game` for one frame
    pub fn act(&mut self, game: &mut Game, delta: Duration) {
        self.keys.tick(delta);

        if game.bottle().pill().is_none() {
            // between pills: the bottle is clearing, cascading or spawning, and anything still
            // queued belonged to a pill that has already locked
            self.keys.abandon();
            self.decided = false;
            return;
        }

        if !self.decided {
            self.decide(game);
            self.decided = true;
        }

        // a speed limited agent gets one key here and then has to wait; at full speed the whole
        // sequence goes in this frame
        while let Some(translation) = self.keys.next_key() {
            match translation {
                Translation::Left => game.left(),
                Translation::Right => game.right(),
                Translation::RotateClockwise => game.rotate(true),
                Translation::RotateAnticlockwise => game.rotate(false),
                Translation::HardDrop => game.hard_drop(),
            }
        }
    }

    fn decide(&mut self, game: &mut Game) {
        match self.brain {
            DrAiKind::N64(ai) => {
                let bottle = game.bottle();
                let placements = bottle.placements(bottle.stats());
                // the N64 ai has no hold to reach for: it plays the pill it is given
                if let Some(chosen) = ai.choose(bottle, &placements) {
                    self.keys.queue(placements[chosen].inputs().clone());
                }
            }
            DrAiKind::Neural(network) => self.decide_by_score(game, Scorer::Network(network)),
            DrAiKind::Linear => self.decide_by_score(game, Scorer::Linear),
        }
    }

    fn decide_by_score(&mut self, game: &mut Game, scorer: Scorer) {
        let bottle = game.bottle();
        let before = bottle.stats();
        let placements = bottle.placements(before);
        let Some((score, chosen)) = best(scorer, placements) else {
            return;
        };

        // Dr. Mario is a colour game, so which pill you play matters as much as where: if the
        // one waiting in hold (or next up, when nothing is held) does better, swap for it.
        if game.can_hold() {
            let alternative = game.held_shape().unwrap_or_else(|| game.next_shape());
            let alternative = best(scorer, bottle.placements_of(alternative, before));
            if alternative.is_some_and(|(alt_score, _)| alt_score > score) {
                // the bottle has no pill until the swap spawns, which is what resets the agent
                game.hold();
                return;
            }
        }

        self.keys.queue(chosen.inputs().clone());
    }

    pub fn reset(&mut self) {
        self.keys.abandon();
        self.decided = false;
    }
}

/// the placement this model likes best, scored once each: the network is the expensive part
fn best(scorer: Scorer, placements: Vec<Placement>) -> Option<(f64, Placement)> {
    placements
        .into_iter()
        .map(|placement| (scorer.evaluate(placement.features()), placement))
        .max_by(|(a_score, a), (b_score, b)| {
            a_score
                .total_cmp(b_score)
                // a tie goes to the simpler sequence
                .then_with(|| b.inputs().cmp(a.inputs()))
        })
}
