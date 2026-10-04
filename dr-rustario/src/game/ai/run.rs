//! What a training run asks of a candidate: its clock, its fitness and the garbage thrown at it.
//! Kept apart from [`super::headless_game`], which the test build compiles out.

use crate::game::random::viruses_at_level;
use engine::ai::GameResult;
use std::iter::Sum;
use std::ops::Add;

/// The last bottle a training game plays, from bottle 0. It must sit above where the best
/// candidates finish or the fitness saturates and cannot tell them apart.
#[cfg_attr(test, allow(dead_code))]
pub const TOP_TRAINING_LEVEL: u32 = 30;

/// How many pills a report game from the first bottle is given to destroy as many viruses as it
/// can, which is the pace a model is compared on. Dying stops the count and dawdling spends it
/// on fewer bottles, so it must bind for both players.
pub const PILL_BUDGET: u32 = 2500;

/// the bottle a report counts a seed as having proven itself past; the finish line is
/// [`survived_the_budget`]
pub const PROVEN_LEVEL: u32 = 20;

/// The bottles a training game starts from, every one of them in every generation, so a pile-up
/// early on costs one game rather than the whole run.
pub const START_LEVELS: [u32; 5] = [0, 5, 10, 15, 20];

/// Games from each of [`START_LEVELS`] the leaders of a generation play; fewer and selection
/// picks luck, which a well-taught seed loses to mutation. Racing plays the rest a sixth or a
/// half of it, so it divides by six.
#[cfg_attr(test, allow(dead_code))]
pub const SEEDS_PER_LEVEL: usize = 48;

/// How many pills a training game is given. It carries on bottle to bottle; short enough that
/// many of them cost what a few whole runs did, long enough to clear a level 20 bottle.
#[cfg_attr(test, allow(dead_code))]
pub const GAME_PILLS: u32 = 500;

/// What a block of garbage sent is worth against the viruses a player destroys. It is the price
/// `ga dr garbage` measures, an opponent's viruses lost per block, discounted so a player only
/// gives up its own virus for garbage costing an opponent four. Lower it if a run's vitamins over
/// a kill or finishes passed climb.
pub const COMBO_DISCOUNT: f64 = 0.25;

/// viruses the n64 port's strongest row loses per block of garbage it receives; measured by
/// `ga dr garbage`, never tuned by hand
pub const GARBAGE_PRICE: f64 = 0.962;

/// What being buried costs a game, in viruses: a whole level 20 bottle wherever it happens. It
/// must not grow with the bottle reached, or finishing one raises the stakes and stalling pays.
pub const BURIAL_CHARGE: u32 = viruses_at_level(20);

/// What finishing a bottle is worth, in viruses, on top of the ones it took. Versus is a race to
/// clear bottles; without it a pill passing up the last virus costs too little for selection to
/// see through the noise of the deal.
pub const BOTTLE_BONUS: f64 = 20.0;

/// A game's fitness: every virus destroyed and [`BOTTLE_BONUS`] a bottle finished, then the
/// garbage it sent at [`GARBAGE_PRICE`] times [`COMBO_DISCOUNT`]. Buried, it keeps none of its
/// garbage and pays [`BURIAL_CHARGE`] as well, so no combo is worth a risk of dying.
pub fn merit(viruses: u32, bottles: u32, blocks_sent: u32, buried: bool) -> f64 {
    let progress = viruses as f64 + BOTTLE_BONUS * bottles as f64;
    if buried {
        progress - BURIAL_CHARGE as f64
    } else {
        progress + COMBO_DISCOUNT * GARBAGE_PRICE * blocks_sent as f64
    }
}

/// Garbage one game sent and was sent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Traffic {
    /// combos that sent garbage
    pub attacks: u32,
    /// those attacks by size, as [`Incoming::sizes`] counts them
    pub sizes: [u32; ATTACK_SIZES],
    pub blocks_sent: u32,
    pub blocks_received: u32,
}

impl Traffic {
    /// count an attack of `blocks` sent
    pub fn sent(&mut self, blocks: u32) {
        self.attacks += 1;
        self.blocks_sent += blocks;
        let bucket = (blocks.max(2) - 2).min(ATTACK_SIZES as u32 - 1);
        self.sizes[bucket as usize] += 1;
    }
}

impl Add for Traffic {
    type Output = Traffic;

    fn add(self, rhs: Self) -> Self::Output {
        Traffic {
            attacks: self.attacks + rhs.attacks,
            sizes: std::array::from_fn(|i| self.sizes[i] + rhs.sizes[i]),
            blocks_sent: self.blocks_sent + rhs.blocks_sent,
            blocks_received: self.blocks_received + rhs.blocks_received,
        }
    }
}

impl Sum for Traffic {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Traffic::default(), Add::add)
    }
}

/// Garbage dropped on a training game in place of an opponent's, so the network has seen it
/// land before a versus match: a chance each pill of an attack, sized as the n64 port's are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Incoming {
    pub per_pill: f64,
    /// the chance of each size of attack, from two blocks up; the last is that size or more
    pub sizes: [f64; ATTACK_SIZES],
}

