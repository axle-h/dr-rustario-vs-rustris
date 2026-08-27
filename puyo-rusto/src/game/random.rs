//! The seeded colour sequence.
//!
//! Puyo Puyo Tsu does not draw a colour when it needs one. It builds a **pool** of 128 pairs
//! when the match starts, shared by every player, and reads it sequentially - looping when it
//! runs off the end. The pool holds each colour equally often, so over 128 pairs a player has
//! had exactly as many of one colour as of another.
//!
//! Sourced from Puyo Nexus,
//! [Upcoming Pair Randomizer](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Upcoming_Pair_Randomizer),
//! read 2026-08-27 - including the part that is easy to miss: the **opening** is dealt from a
//! reduced set, so the first two pairs of any match use only three colours, and in a five
//! colour match the two after that use only four. You cannot be handed a fifth colour before
//! you have anywhere to put it.
//!
//! Building the whole pool up front is also what keeps a match fair. Every player is dealt the
//! same game from one seed, and [`from_seed`] hands out *independent* randomisers that stay in
//! step only because nothing they draw depends on player-local state. A pool that is fixed at
//! construction cannot drift, however far apart a playlist moves two players.

use crate::game::cell::{PuyoColor, PuyoPiece};
pub use engine::game::random::Seed;
use rand::RngExt;
use rand_chacha::ChaChaRng;

/// puyos in the pool: 128 pairs, about three and a half full boards
pub const POOL_PUYOS: usize = 256;
pub const POOL_PAIRS: usize = POOL_PUYOS / 2;

/// how many upcoming pairs a player is shown
pub const PEEK_SIZE: usize = 2;

/// how many pairs of the opening are held down to a reduced set of colours
const OPENING_PAIRS: usize = 2;

/// how many colours a match may deal
pub const MIN_COLORS: usize = 3;
pub const MAX_COLORS: usize = PuyoColor::N;

/// The pair pool of one match, and how far through it this player is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameRandom {
    pool: Vec<PuyoColor>,
    /// index of the next puyo, always even
    next: usize,
    seed: Seed,
}

impl GameRandom {
    pub fn from_seed(seed: Seed, colors: usize) -> Self {
        Self {
            pool: build_pool(&mut seed.rng(), colors),
            next: 0,
            seed,
        }
    }

    /// The seed this match was dealt from, for anything that needs randomness of its own.
    ///
    /// Nothing drawn from it can put two players out of step: the pool above is fixed at
    /// construction, so it is not a stream anybody else is reading from.
    pub fn seed(&self) -> Seed {
        self.seed
    }

    /// the next pair, advancing the pool and looping at the end of it
    pub fn next_pair(&mut self) -> PuyoPiece {
        let piece = self.pair_at(self.next);
        self.next = (self.next + 2) % POOL_PUYOS;
        piece
    }

    /// the next `PEEK_SIZE` pairs, soonest first, without taking them
    pub fn peek(&self) -> Vec<PuyoPiece> {
        (0..PEEK_SIZE)
            .map(|i| self.pair_at((self.next + 2 * i) % POOL_PUYOS))
            .collect()
    }

    fn pair_at(&self, index: usize) -> PuyoPiece {
        PuyoPiece::new(self.pool[index], self.pool[(index + 1) % POOL_PUYOS])
    }

    /// every colour this match can deal, which is what a theme keys its sprites on
    pub fn colors(&self) -> Vec<PuyoColor> {
        let mut colors = self.pool.clone();
        colors.sort();
        colors.dedup();
        colors
    }
}

/// `count` randomisers all dealt from one seed: every player sees the same pairs, in the same
/// order, whenever the playlist gets round to them
pub fn from_seed(seed: Seed, count: usize, colors: usize) -> Vec<GameRandom> {
    (0..count)
        .map(|_| GameRandom::from_seed(seed, colors))
        .collect()
}

pub fn random(count: usize, colors: usize) -> Vec<GameRandom> {
    from_seed(Seed::random(), count, colors)
}

