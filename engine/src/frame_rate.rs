use std::time::{Duration, SystemTime};

/// The longest frame Android is allowed to report. SDL stops the loop dead inside its event
/// poll while the app is in the background, so the first frame back measures however long it
/// was away - minutes, all of which would land on the animations and particles at once. A
/// match has paused itself by then (`GameInputKey::Suspend`); this is for everything else.
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

    /// Registers the start of a new frame, returns the time since the last frame
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
