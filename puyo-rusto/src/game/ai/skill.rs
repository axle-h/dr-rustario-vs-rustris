//! The six players.
//!
//! Dr. Rustario's four difficulties pick between the six rows of weights the N64 game's own ai
//! carries, because that is the one dial the original has. There is no original here to take a
//! table off, so the rows are built rather than found - each one a set of [`Weights`] and a
//! [`SearchConfig`] - and the same rule applies to them as applies over there: **which of them
//! is the better player was measured, not assumed.** [`SKILL_ORDER`] is the answer, and
//! `ga puyo rank` is what produced it.
//!
//! What separates a row from the one above it is not how fast it presses keys. It is how far
//! it can see (how much of the queue it reads and how far past it), how many boards it can
//! hold in mind at once, and - the part that changes the *game* rather than the strength -
//! how big a chain it will hold out for before firing.

use crate::game::ai::beam::SearchConfig;
use crate::game::ai::eval::Weights;
use crate::game::score::TARGET_POINTS;

/// a chain worth this many nuisance puyos, in the game's own points
const fn nuisance(count: u32) -> u32 {
    count * TARGET_POINTS
}

pub struct Skill {
    pub name: &'static str,
    pub weights: Weights,
    pub search: SearchConfig,
}

pub const SKILLS: usize = 6;

/// The six rows, in no particular order - see [`SKILL_ORDER`] for the one that is measured.
pub const ROWS: [Skill; SKILLS] = [
    // Four in a row pops, and nothing else has occurred to it. It reads the board only far
    // enough not to stack itself into a wall, and it takes every clear it can see the moment
    // it can see it. This is Puyo VS's own cpu with the random half taken out.
    Skill {
        name: "greedy",
        weights: Weights::GREEDY,
        search: SearchConfig {
            width: 1,
            queue_depth: 0,
            lookahead: 0,
            queues: 1,
            trigger: 0,
        },
    },
    // Has noticed that a chain is worth more than a clear, and will hold out for a small one.
    Skill {
        name: "tidy",
        weights: Weights::FREESTYLE,
        search: SearchConfig {
            width: 6,
            queue_depth: 1,
            lookahead: 0,
            queues: 1,
            trigger: nuisance(6),
        },
    },
    // Plays for a fast second chain rather than a big first one: it holds much less back, and
    // cares much less where it builds.
    Skill {
        name: "swift",
        weights: Weights::FAST,
        search: SearchConfig {
            width: 12,
            queue_depth: 2,
            lookahead: 0,
            queues: 1,
            trigger: nuisance(12),
        },
    },
    // Builds properly, and holds what it builds until it is worth a good half rock.
    Skill {
        name: "builder",
        weights: Weights::BUILD,
        search: SearchConfig {
            width: 16,
            queue_depth: 2,
            lookahead: 1,
            queues: 1,
            trigger: nuisance(18),
        },
    },
    // The same player, given room to think and the patience to wait for a whole rock.
    Skill {
        name: "patient",
        weights: Weights::BUILD,
        search: SearchConfig {
            width: 20,
            queue_depth: 2,
            lookahead: 2,
            queues: 1,
            trigger: nuisance(30),
        },
    },
    // And again, holding out for a chain that decides a match rather than a turn of one.
    Skill {
        name: "sharp",
        weights: Weights::BUILD,
        search: SearchConfig {
            width: 16,
            queue_depth: 2,
            lookahead: 2,
            queues: 2,
            trigger: nuisance(48),
        },
    },
];

/// The rows worst to best, **as measured**.
///
/// `ga puyo rank 12 600` on 2026-08-28, on a Ryzen 9 7900X: every row played the same twelve
/// seeds, six hundred pairs each, on `normal`, and was ranked on the score it banked. None of
/// them was buried - a solo marathon takes no nuisance, so the measure is what a row *builds*
/// rather than how long it lasts, and a two player ladder would want measuring against each
/// other as well.
///
/// | row | score/pair | best chain | nuisance sent | steps | ms/pair | ms/step |
/// |--|--|--|--|--|--|--|
/// | greedy  |  48.4 |  4 |  5,126 |  1 |  0.02 | 0.02 |
/// | tidy    | 191.1 |  7 | 19,706 |  1 |  0.53 | 0.53 |
/// | swift   | 284.4 |  8 | 29,550 |  4 |  1.97 | 0.49 |
/// | builder | 433.0 | 10 | 44,567 |  6 |  4.71 | 0.79 |
/// | patient | 571.9 | 12 | 58,822 | 12 |  8.38 | 0.70 |
/// | sharp   | 761.8 | 12 | 78,351 | 12 | 10.56 | 0.88 |
///
/// It came out in the order the rows happen to be written in, which is luck rather than
/// design - the first run of it had `patient` and `builder` within four percent of each other
/// for twice the search, which is what made the three of them that share the `BUILD` weights
/// differ in how long they hold a chain rather than only in how hard they think.
///
/// **The column to read on a slow device is `ms/step`, not `ms/pair`.** A search is stepped
/// once a frame, so what a frame costs is the whole think divided by
/// [`SearchConfig::steps`](crate::game::ai::beam::SearchConfig::steps) - under a millisecond
/// for every row here, where the pair each one takes as a whole is up to ten. `ga puyo rank`
/// prints all three columns and runs on the handheld build, so measuring this on the device
/// is one command.
///
/// The four difficulties pick out of this, so a harder setting is a better player as well as
/// a faster one. Re-run the ranking after touching a row, the weights or the evaluation - it
/// is the only thing that makes this list true.
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

    /// the ranking has to name every row exactly once, or a difficulty picks the same player
    /// twice and another one is never played at all
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
