use super::tetromino::TetrominoShape;
use crate::game::board::BOARD_WIDTH;
use engine::game::random::BagRandom;
pub use engine::game::random::RandomMode;
use rand::RngExt;
use rand_chacha::ChaChaRng;

pub const PEEK_SIZE: usize = 7;
/// the garbage hole moves every n rows of garbage
pub const MIN_GARBAGE_PER_HOLE: u32 = 10;

/// the seed a training run counts through, shared with the Dr. Rustario trainer
pub use engine::ai::Seed;

pub fn random_tetrominos(mode: RandomMode, count: usize) -> Vec<RandomTetromino> {
    let seed: Seed = rand::random();
    (0..count)
        .map(|_| RandomTetromino::new(mode, MIN_GARBAGE_PER_HOLE, seed))
        .collect()
}

#[derive(Clone, Debug)]
pub struct RandomTetromino {
    min_garbage_per_hole: u32, // move the garbage hole every n garbage
    garbage_since_last_hole: u32,
    current_garbage_hole: u32,
    shapes: BagRandom<TetrominoShape>,
}

impl RandomTetromino {
    pub fn new(random_mode: RandomMode, min_garbage_per_hole: u32, seed: Seed) -> Self {
        let mut rng: ChaChaRng = seed.into();
        let current_garbage_hole = rng.random_range(0..BOARD_WIDTH);
        Self {
            min_garbage_per_hole,
            garbage_since_last_hole: 0,
            current_garbage_hole,
            shapes: BagRandom::new(rng, random_mode, &TetrominoShape::ALL, PEEK_SIZE),
        }
    }

    pub fn next_garbage_hole(&mut self) -> u32 {
        let result = self.current_garbage_hole;
        self.garbage_since_last_hole += 1;
        if self.garbage_since_last_hole >= self.min_garbage_per_hole {
            self.garbage_since_last_hole = 0;
            self.current_garbage_hole = self.shapes.rng().random_range(0..BOARD_WIDTH);
        }
        result
    }

    pub fn next(&mut self) -> TetrominoShape {
        self.shapes.next()
    }

    pub fn peek_buffer(&self) -> [TetrominoShape; PEEK_SIZE] {
        self.shapes.peek().try_into().unwrap()
    }

    pub fn peek(&self) -> TetrominoShape {
        self.shapes.peek_next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn next_n(random: &mut RandomTetromino, n: usize) -> Vec<TetrominoShape> {
        (0..n).map(|_| random.next()).collect()
    }

    fn next_n_holes(random: &mut RandomTetromino, n: usize) -> Vec<u32> {
        (0..n).map(|_| random.next_garbage_hole()).collect()
    }

    #[test]
    fn bag_random() {
        let mut random = RandomTetromino::new(RandomMode::Bag, 10, rand::random());

        // chunk into 3 bags of 7 shapes (arrays make it easier for creating the sets)
        let bags: Vec<[TetrominoShape; 7]> = next_n(&mut random, 21)
            .chunks(7)
            .map(|chunk| chunk.to_vec().try_into().unwrap())
            .collect();

        // each bag should not be in same order
        assert_ne!(bags[0], bags[1]);
        assert_ne!(bags[1], bags[2]);

        // but should all contain all the shapes
        let all_shapes = HashSet::from(TetrominoShape::ALL);
        assert_eq!(HashSet::from(bags[0]), all_shapes);
        assert_eq!(HashSet::from(bags[1]), all_shapes);
        assert_eq!(HashSet::from(bags[2]), all_shapes);
    }

    #[test]
    fn bag_random_peek() {
        let mut random = RandomTetromino::new(RandomMode::Bag, 10, rand::random());
        let peek = random.peek_buffer();
        let observed: [TetrominoShape; PEEK_SIZE] =
            next_n(&mut random, PEEK_SIZE).try_into().unwrap();
        assert_eq!(observed, peek);
    }

    #[test]
    fn true_random() {
        let mut random = RandomTetromino::new(RandomMode::True, 10, rand::random());
        let observed: [TetrominoShape; 1000] = next_n(&mut random, 1000).try_into().unwrap();
        // should generate all shapes in 1000 tries
        assert_eq!(HashSet::from(observed), HashSet::from(TetrominoShape::ALL));
    }

    #[test]
    fn true_random_peek() {
        let mut random = RandomTetromino::new(RandomMode::True, 10, rand::random());
        let peek = random.peek_buffer();
        let observed: [TetrominoShape; PEEK_SIZE] =
            next_n(&mut random, PEEK_SIZE).try_into().unwrap();
        assert_eq!(observed, peek);
    }

    #[test]
    fn static_garbage_hole() {
        let mut random = RandomTetromino::new(RandomMode::True, 100, rand::random());
        let observed: [u32; 100] = next_n_holes(&mut random, 100).try_into().unwrap();
        assert_eq!(HashSet::from(observed).len(), 1);
    }

    #[test]
    fn dynamic_garbage_hole() {
        let mut random = RandomTetromino::new(RandomMode::True, 1, rand::random());
        let observed: [u32; 100] = next_n_holes(&mut random, 100).try_into().unwrap();
        assert!(HashSet::from(observed).len() > 1);
    }
}
