//! What the engine's renderer needs to know about a Puyo board.

use crate::game::board::{Board, SPAWN};
use crate::game::{ClearDetail, Game, BIG_CLEAR_PUYOS, LONG_CHAIN};
use engine::animate::nuisance::NuisanceFall;
use engine::game::geometry::Point;
use engine::game::GameEvent;
use engine::particles::field::reaction::words;
use engine::render::GameRender;

/// The class of the biggest clear a theme has a sound for.
///
/// It is 3 in every game and it has to be: the background particle field fires its big-clear
/// silhouette on class 3 and on nothing else, so a game that graded its best clear any lower
/// would simply never call one up. See `TETRIS_CLEAR_CLASS` in `rustris/src/render.rs`.
const LONG_CHAIN_CLASS: u16 = 3;

impl GameRender for Game {
    fn name(&self) -> &'static str {
        "Puyo Rusto"
    }

    /// Nuisance is the one thing on this board the player did not put there, and it arrives in
    /// a slab: it drops in from over the top rather than appearing, and the game waits for it.
    fn attack_fall(&self) -> Option<NuisanceFall> {
        Some(crate::game::rules::NUISANCE_FALL)
    }

    /// One step of a chain at a time, graded by how far into the chain it is - so a chain is
    /// *heard* climbing, which is the sound the game is known for.
    ///
    /// A step that takes a lot of puyos at once is graded with the long chains whatever its
    /// position: a first step that clears two whole groups is a bigger moment than the second
    /// step of an ordinary two chain.
    fn clear_class(&self, event: &GameEvent) -> u16 {
        match event {
            GameEvent::Clear { count, detail, .. } => {
                let chain = ClearDetail::from(*detail).chain;
                if chain >= LONG_CHAIN || *count >= BIG_CLEAR_PUYOS {
                    LONG_CHAIN_CLASS
                } else {
                    chain.saturating_sub(1).min(LONG_CHAIN_CLASS as u32 - 1) as u16
                }
            }
            _ => 0,
        }
    }

    /// Every step of a chain says so over the puyos that just went, from the first.
    ///
    /// This is the game's feedback rather than decoration: what a Puyo player is watching for
    /// is whether the chain they built is still going, and a 1-chain saying "1 chain" is what
    /// makes the second step reading "2 chain" mean anything. Tsu keeps no running chain
    /// counter in the HUD for the same reason - a chain is a thing that happens, not a number.
    fn clear_popup(&self, event: &GameEvent) -> Option<String> {
        match event {
            GameEvent::Clear { detail, .. } => {
                let chain = ClearDetail::from(*detail).chain.max(1);
                Some(format!("{chain} chain"))
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

    /// both halves of the pair, which spawn one above the other in the third column
    fn spawn_cells(&self) -> Vec<Point> {
        let above = Point::new(SPAWN.x, SPAWN.y - 1);
        let mut cells = vec![SPAWN];
        if Board::in_bounds(above) {
            cells.push(above);
        }
        cells
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::random::GameRandom;
    use crate::game::rules::Difficulty;
    use engine::game::random::Seed;

    fn game() -> Game {
        Game::new(
            Difficulty::Normal,
            0,
            GameRandom::from_seed(Seed::from_u64(1), Difficulty::Normal.colors()),
            crate::game::cell::PuyoSkin::FIRST,
        )
    }

    fn clear(chain: u32, count: u32, all_clear: bool) -> GameEvent {
        GameEvent::Clear {
            cells: vec![],
            count,
            is_combo: chain > 1,
            detail: ClearDetail { chain, all_clear }.into(),
        }
    }

    /// nuisance is drawn arriving rather than appearing, which is what a game with a tray of
    /// it wants; the other two take their hits the instant they are sent and say nothing
    #[test]
    fn an_attack_falls_in_rather_than_appearing() {
        assert_eq!(
            game().attack_fall(),
            Some(crate::game::rules::NUISANCE_FALL)
        );
    }

    #[test]
    fn a_chain_is_graded_by_how_far_into_it_a_step_is() {
        let game = game();
        for chain in 1..LONG_CHAIN {
            assert_eq!(game.clear_class(&clear(chain, 4, false)), chain as u16 - 1);
        }
    }

    /// the field's silhouette interrupt fires on class 3 and nothing else
    #[test]
    fn a_long_chain_is_graded_as_the_biggest_clear_there_is() {
        let game = game();
        for chain in LONG_CHAIN..LONG_CHAIN + 4 {
            assert_eq!(game.clear_class(&clear(chain, 4, false)), LONG_CHAIN_CLASS);
        }
    }

    #[test]
    fn a_step_that_takes_a_lot_at_once_is_graded_with_the_long_chains() {
        let game = game();
        assert_eq!(
            game.clear_class(&clear(1, BIG_CLEAR_PUYOS, false)),
            LONG_CHAIN_CLASS
        );
        assert_eq!(game.clear_class(&clear(1, BIG_CLEAR_PUYOS - 1, false)), 0);
    }

    #[test]
    fn only_a_chain_worth_watching_is_spelt_out() {
        let game = game();
        assert_eq!(
            game.clear_word(&clear(LONG_CHAIN, 4, false)),
            Some(words::CHAIN)
        );
        for chain in 1..LONG_CHAIN {
            assert_eq!(game.clear_word(&clear(chain, 4, false)), None, "{chain}");
        }
    }

    #[test]
    fn an_emptied_board_is_spelt_out_ahead_of_the_chain_that_emptied_it() {
        let game = game();
        assert_eq!(
            game.clear_word(&clear(LONG_CHAIN + 2, 8, true)),
            Some(words::PERFECT)
        );
        assert_eq!(game.clear_word(&clear(1, 4, true)), Some(words::PERFECT));
    }

    /// anything that is not a clear grades as nothing and says nothing
    #[test]
    fn nothing_else_is_graded_or_spelt_out() {
        let game = game();
        assert_eq!(game.clear_class(&GameEvent::Move), 0);
        assert_eq!(game.clear_word(&GameEvent::Move), None);
        assert_eq!(game.clear_popup(&GameEvent::Move), None);
    }

    /// every step counts itself out, the first one included - a chain that only announced
    /// itself once it was already going would be telling the player something they knew
    #[test]
    fn every_step_of_a_chain_says_so_over_the_puyos_that_went() {
        let game = game();
        for chain in 1..8 {
            assert_eq!(
                game.clear_popup(&clear(chain, 4, false)),
                Some(format!("{chain} chain"))
            );
        }
    }

    #[test]
    fn the_pair_is_thrown_at_the_column_it_spawns_in() {
        let game = game();
        let cells = game.spawn_cells();
        assert!(cells.contains(&SPAWN));
        assert!(cells.iter().all(|p| p.x == SPAWN.x));
    }
}
