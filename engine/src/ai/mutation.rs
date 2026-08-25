use std::array;
use std::ops::RangeInclusive;
use rand::{Rng, RngExt};
use std::collections::VecDeque;
use rand_chacha::ChaChaRng;
use crate::ai::coefficient::{raw_coefficient_range, Coefficient, DEFAULT_MUTATION_STEP};
use crate::ai::generation_stats::GenerationStatistics;
use crate::ai::genome::Genome;
use crate::ai::objective::Objective;
use crate::ai::organism::Organism;
use crate::ai::seed::Seed;

#[derive(Debug, Clone)]
pub struct RateLimits {
    range: RangeInclusive<f64>,
    step: f64,
    current: f64,
}

impl RateLimits {
    pub const fn from_static(chance: f64) -> Self {
        Self { range: chance ..= chance, step: 0.0, current: chance }
    }

    pub const NEVER: Self = Self::from_static(0.0);
    pub const ALWAYS: Self = Self::from_static(1.0);
    
    pub fn new(range: RangeInclusive<f64>) -> Self {
        assert!(*range.start() >= 0.0 && *range.end() <= 1.0);
        let current = (range.start() + range.end()) / 2.0;
        let step = (range.end() - range.start()) / 10.0;
        Self { range, step, current }
    }
    
    fn decrement(&mut self) {
        self.current = (self.current - self.step).max(*self.range.start())
    }
    
    fn increment(&mut self) {
        self.current = (self.current + self.step).min(*self.range.end())
    }
    
    fn test(&self, rng: &mut impl Rng) -> bool {
        let chance: f64 = rng.random();
        chance <= self.current
    }
}

impl Default for RateLimits {
    fn default() -> Self {
        Self::new(0.0 ..= 1.0)
    }
}

pub struct GenomeMutation<const GENOME: usize> {
    mutation_rate: RateLimits,
    crossover_rate: RateLimits,
    delta_range: RangeInclusive<i64>,
    samples: VecDeque<f64>,
    stat_fn: fn(GenerationStatistics<GENOME>) -> f64,
    rng: ChaChaRng,
}

impl<const N: usize> GenomeMutation<N> {
    fn new(
        mutation_rate: RateLimits,
        crossover_rate: RateLimits,
        max_samples: usize,
        seed: Seed,
        stat_fn: fn(GenerationStatistics<N>) -> f64
    ) -> Self {
        Self {
            mutation_rate,
            crossover_rate,
            delta_range: raw_coefficient_range(DEFAULT_MUTATION_STEP),
            samples: VecDeque::with_capacity(max_samples),
            stat_fn,
            rng: seed.into()
        }
    }

    /// change the magnitude of a mutation nudge (the default is 0.1)
    pub fn with_mutation_step(mut self, step: f64) -> Self {
        self.set_mutation_step(step);
        self
    }

    pub fn set_mutation_step(&mut self, step: f64) {
        assert!(step > 0.0, "mutation step must be positive");
        self.delta_range = raw_coefficient_range(step);
    }

    /// reconfigure rates and mutation step, e.g. when switching training phase; forgets the trend samples
    pub fn set_rates(&mut self, mutation_rate: RateLimits, crossover_rate: RateLimits, mutation_step: f64) {
        self.mutation_rate = mutation_rate;
        self.crossover_rate = crossover_rate;
        self.set_mutation_step(mutation_step);
        self.samples.clear();
    }
    
    pub fn current_mutation_rate(&self) -> f64 {
        self.mutation_rate.current
    }
    
    pub fn current_crossover_rate(&self) -> f64 {
        self.crossover_rate.current
    }

    pub fn of_median(
        mutation_rate: RateLimits,
        crossover_rate: RateLimits,
        max_samples: usize,
        seed: Seed,
    ) -> Self {
        Self::new(
            mutation_rate,
            crossover_rate,
            max_samples,
            seed,
            |stats: GenerationStatistics<N>| stats.objective().fitness(&stats.median().result())
        )
    }

    pub fn of_max(
        mutation_rate: RateLimits,
        crossover_rate: RateLimits,
        max_samples: usize,
        seed: Seed,
    ) -> Self {
        Self::new(
            mutation_rate,
            crossover_rate,
            max_samples,
            seed,
            |stats: GenerationStatistics<N>| stats.objective().fitness(&stats.max().result())
        )
    }

    pub fn add_sample(&mut self, stats: GenerationStatistics<N>) {
        let value = (self.stat_fn)(stats);
        if self.samples.len() >= self.samples.capacity() {
            self.samples.pop_front(); // Remove oldest sample if at capacity
        }
        self.samples.push_back(value);

        if self.samples.len() < self.samples.capacity() {
            return;
        }

        // Calculate trend by comparing latest half of samples to earlier half
        let mid = self.samples.len() / 2;
        let older: f64 = self.samples.iter().take(mid).sum::<f64>() / mid as f64;
        let newer: f64 = self.samples.iter().skip(mid).sum::<f64>() / (self.samples.len() - mid) as f64;
        
        // Adjust rates based on trend
        if newer > older {
            self.mutation_rate.decrement();
            self.crossover_rate.decrement();
        } else {
            self.mutation_rate.increment();
            self.crossover_rate.increment();
        }
    }
    
