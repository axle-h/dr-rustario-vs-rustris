use crate::game::tetromino::TetrominoShape;
use crate::game::Game;
use engine::game::geometry::Point;
use engine::game::GameEvent;
use engine::particles::field::reaction::words;
use engine::render::GameRender;

impl GameRender for Game {
    fn name(&self) -> &'static str {
        "Rustris"
    }

    /// lines cleared minus one: single, double, triple, tetris
    fn clear_class(&self, event: &GameEvent) -> u16 {
        match event {
            GameEvent::Clear { count, .. } => count.saturating_sub(1).min(3) as u16,
            _ => 0,
        }
    }

    fn clear_word(&self, event: &GameEvent) -> Option<&'static str> {
        match event {
            GameEvent::Clear { count, .. } if *count >= 4 => Some(words::TETRIS),
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
    use crate::game::random::{RandomMode, RandomTetromino};
    use engine::particles::field::reaction::words;

    fn clear(count: u32) -> GameEvent {
        GameEvent::Clear {
            cells: vec![],
            count,
            is_combo: false,
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
}
