use crate::game::ai::input_sequence::{InputSequence, Translation};
use crate::game::board::Board;
use crate::game::Game;

pub trait ApplyInputs {
    fn apply_inputs(&mut self, seq: &InputSequence) -> bool;
}

impl ApplyInputs for Board {
    fn apply_inputs(&mut self, seq: &InputSequence) -> bool {
        for translation in seq.translations() {
            match translation {
                Translation::Left => {
                    if !self.left() {
                        return false;
                    }
                }
                Translation::Right => {
                    if !self.right() {
                        return false;
                    }
                }
                Translation::RotateClockwise => {
                    if !self.rotate(true) {
                        return false;
                    }
                }
                Translation::RotateAnticlockwise => {
                    if !self.rotate(false) {
                        return false;
                    }
                }
                Translation::HardDrop | Translation::SoftDrop => {
                    // no game logic so soft dropping is equivalent to hard dropping
                    self.hard_drop();
                }
            }
        }

        true
    }
}

impl ApplyInputs for Game {
    fn apply_inputs(&mut self, seq: &InputSequence) -> bool {
        for translation in seq.translations() {
            match translation {
                Translation::Left => {
                    if !self.left() {
                        return false;
                    }
                }
                Translation::Right => {
                    if !self.right() {
                        return false;
                    }
                }
                Translation::RotateClockwise => {
                    if !self.rotate(true) {
                        return false;
                    }
                }
                Translation::RotateAnticlockwise => {
                    if !self.rotate(false) {
                        return false;
                    }
                }
                Translation::HardDrop => {
                    if !self.hard_drop() {
                        return false;
                    }
                }
                Translation::SoftDrop => {
                    self.set_soft_drop(true);
                }
            }
        }

        true
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use crate::game::geometry::Rotation;
    use crate::game::tetromino::TetrominoShape;

    fn having_inputs(shape: TetrominoShape, inputs: InputSequence) -> (Board, bool) {
        let mut board = Board::new();
        board.try_spawn_tetromino(shape).unwrap();
        let result = board.apply_inputs(&inputs);
        (board, result)
    }

    mod applies_empty_inputs {
        use super::*;

        macro_rules! tests {
            ($($name:ident),*) => {
                $(
                    #[test]
                    fn $name() {
                        let shape = TetrominoShape::$name;
                        let (board, success) = having_inputs(shape, InputSequence::default());
                        assert!(success);
                        assert_eq!(board.tetromino().map(|t| t.position()), Some(shape.meta().spawn_point()));
                    }
                )*
            };
        }

        tests! { I, O, T, S, Z, J, L }
    }

    mod moves_left {
        use super::*;

        macro_rules! tests {
            ($($name:ident),*) => {
                $(
                    #[test]
                    fn $name() {
                        let shape = TetrominoShape::$name;
                        let (board, success) = having_inputs(shape, InputSequence::new(vec![Translation::Left]));
                        assert!(success);
                        assert_eq!(board.tetromino().map(|t| t.position()), Some(shape.meta().spawn_point().translate(-1, 0)));
                    }
                )*
            };
        }

        tests! { I, O, T, S, Z, J, L }
    }

    mod moves_right {
        use super::*;

        macro_rules! tests {
            ($($name:ident),*) => {
                $(
                    #[test]
                    fn $name() {
                        let shape = TetrominoShape::$name;
                        let (board, success) = having_inputs(shape, InputSequence::new(vec![Translation::Right]));
                        assert!(success);
                        assert_eq!(board.tetromino().map(|t| t.position()), Some(shape.meta().spawn_point().translate(1, 0)));
                    }
                )*
            };
        }

        tests! { I, O, T, S, Z, J, L }
    }

    mod rotates_clockwise {
        use super::*;

        macro_rules! tests {
            ($($name:ident),*) => {
                $(
                    #[test]
                    fn $name() {
                        let shape = TetrominoShape::$name;
                        let (board, success) = having_inputs(shape, InputSequence::new(vec![Translation::RotateClockwise]));
                        assert!(success);
                        assert_eq!(board.tetromino().map(|t| t.rotation()), Some(Rotation::East));
                    }
                )*
            };
        }

        tests! { I, O, T, S, Z, J, L }
    }

    mod rotates_anticlockwise {
        use super::*;

        macro_rules! tests {
            ($($name:ident),*) => {
                $(
                    #[test]
                    fn $name() {
                        let shape = TetrominoShape::$name;
                        let (board, success) = having_inputs(shape, InputSequence::new(vec![Translation::RotateAnticlockwise]));
                        assert!(success);
                        assert_eq!(board.tetromino().map(|t| t.rotation()), Some(Rotation::West));
                    }
                )*
            };
        }

        tests! { I, O, T, S, Z, J, L }
    }

    #[test]
    fn follows_sequence() {
        let shape = TetrominoShape::J;
        let sequence = InputSequence::new(vec![
            Translation::Left,
            Translation::Left,
            Translation::Left,
            Translation::RotateClockwise,
            Translation::RotateClockwise,
        ]);
        let (board, success) = having_inputs(shape, sequence);
        assert!(success);
        assert_eq!(
            board.tetromino().map(|t| t.position()),
            Some(shape.meta().spawn_point().translate(-3, 0))
        );
        assert_eq!(
            board.tetromino().map(|t| t.rotation()),
            Some(Rotation::South)
        );
    }

    #[test]
    fn blocks_invalid_moves() {
        let shape = TetrominoShape::J;
        let sequence = InputSequence::new(vec![
            Translation::Left,
            Translation::Left,
            Translation::Left,
            Translation::Left,
        ]);
        let (_, success) = having_inputs(shape, sequence);
        assert!(!success);
    }
}