    pub fn random(&mut self) -> Genome<N> {
        Genome::new(array::from_fn(|_| self.rng.random()))
    }
    
    pub fn mutate(&mut self, genome: Genome<N>) -> Genome<N> {
        genome.map(|coefficient| {
            if self.mutation_rate.test(&mut self.rng) {
                self.rng.mutate(coefficient, &self.delta_range)
            } else {
                coefficient
            }
        }).into()
    }

    pub fn crossover(&mut self, genome1: Genome<N>, genome2: Genome<N>) -> [Genome<N>; 2] {
        let mut child1 = [Coefficient::ZERO; N];
        let mut child2 = [Coefficient::ZERO; N];

        for i in 0..N {
            if self.crossover_rate.test(&mut self.rng) {
                child1[i] = genome2[i];
                child2[i] = genome1[i];
            } else {
                child1[i] = genome1[i];
                child2[i] = genome2[i];
            }
        }

        [self.mutate(child1.into()), self.mutate(child2.into())]
    }

    pub fn parents(&mut self, population: &[Organism<N>], count: usize, objective: Objective) -> Vec<[Genome<N>; 2]> {
        let scaled_population = scale_fitness(&population, objective);
        let mut parents: Vec<[Genome<N>; 2]> = vec![];
        for _ in 0..count {
            let mut next_parent = || {
                let p: f64 = self.rng.random();
                let mut cumsum = 0.0;
                for (index, (_, prob)) in scaled_population.iter().enumerate() {
                    cumsum += prob;
                    if p <= cumsum {
                        return index;
                    }
                }
                scaled_population.len() - 1
            };

            let parent1_index = next_parent();
            let (parent1, _) = scaled_population[parent1_index];

            let mut parent2_index = next_parent();
            loop {
                if parent2_index != parent1_index {
                    let (parent2, _) = scaled_population[parent2_index];
                    if parent1 != parent2 {
                        // TODO prefer new parents but do not require
                        // && !parents.contains(&[parent1, parent2])
                        // && !parents.contains(&[parent2, parent1]) {
                        break;
                    }
                }
                parent2_index = (parent2_index + 1) % scaled_population.len();
            }

            let (parent2, _) = scaled_population[parent2_index];
            parents.push([parent1, parent2]);
        }
        parents.into_iter().collect()
    }
}

fn scale_fitness<const N: usize>(population: &[Organism<N>], objective: Objective) -> Vec<(Genome<N>, f64)> {
    let sum_fitness: f64 = population.iter()
        .map(|result| result.fitness(objective))
        .sum();

    if sum_fitness <= 0.0 {
        // nobody has any fitness yet (e.g. no tetris clears): select uniformly
        let uniform = 1.0 / population.len() as f64;
        return population.iter().map(|result| (result.genome(), uniform)).collect();
    }

    population.into_iter().map(|result| (result.genome(), result.fitness(objective) / sum_fitness)).collect()
}

trait RngMutation {
    fn mutate(&mut self, value: Coefficient, delta_range: &RangeInclusive<i64>) -> Coefficient;
}

