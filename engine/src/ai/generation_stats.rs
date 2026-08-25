use std::fmt::{Display, Formatter};
use crate::ai::objective::Objective;
use crate::ai::organism::Organism;
use crate::ai::seed::Seed;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GenerationStatistics<const GENOME: usize> {
    id: usize,
    objective: Objective,
    seed: Seed,
    max: Organism<GENOME>,
    p95: Organism<GENOME>,
    median: Organism<GENOME>,
    mutation_rate: f64,
    crossover_rate: f64,
    total_gameplay_time: Duration,
    generation_duration: Duration,
    game_seconds_per_second: f64,
}

impl<const GENOME: usize> Display for GenerationStatistics<GENOME> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{} {}] p100: {{{}}}, p95: {{{}}}, p50: {{{}}}, mutation_rate: {:.3}, crossover_rate: {:.3}, game seconds/second: {:.2}",
               self.id, self.objective, self.max.result(), self.p95.result(), self.median.result(), self.mutation_rate, self.crossover_rate, self.game_seconds_per_second)
    }
}

impl<const GENOME: usize> GenerationStatistics<GENOME> {
    pub fn new(id: usize, objective: Objective, seed: Seed, max: Organism<GENOME>, p95: Organism<GENOME>, median: Organism<GENOME>, mutation_rate: f64, crossover_rate: f64, total_gameplay_time: Duration, generation_duration: Duration, game_seconds_per_second: f64) -> Self {
        Self { id, objective, seed, max, p95, median, mutation_rate, crossover_rate, total_gameplay_time, generation_duration, game_seconds_per_second }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn objective(&self) -> Objective {
        self.objective
    }

    pub fn max(&self) -> Organism<GENOME> {
        self.max
    }

    pub fn p95(&self) -> Organism<GENOME> {
        self.p95
    }

    pub fn median(&self) -> Organism<GENOME> {
        self.median
    }

    pub fn mutation_rate(&self) -> f64 {
        self.mutation_rate
    }

    pub fn crossover_rate(&self) -> f64 {
        self.crossover_rate
    }

    pub fn seed(&self) -> Seed {
        self.seed
    }

    pub fn total_gameplay_time(&self) -> Duration {
        self.total_gameplay_time
    }

    pub fn generation_duration(&self) -> Duration {
        self.generation_duration
    }

    pub fn game_seconds_per_second(&self) -> f64 {
        self.game_seconds_per_second
    }
}
