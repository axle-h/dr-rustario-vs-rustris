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

// The drain, measured off a capture of Kirby's Avalanche by cross-correlating each column
// band of every frame against the first, which gives that column's displacement to the pixel.
// Its six columns started 0.35s apart in a scattered order, and each accelerated at
// 2226-2554 px/s^2 at a 76px cell without ever reaching a terminal speed.
const DRAIN_STAGGER: Duration = Duration::from_millis(350);
const DRAIN_ACCELERATION: f64 = 30.0;
// the pause before the first column moves is *not* in that capture, which opens mid-fall:
// long enough to read as a pause and not as a hang, and chosen rather than measured
const DRAIN_HOLD: Duration = Duration::from_millis(300);
// ... and the beat after the last column has gone, so the empty well is seen
const DRAIN_EMPTY_FOR: Duration = Duration::from_millis(500);

/// How a lost game is shown.
// no `Eq`: the drain carries an acceleration
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameOverStyle {
    /// after a pause a "game over" graphic with `frames` frames cycles over the board
    Screen { frames: usize },
    /// rows of blocks fill the board like a curtain, from the top or the bottom. `rows` is
    /// how much of the board it covers, counted up from the floor, so a board drawn with a
    /// buffer zone above its skyline keeps it: nothing ever comes to rest up there
    Curtain { from_top: bool, rows: u32 },
    /// After a pause every column falls straight down and off the bottom of the board, each
    /// at its own moment: Puyo's own game over, and what all three of its themes play.
    ///
    /// A column falls as one rigid block, so the gaps in it are carried with it: nothing
    /// re-settles, nothing compacts, and a hole in the middle of a column is still a hole as
    /// it leaves the screen. `rows` is how far a column has to fall to be gone, which is the
    /// visible board's height.
    Drain {
        rows: u32,
        /// before the first column moves
        hold: Duration,
        /// the most any one column is held back by, on top of the hold
        stagger: Duration,
        /// rows a second squared, with no terminal speed: the board is not tall enough to
        /// reach one
        acceleration: f64,
    },
}

impl GameOverStyle {
    /// the drain as it was measured, over a board `rows` tall
    pub fn drain(rows: u32) -> Self {
        Self::Drain {
            rows,
            hold: DRAIN_HOLD,
            stagger: DRAIN_STAGGER,
            acceleration: DRAIN_ACCELERATION,
        }
    }
}

/// The drain in flight: how far down each column has slid, in rows.
///
/// Handed to the renderer, which adds it to the `offset_y` of every cell it is already
/// drawing. Nothing about the rules moves - the board still reports its cells exactly where
/// they were left, and this is only where they are *drawn* on the way out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drain {
    elapsed: Duration,
    hold: Duration,
    stagger: Duration,
    acceleration: f64,
}

impl Drain {
    /// How long a column is held back before it starts to fall.
    ///
    /// The golden ratio walk that [`crate::animate::nuisance`] staggers a falling attack
    /// with: neighbouring columns are given very different offsets, so the field breaks up
    /// rather than tilting, and it is a hash and not a random number - the same board drains
    /// the same way every time and a test can say so.
    fn delay(&self, column: i32) -> f64 {
        const GOLDEN: f64 = 0.618_033_988_749_895;
        self.stagger.as_secs_f64() * (column as f64 * GOLDEN).fract().abs()
    }

    /// how far this column has fallen, in rows, and down the board like every other
    /// `offset_y`
    pub fn offset(&self, column: i32) -> f64 {
        let seconds = self.elapsed.as_secs_f64() - self.hold.as_secs_f64() - self.delay(column);
        if seconds <= 0.0 {
            return 0.0;
        }
        0.5 * self.acceleration * seconds * seconds
    }

