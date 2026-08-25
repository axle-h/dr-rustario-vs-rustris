//! The seed a training run counts through. Distinct from [crate::game::random::Seed], which is
//! the opaque 256 bit seed a game is played on: this one does big integer arithmetic so the
//! genetic algorithm can walk whole blocks of unused seeds, and converts into the game seed.

use crate::game::random::Seed as GameSeed;
use num_bigint::BigUint;
use num_traits::Num;
use rand::distr::StandardUniform;
use rand::prelude::*;
use rand::{Rng, RngExt};
use rand_chacha::ChaChaRng;
use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Deref, DerefMut};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seed(<ChaChaRng as SeedableRng>::Seed);

impl Deref for Seed {
    type Target = <ChaChaRng as SeedableRng>::Seed;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Seed {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Display for Seed {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let bigint: BigUint = (*self).into();
        write!(f, "{}", bigint)
    }
}

impl Add for Seed {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut result = self;
        result += rhs;
        result
    }
}

impl AddAssign for Seed {
    fn add_assign(&mut self, rhs: Self) {
        let mut carry = 0u64;

        // Process 8 bytes at a time using u64
        for i in (0..32).step_by(8) {
            let a = u64::from_le_bytes(self[i..i+8].try_into().unwrap());
            let b = u64::from_le_bytes(rhs[i..i+8].try_into().unwrap());

            // Add previous carry to first number
            let sum = a.wrapping_add(b).wrapping_add(carry);

            // Calculate new carry - if sum is less than either input (or equal to when carry is 1),
            // we wrapped around and need to carry 1
            carry = if (carry == 1 && sum <= a) || (carry == 0 && sum < a) {
                1
            } else {
                0
            };

            self[i..i+8].copy_from_slice(&sum.to_le_bytes());
        }
    }
}

impl Default for Seed {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl Distribution<Seed> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Seed {
        Seed(rng.random())   
    }
}

impl From<u128> for Seed {
    fn from(value: u128) -> Self {
        let mut seed = Seed::default();
        seed[..16].copy_from_slice(&value.to_le_bytes());
        seed
    }
}

impl From<BigUint> for Seed {
    fn from(value: BigUint) -> Self {
        let mut bytes = value.to_bytes_be();
        // pad to 32 bytes
        while bytes.len() < 32 {
            bytes.insert(0, 0);
        }
        Self(bytes.try_into().expect("expecting a 256 bit number"))
    }
}

impl Into<BigUint> for Seed {
    fn into(self) -> BigUint {
        BigUint::from_bytes_be(&*self)
    }
}

impl From<i32> for Seed {
    fn from(value: i32) -> Self {
        let mut seed = Seed::default();
        seed[..4].copy_from_slice(&value.to_le_bytes());
        seed
    }
}

impl From<String> for Seed {
    fn from(value: String) -> Self {
        BigUint::from_str_radix(&value, 10).expect("not a valid seed string").into()
    }
}

impl Into<ChaChaRng> for Seed {
    fn into(self) -> ChaChaRng {
        ChaChaRng::from_seed(self.0)
    }
}

impl From<Seed> for GameSeed {
    fn from(seed: Seed) -> Self {
        Self::from(seed.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_seed() {
        let seed1 = Seed::from(999999999999999999999999999u128);
        let seed2 = Seed::from(1);
        let seed3 = seed1 + seed2;
        assert_eq!(seed3, Seed::from(1000000000000000000000000000u128));
    }

    #[test]
    fn serialize_seed() {
        let bigint = BigUint::parse_bytes(b"111000000000000000000000000000000000000222", 10).unwrap();
        let result: BigUint = Seed::from(bigint.clone()).into();
        assert_eq!(result, bigint);
    }

    #[test]
    fn display_seed() {
        let bigint = BigUint::parse_bytes(b"34028236692093846346337460743176821145500000000000000000000000000000000000000", 10).unwrap();
        let result = format!("{}", Seed::from(bigint.clone()));
        assert_eq!(result, "34028236692093846346337460743176821145500000000000000000000000000000000000000");
    }

    #[test]
    fn same_sequence_from_the_same_seed() {
        let seed: Seed = rand::random();
        let mut rng1 = ChaChaRng::from_seed(seed.0);
        let mut rng2 = ChaChaRng::from_seed(seed.0);

        let next1: u128 = rng1.random();
        let next2: u128 = rng2.random();

        assert_eq!(next1, next2, "RNGs should produce the same sequence with the same seed");
    }
}
