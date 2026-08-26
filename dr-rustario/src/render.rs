use crate::game::cell::DrCell;
use crate::game::pill::VITAMIN_SPAWN_POINTS;
use crate::game::Game;
use crate::theme::data::{CLEAR_VIRUS, CLEAR_VIRUS_COMBO, CLEAR_VITAMIN, CLEAR_VITAMIN_COMBO};
use engine::game::geometry::Point;
use engine::game::{GameEvent, PlacedCell};
use engine::particles::field::reaction::words;
use engine::render::GameRender;

impl GameRender for Game {
    fn name(&self) -> &'static str {
        "Dr. Rustario"
    }

    /// a four in a row that takes another with it as it goes
    fn clear_word(&self, event: &GameEvent) -> Option<&'static str> {
        match event {
            GameEvent::Clear { is_combo: true, .. } => Some(words::COMBO),
            _ => None,
        }
    }

    fn clear_class(&self, event: &GameEvent) -> u16 {
        match event {
            GameEvent::Clear {
                cells, is_combo, ..
            } => {
                let virus = cells.iter().any(|(_, id)| DrCell::from(*id).is_virus());
                match (virus, *is_combo) {
                    (true, true) => CLEAR_VIRUS_COMBO,
                    (true, false) => CLEAR_VIRUS,
                    (false, true) => CLEAR_VITAMIN_COMBO,
                    (false, false) => CLEAR_VITAMIN,
                }
            }
            _ => 0,
        }
    }

    fn spawn_cells(&self) -> Vec<Point> {
        VITAMIN_SPAWN_POINTS.to_vec()
    }

    fn stage_intro_cells(&self) -> Vec<PlacedCell> {
        self.viruses().into_iter().map(PlacedCell::from).collect()
    }
}
