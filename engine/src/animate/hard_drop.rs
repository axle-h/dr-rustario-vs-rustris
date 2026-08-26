use crate::game::PlacedCell;
use std::time::Duration;

const FRAME_DURATION: f64 = 0.004; // 4 millis
const MAX_ALPHA: u8 = 100;
const MAX_TRAIL_FRAMES: u32 = 5;
/// rows the trail falls per frame unless the theme says otherwise
pub const DEFAULT_ROWS_PER_FRAME: f64 = 0.2;

/// A fading trail of the piece falls from where it was to where it landed.
#[derive(Clone, Debug)]
pub struct State {
    cells: Vec<PlacedCell>,
    duration: f64,
    frame: u32,
    max_frames: u32,
    step: f64,
}

impl State {
    fn new(cells: Vec<PlacedCell>, dropped_rows: u32, step: f64) -> Self {
        let max_frames = dropped_rows as f64 / step;
        Self {
            cells,
            max_frames: max_frames.round().max(1.0) as u32,
            duration: 0.0,
            frame: 0,
            step,
        }
    }

    pub fn frames(&self) -> Vec<HardDropAnimationFrame> {
        let mut result = vec![];
        let trail_frames = self.frame.min(MAX_TRAIL_FRAMES);
        for j in 1..=trail_frames {
            let alpha_mod =
                MAX_ALPHA - (MAX_ALPHA as f64 * j as f64 / trail_frames as f64).round() as u8;
            let offset_y = -1.0 * self.step * j as f64;
            result.push(HardDropAnimationFrame::new(offset_y, alpha_mod));
        }

        // fall
        for j in 1..=self.frame {
            let alpha_mod =
                MAX_ALPHA - (MAX_ALPHA as f64 * j as f64 / self.max_frames as f64).round() as u8;
            let offset_y = self.step * j as f64;
            result.push(HardDropAnimationFrame::new(offset_y, alpha_mod));
        }

        result
    }

    pub fn cells(&self) -> &[PlacedCell] {
        &self.cells
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HardDropAnimationFrame {
    pub offset_y: f64,
    pub alpha_mod: u8,
}

impl HardDropAnimationFrame {
    pub fn new(offset_y: f64, alpha_mod: u8) -> Self {
        Self {
            offset_y,
            alpha_mod,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HardDropAnimation {
    state: Option<State>,
    rows_per_frame: f64,
}

impl HardDropAnimation {
    pub fn new(rows_per_frame: f64) -> Self {
        Self {
            state: None,
            rows_per_frame: rows_per_frame.max(0.01),
        }
    }

    pub fn update(&mut self, delta: Duration) {
        let mut finished = false;
        if let Some(state) = self.state.as_mut() {
            state.duration += delta.as_secs_f64();
            if state.duration < FRAME_DURATION {
                return;
            }

            let frame_delta = state.duration / FRAME_DURATION;
            state.duration = 0.0;
            state.frame += frame_delta.round() as u32;
            finished = state.frame >= state.max_frames;
        }
        if finished {
            self.state = None;
        }
    }

    pub fn reset(&mut self) {
        self.state = None;
    }

    pub fn hard_drop(&mut self, cells: &[PlacedCell], dropped_rows: u32) {
        if dropped_rows > 0 {
            self.state = Some(State::new(
                cells.to_vec(),
                dropped_rows,
                self.rows_per_frame,
            ));
        }
    }

    pub fn state(&self) -> Option<&State> {
        self.state.as_ref()
    }
}
