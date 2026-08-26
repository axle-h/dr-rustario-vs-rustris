//! The modern themes' background: one retained pool of particles spanning every player on a
//! particle scene, choreographed by a director and pushed about by what happens in the match.
//!
//! It is a *field*, not an emitter: the particles never die. Ones that leave the canvas are
//! re-seeded on the far side, so the population is constant and nothing is ever removed from
//! the middle of a vector. Feature routines retarget the particles that already exist.
//!
//! The field observes the match and never influences it: it reads no game state directly,
//! shares no RNG with the games, and everything it reacts to is handed to it.

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
use rand::{rng, rngs::ThreadRng, RngExt};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// how much of the pool stays ambient while a feature runs, so the field is never empty and
/// never fully hijacked
const AMBIENT_SHARE: f64 = 0.3;
/// the longest step the field will integrate in one go: a stall must not fling it apart
const MAX_STEP: f64 = 1.0 / 20.0;
/// how far outside the canvas a particle may drift before it is re-seeded on the far side
const WRAP_MARGIN: f64 = 0.06;
/// particles slower than this get a nudge, so an ambient routine never settles to a stop
const MIN_DRIFT: f64 = 0.012;
/// and faster than this are damped back down: a gravity well or a shatter can fling one
const MAX_DRIFT: f64 = 0.35;
/// how fast a re-seeded particle comes back into the canvas. Slow enough to read as drift,
/// fast enough that a shockwave's worth of them is back inside within a few seconds.
const RE_ENTRY_DRIFT: (f64, f64) = (0.06, 0.16);
/// how hard a formation pulls
const GATHER_STIFFNESS: f64 = 90.0;
/// how far beyond a playfield's edge the board's influence reaches, in particle space
const BOARD_MARGIN: f64 = 0.06;
/// what a particle's alpha is multiplied by when it is right behind a playfield. The board is
/// the one thing the player is actually reading, and in a bottle the colours *are* the game -
/// but the field should still show through it rather than stop dead at its edge
const BOARD_ALPHA: f64 = 0.35;
/// how hard a board nudges the ambient field away from itself, per second
const BOARD_PUSH: f64 = 0.08;
/// the least a particle that is part of a formation may be dimmed to over a board. A
/// silhouette is a deliberate thing and has to hold together; break it up over a playfield
/// and it reads as a fault rather than as a shape.
const FORMATION_BOARD_ALPHA: f64 = 0.45;
/// what the ambient share is faded to while a formation holds, so the shape is not lost in
/// the drift it is standing in front of
const AMBIENT_UNDER_FORMATION: f64 = 0.45;
/// how far apart two particles may be and still be linked, as a fraction of canvas height
const LINK_RADIUS: f64 = 0.075;
/// link thickness, as a fraction of the window height
const LINK_WIDTH: f64 = 0.0045;

/// What the renderer and the field share: the outlines the renderer builds for it, and the
/// events the match screen queues into it.
#[derive(Default)]
pub struct FieldBus {
    pub shapes: ShapeBank,
    pub events: Vec<FieldEvent>,
}

pub type SharedBus = Rc<RefCell<FieldBus>>;

/// per particle state the [`Particle`] itself has no room for
#[derive(Clone, Copy, Debug)]
struct Member {
    /// stable 0-1: where this particle reads the palette, so colour flows rather than flickers
    seed: f64,
    base_size: f64,
    base_alpha: f64,
    /// conscripted into a comet, and so not the ambient field's to steer
    in_comet: bool,
}

/// How much a board owns a point: 1.0 anywhere inside the playfield, tapering to 0 at
/// [`BOARD_MARGIN`] beyond its edge, with the direction that leaves it the shortest way.
/// `None` once the point is clear of it altogether.
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
        // inside: out by whichever edge is closest, which for a tall narrow playfield is
        // almost always one of the sides
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
    /// the window's width over its height: particle space is normalised to the window, so
    /// without this every circle would be an ellipse
    window_aspect: f64,
    density: ParticleDensity,
    particles: Vec<Particle>,
    members: Vec<Member>,
    director: Director,
    ambient: Ambient,
    colors: ColorDriver,
    formation: Formation,
    /// the particles taking part in the current formation, in target order
    cast: Vec<usize>,
    link_builder: LinkBuilder,
    bus: SharedBus,
    waves: Vec<Shockwave>,
    comets: Vec<Comet>,
    palls: Vec<Pall>,
    /// the wandering centre a vortex or flow reads from
    focus: Vec2D,
    focus_velocity: Vec2D,
    /// a feature staged by [`ParticleField::force`], for the diagnostic renderer
    forced: Option<Feature>,
    /// a word the match has called for, spelt by the next text formation
    word: Option<&'static str>,
    time: f64,
    rng: ThreadRng,
}

impl ParticleField {
    pub fn new(
        canvas: RectF,
        window_size: (u32, u32),
        density: ParticleDensity,
        bus: SharedBus,
    ) -> Self {
        let window_aspect = window_size.0 as f64 / window_size.1.max(1) as f64;
        let mut field = Self {
            canvas,
            window_aspect,
            density,
            particles: vec![],
            members: vec![],
            director: Director::new(),
            ambient: Ambient::Orbit,
            colors: ColorDriver::new(),
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
            rng: rng(),
        };
        field.resize();
        field
    }

