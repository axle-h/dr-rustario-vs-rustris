//! The seeded gem sequence, and the two rules in it nobody has written down.
//!
//! `FUN_8012fd78` deals in **two phases**, and which one it is in is `+0xd8`, a counter that
//! `FUN_8012fa20` steps once every ten pieces:
//!
//! * **the opening**, `+0xd8 < 2`, so the first [`EARLY_PAIRS`] pieces of a round. The pivot
//!   is drawn from [`EARLY_GEM_TABLES`], which hold **no crash gems at all**, and only the
//!   child can carry one - from [`EARLY_CHILD_TABLE`], twelve entries in thirty two. On top of
//!   that a child that matches the pivot's *colour* is stripped of its crash bit, so an
//!   opening pair is never a crash gem sitting on its own colour. About **0.28** crash gems a
//!   pair, against the 0.73 that comes after: this is the room the game gives you to build.
//! * **the rest of the round**, from there on. Each half is an independent draw from a 64
//!   entry weighted table and **the two halves do not draw from the same distribution**: the
//!   first reads the whole table, the second only its first 32 entries.
//!
//! Nothing in between - it is one step, not a ramp, and it is why a board that felt buildable
//! for the first twenty pieces starts breaking under you afterwards.
//!
//! Three more rules sit over both phases, from `FUN_8012fd78` and `FUN_8012FF2C`:
//!
//! * **no pair self-destructs.** If both halves come out as the same crash gem the first is
//!   demoted to the plain gem of that colour, so a pair can never break on landing.
//! * **the rainbow is on a schedule**, not a chance: every 25th pair, from a table. The guides
//!   are right and now it is sourced.
//! * **the drought rule**, which is documented nowhere. Every gem dealt is counted by colour,
//!   and when a colour's count passes twelve it resets and the *next* pair's first half is
//!   forced to that colour's crash gem. You are guaranteed an answer to whatever you have been
//!   flooded with - and this is the one way the opening deals a crash gem as the pivot.
//!
//! **One rule here is not the executable's**: the [`CRASH_GAP_START`] throttle, which stretches
//! that twenty piece opening into a gap that closes over two hundred pairs, because four crash
//! gems in five pairs leaves nothing to build with. Its own doc comment is where the argument
//! and the measurements are.
//!
//! Which of the eight tables a match uses is `+0x292`, and **nothing in the executable writes
//! it anything but zero** - the PS1 port only ever deals from the first, and the other seven
//! are reachable data with no setter. It is drawn from the seed here instead, and **fixed for
//! the whole match**, which is the compendium's rule that `speed_index` may change how a game
//! feels but never what it deals.

use crate::game::cell::{GemColor, GemPair, Half};
use crate::game::tables::{
    EARLY_CHILD_TABLE, EARLY_GEM_TABLES, EARLY_TABLE_ENTRIES, GEM_TABLES, GEM_TABLES_COUNT,
    RAINBOW_SCHEDULE, SECOND_HALF_ENTRIES, TABLE_ENTRIES,
};
pub use engine::game::random::Seed;
use rand::RngExt;
use rand_chacha::ChaChaRng;
use std::collections::VecDeque;

/// how many upcoming pairs a player is shown - one, the NEXT box
pub const PEEK_SIZE: usize = 1;

/// how many of a colour may be dealt before the drought rule answers with a crash gem
pub const DROUGHT: u8 = 12;

/// How many pairs a round opens with before crash gems come at their full rate.
///
/// `+0xd8` steps once per ten pieces and the deal reads `+0xd8 < 2`, so twenty. A match here
/// is one round, so this counts from the first pair dealt and never resets.
pub const EARLY_PAIRS: u32 = 20;

