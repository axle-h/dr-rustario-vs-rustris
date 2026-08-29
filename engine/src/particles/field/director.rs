//! What the field is doing and what it does next: a small state machine over a resting
//! ambient routine and the occasional feature, with a weighted playlist that never repeats a
//! feature twice running.

use crate::particles::field::{field_rng, FieldRng};
use rand::RngExt;

/// the resting state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ambient {
    /// the gravity well the background has always used, ported in so the current look is
    /// never lost
    Orbit,
    /// sum-of-sines advection: organic, and it allocates nothing
    Flow,
    /// angular velocity falling off with distance: arms form and shear
    Vortex,
    /// slow drift with links drawn between near neighbours
    Constellation,
}

impl Ambient {
    pub const ALL: [Ambient; 4] = [
        Ambient::Orbit,
        Ambient::Flow,
        Ambient::Vortex,
        Ambient::Constellation,
    ];

    /// how often each comes up between features. The constellation is the one people look at,
    /// so it comes up about as often as the other three together.
    const PLAYLIST: [(Ambient, f64); 4] = [
        (Ambient::Constellation, 3.0),
        (Ambient::Flow, 1.2),
        (Ambient::Vortex, 1.2),
        (Ambient::Orbit, 1.0),
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    /// the headline: gather into a silhouette, hold and drift, shatter
    Sprite,
    /// snap to a rectilinear grid, breathe, then shear and collapse
    Lattice,
    /// a waveform ribbon across the canvas
    Oscilloscope,
    /// directional rain or embers, tied to how fast the pieces are falling
    Weather,
    /// concentric rings drifting about the canvas
    Haloes,
    /// an arm winding out of a wandering centre
    Spiral,
    /// a closed figure traced by two perpendicular sines: the oscilloscope's other trace
    Lissajous,
    /// the boards themselves pull the field about
    Wells,
    /// the same machinery as the sprite morph, over a mask taken from rendered text
    Text,
}

impl Feature {
    /// how often each comes up. The sprite morph is the one worth waiting for, and the
    /// curves - waveform, rings, spiral, figure - carry most of the rest between them.
    const PLAYLIST: [(Feature, f64); 9] = [
        (Feature::Sprite, 5.0),
        (Feature::Oscilloscope, 3.0),
        (Feature::Haloes, 2.5),
        (Feature::Lissajous, 2.5),
        (Feature::Spiral, 2.0),
        (Feature::Lattice, 1.5),
        (Feature::Text, 1.2),
        (Feature::Weather, 1.0),
        (Feature::Wells, 1.0),
    ];

    /// how long its hold lasts. The ones that move while they hold earn a longer one.
    fn hold_secs(&self) -> f64 {
        match self {
            Feature::Sprite => 3.5,
            Feature::Lattice => 2.5,
            Feature::Oscilloscope => 4.5,
            Feature::Weather => 5.0,
            Feature::Haloes => 5.0,
            Feature::Spiral => 4.5,
            Feature::Lissajous => 5.0,
            Feature::Wells => 4.0,
            Feature::Text => 3.0,
        }
    }

