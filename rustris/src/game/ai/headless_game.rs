use crate::game::ai::action_evaluator::ActionEvaluator;
use crate::game::ai::agent::AiAgent;
use crate::game::random::{RandomMode, RandomTetromino, Seed, MIN_GARBAGE_PER_HOLE};
use crate::game::Game;
use engine::ai::{EndGame, GameResult};
use engine::game::{Game as _, GameEvent};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// how long a line clear animation holds the game for, matched to the sweep style
const LINE_CLEAR_DURATION: Duration = Duration::from_millis(750);

pub struct HeadlessGame {
    agent: AiAgent,
    game: Game,
    end_game: EndGame,
    options: HeadlessGameOptions,
    duration: Duration,
    game_over: bool,
    pieces: u32,
    tetris_lines: u32,
    /// lines counted from clear events: [Game::lines] saturates at [crate::game::MAX_LINES],
    /// which would stop [EndGame] line caps and the survival phase's line target ever being hit
    lines: u32,
}

impl HeadlessGame {
    pub fn new(
        rng: RandomTetromino,
        agent: AiAgent,
        options: HeadlessGameOptions,
        end_game: EndGame,
    ) -> Self {
        Self {
            agent,
            game: Game::new(0, rng),
            duration: Duration::ZERO,
            game_over: false,
            pieces: 0,
            tetris_lines: 0,
            lines: 0,
            options,
            end_game,
        }
    }

    pub fn play(&mut self) -> GameResult {
        loop {
            if let Some(result) = self.update() {
                return result;
            }
        }
    }

    fn update(&mut self) -> Option<GameResult> {
        self.duration += self.options.step;

        let result = self.result();
        if self.game_over || self.end_game.is_end_game(result, self.duration) {
            return Some(result);
        }

        self.agent.act(&mut self.game, self.options.step);
        let mut events = self.game.drain_events();
        self.game.update(self.options.step);
        events.extend(self.game.drain_events());

        for event in events {
            match event {
                GameEvent::GameOver => {
                    self.game_over = true;
                    return Some(self.result());
                }
                GameEvent::Clear { count, .. } => {
                    // simulate line clear animation
                    self.duration += self.options.line_clear_delay;
                    self.lines += count;
                    if count == 4 {
                        self.tetris_lines += count;
                    }
                }
                GameEvent::Spawn { .. } => {
                    self.pieces += 1;
                }
                _ => (),
            }
        }

        None
    }

