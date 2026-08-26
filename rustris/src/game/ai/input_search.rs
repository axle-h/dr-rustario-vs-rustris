use crate::game::ai::apply_inputs::ApplyInputs;
use crate::game::ai::input_sequence::Translation::SoftDrop;
use crate::game::ai::input_sequence::{InputSequence, ResolvedInputSequence, Translation};
use crate::game::board::Board;
use crate::game::geometry::Pose;
use crate::game::tetromino::{Minos, TetrominoShape};
use itertools::Itertools;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use Translation::{HardDrop, Left, Right, RotateAnticlockwise, RotateClockwise};

pub trait InputSearch {
    fn search_all_inputs(self, shape: TetrominoShape) -> Vec<InputSequenceResult>;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InputSequenceResult {
    inputs: InputSequence,
    board: Board,
    minos: Minos,
}

impl PartialOrd for InputSequenceResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.inputs.partial_cmp(&other.inputs)
    }
}

impl Ord for InputSequenceResult {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inputs.cmp(&other.inputs)
    }
}

impl InputSequenceResult {
    pub fn inputs(&self) -> &InputSequence {
        &self.inputs
    }

    pub fn board(&self) -> Board {
        self.board
    }

    pub fn board_mut(&mut self) -> &mut Board {
        &mut self.board
    }

    pub fn minos(&self) -> Minos {
        self.minos
    }
}

struct Search {
    board: Board,
    visited: HashSet<Pose>,
    results: HashMap<Minos, InputSequenceResult>,
    visit_queue: VecDeque<InputSequence>,
}

impl Search {
    fn new(board: Board) -> Self {
        Self {
            board,
            visited: HashSet::new(),
            results: HashMap::new(),
            visit_queue: VecDeque::new(),
        }
    }

    pub fn search_after_drop(&mut self, drop_type: Translation) {
        self.visit_queue.push_back(InputSequence::empty());
        while let Some(inputs) = self.visit_queue.pop_front() {
            let mut current_board = self.board;
            if !current_board.apply_inputs(&inputs) {
                continue;
            }
            // dedupe on the pose actually reached on the board (after wall kicks), not on the
            // pose a translation would geometrically produce: two different poses can share a
            // geometric neighbour but kick to different places, so pruning on geometry loses
            // reachable placements.
            let pose = current_board.tetromino().unwrap().pose();
            if !self.visited.insert(pose) {
                continue;
            }
            self.next_translations(&inputs);

            // soft drop is emulated with hard drop on a model-less board
            let inputs = if current_board.hard_drop().is_some() {
                inputs.clone() + drop_type
            } else {
                inputs.clone()
            };
            self.try_insert_result(&current_board, &inputs);
        }
    }

    pub fn search_translations_from_current_results(&mut self) {
        // `results` is a HashMap with randomised iteration order. The order in which poses are
        // expanded decides which input sequence first claims a pose in `visited`, so the seeds
        // must be expanded in a deterministic order or the chosen inputs (and via the tie-break
        // in the agent, the chosen placement) change from run to run.
        let seeds: Vec<InputSequence> = self
            .results
            .values()
            .map(|result| {
                let mut inputs = result.inputs.clone();
                if inputs.pop_drop().is_some() {
                    inputs.push(SoftDrop);
                }
                inputs
            })
            .sorted()
            .collect();

        for inputs in seeds {
            self.next_translations(&inputs);
        }
    }

    pub fn into_results(self) -> Vec<InputSequenceResult> {
        let mut values: Vec<InputSequenceResult> = self.results.into_values().sorted().collect();
        for result in values.iter_mut() {
            result.board_mut().lock();
        }
        values
    }

    fn try_insert_result(&mut self, board: &Board, inputs: &InputSequence) {
        let mut minos = board.tetromino().unwrap().minos();
        minos.sort(); // keying by sorted minos disregards mirrored Poses

        let existing = self.results.get(&minos);
        let should_set = if let Some(existing) = existing {
            inputs < &existing.inputs
        } else {
            true
        };
        if should_set {
            self.results.insert(
                minos,
                InputSequenceResult {
                    board: *board,
                    inputs: inputs.clone(),
                    minos,
                },
            );
        }
    }

