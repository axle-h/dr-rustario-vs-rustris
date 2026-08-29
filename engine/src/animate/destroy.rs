use crate::game::{CellId, PlacedCell};
use std::collections::HashMap;
use std::time::Duration;

const POP_DURATION: Duration = Duration::from_millis(300);
const MAX_FLASHES: u32 = 3;
const FLASH_DURATION: Duration = Duration::from_millis(250);
const SWEEP_DURATION: Duration = Duration::from_millis(750);

/// The tell before a pop: the group flashes on and off where it stands, still drawn exactly
/// as it sits on the board, and only then starts its strip.
///
/// It is what Mean Bean Machine does, and it is what makes a chain readable - a step
/// announces which group is going before it goes, so the eye is already in the right place.
/// A theme with no blink says nothing and its strip starts at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blink {
    /// how long the whole blink takes, before the strip starts
    pub duration: Duration,
    /// how many times the group goes out and comes back
    pub times: u32,
}

/// Which beat of a [`DestroyStyle::Pop`] a cell is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopPhase {
    /// still on the board as it stands, being flashed; `on` is whether it is drawn at all
    Blink { on: bool },
    /// playing its own pop strip
    Strip { frame: usize },
}

/// How cleared cells leave the board.
#[derive(Clone, Debug, PartialEq)]
pub enum DestroyStyle {
    /// each cell plays its own pop sprite strip
    Pop {
        default_frames: usize,
        frames: HashMap<CellId, usize>,
        /// how long the whole strip takes, and so how long the game is held
        duration: Duration,
        /// the tell before the strip, if the theme wants one
        blink: Option<Blink>,
        /// How long the strip's **first** frame is held before the rest of it runs.
        ///
        /// A pop is rarely evenly paced: Mean Bean Machine's bean pulls its face and holds
        /// it for a quarter of a second, and then goes in a hurry. Zero - the default -
        /// divides the strip evenly, which is what every other theme wants.
        hold_first: Duration,
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
            blink: None,
            hold_first: Duration::ZERO,
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

    /// flash the group on and off `times` before its strip starts, over `duration`
    ///
    /// It **adds** to the strip: a clear holds the board for both, so this lengthens a chain
    /// step by exactly what it asks for.
    pub fn blinking_for(mut self, duration: Duration, times: u32) -> Self {
        if let DestroyStyle::Pop { blink, .. } = &mut self {
            *blink = Some(Blink { duration, times });
        }
        self
    }

    /// hold the strip's first frame this long before the rest of it runs
    pub fn holding_first(mut self, hold: Duration) -> Self {
        if let DestroyStyle::Pop { hold_first, .. } = &mut self {
            *hold_first = hold;
        }
        self
    }

    fn blink(&self) -> Option<Blink> {
        match self {
            DestroyStyle::Pop { blink, .. } => *blink,
            _ => None,
        }
    }

    /// how long the whole clear holds the board, the tell and the strip together
    fn duration(&self) -> Duration {
        match self {
            DestroyStyle::Pop {
                duration, blink, ..
            } => *duration + blink.map_or(Duration::ZERO, |b| b.duration),
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

    /// Which beat a cell being destroyed is on (`Pop` style).
    ///
    /// The tell comes first and the strip after it, so the strip's own clock starts where the
    /// blink's ends rather than running underneath it.
    pub fn pop_phase(&self, id: CellId) -> Option<PopPhase> {
        let state = self.state.as_ref()?;
        let mut elapsed = state.duration;
        if let Some(blink) = self.style.blink() {
            if elapsed < blink.duration {
                // The group is **on** first: a clear that began by taking the group away
                // reads as a cell deleted rather than as a warning that it is about to go.
                // Out and back is one time, so each is a lit half and a dark half.
                let half = blink.duration / (blink.times * 2).max(1);
                let step = elapsed.as_nanos() / half.as_nanos().max(1);
                return Some(PopPhase::Blink { on: step % 2 == 0 });
            }
            elapsed -= blink.duration;
        }
        let frames = self.style.pop_frames(id);
        let (strip, hold_first) = match &self.style {
            DestroyStyle::Pop {
                duration,
                hold_first,
                ..
            } => (*duration, *hold_first),
            _ => (self.style.duration(), Duration::ZERO),
        };
        // The first frame's slot is at least its even share, and `hold_first` where that is
        // longer; whatever is left over is shared out among the rest. Asking for a hold
        // shorter than an even share therefore changes nothing, which is what a theme that
        // asks for none is doing.
        let first = hold_first.max(strip / frames as u32);
        if frames <= 1 || elapsed < first {
            return Some(PopPhase::Strip { frame: 0 });
        }
        let rest = strip.saturating_sub(first) / (frames - 1) as u32;
        let frame = 1 + ((elapsed - first).as_nanos() / rest.as_nanos().max(1)) as usize;
        Some(PopPhase::Strip {
            frame: frame.min(frames - 1),
        })
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::game::geometry::Point;

    const ID: CellId = CellId(0);

    fn popping(style: DestroyStyle) -> DestroyAnimation {
        let mut destroy = DestroyAnimation::new(style);
        destroy.add(vec![(Point::new(0, 0), ID)]);
        destroy
    }

    /// the tell runs first and the strip after it, rather than the two sharing one clock
    #[test]
    fn a_blink_goes_before_the_strip_and_lengthens_the_clear() {
        let strip = Duration::from_millis(400);
        let blink = Duration::from_millis(300);
        let style = DestroyStyle::pop(4)
            .for_duration(strip)
            .blinking_for(blink, 3);
        assert_eq!(style.duration(), strip + blink, "a clear holds for both");

        let mut destroy = popping(style);
        assert!(
            matches!(destroy.pop_phase(ID), Some(PopPhase::Blink { on: true })),
            "a clear begins by showing the group, not by taking it away"
        );
        destroy.update(blink);
        assert_eq!(
            destroy.pop_phase(ID),
            Some(PopPhase::Strip { frame: 0 }),
            "the strip starts where the blink ends, not underneath it"
        );
        destroy.update(strip / 2);
        assert_eq!(destroy.pop_phase(ID), Some(PopPhase::Strip { frame: 2 }));
        destroy.update(strip);
        assert_eq!(destroy.pop_phase(ID), None, "and then the clear is over");
    }

    #[test]
    fn a_blink_goes_out_and_back_the_number_of_times_it_is_asked_for() {
        let blink = Duration::from_millis(300);
        let mut destroy = popping(DestroyStyle::pop(1).blinking_for(blink, 3));
        let mut seen = vec![];
        for _ in 0..6 {
            seen.push(destroy.pop_phase(ID));
            destroy.update(blink / 6);
        }
        assert_eq!(
            seen,
            vec![
                Some(PopPhase::Blink { on: true }),
                Some(PopPhase::Blink { on: false }),
                Some(PopPhase::Blink { on: true }),
                Some(PopPhase::Blink { on: false }),
                Some(PopPhase::Blink { on: true }),
                Some(PopPhase::Blink { on: false }),
            ]
        );
    }

    /// the beat the original actually has: the bean pulls its face and *holds* it, and then
    /// goes in a hurry
    #[test]
    fn the_first_frame_of_a_strip_may_be_held() {
        let strip = Duration::from_millis(400);
        let style = DestroyStyle::pop(3)
            .for_duration(strip)
            .holding_first(Duration::from_millis(200));
        let mut destroy = popping(style);
        assert_eq!(destroy.pop_phase(ID), Some(PopPhase::Strip { frame: 0 }));
        destroy.update(Duration::from_millis(150));
        assert_eq!(
            destroy.pop_phase(ID),
            Some(PopPhase::Strip { frame: 0 }),
            "still held, where an even strip would be on its second frame"
        );
        destroy.update(Duration::from_millis(60));
        assert_eq!(destroy.pop_phase(ID), Some(PopPhase::Strip { frame: 1 }));
        destroy.update(Duration::from_millis(100));
        assert_eq!(destroy.pop_phase(ID), Some(PopPhase::Strip { frame: 2 }));
    }

    /// a theme that asks for no tell gets none, and its strip starts on the frame it always did
    #[test]
    fn without_a_blink_the_strip_starts_at_once() {
        let style = DestroyStyle::pop(4).for_duration(Duration::from_millis(400));
        assert_eq!(style.duration(), Duration::from_millis(400));
        assert_eq!(
            popping(style).pop_phase(ID),
            Some(PopPhase::Strip { frame: 0 })
        );
    }
}
