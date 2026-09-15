use rand::prelude::*;
use rand::{rng, RngExt};
use rand_chacha::ChaChaRng;
use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use strum::IntoEnumIterator;

/// A 256-bit seed for the deterministic per-game RNG.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Seed(<ChaChaRng as SeedableRng>::Seed);

impl Seed {
    pub fn random() -> Self {
        let mut seed: <ChaChaRng as SeedableRng>::Seed = Default::default();
        rng().fill(&mut seed);
        Self(seed)
    }

    /// expands a small seed the same way as `SeedableRng::seed_from_u64`
    pub fn from_u64(seed: u64) -> Self {
        Self(ChaChaRng::seed_from_u64(seed).get_seed())
    }

    pub fn bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn rng(&self) -> ChaChaRng {
        ChaChaRng::from_seed(self.0)
    }
}

impl From<[u8; 32]> for Seed {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

impl Display for Seed {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for b in self.0 {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Default,
    strum::IntoStaticStr,
    strum::EnumIter,
    strum::EnumString,
)]
pub enum RandomMode {
    /// All pieces placed in a shuffled "bag" and drawn until the bag is empty, after which a new bag is shuffled
    #[strum(serialize = "bag")]
    #[default]
    Bag = 0,

    /// Uniformly random piece every time
    #[strum(serialize = "true")]
    True = 1,
}

impl RandomMode {
    pub fn names() -> Vec<&'static str> {
        Self::iter().map(|e| e.into()).collect()
    }
}

/// A piece randomiser with a look-ahead queue, drawing either from shuffled bags of `all` or
/// uniformly at random.
#[derive(Clone, Debug)]
pub struct BagRandom<T: Copy + 'static> {
    mode: RandomMode,
    rng: ChaChaRng,
    all: &'static [T],
    peek_size: usize,
    queue: VecDeque<T>,
}

impl<T: Copy + 'static> BagRandom<T> {
    pub fn new(rng: ChaChaRng, mode: RandomMode, all: &'static [T], peek_size: usize) -> Self {
        assert!(!all.is_empty());
        let mut result = Self {
            mode,
            rng,
            all,
            peek_size,
            queue: VecDeque::new(),
        };
        match mode {
            RandomMode::True => {
                for _ in 0..peek_size {
                    let next = result.uniform();
                    result.queue.push_back(next);
                }
            }
            RandomMode::Bag => result.assert_bags(),
        }
        result
    }

    pub fn mode(&self) -> RandomMode {
        self.mode
    }

    pub fn rng(&mut self) -> &mut ChaChaRng {
        &mut self.rng
    }

    fn uniform(&mut self) -> T {
        self.all[self.rng.random_range(0..self.all.len())]
    }

    fn assert_bags(&mut self) {
        while self.queue.len() <= self.peek_size {
            let bag = self
                .all
                .sample(&mut self.rng, self.all.len())
                .copied()
                .collect::<Vec<T>>();
            self.queue.extend(bag);
        }
    }

    /// not `Iterator::next`: this never runs out, so an `Option` would be a lie
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> T {
        match self.mode {
            RandomMode::True => {
                let next = self.uniform();
                self.queue.push_back(next);
                self.queue.pop_front().unwrap()
            }
            RandomMode::Bag => {
                let result = self.queue.pop_front().unwrap();
                self.assert_bags();
                result
            }
        }
    }

    /// the next `peek_size` pieces, soonest first
    pub fn peek(&self) -> Vec<T> {
        self.queue.iter().take(self.peek_size).copied().collect()
    }

    pub fn peek_next(&self) -> T {
        *self.queue.front().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [u8; 4] = [1, 2, 3, 4];

    fn random(mode: RandomMode) -> BagRandom<u8> {
        BagRandom::new(Seed::from_u64(42).rng(), mode, &ALL, 3)
    }

    #[test]
    fn bag_draws_each_piece_once_per_bag() {
        let mut random = random(RandomMode::Bag);
        for _ in 0..10 {
            let mut bag = (0..ALL.len()).map(|_| random.next()).collect::<Vec<u8>>();
            bag.sort();
            assert_eq!(bag, ALL);
        }
    }

    #[test]
    fn peek_matches_next() {
        for mode in [RandomMode::Bag, RandomMode::True] {
            let mut random = random(mode);
            let peeked = random.peek();
            assert_eq!(peeked.len(), 3);
            assert_eq!(random.peek_next(), peeked[0]);
            for expected in peeked {
                assert_eq!(random.next(), expected);
            }
        }
    }

    #[test]
    fn same_seed_same_sequence() {
        let mut a = random(RandomMode::True);
        let mut b = random(RandomMode::True);
        let sa = (0..20).map(|_| a.next()).collect::<Vec<u8>>();
        let sb = (0..20).map(|_| b.next()).collect::<Vec<u8>>();
        assert_eq!(sa, sb);
    }
}
