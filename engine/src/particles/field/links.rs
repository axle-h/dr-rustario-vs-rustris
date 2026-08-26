//! Constellation links: soft lines between particles that are near each other.
//!
//! The neighbour search is a uniform grid bucketed at the link radius; each particle scans its
//! own bucket and the eight around it. That is O(n) and needs no tree.

use crate::particles::color::ParticleColor;
use crate::particles::geometry::RectF;
use crate::particles::particle::Particle;
use crate::particles::pool::ParticleLink;

/// the most links one particle may own, so a dense clump does not eat the whole budget
const MAX_PER_PARTICLE: usize = 3;
/// the coarsest the grid is allowed to get, in buckets per side
const MAX_BUCKETS: usize = 64;

#[derive(Default)]
pub struct LinkBuilder {
    /// bucket index -> particle indices
    buckets: Vec<Vec<usize>>,
    cols: usize,
    rows: usize,
    links: Vec<ParticleLink>,
}

impl LinkBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn links(&self) -> &[ParticleLink] {
        &self.links
    }

    pub fn clear(&mut self) {
        self.links.clear();
    }

    /// `radius` and `width` are in particle space; `budget` caps the whole frame
    pub fn build(
        &mut self,
        particles: &[Particle],
        canvas: RectF,
        radius: f64,
        width: f64,
        budget: usize,
        alpha: f64,
    ) {
        self.links.clear();
        if budget == 0 || radius <= 0.0 || particles.is_empty() {
            return;
        }

        self.cols = ((canvas.width() / radius).ceil() as usize).clamp(1, MAX_BUCKETS);
        self.rows = ((canvas.height() / radius).ceil() as usize).clamp(1, MAX_BUCKETS);
        self.buckets.clear();
        self.buckets.resize(self.cols * self.rows, vec![]);
        for bucket in self.buckets.iter_mut() {
            bucket.clear();
        }

        for (index, particle) in particles.iter().enumerate() {
            let (col, row) = self.bucket_of(canvas, particle);
            self.buckets[row * self.cols + col].push(index);
        }

        let radius_squared = radius * radius;
        for (index, particle) in particles.iter().enumerate() {
            if self.links.len() >= budget {
                break;
            }
            let (col, row) = self.bucket_of(canvas, particle);
            let mut owned = 0;
            'neighbours: for j in row.saturating_sub(1)..(row + 2).min(self.rows) {
                for i in col.saturating_sub(1)..(col + 2).min(self.cols) {
                    for other in self.buckets[j * self.cols + i].iter().copied() {
                        // each pair is linked once, by its lower index
                        if other <= index {
                            continue;
                        }
                        let from = particle.position();
                        let to = particles[other].position();
                        let distance_squared = (to - from).magnitude_squared();
                        if distance_squared > radius_squared {
                            continue;
                        }
                        // near neighbours are solid and distant ones fainter, but never so
                        // faint that a link appears out of nothing as two particles close
                        let closeness =
                            0.4 + 0.6 * (1.0 - (distance_squared / radius_squared).sqrt());
                        self.links.push(ParticleLink {
                            from,
                            to,
                            color: particle
                                .color()
                                .lerp(particles[other].color(), 0.5),
                            alpha: alpha * closeness * particle.alpha(),
                            width,
                        });
                        owned += 1;
                        if owned >= MAX_PER_PARTICLE || self.links.len() >= budget {
                            break 'neighbours;
                        }
                    }
                }
            }
        }
    }

    fn bucket_of(&self, canvas: RectF, particle: &Particle) -> (usize, usize) {
        let normalised = canvas.normalise(particle.position());
        let col = ((normalised.x() * self.cols as f64) as isize).clamp(0, self.cols as isize - 1);
        let row = ((normalised.y() * self.rows as f64) as isize).clamp(0, self.rows as isize - 1);
        (col as usize, row as usize)
    }
}

/// the colour a link takes when neither end has one yet
#[allow(dead_code)]
pub const DEFAULT_LINK_COLOR: ParticleColor = ParticleColor::WHITE;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particles::color::ParticleColor;
    use crate::particles::geometry::Vec2D;
    use crate::particles::meta::ParticleSprite;

    fn particle(x: f64, y: f64) -> Particle {
        Particle::new(
            Vec2D::new(x, y),
            Vec2D::ZERO,
            Vec2D::ZERO,
            1.0,
            1.0,
            None,
            ParticleColor::WHITE,
            None,
            ParticleSprite::Circle05,
            1.0,
            0.0,
        )
    }

    fn canvas() -> RectF {
        RectF::new(0.0, 0.0, 1.0, 1.0)
    }

    #[test]
    fn near_neighbours_are_linked_once_each() {
        let particles = [particle(0.5, 0.5), particle(0.52, 0.5)];
        let mut builder = LinkBuilder::new();
        builder.build(&particles, canvas(), 0.05, 0.002, 100, 1.0);
        assert_eq!(builder.links().len(), 1);
    }

    #[test]
    fn distant_particles_are_not_linked() {
        let particles = [particle(0.1, 0.1), particle(0.9, 0.9)];
        let mut builder = LinkBuilder::new();
        builder.build(&particles, canvas(), 0.05, 0.002, 100, 1.0);
        assert!(builder.links().is_empty());
    }

    #[test]
    fn a_clump_never_exceeds_the_budget() {
        let particles = (0..200)
            .map(|i| particle(0.5 + (i % 10) as f64 * 0.001, 0.5 + (i / 10) as f64 * 0.001))
            .collect::<Vec<Particle>>();
        let mut builder = LinkBuilder::new();
        builder.build(&particles, canvas(), 0.2, 0.002, 17, 1.0);
        assert!(builder.links().len() <= 17, "{}", builder.links().len());
    }

    #[test]
    fn a_particle_outside_the_canvas_still_buckets() {
        let particles = [particle(-5.0, -5.0), particle(9.0, 9.0)];
        let mut builder = LinkBuilder::new();
        builder.build(&particles, canvas(), 0.05, 0.002, 100, 1.0);
        assert!(builder.links().is_empty());
    }
}