    /// whether it wants the pool gathered onto targets at all
    pub fn is_formation(&self) -> bool {
        !matches!(self, Feature::Weather | Feature::Wells)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// easing into the targets
    Gather,
    /// there, and moving
    Hold,
    /// an outward shockwave and back to ambient
    Shatter,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stage {
    Ambient {
        routine: Ambient,
        remaining: f64,
    },
    Feature {
        routine: Feature,
        phase: Phase,
        /// since this phase began
        elapsed: f64,
        remaining: f64,
    },
}

/// What the field must do about a change of stage this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition {
    None,
    /// settle into a resting routine
    Ambient(Ambient),
    /// build a formation and hand the pool over to it
    Gather(Feature),
    /// the formation is made
    Hold(Feature),
    /// blow it apart
    Shatter(Feature),
}

const GATHER_SECS: f64 = 1.2;
const SHATTER_SECS: f64 = 1.0;
/// how long the field rests between features. Short: the formations are the point, and the
/// resting routines are what carries the field from one to the next rather than the main act.
const AMBIENT_MIN_SECS: f64 = 2.5;
const AMBIENT_MAX_SECS: f64 = 5.5;

pub struct Director {
    stage: Stage,
    last_feature: Option<Feature>,
    last_ambient: Option<Ambient>,
    rng: FieldRng,
}

impl Director {
    pub fn new(rng: FieldRng) -> Self {
        let mut director = Self {
            stage: Stage::Ambient {
                routine: Ambient::Orbit,
                remaining: AMBIENT_MIN_SECS,
            },
            last_feature: None,
            last_ambient: Some(Ambient::Orbit),
            rng,
        };
        director.stage = Stage::Ambient {
            routine: Ambient::Orbit,
            remaining: director.ambient_secs(),
        };
        director
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    /// how far through the current phase, 0-1
    pub fn phase_progress(&self) -> f64 {
        match self.stage {
            Stage::Ambient { .. } => 1.0,
            Stage::Feature {
                elapsed, remaining, ..
            } => {
                let total = elapsed + remaining.max(0.0);
                if total <= 0.0 {
                    1.0
                } else {
                    (elapsed / total).clamp(0.0, 1.0)
                }
            }
        }
    }

    /// seconds since the current phase began, which is what the formations move against
    pub fn phase_elapsed(&self) -> f64 {
        match self.stage {
            Stage::Ambient { .. } => 0.0,
            Stage::Feature { elapsed, .. } => elapsed,
        }
    }

    /// `energy` shortens the wait: a match being played hard reaches for a feature sooner
    pub fn update(&mut self, delta_time: f64, energy: f64) -> Transition {
        match &mut self.stage {
            Stage::Ambient { remaining, .. } => {
                *remaining -= delta_time * (1.0 + energy);
                if *remaining > 0.0 {
                    return Transition::None;
                }
                let feature = self.next_feature();
                self.begin_feature(feature)
            }
            Stage::Feature {
                routine,
                phase,
                elapsed,
                remaining,
            } => {
                *elapsed += delta_time;
                *remaining -= delta_time;
                if *remaining > 0.0 {
                    return Transition::None;
                }
                let routine = *routine;
                match phase {
                    Phase::Gather => {
                        self.stage = Stage::Feature {
                            routine,
                            phase: Phase::Hold,
                            elapsed: 0.0,
                            remaining: routine.hold_secs(),
                        };
                        Transition::Hold(routine)
                    }
                    Phase::Hold => {
                        self.stage = Stage::Feature {
                            routine,
                            phase: Phase::Shatter,
                            elapsed: 0.0,
                            remaining: SHATTER_SECS,
                        };
                        Transition::Shatter(routine)
                    }
                    Phase::Shatter => {
                        let ambient = self.next_ambient();
                        self.stage = Stage::Ambient {
                            routine: ambient,
                            remaining: self.ambient_secs(),
                        };
                        self.last_ambient = Some(ambient);
                        Transition::Ambient(ambient)
                    }
                }
            }
        }
    }

    /// the big one: a Tetris or a four virus clear takes the field over whatever it was doing
    pub fn interrupt(&mut self, feature: Feature) -> Transition {
        self.begin_feature(feature)
    }

    /// back to ambient right now, with no shatter. Used when the canvas changes under a
    /// half-finished routine, which is never worth trying to keep.
    pub fn abandon(&mut self) -> Transition {
        let ambient = self.last_ambient.unwrap_or(Ambient::Orbit);
        self.stage = Stage::Ambient {
            routine: ambient,
            remaining: self.ambient_secs(),
        };
        Transition::Ambient(ambient)
    }

    fn begin_feature(&mut self, feature: Feature) -> Transition {
        self.stage = Stage::Feature {
            routine: feature,
            phase: Phase::Gather,
            elapsed: 0.0,
            remaining: GATHER_SECS,
        };
        self.last_feature = Some(feature);
        Transition::Gather(feature)
    }

    fn ambient_secs(&mut self) -> f64 {
        AMBIENT_MIN_SECS + (AMBIENT_MAX_SECS - AMBIENT_MIN_SECS) * self.rng.random::<f64>()
    }

    fn next_ambient(&mut self) -> Ambient {
        let choices = Ambient::PLAYLIST
            .into_iter()
            .filter(|(a, _)| Some(*a) != self.last_ambient)
            .collect::<Vec<(Ambient, f64)>>();
        Self::weighted(&mut self.rng, &choices)
    }

    fn next_feature(&mut self) -> Feature {
        let choices = Feature::PLAYLIST
            .into_iter()
            .filter(|(f, _)| Some(*f) != self.last_feature)
            .collect::<Vec<(Feature, f64)>>();
        Self::weighted(&mut self.rng, &choices)
    }

    fn weighted<T: Copy>(rng: &mut FieldRng, choices: &[(T, f64)]) -> T {
        let total: f64 = choices.iter().map(|(_, weight)| weight).sum();
        let mut pick = rng.random::<f64>() * total;
        for (choice, weight) in choices.iter() {
            pick -= weight;
            if pick <= 0.0 {
                return *choice;
            }
        }
        choices.last().unwrap().0
    }
}

impl Default for Director {
    fn default() -> Self {
        Self::new(field_rng())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particles::field::test_rng;

    fn run(director: &mut Director, seconds: f64) -> Vec<Transition> {
        let mut transitions = vec![];
        let steps = (seconds / 0.016).round() as u32;
        for _ in 0..steps {
            match director.update(0.016, 0.0) {
                Transition::None => {}
                other => transitions.push(other),
            }
        }
        transitions
    }

    #[test]
    fn a_feature_runs_gather_then_hold_then_shatter_then_ambient() {
        let mut director = Director::new(test_rng());
        let transitions = run(&mut director, 60.0);
        let features = transitions
            .iter()
            .filter(|t| matches!(t, Transition::Gather(_)))
            .count();
        assert!(features >= 2, "{transitions:?}");
        // every gather is followed by a hold, a shatter and a return to ambient
        let mut iter = transitions
            .iter()
            .skip_while(|t| !matches!(t, Transition::Gather(_)));
        let Some(Transition::Gather(feature)) = iter.next() else {
            panic!("{transitions:?}")
        };
        assert_eq!(iter.next(), Some(&Transition::Hold(*feature)));
        assert_eq!(iter.next(), Some(&Transition::Shatter(*feature)));
        assert!(matches!(iter.next(), Some(Transition::Ambient(_))));
    }

    #[test]
    fn a_feature_never_repeats_twice_running() {
        let mut director = Director::new(test_rng());
        let features = run(&mut director, 600.0)
            .into_iter()
            .filter_map(|t| match t {
                Transition::Gather(feature) => Some(feature),
                _ => None,
            })
            .collect::<Vec<Feature>>();
        assert!(features.len() > 5, "{features:?}");
        assert!(features.windows(2).all(|w| w[0] != w[1]), "{features:?}");
    }

    #[test]
    fn an_ambient_routine_never_repeats_twice_running() {
        let mut director = Director::new(test_rng());
        let ambients = run(&mut director, 600.0)
            .into_iter()
            .filter_map(|t| match t {
                Transition::Ambient(ambient) => Some(ambient),
                _ => None,
            })
            .collect::<Vec<Ambient>>();
        assert!(ambients.len() > 5, "{ambients:?}");
        assert!(ambients.windows(2).all(|w| w[0] != w[1]), "{ambients:?}");
    }

    #[test]
    fn the_field_spends_more_of_its_time_in_a_feature_than_resting() {
        let mut director = Director::new(test_rng());
        let mut featuring = 0.0;
        let mut total = 0.0;
        for _ in 0..(600.0 / 0.016) as u32 {
            director.update(0.016, 0.0);
            total += 0.016;
            if matches!(director.stage(), Stage::Feature { .. }) {
                featuring += 0.016;
            }
        }
        let share = featuring / total;
        assert!(share > 0.55, "only {share:.2} of the time in a feature");
    }

    #[test]
    fn every_feature_comes_up_over_a_long_enough_match() {
        let mut director = Director::new(test_rng());
        let seen = run(&mut director, 3_000.0)
            .into_iter()
            .filter_map(|t| match t {
                Transition::Gather(feature) => Some(feature),
                _ => None,
            })
            .collect::<Vec<Feature>>();
        for (feature, _) in Feature::PLAYLIST {
            assert!(seen.contains(&feature), "{feature:?} never came up");
        }
    }

    #[test]
    fn an_interrupt_takes_the_field_over_whatever_it_was_doing() {
        let mut director = Director::new(test_rng());
        assert_eq!(
            director.interrupt(Feature::Sprite),
            Transition::Gather(Feature::Sprite)
        );
        assert!(matches!(
            director.stage(),
            Stage::Feature {
                routine: Feature::Sprite,
                phase: Phase::Gather,
                ..
            }
        ));
    }

    #[test]
    fn abandoning_drops_straight_back_to_ambient() {
        let mut director = Director::new(test_rng());
        director.interrupt(Feature::Lattice);
        assert!(matches!(director.abandon(), Transition::Ambient(_)));
        assert!(matches!(director.stage(), Stage::Ambient { .. }));
    }
}
