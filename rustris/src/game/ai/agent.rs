use std::cmp::Ordering;
use itertools::Itertools;
use crate::game::ai::action_evaluator::ActionEvaluator;
use crate::game::ai::input_search::{InputSearch, InputSequenceResult};
use crate::game::ai::input_sequence::{InputSequence, Translation};
use crate::game::{Game, GameState};
use crate::game::ai::board_features::{BoardFeatures, StackStats};
use crate::game::ai::headless_game::DEFAULT_LOOKAHEAD;
use crate::game::ai::linear::LinearCoefficients;
use crate::game::ai::models::{self, TetrisNeuralNetwork};
use crate::game::board::Board;
use crate::game::tetromino::TetrominoShape;
use crate::game::ai::recording::{GamePlayback, GameRecording};
use std::path::Path;
use std::collections::VecDeque;
use std::time::Duration;

pub struct AiAgent {
    action_evaluate: ActionEvaluator,
    wait_sate: Option<AgentWaitState>,
    look_ahead: usize,
    /// Optional recording of agent decisions
    recording: Option<GameRecording>,
    /// Optional playback for replaying recorded decisions
    playback: Option<GamePlayback>,
    /// Minimum time between simulated key presses (zero = apply a whole decision instantly)
    key_delay: Duration,
    /// Time elapsed since the last simulated key press
    since_last_key: Duration,
    /// Keys that have been decided on but not yet pressed
    pending: VecDeque<Translation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum AgentWaitState {
    Spawn,
    SoftDrop(InputSequence),
    Alt(TetrominoShape, InputSequence),
}

impl AiAgent {
    pub fn new(action_evaluate: ActionEvaluator, look_ahead: usize) -> Self {
        Self {
            action_evaluate,
            wait_sate: None,
            look_ahead,
            recording: None,
            playback: None,
            key_delay: Duration::ZERO,
            since_last_key: Duration::MAX,
            pending: VecDeque::new(),
        }
    }

    /// Limit the agent to pressing at most one key per `key_delay`
    pub fn with_key_delay(mut self, key_delay: Duration) -> Self {
        self.key_delay = key_delay;
        self
    }
    
    pub fn default_linear() -> Self {
        Self::new(ActionEvaluator::Linear(LinearCoefficients::default()), DEFAULT_LOOKAHEAD)
    }
    
    pub fn default_neural() -> Self {
        Self::neural(models::tetris_clear_trained())
    }

    /// An agent playing the given trained network
    pub fn neural(network: TetrisNeuralNetwork) -> Self {
        Self::new(ActionEvaluator::NeuralNetwork(network), DEFAULT_LOOKAHEAD)
    }

    /// Queue the inputs to be pressed, then press as many as the key delay allows
    fn apply_inputs(&mut self, game: &mut Game, inputs: &InputSequence) {
        self.pending = inputs.translations().iter().copied().collect();
        self.press_pending(game);
    }

    fn can_press(&self) -> bool {
        self.since_last_key >= self.key_delay
    }

    /// Press pending keys, one per key delay
    fn press_pending(&mut self, game: &mut Game) {
        while !self.pending.is_empty() && self.can_press() {
            if !matches!(game.state, GameState::Fall(_) | GameState::Lock(_)) {
                // the piece locked before we finished, abandon the remaining keys
                self.pending.clear();
                self.wait_sate = Some(AgentWaitState::Spawn);
                return;
            }

            let translation = self.pending.pop_front().unwrap();
            self.since_last_key = Duration::ZERO;
            match translation {
                Translation::Left => { game.left(); }
                Translation::Right => { game.right(); }
                Translation::RotateClockwise => { game.rotate(true); }
                Translation::RotateAnticlockwise => { game.rotate(false); }
                Translation::HardDrop => { game.hard_drop(); }
                Translation::SoftDrop => {
                    game.set_soft_drop(true);
                    let after_soft_drop = InputSequence::new(self.pending.drain(..).collect());
                    self.wait_sate = Some(AgentWaitState::SoftDrop(after_soft_drop));
                    return;
                }
            }
        }

        if self.pending.is_empty() {
            self.wait_sate = Some(AgentWaitState::Spawn);
        }
    }

    pub fn reset(&mut self) {
        self.wait_sate = None;
        self.pending.clear();
        self.since_last_key = Duration::MAX;
    }

