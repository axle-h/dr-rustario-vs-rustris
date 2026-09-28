//! The particle themes' background: a retained pool that owns its particles for the whole
//! match, reacts to a [`SceneContext`] and is driven by [`director`]. It never touches game
//! state or shares an RNG with the games; a particle leaving the canvas is re-seeded opposite.

pub mod color;
pub mod context;
pub mod director;
pub mod formation;
pub mod links;
pub mod reaction;
pub mod shapes;

use crate::config::ParticleDensity;
use crate::particles::color::ParticleColor;
use crate::particles::field::color::ColorDriver;
use crate::particles::field::context::SceneContext;
use crate::particles::field::director::{Ambient, Director, Feature, Phase, Stage, Transition};
use crate::particles::field::formation::Formation;
use crate::particles::field::links::LinkBuilder;
use crate::particles::field::reaction::{
    attack_endpoints, words, AttackEnd, Comet, FieldEvent, Pall, Shockwave,
};
use crate::particles::field::shapes::ShapeBank;
use crate::particles::geometry::{RectF, Vec2D};
use crate::particles::meta::ParticleSprite;
use crate::particles::particle::{Field, Particle, ParticleTarget};
use crate::particles::pool::{ParticleLink, ParticlePool};
use rand::{rng, RngExt, SeedableRng};
use rand_chacha::ChaChaRng;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// Seedable so tests over a run of frames see the same frames every run; a match seeds it from
/// entropy.
pub type FieldRng = ChaChaRng;

pub fn field_rng() -> FieldRng {
    FieldRng::from_rng(&mut rng())
}

/// the seed every test here runs on, so a failure is a change and not a draw
#[cfg(test)]
pub(crate) const TEST_SEED: u64 = 20_260_829;

#[cfg(test)]
pub(crate) fn test_rng() -> FieldRng {
    FieldRng::seed_from_u64(TEST_SEED)
}

/// the share of the pool that stays ambient while a feature runs
const AMBIENT_SHARE: f64 = 0.3;
/// the longest step integrated at once, so a stall cannot fling the field apart
const MAX_STEP: f64 = 1.0 / 20.0;
/// how far outside the canvas a particle drifts before it is re-seeded
const WRAP_MARGIN: f64 = 0.06;
/// slower particles are nudged, so an ambient routine never stops
const MIN_DRIFT: f64 = 0.012;
const MAX_DRIFT: f64 = 0.35;
const RE_ENTRY_DRIFT: (f64, f64) = (0.06, 0.16);
const GATHER_STIFFNESS: f64 = 90.0;
/// how far beyond a playfield's edge the board's influence reaches, in particle space
const BOARD_MARGIN: f64 = 0.06;
/// alpha multiplier for a particle right behind a playfield
const BOARD_ALPHA: f64 = 0.35;
/// how hard a board pushes the ambient field away, per second
const BOARD_PUSH: f64 = 0.08;
/// the least a formation member is dimmed to over a board, so a silhouette holds together
const FORMATION_BOARD_ALPHA: f64 = 0.45;
/// the ambient share's alpha while a formation holds
const AMBIENT_UNDER_FORMATION: f64 = 0.45;
/// link radius, as a fraction of canvas height
const LINK_RADIUS: f64 = 0.075;
/// link thickness, as a fraction of window height
const LINK_WIDTH: f64 = 0.0045;

/// The outlines the renderer builds for the field and the events the match screen queues into it.
#[derive(Default)]
pub struct FieldBus {
    pub shapes: ShapeBank,
    pub events: Vec<FieldEvent>,
}

pub type SharedBus = Rc<RefCell<FieldBus>>;

#[derive(Clone, Copy, Debug)]
struct Member {
    /// stable 0-1 palette position, so colour flows rather than flickers
    seed: f64,
    base_size: f64,
    base_alpha: f64,
    /// in a comet, so not the ambient field's to steer
    in_comet: bool,
}

/// How much a board owns a point, 1.0 inside tapering to 0 at [`BOARD_MARGIN`] beyond it, with
/// the shortest way out; `None` clear of it.
fn board_influence(board: &RectF, point: Vec2D) -> Option<(f64, Vec2D)> {
    let nearest = Vec2D::new(
        point.x().clamp(board.x(), board.right()),
        point.y().clamp(board.y(), board.bottom()),
    );
    let out = point - nearest;
    let distance = out.magnitude();
    if distance >= BOARD_MARGIN {
        return None;
    }
    let away = if distance > 0.0 {
        out.unit_vector()
    } else {
        // inside: out by the closest edge
        let gaps = [
            (point.x() - board.x(), Vec2D::new(-1.0, 0.0)),
            (board.right() - point.x(), Vec2D::new(1.0, 0.0)),
            (point.y() - board.y(), Vec2D::new(0.0, -1.0)),
            (board.bottom() - point.y(), Vec2D::new(0.0, 1.0)),
        ];
        gaps.into_iter()
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, direction)| direction)
            .unwrap()
    };
    Some((1.0 - distance / BOARD_MARGIN, away))
}

pub struct ParticleField {
    canvas: RectF,
    /// particle space is normalised to the window, so without this every circle is an ellipse
    window_aspect: f64,
    density: ParticleDensity,
    particles: Vec<Particle>,
    members: Vec<Member>,
    director: Director,
    ambient: Ambient,
    colors: ColorDriver,
    formation: Formation,
    /// the current formation's particles, in target order
    cast: Vec<usize>,
    link_builder: LinkBuilder,
    bus: SharedBus,
    waves: Vec<Shockwave>,
    comets: Vec<Comet>,
    palls: Vec<Pall>,
    /// the wandering centre a vortex or flow reads from
    focus: Vec2D,
    focus_velocity: Vec2D,
    /// a feature staged by [`ParticleField::force`]
    forced: Option<Feature>,
    /// a word the match called for, spelt by the next text formation
    word: Option<&'static str>,
    time: f64,
    rng: FieldRng,
}

impl ParticleField {
    pub fn new(
        canvas: RectF,
        window_size: (u32, u32),
        density: ParticleDensity,
        bus: SharedBus,
    ) -> Self {
        Self::with_rng(canvas, window_size, density, bus, field_rng())
    }

