use std::time::{Duration, SystemTime};

/// The longest frame Android may report: SDL stops the loop while backgrounded, so the first
/// frame back would measure the whole absence.
#[cfg(feature = "android")]
const MAX_DELTA: Duration = Duration::from_millis(100);

#[derive(Debug, Copy, Clone)]
pub struct FrameRate {
    t0: SystemTime,
}

impl Default for FrameRate {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameRate {
    pub fn new() -> Self {
        Self {
            t0: SystemTime::now(),
        }
    }

    /// registers the start of a new frame, returning the time since the last one
    pub fn update(&mut self) -> Result<Duration, String> {
        // TODO have option of limiting/recording the effective framerate
        let now = SystemTime::now();
        let delta = now.duration_since(self.t0).map_err(|e| e.to_string())?;
        self.t0 = now;
        #[cfg(feature = "android")]
        let delta = delta.min(MAX_DELTA);
        Ok(delta)
    }
}
