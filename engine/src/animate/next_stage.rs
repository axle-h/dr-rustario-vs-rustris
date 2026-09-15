use crate::game::PlacedCell;
use rand::prelude::ThreadRng;
use rand::rng;
use rand::seq::SliceRandom;
use std::time::Duration;

const POP_IN_DURATION: Duration = Duration::from_millis(1500);
const NEXT_CELL_DURATION: Duration = Duration::from_millis(100);

/// The fixed cells of a fresh stage (Dr. Mario's viruses) pop into the board one by one.
#[derive(Clone, Debug)]
pub struct State {
    cells: Vec<PlacedCell>,
    duration: Duration,
}

impl State {
    pub fn display_cells(&self) -> Vec<PlacedCell> {
        if self.cells.is_empty() {
            return vec![];
        }
        let next_cell_duration = NEXT_CELL_DURATION.min(POP_IN_DURATION / self.cells.len() as u32);
        let count = ((self.duration.as_millis() / next_cell_duration.as_millis().max(1)) as usize)
            .min(self.cells.len());
        self.cells.iter().take(count).copied().collect()
    }
}

#[derive(Clone, Debug)]
pub struct NextStageAnimation {
    state: Option<State>,
    rng: ThreadRng,
}

impl Default for NextStageAnimation {
    fn default() -> Self {
        Self::new()
    }
}

impl NextStageAnimation {
    pub fn new() -> Self {
        Self {
            state: None,
            rng: rng(),
        }
    }

    pub fn update(&mut self, delta: Duration) {
        if let Some(state) = self.state.as_mut() {
            state.duration += delta;
            if state.duration >= POP_IN_DURATION {
                self.state = None;
            }
        }
    }

    pub fn state(&self) -> Option<&State> {
        self.state.as_ref()
    }

    pub fn next_stage(&mut self, cells: &[PlacedCell]) {
        let mut cells = cells.to_vec();
        cells.shuffle(&mut self.rng);
        self.state = Some(State {
            cells,
            duration: Duration::ZERO,
        });
    }
}