/// Build the pool a match of `colors` colours is dealt from.
///
/// The three pools are all built, whatever the match wants, because the smaller ones are what
/// hold the opening down - and building them unconditionally keeps one seed dealing one
/// sequence whichever difficulty asks for it.
fn build_pool(rng: &mut ChaChaRng, colors: usize) -> Vec<PuyoColor> {
    let colors = colors.clamp(MIN_COLORS, MAX_COLORS);

    // which colours this match uses at all, and in what order the subsets take them
    let mut set = PuyoColor::ALL;
    for i in (1..set.len()).rev() {
        set.swap(i, rng.random_range(0..=i));
    }

    let mut pools: Vec<Vec<PuyoColor>> = (MIN_COLORS..=MAX_COLORS)
        .map(|n| {
            let mut pool: Vec<PuyoColor> = (0..POOL_PUYOS).map(|i| set[i % n]).collect();
            // the game's own shuffle: walk the pool from the end, swapping each puyo with one
            // picked at random from anywhere in it
            for i in (0..POOL_PUYOS).rev() {
                pool.swap(i, rng.random_range(0..POOL_PUYOS));
            }
            pool
        })
        .collect();

    // hold the opening down to three colours, then to four: the first two pairs of every
    // larger pool are the three-colour pool's, and in the five colour pool the two pairs
    // after that are the four-colour pool's
    let opening = OPENING_PAIRS * 2;
    for larger in 1..pools.len() {
        for i in 0..opening {
            pools[larger][i] = pools[0][i];
        }
    }
    if pools.len() > 2 {
        for i in opening..2 * opening {
            pools[2][i] = pools[1][i];
        }
    }

    pools.swap_remove(colors - MIN_COLORS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn pool_of(seed: u64, colors: usize) -> GameRandom {
        GameRandom::from_seed(Seed::from_u64(seed), colors)
    }

    fn drawn(random: &mut GameRandom, pairs: usize) -> Vec<PuyoPiece> {
        (0..pairs).map(|_| random.next_pair()).collect()
    }

    fn counts(colors: &[PuyoColor]) -> HashMap<PuyoColor, usize> {
        let mut counts = HashMap::new();
        for color in colors {
            *counts.entry(*color).or_insert(0) += 1;
        }
        counts
    }

    #[test]
    fn a_match_deals_the_number_of_colours_it_asked_for() {
        for colors in MIN_COLORS..=MAX_COLORS {
            let random = pool_of(7, colors);
            assert_eq!(random.colors().len(), colors, "{colors} colours");
        }
    }

    /// the pool holds each colour equally often, so nobody is starved of one over a match
    #[test]
    fn the_pool_is_evenly_divided_between_the_colours() {
        for colors in MIN_COLORS..=MAX_COLORS {
            let random = pool_of(11, colors);
            let counts = counts(&random.pool);
            // the opening overwrite is the only thing that puts it out, and only by the two
            // pairs it rewrites
            let expected = POOL_PUYOS / colors;
            for (color, count) in counts {
                let slack = 2 * OPENING_PAIRS * 2;
                assert!(
                    count.abs_diff(expected) <= slack,
                    "{colors} colours: {color:?} appeared {count} times, expected about {expected}"
                );
            }
        }
    }

    /// the sourced opening rule: no fourth colour in the first two pairs, whatever the match
    /// is otherwise dealing
    #[test]
    fn the_first_two_pairs_use_only_three_colours() {
        for colors in MIN_COLORS..=MAX_COLORS {
            for seed in 0..40 {
                let mut random = pool_of(seed, colors);
                let opening: Vec<PuyoColor> = drawn(&mut random, OPENING_PAIRS)
                    .iter()
                    .flat_map(|pair| [pair.pivot, pair.child])
                    .collect();
                let distinct = counts(&opening).len();
                assert!(
                    distinct <= MIN_COLORS,
                    "{colors} colours, seed {seed}: the opening used {distinct}"
                );
            }
        }
    }

    /// ... and in a five colour match the two pairs after that are still held to four
    #[test]
    fn a_five_colour_match_holds_the_next_two_pairs_to_four() {
        for seed in 0..40 {
            let mut random = pool_of(seed, 5);
            let opening: Vec<PuyoColor> = drawn(&mut random, 2 * OPENING_PAIRS)
                .iter()
                .flat_map(|pair| [pair.pivot, pair.child])
                .collect();
            let distinct = counts(&opening).len();
            assert!(distinct <= 4, "seed {seed}: the opening used {distinct}");
        }
    }

    /// every player of a match is dealt the same pairs, however far apart a playlist has
    /// moved them
    #[test]
    fn one_seed_deals_every_player_the_same_pairs() {
        let mut players = from_seed(Seed::from_u64(1234), 3, 4);
        let first = drawn(&mut players[0], 200);
        for player in players[1..].iter_mut() {
            assert_eq!(drawn(player, 200), first);
        }
        // ... and a player dealt their game later still gets the same one
        let mut late = GameRandom::from_seed(Seed::from_u64(1234), 4);
        assert_eq!(drawn(&mut late, 200), first);
    }

    #[test]
    fn another_seed_deals_another_game() {
        let mut a = pool_of(1, 4);
        let mut b = pool_of(2, 4);
        assert_ne!(drawn(&mut a, 128), drawn(&mut b, 128));
    }

    /// the pool loops rather than running out
    #[test]
    fn the_pool_repeats_after_a_hundred_and_twenty_eight_pairs() {
        let mut random = pool_of(99, 4);
        let first = drawn(&mut random, POOL_PAIRS);
        assert_eq!(drawn(&mut random, POOL_PAIRS), first);
    }

    #[test]
    fn peeking_shows_what_is_dealt_next() {
        let mut random = pool_of(5, 4);
        for _ in 0..300 {
            let peeked = random.peek();
            assert_eq!(peeked.len(), PEEK_SIZE);
            assert_eq!(random.next_pair(), peeked[0]);
            assert_eq!(random.peek()[0], peeked[1]);
        }
    }

    /// a three colour match never shows a fourth, anywhere in the pool
    #[test]
    fn a_three_colour_match_deals_only_three_colours() {
        let mut random = pool_of(3, 3);
        let colors = random.colors();
        assert_eq!(colors.len(), 3);
        for pair in drawn(&mut random, 2 * POOL_PAIRS) {
            assert!(colors.contains(&pair.pivot));
            assert!(colors.contains(&pair.child));
        }
    }

    /// asking for a silly number of colours is clamped rather than panicking
    #[test]
    fn the_colour_count_is_held_to_what_the_game_has() {
        assert_eq!(pool_of(1, 0).colors().len(), MIN_COLORS);
        assert_eq!(pool_of(1, 99).colors().len(), MAX_COLORS);
    }

    /// different seeds pick different colours for a three colour game, so it is not always
    /// the same three
    #[test]
    fn a_three_colour_match_does_not_always_use_the_same_three() {
        let sets: std::collections::HashSet<Vec<PuyoColor>> =
            (0..50).map(|seed| pool_of(seed, 3).colors()).collect();
        assert!(sets.len() > 1, "every seed dealt the same three colours");
    }
}