    /// how long a drain over a board `rows` tall takes, worst column and the beat after it
    fn duration(rows: u32, hold: Duration, stagger: Duration, acceleration: f64) -> Duration {
        let falling = (2.0 * rows as f64 / acceleration.max(f64::EPSILON)).sqrt();
        hold + stagger + Duration::from_secs_f64(falling) + DRAIN_EMPTY_FOR
    }
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
            // the drain runs its own length: there is no card to read afterwards, so it is
            // over once the last column is off the board and a beat has passed
            state.is_complete = match self.style {
                GameOverStyle::Drain {
                    rows,
                    hold,
                    stagger,
                    acceleration,
                } => state.duration >= Drain::duration(rows, hold, stagger, acceleration),
                _ => state.duration >= (GAME_OVER_SCREEN_DELAY + GAME_OVER_SCREEN_VISIBLE_FOR),
            };
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
            GameOverStyle::Screen { .. } | GameOverStyle::Drain { .. } => None,
        }
    }

    /// the drain in flight, once a game has been lost on a theme that plays one
    /// (`Drain` style)
    pub fn drain(&self) -> Option<Drain> {
        let GameOverStyle::Drain {
            hold,
            stagger,
            acceleration,
            ..
        } = self.style
        else {
            return None;
        };
        let state = self.state.as_ref()?;
        Some(Drain {
            elapsed: state.duration,
            hold,
            stagger,
            acceleration,
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    const ROWS: u32 = 12;

    fn drained(after: Duration) -> GameOverAnimation {
        let mut animation = GameOverAnimation::new(GameOverStyle::drain(ROWS), None);
        animation.game_over();
        animation.update(after);
        animation
    }

    fn offsets(after: Duration) -> Vec<f64> {
        let drain = drained(after).drain().unwrap();
        (0..6).map(|column| drain.offset(column)).collect()
    }

    #[test]
    fn nothing_drains_until_a_game_is_lost() {
        let mut animation = GameOverAnimation::new(GameOverStyle::drain(ROWS), None);
        animation.update(Duration::from_secs(1));
        assert!(animation.drain().is_none());
        assert!(animation.curtain_height().is_none());
    }

    /// the pause that reads as a pause: the board is still whole when it starts
    #[test]
    fn the_field_holds_still_before_it_falls() {
        assert!(offsets(DRAIN_HOLD).iter().all(|offset| *offset == 0.0));
        assert!(offsets(DRAIN_HOLD + Duration::from_millis(50))
            .iter()
            .any(|offset| *offset > 0.0));
    }

    /// every column has its own start and neighbours are deliberately unalike, which is what
    /// the golden ratio walk buys over a sweep
    #[test]
    fn the_columns_leave_in_a_scattered_order() {
        let started = DRAIN_HOLD + DRAIN_STAGGER / 2;
        let moving = offsets(started)
            .iter()
            .map(|offset| *offset > 0.0)
            .collect::<Vec<_>>();
        assert_eq!(moving, vec![true, false, true, false, true, true]);

        let mut order = (0..6).collect::<Vec<i32>>();
        let drain = drained(started).drain().unwrap();
        order.sort_by(|a, b| drain.delay(*a).total_cmp(&drain.delay(*b)));
        assert_eq!(order, vec![0, 5, 2, 4, 1, 3]);
    }

    /// a hash and not a random number: the same board drains the same way twice
    #[test]
    fn the_same_board_drains_the_same_way_twice() {
        let at = DRAIN_HOLD + Duration::from_millis(400);
        assert_eq!(offsets(at), offsets(at));
    }

    /// it accelerates the whole way down rather than reaching a terminal speed: twice as long
    /// falling is four times as far
    #[test]
    fn a_column_accelerates_all_the_way_off_the_board() {
        let column = 0;
        let fallen = |after: Duration| drained(DRAIN_HOLD + after).drain().unwrap().offset(column);
        let first = fallen(Duration::from_millis(300));
        let second = fallen(Duration::from_millis(600));
        assert!((second / first - 4.0).abs() < 1e-9, "{first} then {second}");
    }

    /// the capture's own number: about 1.3s from the first column moving to the last one gone
    #[test]
    fn the_field_is_gone_about_a_second_and_a_third_after_it_starts() {
        let all = Drain::duration(ROWS, DRAIN_HOLD, DRAIN_STAGGER, DRAIN_ACCELERATION);
        let falling = all - DRAIN_HOLD - DRAIN_EMPTY_FOR;
        assert!(
            (falling.as_secs_f64() - 1.3).abs() < 0.1,
            "{falling:?} to empty the board"
        );
    }

    /// and it is over when the board is empty and a beat has passed, not on the game over
    /// screen's own clock: there is no card to read
    #[test]
    fn the_drain_ends_when_the_last_column_is_gone_and_a_beat_has_passed() {
        let all = Drain::duration(ROWS, DRAIN_HOLD, DRAIN_STAGGER, DRAIN_ACCELERATION);
        assert!(all < GAME_OVER_SCREEN_DELAY);

        let before = drained(all - Duration::from_millis(100));
        assert!(!before.state().unwrap().is_complete());
        let last = before.drain().unwrap();
        assert!((0..6).all(|column| last.offset(column) >= ROWS as f64));

        assert!(drained(all).state().unwrap().is_complete());
    }
}
