use crate::animate::frames::FrameAnimation;
use crate::animate::mascot::MascotMeta;
use std::ops::Range;
use std::time::Duration;

// delay until game over screen is displayed
const GAME_OVER_SCREEN_DELAY: Duration = Duration::from_secs(3);

// delay after game over screen visible that the game over animation is complete
const GAME_OVER_SCREEN_VISIBLE_FOR: Duration = Duration::from_secs(3);

const GAME_OVER_FRAME_DURATION: Duration = Duration::from_millis(300);

const CURTAIN_LINE_DELAY: Duration = Duration::from_millis(30);
const CURTAIN_CLOSED_FOR: Duration = Duration::from_millis(2000);

/// How a lost game is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameOverStyle {
    /// after a pause a "game over" graphic with `frames` frames cycles over the board
    Screen { frames: usize },
    /// rows of blocks fill the board like a curtain, from the top or the bottom. `rows` is
    /// how much of the board it covers, counted up from the floor, so a board drawn with a
    /// buffer zone above its skyline keeps it: nothing ever comes to rest up there
    Curtain { from_top: bool, rows: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurtainPhase {
    Closing,
    Closed,
    Opening,
}

#[derive(Clone, Debug)]
pub struct State {
    duration: Duration,
    mascot: Option<FrameAnimation>,
    screen_frame: usize,
    is_complete: bool,
    is_dismissed: bool,
}

impl State {
    pub fn is_complete(&self) -> bool {
        self.is_complete || self.is_dismissed
    }

    pub fn is_dismissed(&self) -> bool {
        self.is_dismissed
    }

    pub fn mascot_frame(&self) -> Option<usize> {
        self.mascot.map(|m| m.frame())
    }

    /// the game over graphic frame, once the graphic should be visible (`Screen` style)
    pub fn screen_frame(&self) -> Option<usize> {
        if self.duration >= GAME_OVER_SCREEN_DELAY {
            Some(self.screen_frame)
        } else {
            None
        }
    }

    fn is_shown(&self) -> bool {
        self.duration >= GAME_OVER_SCREEN_DELAY
    }
}

#[derive(Clone, Debug)]
pub struct GameOverAnimation {
    style: GameOverStyle,
    mascot: Option<MascotMeta>,
    state: Option<State>,
}

impl GameOverAnimation {
    pub fn new(style: GameOverStyle, mascot: Option<MascotMeta>) -> Self {
        Self {
            style,
            mascot,
            state: None,
        }
    }

    pub fn style(&self) -> GameOverStyle {
        self.style
    }

    pub fn update(&mut self, delta: Duration) {
        if let Some(state) = self.state.as_mut() {
            state.duration += delta;
            if let Some(mascot) = state.mascot.as_mut() {
                mascot.update(delta);
            }
            state.is_complete =
                state.duration >= (GAME_OVER_SCREEN_DELAY + GAME_OVER_SCREEN_VISIBLE_FOR);
            if let GameOverStyle::Screen { frames } = self.style {
                state.screen_frame =
                    (state.duration.as_millis() / GAME_OVER_FRAME_DURATION.as_millis()) as usize
                        % frames.max(1);
            }
        }
    }

    pub fn game_over(&mut self) {
        self.state = Some(State {
            duration: Duration::ZERO,
            mascot: self.mascot.map(|m| m.game_over()),
            screen_frame: 0,
            is_complete: false,
            is_dismissed: false,
        });
    }

    pub fn state(&self) -> Option<&State> {
        self.state.as_ref()
    }

    fn curtain(&self) -> Option<(CurtainPhase, Range<u32>)> {
        let GameOverStyle::Curtain { from_top, rows } = self.style else {
            return None;
        };
        let state = self.state.as_ref()?;
        let closing = CURTAIN_LINE_DELAY * rows;
        let (phase, covered) = if state.duration < closing {
            (
                CurtainPhase::Closing,
                (state.duration.as_millis() / CURTAIN_LINE_DELAY.as_millis()) as u32,
            )
        } else if state.duration < closing + CURTAIN_CLOSED_FOR {
            (CurtainPhase::Closed, rows)
        } else {
            let opening = state.duration - closing - CURTAIN_CLOSED_FOR;
            (
                CurtainPhase::Opening,
                rows.saturating_sub((opening.as_millis() / CURTAIN_LINE_DELAY.as_millis()) as u32),
            )
        };
        let range = if from_top {
            0..covered
        } else {
            (rows - covered)..rows
        };
        Some((phase, range))
    }

    /// the rows currently covered by the curtain, counted from the top of its span
    /// (`Curtain` style)
    pub fn curtain_rows(&self) -> Option<Range<u32>> {
        self.curtain().map(|(_, rows)| rows)
    }

    /// how much of the board the curtain covers, counted up from the floor
    pub fn curtain_height(&self) -> Option<u32> {
        match self.style {
            GameOverStyle::Curtain { rows, .. } => Some(rows),
            GameOverStyle::Screen { .. } => None,
        }
    }

    pub fn curtain_phase(&self) -> Option<CurtainPhase> {
        self.curtain().map(|(phase, _)| phase)
    }

    /// dismiss the game over screen, but only once it has been shown: keys still held from
    /// the end of the match auto-repeat and would otherwise skip straight past it
    pub fn dismiss(&mut self) {
        if let Some(state) = self.state.as_mut() {
            if state.is_shown() {
                state.is_dismissed = true;
            }
        }
    }
}
