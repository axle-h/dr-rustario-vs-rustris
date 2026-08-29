use crate::game::{CellId, PlacedCell};
use std::collections::HashMap;
use std::time::Duration;

const POP_DURATION: Duration = Duration::from_millis(300);
const MAX_FLASHES: u32 = 3;
const FLASH_DURATION: Duration = Duration::from_millis(250);
const SWEEP_DURATION: Duration = Duration::from_millis(750);

/// How cleared cells leave the board.
#[derive(Clone, Debug, PartialEq)]
pub enum DestroyStyle {
    /// each cell plays its own pop sprite strip
    Pop {
        default_frames: usize,
        frames: HashMap<CellId, usize>,
        /// how long the whole strip takes, and so how long the game is held
        duration: Duration,
    },
    /// the cleared cells flash on and off
    Flash,
    /// a wipe sweeps across the cleared cells
    Sweep,
    /// the cells just disappear (particles do the work); the game holds for `hold`
    Vanish { hold: Duration },
}

impl DestroyStyle {
    pub fn pop(default_frames: usize) -> Self {
        Self::Pop {
            default_frames,
            frames: HashMap::new(),
            duration: POP_DURATION,
        }
    }

    /// how long the strip takes. The game does not tick while a clear plays, so a theme whose
    /// game already waits on a clear of its own sets this under that wait and costs it
    /// nothing; left alone it is three hundred milliseconds.
    pub fn for_duration(mut self, wanted: Duration) -> Self {
        if let DestroyStyle::Pop { duration, .. } = &mut self {
            *duration = wanted;
        }
        self
    }

    pub fn with_pop_frames(mut self, id: CellId, count: usize) -> Self {
        if let DestroyStyle::Pop { frames, .. } = &mut self {
            frames.insert(id, count);
        }
        self
    }

    fn duration(&self) -> Duration {
        match self {
            DestroyStyle::Pop { duration, .. } => *duration,
            DestroyStyle::Flash => FLASH_DURATION * MAX_FLASHES,
            DestroyStyle::Sweep => SWEEP_DURATION,
            DestroyStyle::Vanish { hold } => *hold,
        }
    }

    fn pop_frames(&self, id: CellId) -> usize {
        match self {
            DestroyStyle::Pop {
                default_frames,
                frames,
                ..
            } => frames.get(&id).copied().unwrap_or(*default_frames).max(1),
            _ => 1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct State {
    cells: Vec<PlacedCell>,
    duration: Duration,
}

impl State {
    pub fn cells(&self) -> &[PlacedCell] {
        &self.cells
    }
}

#[derive(Clone, Debug)]
pub struct DestroyAnimation {
    style: DestroyStyle,
    state: Option<State>,
}

impl DestroyAnimation {
    pub fn new(style: DestroyStyle) -> Self {
        Self { style, state: None }
    }

    pub fn style(&self) -> &DestroyStyle {
        &self.style
    }

    pub fn update(&mut self, delta: Duration) {
        if let Some(state) = self.state.as_mut() {
            state.duration += delta;
            if state.duration >= self.style.duration() {
                self.state = None;
            }
        }
    }

    pub fn reset(&mut self) {
        self.state = None;
    }

    pub fn add(&mut self, cells: Vec<PlacedCell>) {
        self.state = Some(State {
            cells,
            duration: Duration::ZERO,
        })
    }

    pub fn state(&self) -> Option<&State> {
        self.state.as_ref()
    }

    /// the pop frame for a cell being destroyed (`Pop` style)
    pub fn pop_frame(&self, id: CellId) -> Option<usize> {
        let state = self.state.as_ref()?;
        let frames = self.style.pop_frames(id);
        let frame_duration = self.style.duration() / frames as u32;
        Some((state.duration.as_millis() / frame_duration.as_millis().max(1)) as usize % frames)
    }

    /// whether the cleared cells are currently hidden (`Flash` style)
    pub fn is_flashed_off(&self) -> bool {
        match (&self.style, &self.state) {
            (DestroyStyle::Flash, Some(state)) => {
                (state.duration.as_millis() / FLASH_DURATION.as_millis()) % 2 == 0
            }
            _ => false,
        }
    }

    /// how far across the cleared cells the wipe has swept, 0..1 (`Sweep` style)
    pub fn sweep_progress(&self) -> Option<f64> {
        match (&self.style, &self.state) {
            (DestroyStyle::Sweep, Some(state)) => {
                Some((state.duration.as_secs_f64() / SWEEP_DURATION.as_secs_f64()).min(1.0))
            }
            _ => None,
        }
    }
}
