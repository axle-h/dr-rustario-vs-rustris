//! The genetic algorithm: generic over genome size and over how a genome is scored.
//! Each game supplies a [Fitness] that plays its own headless game.

use crate::ai::end_game::EndGame;
use crate::ai::game_result::GameResult;
use crate::ai::generation_record::GenerationRecord;
use crate::ai::generation_stats::GenerationStatistics;
use crate::ai::genome::Genome;
use crate::ai::mutation::GenomeMutation;
use crate::ai::objective::{Objective, Phase};
use crate::ai::organism::Organism;
use crate::ai::seed::Seed;
use rayon::prelude::*;
use std::time::{Duration, Instant};

/// how a genome is turned into a score: play it and report how the game went.
/// This is the whole game-specific half of training.
pub trait Fitness<const GENOME: usize>: Send + Sync {
    /// play `genome` over the current block of seeds and return the averaged result
    fn evaluate(&self, genome: &Genome<GENOME>) -> GameResult;

    /// advance to the next block of unused seeds, so elites cannot overfit one seed
    fn next_seed(&mut self);

    /// the seed block the last generation played
    fn current_seed(&self) -> Seed;

    fn seeds_per_game(&self) -> usize;

    fn set_seeds_per_game(&mut self, seeds_per_game: usize);

    fn set_end_game(&mut self, end_game: EndGame);
}

#[derive(Debug, Clone, Copy)]
pub struct HyperParameters {
    population_size: usize,
    elite_count: usize,    // elites are passed onto the next generation unchanged
    survivor_count: usize, // only survivors are selected to breed
    parent_count: usize, // the number of breeding pairs each generation, the parents are selected from the surviving population weighted by fitness
}

impl HyperParameters {
    pub fn new(population_size: usize, elite_rate: f64, survival_rate: f64) -> Self {
        fn rate_to_count(population_size: usize, rate: f64) -> usize {
            assert!(
                rate >= 0.0 && rate <= 1.0,
                "rates must be between 0.0 and 1.0"
            );
            (population_size as f64 * rate) as usize
        }

        let elite_count = rate_to_count(population_size, elite_rate);
        let survivor_count = rate_to_count(population_size, survival_rate);

        assert!(
            elite_count + survivor_count < population_size,
            "too many elites and survivors"
        );
        assert!(survivor_count >= 2, "need at least two survivors to breed");

        Self {
            population_size,
            elite_count,
            survivor_count,
            parent_count: ((population_size as f64 - elite_count as f64) / 2.0).ceil() as usize,
        }
    }
}

impl Default for HyperParameters {
    fn default() -> Self {
        Self::new(1000, 0.005, 0.5)
    }
}

pub struct GeneticAlgorithm<const GENOME: usize, F: Fitness<GENOME>> {
    population: Vec<Organism<GENOME>>,
    generations: Vec<GenerationStatistics<GENOME>>,
    fitness: F,
    mutation: GenomeMutation<GENOME>,
    hyper_parameters: HyperParameters,
    phases: Vec<Phase>,
    phase_index: usize,
    phase_generations: usize,
}

impl<const N: usize, F: Fitness<N>> GeneticAlgorithm<N, F> {
    /// `phases` are run in order; a phase ends when it is complete (see [Phase::is_complete]) or has run
    /// for its `max_generations`, the best member is then used to seed the population of the next phase.
    pub fn new(
        mut fitness: F,
        mut mutation: GenomeMutation<N>,
        hyper_parameters: HyperParameters,
        phases: Vec<Phase>,
        population_seed: Option<Genome<N>>,
    ) -> Self {
        assert!(!phases.is_empty(), "at least one phase is required");
        Self::apply_phase(&phases[0], &mut fitness, &mut mutation);

        let population =
            Self::initial_population(&hyper_parameters, &mut mutation, population_seed);

        Self {
            population,
            generations: vec![],
            fitness,
            mutation,
            hyper_parameters,
            phases,
            phase_index: 0,
            phase_generations: 0,
        }
    }

    fn apply_phase(phase: &Phase, fitness: &mut F, mutation: &mut GenomeMutation<N>) {
        fitness.set_end_game(phase.end_game);
        fitness.set_seeds_per_game(phase.seeds_per_game);
        mutation.set_rates(
            phase.mutation_rate.clone(),
            phase.crossover_rate.clone(),
            phase.mutation_step,
        );
    }

    /// a seeded population keeps one pristine copy of the seed, the rest are mutations of it
    fn initial_population(
        hyper_parameters: &HyperParameters,
        mutation: &mut GenomeMutation<N>,
        population_seed: Option<Genome<N>>,
    ) -> Vec<Organism<N>> {
        let mut population = Vec::with_capacity(hyper_parameters.population_size);
        for i in 0..hyper_parameters.population_size {
            let genome = match population_seed {
                Some(seed) if i == 0 => seed,
                Some(seed) => mutation.mutate(seed),
                None => mutation.random(),
            };
            population.push(Organism::new(genome));
        }
        population
    }

