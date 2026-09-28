//! The seeded gem sequence, after `FUN_8012fd78` and `FUN_8012FF2C`. The first [`EARLY_PAIRS`]
//! pairs draw the pivot from [`EARLY_GEM_TABLES`], which hold no crash gems, and demote a child
//! that matches the pivot's colour; after that the pivot reads the whole 64 entry table and the
//! child only its first 32. A pair that would self-destruct is demoted, every 25th pair carries
//! a rainbow, and a colour dealt past [`DROUGHT`] forces its crash gem into the next pivot.
//!
//! The executable only ever deals from table zero; here the table is drawn from the seed and
//! fixed for the match, since `speed_index` may never change what is dealt.

use crate::game::cell::{GemColor, GemPair, Half};
use crate::game::tables::{
    EARLY_CHILD_TABLE, EARLY_GEM_TABLES, EARLY_TABLE_ENTRIES, GEM_TABLES, GEM_TABLES_COUNT,
    RAINBOW_SCHEDULE, SECOND_HALF_ENTRIES, TABLE_ENTRIES,
};
pub use engine::game::random::Seed;
use rand::RngExt;
use rand_chacha::ChaChaRng;
use std::collections::VecDeque;

pub const PEEK_SIZE: usize = 1;

/// how many of a colour may be dealt before the drought rule answers with a crash gem
pub const DROUGHT: u8 = 12;

/// Pairs dealt from the opening tables: `+0xd8` steps once per ten pieces and the deal reads
/// `+0xd8 < 2`.
pub const EARLY_PAIRS: u32 = 20;

/// The crash gem throttle, this game's one deliberate departure from the original's deal.
/// After its opening the original deals a crash gem in most pairs, which leaves nothing to
/// build a power gem with, so a crash gem is demoted to its plain gem until [`crash_gap`] pairs
/// have passed since the last one. The gap depends only on the pair count so players sharing a
/// seed stay in step, and a drought's crash gem waits for the gap rather than skipping it.
pub const CRASH_GAP_START: u32 = 10;
pub const CRASH_GAP_END: u32 = 0;
/// the pair by which the gap has closed to [`CRASH_GAP_END`]
pub const CRASH_GAP_CLOSES_BY: u32 = 200;

/// The gem sequence of one match. It reads no player-local state, so every player of a seed is
/// dealt the same game however far apart they are.
#[derive(Clone, Debug)]
pub struct GameRandom {
    seed: Seed,
    rng: ChaChaRng,
    /// `+0x292`: which weighted table this match deals from
    table: usize,
    /// `+0x106`, pairs dealt, never reset
    dealt: u32,
    /// `+0x10a`, how many rainbow gems have been handed out
    rainbows: usize,
    /// `+0x26c`..`+0x26f`, gems dealt of each colour since that colour last ran dry
    colors_dealt: [u8; GemColor::N],
    /// the drought's answer, waiting for the next pair's first half
    owed: Option<GemColor>,
    /// pairs dealt since a crash gem last survived the throttle
    since_crash: u32,
    queue: VecDeque<GemPair>,
}

impl GameRandom {
    pub fn from_seed(seed: Seed) -> GameRandom {
        let mut rng = seed.rng();
        let table = rng.random_range(0..GEM_TABLES_COUNT);
        GameRandom {
            seed,
            rng,
            table,
            dealt: 0,
            rainbows: 0,
            colors_dealt: [0; GemColor::N],
            owed: None,
            // zero, so the first crash gem waits out the opening gap too
            since_crash: 0,
            queue: VecDeque::new(),
        }
    }

    /// The seed this match was dealt from, for anything that needs randomness of its own; a
    /// draw off the sequence itself would put two players out of step.
    pub fn seed(&self) -> Seed {
        self.seed
    }

    pub fn table(&self) -> usize {
        self.table
    }

    pub fn next_pair(&mut self) -> GemPair {
        self.fill(PEEK_SIZE + 1);
        self.queue.pop_front().expect("the queue was just filled")
    }

    /// the pairs after the one in play, as the NEXT box shows them
    pub fn peek(&mut self) -> Vec<GemPair> {
        self.fill(PEEK_SIZE);
        self.peeked()
    }