/// **The crash gem throttle - a house rule, and the one place this game is not the arcade's.**
///
/// The executable's own opening (above) is twenty pieces long and then stops, and what it
/// stops into deals a crash gem in four pairs out of five. That is the arcade's answer and it
/// is not the one we want: a power gem worth building is eight or ten pairs of one colour, and
/// a crash gem every 1.3 pairs is a decision about breaking taken for you before you have
/// anything to break. So a crash gem is **demoted to its plain gem unless enough pairs have
/// passed since the last one survived**, and the gap that "enough" means closes over the match:
///
/// * [`CRASH_GAP_START`] pairs of clear air to open with, **held flat** for the length of the
///   executable's own opening ([`EARLY_PAIRS`]), so the match starts with a stretch of nothing
///   but building material
/// * then closing to [`CRASH_GAP_END`] by pair [`CRASH_GAP_CLOSES_BY`], where the deal is the
///   executable's again and the throttle is doing nothing at all
///
/// **It closes as the square of the pairs left, not linearly**, and that is measured rather
/// than chosen for looks. What a player feels is the *rate*, and the rate is about
/// `1 / (gap + 2)` - so a gap that closes linearly holds the rate almost still for as long as
/// the gap is wide and then drops it off a cliff at the end: 0.14, 0.18, 0.35 over the three
/// windows below where squaring gives 0.15, 0.28, 0.70. Squaring spends the gap where it is
/// worth something and leaves the opening alone.
///
/// **The drought waits rather than being exempted.** `FUN_8012FF2C`'s answer to a colour you
/// have been flooded with is owed until the first pair the gap is not holding, and then paid
/// in full - so the promise survives and the gap is the whole story of *when* a crash gem
/// arrives. Exempting it instead was tried and is the worse rule: a colour runs dry every six
/// or seven pairs across four colours, so an exempt drought is a floor of its own and the dial
/// below stops meaning anything past the opening (one crash gem every 5 pairs at pair 60,
/// against the 7 that waiting gives).
///
/// The gap is a function of the pair count alone, so it is the same for every player of a
/// shared seed however far apart in a playlist they are.
///
/// What it deals, measured over 500 seeds:
///
/// | pairs | crash gems a pair | one every | median gap |
/// |--|--|--|--|
/// | 0-20 | 0.05 | 19.7 pairs | 12 |
/// | 20-60 | 0.15 | 6.6 pairs | 9 |
/// | 60-120 | 0.28 | 3.6 pairs | 4 |
/// | 120-200 | 0.70 | 1.4 pairs | 1 |
/// | 200-400 | 0.79 | 1.3 pairs | 1 |
///
/// In twenties, which is where the climb is: 0.05, 0.15, 0.16, 0.23, 0.26, 0.36, 0.49, 0.74,
/// and the executable's own rate from pair 160.
pub const CRASH_GAP_START: u32 = 10;
/// where [`CRASH_GAP_START`] ends up: nothing held back, the executable's own rate
pub const CRASH_GAP_END: u32 = 0;
/// the pair by which the gap has closed to [`CRASH_GAP_END`]; raise it to stretch the climb
/// out without touching the opening, which the hold owns
pub const CRASH_GAP_CLOSES_BY: u32 = 200;

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
    /// house rule: pairs dealt since a crash gem last survived the throttle
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
            // zero, not the opening gap: the first crash gem waits like every other one, which
            // is what makes the first stretch of the match clear
            since_crash: 0,
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
        // The house throttle, decided before the draw. What the drought owes is not spent
        // inside the gap - it waits for it, so the gap below is the whole story of when a
        // crash gem arrives - and it is followed by colour rather than by position, because
        // the self-destruct demotion can move it from the pivot to the child.
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

        // A pair never self-destructs on landing, and this is last for the same reason it is
        // last in `FUN_8012fd78`: everything that can *put* a crash gem in the pair - the
        // rainbow taking the child, the drought taking the pivot - has already had its say, so
        // there is exactly one place the two halves are compared and it sees the final pair.
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

    /// The house rule: [`CRASH_GAP_START`] and what it is for.
    ///
    /// A crash gem inside the gap is demoted to its plain gem, which is the same thing the
    /// deal's own two demotions do and so needs no new vocabulary on the board. `forced` is
    /// the colour the drought paid into this pair, which only happens outside the gap and is
    /// therefore never held; a rainbow is not a crash gem and is never touched.
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

    /// The opening draw: a pivot that cannot be a crash gem, and a child that is stripped of
    /// its crash bit when it lands on the pivot's own colour.
    ///
    /// The colour test ignores the crash bit on both sides, which is `FUN_8012fd78`'s
    /// `if (7 < iVar) iVar -= 8` before the comparison - so a same-coloured opening pair is
    /// two plain gems whichever half was the crash one.
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

    /// The draw the rest of the round uses: the whole table for the pivot, its first half for
    /// the child.
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

/// How many pairs of clear air the throttle asks for once `dealt` pairs have gone by.
///
/// [`CRASH_GAP_START`] flat until [`EARLY_PAIRS`], then falling as the square of the pairs
/// still to run to [`CRASH_GAP_END`] at [`CRASH_GAP_CLOSES_BY`], and staying there. Integer
/// throughout, and squared rather than `powf`'d on purpose: every player is dealt one seed's
/// game, and while the four basic float operations give the same answer everywhere, `powf` is
/// not required to.
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

    /// every table holds nothing but the eight gems, and each is the shape the rules doc
    /// reports: six bias two colours at twelve entries over two at seven, and the last two
    /// are flat at nine each - 26 crash gems in the biased six, 28 in the flat two
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

    /// **the opening deals no crash gem in the pivot.** The eight tables it draws that half
    /// from hold only plain gems; the child's one table is where the crash gems are.
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

    /// **the crash gems come in closing gaps.** The house throttle holds the opening down to
    /// one crash gem in twenty pairs and lets go by [`CRASH_GAP_CLOSES_BY`], where the rate is
    /// the executable's own - and it climbs the whole way rather than sitting flat and then
    /// jumping, which is what a linear gap did.
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

    /// **and nothing at all falls inside the gap.** Every crash gem, the drought's answer
    /// included, sits at least [`crash_gap`] pairs after the one before it.
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

    /// **the drought rule**, and what the house throttle does to it. Deal thirteen of a
    /// colour and that colour's crash gem is owed - but it is paid on the first pair the gap
    /// is not holding rather than on the very next one, so the answer still comes and the
    /// long opening gaps survive it.
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