    /// Everything the field draws comes off `seed`, so it plays out the same every time.
    pub fn seeded(
        canvas: RectF,
        window_size: (u32, u32),
        density: ParticleDensity,
        bus: SharedBus,
        seed: u64,
    ) -> Self {
        Self::with_rng(
            canvas,
            window_size,
            density,
            bus,
            FieldRng::seed_from_u64(seed),
        )
    }

    fn with_rng(
        canvas: RectF,
        window_size: (u32, u32),
        density: ParticleDensity,
        bus: SharedBus,
        mut rng: FieldRng,
    ) -> Self {
        let window_aspect = window_size.0 as f64 / window_size.1.max(1) as f64;
        // separate streams, so one drawing more numbers cannot shift the others
        let director = Director::new(FieldRng::from_rng(&mut rng));
        let colors = ColorDriver::new(FieldRng::from_rng(&mut rng));
        let mut field = Self {
            canvas,
            window_aspect,
            density,
            particles: vec![],
            members: vec![],
            director,
            ambient: Ambient::Orbit,
            colors,
            formation: Formation::Free,
            cast: vec![],
            link_builder: LinkBuilder::new(),
            bus,
            waves: vec![],
            comets: vec![],
            palls: vec![],
            focus: canvas.center(),
            focus_velocity: Vec2D::new(0.03, 0.02),
            forced: None,
            word: None,
            time: 0.0,
            rng,
        };
        field.resize();
        field
    }

    /// the canvas's width over height in real pixels, which formations are authored against
    fn canvas_aspect(&self) -> f64 {
        (self.canvas.width() * self.window_aspect / self.canvas.height()).max(0.01)
    }

    /// scaled by the canvas's share of the window, so density stays constant
    fn wanted_len(&self) -> usize {
        let area = (self.canvas.width() * self.canvas.height()).clamp(0.0, 1.0);
        ((self.density.pool_size() as f64) * area).round() as usize
    }

    fn resize(&mut self) {
        let wanted = self.wanted_len();
        while self.particles.len() > wanted {
            self.particles.pop();
            self.members.pop();
        }
        while self.particles.len() < wanted {
            let (particle, member) = self.seed_particle();
            self.particles.push(particle);
            self.members.push(member);
        }
    }

    fn seed_particle(&mut self) -> (Particle, Member) {
        let position = Vec2D::new(
            self.canvas.x() + self.canvas.width() * self.rng.random::<f64>(),
            self.canvas.y() + self.canvas.height() * self.rng.random::<f64>(),
        );
        self.build_particle(position)
    }

    fn build_particle(&mut self, position: Vec2D) -> (Particle, Member) {
        let roll = self.rng.random::<f64>();
        let (sprite, size) = if roll < 0.8 {
            (
                ParticleSprite::Circle05,
                0.7 + 0.6 * self.rng.random::<f64>(),
            )
        } else if roll < 0.9 {
            let index = self
                .rng
                .random_range(0..ParticleSprite::HOLLOW_CIRCLES.len());
            (
                ParticleSprite::HOLLOW_CIRCLES[index],
                1.1 + 0.8 * self.rng.random::<f64>(),
            )
        } else {
            let index = self.rng.random_range(0..ParticleSprite::STARS.len());
            (
                ParticleSprite::STARS[index],
                1.2 + 0.8 * self.rng.random::<f64>(),
            )
        };
        let alpha = 0.55 + 0.35 * self.rng.random::<f64>();
        let velocity = Vec2D::new(
            (self.rng.random::<f64>() - 0.5) * 0.06,
            (self.rng.random::<f64>() - 0.5) * 0.06,
        );
        let particle = Particle::new(
            position,
            velocity,
            Vec2D::ZERO,
            alpha,
            alpha,
            None,
            ParticleColor::WHITE,
            None,
            sprite,
            size,
            0.0,
        );
        let member = Member {
            seed: self.rng.random::<f64>(),
            base_size: size,
            base_alpha: alpha,
            in_comet: false,
        };
        (particle, member)
    }

    /// follow a moved canvas and abandon any half-finished routine
    fn set_canvas(&mut self, canvas: RectF) {
        let previous = self.canvas;
        self.canvas = canvas;
        for particle in self.particles.iter_mut() {
            particle.set_position(previous.remap(particle.position(), &canvas));
            particle.set_target(None);
        }
        self.focus = previous.remap(self.focus, &canvas);
        self.cast.clear();
        self.formation = Formation::Free;
        self.waves.clear();
        self.palls.clear();
        for comet in self.comets.drain(..) {
            for index in comet.members() {
                if let Some(member) = self.members.get_mut(*index) {
                    member.in_comet = false;
                }
            }
        }
        let transition = self.director.abandon();
        self.apply_transition(transition, None);
        self.resize();
    }

    /// Stage `feature` on the next update, whatever the director had in mind.
    pub fn force(&mut self, feature: Feature) {
        self.forced = Some(feature);
    }

    pub fn set_density(&mut self, density: ParticleDensity) {
        if density != self.density {
            self.density = density;
            self.resize();
        }
    }

    /// The nearest point to `start` in the canvas that no board sits on.
    fn clear_of_boards(&self, ctx: &SceneContext, start: Vec2D) -> Vec2D {
        let mut point = start;
        for _ in 0..6 {
            let mut moved = false;
            for region in ctx.visible() {
                if let Some((strength, away)) = board_influence(&region.board, point) {
                    point += away * (BOARD_MARGIN * (0.5 + strength));
                    moved = true;
                }
            }
            if !moved {
                break;
            }
        }
        Vec2D::new(
            point.x().clamp(self.canvas.x(), self.canvas.right()),
            point.y().clamp(self.canvas.y(), self.canvas.bottom()),
        )
    }

