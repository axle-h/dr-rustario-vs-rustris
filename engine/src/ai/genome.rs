use std::fmt::{Debug, Display};
use std::ops::Deref;
use std::vec::IntoIter;
use crate::ai::coefficient::Coefficient;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Genome<const N: usize> {
    chromosome: [Coefficient; N]
}

impl<const N: usize> IntoIterator for Genome<N> {
    type Item = Coefficient;
    type IntoIter = IntoIter<Coefficient>;

    fn into_iter(self) -> Self::IntoIter {
        Vec::from(self.chromosome).into_iter()
    }
}

impl<const N: usize> Deref for Genome<N> {
    type Target = [Coefficient; N];

    fn deref(&self) -> &Self::Target {
        &self.chromosome
    }
}


impl<const N: usize> Genome<N> {
    pub fn new(chromosome: [Coefficient; N]) -> Self {
        Self { chromosome }
    }

    pub fn chromosome(&self) -> [Coefficient; N] {
        self.chromosome
    }
}

impl<const N: usize> Display for Genome<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.chromosome)
    }
}

impl<const N: usize> From<[Coefficient; N]> for Genome<N> {
    fn from(chromosome: [Coefficient; N]) -> Self {
        Self::new(chromosome)
    }
}

impl<const N: usize> Into<[Coefficient; N]> for Genome<N> {
    fn into(self) -> [Coefficient; N] {
        self.chromosome
    }
}

impl<const N: usize> From<[f64; N]> for Genome<N> {
    fn from(value: [f64; N]) -> Self {
        Self::new(value.map(Coefficient::from))
    }
}

impl<const N: usize> Into<[f64; N]> for Genome<N> {
    fn into(self) -> [f64; N] {
        self.chromosome.map(Coefficient::into_f64)
    }
}
