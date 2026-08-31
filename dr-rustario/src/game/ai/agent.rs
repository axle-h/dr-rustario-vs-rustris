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
    /// The plan has reached a [`Translation::Rest`] and nothing more is pressed until the pill
    /// stops falling. This is the whole of what executing a tuck takes: the pill cannot fall
    /// past where it comes to rest, so there is no row to hit and nothing to time, and once it
    /// is there extended placement lock down gives the agent another lock delay - restarted by
    /// every move it makes - to walk it sideways under the overhang.
    resting: bool,
    /// Hold has been pressed and the swapped pill has not spawned yet. The bottle has no pill
    /// in it while that is true, which is otherwise the signal to throw the plan away - so this
    /// says the plan is still good and the pill it was made for is on its way.
    swapping: bool,
    /// whether this agent is allowed to reach for the held pill at all
    hold: Hold,
    /// how far the agent's placement search may walk the pill. On by default, and the dial is
    /// here so that what tucking is worth can be measured again rather than remembered.
    reach: Reach,
}

/// Whether an agent weighs the pill it is holding against the one in play.
///
/// This is off by default and that is a measurement, not an oversight. What a model learns to
/// rank from ([`crate::game::ai::imitation`]) has no hold: the deterministic ai scores a
/// *placement* against the bottle, with no notion of what giving up the pill in play costs, so
/// no lesson in the corpus says anything about a swap. Taught with hold on offer, a model
/// played 25 viruses over five games and buried itself in every one; taught without it, the
/// same model played 3143 and finished 80 bottles.
///
/// What was wrong there was the *weights*, not the idea. Gradient descent only moves a weight
/// the corpus gives it a gradient for, and the input that says "this is the other pill" is zero
/// on every lesson - so a taught network came out of stage one holding an arbitrary opinion
/// about swapping, drawn at random and applied with full confidence. Stage one now silences
/// that input outright ([`engine::ai::NeuralNetwork::silence_input`]), which leaves a taught
/// model indifferent to a swap rather than superstitious about one, and leaves the genetic
/// algorithm - which *can* judge a swap, because it plays whole games - free to move it either
/// way. Turning it on doubles what the search costs, which is why it is a dial.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Hold {
    #[default]
    Off,
    On,
}

impl DrAiAgent {
    /// Dr. Mario 64's own deterministic opponent, on its strongest row. Named outright rather
    /// than taken from [`DrAiKind::default`], which is the trained network now.
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
            // a swap is the one time the bottle is empty and the plan is still good: the pill
            // it was made for is the one about to spawn
            if self.swapping {
                return;
            }
            // between pills: the bottle is clearing, cascading or spawning, and anything still
            // queued belonged to a pill that has already locked
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

        // the plan is waiting for the pill to land before the rest of it can be pressed
        if self.resting {
            if !game.bottle().is_collision() {
                return;
            }
            self.resting = false;
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
                Translation::Rest => {
                    if !game.bottle().is_collision() {
                        self.resting = true;
                        return;
                    }
                }
                Translation::Hold => {
                    game.hold();
                    // the swapped pill spawns over the next few frames and the rest of the plan
                    // is for that pill, so nothing more is pressed until it is there
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

    /// Choose with the deterministic ai, which the original never had a hold to offer.
    ///
    /// Pooling both pills' placements into one [`N64Ai::read`] suits *this* scorer far better
    /// than it suits the network. The N64 reads the situation off the bottle and picks its
    /// weight table once per call, and the bottle is the same whichever pill is played - so
    /// every candidate in a pooled call is scored on one table, and its priorities are absolute
    /// rather than centred on the candidates the way [`Scorer`]'s are. Comparing a placement of
    /// one pill against a placement of the other is therefore a comparison the original's own
    /// numbers can carry, which is exactly what it could not do for the network.
    ///
    /// It still does not pay, which is measured: over twelve seeds at the training budget the
    /// strongest row played 10,133 viruses and 235 bottles with a hold against 10,730 and 241
    /// without one. What it buys is safety - burials fell from three to one - and what it costs
    /// is pace. That is the same answer the network gave far more loudly (2,996 against 4,595),
    /// and for the same reason: neither scorer prices what giving up the pill in play *costs*,
    /// so both swap whenever the other pill's best placement scores a shade higher.
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

    /// Choose between the placements in front of the agent, scoring them all in one call: the
    /// network is shown what separates the candidates from each other, so the scores mean
    /// something within the call and nothing at all outside it.
    ///
    /// With [`Hold::On`] the candidates are both pills' - the one in play and the one a swap
    /// would bring in - pooled into a single call, which is the only way a scorer that ranks by
    /// what separates its candidates can compare them at all. Every placement of the swapped
    /// pill is marked, and the input that says so is what the genetic algorithm has to express
    /// what a swap is worth; see [`Hold`] for why it is silenced rather than taught.
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