    /// the canvas's own width over height in real pixels, which is what formations are
    /// authored against
    fn canvas_aspect(&self) -> f64 {
        (self.canvas.width() * self.window_aspect / self.canvas.height()).max(0.01)
    }

    /// scaled by the share of the window the canvas covers, so half a screen is half as many
    /// particles rather than twice the density
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
        // the same mixture the background has always had: mostly plain circles, with the
        // occasional hollow one or star
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

    /// a theme switch or a resize moved the canvas: bring the field with it rather than
    /// leaving half of it stranded, and give up on any half-finished routine
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

    /// Stage one named feature on the next update, whatever the director had in mind. The
    /// diagnostic renderer's way in: a routine that only comes up every minute or so is not
    /// one you can eyeball by waiting for it.
    pub fn force(&mut self, feature: Feature) {
        self.forced = Some(feature);
    }

    pub fn set_density(&mut self, density: ParticleDensity) {
        if density != self.density {
            self.density = density;
            self.resize();
        }
    }

    /// A point in the canvas that no board is sitting on. A gravity well behind a playfield
    /// is a permanent stream of particles through the one place the player is trying to read.
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

    /// the point a vortex winds around and a flow reads from, wandering inside the canvas
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
        // a vortex winding up behind a board is the same problem as a well sitting on one
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
        // the playfields, in the canvas-normalised coordinates the routines are authored in
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
                // the sprite set is the union of the games being played: the outlines are
                // only ever those of the themes the players are actually on
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
                // a word the match called for, else whatever the match screen has offered
                // and the renderer has outlined
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

    /// hand roughly 70% of the pool to the formation, leaving the rest ambient
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
        // a particle already flying an attack is not the formation's to take
        let mut indices = (0..self.particles.len())
            .filter(|index| !self.members[*index].in_comet)
            .collect::<Vec<usize>>();
        // a shuffle, so a formation is not always built from the same particles
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

