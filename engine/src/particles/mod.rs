use crate::particles::field::context::SceneContext;
use crate::particles::pool::ParticlePool;
use crate::particles::source::ParticleSource;
use particle::{Particle, ParticleGroup};

use std::time::Duration;

pub mod color;
pub mod field;
pub mod geometry;
pub mod meta;
pub mod particle;
pub mod pool;
pub mod prescribed;
pub mod quantity;
pub mod render;
pub mod scale;
pub mod source;

pub struct Particles {
    particles: Vec<ParticleGroup>,
    sources: Vec<Box<dyn ParticleSource>>,
    /// retained pools, which own their particles for as long as they live and update them in
    /// place. The emitter machinery above is untouched, so the menus keep working unchanged.
    pools: Vec<Box<dyn ParticlePool>>,
    max_particles: usize,
}

impl Particles {
    pub fn new(max_particles: usize) -> Self {
        Self {
            sources: vec![],
            particles: vec![],
            pools: vec![],
            max_particles,
        }
    }

    /// the particles of the emitted groups; see [`Particles::pools`] for the rest
    pub fn group_particles(&self) -> Vec<&Particle> {
        self.particles.iter().flat_map(|g| g.particles()).collect()
    }

    pub fn pools(&self) -> &[Box<dyn ParticlePool>] {
        &self.pools
    }

    pub fn pools_mut(&mut self) -> &mut [Box<dyn ParticlePool>] {
        &mut self.pools
    }

    pub fn add_pool(&mut self, pool: Box<dyn ParticlePool>) {
        self.pools.push(pool);
    }

    /// `scene` is what the pools update against; `None` ticks the emitters only, which is all
    /// a menu ever has
    pub fn update(&mut self, delta: Duration, scene: Option<&SceneContext>) {
        let delta_time = delta.as_secs_f64();
        self.update_life(delta_time);
        self.update_particles(delta_time);
        self.emit_particles(delta);
        if let Some(scene) = scene {
            for pool in self.pools.iter_mut() {
                pool.update(delta, scene);
            }
        }
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.sources.clear();
        self.pools.clear();
    }

    fn update_life(&mut self, delta_time: f64) {
        let mut to_remove = vec![];
        for (i, group) in self.particles.iter_mut().enumerate() {
            group.update_life(delta_time);
            if group.is_empty() {
                to_remove.push(i);
            }
        }
        for i in to_remove.into_iter().rev() {
            self.particles.remove(i);
        }
    }

    fn update_particles(&mut self, delta_time: f64) {
        for group in self.particles.iter_mut() {
            group.update_particles(delta_time);
        }
    }

    fn emit_particles(&mut self, delta: Duration) {
        let current_particles = self.particles.iter().map(|g| g.len()).sum::<usize>() as i32;
        let mut max_particles = self.max_particles as i32 - current_particles;

        let mut to_remove = vec![];
        for (index, source) in self.sources.iter_mut().enumerate() {
            if max_particles <= 0 {
                return;
            }

            for group in source.update(delta, max_particles as u32) {
                max_particles -= group.len() as i32;
                self.particles.push(group);
            }

            if source.is_complete() {
                to_remove.push(index);
            }
        }
        for index in to_remove.into_iter().rev() {
            self.sources.remove(index);
        }
    }
}

#[cfg(test)]
mod tests {}