    pub fn act(&mut self, game: &mut Game, delta: Duration) {
        self.since_last_key = self.since_last_key.saturating_add(delta);

        if !self.pending.is_empty() {
            self.press_pending(game);
            return;
        }

        // Handle wait states (this works for both playback and AI modes)
        if let Some(wait_state) = self.wait_sate.clone() {
            match wait_state {
                AgentWaitState::Spawn => {
                    if matches!(game.state, GameState::Spawn(_, _)) {
                        self.wait_sate = None;
                    }
                }
                AgentWaitState::SoftDrop(post_soft_drop_inputs)  => {
                    match game.state {
                        GameState::Fall(_) => {
                            // continue soft dropping until a lock
                            game.set_soft_drop(true);
                            return;
                        }
                        GameState::Lock(_) => {
                            // if we are in a lock state, we can apply the soft drop inputs
                            self.wait_sate = None;
                            self.apply_inputs(game, &post_soft_drop_inputs);
                        }
                        _ => (),
                    }
                }
                AgentWaitState::Alt(alt_shape, alt_inputs) => {
                    if matches!(game.state, GameState::Fall(_)) {
                        if let Some(shape) = game.board.tetromino().map(|t| t.shape()) {
                            if shape == alt_shape {
                                // we are in the alt state, apply the inputs
                                self.wait_sate = None;
                                self.apply_inputs(game, &alt_inputs);
                            }
                        }
                    }
                }
            }
            return; // wait for wait state to be resolved
        }

        if !matches!(game.state, GameState::Fall(_)) {
            return; // only act when in a fall state
        }

        if !self.can_press() {
            return; // still "thinking"
        }
        
        if let Some(shape) = game.board.tetromino().map(|t| t.shape()) {
            let (best_inputs, is_alt) = if let Some(playback) = &mut self.playback {
                // Playback mode: get the recorded decision
                if let Some(recorded_input) = playback.next_decision() {
                    match recorded_input.keys {
                        Some(input_sequence) => (input_sequence, recorded_input.is_alt),
                        None => {
                            // This was a null decision - do nothing
                            return;
                        }
                    }
                } else {
                    // Playback finished
                    return;
                }
            } else {
                // Normal AI decision-making when not in playback mode
                let best_result = self.best_move(game, shape, &game.random.peek_buffer());

                let (alt_next_shape, alt_next_peek) = game.hold
                    .map(|state| (state.piece, 0..))
                    .unwrap_or_else(|| (game.random.peek(), 1..));

                let alt_best_move = self.best_move(game, alt_next_shape, &game.random.peek_buffer()[alt_next_peek]);
                let Some((best_inputs, is_alt)) = Self::choose(best_result, alt_best_move) else {
                    // Record a null decision if no moves are possible
                    if let Some(recording) = &mut self.recording {
                        recording.record_null_decision();
                    }
                    return;
                };

                // Record the decision we're making (only in AI mode)
                if let Some(recording) = &mut self.recording {
                    recording.record_decision(best_inputs.clone(), is_alt);
                }

                (best_inputs, is_alt)
            };

            // Execute the decision (same code path for both playback and AI)
            if is_alt {
                let alt_next_shape = game.hold
                    .map(|state| state.piece)
                    .unwrap_or_else(|| game.random.peek());

                // hold the current and wait for the alt shape to fall
                self.wait_sate = Some(AgentWaitState::Alt(alt_next_shape, best_inputs));
                self.since_last_key = Duration::ZERO;
                game.hold();
            } else {
                self.apply_inputs(game, &best_inputs);
            }
        } else {
            unreachable!("the game should nver be in a fall state without a tetromino");
        }
    }
    
    /// Choose between the best move for the current piece and the best move for the held/next
    /// piece (reached via a hold). Scores are "higher is better", matching [`Self::best_single_move`].
    /// The current piece wins ties, so the agent only holds when it is strictly better.
    fn choose(current: Option<(InputSequence, f64)>, alt: Option<(InputSequence, f64)>) -> Option<(InputSequence, bool)> {
        match (current, alt) {
            (None, None) => None,
            (Some((m, _)), None) => Some((m, false)),
            (None, Some((m, _))) => Some((m, true)),
            (Some((m1, c1)), Some((m2, c2))) =>
                if c2 > c1 { Some((m2, true)) } else { Some((m1, false)) }
        }
    }

    fn best_move(&self, game: &Game, shape: TetrominoShape, peek: &[TetrominoShape]) -> Option<(InputSequence, f64)> {
        // Normal AI decision-making (playback is now handled at the act level)
        self.best_single_move(game.board, game.board.stack_stats(), shape)
            .map(|(result, cost)| (result.inputs().clone(), cost))
    }

    fn best_single_move(&self, board_from: Board, stack_stats_before: StackStats, shape: TetrominoShape) -> Option<(InputSequenceResult, f64)> {
        board_from.search_all_inputs(shape)
            .into_iter()
            .map(|r| {
                let score = self.action_evaluate.evaluate_action(&board_from, stack_stats_before, r.board());
                (r, score)
            })
            .max_by(|m1, m2| self.compare_moves(m1, m2))
    }

    fn compare_moves(&self, (result1, cost1): &(InputSequenceResult, f64), (result2, cost2): &(InputSequenceResult, f64)) -> Ordering {
        // if multiple moves have teh same score then we must order them to deterministically choose
        cost1.total_cmp(cost2).then_with(|| result1.inputs().cmp(&result2.inputs()))
    }

    /// Start recording agent decisions
    pub fn start_recording(&mut self) -> Result<(), String> {
        if self.playback.is_some() {
            Err("Cannot start recording while in playback mode".to_string())
        } else {
            self.recording = Some(GameRecording::new());
            Ok(())
        }
    }