    fn tick(&mut self, delta: Duration, ctx: &SceneContext) {
        let delta_time = delta.as_secs_f64().min(MAX_STEP);
        if delta_time <= 0.0 {
            return;
        }
        self.time += delta_time;
        self.colors.update(delta_time, ctx);
        self.drain_events(ctx);
        self.update_focus(delta_time, ctx);

        let transition = match self.forced.take() {
            Some(feature) => self.director.interrupt(feature),
            None => self.director.update(delta_time, self.colors.energy()),
        };
        self.apply_transition(transition, Some(ctx));

        self.update_effects(delta_time);
        self.update_particles(delta_time, ctx);
        self.update_links();
    }

    fn update_focus(&mut self, delta_time: f64, ctx: &SceneContext) {
        self.focus += self.focus_velocity * delta_time;
        let inset = 0.2;
        let (min_x, max_x) = (
            self.canvas.x() + self.canvas.width() * inset,
            self.canvas.right() - self.canvas.width() * inset,
        );
        let (min_y, max_y) = (
            self.canvas.y() + self.canvas.height() * inset,
            self.canvas.bottom() - self.canvas.height() * inset,
        );
        if self.focus.x() < min_x || self.focus.x() > max_x {
            self.focus_velocity = Vec2D::new(-self.focus_velocity.x(), self.focus_velocity.y());
        }
        if self.focus.y() < min_y || self.focus.y() > max_y {
            self.focus_velocity = Vec2D::new(self.focus_velocity.x(), -self.focus_velocity.y());
        }
        self.focus = Vec2D::new(
            self.focus.x().clamp(min_x, max_x),
            self.focus.y().clamp(min_y, max_y),
        );
        self.focus = self.clear_of_boards(ctx, self.focus);
    }

    fn apply_transition(&mut self, transition: Transition, ctx: Option<&SceneContext>) {
        match transition {
            Transition::None => {}
            Transition::Ambient(ambient) => {
                self.ambient = ambient;
                self.release_cast();
                self.formation = Formation::Free;
            }
            Transition::Gather(feature) => {
                let Some(ctx) = ctx else {
                    return;
                };
                let formation = self.build_formation(feature, ctx);
                self.formation = formation;
                self.assign_cast();
            }
            Transition::Hold(_) => {}
            Transition::Shatter(_) => self.shatter(),
        }
    }

    fn build_formation(&mut self, feature: Feature, ctx: &SceneContext) -> Formation {
        let aspect = self.canvas_aspect();
        let boards = ctx
            .visible()
            .map(|region| {
                let top_left = self
                    .canvas
                    .normalise(Vec2D::new(region.board.x(), region.board.y()));
                let size = self
                    .canvas
                    .normalise(Vec2D::new(region.board.right(), region.board.bottom()))
                    - top_left;
                RectF::new(
                    top_left.x(),
                    top_left.y(),
                    size.x().max(1e-6),
                    size.y().max(1e-6),
                )
            })
            .collect::<Vec<RectF>>();
        match feature {
            Feature::Sprite => {
                // outlines only come from the themes the players are on
                let themes = ctx.regions().map(|r| r.theme).collect::<Vec<usize>>();
                let bus = self.bus.clone();
                let bus = bus.borrow();
                let available = themes
                    .iter()
                    .filter(|theme| bus.shapes.has_shapes(**theme))
                    .copied()
                    .collect::<Vec<usize>>();
                if available.is_empty() {
                    // nothing outlined yet: fall back to something that needs no art
                    return Formation::lattice(&mut self.rng);
                }
                let theme = available[self.rng.random_range(0..available.len())];
                let shapes = bus.shapes.shapes(theme);
                let shape = &shapes[self.rng.random_range(0..shapes.len())];
                Formation::sprite(shape, aspect, &mut self.rng, &boards)
            }
            Feature::Text => {
                let bus = self.bus.clone();
                let bus = bus.borrow();
                let wanted = self
                    .word
                    .take()
                    .filter(|word| bus.shapes.text(word).is_some());
                let available = ctx
                    .captions
                    .iter()
                    .filter(|caption| bus.shapes.text(caption).is_some())
                    .map(|caption| caption.as_str())
                    .collect::<Vec<&str>>();
                let caption = match wanted {
                    Some(word) => word,
                    None if available.is_empty() => return Formation::ribbon(&mut self.rng),
                    None => available[self.rng.random_range(0..available.len())],
                };
                let shape = bus.shapes.text(caption).unwrap();
                Formation::text(shape, aspect, &mut self.rng, &boards)
            }
            Feature::Lattice => Formation::lattice(&mut self.rng),
            Feature::Oscilloscope => Formation::ribbon(&mut self.rng),
            Feature::Haloes => Formation::haloes(&mut self.rng, aspect),
            Feature::Spiral => Formation::spiral(&mut self.rng, aspect),
            Feature::Lissajous => Formation::lissajous(&mut self.rng, aspect),
            Feature::Weather | Feature::Wells => Formation::Free,
        }
    }

    /// hand the formation all but [`AMBIENT_SHARE`] of the pool
    fn assign_cast(&mut self) {
        self.release_cast();
        let capacity = self.formation.capacity();
        if matches!(self.formation, Formation::Free) {
            return;
        }
        let available = ((self.particles.len() as f64) * (1.0 - AMBIENT_SHARE)).round() as usize;
        let available = available
            .min(self.particles.len() - self.members.iter().filter(|m| m.in_comet).count());
        let wanted = match capacity {
            Some(capacity) => capacity.min(available),
            None => available,
        };
        // a particle flying an attack is not the formation's to take
        let mut indices = (0..self.particles.len())
            .filter(|index| !self.members[*index].in_comet)
            .collect::<Vec<usize>>();
        for i in (1..indices.len()).rev() {
            indices.swap(i, self.rng.random_range(0..=i));
        }
        indices.truncate(wanted);
        self.cast = indices;
    }

    fn release_cast(&mut self) {
        for index in self.cast.drain(..) {
            if let Some(particle) = self.particles.get_mut(index) {
                particle.set_target(None);
            }
        }
    }

