use std::fmt::Debug;
use crate::game::ai::board_features::{BoardFeatures, BoardStats, StackStats};
use crate::game::ai::linear::LinearCoefficients;
use crate::game::ai::models::TetrisNeuralNetwork;
use engine::ai::Tensor;
use crate::game::board::Board;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ActionEvaluator {
    Linear(LinearCoefficients),
    NeuralNetwork(TetrisNeuralNetwork)
}

impl ActionEvaluator {
    pub fn evaluate_action(&self, board_before_action: &Board, stats_before_action: StackStats, board_after_action: Board) -> f64 {
        let stats = board_after_action.features_after_action(board_before_action, stats_before_action);
        match self {
            ActionEvaluator::Linear(coefficients) => Self::linear_score(coefficients, stats),
            ActionEvaluator::NeuralNetwork(network) => Self::neural_score(network, stats)
        }
    }

    fn linear_score(coefficients: &LinearCoefficients, stats: BoardStats) -> f64 {
        let delta = stats.delta();

        delta.open_holes() as f64 * coefficients.open_holes() +
            delta.closed_holes() as f64 * coefficients.closed_holes() +
            delta.max_height() as f64 * coefficients.max_stack_height() +
            delta.sum_roughness() as f64 * coefficients.sum_stack_roughness() +
            delta.max_roughness() as f64 * coefficients.max_stack_roughness() +
            stats.max_tetromino_y() as f64 * coefficients.max_tetromino_y() +
            delta.pillars() as f64 * coefficients.pillars() +
            match stats.cleared_lines() {
                1..=3 => stats.cleared_lines() as f64 * coefficients.line_clear(),
                4 => stats.cleared_lines() as f64 * coefficients.tetris_clear(),
                _ => 0.0
            }
    }
    
    fn neural_score(network: &TetrisNeuralNetwork, stats: BoardStats) -> f64 {
        let delta = stats.delta();
        let global = stats.global();

        let mut input_values = [0.0; 20];

        input_values[0] = delta.open_holes() as f64;
        input_values[1] = delta.closed_holes() as f64;
        input_values[2] = delta.max_height() as f64;
        input_values[3] = delta.min_height() as f64;
        input_values[4] = delta.sum_roughness() as f64;
        input_values[5] = delta.max_roughness() as f64;
        input_values[6] = delta.pillars() as f64;
        input_values[7] = delta.hole_cover() as f64;
        input_values[8] = delta.rhs_column_height() as f64;

        input_values[9] = global.open_holes() as f64;
        input_values[10] = global.closed_holes() as f64;
        input_values[11] = global.max_height() as f64;
        input_values[12] = global.min_height() as f64;
        input_values[13] = global.sum_roughness() as f64;
        input_values[14] = global.max_roughness() as f64;
        input_values[15] = global.pillars() as f64;
        input_values[16] = global.hole_cover() as f64;
        input_values[17] = global.rhs_column_height() as f64;

        input_values[18] = stats.max_tetromino_y() as f64;
        input_values[19] = stats.cleared_lines() as f64;

        let input = Tensor::vector(input_values);
        network.forward(&input).value()
    }
}
