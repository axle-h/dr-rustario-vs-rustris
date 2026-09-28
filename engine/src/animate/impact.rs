//! What jolts the board: the ricochet of a hard drop, and an opt-in rumble when a slab of nuisance
//! lands (fed by `AnimationMeta::nuisance_rumble`). Only the board moves; the panel and its shadow
//! stay put.

use std::time::Duration;

const ACCELERATION: f64 = 10.0;
const SPEED: f64 = 5.0;
const LIMIT: f64 = 0.25;

/// how many times a rumble crosses the middle over its length
const RUMBLE_SHAKES: f64 = 7.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    Rest,
    Impacted {
        acceleration: f64,
        speed: f64,
    },
    Return {
        acceleration: f64,
        speed: f64,
    },
    /// a decaying two-axis shake, for a theme that wants one
    Rumble {
        elapsed: Duration,
        duration: Duration,
        amplitude: f64,
    },
}

#[derive(Clone, Debug)]
pub struct ImpactAnimation {
    offset_x: f64,
    offset_y: f64,
    state: State,
}

impl Default for ImpactAnimation {
    fn default() -> Self {
        Self::new()
    }
}

impl ImpactAnimation {
    pub fn new() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            state: State::Rest,
        }
    }

    pub fn update(&mut self, delta: Duration) {
        self.state = match self.state {
            State::Rest => State::Rest,
            State::Impacted {
                acceleration,
                speed,
            } => {
                let duration_secs = delta.as_secs_f64();
                let speed = speed + acceleration * duration_secs;
                self.offset_y += speed * duration_secs;
                if self.offset_y > LIMIT {
                    State::Return {
                        acceleration,
                        speed: 0.0,
                    }
                } else {
                    State::Impacted {
                        acceleration,
                        speed,
                    }
                }
            }
            State::Return {
                acceleration,
                speed,
            } => {
                let duration_secs = delta.as_secs_f64();
                let speed = speed - acceleration * duration_secs;
                self.offset_y += speed * duration_secs;
                if self.offset_y <= 0.0 {
                    self.offset_y = 0.0;
                    State::Rest
                } else {
                    State::Return {
                        acceleration,
                        speed,
                    }
                }
            }
            State::Rumble {
                elapsed,
                duration,
                amplitude,
            } => {
                let elapsed = elapsed + delta;
                if elapsed >= duration {
                    self.offset_x = 0.0;
                    self.offset_y = 0.0;
                    State::Rest
                } else {
                    // it dies away so the board settles rather than snapping back
                    let left = 1.0 - elapsed.as_secs_f64() / duration.as_secs_f64();
                    let phase = elapsed.as_secs_f64() / duration.as_secs_f64()
                        * RUMBLE_SHAKES
                        * std::f64::consts::TAU;
                    self.offset_x = amplitude * left * phase.sin();
                    // the vertical runs at double the rate, a shudder rather than a diagonal wobble
                    self.offset_y = amplitude * left * (phase * 2.0).cos();
                    State::Rumble {
                        elapsed,
                        duration,
                        amplitude,
                    }
                }
            }
        };
    }

    pub fn reset(&mut self) {
        self.offset_x = 0.0;
        self.offset_y = 0.0;
        self.state = State::Rest;
    }

    /// Shake the board, in fractions of a block. A rumble already running is restarted rather than
    /// added to, so a slab landing column by column shakes once.
    pub fn rumble(&mut self, amplitude: f64, duration: Duration) {
        if amplitude <= 0.0 || duration.is_zero() {
            return;
        }
        self.state = State::Rumble {
            elapsed: Duration::ZERO,
            duration,
            amplitude,
        };
    }

    pub fn impact(&mut self) {
        self.state = match self.state {
            State::Rest | State::Return { .. } | State::Rumble { .. } => State::Impacted {
                acceleration: ACCELERATION,
                speed: SPEED,
            },
            State::Impacted {
                acceleration,
                speed,
            } => State::Impacted {
                acceleration: acceleration + ACCELERATION,
                speed: speed + SPEED,
            },
        };
    }

    pub fn current_offset(&self) -> (f64, f64) {
        (self.offset_x, self.offset_y)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const LENGTH: Duration = Duration::from_millis(300);

    #[test]
    fn a_rumble_shakes_both_ways_and_dies_away() {
        let mut impact = ImpactAnimation::new();
        impact.rumble(0.2, LENGTH);
        let mut early = 0.0f64;
        let mut late = 0.0f64;
        for step in 0..30 {
            impact.update(LENGTH / 30);
            let (x, y) = impact.current_offset();
            if step < 10 {
                early = early.max(x.abs()).max(y.abs());
            } else if step >= 20 {
                late = late.max(x.abs()).max(y.abs());
            }
        }
        assert!(early > 0.0, "it moved");
        assert!(late < early, "and quieted down: {early} then {late}");
        assert_eq!(
            impact.current_offset(),
            (0.0, 0.0),
            "and comes to rest exactly true"
        );
    }

    /// A theme that asks for no rumble never shakes.
    #[test]
    fn a_rumble_of_nothing_is_nothing() {
        let mut impact = ImpactAnimation::new();
        impact.rumble(0.0, LENGTH);
        impact.update(LENGTH / 2);
        assert_eq!(impact.current_offset(), (0.0, 0.0));
    }

    /// A slab landing column by column shakes once.
    #[test]
    fn a_rumble_restarts_rather_than_piling_up() {
        let mut impact = ImpactAnimation::new();
        impact.rumble(0.2, LENGTH);
        impact.update(LENGTH / 2);
        impact.rumble(0.2, LENGTH);
        impact.update(LENGTH / 2);
        let (x, y) = impact.current_offset();
        assert!(x.abs() <= 0.2 && y.abs() <= 0.2, "never past its amplitude");
        impact.update(LENGTH);
        assert_eq!(impact.current_offset(), (0.0, 0.0));
    }
}
