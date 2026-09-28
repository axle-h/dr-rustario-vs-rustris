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

/// The class of the biggest break; must be 3, the only class the particle field's big-clear
/// silhouette fires on.
const LONG_CHAIN_CLASS: u16 = 3;

impl GameRender for Game {
    fn name(&self) -> &'static str {
        "Super Rustle Fighter"
    }

    /// counter gems drop in from the top rather than appearing
    fn attack_fall(&self) -> Option<NuisanceFall> {
        Some(rules::COUNTER_FALL)
    }

    /// Each chain pass is graded by its depth, and a pass of at least `BIG_BREAK_GEMS` gems
    /// is graded with the long chains whatever its depth.
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

    /// every chain pass pops its depth, and a Tech Bonus pops its own word instead
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

    /// an emptied board, then a long chain
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