    fn next_translations(&mut self, inputs: &InputSequence) {
        for next_translation in [Left, Right, RotateClockwise, RotateAnticlockwise] {
            self.visit_queue
                .push_back(inputs.clone() + next_translation);
        }
    }
}

impl InputSearch for Board {
    fn search_all_inputs(mut self, shape: TetrominoShape) -> Vec<InputSequenceResult> {
        if let Some(current_shape) = self.tetromino().map(|t| t.shape()) {
            if current_shape != shape {
                // searching for a different shape than the current one, e.g. a hold, so clear it
                self.clear_tetromino();
            }
        }

        if self.tetromino().is_none() && self.try_spawn_tetromino(shape).is_none() {
            // cannot even spawn the shape, no possible inputs
            return vec![];
        }

        let mut search = Search::new(self);
        search.search_after_drop(HardDrop);
        search.search_translations_from_current_results();
        search.search_after_drop(SoftDrop);
        search.into_results()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::block::BlockState;
    use crate::game::geometry::Rotation;

    #[test]
    fn searches() {
        let board = Board::new();
        let inputs = board.search_all_inputs(TetrominoShape::I);
        assert_eq!(inputs.len(), 17); // 7 flat positions + 10 upright positions
    }

    #[test]
    fn searches_with_open_hole() {
        let mut board = Board::new();
        board.set_block(
            (0, 1),
            BlockState::Stack(TetrominoShape::I, Rotation::North, 0),
        );
        let inputs = board.search_all_inputs(TetrominoShape::I);
        assert_eq!(inputs.len(), 18); // 7 flat positions + 10 upright positions + 1 open hole position
    }

    /// Regression: on this board (from a real game) the S tuck into the gap at the bottom is reachable
    /// by several equally-ranked soft drop sequences, and the search used to return whichever one the
    /// random HashMap iteration order happened to find first, making AI games non-reproducible.
    #[test]
    fn search_is_deterministic_for_tied_soft_drop_sequences() {
        fn board() -> Board {
            let rows = [
                "SSS  SSSSS",
                "SSSS  SSSS",
                "S     SSSS",
                "S      SSS",
                "        SS",
                "         S",
                "         S",
                "         S",
                "         S",
                "         S",
            ];
            let mut board = Board::new();
            for (y, row) in rows.iter().enumerate() {
                for (x, c) in row.chars().enumerate() {
                    if c == 'S' {
                        board.set_block(
                            (x as i32, y as i32),
                            BlockState::Stack(TetrominoShape::S, Rotation::North, 0),
                        );
                    }
                }
            }
            board
        }

        let expected: Vec<InputSequence> = board()
            .search_all_inputs(TetrominoShape::S)
            .into_iter()
            .map(|r| r.inputs().clone())
            .collect();
        let tuck = InputSequence::new(vec![Left, SoftDrop, RotateClockwise, RotateClockwise]);
        assert!(
            expected.contains(&tuck),
            "expected the canonical tuck sequence, got {:?}",
            expected
        );
        assert_eq!(expected.len(), 18);
        for _ in 0..100 {
            let again: Vec<InputSequence> = board()
                .search_all_inputs(TetrominoShape::S)
                .into_iter()
                .map(|r| r.inputs().clone())
                .collect();
            assert_eq!(again, expected);
        }
    }

    #[test]
    fn searches_from_non_spawn_position() {
        let mut board = Board::new();
        board.set_block(
            (0, 1),
            BlockState::Stack(TetrominoShape::I, Rotation::North, 0),
        );

        board.try_spawn_tetromino(TetrominoShape::L).unwrap();
        for _ in 0..10 {
            board.step_down();
        }
        board.rotate(true);
        board.right();
        board.right();

        let inputs = board.search_all_inputs(TetrominoShape::L);
        assert_eq!(inputs.len(), 35);
    }
}