    /// The same, without dealing anything: [`engine::game::Game::queue`] has only `&self`, so
    /// [`Self::next_pair`] keeps the buffer one pair ahead.
    pub fn peeked(&self) -> Vec<GemPair> {
        self.queue.iter().take(PEEK_SIZE).copied().collect()
    }

    fn fill(&mut self, want: usize) {
        while self.queue.len() < want {
            let pair = self.deal();
            self.queue.push_back(pair);
        }
    }

    fn deal(&mut self) -> GemPair {
        // What the drought owes waits out the gap, and is tracked by colour because the
        // self-destruct demotion can move it from the pivot to the child.
        let held = self.since_crash < crash_gap(self.dealt);
        let forced = if held { None } else { self.owed.take() };
        let (pivot, child) = if self.dealt < EARLY_PAIRS {
            self.opening(forced)
        } else {
            self.settled(forced)
        };
        self.dealt += 1;
        let child = if RAINBOW_SCHEDULE.get(self.rainbows) == Some(&self.dealt) {
            self.rainbows += 1;
            Half::Rainbow
        } else {
            child
        };

        let (pivot, child) = self.throttled(pivot, child, held, forced);

        // Last, as in `FUN_8012fd78`, so it sees the pair after the rainbow and the drought.
        let pivot = if pivot == child {
            pivot.demoted()
        } else {
            pivot
        };

        for half in [pivot, child] {
            if let Some(color) = color_of(half) {
                let count = &mut self.colors_dealt[color.index()];
                *count += 1;
                if *count > DROUGHT {
                    *count = 0;
                    self.owed = Some(color);
                }
            }
        }
        GemPair::new(pivot, child)
    }

    /// Demotes a crash gem inside the gap, except the drought's `forced` colour; a rainbow is
    /// never touched.
    fn throttled(
        &mut self,
        pivot: Half,
        child: Half,
        held: bool,
        forced: Option<GemColor>,
    ) -> (Half, Half) {
        let crash = |half: Half| matches!(half, Half::Crash(_));
        let hold = |half: Half| match half {
            Half::Crash(color) if held && Some(color) != forced => half.demoted(),
            half => half,
        };
        let (pivot, child) = (hold(pivot), hold(child));

        if crash(pivot) || crash(child) {
            self.since_crash = 0;
        } else {
            self.since_crash += 1;
        }
        (pivot, child)
    }

    /// The opening draw: a pivot that cannot be a crash gem, and a child demoted when it shares
    /// the pivot's colour, crash bit ignored on both sides.
    fn opening(&mut self, forced: Option<GemColor>) -> (Half, Half) {
        let table = &EARLY_GEM_TABLES[self.table];
        let pivot = match forced {
            Some(color) => Half::Crash(color),
            None => half(table[self.rng.random_range(0..EARLY_TABLE_ENTRIES)]),
        };
        let child = half(EARLY_CHILD_TABLE[self.rng.random_range(0..EARLY_TABLE_ENTRIES)]);
        let child = if color_of(pivot) == color_of(child) {
            child.demoted()
        } else {
            child
        };
        (pivot, child)
    }

    /// The draw after the opening: the whole table for the pivot, its first half for the child.
    fn settled(&mut self, forced: Option<GemColor>) -> (Half, Half) {
        let table = &GEM_TABLES[self.table];
        let pivot = match forced {
            Some(color) => Half::Crash(color),
            None => half(table[self.rng.random_range(0..TABLE_ENTRIES)]),
        };
        let child = half(table[self.rng.random_range(0..SECOND_HALF_ENTRIES)]);
        (pivot, child)
    }
}

/// Pairs of clear air the throttle asks for once `dealt` pairs have gone by: [`CRASH_GAP_START`]
/// until [`EARLY_PAIRS`], then falling as the square of the pairs left to [`CRASH_GAP_CLOSES_BY`].
/// Integer so every platform deals the same game.
pub fn crash_gap(dealt: u32) -> u32 {
    let span = CRASH_GAP_START.saturating_sub(CRASH_GAP_END);
    let closes = (CRASH_GAP_CLOSES_BY - EARLY_PAIRS).max(1);
    let left = closes - dealt.saturating_sub(EARLY_PAIRS).min(closes);
    CRASH_GAP_END + span * left * left / (closes * closes)
}