    /// the end of a formation: everything it held flies outward
    fn shatter(&mut self) {
        let centre = match self.formation {
            Formation::Sprite { centre, .. } => self.canvas.denormalise(centre),
            _ => self.canvas.center(),
        };
        let cast = std::mem::take(&mut self.cast);
        for index in cast {
            let Some(particle) = self.particles.get_mut(index) else {
                continue;
            };
            particle.set_target(None);
            let delta = particle.position() - centre;
            let direction = if delta.is_zero() {
                Vec2D::new(0.0, -1.0)
            } else {
                delta.unit_vector()
            };
            particle.add_velocity(direction * (0.15 + 0.2 * self.rng.random::<f64>()));
        }
        self.add_wave(Shockwave::ring(centre, 0.3, ParticleColor::WHITE).with_speed(1.2));
        self.formation = Formation::Free;
    }

    /// caps the live shockwaves at what the density allows
    fn add_wave(&mut self, wave: Shockwave) {
        if self.waves.len() < self.density.max_effects() {
            self.waves.push(wave);
        }
    }

    fn drain_events(&mut self, ctx: &SceneContext) {
        let events = std::mem::take(&mut self.bus.borrow_mut().events);
        for event in events {
            self.react(event, ctx);
        }
    }

    fn react(&mut self, event: FieldEvent, ctx: &SceneContext) {
        match event {
            FieldEvent::Clear {
                player,
                rows,
                class,
                count,
                is_combo,
            } => {
                self.colors.on_clear(class, count, is_combo);
                let color = ctx
                    .region(player)
                    .map(|r| r.palette.pick(self.colors.phase()))
                    .unwrap_or(ParticleColor::WHITE);
                let strength = 0.12 + 0.06 * class as f64 + if is_combo { 0.08 } else { 0.0 };
                for row in rows.iter().take(2) {
                    self.add_wave(Shockwave::horizontal(row.center(), strength, color));
                }
                // every game grades its largest clear as class 3
                if class >= 3 {
                    self.colors.add_energy(0.5);
                    let centre = rows
                        .first()
                        .map(|r| r.center())
                        .unwrap_or_else(|| self.canvas.center());
                    self.add_wave(Shockwave::ring(centre, 0.45, color).with_speed(1.15));
                    let transition = self.director.interrupt(Feature::Sprite);
                    self.apply_transition(transition, Some(ctx));
                }
            }
            FieldEvent::SpeedUp { .. } => {
                self.colors.on_speed_up();
                self.add_wave(
                    Shockwave::ring(self.canvas.center(), 0.25, ParticleColor::WHITE)
                        .with_speed(1.5),
                );
                for particle in self.particles.iter_mut() {
                    particle.set_velocity(particle.velocity() * 1.25);
                }
            }
            FieldEvent::Attack { from, to, strength } => self.launch_comet(from, to, strength, ctx),
            FieldEvent::AttackReceived { player } => {
                if let Some(region) = ctx.region(player).filter(|r| r.in_canvas) {
                    self.palls.push(Pall::new(region.clip, 1.6, 0.35, 0.6));
                }
            }
            FieldEvent::Victory { player } | FieldEvent::StageComplete { player } => {
                self.colors.add_energy(0.7);
                if let Some(region) = ctx.region(player).filter(|r| r.in_canvas) {
                    let color = region.palette.pick(self.colors.phase());
                    self.add_wave(
                        Shockwave::ring(region.board.center(), 0.4, color).with_speed(1.15),
                    );
                }
            }
            FieldEvent::GameOver { player } => {
                if let Some(region) = ctx.region(player).filter(|r| r.in_canvas) {
                    self.palls.push(Pall::new(region.clip, 4.0, 0.5, 0.9));
                }
                self.spell(words::GAME_OVER, ctx);
            }
            FieldEvent::Spell { word } => self.spell(word, ctx),
        }
    }

    /// Take the field over and spell `word`; ignored until the renderer has outlined it.
    fn spell(&mut self, word: &'static str, ctx: &SceneContext) {
        if self.bus.borrow().shapes.text(word).is_none() {
            return;
        }
        self.word = Some(word);
        let transition = self.director.interrupt(Feature::Text);
        self.apply_transition(transition, Some(ctx));
    }

    fn launch_comet(&mut self, from: u32, to: u32, strength: u32, ctx: &SceneContext) {
        if self.comets.len() >= self.density.max_effects() {
            return;
        }
        let end_of = |player: u32| -> Option<AttackEnd> {
            ctx.region(player)
                .map(|region| AttackEnd::new(region.board, region.in_canvas))
        };
        let (Some(attacker), Some(victim)) = (end_of(from), end_of(to)) else {
            return;
        };
        let Some((start, end)) = attack_endpoints(self.canvas, attacker, victim) else {
            return;
        };
        let color = ctx
            .region(from)
            .map(|r| r.palette.pick(self.colors.phase()))
            .unwrap_or(ParticleColor::WHITE);

        let wanted = (8 + 6 * strength.min(6) as usize).min(self.particles.len() / 4);
        let members = self.conscript(wanted);
        if members.is_empty() {
            return;
        }
        for index in members.iter() {
            self.particles[*index].set_target(None);
            self.particles[*index].set_position(start);
        }
        self.comets
            .push(Comet::new(start, end, strength, color, members));
    }

    /// take particles out of the ambient field for a comet, preferring ones not already busy
    fn conscript(&mut self, wanted: usize) -> Vec<usize> {
        let mut taken = vec![];
        let len = self.particles.len();
        if len == 0 {
            return taken;
        }
        let start = self.rng.random_range(0..len);
        for offset in 0..len {
            if taken.len() >= wanted {
                break;
            }
            let index = (start + offset) % len;
            if self.members[index].in_comet || self.cast.contains(&index) {
                continue;
            }
            self.members[index].in_comet = true;
            taken.push(index);
        }
        taken
    }

