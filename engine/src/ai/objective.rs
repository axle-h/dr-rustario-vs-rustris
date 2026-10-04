use crate::ai::end_game::EndGame;
use crate::ai::game_result::GameResult;
use crate::ai::mutation::RateLimits;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Objective {
    /// a game that is not over beats any game that is, then higher score wins
    Survival,
    /// maximise the bonus counter within a fixed piece budget, then higher score wins
    Score,
    /// Clear as much of the game as possible from the first board. The phase's piece budget stops
    /// a game, so dawdling costs the boards never reached rather than a speed term.
    Progress,
    /// the game's own [`GameResult::merit`], for a fitness weighing several counters. It has no
    /// finish line, so a phase runs to its `max_generations` or is stopped by hand.
    Merit,
}

impl Display for Objective {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Objective::Survival => write!(f, "survival"),
            Objective::Score => write!(f, "score"),
            Objective::Progress => write!(f, "progress"),
            Objective::Merit => write!(f, "merit"),
        }
    }
}

impl Objective {
    /// parent selection weight, >= 0
    pub fn fitness(&self, result: &GameResult) -> f64 {
        match self {
            Objective::Survival => result.score() as f64,
            Objective::Score => result.bonus() as f64,
            Objective::Progress => result.cleared() as f64,
            // a candidate below nothing is not bred from
            Objective::Merit => result.merit().max(0.0),
        }
    }

    /// `Greater` means `a` is the better result
    pub fn cmp(&self, a: &GameResult, b: &GameResult) -> Ordering {
        match self {
            Objective::Survival => b
                .game_over()
                .cmp(&a.game_over())
                .then_with(|| a.score().cmp(&b.score())),
            Objective::Score => a
                .bonus()
                .cmp(&b.bonus())
                .then_with(|| a.score().cmp(&b.score())),
            // further wins, then more boards finished on the way
            Objective::Progress => a
                .cleared()
                .cmp(&b.cleared())
                .then_with(|| a.bonus().cmp(&b.bonus()))
                .then_with(|| a.score().cmp(&b.score())),
            Objective::Merit => a
                .merit()
                .total_cmp(&b.merit())
                .then_with(|| a.cleared().cmp(&b.cleared())),
        }
    }
}

/// A phase of training: the objective and how games are evaluated and genomes mutated.
#[derive(Clone, Debug)]
pub struct Phase {
    pub objective: Objective,
    pub end_game: EndGame,
    pub seeds_per_game: usize,
    pub mutation_rate: RateLimits,
    pub crossover_rate: RateLimits,
    /// magnitude of a coefficient nudge when a gene mutates
    pub mutation_step: f64,
    pub max_generations: usize,
    /// Empty plays every member the whole block. Otherwise each rung in turn plays on the best of
    /// what the last rung played, so the games go where selection decides something.
    pub racing: Vec<Rung>,
}

/// One step of racing a generation: the best `share` of the population, by the games each has
/// played so far, plays on to `games` of the block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rung {
    pub games: usize,
    pub share: f64,
}

impl Phase {
    /// train from scratch until a member survives `clear_cap` of the game's progress counter
    pub fn survival(clear_cap: u32) -> Self {
        Self {
            objective: Objective::Survival,
            end_game: EndGame::of_cleared(clear_cap),
            seeds_per_game: 1,
            mutation_rate: RateLimits::new(0.1..=0.20),
            crossover_rate: RateLimits::new(0.1..=0.20),
            mutation_step: 0.1,
            max_generations: usize::MAX,
            racing: vec![],
        }
    }

    /// fine-tune an already surviving model for bonus play within `piece_cap` pieces
    pub fn score(piece_cap: u32) -> Self {
        Self {
            objective: Objective::Score,
            end_game: EndGame::of_pieces(piece_cap),
            seeds_per_game: 3,
            mutation_rate: RateLimits::new(0.01..=0.05),
            crossover_rate: RateLimits::new(0.01..=0.05),
            mutation_step: 0.02,
            max_generations: usize::MAX,
            racing: vec![],
        }
    }

    pub fn with_max_generations(mut self, max_generations: usize) -> Self {
        self.max_generations = max_generations;
        self
    }

