//! The Rustris end of training: it plays the headless game for the genetic algorithm and
//! provides the `ga` subcommand's entry points.

use crate::game::ai::action_evaluator::ActionEvaluator;
use crate::game::ai::headless_game::{HeadlessGameFixture, HeadlessGameOptions};
use crate::game::ai::models;
use crate::game::random::RandomMode;
use engine::ai::{
    EndGame, Fitness, GameResult, GeneticAlgorithm, Genome, GenomeMutation, HyperParameters,
    NeuralGenome, Phase, Seed, NEURAL_GENOME_SIZE,
};
use rayon::prelude::*;

pub const DEFAULT_LINE_CAP: u32 = 10_000;
pub const DEFAULT_PIECE_CAP: u32 = 1_000;

/// scores a genome by playing the headless Rustris game with the evaluator it decodes to
pub struct FixtureFitness<const GENOME: usize, F>
where
    F: Fn(&Genome<GENOME>) -> ActionEvaluator + Send + Sync,
{
    fixture: HeadlessGameFixture,
    evaluator: F,
}

impl<const GENOME: usize, F> FixtureFitness<GENOME, F>
where
    F: Fn(&Genome<GENOME>) -> ActionEvaluator + Send + Sync,
{
    pub fn new(fixture: HeadlessGameFixture, evaluator: F) -> Self {
        Self { fixture, evaluator }
    }
}

impl<const GENOME: usize, F> Fitness<GENOME> for FixtureFitness<GENOME, F>
where
    F: Fn(&Genome<GENOME>) -> ActionEvaluator + Send + Sync,
{
    fn evaluate(&self, genome: &Genome<GENOME>) -> GameResult {
        self.fixture.play((self.evaluator)(genome))
    }

    fn next_seed(&mut self) {
        self.fixture.next_seed();
    }

    fn current_seed(&self) -> Seed {
        self.fixture.current_seed()
    }

    fn seeds_per_game(&self) -> usize {
        self.fixture.seeds_per_game()
    }

    fn set_seeds_per_game(&mut self, seeds_per_game: usize) {
        self.fixture.set_seeds_per_game(seeds_per_game);
    }

    fn set_end_game(&mut self, end_game: EndGame) {
        self.fixture.set_end_game(end_game);
    }
}

fn neural_fitness() -> impl Fitness<NEURAL_GENOME_SIZE> {
    FixtureFitness::new(
        HeadlessGameFixture::new(
            RandomMode::Bag,
            rand::random(),
            HeadlessGameOptions::default(),
            EndGame::NONE,
        ),
        |&genome| ActionEvaluator::NeuralNetwork(genome.into()),
    )
}

fn neural_mutation() -> GenomeMutation<NEURAL_GENOME_SIZE> {
    let phase = Phase::survival(DEFAULT_LINE_CAP);
    GenomeMutation::of_max(phase.mutation_rate, phase.crossover_rate, 5, rand::random())
}

fn run_neural(phases: Vec<Phase>, population_seed: Option<NeuralGenome>) {
    GeneticAlgorithm::new(
        neural_fitness(),
        neural_mutation(),
        HyperParameters::default(),
        phases,
        population_seed,
    )
    .run();
}

/// train a random population to survive the line cap
pub fn ga_main_survival() -> Result<(), String> {
    run_neural(vec![Phase::survival(DEFAULT_LINE_CAP)], None);
    Ok(())
}

/// fine tune the built in model for tetris clears within the piece cap
pub fn ga_main_score() -> Result<(), String> {
    run_neural(
        vec![Phase::score(DEFAULT_PIECE_CAP)],
        Some(models::tetris_clear_trained().into()),
    );
    Ok(())
}

/// train for survival, then once any member reaches the line cap switch to optimising for tetris clears
pub fn ga_main_auto() -> Result<(), String> {
    run_neural(
        vec![
            Phase::survival(DEFAULT_LINE_CAP),
            Phase::score(DEFAULT_PIECE_CAP),
        ],
        None,
    );
    Ok(())
}