    fn update_effects(&mut self, delta_time: f64) {
        for wave in self.waves.iter_mut() {
            wave.update(delta_time);
        }
        self.waves.retain(|wave| !wave.is_spent());

        for pall in self.palls.iter_mut() {
            pall.update(delta_time);
        }
        self.palls.retain(|pall| !pall.is_spent());

        let mut arrivals: Vec<(Vec2D, ParticleColor, Vec<usize>)> = vec![];
        for comet in self.comets.iter_mut() {
            let arrived = comet.update(delta_time);
            for (slot, index) in comet.members().iter().enumerate() {
                let target = comet.position_of(slot);
                self.particles[*index].set_target(Some(ParticleTarget::new(target, 260.0)));
            }
            if arrived {
                arrivals.push((comet.target(), comet.color(), comet.members().to_vec()));
            }
        }
        for (target, color, members) in arrivals {
            for index in members {
                self.members[index].in_comet = false;
                self.particles[index].set_target(None);
                let direction = Vec2D::new(
                    self.rng.random::<f64>() - 0.5,
                    self.rng.random::<f64>() - 0.5,
                )
                .unit_vector();
                self.particles[index].add_velocity(direction * 0.6);
            }
            self.add_wave(Shockwave::ring(target, 0.35, color));
        }
        self.comets.retain(|comet| !comet.is_spent());
    }

    fn update_particles(&mut self, delta_time: f64, ctx: &SceneContext) {
        let aspect = self.canvas_aspect();
        let energy = self.colors.energy();
        let elapsed = self.director.phase_elapsed();
        let gathering = matches!(
            self.director.stage(),
            Stage::Feature {
                phase: Phase::Gather | Phase::Hold,
                ..
            }
        );
        let count = self.cast.len();

        if gathering {
            for (slot, index) in self.cast.iter().enumerate() {
                let Some(target) = self.formation.target(slot, count, elapsed, aspect, energy)
                else {
                    continue;
                };
                let target = self.canvas.denormalise(target);
                self.particles[*index]
                    .set_target(Some(ParticleTarget::new(target, GATHER_STIFFNESS)));
            }
        }

        let field = self.ambient_field(ctx);
        let feature = match self.director.stage() {
            Stage::Feature { routine, .. } if !routine.is_formation() => Some(routine),
            _ => None,
        };

        for index in 0..self.particles.len() {
            let member = self.members[index];
            let has_target = self.particles[index].target().is_some();

            let active_field = if has_target { None } else { Some(&field) };
            if !has_target {
                if let Some(feature) = feature {
                    self.apply_free_feature(index, feature, delta_time, ctx);
                }
                // only the gravity wells routine may gather on a board
                if feature != Some(Feature::Wells) {
                    self.push_off_boards(index, delta_time, ctx);
                }
                self.settle(index, delta_time);
            }
            self.particles[index].step(delta_time, active_field);

            let mut flash = 0.0;
            for wave in self.waves.iter() {
                flash += wave.apply(&mut self.particles[index], delta_time);
            }
            let mut drain = 0.0;
            for pall in self.palls.iter() {
                drain += pall.apply(&mut self.particles[index], delta_time);
            }

            if !has_target && !member.in_comet {
                self.wrap(index);
            }
            self.paint(index, ctx, flash.min(1.0), drain.min(1.0));
        }
    }

    fn ambient_field(&self, ctx: &SceneContext) -> Field {
        match self.ambient {
            Ambient::Orbit => Field::Orbit(self.clear_of_boards(ctx, self.canvas.center())),
            Ambient::Flow => Field::Flow {
                scale: 7.0,
                strength: 0.05,
                phase: self.time * 0.25,
            },
            Ambient::Vortex => Field::Vortex {
                centre: self.focus,
                strength: 0.012,
            },
            Ambient::Constellation => Field::Flow {
                scale: 3.0,
                strength: 0.012,
                phase: self.time * 0.1,
            },
        }
    }

    fn apply_free_feature(
        &mut self,
        index: usize,
        feature: Feature,
        delta_time: f64,
        ctx: &SceneContext,
    ) {
        match feature {
            Feature::Weather => {
                let speed = ctx.regions().map(|r| r.speed_index).max().unwrap_or(0) as f64;
                let fall = 0.18 + 0.02 * speed.min(15.0);
                let wind = (self.time * 0.4).sin() * 0.05;
                self.particles[index].add_velocity(Vec2D::new(wind, fall) * delta_time);
            }
            Feature::Wells => {
                // a board pulls the field in, and pushes it away once its stack nears the top
                for region in ctx.visible() {
                    let centre = region.board.center();
                    let field = if region.danger > 0.7 {
                        Field::Repel {
                            centre,
                            strength: 0.004,
                        }
                    } else {
                        Field::Orbit(centre)
                    };
                    field.apply(&mut self.particles[index], delta_time * 4.0);
                }
            }
            _ => {}
        }
    }

    fn push_off_boards(&mut self, index: usize, delta_time: f64, ctx: &SceneContext) {
        let position = self.particles[index].position();
        for region in ctx.visible() {
            if let Some((strength, away)) = board_influence(&region.board, position) {
                self.particles[index].add_velocity(away * (BOARD_PUSH * strength * delta_time));
            }
        }
    }

    fn board_shade(ctx: &SceneContext, point: Vec2D) -> f64 {
        ctx.visible()
            .filter_map(|region| board_influence(&region.board, point))
            .map(|(strength, _)| 1.0 - strength * (1.0 - BOARD_ALPHA))
            .fold(1.0, f64::min)
    }

    fn settle(&mut self, index: usize, delta_time: f64) {
        let velocity = self.particles[index].velocity();
        let speed = velocity.magnitude();
        if speed > MAX_DRIFT {
            self.particles[index].set_velocity(velocity * (1.0 - (3.0 * delta_time).min(0.9)));
        } else if speed < MIN_DRIFT {
            let nudge = Vec2D::new(
                self.rng.random::<f64>() - 0.5,
                self.rng.random::<f64>() - 0.5,
            );
            self.particles[index].add_velocity(nudge.unit_vector() * (0.08 * delta_time));
        }
    }

