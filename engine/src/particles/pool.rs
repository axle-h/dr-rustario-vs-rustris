//! A retained pool of particles that owns and updates itself in place, alongside the
//! fire-and-forget emitter model in [`super::source`].
//!
//! A [`super::source::ParticleSource`] emits a group and then has no further say: the group
//! evolves on its own until every particle's time to live expires. That is right for a burst
//! and wrong for a permanent field, which needs to retarget the particles it already has. A
//! pool keeps them for the life of the match and steers them every frame.

use crate::particles::color::ParticleColor;
use crate::particles::field::context::SceneContext;
use crate::particles::field::director::Feature;
use crate::particles::geometry::{RectF, Vec2D};
use crate::particles::particle::Particle;
use std::time::Duration;

/// A soft line drawn between two particles, see the constellation routine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleLink {
    pub from: Vec2D,
    pub to: Vec2D,
    pub color: ParticleColor,
    pub alpha: f64,
    /// thickness as a fraction of the window height
    pub width: f64,
}

pub trait ParticlePool {
    fn update(&mut self, delta: Duration, ctx: &SceneContext);

    fn particles(&self) -> &[Particle];

    /// where the pool may draw, in particle space; `None` is the whole window
    fn clip(&self) -> Option<RectF> {
        None
    }

    fn links(&self) -> &[ParticleLink] {
        &[]
    }

    /// stage a named feature routine, for the diagnostic renderer. A pool with no features
    /// has nothing to stage.
    fn force_feature(&mut self, _feature: Feature) {}
}
