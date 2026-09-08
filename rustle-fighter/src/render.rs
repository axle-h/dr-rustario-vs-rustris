//! What the engine's renderer needs to know about a Super Rustle Fighter board.

use crate::game::board::SPAWN;
use crate::game::play::{ClearDetail, BIG_BREAK_GEMS, LONG_CHAIN};
use crate::game::rules;
use crate::game::Game;
use engine::animate::nuisance::NuisanceFall;
use engine::game::geometry::Point;
use engine::game::GameEvent;
use engine::particles::field::reaction::words;
use engine::render::GameRender;

/// The class of the biggest break a theme has a sound for.
///
/// Three in every game and it has to be: the background particle field fires its big-clear
/// silhouette on class 3 and nothing else, so a game grading its best clear any lower would
/// never call one up.
const LONG_CHAIN_CLASS: u16 = 3;

impl GameRender for Game {
    fn name(&self) -> &'static str {
        "Super Rustle Fighter"
    }

    /// Counter gems arrive in a slab and the game waits for them.
    ///
    /// They are the one thing on this board the player did not put there, and a tray that has
    /// been filling while you built is the moment you have been watching for - so they drop in
    /// from over the top rather than appearing.
    fn attack_fall(&self) -> Option<NuisanceFall> {
        Some(rules::COUNTER_FALL)
    }

    /// One pass of a chain at a time, graded by how far into the chain it is - so a chain is
    /// *heard* climbing.
    ///
    /// A pass that takes a lot of gems at once is graded with the long chains whatever its
    /// position: a first pass that takes a whole power gem is a bigger moment than the second
    /// pass of an ordinary two chain.
    fn clear_class(&self, event: &GameEvent) -> u16 {
        match event {
            GameEvent::Clear { count, detail, .. } => {
                let chain = ClearDetail::from(*detail).chain;
                if chain >= LONG_CHAIN || *count >= BIG_BREAK_GEMS {
                    LONG_CHAIN_CLASS
                } else {
                    chain.saturating_sub(1).min(LONG_CHAIN_CLASS as u32 - 1) as u16
                }
            }
            _ => 0,
        }
    }

    /// Every pass of a chain says so over the gems that just went.
    ///
    /// The Tech Bonus gets its own word instead, because ten thousand points for dropping a
    /// rainbow down an empty lane is the one moment in this game that is worth more than the
    /// chain it did not make - and a player who has just spent a rainbow needs telling whether
    /// they spent it well.
    fn clear_popup(&self, event: &GameEvent) -> Option<String> {
        match event {
            GameEvent::Clear { detail, .. } => {
                let detail = ClearDetail::from(*detail);
                if detail.tech_bonus {
                    Some("tech bonus".to_string())
                } else {
                    Some(format!("{} chain", detail.chain.max(1)))
                }
            }
            _ => None,
        }
    }

    /// an emptied board is worth saying so, then a chain long enough to be worth watching
    fn clear_word(&self, event: &GameEvent) -> Option<&'static str> {
        match event {
            GameEvent::Clear { detail, .. } => {
                let detail = ClearDetail::from(*detail);
                if detail.all_clear {
                    Some(words::PERFECT)
                } else if detail.chain >= LONG_CHAIN {
                    Some(words::CHAIN)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn spawn_cells(&self) -> Vec<Point> {
        vec![SPAWN, SPAWN.translate(0, -1)]
    }
}
