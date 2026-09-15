use crate::game::geometry::Point;
use crate::game::PlacedCell;
use std::collections::HashSet;
use std::time::Duration;

const LOCK_DURATION: Duration = Duration::from_millis(100);
const FRAMES: u32 = 1;
const MAX_OFFSET: f64 = 0.1;

/// The just-locked piece settles into the stack with a small bounce.
#[derive(Clone, Debug, Default)]
pub struct State {
    cells: HashSet<Point>,
    duration: Duration,
    frame: u32,
}

impl State {
    pub fn animates(&self, point: Point) -> bool {
        self.cells.contains(&point)
    }

    /// offset the cells by this fraction of a block
    pub fn offset_y(&self) -> f64 {
        (self.frame + 1) as f64 * (MAX_OFFSET / FRAMES as f64)
    }
}

#[derive(Clone, Debug)]
pub struct LockAnimation {
    state: Option<State>,
    frame_duration: Duration,
}

impl Default for LockAnimation {
    fn default() -> Self {
        Self::new()
    }
}

impl LockAnimation {
    pub fn new() -> Self {
        Self {
            state: None,
            frame_duration: LOCK_DURATION / FRAMES,
        }
    }

    pub fn update(&mut self, delta: Duration) {
        let mut finished = false;
        if let Some(state) = self.state.as_mut() {
            state.duration += delta;
            if state.duration < self.frame_duration {
                return;
            }
            state.duration = Duration::ZERO;
            state.frame += 1;
            finished = state.frame == FRAMES;
        }
        if finished {
            self.state = None;
        }
    }

    pub fn reset(&mut self) {
        self.state = None;
    }

    pub fn lock(&mut self, cells: &[PlacedCell]) {
        self.state = Some(State {
            cells: cells.iter().map(|(p, _)| *p).collect(),
            ..State::default()
        });
    }

    pub fn state(&self) -> Option<&State> {
        self.state.as_ref()
    }
}
