use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use crate::ai::game_result::GameResult;
use crate::ai::end_game::EndGame;
use crate::ai::mutation::RateLimits;

/// what the genetic algorithm is optimising for
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Objective {
    /// do not lose: a game that is not over beats any game that is, then higher score wins
    Survival,
    /// maximise the bonus counter within a fixed piece budget, then higher score wins
    Score,
    /// get as far through the game as you can: play it from the first board to the last and see
    /// how much of it you clear before you are buried. Nothing here rewards speed, so a model
    /// trained on it is free to take as long as it needs over a board.
    Progress,
}

impl Display for Objective {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Objective::Survival => write!(f, "survival"),
            Objective::Score => write!(f, "score"),
            Objective::Progress => write!(f, "progress"),
        }
    }
}

impl Objective {
    /// fitness used for weighting parents during selection (must be >= 0)
    pub fn fitness(&self, result: &GameResult) -> f64 {
        match self {
            Objective::Survival => result.score() as f64,
            Objective::Score => result.bonus() as f64,
            Objective::Progress => result.cleared() as f64,
        }
    }

    /// ordering of two results, `Greater` means `a` is the better result
    pub fn cmp(&self, a: &GameResult, b: &GameResult) -> Ordering {
        match self {
            Objective::Survival => b.game_over().cmp(&a.game_over())
                .then_with(|| a.score().cmp(&b.score())),
            Objective::Score => a.bonus().cmp(&b.bonus())
                .then_with(|| a.score().cmp(&b.score())),
            // whoever got further wins, then whoever finished more boards on the way
            Objective::Progress => a.cleared().cmp(&b.cleared())
                .then_with(|| a.bonus().cmp(&b.bonus()))
                .then_with(|| a.score().cmp(&b.score())),
        }
    }
}

/// a phase of training: the objective plus everything about how games are evaluated and genomes mutated
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
}

impl Phase {
    /// train from scratch until a member survives `clear_cap` of the game's progress counter
    pub fn survival(clear_cap: u32) -> Self {
        Self {
            objective: Objective::Survival,
            end_game: EndGame::of_cleared(clear_cap),
            seeds_per_game: 1,
            mutation_rate: RateLimits::new(0.1 ..= 0.20),
            crossover_rate: RateLimits::new(0.1 ..= 0.20),
            mutation_step: 0.1,
            max_generations: usize::MAX,
        }
    }

    /// gently fine-tune an already surviving model for bonus play within `piece_cap` pieces
    pub fn score(piece_cap: u32) -> Self {
        Self {
            objective: Objective::Score,
            end_game: EndGame::of_pieces(piece_cap),
            seeds_per_game: 3,
            mutation_rate: RateLimits::new(0.01 ..= 0.05),
            crossover_rate: RateLimits::new(0.01 ..= 0.05),
            mutation_step: 0.02,
            max_generations: usize::MAX,
        }
    }

    pub fn with_max_generations(mut self, max_generations: usize) -> Self {
        self.max_generations = max_generations;
        self
    }

    /// the survival phase is complete once a member has reached the line cap without losing
    pub fn is_complete(&self, best: &GameResult) -> bool {
        match self.objective {
            // survived all the way to whichever cap the phase set
            Objective::Survival => !best.game_over() && self.end_game.reached(*best),
            // cleared every board it was given, on every seed it played
            Objective::Progress => !best.game_over() && self.end_game.reached(*best),
            Objective::Score => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use super::*;

    fn result(score: u32, cleared: u32, game_over: bool, bonus: u32) -> GameResult {
        GameResult::new(score, cleared, 0, game_over, Duration::ZERO).with_pieces(0, bonus)
    }

    #[test]
    fn survival_prefers_not_losing_over_score() {
        let alive = result(10, 10, false, 0);
        let dead = result(1000, 100, true, 40);
        assert_eq!(Objective::Survival.cmp(&alive, &dead), Ordering::Greater);
        assert_eq!(Objective::Survival.cmp(&dead, &alive), Ordering::Less);
        assert_eq!(Objective::Survival.cmp(&alive, &result(20, 10, false, 0)), Ordering::Less);
    }

    #[test]
    fn score_prefers_the_bonus_counter_regardless_of_game_over() {
        let timid = result(5000, 100, false, 0);
        let aggressive = result(100, 8, true, 8);
        assert_eq!(Objective::Score.cmp(&aggressive, &timid), Ordering::Greater);
        // same bonus, break tie on score
        assert_eq!(Objective::Score.cmp(&result(100, 8, true, 8), &result(200, 8, false, 8)), Ordering::Less);
    }

    #[test]
    fn progress_prefers_whoever_got_further() {
        let further = result(0, 60, true, 2).with_pieces(300, 2);
        let stalled = result(0, 20, false, 1).with_pieces(300, 1);
        assert_eq!(Objective::Progress.cmp(&further, &stalled), Ordering::Greater);
        // taking longer over it costs nothing: there is no speed term
        let slow = result(0, 60, true, 2).with_pieces(9000, 2);
        assert_eq!(Objective::Progress.cmp(&slow, &further), Ordering::Equal);
    }

    #[test]
    fn a_progress_phase_is_complete_once_every_board_is_cleared() {
        let mut phase = Phase::survival(u32::MAX);
        phase.objective = Objective::Progress;
        phase.end_game = EndGame::of_cleared(924);

        // still alive but short of the last board
        assert!(!phase.is_complete(&result(0, 900, false, 20)));
        // cleared the lot on every seed it played
        assert!(phase.is_complete(&result(0, 924, false, 21)));
        // cleared the lot on some seeds but was buried on another
        assert!(!phase.is_complete(&result(0, 924, true, 21)));
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
