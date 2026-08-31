//! What a training run asks of a candidate: the clock it plays against, where the finish line
//! is, and when one is not worth playing out. It is here rather than in
//! [`super::headless_game`] because it is arithmetic over results and nothing else - the
//! fixture that plays the games is compiled out of the crate's own test build, which swaps the
//! real [`crate::game::Game`] for a mock.

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

/// **The clock.** How many pills a training game is given to destroy as many viruses as it can.
///
/// Training used to run this as a stage of its own, after a survival stage that had no clock at
/// all, and that was wrong twice over. Without a clock, a model that takes nine hundred pills
/// over a bottle scores exactly what one that takes three hundred does, and the survival stage
/// selected for the first: the model it produced plays every bottle from the third upwards
/// around 40% slower than the deterministic ai it learned from (105 pills a bottle against 147,
/// measured over six seeds and bottles 1 to 17). And the stage that was meant to fix that
/// scored on *bottles finished*, which [`GameResult`] averages over the seeds and rounds to a
/// whole number - so two hundred and fifty candidates were ranked on a fitness with about five
/// distinct values, and selection fell through to the tiebreak.
///
/// One number does both jobs: viruses destroyed inside a fixed budget of pills. Dying stops the
/// count, so it still selects for staying alive; dawdling spends the budget on fewer bottles,
/// so it now costs something. It is in the hundreds and never rounded, so a generation is
/// ranked on a measure that can tell its members apart.
///
/// The budget has to *bind* or there is no clock. Two thousand five hundred is where both
/// players live: over the same six seeds the deterministic ai reaches bottle 17 to 21 in it and
/// the embedded model reaches bottle 17 on five of six, so a model that gets no faster cannot
/// climb, and there is a clear four bottles of headroom above where either of them stands.
pub const PILL_BUDGET: u32 = 2500;

/// The bottle every seed is expected to be past before a candidate counts as having survived
/// its budget. Reporting only: the finish line itself is [`survived_the_budget`], since with a
/// clock running "how far did it get" is already what the fitness measures.
pub const PROVEN_LEVEL: u32 = 20;

/// How many seeds a candidate plays before the rest of them are decided to be worth the machine
/// time.
pub const PROBE_SEEDS: usize = 2;

/// What it has to average over those to be played out. Far below the median member - the median
/// of a taught population clears between 380 and 460 - so this only catches candidates dying in
/// the first few bottles, which are most of a generation's cost and none of its result.
pub const ABANDON_BELOW: u32 = 200;

/// Whether these games, one per seed, add up to a candidate that is still standing: it spent
/// the whole of [`PILL_BUDGET`] on every one of them without being buried.
///
/// This is the finish line, and it is inside the fitness rather than bolted on after it, so
/// what training selects for and what ends training are the same thing. It is not, on its own,
/// enough to *stop* a run - best of two hundred and fifty candidates over four seeds clears a
/// bar like this on luck, which is how a run once ended on a model that then reached bottle 19
/// on every seed it had not seen. [`engine::ai::Fitness::confirm`] is the other half: a
/// candidate that trips this is asked the same question again on seeds it has never played.
pub fn survived_the_budget(results: &[GameResult]) -> bool {
    !results.is_empty() && results.iter().all(|result| !result.game_over())
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
        // one seed buried is the whole run: a model believed on three of four is a model
        // believed on the seeds that happened to suit it
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