impl<R: Rng + ?Sized> RngMutation for R {
    fn mutate(&mut self, value: Coefficient, delta_range: &RangeInclusive<i64>) -> Coefficient {
        let random_chance: f64 = self.random();
        if random_chance < 0.2 {
            // small chance of a completely random value
            self.random()
        } else {
            // otherwise just nudge the existing value
            let delta = Coefficient::new(self.random_range(delta_range.clone()));
            value + delta
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use std::time::Duration;
    use itertools::Itertools;
    use crate::ai::game_result::GameResult;
    use super::*;

    const TEST_GENES: usize = 9;

    fn mutation<MR: Into<Option<RateLimits>>, CR : Into<Option<RateLimits>>>(
        mutation_rate: MR,
        crossover_rate: CR
    ) -> GenomeMutation<TEST_GENES> {
        GenomeMutation::of_max(
            mutation_rate.into().unwrap_or(RateLimits::default()),
            crossover_rate.into().unwrap_or(RateLimits::default()),
            10,
            Seed::default()
        )
    }
    
    fn genome(i: i32) -> Genome<TEST_GENES> {
        [Coefficient::new(i as i64); TEST_GENES].into()
    }
    
    #[test]
    fn fittest_genome_can_be_parent_multiple_times() {
        const N: usize = 100;
        let population: Vec<_> = (0 .. N as i32).map(|i| {
            let fitness = 0.5f64.powi(i); // 1.0, 0.5, 0.25, 0.125 etc
            let mut member = Organism::new(genome(i + 1));
            let mut result = GameResult::new(
                (fitness * 1_000_000.0) as u32,
                10,
                1,
                true,
                Duration::from_millis(100)
            );
            member.set_result(|_| result);
            member
        }).collect();
        let parents = mutation(None, None).parents(&population, N / 2, Objective::Survival);

        assert_eq!(parents.len(), N / 2);

        let counts: Vec<_> = parents.iter()
            .flatten()
            .map(|g| g.chromosome())
            .map(|[c0, ..]| c0)
            .sorted()
            .dedup_with_count()
            .collect();

        assert!(counts[0].0 > 1, "Coefficient(1) should appear more than once, got {}", counts[0].0);
        assert!(counts[0].0 > counts[1].0, "Coefficient(1) should appear more often than Coefficient(2), got {} & {}", counts[0].0, counts[1].0);
    }
    

    #[test]
    fn zero_fitness_population_selects_uniformly() {
        let population: Vec<_> = (0 .. 10).map(|i| {
            let mut member = Organism::new(genome(i + 1));
            member.set_result(|_| GameResult::default());
            member
        }).collect();
        let parents = mutation(None, None).parents(&population, 5, Objective::Score);
        assert_eq!(parents.len(), 5);
        for [p1, p2] in parents {
            assert_ne!(p1, p2);
        }
    }

    #[test]
    fn mutation_step_limits_the_nudge() {
        let parent = genome(0);
        let child = mutation(RateLimits::ALWAYS, RateLimits::NEVER)
            .with_mutation_step(0.01)
            .mutate(parent);
        // 20% of genes are fully randomised; the rest must be within the step
        let limit = Coefficient::from_f64(0.01).raw();
        let nudged = child.iter().filter(|&&c| c.raw().abs() <= limit).count();
        assert!(nudged >= 1, "expected at least one small nudge");
    }

    #[test]
    fn crossover_can_swap_all_genes_without_modification() {
        let parent1 = genome(1);
        let parent2 = genome(2);

        let [child1, child2] = mutation(RateLimits::NEVER, RateLimits::ALWAYS)
            .crossover(parent1, parent2);

        // With 100% crossover rate and 0% mutation rate, children should have swapped all genes without modification
        assert_eq!(child1, parent2);
        assert_eq!(child2, parent1);
    }

    #[test]
    fn crossover_swaps_no_genes_when_disabled() {
        let parent1 = genome(1);
        let parent2 = genome(2);

        let [child1, child2] = mutation(RateLimits::NEVER, RateLimits::NEVER)
            .crossover(parent1, parent2);

        // With 0% crossover rate and 0% mutation rate, children should be identical to parents
        assert_eq!(child1, parent1);
        assert_eq!(child2, parent2);
    }

    #[test]
    fn crossover_can_mutate_all_genes() {
        let parent1 = genome(1);
        let parent2 = genome(2);

        let [child1, child2] = mutation(RateLimits::ALWAYS, RateLimits::NEVER)
            .crossover(parent1, parent2);

        // With 100% mutation rate and non-zero magnitude, all genes should be modified
        assert!(child1 != parent1 && child1 != parent2);
        assert!(child2 != parent1 && child2 != parent2);
    }
    
    #[test]
    fn crossover_can_both_mutate_and_swap_some_genes() {
        let parent1 = genome(1);
        let parent2 = genome(2);

        let children = mutation(RateLimits::from_static(0.1), RateLimits::from_static(0.15))
            .crossover(parent1, parent2);

        for child in children {
            // With 50% rates, some genes should be swapped and some should be mutated
            let has_parent1_genes = child.iter().any(|&gene| gene == parent1[0]);
            let has_parent2_genes = child.iter().any(|&gene| gene == parent2[0]);
            let has_mutated_genes = child.iter().any(|&gene| gene != parent1[0] && gene != parent2[0]);

            assert!(has_parent1_genes, "Child should contain some genes from parent1");
            assert!(has_parent2_genes, "Child should contain some genes from parent2");
            assert!(has_mutated_genes, "Child should contain some mutated genes");
        }
    }

    #[test]
    fn mutates_coefficients() {
        let mut rng = ChaChaRng::seed_from_u64(101);
        let coefficient = Coefficient::from_f64(1000.0);
        let result = rng.mutate(coefficient, &raw_coefficient_range(DEFAULT_MUTATION_STEP));
        assert_ne!(result, coefficient);
        assert!(result >= Coefficient::from_f64(990.0) && result <= Coefficient::from_f64(1010.0));
    }
    
    #[test]
    fn mutates_negative_coefficients() {
        let mut rng = ChaChaRng::seed_from_u64(101);
        let coefficient = Coefficient::from_f64(-1000.0);
        let result = rng.mutate(coefficient, &raw_coefficient_range(DEFAULT_MUTATION_STEP));
        assert_ne!(result, coefficient);
        assert!(result >= Coefficient::from_f64(-1010.0) && result <= Coefficient::from_f64(-990.0));
    }

    #[test]
    fn mutation_can_completely_randomize_coefficients() {
        let mut rng = ChaChaRng::seed_from_u64(100); // <- this seed triggers the rule
        let coefficient = Coefficient::from_f64(1000.0);
        let result = rng.mutate(coefficient, &raw_coefficient_range(DEFAULT_MUTATION_STEP));
        assert_ne!(result, coefficient);
        assert!(result < Coefficient::from_f64(990.0) || result > Coefficient::from_f64(1010.0));
    }
}
