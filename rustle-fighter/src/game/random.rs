//! The seeded gem sequence, and the rule in it nobody has written down.
//!
//! Each half of a pair is an independent draw from a 64 entry weighted table, and **the two
//! halves do not draw from the same distribution**: the first reads the whole table, the
//! second only its first 32 entries. On top of that sit three rules, all read out of
//! `FUN_8012fd78` and `FUN_8012FF2C`:
//!
//! * **no pair self-destructs.** If both halves come out as the same crash gem the first is
//!   demoted to the plain gem of that colour, so a pair can never break on landing.
//! * **the rainbow is on a schedule**, not a chance: every 25th pair, from a table. The guides
//!   are right and now it is sourced.
//! * **the drought rule**, which is documented nowhere. Every gem dealt is counted by colour,
//!   and when a colour's count passes twelve it resets and the *next* pair's first half is
//!   forced to that colour's crash gem. You are guaranteed an answer to whatever you have been
//!   flooded with.
//!
//! Which of the four tables a match uses is `+0x292`, and what sets it is not read; it is
//! drawn from the seed here and **fixed for the whole match**, which is the compendium's rule
//! that `speed_index` may change how a game feels but never what it deals.

use crate::game::cell::{GemColor, GemPair, Half};
use crate::game::tables::{
    GEM_TABLES, GEM_TABLES_COUNT, RAINBOW_SCHEDULE, SECOND_HALF_ENTRIES, TABLE_ENTRIES,
};
pub use engine::game::random::Seed;
use rand::RngExt;
use rand_chacha::ChaChaRng;
use std::collections::VecDeque;

/// how many upcoming pairs a player is shown - one, the NEXT box
pub const PEEK_SIZE: usize = 1;

/// how many of a colour may be dealt before the drought rule answers with a crash gem
pub const DROUGHT: u8 = 12;

/// The gem sequence of one match.
///
/// Every player is dealt the same game from one seed. Nothing here reads player-local state,
/// so two players who are hundreds of pairs apart in a playlist are still on the same
/// sequence - which is the only way a shared seed means anything.
#[derive(Clone, Debug)]
pub struct GameRandom {
    seed: Seed,
    rng: ChaChaRng,
    /// `+0x292`: which of the four weighted tables this match deals from
    table: usize,
    /// `+0x106`, pairs dealt, never reset
    dealt: u32,
    /// `+0x10a`, how many rainbow gems have been handed out
    rainbows: usize,
    /// `+0x26c`..`+0x26f`, gems dealt of each colour since that colour last ran dry
    colors_dealt: [u8; GemColor::N],
    /// the drought's answer, waiting for the next pair's first half
    owed: Option<GemColor>,
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
            queue: VecDeque::new(),
        }
    }

    /// The seed this match was dealt from, for anything that needs randomness of its own.
    ///
    /// Take it from here rather than from the stream above: a draw off the sequence itself
    /// would put two players out of step by exactly one gem.
    pub fn seed(&self) -> Seed {
        self.seed
    }

    /// which of the four weighted tables this match is dealing from
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

    /// The same, without dealing anything.
    ///
    /// [`engine::game::Game::queue`] has only `&self`, so what the NEXT box draws has to be
    /// already in the buffer - which [`Self::next_pair`] keeps topped up, since it fills for
    /// one more than it takes.
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
        let table = &GEM_TABLES[self.table];
        let pivot = match self.owed.take() {
            Some(color) => Half::Crash(color),
            None => half(table[self.rng.random_range(0..TABLE_ENTRIES)]),
        };
        let child = half(table[self.rng.random_range(0..SECOND_HALF_ENTRIES)]);
        // A pair never self-destructs on landing - and this runs *after* the drought rule
        // has had its say, so a forced crash gem is demoted like any other when the second
        // half happens to match it. The two rules' order is not pinned by the disassembly;
        // this way round is the one that keeps the promise the demotion exists to make.
        let pivot = if pivot == child {
            pivot.demoted()
        } else {
            pivot
        };

        self.dealt += 1;
        let child = if RAINBOW_SCHEDULE.get(self.rainbows) == Some(&self.dealt) {
            self.rainbows += 1;
            Half::Rainbow
        } else {
            child
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

    /// every table holds nothing but the eight gems, and each is the shape the rules doc
    /// reports: two favoured colours at twelve entries, two at seven, and 26 crash gems
    #[test]
    fn the_weighted_tables_are_the_shape_they_were_read_as() {
        for table in GEM_TABLES {
            for entry in table {
                assert!(matches!(entry, 1..=4 | 9..=12), "{entry} is not a gem");
            }
            let crash = table.iter().filter(|e| **e >= 9).count();
            assert_eq!(crash, 26, "40.6% of the first half's draw");
            let crash_first_half = table[..SECOND_HALF_ENTRIES]
                .iter()
                .filter(|e| **e >= 9)
                .count();
            assert_eq!(crash_first_half, 11, "34.4% of the second's");
            let mut plain: Vec<usize> = (1..=4)
                .map(|c| table.iter().filter(|e| **e == c).count())
                .collect();
            plain.sort_unstable();
            assert_eq!(
                plain,
                vec![7, 7, 12, 12],
                "two colours are favoured over two"
            );
        }
    }

    /// the same seed deals the same game, which is the whole of a shared seed
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

    /// **a pair never self-destructs**: two crash gems of a colour would break the moment they
    /// landed, so the first half is demoted
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

    /// the rainbow is a schedule, not a chance: the 25th pair carries one, and only the second
    /// half is ever a rainbow
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

    /// **the drought rule.** Deal thirteen of a colour and the next pair answers with that
    /// colour's crash gem.
    #[test]
    fn a_flood_of_one_colour_is_answered_with_its_crash_gem() {
        let mut random = random();
        let mut counts = [0u8; GemColor::N];
        let mut owed: Option<GemColor> = None;
        let mut answered = 0;
        for _ in 0..600 {
            let pair = random.next_pair();
            if let Some(color) = owed.take() {
                answered += 1;
                assert!(
                    pair.pivot == Half::Crash(color)
                        || (pair.pivot == Half::Plain(color) && pair.child == Half::Crash(color)),
                    "{color:?} was owed a crash gem and got {:?}",
                    pair.pivot
                );
            }
            for half in [pair.pivot, pair.child] {
                if let Some(color) = color_of(half) {
                    let count = &mut counts[color.index()];
                    *count += 1;
                    if *count > DROUGHT {
                        *count = 0;
                        owed = Some(color);
                    }
                }
            }
        }
        assert!(answered > 20, "and the rule fired often, not once");
    }
}