/// attack sizes told apart, from a two pattern combo's two blocks up
pub const ATTACK_SIZES: usize = 4;

impl Incoming {
    /// the size of attack a roll in 0..1 lands on
    pub fn size(&self, roll: f64) -> u32 {
        let mut cumulative = 0.0;
        for (i, share) in self.sizes.iter().enumerate() {
            cumulative += share;
            if roll < cumulative {
                return i as u32 + 2;
            }
        }
        self.sizes.len() as u32 + 1
    }
}

/// What an opponent throws, by the level a game started from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fire {
    /// one per [`START_LEVELS`], in order
    pub by_level: [Incoming; START_LEVELS.len()],
}

impl Fire {
    /// what a game started at `level` receives: the nearest of [`START_LEVELS`] at or below it
    pub fn at(&self, level: u32) -> Incoming {
        let index = START_LEVELS
            .iter()
            .rposition(|start| *start <= level)
            .unwrap_or(0);
        self.by_level[index]
    }
}

/// The n64 port's strongest row as an opponent: how often it sends garbage over [`GAME_PILLS`]
/// from each of [`START_LEVELS`], and how big. Measured by `ga dr garbage`.
pub const N64_FIRE: Fire = Fire {
    by_level: [
        Incoming {
            per_pill: 0.0651,
            sizes: [0.954, 0.044, 0.002, 0.000],
        },
        Incoming {
            per_pill: 0.0730,
            sizes: [0.940, 0.058, 0.001, 0.001],
        },
        Incoming {
            per_pill: 0.0751,
            sizes: [0.939, 0.057, 0.004, 0.001],
        },
        Incoming {
            per_pill: 0.0817,
            sizes: [0.941, 0.057, 0.002, 0.000],
        },
        Incoming {
            per_pill: 0.0800,
            sizes: [0.953, 0.044, 0.002, 0.000],
        },
    ],
};

/// Every seed spent the whole [`PILL_BUDGET`] unburied, or never buried inside its game.
pub fn survived_the_budget(results: &[GameResult]) -> bool {
    !results.is_empty() && results.iter().all(|result| !result.game_over())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// a whole game that finished `bottles` of them and was or was not buried doing it
    fn game(bottles: u32, buried: bool) -> GameResult {
        GameResult::new(0, 0, 0, buried, Duration::ZERO).with_pieces(PILL_BUDGET, bottles)
    }

    #[test]
    fn a_candidate_survives_its_budget_only_when_every_seed_does() {
        let alive = game(PROVEN_LEVEL + 1, false);
        assert!(survived_the_budget(&[alive, alive, alive, alive]));
        // one seed buried fails the whole run
        assert!(!survived_the_budget(&[alive, alive, game(28, true), alive]));
        // a candidate cut after its probe seeds was buried on both of them
        assert!(!survived_the_budget(&[game(3, true), game(2, true)]));
        assert!(!survived_the_budget(&[]));
    }

    #[test]
    fn a_buried_game_forfeits_its_garbage_and_pays_the_same_wherever_it_dies() {
        let standing = merit(40, 2, 10, false);
        assert_eq!(
            standing,
            40.0 + 2.0 * BOTTLE_BONUS + COMBO_DISCOUNT * GARBAGE_PRICE * 10.0,
            "bottles and garbage count on top of the viruses"
        );
        assert_eq!(
            merit(40, 2, 10, true),
            40.0 + 2.0 * BOTTLE_BONUS - BURIAL_CHARGE as f64
        );
        assert!(merit(40, 0, 1000, true) < merit(40, 0, 0, false));
    }

    #[test]
    fn a_game_takes_the_fire_of_the_level_it_started_nearest_below() {
        let mut fire = N64_FIRE;
        for (i, incoming) in fire.by_level.iter_mut().enumerate() {
            incoming.per_pill = i as f64;
        }
        assert_eq!(fire.at(0).per_pill, 0.0);
        assert_eq!(fire.at(7).per_pill, 1.0);
        assert_eq!(fire.at(20).per_pill, 4.0);
        assert_eq!(fire.at(30).per_pill, 4.0);
    }

    #[test]
    fn an_attack_is_sized_by_its_share() {
        let incoming = Incoming {
            per_pill: 0.1,
            sizes: [0.5, 0.3, 0.2, 0.0],
        };
        assert_eq!(incoming.size(0.0), 2);
        assert_eq!(incoming.size(0.6), 3);
        assert_eq!(incoming.size(0.9), 4);
        assert_eq!(incoming.size(1.0), 5);
    }

    #[test]
    fn an_attack_sent_is_counted_by_size_with_the_biggest_lumped() {
        let mut traffic = Traffic::default();
        for blocks in [2, 2, 3, 5, 7] {
            traffic.sent(blocks);
        }
        assert_eq!(traffic.attacks, 5);
        assert_eq!(traffic.blocks_sent, 19);
        assert_eq!(traffic.sizes, [2, 1, 0, 2]);
    }
}