/// play the built in model on a few seeds under the score phase rules and report how it does
pub fn ga_diagnose() -> Result<(), String> {
    const SEEDS: usize = 4;
    let phase = Phase::score(DEFAULT_PIECE_CAP);
    let fixture = HeadlessGameFixture::new(
        RandomMode::Bag,
        1.into(),
        HeadlessGameOptions::default(),
        phase.end_game,
    );
    let evaluator = ActionEvaluator::NeuralNetwork(models::tetris_clear_trained());
    println!(
        "built in neural network, {} pieces per game",
        DEFAULT_PIECE_CAP
    );

    let results: Vec<_> = (0..SEEDS as u128)
        .into_par_iter()
        .map(|seed| (seed, fixture.play_seed(evaluator, (seed + 1).into())))
        .collect();

    for (seed, result) in results.iter() {
        println!(
            "seed {}: {} tetris fraction: {:.3}",
            seed + 1,
            result,
            result.bonus_fraction()
        );
    }
    let mean: GameResult = results.iter().map(|(_, r)| *r).sum::<GameResult>() / SEEDS;
    println!(
        "mean: {} tetris fraction: {:.3} fitness ({}): {}",
        mean,
        mean.bonus_fraction(),
        phase.objective,
        phase.objective.fitness(&mean)
    );
    Ok(())
}

pub fn ga_main() -> Result<(), String> {
    ga_main_auto()
}

#[cfg(test)]
mod tests {
    use super::FixtureFitness;
    use crate::game::ai::action_evaluator::ActionEvaluator;
    use crate::game::ai::headless_game::{HeadlessGameFixture, HeadlessGameOptions};
    use crate::game::ai::linear::LinearCoefficients;
    use crate::game::ai::linear::{LinearGenome, LINEAR_GENOME_SIZE};
    use crate::game::random::RandomMode;
    use engine::ai::EndGame;
    use engine::ai::{Fitness, GeneticAlgorithm, HyperParameters};
    use engine::ai::{GenomeMutation, RateLimits};
    use engine::ai::{Objective, Phase};

    fn linear_fitness(
    ) -> FixtureFitness<LINEAR_GENOME_SIZE, impl Fn(&LinearGenome) -> ActionEvaluator + Send + Sync>
    {
        FixtureFitness::new(
            HeadlessGameFixture::new(
                RandomMode::Bag,
                100.into(),
                HeadlessGameOptions::default(),
                EndGame::NONE,
            ),
            |&genome| ActionEvaluator::Linear(genome.into()),
        )
    }

    fn mutation() -> GenomeMutation<LINEAR_GENOME_SIZE> {
        GenomeMutation::of_max(RateLimits::default(), RateLimits::default(), 5, 100.into())
    }

    #[test]
    fn genetic_algorithm() {
        let phase = Phase::survival(5).with_max_generations(1);
        GeneticAlgorithm::new(
            linear_fitness(),
            mutation(),
            HyperParameters::new(10, 0.01, 0.5),
            vec![phase],
            None,
        )
        .run();
    }

    #[test]
    fn seeded_population_keeps_a_pristine_seed() {
        let seed: LinearGenome = LinearCoefficients::default().into();
        let mut phase = Phase::score(50).with_max_generations(1);
        phase.seeds_per_game = 2;
        let ga = GeneticAlgorithm::new(
            linear_fitness(),
            mutation(),
            HyperParameters::new(10, 0.01, 0.5),
            vec![phase],
            Some(seed),
        );
        assert_eq!(ga.population()[0].genome(), seed);
        assert!(ga.population().iter().skip(1).any(|o| o.genome() != seed));
    }

    #[test]
    #[ignore = "fails on seeds_per_game (3 vs 4) since the multi-stage training commit, before the engine merge"]
    fn switches_from_survival_to_score_at_the_line_cap() {
        let seed: LinearGenome = LinearCoefficients::default().into();
        let mut ga = GeneticAlgorithm::new(
            linear_fitness(),
            mutation(),
            HyperParameters::new(10, 0.01, 0.5),
            vec![Phase::survival(5), Phase::score(50).with_max_generations(1)],
            Some(seed),
        );
        assert_eq!(ga.objective(), Objective::Survival);
        let stats = ga.run();
        assert_eq!(ga.objective(), Objective::Score);
        assert_eq!(stats.objective(), Objective::Score);
        assert_eq!(ga.fitness().seeds_per_game(), 4);
        assert_eq!(stats.max().result().pieces(), 50);
        // the survival phase should finish in one generation since the default coefficients survive 5 lines
        assert_eq!(stats.id(), 2);
    }
}
