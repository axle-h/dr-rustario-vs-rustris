use std::ops::{Add, Mul, RangeInclusive};
use std::fmt::{Debug, Display, Formatter};
use rand::distr::{Distribution, StandardUniform};
use rand::{Rng, RngExt};

pub const COEFFICIENT_STEP: f64 = 0.000001;

pub const RANDOM_RAW_COEFFICIENT_RANGE: RangeInclusive<i64> = raw_coefficient_range(1.0);

pub const DEFAULT_MUTATION_STEP: f64 = 0.1;

pub const fn raw_coefficient_range(delta: f64) -> RangeInclusive<i64> {
    let from = Coefficient::from_f64_unchecked(-delta).raw();
    let to = Coefficient::from_f64_unchecked(delta).raw();
    from ..= to
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Coefficient(i64);

impl Coefficient {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(1);

    pub const fn new(magnitude: i64) -> Self {
        Self(magnitude)
    }
    
    pub const fn from_f64_unchecked(value: f64) -> Self {
        Self((value / COEFFICIENT_STEP) as i64)
    }

    pub fn from_f64(value: f64) -> Self {
        Self((value / COEFFICIENT_STEP).round() as i64)
    }
    
    pub const fn raw(&self) -> i64 {
        self.0
    }
    
    pub const fn into_f64(self) -> f64 {
        self.raw() as f64 * COEFFICIENT_STEP
    }
}

impl Display for Coefficient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // let value: f64 = (*self).into();
        write!(f, "{}", self.0)
    }
}

impl Debug for Coefficient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Mul<f64> for Coefficient {
    type Output = f64;

    fn mul(self, rhs: f64) -> Self::Output {
        let value: f64 = self.into();
        value * rhs
    }
}

impl Mul<Coefficient> for Coefficient {
    type Output = Coefficient;

    fn mul(self, rhs: Coefficient) -> Self::Output {
        let lhs: f64 = self.into();
        let rhs: f64 = rhs.into();
        Self::from_f64(lhs * rhs)
    }
}

impl Mul<Coefficient> for f64 {
    type Output = f64;

    fn mul(self, rhs: Coefficient) -> Self::Output {
        let value: f64 = rhs.into();
        self * value
    }
}

impl Add<Coefficient> for Coefficient {
    type Output = Coefficient;
    fn add(self, rhs: Coefficient) -> Self::Output {
        Self(self.raw() + rhs.raw())
    }
}

impl Into<f64> for Coefficient {
    fn into(self) -> f64 {
        self.into_f64()
    }
}

impl From<f64> for Coefficient {
    fn from(value: f64) -> Self {
        Self::from_f64(value)
    }
}

impl From<i64> for Coefficient {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl Distribution<Coefficient> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Coefficient {
        Coefficient(rng.random_range(RANDOM_RAW_COEFFICIENT_RANGE))
    }
}