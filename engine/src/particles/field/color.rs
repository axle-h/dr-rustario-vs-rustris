//! The field's colour, which is a layer of its own: it is independent of the motion layer, so
//! routines multiply with palettes rather than each carrying their own look.

use crate::particles::color::ParticleColor;
use crate::particles::field::context::SceneContext;
use crate::particles::field::{field_rng, FieldRng};
use crate::particles::geometry::Vec2D;
use rand::RngExt;

/// how far the wandering hue may stray from the palette it was seeded from
const HUE_RANGE: f64 = 26.0;
/// degrees per second the walk may accelerate by
const HUE_ACCELERATION: f64 = 9.0;
/// how fast colour flows around the palette ring
const PHASE_RATE: f64 = 0.035;
/// what one line of a clear adds to the field's energy
const CLEAR_ENERGY: f64 = 0.25;
/// energy lost per second
const ENERGY_DECAY: f64 = 0.55;
/// the colour a stack near the top drains the field toward
const DANGER: ParticleColor = ParticleColor::rgb(0.85, 0.05, 0.05);

pub struct ColorDriver {
    /// a slow constrained random walk around the palette's hue: always related to the theme,
    /// never static
    hue: f64,
    hue_velocity: f64,
    /// a clear nudges the hue hard and it eases back
    push: f64,
    /// 0-1, raised by clears and speed ups and decaying on its own. Brightens the field and
    /// makes the director reach for a feature sooner.
    energy: f64,
    /// where the palette ring is being read from, so colour drifts through the field
    phase: f64,
    danger: f64,
    rng: FieldRng,
}

impl ColorDriver {
    pub fn new(rng: FieldRng) -> Self {
        Self {
            hue: 0.0,
            hue_velocity: 0.0,
            push: 0.0,
            energy: 0.0,
            phase: 0.0,
            danger: 0.0,
            rng,
        }
    }

    pub fn update(&mut self, delta_time: f64, ctx: &SceneContext) {
        // a random walk that is pulled back toward the palette it started from, so it
        // wanders without ever leaving the theme behind
        self.hue_velocity += (self.rng.random::<f64>() - 0.5) * HUE_ACCELERATION * delta_time;
        self.hue_velocity -= self.hue * 0.25 * delta_time;
        self.hue_velocity *= 1.0 - 0.5 * delta_time;
        self.hue = (self.hue + self.hue_velocity * delta_time * 10.0).clamp(-HUE_RANGE, HUE_RANGE);

        self.push -= self.push * 2.0 * delta_time;
        self.energy = (self.energy - ENERGY_DECAY * delta_time).max(0.0);
        self.phase += PHASE_RATE * (1.0 + self.energy) * delta_time;

        // danger has no event: it is read from the boards every frame, and eased so a piece
        // clearing the top row does not snap the whole field back
        let danger = ctx.danger();
        let rate = if danger > self.danger { 1.5 } else { 0.6 };
        self.danger += (danger - self.danger) * (rate * delta_time).min(1.0);
    }

    /// a clear pushes the palette; a big one pushes it hard
    pub fn on_clear(&mut self, class: u16, count: u32, is_combo: bool) {
        let magnitude = (class as f64 + 1.0) * (count.max(1) as f64).sqrt();
        self.push += 14.0 * magnitude * if is_combo { 1.6 } else { 1.0 };
        self.push = self.push.clamp(-140.0, 140.0);
        self.energy = (self.energy + CLEAR_ENERGY * magnitude).min(1.0);
    }

    pub fn on_speed_up(&mut self) {
        self.push += 45.0;
        self.energy = (self.energy + 0.4).min(1.0);
    }

    pub fn add_energy(&mut self, amount: f64) {
        self.energy = (self.energy + amount).clamp(0.0, 1.0);
    }

    pub fn energy(&self) -> f64 {
        self.energy
    }

    pub fn danger(&self) -> f64 {
        self.danger
    }

    pub fn phase(&self) -> f64 {
        self.phase
    }

    /// the colour of one particle: its place in the palette, where it is on the canvas, and
    /// everything the drivers above have done to the field since
    pub fn color(&self, ctx: &SceneContext, position: Vec2D, seed: f64) -> ParticleColor {
        let color = ctx.radiated(position, seed + self.phase);
        let color = color.shift_hue(self.hue + self.push);
        if self.danger <= 0.01 {
            return color;
        }
        // saturation bleeds toward red as a stack nears the top
        color.lerp_hue(DANGER, self.danger * 0.7)
    }

    /// how bright the field burns, which is where energy is actually seen
    pub fn alpha(&self, base: f64) -> f64 {
        (base * (1.0 + 0.35 * self.energy)).min(1.0)
    }
}

impl Default for ColorDriver {
    fn default() -> Self {
        Self::new(field_rng())
    }
}
