use crate::game::board::{compact_destroy_lines, Board, BOARD_HEIGHT, BOARD_WIDTH};
use crate::game::geometry::Point;
use std::collections::{HashSet, VecDeque};
use std::ops::Sub;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct BoardStats {
    global: StackStats,
    delta: StackStats,
    cleared_lines: u32,
    max_tetromino_y: u32,
}

impl BoardStats {
    pub fn global(&self) -> StackStats {
        self.global
    }

    pub fn delta(&self) -> StackStats {
        self.delta
    }

    pub fn cleared_lines(&self) -> u32 {
        self.cleared_lines
    }

    pub fn max_tetromino_y(&self) -> u32 {
        self.max_tetromino_y
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct StackStats {
    open_holes: i32,
    closed_holes: i32,
    max_height: i32,
    min_height: i32,
    sum_roughness: i32,
    max_roughness: i32,
    pillars: i32,
    hole_cover: i32,
    column_heights: [i32; BOARD_WIDTH as usize],
}

impl StackStats {
    pub fn open_holes(&self) -> i32 {
        self.open_holes
    }

    pub fn closed_holes(&self) -> i32 {
        self.closed_holes
    }

    pub fn holes(&self) -> i32 {
        self.open_holes + self.closed_holes
    }

    pub fn max_height(&self) -> i32 {
        self.max_height
    }

    pub fn min_height(&self) -> i32 {
        self.min_height
    }

    pub fn sum_roughness(&self) -> i32 {
        self.sum_roughness
    }

    pub fn max_roughness(&self) -> i32 {
        self.max_roughness
    }

    pub fn pillars(&self) -> i32 {
        self.pillars
    }

    pub fn hole_cover(&self) -> i32 {
        self.hole_cover
    }

    pub fn column_heights(&self) -> [i32; BOARD_WIDTH as usize] {
        self.column_heights
    }

    pub fn rhs_column_height(&self) -> i32 {
        self.column_heights[BOARD_WIDTH as usize - 1]
    }
}

impl Sub<StackStats> for StackStats {
    type Output = StackStats;

    fn sub(self, rhs: StackStats) -> Self::Output {
        StackStats {
            open_holes: self.open_holes - rhs.open_holes,
            closed_holes: self.closed_holes - rhs.closed_holes,
            max_height: self.max_height - rhs.max_height,
            min_height: self.min_height - rhs.min_height,
            sum_roughness: self.sum_roughness - rhs.sum_roughness,
            max_roughness: self.max_roughness - rhs.max_roughness,
            pillars: self.pillars - rhs.pillars,
            hole_cover: self.hole_cover - rhs.hole_cover,
            column_heights: self
                .column_heights
                .iter()
                .zip(rhs.column_heights.iter())
                .map(|(a, b)| a - b)
                .collect::<Vec<i32>>()
                .try_into()
                .unwrap_or_else(|_| [0; BOARD_WIDTH as usize]),
        }
    }
}

pub trait BoardFeatures {
    fn stack_stats(&self) -> StackStats;
    fn features_after_action(
        &self,
        board_before_action: &Board,
        stats_before_action: StackStats,
    ) -> BoardStats;
}

impl BoardFeatures for Board {
    fn stack_stats(self: &Board) -> StackStats {
        let mut holes: HashSet<Point> = HashSet::new();
        let mut max_heights = [0; BOARD_WIDTH as usize];
        let mut hole_cover = 0;

        for x in 0..BOARD_WIDTH as i32 {
            let mut stack_depth = 0;
            let mut current_hole_cover = 0;
            for y in (0..BOARD_HEIGHT as i32).rev() {
                let point = Point::new(x, y);
                if self.block(point).is_stack() {
                    stack_depth += 1;
                    if stack_depth == 1 {
                        // first stack block reached
                        max_heights[x as usize] = y as u32 + 1;
                    }
                    current_hole_cover += 1;
                } else if stack_depth > 0 {
                    holes.insert(point);
                    hole_cover += current_hole_cover;
                    current_hole_cover = 0;
                }
            }
        }

        let mut open_holes: HashSet<Point> = HashSet::new();
        for p in holes.iter() {
            let mut to_visit = VecDeque::from([p.translate(-1, 0), p.translate(1, 0)]);
            while let Some(neighbour) = to_visit.pop_front() {
                if neighbour.x < 0
                    || neighbour.x >= BOARD_WIDTH as i32
                    || !self.block(neighbour).is_empty()
                {
                    continue;
                }

                if open_holes.contains(&neighbour) || !holes.contains(&neighbour) {
                    // a neighbour is an open hole this hole must also be open
                    // OR a neighbour is empty but not a hole then this must be an open hole
                    open_holes.insert(*p);
                    break;
                }

                // neighbour is a hole, but we're not sure if it's an open hole yet so check the 2nd order neighbour
                let neighbour2 = if neighbour.x > p.x {
                    neighbour.translate(1, 0)
                } else {
                    neighbour.translate(-1, 0)
                };
                to_visit.push_back(neighbour2);
            }
        }

        let mut max_roughness = 0;
        let mut sum_roughness = 0;
        let mut max_height = 0;
        let mut min_height = BOARD_HEIGHT;
        let mut pillars = 0;

        for i in 0..max_heights.len() {
            let prev = if i > 0 {
                Some(max_heights[i - 1])
            } else {
                None
            };
            let height = max_heights[i];
            let next = max_heights.get(i + 1).copied();

            max_height = max_height.max(height);
            min_height = min_height.min(height);

            let prev_height_delta = if let Some(prev) = prev {
                height as i32 - prev as i32
            } else {
                0
            };

            if prev.is_none() || prev_height_delta < -2 {
                if next.is_none() {
                    // against the right edge
                    pillars += 1;
                } else if let Some(next) = next {
                    let next_height_delta = next as i32 - height as i32;
                    if next_height_delta > 2 {
                        pillars += 1;
                    }
                }
            }

            let roughness = prev_height_delta.abs() as u32;
            max_roughness = max_roughness.max(roughness);
            sum_roughness += roughness;
        }

        StackStats {
            open_holes: open_holes.len() as i32,
            closed_holes: (holes.len() - open_holes.len()) as i32,
            max_height: max_height as i32,
            min_height: min_height as i32,
            sum_roughness: sum_roughness as i32,
            max_roughness: max_roughness as i32,
            pillars,
            hole_cover,
            column_heights: max_heights.map(|h| h as i32),
        }
    }

    fn features_after_action(
        &self,
        board_before_action: &Board,
        stats_before_action: StackStats,
    ) -> BoardStats {
        let mut max_tetromino_y = 0u32;
        for x in 0..BOARD_WIDTH as i32 {
            for y in (0..BOARD_HEIGHT as i32).rev() {
                if self.block((x, y)).is_stack() && board_before_action.block((x, y)).is_empty() {
                    max_tetromino_y = max_tetromino_y.max(y as u32);
                }
            }
        }

        // get stack stats AFTER the tetromino was locked AND any lines were cleared
        let mut board = *self;
        let patterns = board.pattern();
        let cleared_lines = compact_destroy_lines(patterns).len() as u32;
        board.destroy(patterns);
        let global = board.stack_stats();

        BoardStats {
            global,
            delta: global - stats_before_action,
            cleared_lines,
            max_tetromino_y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::block::BlockState;
    use crate::game::geometry::Rotation;
    use crate::game::tetromino::TetrominoShape;

    #[test]
    fn empty_board() {
        let stats = Board::new().features_from_empty();
        assert_eq!(stats, BoardStats::default());
    }

    #[test]
    fn ignores_tetromino() {
        let mut board = Board::new();
        board.try_spawn_tetromino(TetrominoShape::J);
        let stats = board.stack_stats();
        assert_eq!(stats, StackStats::default());
    }

    #[test]
    fn no_holes() {
        let stats = Board::new().having_stack_at(&[(0, 0)]).stack_stats();
        assert_eq!(stats.closed_holes, 0);
        assert_eq!(stats.open_holes, 0);
        assert_eq!(stats.hole_cover, 0);
    }

    #[test]
    fn one_closed_hole() {
        let stats = Board::new()
            .having_stack_at(&[(0, 1), (1, 1), (2, 1), (0, 0), (2, 0)])
            .stack_stats();
        assert_eq!(stats.closed_holes, 1);
        assert_eq!(stats.open_holes, 0);
        assert_eq!(stats.hole_cover, 1);
    }

    #[test]
    fn one_closed_hole_against_the_edge() {
        let stats = Board::new()
            .having_stack_at(&[(0, 1), (1, 1), (1, 0)])
            .stack_stats();
        assert_eq!(stats.closed_holes, 1);
        assert_eq!(stats.open_holes, 0);
        assert_eq!(stats.hole_cover, 1);
    }

    #[test]
    fn double_closed_hole() {
        let stats = Board::new()
            .having_stack_at(&[(0, 1), (1, 1), (2, 1), (3, 1), (0, 0), (3, 0)])
            .stack_stats();
        assert_eq!(stats.closed_holes, 2);
        assert_eq!(stats.open_holes, 0);
        assert_eq!(stats.hole_cover, 2);
    }

    #[test]
    fn two_closed_holes() {
        let stats = Board::new()
            .having_stack_at(&[
                (0, 3),
                (1, 3),
                (2, 3),
                (0, 2),
                (2, 2),
                (0, 1),
                (1, 1),
                (2, 1),
                (0, 0),
                (2, 0),
            ])
            .stack_stats();
        assert_eq!(stats.closed_holes, 2);
        assert_eq!(stats.open_holes, 0);
        assert_eq!(stats.hole_cover, 2);
    }

    #[test]
    fn one_open_hole_right() {
        let stats = Board::new()
            .having_stack_at(&[(0, 1), (1, 1), (0, 0)])
            .stack_stats();
        assert_eq!(stats.closed_holes, 0);
        assert_eq!(stats.open_holes, 1);
        assert_eq!(stats.hole_cover, 1);
    }

    #[test]
    fn double_open_hole_right() {
        let stats = Board::new()
            .having_stack_at(&[(0, 1), (1, 1), (2, 1), (0, 0)])
            .stack_stats();
        assert_eq!(stats.closed_holes, 0);
        assert_eq!(stats.open_holes, 2);
        assert_eq!(stats.hole_cover, 2);
    }

    #[test]
    fn one_open_hole_left() {
        let stats = Board::new()
            .having_stack_at(&[(8, 1), (9, 1), (9, 0)])
            .stack_stats();
        assert_eq!(stats.closed_holes, 0);
        assert_eq!(stats.open_holes, 1);
        assert_eq!(stats.hole_cover, 1);
    }

    #[test]
    fn double_open_hole_left() {
        let stats = Board::new()
            .having_stack_at(&[(7, 1), (8, 1), (9, 1), (9, 0)])
            .stack_stats();
        assert_eq!(stats.closed_holes, 0);
        assert_eq!(stats.open_holes, 2);
        assert_eq!(stats.hole_cover, 2);
    }

    #[test]
    fn big_open_holes() {
        let stats = Board::new()
            .having_stack_at(&[
                (0, 1),
                (1, 1),
                (2, 1),
                (3, 1),
                (4, 1),
                (6, 1),
                (7, 1),
                (8, 1),
                (9, 1),
                (0, 0),
                (9, 0),
            ])
            .stack_stats();
        assert_eq!(stats.closed_holes, 0);
        assert_eq!(stats.open_holes, 7);
        assert_eq!(stats.hole_cover, 7);
    }

    #[test]
    fn kitchen_sink_of_holes() {
        let stats = Board::new()
            .having_stack_at(&[
                (2, 3),
                (3, 3),
                (4, 3),
                (6, 3),
                (7, 3),
                (8, 3),
                (9, 3),
                (0, 2),
                (1, 2),
                (2, 2),
                (3, 2),
                (4, 2),
                (6, 2),
                (8, 2),
                (0, 1),
                (1, 1),
                (3, 1),
                (4, 1),
                (6, 1),
                (7, 1),
                (8, 1),
                (0, 0),
                (6, 0),
                (9, 0),
            ])
            .stack_stats();
        assert_eq!(stats.closed_holes, 6);
        assert_eq!(stats.open_holes, 4);
        assert_eq!(stats.hole_cover, 16);
    }

    #[test]
    fn single_block_change() {
        let stats = Board::new().having_stack_at(&[(0, 0)]).stack_stats();
        assert_eq!(stats.max_height, 1);
        assert_eq!(stats.min_height, 0);
        assert_eq!(stats.sum_roughness, 1);
        assert_eq!(stats.max_roughness, 1);
        assert_eq!(stats.pillars, 0);
        assert_eq!(stats.column_heights, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn double_block_change() {
        let stats = Board::new().having_stack_at(&[(1, 0)]).stack_stats();
        assert_eq!(stats.max_height, 1);
        assert_eq!(stats.min_height, 0);
        assert_eq!(stats.sum_roughness, 2);
        assert_eq!(stats.max_roughness, 1);
        assert_eq!(stats.pillars, 0);
        assert_eq!(stats.column_heights, [0, 1, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn multi_level_change() {
        let stats = Board::new()
            .having_stack_at(&[(1, 1), (1, 0)])
            .stack_stats();

        assert_eq!(stats.max_height, 2);
        assert_eq!(stats.min_height, 0);
        assert_eq!(stats.sum_roughness, 4);
        assert_eq!(stats.max_roughness, 2);
        assert_eq!(stats.pillars, 0);
        assert_eq!(stats.column_heights, [0, 2, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn based_on_post_line_clear() {
        let stats = Board::new()
            .having_stack_at(&[
                (0, 1),
                (0, 0),
                (1, 0),
                (2, 0),
                (3, 0),
                (4, 0),
                (5, 0),
                (6, 0),
                (7, 0),
                (8, 0),
                (9, 0),
            ])
            .features_from_empty();

        assert_eq!(
            stats.global(),
            StackStats {
                open_holes: 0,
                closed_holes: 0,
                max_height: 1,
                min_height: 0,
                sum_roughness: 1,
                max_roughness: 1,
                pillars: 0,
                hole_cover: 0,
                column_heights: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            }
        );
        assert_eq!(stats.cleared_lines, 1);
    }

    #[test]
    fn pillars_against_edges() {
        let stats = Board::new()
            .having_stack_at(&[(1, 2), (8, 2), (1, 1), (8, 1), (1, 0), (8, 0)])
            .stack_stats();
        assert_eq!(stats.pillars, 2);
    }

    #[test]
    fn pillars_in_middle() {
        let stats = Board::new()
            .having_stack_at(&[(0, 2), (2, 2), (0, 1), (2, 1), (0, 0), (2, 0)])
            .stack_stats();
        assert_eq!(stats.pillars, 1);
    }

    #[test]
    fn pillars_at_altitude() {
        let stats = Board::new()
            .having_stack_at(&[
                (0, 3),
                (2, 3),
                (0, 2),
                (2, 2),
                (0, 1),
                (2, 1),
                (0, 0),
                (1, 0),
                (2, 0),
            ])
            .stack_stats();
        assert_eq!(stats.pillars, 1);
    }

    #[test]
    fn kitchen_sink() {
        let stats = Board::new()
            .having_stack_at(&[
                (2, 3),
                (4, 3),
                (6, 3),
                (8, 3),
                (0, 2),
                (1, 2),
                (2, 2),
                (3, 2),
                (4, 2),
                (6, 2),
                (8, 2),
                (0, 1),
                (1, 1),
                (3, 1),
                (4, 1),
                (6, 1),
                (7, 1),
                (8, 1),
                (0, 0),
                (6, 0),
                (9, 0),
            ])
            .stack_stats();

        assert_eq!(stats.max_height, 4);
        assert_eq!(stats.min_height, 0);
        assert_eq!(stats.sum_roughness, 18);
        assert_eq!(stats.max_roughness, 4);
        assert_eq!(stats.pillars, 2); // one at x=5 and another at x=9
        assert_eq!(stats.column_heights, [3, 3, 4, 3, 4, 0, 4, 2, 4, 1]);
    }

    #[test]
    fn cleared_lines_1() {
        let stats = Board::new()
            .having_stack_at(&[
                (0, 0),
                (1, 0),
                (2, 0),
                (3, 0),
                (4, 0),
                (5, 0),
                (6, 0),
                (7, 0),
                (8, 0),
                (9, 0),
            ])
            .features_from_empty();
        assert_eq!(stats.cleared_lines, 1);
    }

    #[test]
    fn cleared_lines_2() {
        let stats = Board::new()
            .having_stack_at(&[
                (0, 1),
                (1, 1),
                (2, 1),
                (3, 1),
                (4, 1),
                (5, 1),
                (6, 1),
                (7, 1),
                (8, 1),
                (9, 1),
                (0, 0),
                (1, 0),
                (2, 0),
                (3, 0),
                (4, 0),
                (5, 0),
                (6, 0),
                (7, 0),
                (8, 0),
                (9, 0),
            ])
            .features_from_empty();
        assert_eq!(stats.cleared_lines, 2);
    }

    #[test]
    fn cleared_lines_3() {
        let stats = Board::new()
            .having_stack_at(&[
                (0, 2),
                (1, 2),
                (2, 2),
                (3, 2),
                (4, 2),
                (5, 2),
                (6, 2),
                (7, 2),
                (8, 2),
                (9, 2),
                (0, 1),
                (1, 1),
                (2, 1),
                (3, 1),
                (4, 1),
                (5, 1),
                (6, 1),
                (7, 1),
                (8, 1),
                (9, 1),
                (0, 0),
                (1, 0),
                (2, 0),
                (3, 0),
                (4, 0),
                (5, 0),
                (6, 0),
                (7, 0),
                (8, 0),
                (9, 0),
            ])
            .features_from_empty();
        assert_eq!(stats.cleared_lines, 3);
    }

    #[test]
    fn cleared_lines_4() {
        let stats = Board::new()
            .having_stack_at(&[
                (0, 3),
                (1, 3),
                (2, 3),
                (3, 3),
                (4, 3),
                (5, 3),
                (6, 3),
                (7, 3),
                (8, 3),
                (9, 3),
                (0, 2),
                (1, 2),
                (2, 2),
                (3, 2),
                (4, 2),
                (5, 2),
                (6, 2),
                (7, 2),
                (8, 2),
                (9, 2),
                (0, 1),
                (1, 1),
                (2, 1),
                (3, 1),
                (4, 1),
                (5, 1),
                (6, 1),
                (7, 1),
                (8, 1),
                (9, 1),
                (0, 0),
                (1, 0),
                (2, 0),
                (3, 0),
                (4, 0),
                (5, 0),
                (6, 0),
                (7, 0),
                (8, 0),
                (9, 0),
            ])
            .features_from_empty();
        assert_eq!(stats.cleared_lines, 4);
    }

    #[test]
    fn new_closed_hole() {
        let board0 = Board::new().having_stack_at(&[(0, 0), (2, 0)]);

        let board1 = board0.having_stack_at(&[(0, 2), (1, 2), (0, 1), (1, 1)]);

        let stats = board1.features_after_action(&board0, StackStats::default());
        assert_eq!(stats.global.closed_holes, 1);
        assert_eq!(stats.global.open_holes, 0);
        assert_eq!(stats.delta.closed_holes, 1);
        assert_eq!(stats.delta.open_holes, 0);
    }

    #[test]
    fn new_open_hole() {
        let board0 = Board::new().having_stack_at(&[(0, 0)]);

        let board1 = board0.having_stack_at(&[(0, 2), (1, 2), (0, 1), (1, 1)]);

        let stats = board1.features_after_action(&board0, StackStats::default());
        assert_eq!(stats.global.closed_holes, 0);
        assert_eq!(stats.global.open_holes, 1);
        assert_eq!(stats.delta.closed_holes, 0);
        assert_eq!(stats.delta.open_holes, 1);
    }

    trait BoardHarness {
        fn having_stack_at(self, points: &[(u32, u32)]) -> Self;

        fn features_from_empty(&self) -> BoardStats;
    }

    impl BoardHarness for Board {
        fn having_stack_at(mut self, points: &[(u32, u32)]) -> Self {
            for point in points.into_iter() {
                self.set_block(
                    *point,
                    BlockState::Stack(TetrominoShape::L, Rotation::North, 0),
                );
            }
            self
        }

        fn features_from_empty(&self) -> BoardStats {
            self.features_after_action(&Board::new(), StackStats::default())
        }
    }
}
