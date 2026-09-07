use std::time::Duration;

/// How a strip of sprite frames is played back.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FrameAnimationType {
    Static,
    Linear {
        fps: u32,
    },
    /// Runs `0..n` and back, **repeating each end** - `0 1 2 2 1 0`.
    ///
    /// That is not the shape of a ping-pong as the games this compendium is drawn from play
    /// one: every one measured off a capture holds each end *once*. Nothing here uses this,
    /// and a ping-pong is instead cut unrolled into its strip and played as a plain
    /// [`FrameAnimationType::Linear`]. Reach for it only if a repeated end is what you want.
    YoYo {
        fps: u32,
    },
    /// Plays the strip and then waits, and **holds the last frame while it waits** - so a pass
    /// is `n/fps + pause_for` and a row has to be cut *action first, rest last*: the pose the
    /// sprite settles into is the one at the end of the strip, not the one at the start.
    LinearWithPause {
        fps: u32,
        pause_for: Duration,
        resume_from_frame: usize,
    },
}

impl FrameAnimationType {
    pub fn fps(&self) -> Option<u32> {
        match self {
            FrameAnimationType::Static => None,
            FrameAnimationType::Linear { fps }
            | FrameAnimationType::YoYo { fps }
            | FrameAnimationType::LinearWithPause { fps, .. } => Some(*fps),
        }
    }

    pub fn frame_duration(&self) -> Duration {
        self.fps()
            .map(|fps| Duration::from_secs(1) / fps)
            .unwrap_or(Duration::ZERO)
    }
}

/// Plays `max_frame` frames according to a [`FrameAnimationType`].
#[derive(Clone, Copy, Debug)]
pub struct FrameAnimation {
    animation_type: FrameAnimationType,
    duration: Duration,
    frame_duration: Duration,
    paused_for: Option<Duration>,
    frame: usize,
    invert: bool,
    iteration: usize,
    max_frame: usize,
}

impl FrameAnimation {
    pub fn new(animation_type: FrameAnimationType, max_frame: usize) -> Self {
        assert!(max_frame > 0);
        Self {
            animation_type,
            duration: Duration::ZERO,
            frame_duration: animation_type.frame_duration(),
            paused_for: None,
            frame: 0,
            invert: false,
            iteration: 0,
            max_frame,
        }
    }

    pub fn update(&mut self, delta: Duration) {
        self.duration += delta;
        match self.animation_type {
            FrameAnimationType::Static => {
                self.frame = 0;
                self.iteration = 0;
            }
            FrameAnimationType::Linear { .. } => self.next_linear(false),
            FrameAnimationType::YoYo { .. } => self.next_linear(true),
            FrameAnimationType::LinearWithPause {
                pause_for,
                resume_from_frame,
                ..
            } => {
                if let Some(paused_for) = self.paused_for {
                    // maybe unpause
                    self.paused_for = paused_for.checked_sub(delta);
                    if self.paused_for == Some(Duration::ZERO) {
                        self.paused_for = None;
                    }
                    if self.paused_for.is_none() {
                        self.duration = Duration::ZERO;
                        self.iteration += 1;
                        self.frame = resume_from_frame;
                    }
                } else {
                    self.register_frames();
                    if self.frame >= self.max_frame {
                        self.frame = self.max_frame - 1;
                        self.paused_for = Some(pause_for);
                    }
                }
            }
        }
    }

    fn register_frames(&mut self) {
        while let Some(remainder) = self.duration.checked_sub(self.frame_duration) {
            if self.frame_duration.is_zero() {
                break;
            }
            self.duration = remainder;
            self.frame += 1;
        }
    }

    fn next_linear(&mut self, invert: bool) {
        self.register_frames();
        if self.frame >= self.max_frame {
            self.iteration += 1;
            self.frame %= self.max_frame;
            if invert {
                self.invert = !self.invert;
            }
        }
    }

    pub fn reset(&mut self) {
        self.duration = Duration::ZERO;
        self.frame = 0;
        self.invert = false;
        self.paused_for = None;
    }

    pub fn frame(&self) -> usize {
        if self.invert {
            self.max_frame - self.frame - 1
        } else {
            self.frame
        }
    }

    pub fn iteration(&self) -> usize {
        self.iteration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_wraps_and_counts_iterations() {
        let mut a = FrameAnimation::new(FrameAnimationType::Linear { fps: 10 }, 3);
        a.update(Duration::from_millis(250));
        assert_eq!(a.frame(), 2);
        a.update(Duration::from_millis(100));
        assert_eq!(a.frame(), 0);
        assert_eq!(a.iteration(), 1);
    }

    #[test]
    fn yo_yo_reverses() {
        let mut a = FrameAnimation::new(FrameAnimationType::YoYo { fps: 10 }, 3);
        a.update(Duration::from_millis(300));
        assert_eq!(a.frame(), 2);
        a.update(Duration::from_millis(100));
        assert_eq!(a.frame(), 1);
    }

    #[test]
    fn pause_holds_last_frame_then_resumes() {
        let t = FrameAnimationType::LinearWithPause {
            fps: 10,
            pause_for: Duration::from_millis(200),
            resume_from_frame: 1,
        };
        let mut a = FrameAnimation::new(t, 2);
        a.update(Duration::from_millis(200));
        assert_eq!(a.frame(), 1);
        a.update(Duration::from_millis(100));
        assert_eq!(a.frame(), 1);
        assert_eq!(a.iteration(), 0);
        a.update(Duration::from_millis(100));
        assert_eq!(a.frame(), 1);
        assert_eq!(a.iteration(), 1);
    }
}