    /// Wrap a particle that left the canvas to the far edge with a fresh gentle drift across it;
    /// keeping its heading re-admits a shockwave's worth as one clump.
    fn wrap(&mut self, index: usize) {
        let position = self.particles[index].position();
        let horizontal = if position.x() < self.canvas.x() - WRAP_MARGIN {
            Some(-1.0)
        } else if position.x() > self.canvas.right() + WRAP_MARGIN {
            Some(1.0)
        } else {
            None
        };
        let vertical = if position.y() < self.canvas.y() - WRAP_MARGIN {
            Some(-1.0)
        } else if position.y() > self.canvas.bottom() + WRAP_MARGIN {
            Some(1.0)
        } else {
            None
        };
        if horizontal.is_none() && vertical.is_none() {
            return;
        }

        let along = self.rng.random::<f64>();
        let drift =
            RE_ENTRY_DRIFT.0 + (RE_ENTRY_DRIFT.1 - RE_ENTRY_DRIFT.0) * self.rng.random::<f64>();
        let spread = (self.rng.random::<f64>() - 0.5) * drift;
        // just inside, so it is drawn from the frame it returns
        let inset = 0.001;
        let (position, velocity) = match (horizontal, vertical) {
            (Some(direction), _) => (
                Vec2D::new(
                    if direction > 0.0 {
                        self.canvas.x() + inset
                    } else {
                        self.canvas.right() - inset
                    },
                    self.canvas.y() + self.canvas.height() * along,
                ),
                Vec2D::new(direction * drift, spread),
            ),
            (None, Some(direction)) => (
                Vec2D::new(
                    self.canvas.x() + self.canvas.width() * along,
                    if direction > 0.0 {
                        self.canvas.y() + inset
                    } else {
                        self.canvas.bottom() - inset
                    },
                ),
                Vec2D::new(spread, direction * drift),
            ),
            (None, None) => unreachable!(),
        };
        self.particles[index].set_position(position);
        self.particles[index].set_velocity(velocity);
    }

    fn paint(&mut self, index: usize, ctx: &SceneContext, flash: f64, drain: f64) {
        let member = self.members[index];
        let position = self.particles[index].position();
        let in_formation = self.particles[index].target().is_some() && !member.in_comet;
        let forming = matches!(
            self.director.stage(),
            Stage::Feature {
                routine,
                phase: Phase::Gather | Phase::Hold,
                ..
            } if routine.is_formation()
        );
        let mut color = self.colors.color(ctx, position, member.seed);
        if flash > 0.0 {
            color = color.lerp(ParticleColor::WHITE, flash * 0.7);
        }
        if drain > 0.0 {
            let (_, _, value) = color.to_hsv();
            color = color.lerp(ParticleColor::rgb(value, value, value), drain * 0.8);
        }
        if member.in_comet {
            color = color.lerp(ParticleColor::WHITE, 0.35);
        }
        self.particles[index].set_color(color);

        let shade = Self::board_shade(ctx, position);
        let shade = if in_formation {
            shade.max(FORMATION_BOARD_ALPHA)
        } else {
            shade
        };
        let focus = match (forming, in_formation) {
            (true, true) => 1.2,
            (true, false) => AMBIENT_UNDER_FORMATION,
            _ => 1.0,
        };
        let alpha = self.colors.alpha(member.base_alpha) * (1.0 - 0.5 * drain) * shade * focus;
        self.particles[index].set_alpha(alpha + flash * 0.3 * shade);

        let size = if member.in_comet {
            member.base_size * 1.4
        } else if self.particles[index].target().is_some() {
            // at full size adjacent lattice sprites overlap and the shape turns to mush
            member.base_size * 0.65
        } else {
            member.base_size
        };
        self.particles[index].set_size(size);
    }

    fn update_links(&mut self) {
        let budget = self.density.link_budget();
        let (budget, alpha) = match self.ambient {
            Ambient::Constellation => (budget, 0.85),
            _ => (budget / 2, 0.5),
        };
        if budget == 0 {
            self.link_builder.clear();
            return;
        }
        let radius = LINK_RADIUS * self.canvas.height();
        self.link_builder.build(
            &self.particles,
            self.canvas,
            radius,
            LINK_WIDTH,
            budget,
            alpha * self.colors.alpha(1.0),
        );
    }
}

impl ParticlePool for ParticleField {
    fn update(&mut self, delta: Duration, ctx: &SceneContext) {
        if ctx.canvas != self.canvas {
            self.set_canvas(ctx.canvas);
        }
        if self.particles.len() != self.wanted_len() {
            self.resize();
        }
        self.tick(delta, ctx);
    }

    fn particles(&self) -> &[Particle] {
        &self.particles
    }

    fn clip(&self) -> Option<RectF> {
        Some(self.canvas)
    }

    fn links(&self) -> &[ParticleLink] {
        self.link_builder.links()
    }

