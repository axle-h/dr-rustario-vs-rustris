//! The agent that plays a bottle: it picks a placement for each pill as it spawns and then
//! presses the keys to reach it, at whatever rate its difficulty allows.

use crate::game::ai::evaluator::Scorer;
use crate::game::ai::features::{BottleAnalysis, BottleFeatures};
use crate::game::ai::input_sequence::Translation;
use crate::game::ai::models::DrNeuralNetwork;
use crate::game::ai::placement::PlacementSearch;
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

    /// Choose between the placements of the pill in play, scoring them all in one call: the
    /// network is shown what separates the candidates in front of it, so the scores mean
    /// something against each other and nothing at all against another pill's.
    ///
    /// Reaching for the held pill is deliberately not on offer. The model learns to rank from
    /// [`crate::game::ai::imitation`], and what it learns from has no hold: the N64 ai scores a
    /// *placement* against the bottle, with no notion of what giving up the pill in play costs,
    /// so asking it which of two pills to play gives an answer that sounds reasonable and is
    /// not. Taught with hold on offer, a model plays 25 viruses over five games and buries
    /// itself in every one; taught without it, the same model plays 3143 and finishes 80
    /// bottles. Hold is worth having back, but only behind something that can judge it.
    fn decide_by_score(&mut self, game: &mut Game, scorer: Scorer) {
        let bottle = game.bottle();
        let placements = bottle.placements(bottle.stats());
        if placements.is_empty() {
            return;
        }

        let features: Vec<BottleFeatures> = placements.iter().map(|p| p.features()).collect();
        let scores = scorer.rank(&features);
        let Some(best) = (0..placements.len()).max_by(|a, b| {
            scores[*a]
                .total_cmp(&scores[*b])
                // a tie goes to the simpler sequence
                .then_with(|| placements[*b].inputs().cmp(placements[*a].inputs()))
        }) else {
            return;
        };
        self.keys.queue(placements[best].inputs().clone());
    }

    pub fn reset(&mut self) {
        self.keys.abandon();
        self.decided = false;
    }
}
