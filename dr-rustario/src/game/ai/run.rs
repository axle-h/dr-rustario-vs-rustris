//! What a training run asks of a candidate: its clock, its finish line, and when it is not
//! worth playing out. Kept apart from [`super::headless_game`], which the test build compiles out.

use engine::ai::GameResult;

/// The last bottle a training game plays, from bottle 0. It must sit above where the best
/// candidates finish or the fitness saturates and cannot tell them apart.
#[cfg_attr(test, allow(dead_code))]
pub const TOP_TRAINING_LEVEL: u32 = 30;

/// How many pills a training game is given to destroy as many viruses as it can. Dying stops
/// the count and dawdling spends it on fewer bottles, so it must bind for both players.
pub const PILL_BUDGET: u32 = 2500;

/// the bottle a report counts a seed as having proven itself past; the finish line is
/// [`survived_the_budget`]
pub const PROVEN_LEVEL: u32 = 20;

/// seeds a candidate plays before deciding whether the rest are worth playing
pub const PROBE_SEEDS: usize = 2;

/// Viruses the probe seeds must average for the rest to be played. Far below a taught median,
/// so it only cuts candidates dying in the first few bottles.
pub const ABANDON_BELOW: u32 = 200;

/// The finish line: every seed spent the whole [`PILL_BUDGET`] unburied. A population clears it
/// on luck, so [`engine::ai::Fitness::confirm`] asks again on unseen seeds before a run stops.
pub fn survived_the_budget(results: &[GameResult]) -> bool {
    !results.is_empty() && results.iter().all(|result| !result.game_over())
}

/// Whether the probe seeds were poor enough to call the rest off.
pub fn going_nowhere(probe: &[GameResult], seeds_per_game: usize) -> bool {
    let cleared: u32 = probe.iter().map(GameResult::cleared).sum();
    seeds_per_game > PROBE_SEEDS && cleared / (probe.len() as u32) < ABANDON_BELOW
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// a whole game that finished `bottles` of them and was or was not buried doing it
    fn game(bottles: u32, buried: bool) -> GameResult {
        GameResult::new(0, 0, 0, buried, Duration::ZERO).with_pieces(PILL_BUDGET, bottles)
    }

    /// a whole game that destroyed `viruses`, which is all the cut looks at
    fn played(viruses: u32) -> GameResult {
        GameResult::new(0, viruses, 0, true, Duration::ZERO)
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
    fn a_candidate_going_nowhere_is_cut_and_one_worth_playing_is_not() {
        assert!(going_nowhere(&[played(0), played(ABANDON_BELOW - 1)], 4));
        assert!(!going_nowhere(&[played(0), played(ABANDON_BELOW * 2)], 4));
        // with no seeds left to save there is nothing to call off
        assert!(!going_nowhere(&[played(0), played(0)], PROBE_SEEDS));
    }
}
