use crate::animate::frames::FrameAnimationType;
use crate::particles::color::ParticleColor;
use crate::particles::geometry::Vec2D;
use crate::particles::meta::ParticleSprite;

/// A particle wave modelled as a sin function magnitude * sin(frequency * lifetime)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleWave {
    magnitude: f64,
    frequency: f64,
}

impl ParticleWave {
    pub fn new(magnitude: f64, frequency: f64) -> Self {
        Self {
            magnitude,
            frequency,
        }
    }

    pub fn next(&self, lifetime: f64) -> f64 {
        self.magnitude * (self.frequency * lifetime).sin()
    }
    pub fn magnitude(&self) -> f64 {
        self.magnitude
    }
    pub fn frequency(&self) -> f64 {
        self.frequency
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub enum ParticleAnimationType {
    #[default]
    Static,
    Linear {
        fps: u32,
        frames: usize,
    },
    YoYo {
        fps: u32,
        frames: usize,
    },
}

impl ParticleAnimationType {
    /// how a sprite strip plays as a particle; a paused strip just loops
    pub fn from_frames(animation_type: FrameAnimationType, frames: usize) -> Self {
        match animation_type {
            FrameAnimationType::Static => Self::Static,
            FrameAnimationType::Linear { fps } | FrameAnimationType::LinearWithPause { fps, .. } => {
                Self::Linear { fps, frames }
            }
            FrameAnimationType::YoYo { fps } => Self::YoYo { fps, frames },
        }
    }
}

impl ParticleAnimationType {
    fn frame_duration(&self) -> f64 {
        match self {
            Self::Linear { fps, .. } => 1.0 / *fps as f64,
            Self::YoYo { fps, .. } => 1.0 / *fps as f64,
            _ => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleAnimation {
    frame: usize,
    iteration: u32,
    frame_duration: f64,
    animation_type: ParticleAnimationType,
}

impl ParticleAnimation {
    pub fn new(animation_type: ParticleAnimationType) -> Self {
        Self {
            frame: 0,
            iteration: 0,
            frame_duration: animation_type.frame_duration(),
            animation_type,
        }
    }
}

/// A force acting on every particle of a group or pool. [`Field::Orbit`] is the gravity well
/// the background has always used; the rest are what the scene routines steer with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Field {
    /// Newtonian attraction toward a point
    Orbit(Vec2D),
    /// spiral: tangential about `centre` with a mild inward pull, so arms form and shear
    Vortex { centre: Vec2D, strength: f64 },
    /// sum-of-sines advection; `phase` walks with time to keep it from repeating
    Flow {
        scale: f64,
        strength: f64,
        phase: f64,
    },
    /// pushed away from a point
    Repel { centre: Vec2D, strength: f64 },
}

impl Field {
    /// the empirically ideal `G * m` of the original orbit source
    const GRAVITY: f64 = 0.001;

    /// nudge one particle for one step
    pub fn apply(&self, particle: &mut Particle, delta_time: f64) {
        match self {
            Field::Orbit(orbit) => {
                let delta = particle.position - *orbit;
                let magnitude_squared = delta.magnitude_squared();
                // only apply gravitation when particle is sufficiently distant as this
                // approximation breaks down for small distances
                if magnitude_squared > 0.001 {
                    // Vector form of Newtons law of gravitation
                    let f = delta.unit_vector() * (-Self::GRAVITY / magnitude_squared);
                    particle.velocity += f * delta_time;
                }
            }
            Field::Repel { centre, strength } => {
                let delta = particle.position - *centre;
                let magnitude_squared = delta.magnitude_squared();
                if magnitude_squared > 0.001 {
                    let f = delta.unit_vector() * (strength / magnitude_squared);
                    particle.velocity += f * delta_time;
                }
            }
            Field::Vortex { centre, strength } => {
                let delta = particle.position - *centre;
                let magnitude = delta.magnitude();
                if magnitude > 0.02 {
                    let unit = delta * (1.0 / magnitude);
                    // angular velocity falls off with distance, so the middle winds up first
                    let tangent = unit.perpendicular() * (strength / magnitude);
                    // and a light pull inward keeps the arms from unwinding forever
                    let inward = unit * (-strength * 0.25);
                    particle.velocity += (tangent + inward) * delta_time;
                }
            }
            Field::Flow {
                scale,
                strength,
                phase,
            } => {
                let (x, y) = (particle.position.x() * scale, particle.position.y() * scale);
                // the curl of a sum of sines: divergence free, so the field circulates
                // rather than piling particles up in one corner
                let vx = (y + phase).sin() + 0.5 * (2.0 * y - phase * 0.7).cos();
                let vy = (x - phase * 0.9).cos() + 0.5 * (2.0 * x + phase * 0.5).sin();
                particle.velocity += Vec2D::new(vx, vy) * (*strength * delta_time);
            }
        }
    }
}

/// Somewhere a particle is being pulled to, as a critically damped spring. This is the
/// primitive every formation routine is built on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleTarget {
    pub position: Vec2D,
    pub stiffness: f64,
    pub damping: f64,
}

impl ParticleTarget {
    pub fn new(position: Vec2D, stiffness: f64) -> Self {
        // critical damping for a unit mass: nothing overshoots its place in a silhouette
        Self {
            position,
            stiffness,
            damping: 2.0 * stiffness.sqrt(),
        }
    }

    pub fn with_damping(self, damping: f64) -> Self {
        Self { damping, ..self }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    position: Vec2D,
    velocity: Vec2D,
    acceleration: Vec2D,
    max_alpha: f64,
    alpha: f64,
    pulse: Option<ParticleWave>,
    color: ParticleColor,
    time_to_live: Option<f64>,
    sprite: ParticleSprite,
    size: f64,
    rotation: f64,
    angular_velocity: f64,
    animation: Option<ParticleAnimation>,
    target: Option<ParticleTarget>,
}

impl Particle {
    pub fn new(
        position: Vec2D,
        velocity: Vec2D,
        acceleration: Vec2D,
        max_alpha: f64,
        alpha: f64,
        pulse: Option<ParticleWave>,
        color: ParticleColor,
        time_to_live: Option<f64>,
        sprite: ParticleSprite,
        size: f64,
        angular_velocity: f64,
    ) -> Self {
        let animation = sprite.animation().map(|pa| ParticleAnimation::new(pa));
        Self {
            position,
            velocity,
            acceleration,
            max_alpha,
            alpha,
            pulse,
            color,
            time_to_live,
            sprite,
            size,
            rotation: 0.0,
            angular_velocity,
            animation,
            target: None,
        }
    }

    /// checks if the particle is out of bounds (0-1) and trajectory will not bring it back
    pub fn is_escaped(&self) -> bool {
        const THRESHOLD_MAX: f64 = 1.05;
        const THRESHOLD_MIN: f64 = -0.05;
        (self.position.x() > THRESHOLD_MAX
            && self.velocity.x() >= 0.0
            && self.acceleration.x() >= 0.0)
            || (self.position.x() < THRESHOLD_MIN
                && self.velocity.x() <= 0.0
                && self.acceleration.x() <= 0.0)
            || (self.position.y() > THRESHOLD_MAX
                && self.velocity.y() >= 0.0
                && self.acceleration.y() >= 0.0)
            || (self.position.y() < THRESHOLD_MIN
                && self.velocity.y() <= 0.0
                && self.acceleration.y() <= 0.0)
    }

    pub fn update(&mut self, delta_time: f64, lifetime: f64) {
        if let Some(target) = self.target {
            let displacement = target.position - self.position;
            let spring = displacement * target.stiffness - self.velocity * target.damping;
            self.velocity += spring * delta_time;
        }
        self.velocity += self.acceleration * delta_time;
        self.position += self.velocity * delta_time;
        self.rotation += self.angular_velocity * delta_time;
        if let Some(animation) = self.animation.as_mut() {
            match animation.animation_type {
                ParticleAnimationType::Static => {}
                ParticleAnimationType::Linear { frames, .. }
                | ParticleAnimationType::YoYo { frames, .. } => {
                    animation.frame =
                        (lifetime / animation.frame_duration).floor() as usize % frames;
                    animation.iteration =
                        (lifetime / (animation.frame_duration * frames as f64)).floor() as u32;
                }
            }
        }
    }

    pub fn position(&self) -> Vec2D {
        self.position
    }
    pub fn velocity(&self) -> Vec2D {
        self.velocity
    }
    pub fn target(&self) -> Option<ParticleTarget> {
        self.target
    }
    pub fn set_position(&mut self, position: Vec2D) {
        self.position = position;
    }
    pub fn set_velocity(&mut self, velocity: Vec2D) {
        self.velocity = velocity;
    }
    pub fn add_velocity(&mut self, velocity: Vec2D) {
        self.velocity += velocity;
    }
    pub fn set_acceleration(&mut self, acceleration: Vec2D) {
        self.acceleration = acceleration;
    }
    pub fn set_target(&mut self, target: Option<ParticleTarget>) {
        self.target = target;
    }
    pub fn set_color(&mut self, color: ParticleColor) {
        self.color = color;
    }
    pub fn set_alpha(&mut self, alpha: f64) {
        self.alpha = alpha.clamp(0.0, 1.0);
        self.max_alpha = self.alpha;
    }
    pub fn set_size(&mut self, size: f64) {
        self.size = size;
    }
    pub fn set_sprite(&mut self, sprite: ParticleSprite) {
        self.sprite = sprite;
        self.animation = sprite.animation().map(ParticleAnimation::new);
    }
    pub fn set_rotation(&mut self, rotation: f64) {
        self.rotation = rotation;
    }
    pub fn set_angular_velocity(&mut self, angular_velocity: f64) {
        self.angular_velocity = angular_velocity;
    }
    /// step the particle with no group around it: pools own their own lifetimes
    pub fn step(&mut self, delta_time: f64, field: Option<&Field>) {
        if let Some(field) = field {
            field.apply(self, delta_time);
        }
        self.update(delta_time, 0.0);
    }
    pub fn alpha(&self) -> f64 {
        self.alpha
    }
    pub fn color(&self) -> ParticleColor {
        self.color
    }
    pub fn sprite(&self) -> ParticleSprite {
        self.sprite
    }
    pub fn size(&self) -> f64 {
        self.size
    }
    pub fn rotation(&self) -> f64 {
        self.rotation
    }
    pub fn animation_frame(&self) -> usize {
        let animation = self.animation.as_ref().expect("not an animating particle");
        match animation.animation_type {
            ParticleAnimationType::YoYo { frames, .. } if animation.iteration % 2 == 1 => {
                frames - animation.frame - 1
            }
            _ => animation.frame,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParticleGroup {
    lifetime: f64,
    anchor_for: Option<f64>,
    fade_in: Option<f64>,
    fade_out: bool,
    field: Option<Field>,
    particles: Vec<Particle>,
}

impl ParticleGroup {
    pub fn new(
        anchor_for: Option<f64>,
        fade_in: Option<f64>,
        fade_out: bool,
        field: Option<Field>,
        particles: Vec<Particle>,
    ) -> Self {
        Self {
            lifetime: 0.0,
            anchor_for,
            fade_in,
            fade_out,
            field,
            particles,
        }
    }

    pub fn update_life(&mut self, delta_time: f64) {
        self.lifetime += delta_time;

        // remove dead particles
        let mut to_remove = vec![];
        for (index, particle) in self.particles.iter().enumerate() {
            if particle.is_escaped() {
                to_remove.push(index);
            } else if let Some(time_to_live) = particle.time_to_live {
                if self.lifetime >= time_to_live {
                    to_remove.push(index);
                }
            }
        }
        for index in to_remove.into_iter().rev() {
            self.particles.remove(index);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    pub fn len(&self) -> usize {
        self.particles.len()
    }

    pub fn update_particles(&mut self, delta_time: f64) {
        // spatial
        if let Some(anchor_for) = self.anchor_for {
            self.anchor_for = if delta_time >= anchor_for {
                None
            } else {
                Some(anchor_for - delta_time)
            }
        } else {
            if let Some(field) = self.field {
                for particle in self.particles.iter_mut() {
                    field.apply(particle, delta_time);
                }
            }

            for particle in self.particles.iter_mut() {
                particle.update(delta_time, self.lifetime);
            }
        }

        // alpha
        if let Some(fade_in) = self.fade_in {
            if self.lifetime >= fade_in {
                self.fade_in = None;
            } else {
                for particle in self.particles.iter_mut() {
                    particle.alpha = particle.max_alpha * self.lifetime.min(fade_in) / fade_in;
                }
            }
        }
        // fade out
        else if self.fade_out {
            for particle in self.particles.iter_mut() {
                if let Some(ttl) = particle.time_to_live {
                    particle.alpha = particle.max_alpha * (1.0 - self.lifetime.min(ttl) / ttl);
                }
            }
        }

        for particle in self.particles.iter_mut() {
            // pulse
            if let Some(pulse) = particle.pulse.as_ref() {
                let pulse_magnitude = pulse.next(self.lifetime);
                particle.alpha = (particle.alpha + pulse_magnitude)
                    .min(particle.max_alpha)
                    .max(0.0);
            }
        }
    }

    pub fn particles(&self) -> &[Particle] {
        self.particles.as_slice()
    }
}