    fn result(&self) -> GameResult {
        GameResult::new(
            self.game.score(),
            self.lines,
            self.game.level(),
            self.game_over,
            self.duration,
        )
        .with_pieces(self.pieces, self.tetris_lines)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HeadlessGameOptions {
    pub line_clear_delay: Duration,
    pub step: Duration,
    pub look_ahead: usize,
    pub record: bool,
}

pub const DEFAULT_LOOKAHEAD: usize = 0;

impl Default for HeadlessGameOptions {
    fn default() -> Self {
        Self {
            step: Duration::from_millis(16), // 60hz
            line_clear_delay: LINE_CLEAR_DURATION,
            look_ahead: DEFAULT_LOOKAHEAD,
            record: false,
        }
    }
}

pub struct HeadlessGameFixture {
    random_mode: RandomMode,
    seed: Seed,
    seeds_per_game: usize,
    game_options: HeadlessGameOptions,
    end_game: EndGame,
}

impl HeadlessGameFixture {
    pub fn new(
        random_mode: RandomMode,
        seed: Seed,
        game_options: HeadlessGameOptions,
        end_game: EndGame,
    ) -> Self {
        Self {
            random_mode,
            seed,
            seeds_per_game: 1,
            game_options,
            end_game,
        }
    }

    /// evaluate each genome over this many consecutive seeds and average the results
    pub fn with_seeds_per_game(mut self, seeds_per_game: usize) -> Self {
        self.set_seeds_per_game(seeds_per_game);
        self
    }

    pub fn set_seeds_per_game(&mut self, seeds_per_game: usize) {
        assert!(seeds_per_game > 0, "must play at least one seed per game");
        self.seeds_per_game = seeds_per_game;
    }

    pub fn seeds_per_game(&self) -> usize {
        self.seeds_per_game
    }

    pub fn set_end_game(&mut self, end_game: EndGame) {
        self.end_game = end_game;
    }

    pub fn end_game(&self) -> EndGame {
        self.end_game
    }

    /// advance to the next block of unused seeds
    pub fn next_seed(&mut self) {
        self.seed += Seed::from(self.seeds_per_game as u128);
    }

    pub fn current_seed(&self) -> Seed {
        self.seed
    }

    /// play one game per seed and return the averaged result
    pub fn play(&self, action_evaluate: ActionEvaluator) -> GameResult {
        let total: GameResult = (0..self.seeds_per_game as u128)
            .map(|i| self.play_seed(action_evaluate, self.seed + Seed::from(i)))
            .sum();
        total / self.seeds_per_game
    }

    pub fn play_seed(&self, action_evaluate: ActionEvaluator, seed: Seed) -> GameResult {
        let rng = RandomTetromino::new(self.random_mode, MIN_GARBAGE_PER_HOLE, seed);
        let mut agent = AiAgent::new(action_evaluate, self.game_options.look_ahead);
        if self.game_options.record {
            agent.start_recording().expect("Failed to start recording");
        }

        let mut game = HeadlessGame::new(rng, agent, self.game_options, self.end_game);
        let result = game.play();

        if self.game_options.record {
            let system_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            // TODO return this somehow
            let path = format!("game-{}.json", system_time.as_secs());
            game.agent
                .save_recording(path)
                .expect("Failed to save recording");
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ai::linear::LinearCoefficients;

    fn test_fixture() -> HeadlessGameFixture {
        HeadlessGameFixture::new(
            RandomMode::Bag,
            100.into(),
            HeadlessGameOptions::default(),
            EndGame::of_seconds(5),
        )
    }

    #[test]
    fn runs_headless_game() {
        let fixture = test_fixture();
        let evaluator = ActionEvaluator::Linear(LinearCoefficients::default());
        let result = fixture.play(evaluator);
        assert!(result.score() > 0);
    }

    #[test]
    fn line_cap_ends_the_game() {
        let fixture = HeadlessGameFixture::new(
            RandomMode::Bag,
            100.into(),
            HeadlessGameOptions::default(),
            EndGame::of_cleared(3),
        );
        let result = fixture.play(ActionEvaluator::Linear(LinearCoefficients::default()));
        assert!(result.cleared() >= 3);
        assert!(!result.game_over());
    }

    #[test]
    fn piece_cap_ends_the_game() {
        let fixture = HeadlessGameFixture::new(
            RandomMode::Bag,
            100.into(),
            HeadlessGameOptions::default(),
            EndGame::of_pieces(100),
        );
        let result = fixture.play(ActionEvaluator::Linear(LinearCoefficients::default()));
        assert_eq!(result.pieces(), 100);
        assert!(!result.game_over());
        assert!(result.cleared() > 0);
        assert!(result.bonus() <= result.cleared());
    }

    #[test]
    fn multi_seed_is_the_average_of_single_seeds() {
        let fixture = HeadlessGameFixture::new(
            RandomMode::Bag,
            100.into(),
            HeadlessGameOptions::default(),
            EndGame::of_pieces(50),
        )
        .with_seeds_per_game(2);
        let evaluator = ActionEvaluator::Linear(LinearCoefficients::default());
        let averaged = fixture.play(evaluator);
        let a = fixture.play_seed(evaluator, 100.into());
        let b = fixture.play_seed(evaluator, 101.into());
        assert_eq!(averaged, (a + b) / 2);
        assert_ne!(a, b);
    }

    #[test]
    fn same_score_for_the_same_inputs() {
        let fixture = test_fixture();
        let evaluator = ActionEvaluator::Linear(LinearCoefficients::default());
        let result1 = fixture.play(evaluator);
        let result2 = fixture.play(evaluator);
        assert_eq!(result1, result2);
    }
}