    fn force_feature(&mut self, feature: Feature) {
        self.force(feature);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::GameId;
    use crate::particles::field::context::{Palette, PlayerRegion};

    fn region(player: u32, clip: RectF, in_canvas: bool) -> PlayerRegion {
        PlayerRegion {
            player,
            clip,
            board: RectF::new(clip.x() + clip.width() * 0.3, 0.2, clip.width() * 0.4, 0.6),
            theme: 0,
            game: GameId(1),
            palette: Palette::from_sdl(&[
                sdl2::pixels::Color::RGB(0xE8, 0x06, 0x06),
                sdl2::pixels::Color::RGB(0x2E, 0x7B, 0xD6),
            ]),
            in_canvas,
            danger: 0.0,
            speed_index: 3,
            held_up: false,
        }
    }

    fn two_players(both_modern: bool) -> SceneContext {
        SceneContext::new(vec![
            region(0, RectF::new(0.0, 0.0, 0.5, 1.0), true),
            region(1, RectF::new(0.5, 0.0, 0.5, 1.0), both_modern),
        ])
        .unwrap()
    }

    fn field(ctx: &SceneContext) -> ParticleField {
        ParticleField::seeded(
            ctx.canvas,
            (1920, 1080),
            ParticleDensity::High,
            Rc::new(RefCell::new(FieldBus::default())),
            TEST_SEED,
        )
    }

    const FRAME: Duration = Duration::from_micros(16_667);

    fn run(field: &mut ParticleField, ctx: &SceneContext, seconds: f64) {
        for _ in 0..(seconds / FRAME.as_secs_f64()).round() as u32 {
            field.update(FRAME, ctx);
        }
    }

    fn assert_sane(field: &ParticleField, ctx: &SceneContext) {
        for particle in field.particles() {
            let position = particle.position();
            assert!(
                position.x().is_finite() && position.y().is_finite(),
                "{position:?}"
            );
            assert!(
                position.x() > ctx.canvas.x() - 1.0 && position.x() < ctx.canvas.right() + 1.0,
                "{position:?}"
            );
            assert!(
                position.y() > ctx.canvas.y() - 1.0 && position.y() < ctx.canvas.bottom() + 1.0,
                "{position:?}"
            );
            assert!(
                (0.0..=1.0).contains(&particle.alpha()),
                "{}",
                particle.alpha()
            );
            let color = particle.color();
            for channel in [color.red(), color.green(), color.blue()] {
                assert!((0.0..=1.0).contains(&channel), "{color:?}");
            }
        }
    }

    #[test]
    fn the_field_holds_its_population_and_stays_put() {
        let ctx = two_players(true);
        let mut field = field(&ctx);
        let population = field.particles().len();
        assert_eq!(population, ParticleDensity::High.pool_size());
        run(&mut field, &ctx, 120.0);
        assert_eq!(field.particles().len(), population);
        assert_sane(&field, &ctx);
    }

    #[test]
    fn half_a_canvas_is_half_the_particles_not_twice_the_density() {
        let whole = two_players(true);
        let half = two_players(false);
        let mut field = field(&whole);
        run(&mut field, &whole, 2.0);
        let whole_population = field.particles().len();

        field.update(FRAME, &half);
        assert_eq!(field.clip(), Some(half.canvas));
        assert!(
            (field.particles().len() as f64 - whole_population as f64 / 2.0).abs() <= 1.0,
            "{} vs {}",
            field.particles().len(),
            whole_population
        );
        run(&mut field, &half, 30.0);
        assert_sane(&field, &half);
    }

    #[test]
    fn a_canvas_change_brings_the_field_with_it() {
        let whole = two_players(true);
        let half = two_players(false);
        let mut field = field(&whole);
        run(&mut field, &whole, 1.0);
        let index = 0;
        let before = whole.canvas.normalise(field.particles()[index].position());
        field.update(FRAME, &half);
        let after = half.canvas.normalise(field.particles()[index].position());
        assert!(
            (before.x() - after.x()).abs() < 0.05,
            "{before:?} {after:?}"
        );
        assert!(
            (before.y() - after.y()).abs() < 0.05,
            "{before:?} {after:?}"
        );
    }

    #[test]
    fn every_reaction_leaves_the_field_sane() {
        let ctx = two_players(true);
        let mut field = field(&ctx);
        let rows = vec![RectF::new(0.1, 0.5, 0.3, 0.02)];
        for event in [
            FieldEvent::Clear {
                player: 0,
                rows: rows.clone(),
                class: 3,
                count: 4,
                is_combo: true,
            },
            FieldEvent::SpeedUp { player: 1 },
            FieldEvent::Attack {
                from: 0,
                to: 1,
                strength: 4,
            },
            FieldEvent::AttackReceived { player: 1 },
            FieldEvent::StageComplete { player: 0 },
            FieldEvent::Victory { player: 0 },
            FieldEvent::GameOver { player: 1 },
        ] {
            field.bus.borrow_mut().events.push(event);
            run(&mut field, &ctx, 0.5);
            assert_sane(&field, &ctx);
        }
        run(&mut field, &ctx, 20.0);
        assert_sane(&field, &ctx);
    }

    #[test]
    fn an_attack_on_a_player_outside_the_canvas_still_flies() {
        let ctx = two_players(false);
        let mut field = field(&ctx);
        field.bus.borrow_mut().events.push(FieldEvent::Attack {
            from: 0,
            to: 1,
            strength: 3,
        });
        field.update(FRAME, &ctx);
        assert_eq!(field.comets.len(), 1);
        assert_eq!(field.comets[0].target().x(), ctx.canvas.right());
        run(&mut field, &ctx, 3.0);
        assert!(field.comets.is_empty());
        assert!(field.members.iter().all(|m| !m.in_comet));
        assert_sane(&field, &ctx);
    }

    fn board() -> RectF {
        RectF::new(0.2, 0.2, 0.2, 0.6)
    }

    #[test]
    fn a_board_owns_everything_inside_it_and_nothing_far_from_it() {
        let (strength, _) = board_influence(&board(), Vec2D::new(0.3, 0.5)).unwrap();
        assert_eq!(strength, 1.0);
        assert!(board_influence(&board(), Vec2D::new(0.8, 0.5)).is_none());
    }

    #[test]
    fn a_board_pushes_out_the_nearest_way() {
        let (_, away) = board_influence(&board(), Vec2D::new(0.22, 0.5)).unwrap();
        assert_eq!(away, Vec2D::new(-1.0, 0.0));
        let (_, away) = board_influence(&board(), Vec2D::new(0.3, 0.21)).unwrap();
        assert_eq!(away, Vec2D::new(0.0, -1.0));
        let (_, away) = board_influence(&board(), Vec2D::new(0.42, 0.5)).unwrap();
        assert_eq!(away, Vec2D::new(1.0, 0.0));
    }

    #[test]
    fn the_shade_fades_in_over_the_margin_rather_than_stepping() {
        let ctx = two_players(true);
        let board = ctx.players[0].board;
        let inside = ParticleField::board_shade(&ctx, board.center());
        let edge = ParticleField::board_shade(&ctx, Vec2D::new(board.right(), board.center().y()));
        let near = ParticleField::board_shade(
            &ctx,
            Vec2D::new(board.right() + BOARD_MARGIN * 0.5, board.center().y()),
        );
        let clear = ParticleField::board_shade(
            &ctx,
            Vec2D::new(board.right() + BOARD_MARGIN * 2.0, board.center().y()),
        );
        assert!((inside - BOARD_ALPHA).abs() < 1e-9, "{inside}");
        assert!((edge - BOARD_ALPHA).abs() < 1e-9, "{edge}");
        assert!(near > edge && near < clear, "{edge} {near} {clear}");
        assert_eq!(clear, 1.0);
    }

    #[test]
    fn the_field_thins_out_behind_a_board() {
        let ctx = two_players(true);
        let mut field = field(&ctx);
        let boards = ctx.players.iter().map(|p| p.board).collect::<Vec<RectF>>();
        let board_area = boards.iter().map(|b| b.width() * b.height()).sum::<f64>();
        let canvas_area = ctx.canvas.width() * ctx.canvas.height();
        run(&mut field, &ctx, 5.0);

        // sampled over half a minute, since one frame swings as routines come and go
        const SAMPLES: usize = 30;
        let (mut over, mut over_alpha) = (0.0, 0.0);
        let (mut clear, mut clear_alpha) = (0.0, 0.0);
        // formations may cross a board, so only the resting field must thin out
        let (mut resting_over, mut resting_frames) = (0.0, 0.0);
        for _ in 0..SAMPLES {
            run(&mut field, &ctx, 1.0);
            let resting = matches!(field.director.stage(), Stage::Ambient { .. });
            for particle in field.particles() {
                let on_board = boards.iter().any(|b| b.contains(particle.position()));
                if on_board {
                    over += 1.0;
                    over_alpha += particle.alpha();
                    resting_over += resting as u8 as f64;
                } else {
                    clear += 1.0;
                    clear_alpha += particle.alpha();
                }
            }
            resting_frames += resting as u8 as f64;
        }
        assert!(resting_frames > 3.0, "never caught the field at rest");
        let over_alpha = over_alpha / over;
        let clear_alpha = clear_alpha / clear;
        let over_density = resting_over / resting_frames / board_area;
        let overall_density = field.particles().len() as f64 / canvas_area;
        eprintln!(
            "over a board: {over_density:.0} per unit area at rest, alpha {over_alpha:.3}; \
             {overall_density:.0} per unit area overall, {clear_alpha:.3} clear of one"
        );

        assert!(
            over_alpha < clear_alpha * 0.6,
            "{over_alpha:.3} over a board against {clear_alpha:.3} clear of one"
        );
        assert!(
            over_alpha > clear_alpha * 0.15,
            "{over_alpha:.3} over a board is nearly invisible"
        );
        assert!(
            over_density < overall_density * 0.8,
            "{over_density:.0} per unit area over the boards at rest, \
             against {overall_density:.0} overall"
        );
    }

    /// a wrapped particle is re-admitted heading inwards, so a big clear cannot empty the canvas
    #[test]
    fn a_big_clear_never_sweeps_the_field_off_the_canvas() {
        let ctx = two_players(true);
        let mut field = field(&ctx);
        let inside = |field: &ParticleField| {
            field
                .particles()
                .iter()
                .filter(|p| ctx.canvas.contains(p.position()))
                .count()
        };
        run(&mut field, &ctx, 1.0);
        let population = field.particles().len();
        field.bus.borrow_mut().events.push(FieldEvent::Clear {
            player: 0,
            rows: vec![RectF::new(0.05, 0.55, 0.3, 0.02)],
            class: 3,
            count: 4,
            is_combo: true,
        });
        for _ in 0..20 {
            run(&mut field, &ctx, 1.0);
            let inside = inside(&field);
            assert!(
                inside as f64 > population as f64 * 0.7,
                "only {inside} of {population} left on the canvas"
            );
        }
    }

    fn field_knowing(word: &str, ctx: &SceneContext) -> ParticleField {
        let field = field(ctx);
        field.bus.borrow_mut().shapes.insert_text(
            word,
            crate::particles::field::shapes::EdgeShape::unit_square(),
        );
        field
    }

    #[test]
    fn a_word_the_match_calls_for_takes_the_field_over() {
        let ctx = two_players(true);
        let mut field = field_knowing(words::TETRIS, &ctx);
        field.update(FRAME, &ctx);
        field.bus.borrow_mut().events.push(FieldEvent::Spell {
            word: words::TETRIS,
        });
        field.update(FRAME, &ctx);
        assert!(
            matches!(
                field.director.stage(),
                Stage::Feature {
                    routine: Feature::Text,
                    ..
                }
            ),
            "{:?}",
            field.director.stage()
        );
        assert!(matches!(field.formation, Formation::Sprite { .. }));
        assert_eq!(field.word, None, "the word is spent once it is spelt");
    }

    #[test]
    fn a_game_over_spells_it_out() {
        let ctx = two_players(true);
        let mut field = field_knowing(words::GAME_OVER, &ctx);
        field.update(FRAME, &ctx);
        field
            .bus
            .borrow_mut()
            .events
            .push(FieldEvent::GameOver { player: 0 });
        field.update(FRAME, &ctx);
        assert!(matches!(
            field.director.stage(),
            Stage::Feature {
                routine: Feature::Text,
                ..
            }
        ));
    }

    #[test]
    fn a_word_that_has_not_been_outlined_yet_is_let_go() {
        let ctx = two_players(true);
        let mut field = field(&ctx);
        field.update(FRAME, &ctx);
        field
            .bus
            .borrow_mut()
            .events
            .push(FieldEvent::Spell { word: words::COMBO });
        field.update(FRAME, &ctx);
        assert!(matches!(field.director.stage(), Stage::Ambient { .. }));
        assert_eq!(field.word, None);
    }

    #[test]
    fn links_never_exceed_the_density_budget() {
        let ctx = two_players(true);
        let mut field = field(&ctx);
        for _ in 0..600 {
            field.update(FRAME, &ctx);
            assert!(
                field.links().len() <= ParticleDensity::High.link_budget(),
                "{}",
                field.links().len()
            );
        }
    }

    #[test]
    fn the_density_dial_sizes_the_pool() {
        let ctx = two_players(true);
        let mut field = field(&ctx);
        field.set_density(ParticleDensity::Low);
        assert_eq!(field.particles().len(), ParticleDensity::Low.pool_size());
        run(&mut field, &ctx, 5.0);
        assert_sane(&field, &ctx);
    }
}
