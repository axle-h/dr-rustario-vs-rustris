//! What a training run asks of a candidate: where the finish line is, and when one is not worth
//! playing out. It is here rather than in [`super::headless_game`] because it is arithmetic over
//! results and nothing else - the fixture that plays the games is compiled out of the crate's
//! own test build, which swaps the real [`crate::game::Game`] for a mock.

use engine::ai::GameResult;

/// The last bottle a training game plays: a candidate starts on bottle 0 and works up.
///
/// It is past the twenty a run used to stop at because stopping there put a ceiling on the
/// measure that the best member of a population reached in its first generation and then sat
/// on: over a run of 641 generations the best member scored the exact maximum in 518 of them,
/// so for nine hours selection at the top was between candidates the fitness could not tell
/// apart. The bottles above it are not more of the same, either - level 19 and up confine their
/// viruses to the top three rows, and level 24 up carries the game's maximum of 99 - so the
/// measure keeps discriminating all the way here.
pub const TOP_TRAINING_LEVEL: u32 = 30;

/// The bottle every other seed has to clear for a run to be finished. One seed has to come out
/// of [`TOP_TRAINING_LEVEL`] and the rest only have to get this far, because clearing a whole
/// game on one seed is as much luck as skill: asking every seed for it is a lottery, and one
/// that a whole night of training lost 516 times out of 516.
pub const PROVEN_LEVEL: u32 = 20;

/// How many seeds a candidate plays before the rest of them are decided to be worth the machine
/// time.
pub const PROBE_SEEDS: usize = 2;

/// What it has to average over those to be played out. Far below the median member - the median
/// of a taught population clears between 380 and 460 - so this only catches candidates dying in
/// the first few bottles, which are most of a generation's cost and none of its result.
pub const ABANDON_BELOW: u32 = 200;

/// Whether these games, one per seed, add up to a finished run: one of them came out of the last
/// bottle and every other one cleared at least as far as [`PROVEN_LEVEL`]. `bonus` counts the
/// bottles a game finished, so clearing bottle n leaves it at n + 1.
pub fn run_finished(results: &[GameResult], top_level: u32) -> bool {
    results.iter().all(|result| result.bonus() > PROVEN_LEVEL)
        && results.iter().any(|result| result.bonus() > top_level)
}

/// Whether the probe seeds were poor enough to call the rest of them off. There is nothing to
/// call off when the probe is the whole of what a candidate was going to play.
pub fn going_nowhere(probe: &[GameResult], seeds_per_game: usize) -> bool {
    let cleared: u32 = probe.iter().map(GameResult::cleared).sum();
    seeds_per_game > PROBE_SEEDS && cleared / (probe.len() as u32) < ABANDON_BELOW
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// a whole game that finished `bottles` of them, holding nothing else the finish line reads
    fn game(bottles: u32) -> GameResult {
        GameResult::new(0, 0, 0, false, Duration::ZERO).with_pieces(0, bottles)
    }

    /// a whole game that destroyed `viruses`, which is all the cut looks at
    fn played(viruses: u32) -> GameResult {
        GameResult::new(0, viruses, 0, true, Duration::ZERO)
    }

    #[test]
    fn a_run_is_finished_when_one_seed_goes_all_the_way_and_the_rest_are_proven() {
        let all = game(TOP_TRAINING_LEVEL + 1);
        let proven = game(PROVEN_LEVEL + 1);
        assert!(run_finished(
            &[all, proven, proven, proven],
            TOP_TRAINING_LEVEL
        ));
        // every seed proven, but none of them came out of the last bottle
        assert!(!run_finished(
            &[proven, proven, proven, proven],
            TOP_TRAINING_LEVEL
        ));
        // one seed all the way and another a bottle short of proven: the half that stops a
        // model being believed on one lucky game
        assert!(!run_finished(
            &[all, game(PROVEN_LEVEL), proven, proven],
            TOP_TRAINING_LEVEL
        ));
        // a candidate cut after its probe seeds never reached the proven bottle to begin with
        assert!(!run_finished(&[game(3), game(2)], TOP_TRAINING_LEVEL));
    }

    #[test]
    fn a_candidate_going_nowhere_is_cut_and_one_worth_playing_is_not() {
        assert!(going_nowhere(&[played(0), played(ABANDON_BELOW - 1)], 4));
        assert!(!going_nowhere(&[played(0), played(ABANDON_BELOW * 2)], 4));
        // with no seeds left to save there is nothing to call off
        assert!(!going_nowhere(&[played(0), played(0)], PROBE_SEEDS));
    }
}
