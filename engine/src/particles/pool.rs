//! A retained pool of particles kept for the life of a match and steered every frame, where
//! the fire-and-forget [`super::source`] model cannot retarget a group once it is emitted.

use crate::particles::color::ParticleColor;
use crate::particles::field::context::SceneContext;
use crate::particles::field::director::Feature;
use crate::particles::geometry::{RectF, Vec2D};
use crate::particles::particle::Particle;
use std::time::Duration;

/// A soft line drawn between two particles.
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

    /// stage a named feature routine, for the diagnostic renderer
    fn force_feature(&mut self, _feature: Feature) {}
}
