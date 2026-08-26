use crate::game::board::Spin;
use crate::game::tetromino::TetrominoShape;
use crate::game::{ClearAction, Game};
use engine::game::geometry::Point;
use engine::game::GameEvent;
use engine::particles::field::reaction::words;
use engine::render::GameRender;

/// the class of a tetris, the biggest clear a theme has a sound for
const TETRIS_CLEAR_CLASS: u16 = 3;

impl GameRender for Game {
    fn name(&self) -> &'static str {
        "Rustris"
    }

    /// lines cleared minus one: single, double, triple, tetris. A T-spin that cleared anything
    /// is graded a tetris, so it lands with a theme's biggest clear rather than with the one or
    /// two lines it happened to take
    fn clear_class(&self, event: &GameEvent) -> u16 {
        match event {
            GameEvent::Clear { count, detail, .. } => {
                if ClearAction::from_detail(*detail).spin == Some(Spin::Full) {
                    TETRIS_CLEAR_CLASS
                } else {
                    count.saturating_sub(1).min(3) as u16
                }
            }
            _ => 0,
        }
    }

    /// an emptied board is worth saying so, then a spin, then a tetris
    fn clear_word(&self, event: &GameEvent) -> Option<&'static str> {
        match event {
            GameEvent::Clear { count, detail, .. } => {
                let action = ClearAction::from_detail(*detail);
                if action.perfect_clear {
                    Some(words::PERFECT)
                } else if action.spin.is_some() {
                    Some(words::T_SPIN)
                } else if *count >= 4 {
                    Some(words::TETRIS)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn spawn_cells(&self) -> Vec<Point> {
        let shape = self
            .board()
            .tetromino()
            .map(|t| t.shape())
            .unwrap_or(TetrominoShape::I);
        let spawn = shape.meta().spawn_point();
        shape
            .meta()
            .rotated_minos(engine::game::geometry::Rotation::North)
            .into_iter()
            .map(|p| {
                let p = p + spawn;
                Point::new(p.x, crate::game::board::TOTAL_HEIGHT as i32 - 1 - p.y)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::board::Spin;
    use crate::game::random::{RandomMode, RandomTetromino};
    use engine::particles::field::reaction::words;

    fn clear(count: u32) -> GameEvent {
        clear_action(ClearAction {
            lines: count,
            spin: None,
            perfect_clear: false,
        })
    }

    fn clear_action(action: ClearAction) -> GameEvent {
        GameEvent::Clear {
            cells: vec![],
            count: action.lines,
            is_combo: false,
            detail: action.to_detail(),
        }
    }

    #[test]
    fn only_a_tetris_is_worth_spelling_out() {
        let game = Game::new(0, RandomTetromino::new(RandomMode::Bag, 10, 7.into()));
        assert_eq!(game.clear_word(&clear(4)), Some(words::TETRIS));
        for count in 1..4 {
            assert_eq!(game.clear_word(&clear(count)), None, "{count} lines");
        }
    }

    #[test]
    fn a_t_spin_is_worth_spelling_out_however_many_lines_it_took() {
        let game = Game::new(0, RandomTetromino::new(RandomMode::Bag, 10, 7.into()));
        for spin in [Spin::Full, Spin::Mini] {
            for lines in 1..=3 {
                let event = clear_action(ClearAction {
                    lines,
                    spin: Some(spin),
                    perfect_clear: false,
                });
                assert_eq!(
                    game.clear_word(&event),
                    Some(words::T_SPIN),
                    "{spin:?}, {lines} lines"
                );
            }
        }
    }

    #[test]
    fn an_emptied_board_is_spelt_out_ahead_of_the_spin_that_emptied_it() {
        let game = Game::new(0, RandomTetromino::new(RandomMode::Bag, 10, 7.into()));
        let event = clear_action(ClearAction {
            lines: 2,
            spin: Some(Spin::Full),
            perfect_clear: true,
        });
        assert_eq!(game.clear_word(&event), Some(words::PERFECT));
    }

    #[test]
    fn a_t_spin_sounds_like_a_tetris() {
        let game = Game::new(0, RandomTetromino::new(RandomMode::Bag, 10, 7.into()));
        for lines in 1..=3 {
            let event = clear_action(ClearAction {
                lines,
                spin: Some(Spin::Full),
                perfect_clear: false,
            });
            assert_eq!(
                game.clear_class(&event),
                TETRIS_CLEAR_CLASS,
                "{lines} lines"
            );
        }
    }

    #[test]
    fn line_clears_are_graded_by_the_lines_they_cleared() {
        let game = Game::new(0, RandomTetromino::new(RandomMode::Bag, 10, 7.into()));
        for lines in 1..=4 {
            assert_eq!(game.clear_class(&clear(lines)), lines as u16 - 1, "{lines}");
        }
    }

    #[test]
    fn a_perfect_clear_is_worth_spelling_out_however_many_lines_it_took() {
        let game = Game::new(0, RandomTetromino::new(RandomMode::Bag, 10, 7.into()));
        for lines in 1..=4 {
            let event = clear_action(ClearAction {
                lines,
                spin: None,
                perfect_clear: true,
            });
            assert_eq!(
                game.clear_word(&event),
                Some(words::PERFECT),
                "{lines} lines"
            );
        }
    }
}