    /// every shockwave goes through here, so a burst of clears can never pile up more of them
    /// than the density allows
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
                // the big one: both games grade their largest clear as class 3, so a Tetris
                // and a four virus combo take the field over the same way
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
                // the loser's half desaturates and falls
                if let Some(region) = ctx.region(player).filter(|r| r.in_canvas) {
                    self.palls.push(Pall::new(region.clip, 4.0, 0.5, 0.9));
                }
                self.spell(words::GAME_OVER, ctx);
            }
            FieldEvent::Spell { word } => self.spell(word, ctx),
        }
    }

    /// take the field over and spell `word` out. Ignored if the renderer has not outlined it
    /// yet, which is a couple of frames at the start of a match: a word half spelt is worse
    /// than one missed.
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

        // thickness and count scale with how big the attack is
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

        // comets steer their own members, then burst on arrival
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

        // hand the formation's members their targets
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

            // a particle with somewhere to be is not the ambient field's to steer
            let active_field = if has_target { None } else { Some(&field) };
            if !has_target {
                if let Some(feature) = feature {
                    self.apply_free_feature(index, feature, delta_time, ctx);
                }
                // the gravity wells routine is the one thing allowed to gather on a board
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
            // a constellation wants its particles drifting slowly and evenly, so the links
            // between them keep making and breaking: a very weak flow is enough
            Ambient::Constellation => Field::Flow {
                scale: 3.0,
                strength: 0.012,
                phase: self.time * 0.1,
            },
        }
    }

    /// the routines that steer with forces rather than targets
    fn apply_free_feature(
        &mut self,
        index: usize,
        feature: Feature,
        delta_time: f64,
        ctx: &SceneContext,
    ) {
        match feature {
            Feature::Weather => {
                // speed tied to how fast the pieces are falling
                let speed = ctx.regions().map(|r| r.speed_index).max().unwrap_or(0) as f64;
                let fall = 0.18 + 0.02 * speed.min(15.0);
                let wind = (self.time * 0.4).sin() * 0.05;
                self.particles[index].add_velocity(Vec2D::new(wind, fall) * delta_time);
            }
            Feature::Wells => {
                // every board pulls the field toward it, and a board whose stack is near the
                // top pushes it away instead
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

    /// boards hold the drifting field off them, so it thins out over a playfield instead of
    /// piling up behind it
    fn push_off_boards(&mut self, index: usize, delta_time: f64, ctx: &SceneContext) {
        let position = self.particles[index].position();
        for region in ctx.visible() {
            if let Some((strength, away)) = board_influence(&region.board, position) {
                self.particles[index].add_velocity(away * (BOARD_PUSH * strength * delta_time));
            }
        }
    }

    /// how much of its brightness a particle keeps where it is
    fn board_shade(ctx: &SceneContext, point: Vec2D) -> f64 {
        ctx.visible()
            .filter_map(|region| board_influence(&region.board, point))
            .map(|(strength, _)| 1.0 - strength * (1.0 - BOARD_ALPHA))
            .fold(1.0, f64::min)
    }

    /// an ambient routine should never settle to a stop, and nothing in it should be moving
    /// like a bullet either: a 1/r^2 well or a shatter can leave a particle very fast indeed
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

    /// particles never die: one that leaves the canvas comes back in on the far side.
    ///
    /// It comes back anywhere along that edge with a fresh gentle drift *across* the canvas,
    /// rather than keeping the speed and heading it left with. A shockwave sweeps a good part
    /// of the field off one side at once, and re-admitting all of it at the same point moving
    /// the same way leaves a clump crossing the canvas for the next several seconds.
    fn wrap(&mut self, index: usize) {
        let position = self.particles[index].position();
        let horizontal = if position.x() < self.canvas.x() - WRAP_MARGIN {
            // off the left, so back in from the right, heading left
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
        // just inside the far edge, so it is drawn from the frame it returns
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
        // while a formation holds, the drift behind it steps back
        let focus = match (forming, in_formation) {
            (true, true) => 1.2,
            (true, false) => AMBIENT_UNDER_FORMATION,
            _ => 1.0,
        };
        let alpha = self.colors.alpha(member.base_alpha) * (1.0 - 0.5 * drain) * shade * focus;
        self.particles[index].set_alpha(alpha + flash * 0.3 * shade);

        // formation members tighten up, comet members burn brighter and bigger
        let size = if member.in_comet {
            member.base_size * 1.4
        } else if self.particles[index].target().is_some() {
            // a formation reads by its outline, so its members tighten up: at full size the
            // sprites of adjacent lattice points overlap and the shape turns to mush
            member.base_size * 0.65
        } else {
            member.base_size
        };
        self.particles[index].set_size(size);
    }

    fn update_links(&mut self) {
        let budget = self.density.link_budget();
        // links belong to the constellation, and are a hint of one everywhere else
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
        ParticleField::new(
            ctx.canvas,
            (1920, 1080),
            ParticleDensity::High,
            Rc::new(RefCell::new(FieldBus::default())),
        )
    }

    /// one frame at 60fps
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
            // a shatter or a comet can throw one well past the edge, but never off to
            // infinity: anything loose is wrapped and slowed
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
        // long enough to run several ambient routines and several features
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

        // a theme switch takes the right hand player retro under the field's feet
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
        // a particle in the middle of the window is in the middle of the left half after
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
        // the right hand player is retro, so only the left half is drawn
        let ctx = two_players(false);
        let mut field = field(&ctx);
        field.bus.borrow_mut().events.push(FieldEvent::Attack {
            from: 0,
            to: 1,
            strength: 3,
        });
        field.update(FRAME, &ctx);
        assert_eq!(field.comets.len(), 1);
        // and it leaves on the victim's side
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
        // just inside the left edge of a tall narrow playfield: out to the left, not up
        let (_, away) = board_influence(&board(), Vec2D::new(0.22, 0.5)).unwrap();
        assert_eq!(away, Vec2D::new(-1.0, 0.0));
        // just inside the top edge, which is nearer than either side from here
        let (_, away) = board_influence(&board(), Vec2D::new(0.3, 0.21)).unwrap();
        assert_eq!(away, Vec2D::new(0.0, -1.0));
        // outside, straight out from the nearest point on the rect
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

        // how much is over a playfield at any one moment swings about as routines come and
        // go, so this samples over half a minute rather than trusting a single frame
        const SAMPLES: usize = 30;
        let (mut over, mut over_alpha) = (0.0, 0.0);
        let (mut clear, mut clear_alpha) = (0.0, 0.0);
        // a formation is authored across the whole canvas and is allowed to cross a board -
        // it is placed away from them where it can be, and dimmed where it cannot - so it is
        // the resting field that has to thin out
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

        // toned down, but still plainly visible through the board
        assert!(
            over_alpha < clear_alpha * 0.6,
            "{over_alpha:.3} over a board against {clear_alpha:.3} clear of one"
        );
        assert!(
            over_alpha > clear_alpha * 0.15,
            "{over_alpha:.3} over a board is nearly invisible"
        );
        // and thinner, because the boards hold the drift off them
        assert!(
            over_density < overall_density * 0.8,
            "{over_density:.0} per unit area over the boards at rest, \
             against {overall_density:.0} overall"
        );
    }

    /// the whole field once swept itself off the canvas and sat in a ring just outside it,
    /// because a wrapped particle was re-admitted on the far side still heading outward
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
            // the bug this guards against left about 5% of the field on the canvas; the
            // usual figure is 80-95%, the rest being briefly out in the wrap margin
            assert!(
                inside as f64 > population as f64 * 0.7,
                "only {inside} of {population} left on the canvas"
            );
        }
    }

    /// a field with one word already outlined, as the renderer would have left it
    fn field_knowing(word: &str, ctx: &SceneContext) -> ParticleField {
        let mut field = field(ctx);
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
        // it is spelling the word it was given, not falling back to a ribbon
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
        // half a word is worse than none: the field carries on with what it was doing
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