    /// Race each generation up `rungs`: the first plays everyone, so its share is ignored. Games
    /// climb, shares fall, and the last rung is the whole block.
    pub fn with_racing(mut self, rungs: Vec<Rung>) -> Self {
        assert!(
            rungs
                .windows(2)
                .all(|pair| pair[0].games < pair[1].games && pair[0].share >= pair[1].share),
            "rungs must play more games among fewer members"
        );
        assert!(
            rungs
                .iter()
                .all(|rung| rung.share > 0.0 && rung.share <= 1.0),
            "a rung's share is a fraction of the population"
        );
        if let Some(last) = rungs.last() {
            self.seeds_per_game = last.games;
        }
        self.racing = rungs;
        self
    }

    pub fn is_complete(&self, best: &GameResult) -> bool {
        match self.objective {
            Objective::Survival => !best.game_over() && self.end_game.reached(*best),
            // a progress phase ends only when every board is cleared on every seed without being
            // buried; otherwise it runs for `max_generations`
            Objective::Progress => !best.game_over() && best.cleared() >= self.end_game.cleared,
            Objective::Score | Objective::Merit => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn result(score: u32, cleared: u32, game_over: bool, bonus: u32) -> GameResult {
        GameResult::new(score, cleared, 0, game_over, Duration::ZERO).with_pieces(0, bonus)
    }

    #[test]
    fn survival_prefers_not_losing_over_score() {
        let alive = result(10, 10, false, 0);
        let dead = result(1000, 100, true, 40);
        assert_eq!(Objective::Survival.cmp(&alive, &dead), Ordering::Greater);
        assert_eq!(Objective::Survival.cmp(&dead, &alive), Ordering::Less);
        assert_eq!(
            Objective::Survival.cmp(&alive, &result(20, 10, false, 0)),
            Ordering::Less
        );
    }

    #[test]
    fn score_prefers_the_bonus_counter_regardless_of_game_over() {
        let timid = result(5000, 100, false, 0);
        let aggressive = result(100, 8, true, 8);
        assert_eq!(Objective::Score.cmp(&aggressive, &timid), Ordering::Greater);
        assert_eq!(
            Objective::Score.cmp(&result(100, 8, true, 8), &result(200, 8, false, 8)),
            Ordering::Less
        );
    }

    #[test]
    fn progress_prefers_whoever_got_further() {
        let further = result(0, 60, true, 2).with_pieces(300, 2);
        let stalled = result(0, 20, false, 1).with_pieces(300, 1);
        assert_eq!(
            Objective::Progress.cmp(&further, &stalled),
            Ordering::Greater
        );
        let slow = result(0, 60, true, 2).with_pieces(9000, 2);
        assert_eq!(Objective::Progress.cmp(&slow, &further), Ordering::Equal);
    }

    #[test]
    fn merit_ranks_on_the_game_own_measure_and_never_breeds_from_below_zero() {
        let buried = result(0, 80, true, 0).with_merit(-12.0);
        let standing = result(0, 60, false, 0).with_merit(70.5);
        assert_eq!(Objective::Merit.cmp(&standing, &buried), Ordering::Greater);
        assert_eq!(Objective::Merit.fitness(&buried), 0.0);
        assert_eq!(Objective::Merit.fitness(&standing), 70.5);
    }

    #[test]
    fn a_progress_phase_is_complete_only_when_the_whole_game_has_been_cleared() {
        let mut phase = Phase::survival(u32::MAX);
        phase.objective = Objective::Progress;
        phase.end_game = EndGame::of_cleared(924);

        assert!(phase.is_complete(&result(0, 1166, false, 24)));
        assert!(!phase.is_complete(&result(0, 1800, true, 30)));
        assert!(!phase.is_complete(&result(0, 700, false, 17)));
    }

    #[test]
    fn survival_phase_completes_at_the_clear_cap() {
        let phase = Phase::survival(100);
        assert!(!phase.is_complete(&result(0, 99, false, 0)));
        assert!(!phase.is_complete(&result(0, 100, true, 0)));
        assert!(phase.is_complete(&result(0, 100, false, 0)));
        assert!(!Phase::score(10).is_complete(&result(0, 1000, false, 1000)));
    }
}