    pub fn phase(&self) -> &Phase {
        &self.phases[self.phase_index]
    }

    pub fn objective(&self) -> Objective {
        self.phase().objective
    }

    pub fn fitness(&self) -> &F {
        &self.fitness
    }

    pub fn population(&self) -> &[Organism<N>] {
        &self.population
    }

    pub fn run(&mut self) -> GenerationStatistics<N> {
        self.run_confirmed(|_| true)
    }

    /// The same, asking `confirm` whether a phase that has met its own completion test really is
    /// finished. Answering false carries the *same population* on to another generation rather
    /// than ending the phase: a caller that wants to check a candidate against something the
    /// phase cannot see - seeds it has never played, say - can do that without the search
    /// having to start again from one genome, which would throw away every other line it has
    /// found.
    pub fn run_confirmed(
        &mut self,
        mut confirm: impl FnMut(&GenerationStatistics<N>) -> bool,
    ) -> GenerationStatistics<N> {
        let mut record = GenerationRecord::new().expect("Failed to create generation record");
        println!(
            "{} phase, population {}, records in {}",
            self.objective(),
            self.hyper_parameters.population_size,
            record.path().display()
        );

        loop {
            let stats = self.evolve();
            println!("{}", stats);
            record
                .add(&stats)
                .expect("Failed to write to generation record");

            let complete = self.phase().is_complete(&stats.max().result()) && confirm(&stats);
            let phase_over = complete || self.phase_generations >= self.phase().max_generations;
            if !phase_over {
                self.next_generation();
                continue;
            }

            if self.phase_index + 1 >= self.phases.len() {
                return stats;
            }

            self.next_phase(stats.max().genome());
        }
    }

    /// switch to the next phase, re-seeding the population from `best`
    fn next_phase(&mut self, best: Genome<N>) {
        self.phase_index += 1;
        self.phase_generations = 0;
        let phase = self.phases[self.phase_index].clone();
        println!(
            "{} phase complete after generation {}, switching to {} phase",
            self.phases[self.phase_index - 1].objective,
            self.generations.len(),
            phase.objective
        );
        Self::apply_phase(&phase, &mut self.fitness, &mut self.mutation);
        self.population =
            Self::initial_population(&self.hyper_parameters, &mut self.mutation, Some(best));
    }

    /// evaluate the population on fresh seeds and sort it best first, but do not breed
    fn evolve(&mut self) -> GenerationStatistics<N> {
        let objective = self.objective();

        // every generation plays new piece sequences, elites included, so nothing can overfit one seed
        self.fitness.next_seed();
        self.population.iter_mut().for_each(Organism::unset_result);

        // Calculate fitness in parallel
        let generation_start = Instant::now();
        self.population.par_iter_mut().for_each(|member| {
            member.set_result(|genome| self.fitness.evaluate(genome));
        });
        self.population
            .sort_by(|s1, s2| objective.cmp(&s2.result(), &s1.result()));
        let generation_duration = generation_start.elapsed();

        // Calculate total gameplay time
        let total_gameplay_time: Duration = self
            .population
            .iter()
            .map(|organism| organism.result().time() * self.fitness.seeds_per_game() as u32)
            .sum();

        // Calculate game seconds per real second
        let game_seconds_per_second = if generation_duration.as_secs_f64() > 0.0 {
            total_gameplay_time.as_secs_f64() / generation_duration.as_secs_f64()
        } else {
            0.0 // Avoid division by zero
        };

        let p95_index = (self.hyper_parameters.population_size as f64 * 0.05).floor() as usize;
        let p50_index = self.hyper_parameters.population_size / 2;
        let stats = GenerationStatistics::new(
            self.generations.len() + 1,
            objective,
            self.fitness.current_seed(),
            self.population[0],
            self.population[p95_index],
            self.population[p50_index],
            self.mutation.current_mutation_rate(),
            self.mutation.current_crossover_rate(),
            total_gameplay_time,
            generation_duration,
            game_seconds_per_second,
        );
        self.generations.push(stats);
        self.phase_generations += 1;
        self.mutation.add_sample(stats);

        stats
    }

    fn next_generation(&mut self) {
        let objective = self.objective();
        let surviving_population: Vec<_> = self
            .population
            .iter()
            .take(self.hyper_parameters.survivor_count)
            .copied()
            .collect();

        self.population.clear();

        for elite in surviving_population
            .iter()
            .take(self.hyper_parameters.elite_count)
        {
            self.population.push(*elite);
        }

        let parents = self.mutation.parents(
            &surviving_population,
            self.hyper_parameters.parent_count,
            objective,
        );

        let mut required_children = self.hyper_parameters.population_size - self.population.len();
        while required_children > 0 {
            for [parent1, parent2] in parents.iter() {
                let [child1, child2] = self
                    .mutation
                    .crossover(*parent1, *parent2)
                    .map(Organism::new);
                self.population.push(child1);
                required_children -= 1;

                if required_children > 0 {
                    self.population.push(child2);
                    required_children -= 1;
                }

                if required_children == 0 {
                    break;
                }
            }
        }
    }
}