/// a table entry: 1-4 is a plain gem of that colour, 9-12 its crash gem
fn half(entry: u8) -> Half {
    let color = GemColor::from_game_index(entry & 7).expect("a gem table holds colours 1-4");
    if entry & 8 == 0 {
        Half::Plain(color)
    } else {
        Half::Crash(color)
    }
}

fn color_of(half: Half) -> Option<GemColor> {
    match half {
        Half::Plain(color) | Half::Crash(color) => Some(color),
        Half::Rainbow => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random() -> GameRandom {
        GameRandom::from_seed(Seed::from_u64(42))
    }

    /// every weighted table holds only gems, six biased and two flat as the rules doc reports
    #[test]
    fn the_weighted_tables_are_the_shape_they_were_read_as() {
        let mut shapes = vec![];
        for table in GEM_TABLES {
            for entry in table {
                assert!(matches!(entry, 1..=4 | 9..=12), "{entry} is not a gem");
            }
            let crash = table.iter().filter(|e| **e >= 9).count();
            let crash_first_half = table[..SECOND_HALF_ENTRIES]
                .iter()
                .filter(|e| **e >= 9)
                .count();
            let mut plain: Vec<usize> = (1..=4)
                .map(|c| table.iter().filter(|e| **e == c).count())
                .collect();
            plain.sort_unstable();
            assert_eq!(
                (crash, crash_first_half),
                if plain == vec![9, 9, 9, 9] {
                    (28, 12)
                } else {
                    (26, 11)
                },
                "40.6% of the first half's draw and 34.4% of the second's"
            );
            shapes.push(plain);
        }
        let biased = shapes.iter().filter(|s| **s == vec![7, 7, 12, 12]).count();
        let flat = shapes.iter().filter(|s| **s == vec![9, 9, 9, 9]).count();
        assert_eq!((biased, flat), (6, 2), "six biased tables and two flat");
    }

    /// the opening pivot tables hold no crash gems; only the child's table does
    #[test]
    fn the_opening_tables_hold_no_crash_gems() {
        for table in EARLY_GEM_TABLES {
            for entry in table {
                assert!(matches!(entry, 1..=4), "{entry} is not a plain gem");
            }
        }
        let crash = EARLY_CHILD_TABLE.iter().filter(|e| **e >= 9).count();
        assert_eq!(crash, 12, "37.5% of the child's draw, before the demotion");
    }

    /// the crash gem rate over 200 seeds is low in the opening and rises in every window after
    #[test]
    fn crash_gems_open_in_long_gaps_and_close_up() {
        let windows = [(0, 20), (20, 60), (60, 120), (120, 200), (200, 400)];
        let rates: Vec<f64> = windows
            .iter()
            .map(|(lo, hi)| {
                let (mut crash, mut pairs) = (0.0, 0.0);
                for seed in 0..200u64 {
                    let mut random = GameRandom::from_seed(Seed::from_u64(seed));
                    for n in 0..*hi {
                        let pair = random.next_pair();
                        if n >= *lo {
                            pairs += 1.0;
                            crash += [pair.pivot, pair.child]
                                .iter()
                                .filter(|h| matches!(h, Half::Crash(_)))
                                .count() as f64;
                        }
                    }
                }
                crash / pairs
            })
            .collect();
        assert!(
            rates[0] < 0.1,
            "the opening is one crash gem in ten pairs or better, got {}",
            rates[0]
        );
        for pair in rates.windows(2) {
            assert!(
                pair[1] > pair[0],
                "and every window after it is busier: {rates:?}"
            );
        }
        assert!(
            rates[3] > 0.5,
            "and it is most of the way there before it closes, got {}",
            rates[3]
        );
        assert!(
            rates[4] > 0.7,
            "the last one is the executable's own 0.79, got {}",
            rates[4]
        );
    }

    /// every crash gem, the drought's included, is at least [`crash_gap`] pairs after the last
    #[test]
    fn no_crash_gem_falls_inside_the_gap() {
        for seed in 0..50u64 {
            let mut random = GameRandom::from_seed(Seed::from_u64(seed));
            let mut last: Option<u32> = None;
            for dealt in 0..400u32 {
                let pair = random.next_pair();
                if !([pair.pivot, pair.child])
                    .iter()
                    .any(|half| matches!(half, Half::Crash(_)))
                {
                    continue;
                }
                if let Some(last) = last {
                    assert!(
                        dealt - last >= crash_gap(dealt),
                        "{pair:?} at pair {dealt} is {} pairs after the last crash gem, \
                         inside a gap of {}",
                        dealt - last,
                        crash_gap(dealt)
                    );
                }
                last = Some(dealt);
            }
        }
    }

    /// the same seed deals the same game
    #[test]
    fn one_seed_deals_one_sequence() {
        let deal = |mut r: GameRandom| (0..50).map(|_| r.next_pair()).collect::<Vec<_>>();
        assert_eq!(deal(random()), deal(random()));
        assert_ne!(
            deal(random()),
            deal(GameRandom::from_seed(Seed::from_u64(43)))
        );
    }

    /// peeking does not consume, and what was peeked is what arrives
    #[test]
    fn the_next_box_shows_the_pair_that_comes_next() {
        let mut random = random();
        random.next_pair();
        let peeked = random.peek();
        assert_eq!(peeked.len(), PEEK_SIZE);
        assert_eq!(random.peek(), peeked, "and peeking again shows the same");
        assert_eq!(random.next_pair(), peeked[0]);
    }

    /// no pair of 5000 is two crash gems of one colour
    #[test]
    fn a_pair_is_never_two_crash_gems_of_one_colour() {
        let mut random = random();
        for _ in 0..5000 {
            let pair = random.next_pair();
            assert!(
                !matches!((pair.pivot, pair.child), (Half::Crash(a), Half::Crash(b)) if a == b),
                "{pair:?} would break on landing"
            );
        }
    }

    /// every 25th pair's child is a rainbow, and no pivot ever is
    #[test]
    fn every_twenty_fifth_pair_carries_the_rainbow() {
        let mut random = random();
        let mut rainbows = vec![];
        for n in 1..=100u32 {
            let pair = random.next_pair();
            assert_ne!(pair.pivot, Half::Rainbow, "never the first half");
            if pair.child == Half::Rainbow {
                rainbows.push(n);
            }
        }
        assert_eq!(rainbows, vec![25, 50, 75, 100]);
    }

    /// a colour dealt past [`DROUGHT`] is answered with its crash gem once the gap lets go
    #[test]
    fn a_flood_of_one_colour_is_answered_with_its_crash_gem() {
        let mut random = random();
        let mut counts = [0u8; GemColor::N];
        let mut owed: Option<GemColor> = None;
        let mut since_crash = 0;
        let (mut answered, mut waited, mut longest) = (0, 0, 0);
        for dealt in 0..2000u32 {
            let held = since_crash < crash_gap(dealt);
            let paid = if held { None } else { owed.take() };
            let pair = random.next_pair();
            if let Some(color) = paid {
                answered += 1;
                longest = waited.max(longest);
                assert!(
                    pair.pivot == Half::Crash(color)
                        || (pair.pivot == Half::Plain(color) && pair.child == Half::Crash(color)),
                    "{color:?} was owed a crash gem and got {pair:?}"
                );
            } else if owed.is_some() {
                waited += 1;
            }
            since_crash = match [pair.pivot, pair.child]
                .iter()
                .any(|half| matches!(half, Half::Crash(_)))
            {
                true => 0,
                false => since_crash + 1,
            };
            for half in [pair.pivot, pair.child] {
                if let Some(color) = color_of(half) {
                    let count = &mut counts[color.index()];
                    *count += 1;
                    if *count > DROUGHT {
                        *count = 0;
                        owed = Some(color);
                        waited = 0;
                    }
                }
            }
        }
        assert!(answered > 50, "and the rule fired often, not once");
        assert!(
            longest <= CRASH_GAP_START + 1,
            "an owed crash gem waits out the gap and no longer, and one waited {longest}"
        );
    }
}
