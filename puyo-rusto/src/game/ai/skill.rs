//! The six ai players, each a set of [`Weights`] and a [`SearchConfig`]. Rows differ in how far
//! they see, how many boards they hold, how big a chain they wait for and how deep a tray they
//! answer, not in key speed.

use crate::game::ai::beam::SearchConfig;
use crate::game::ai::eval::Weights;
use crate::game::score::TARGET_POINTS;

/// A chain worth this many nuisance puyos, in points: the unit of [`SearchConfig::trigger`] and
/// of [`Candidate::fires`](crate::game::ai::beam::Candidate::fires).
pub(crate) const fn nuisance(count: u32) -> u32 {
    count * TARGET_POINTS
}

pub struct Skill {
    pub name: &'static str,
    pub weights: Weights,
    pub search: SearchConfig,
}

pub const SKILLS: usize = 6;

/// The six rows, unordered; [`SKILL_ORDER`] ranks them.
pub const ROWS: [Skill; SKILLS] = [
    // Puyo VS's own cpu without its randomness: takes every clear the moment it sees it.
    Skill {
        name: "greedy",
        weights: Weights::GREEDY,
        search: SearchConfig {
            width: 1,
            queue_depth: 0,
            lookahead: 0,
            queues: 1,
            trigger: 0,
            answer_at: u32::MAX,
        },
    },
    // holds out for a small chain, never answers the tray
    Skill {
        name: "tidy",
        weights: Weights::FREESTYLE,
        search: SearchConfig {
            width: 6,
            queue_depth: 1,
            lookahead: 0,
            queues: 1,
            trigger: nuisance(6),
            answer_at: u32::MAX,
        },
    },
    // plays for tempo, answers only two rocks
    Skill {
        name: "swift",
        weights: Weights::FAST,
        search: SearchConfig {
            width: 12,
            queue_depth: 2,
            lookahead: 0,
            queues: 1,
            trigger: nuisance(12),
            answer_at: 60,
        },
    },
    // holds for a half rock, answers a rock and a row
    Skill {
        name: "builder",
        weights: Weights::BUILD,
        search: SearchConfig {
            width: 16,
            queue_depth: 2,
            lookahead: 1,
            queues: 1,
            trigger: nuisance(18),
            answer_at: 36,
        },
    },
    // holds for a whole rock, answers one rock, the most that can land at once
    Skill {
        name: "patient",
        weights: Weights::BUILD,
        search: SearchConfig {
            width: 20,
            queue_depth: 2,
            lookahead: 2,
            queues: 1,
            trigger: nuisance(30),
            answer_at: 30,
        },
    },
    // holds for a match-deciding chain, and breaks off building it to answer a rock
    Skill {
        name: "sharp",
        weights: Weights::BUILD,
        search: SearchConfig {
            width: 16,
            queue_depth: 2,
            lookahead: 2,
            queues: 2,
            trigger: nuisance(48),
            answer_at: 30,
        },
    },
];

/// The rows worst to best in a solo marathon, as ranked by `ga puyo rank`; re-run it after
/// changing a row, the weights or the evaluation. The ladder under fire (`ga puyo duel`) is not
/// the marathon's and roughly reverses at the top, and difficulties follow the marathon's.
pub const SKILL_ORDER: [usize; SKILLS] = [0, 1, 2, 3, 4, 5];

/// the `nth` weakest row, which is what a difficulty asks for
pub fn nth_weakest(nth: usize) -> &'static Skill {
    &ROWS[SKILL_ORDER[nth.min(SKILLS - 1)]]
}

pub fn by_name(name: &str) -> Option<usize> {
    ROWS.iter().position(|row| row.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the ranking names every row exactly once
    #[test]
    fn the_ranking_is_a_permutation_of_the_rows() {
        let mut sorted = SKILL_ORDER;
        sorted.sort();
        assert_eq!(sorted, std::array::from_fn::<usize, SKILLS, _>(|i| i));
    }

    #[test]
    fn every_row_is_named_once() {
        for (index, row) in ROWS.iter().enumerate() {
            assert_eq!(by_name(row.name), Some(index));
        }
    }
}
