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
//! hold in mind at once, and - the parts that change the *game* rather than the strength -
//! how big a chain it will hold out for before firing, and how deep the tray has to get
//! before it will spend one answering what has been thrown at it.

use crate::game::ai::beam::SearchConfig;
use crate::game::ai::eval::Weights;
use crate::game::score::TARGET_POINTS;

/// a chain worth this many nuisance puyos, in the game's own points.
///
/// It is the conversion between the tray - which counts puyos - and
/// [`Candidate::fires`](crate::game::ai::beam::Candidate::fires), which is in points, so it is
/// what [`beam::ranking`](crate::game::ai::beam::ranking) measures an answer against as well
/// as what a [`SearchConfig::trigger`] is written in.
pub(crate) const fn nuisance(count: u32) -> u32 {
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
    // it can see it. This is Puyo VS's own cpu with the random half taken out. It never looks
    // at the tray because it never holds anything back to look at it with.
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
    // Has noticed that a chain is worth more than a clear, and will hold out for a small one.
    // Still fires at almost anything, so what is coming its way never changes its mind.
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
    // Plays for a fast second chain rather than a big first one: it holds much less back, and
    // cares much less where it builds. Notices the tray only when two rocks are hanging over
    // it, by which point it has usually fired anyway.
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
    // Builds properly, and holds what it builds until it is worth a good half rock - unless a
    // rock and a row are queued against it, which it will spend a chain to answer.
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
    // The same player, given room to think and the patience to wait for a whole rock. Answers
    // a single rock, which is the most that can land at once.
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
    // And again, holding out for a chain that decides a match rather than a turn of one - and
    // the one row that will break off a match-deciding chain to answer a rock.
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

/// The rows worst to best, **as measured**.
///
/// `ga puyo rank 12 600` on 2026-08-28, on a Ryzen 9 7900X: every row played the same twelve
/// seeds, six hundred pairs each, on `normal`, and was ranked on the score it banked. None of
/// them was buried - a solo marathon takes no nuisance, so the measure is what a row *builds*
/// rather than how long it lasts. The two player ladder it asked for is `ga puyo duel`, and it
/// is below, and it is not this one.
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
///
/// # The ladder under fire is a different ladder
///
/// `ga puyo duel 12 400 normal` on 2026-09-04 played every row against every other over twelve
/// seeds, each sending the other what its chains buy. **It roughly reverses at the top**, and
/// it says so at both ends of the key-delay dial - at full speed and at the 400 ms a fielded
/// opponent presses keys at:
///
/// | row | wins | losses | nuisance sent |
/// |--|--|--|--|
/// | greedy  |  7 | 53 |   523 |
/// | tidy    | 32 | 28 | 2,179 |
/// | swift   | 54 |  6 | 3,143 |
/// | builder | 36 | 24 | 3,202 |
/// | patient | 27 | 33 | 2,356 |
/// | sharp   | 24 | 36 | 2,873 |
///
/// `swift` wins nine duels in ten and `sharp` - the row the marathon calls best, and the one a
/// `hard` opponent is - loses more than it wins. **A marathon rewards patience and a fight
/// rewards tempo**: holding out for a chain worth forty eight nuisance takes a dozen pairs,
/// and a row throwing twelve every four pairs buries you inside them. It is not a speed
/// artefact; the ordering is the same when both rows are given 400 ms a key and the thinking
/// difference between them is a rounding error.
///
/// **`SKILL_ORDER` is still the marathon's**, deliberately, until someone decides which of the
/// two a difficulty is meant to be. Changing it would make `swift` the hardest opponent and
/// would change what every one of the four dials means, which is a gameplay decision rather
/// than a measurement.
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