    /// Save the current recording to a file
    pub fn save_recording<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        if let Some(recording) = &self.recording {
            recording.save_to_file(path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Start playback from a file
    pub fn start_playback<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        if self.recording.is_some() {
            Err("Cannot start playback while in recording mode".to_string())
        } else {
            self.playback = Some(GamePlayback::load_from_file(path).map_err(|e| e.to_string())?);
            Ok(())
        }
    }
}

impl Default for AiAgent {
    fn default() -> Self {
        Self::default_neural()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::random::{RandomMode, RandomTetromino};

    fn falling_game() -> Game {
        let mut game = Game::new(0, RandomTetromino::new(RandomMode::Bag, 10, 100.into()));
        for _ in 0..1000 {
            if matches!(game.state, GameState::Fall(_)) {
                return game;
            }
            game.update(Duration::from_millis(10));
        }
        panic!("game never reached a fall state");
    }

    fn agent(key_delay: Duration) -> AiAgent {
        AiAgent::default_linear().with_key_delay(key_delay)
    }

    #[test]
    fn instant_agent_applies_a_whole_decision_in_one_act() {
        let mut game = falling_game();
        let mut agent = agent(Duration::ZERO);
        agent.act(&mut game, Duration::ZERO);
        assert!(agent.pending.is_empty());
        assert!(agent.wait_sate.is_some(), "agent should be waiting for the next piece");
    }

    #[test]
    fn paced_agent_presses_one_key_per_delay() {
        let mut game = falling_game();
        let mut agent = agent(Duration::from_millis(100));
        agent.pending = VecDeque::from(vec![Translation::Left, Translation::Left, Translation::HardDrop]);

        agent.act(&mut game, Duration::ZERO); // since_last_key starts at max so the first key is pressed
        assert_eq!(agent.pending.len(), 2);

        agent.act(&mut game, Duration::from_millis(50));
        assert_eq!(agent.pending.len(), 2);

        agent.act(&mut game, Duration::from_millis(50));
        assert_eq!(agent.pending.len(), 1);

        agent.act(&mut game, Duration::from_millis(100));
        assert!(agent.pending.is_empty());
        assert_eq!(agent.wait_sate, Some(AgentWaitState::Spawn));
    }

    #[test]
    fn paced_agent_waits_before_deciding() {
        let mut game = falling_game();
        let mut agent = agent(Duration::from_millis(100));
        agent.since_last_key = Duration::ZERO;

        agent.act(&mut game, Duration::from_millis(50));
        assert!(agent.pending.is_empty());
        assert_eq!(agent.wait_sate, None, "should not have decided yet");

        agent.act(&mut game, Duration::from_millis(50));
        assert!(agent.wait_sate.is_some() || !agent.pending.is_empty(), "should have decided");
    }

    #[test]
    fn paced_agent_plays_a_game() {
        // simulate a "hard" ai for a minute of game time at 60hz
        let mut game = Game::new(0, RandomTetromino::new(RandomMode::Bag, 10, 7.into()));
        let mut agent = agent(crate::game::rules::AiDifficulty::HARD_KEY_DELAY);
        let step = Duration::from_millis(16);
        let mut pieces = 0;
        'play: for _ in 0..(60 * 1000 / 16) {
            agent.act(&mut game, step);
            game.update(step);
            for event in engine::game::Game::drain_events(&mut game) {
                match event {
                    engine::game::GameEvent::GameOver => break 'play,
                    engine::game::GameEvent::Spawn { .. } => pieces += 1,
                    _ => {}
                }
            }
        }
        assert!(pieces > 10, "the agent should have placed pieces, placed {}", pieces);
        assert!(game.lines() > 0, "the agent should have cleared lines");
    }

    #[test]
    fn holds_only_when_the_alternative_scores_higher() {
        let current = InputSequence::new(vec![Translation::HardDrop]);
        let alt = InputSequence::new(vec![Translation::Left, Translation::HardDrop]);
        let choose = |c: f64, a: f64| AiAgent::choose(Some((current.clone(), c)), Some((alt.clone(), a))).unwrap();
        assert_eq!(choose(1.0, 2.0), (alt.clone(), true));
        assert_eq!(choose(2.0, 1.0), (current.clone(), false));
        assert_eq!(choose(1.0, 1.0), (current.clone(), false), "ties go to the current piece");
        assert_eq!(AiAgent::choose(None, Some((alt.clone(), 0.0))), Some((alt.clone(), true)));
        assert_eq!(AiAgent::choose(Some((current.clone(), 0.0)), None), Some((current.clone(), false)));
        assert_eq!(AiAgent::choose(None, None), None);
    }

    #[test]
    fn reset_clears_pending_keys() {
        let mut agent = agent(Duration::from_millis(100));
        agent.pending = VecDeque::from(vec![Translation::Left]);
        agent.wait_sate = Some(AgentWaitState::Spawn);
        agent.reset();
        assert!(agent.pending.is_empty());
        assert_eq!(agent.wait_sate, None);
        assert!(agent.can_press());
    }
}
