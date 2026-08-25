use std::fmt::{Display, Formatter};
use crate::ai::game_result::GameResult;
use crate::ai::genome::Genome;
use crate::ai::objective::Objective;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Organism<const GENOME: usize> {
    genome: Genome<GENOME>,
    result: Option<GameResult>,
}

impl<const GENOME: usize> Organism<GENOME> {
    pub fn new(genome: Genome<GENOME>) -> Self {
        Self { genome, result: None }
    }

    pub fn genome(&self) -> Genome<GENOME> {
        self.genome
    }

    pub fn result(&self) -> GameResult {
        self.result.unwrap()
    }
    
    pub fn fitness(&self, objective: Objective) -> f64 {
        objective.fitness(&self.result())
    }

    pub fn set_result<F>(&mut self, f: F) where F : FnOnce(&Genome<GENOME>) -> GameResult {
        if self.result.is_none() {
            self.result = Some(f(&self.genome));
        }
    }
    
    pub fn unset_result(&mut self) {
        self.result = None;
    }
}

impl<const GENOME: usize> Display for Organism<GENOME> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(result) = self.result {
            write!(f, "{} {}", result, self.genome)
        } else {
            write!(f, "{}", self.genome)
        }
    }
}