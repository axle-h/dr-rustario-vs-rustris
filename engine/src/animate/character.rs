//! A character beside the board whose face answers how that player's match is going.
//!
//! This is the state machine and its clocks; the art is [`crate::render::character`] and the
//! per-game cast is a theme's business. Nothing here knows what game is being played: a
//! character is entered into a state by engine [`crate::game::GameEvent`]s and by two numbers
//! the match screen reads every frame, and all four states mean the same thing in any game.
//!
//! It is close to [`crate::animate::mascot`] and deliberately not part of it. A mascot's strip
//! is chosen by the *animation phase* - a piece is spawning, the match is over - and there is
//! no seam there that reads how the board is going. This reads exactly that, and nothing else.
//!
//! Mean Bean Machine draws the opponent in this box, reacting to you. A panel here belongs to
//! one player and there may be only one of them, so the face is that **player's own** and every
//! state below is read off that player's board.

use crate::animate::frames::{FrameAnimation, FrameAnimationType};
use std::time::Duration;

/// The four rows of a character's sheet, in the order every source draws them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CharacterState {
    Idle,
    Winning,
    Losing,
    Defeat,
}

impl CharacterState {
    pub const ALL: [CharacterState; 4] = [
        CharacterState::Idle,
        CharacterState::Winning,
        CharacterState::Losing,
        CharacterState::Defeat,
    ];

    pub fn index(&self) -> usize {
        match self {
            CharacterState::Idle => 0,
            CharacterState::Winning => 1,
            CharacterState::Losing => 2,
            CharacterState::Defeat => 3,
        }
    }
}

/// Once entered, a state holds this long whatever else happens short of a game over.
///
/// Without it a face can be entered and left inside one animation cycle, which reads as a
/// glitch rather than as a reaction.
pub const MIN_DWELL: Duration = Duration::from_millis(700);

/// How long `winning` outlives its last clear, and `losing` the last thing that caused it.
///
/// A chain fires one clear per *step*, so this is refreshed by each one: a nine chain holds the
/// face for nine steps and a bit rather than restarting it. And a chain that cancels the tray
/// outright must not snap the face back mid-pop.
pub const LINGER: Duration = Duration::from_millis(1200);

/// Enter `losing` above this much of the board filled, and leave it only below the other.
///
/// Two numbers rather than one, with a real gap: a stack sitting exactly on a single threshold
/// strobes as each piece locks. The dial is the highest column as a fraction of the visible
/// height, which is [`crate::app::screens::match_screen`]'s `stack_danger`.
pub const DANGER_ENTER: f64 = 0.60;
pub const DANGER_LEAVE: f64 = 0.45;

/// A small sprite drawn **over** the portrait, in the box, on a clock of its own.
///
/// The one idea that collapsed two kinds into one. The reading found what looked like two
/// separate things - an *overlay* (Humpty's arc crackling between his antennae, his wrung
/// hands, Sir Ffuzzy-Logik's eyes) and a *palette cycle* (Arms' rim lights pulsing, Coconuts'
/// coin flaring, Ffuzzy's eye yellow) - and they are the same thing: a small sprite, at an
/// anchor in box coordinates, on its own clock. A cycle's variants merely happen to be
/// recolours, and the ripper bakes each step as a frame. So palette cycling stays out of the
/// renderer, which is what three sprites wanting it never justified.
///
/// A layer has to be its own clock and cannot be baked into the portrait: measured across
/// three characters, a row's cycle and its pose animation never divide - Coconuts' winning row
/// is a 1.36 s wink over a 0.60 s flare - so one strip carrying both would need the product of
/// the two.
#[derive(Clone, Debug)]
pub struct LayerMeta {
    /// (frames, how it plays) per [`CharacterState`]; **zero frames means it is not drawn in
    /// that state at all**, which is Coconuts' coin on idle and Ffuzzy's eyes off his losing row
    pub states: [(usize, FrameAnimationType); 4],
    /// where it goes in the box. More than one and it *jumps* between them, which is what
    /// Humpty's arc and his wrung hands do - they do not travel, they appear at one of a
    /// handful of slots, flash, and go.
    pub anchors: &'static [(i32, i32)],
    /// how often it moves to another anchor; `None` pins it to the first
    pub wander: Option<Duration>,
}

/// What makes an emitter fire.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EmitterTrigger {
    /// a burst on a clock, for as long as the state is up - Frankly's sparks
    Every(Duration),
    /// a burst the moment the portrait reaches one of these frames of its strip, which is how
    /// Humpty fires from his antenna tips exactly as they are drawn in. A slice rather than one
    /// frame because a strip may run its gesture more than once - his runs it twice.
    OnFrame(&'static [usize]),
    /// one particle at a time, at a rate scaling with how badly the board is going - the
    /// sweat, and the only trigger that is not the character's own
    Danger { above: f64, per_second: f64 },
}

/// Where an emitter throws from, and which ways.
///
/// A source carries its own directions because Frankly's two antenna balls do not throw the
/// same way: each omits the diagonal that would go into his body, so the left ball throws
/// up-left, up-right and down-left and the right ball up-right, up-left and down-right.
#[derive(Clone, Copy, Debug)]
pub struct EmitterSource {
    /// in box coordinates
    pub at: (f64, f64),
    /// one particle each, as unit-ish vectors with y downward
    pub directions: &'static [(f64, f64)],
}

/// Particles thrown off a character, which **leave the box** and are not clipped to it.
///
/// On the Genesis these cross the stone of the centre column and go on over the playfield, so
/// they are drawn on the window rather than into the panel - the same seam
/// [`crate::animate::debris`] uses, and for the same reason.
#[derive(Clone, Debug)]
pub struct EmitterMeta {
    /// what fires it per [`CharacterState`]; `None` and it is silent in that state
    pub triggers: [Option<EmitterTrigger>; 4],
    pub sources: &'static [EmitterSource],
    /// box pixels per axis per 60 Hz frame, which is how the captures were measured
    pub speed: f64,
    pub life: Duration,
    /// the fraction of its life a particle spends fading out
    pub fade_last: f64,
    /// the particle's own strip: four bolts for Frankly, which each spark cycles through as
    /// it flies rather than one bolt per spark
    pub frames: usize,
    pub fps: u32,
}

/// One particle in the air, in **box coordinates** - so a mirror is one flip at draw time.
#[derive(Clone, Copy, Debug)]
pub struct CharacterParticle {
    pub x: f64,
    pub y: f64,
    velocity: (f64, f64),
    elapsed: Duration,
    life: Duration,
    fade_last: f64,
    /// which emitter of the character's owns it, and so whose art it is drawn in
    pub emitter: usize,
    pub frame: usize,
    frames: usize,
    fps: u32,
}

impl CharacterParticle {
    /// 0..=255, for `set_alpha_mod`. They are last seen fading rather than stopping.
    pub fn alpha(&self) -> u8 {
        let through = self.elapsed.as_secs_f64() / self.life.as_secs_f64().max(f64::EPSILON);
        if self.fade_last <= 0.0 || through < 1.0 - self.fade_last {
            return 255;
        }
        let fade = (1.0 - through) / self.fade_last;
        (fade.clamp(0.0, 1.0) * 255.0) as u8
    }
}

/// How many particles one character may have in the air.
///
/// A burst is six and they live a fifth of a second, so this is never approached by a
/// character's own emitters; it is the sweat, which fires on a dial that a buried board holds
/// wide open, that needs a ceiling at all.
const MAX_PARTICLES: usize = 64;

/// A well spread deterministic sequence, the golden ratio walk the nuisance stagger uses.
///
/// Deterministic rather than random on purpose: this is on the render path, and the same
/// character in the same state should look the same twice. It only has to *spread*, and a
/// low-discrepancy sequence spreads better than an RNG does over a handful of draws.
fn hashed(n: u64) -> f64 {
    (n as f64 * 0.618_033_988_749_894_9).fract()
}

/// Everything a theme declares about one character: how each of its four rows plays.
///
/// Frame counts are the strip's own, in play order - a row is cut action first and rest last so
/// that [`FrameAnimationType::LinearWithPause`], which holds the *last* frame, holds the pose
/// the character rests in.
#[derive(Clone, Debug)]
pub struct CharacterMeta {
    /// (frames, how it plays) per [`CharacterState`], in `CharacterState::ALL` order
    pub states: [(usize, FrameAnimationType); 4],
    /// what is drawn over the portrait, in the box
    pub layers: Vec<LayerMeta>,
    /// what is thrown off it, which leaves the box
    pub emitters: Vec<EmitterMeta>,
}

impl CharacterMeta {
    /// a cast with no extras at all, which is nine of Mean Bean Machine's thirteen
    pub fn plain(states: [(usize, FrameAnimationType); 4]) -> Self {
        Self {
            states,
            layers: vec![],
            emitters: vec![],
        }
    }

    fn animation(&self, state: CharacterState) -> FrameAnimation {
        let (frames, animation_type) = self.states[state.index()];
        FrameAnimation::new(animation_type, frames)
    }
}

/// One layer's clock, which is reset whenever the state it is drawn on changes.
#[derive(Clone, Debug)]
struct LayerRun {
    frames: Option<FrameAnimation>,
    anchor: usize,
    since_move: Duration,
}

/// One player's character: which one was dealt, which way it faces, and what it is doing.
#[derive(Clone, Debug)]
pub struct CharacterAnimation {
    meta: Option<CharacterMeta>,
    /// which of the theme's cast this player was dealt
    character: usize,
    /// drawn flipped, so that in a two player match each faces the other player's board
    mirrored: bool,
    state: CharacterState,
    frames: Option<FrameAnimation>,
    /// how long the current state has been held, for `MIN_DWELL`
    held: Duration,
    /// time left on the linger that keeps `winning` or `losing` up after its cause has gone
    linger: Duration,
    /// a won match holds `winning` for the rest of the match rather than lingering
    latched: bool,
    /// one per [`LayerMeta`], in the same order
    layers: Vec<LayerRun>,
    /// one per [`EmitterMeta`]: what it has banked towards its next firing
    fired: Vec<Duration>,
    /// a `Danger` emitter's fractional budget, and the portrait frame an `OnFrame` last saw
    budget: Vec<f64>,
    last_frame: Vec<usize>,
    particles: Vec<CharacterParticle>,
    /// how badly this player's board is going, fed every frame by the match screen
    danger: f64,
    /// what the deterministic spread has got to
    tick: u64,
}

impl Default for CharacterAnimation {
    fn default() -> Self {
        Self::new()
    }
}

impl CharacterAnimation {
    pub fn new() -> Self {
        Self {
            meta: None,
            character: 0,
            mirrored: false,
            state: CharacterState::Idle,
            frames: None,
            held: Duration::ZERO,
            linger: Duration::ZERO,
            latched: false,
            layers: vec![],
            fired: vec![],
            budget: vec![],
            last_frame: vec![],
            particles: vec![],
            danger: 0.0,
            tick: 0,
        }
    }

    /// Deal this player a character. Until this is called nothing is drawn at all, which is
    /// what a theme with no cast wants.
    pub fn deal(&mut self, meta: CharacterMeta, character: usize, mirrored: bool) {
        self.frames = Some(meta.animation(CharacterState::Idle));
        self.layers = meta.layers.iter().map(|_| LayerRun::default()).collect();
        self.fired = vec![Duration::ZERO; meta.emitters.len()];
        self.budget = vec![0.0; meta.emitters.len()];
        self.last_frame = vec![usize::MAX; meta.emitters.len()];
        self.meta = Some(meta);
        self.character = character;
        self.mirrored = mirrored;
        self.state = CharacterState::Idle;
        self.held = Duration::ZERO;
        self.linger = Duration::ZERO;
        self.latched = false;
        self.particles.clear();
        self.danger = 0.0;
        self.start_layers();
    }

    pub fn character(&self) -> Option<usize> {
        self.meta.as_ref().map(|_| self.character)
    }

    pub fn mirrored(&self) -> bool {
        self.mirrored
    }

    pub fn state(&self) -> CharacterState {
        self.state
    }

    pub fn frame(&self) -> usize {
        self.frames.map(|f| f.frame()).unwrap_or(0)
    }

    /// Every layer that is drawn in this state, in the [`LayerMeta`] order, as
    /// (which layer, which frame, where in the box). A layer absent from the state is simply
    /// not in the list.
    pub fn layers(&self) -> Vec<(usize, usize, (i32, i32))> {
        let Some(meta) = self.meta.as_ref() else {
            return vec![];
        };
        let mut out = vec![];
        for (index, (layer, run)) in meta.layers.iter().zip(self.layers.iter()).enumerate() {
            let Some(frames) = run.frames.as_ref() else {
                continue;
            };
            let anchor = layer.anchors[run.anchor.min(layer.anchors.len() - 1)];
            out.push((index, frames.frame(), anchor));
        }
        out
    }

    pub fn particles(&self) -> &[CharacterParticle] {
        &self.particles
    }

    pub fn update(&mut self, delta: Duration) {
        if let Some(frames) = self.frames.as_mut() {
            frames.update(delta);
        }
        self.held += delta;
        self.linger = self.linger.saturating_sub(delta);
        self.update_layers(delta);
        // the particles already in the air move first, so one born this tick starts where it
        // was thrown rather than a tick down its flight
        self.update_particles(delta);
        self.update_emitters(delta);
    }

    fn update_layers(&mut self, delta: Duration) {
        let Some(meta) = self.meta.as_ref() else {
            return;
        };
        for (layer, run) in meta.layers.iter().zip(self.layers.iter_mut()) {
            let Some(frames) = run.frames.as_mut() else {
                continue;
            };
            frames.update(delta);
            let Some(every) = layer.wander else {
                continue;
            };
            run.since_move += delta;
            if run.since_move >= every && layer.anchors.len() > 1 {
                run.since_move = Duration::ZERO;
                self.tick = self.tick.wrapping_add(1);
                let next = (hashed(self.tick) * layer.anchors.len() as f64) as usize;
                // never twice in the same slot: a flicker that does not move is not a flicker
                run.anchor = if next == run.anchor {
                    (next + 1) % layer.anchors.len()
                } else {
                    next
                };
            }
        }
    }

    fn update_emitters(&mut self, delta: Duration) {
        let Some(meta) = self.meta.as_ref() else {
            return;
        };
        let state = self.state.index();
        let portrait = self.frames.map(|f| f.frame()).unwrap_or(0);
        let danger = self.danger;
        let mut fire: Vec<usize> = vec![];
        let mut trickle: Vec<usize> = vec![];
        for (index, emitter) in meta.emitters.iter().enumerate() {
            let Some(trigger) = emitter.triggers[state] else {
                // a state it is silent in resets its clock, so entering the state again
                // starts a whole period rather than firing at once
                self.fired[index] = Duration::ZERO;
                self.last_frame[index] = usize::MAX;
                continue;
            };
            match trigger {
                EmitterTrigger::Every(period) => {
                    self.fired[index] += delta;
                    if self.fired[index] >= period {
                        self.fired[index] = Duration::ZERO;
                        fire.push(index);
                    }
                }
                EmitterTrigger::OnFrame(frames) => {
                    // edge triggered: a strip that holds the frame fires once, not every tick
                    if frames.contains(&portrait) && self.last_frame[index] != portrait {
                        fire.push(index);
                    }
                    self.last_frame[index] = portrait;
                }
                EmitterTrigger::Danger { above, per_second } => {
                    let over = ((danger - above) / (1.0 - above).max(f64::EPSILON)).clamp(0.0, 1.0);
                    self.budget[index] += over * per_second * delta.as_secs_f64();
                    while self.budget[index] >= 1.0 {
                        self.budget[index] -= 1.0;
                        trickle.push(index);
                    }
                }
            }
        }
        for index in fire {
            self.burst(index);
        }
        for index in trickle {
            self.trickle(index);
        }
    }

    /// A whole burst: one particle down every direction of every source at once.
    fn burst(&mut self, index: usize) {
        let Some(meta) = self.meta.as_ref() else {
            return;
        };
        let emitter = &meta.emitters[index];
        let mut born = vec![];
        for source in emitter.sources {
            for direction in source.directions {
                born.push(new_particle(emitter, index, source, *direction));
            }
        }
        self.push(born);
    }

    /// One particle, from a source and direction spread over the whole set - which is what a
    /// rate rather than a burst means, and what keeps one or two drops in the air at a time.
    fn trickle(&mut self, index: usize) {
        let Some(meta) = self.meta.as_ref() else {
            return;
        };
        let emitter = &meta.emitters[index];
        let choices: usize = emitter.sources.iter().map(|s| s.directions.len()).sum();
        if choices == 0 {
            return;
        }
        self.tick = self.tick.wrapping_add(1);
        let mut pick = (hashed(self.tick) * choices as f64) as usize;
        for source in emitter.sources {
            if pick < source.directions.len() {
                let born = new_particle(emitter, index, source, source.directions[pick]);
                self.push(vec![born]);
                return;
            }
            pick -= source.directions.len();
        }
    }

    fn push(&mut self, born: Vec<CharacterParticle>) {
        self.particles.extend(born);
        if self.particles.len() > MAX_PARTICLES {
            // the oldest are the faintest, which is the right end to lose from
            self.particles.drain(..self.particles.len() - MAX_PARTICLES);
        }
    }

    fn update_particles(&mut self, delta: Duration) {
        if self.particles.is_empty() {
            return;
        }
        let step = delta.as_secs_f64();
        for particle in self.particles.iter_mut() {
            particle.x += particle.velocity.0 * step;
            particle.y += particle.velocity.1 * step;
            particle.elapsed += delta;
            if particle.frames > 1 && particle.fps > 0 {
                let ticks = (particle.elapsed.as_secs_f64() * particle.fps as f64) as usize;
                particle.frame = ticks % particle.frames;
            }
        }
        self.particles.retain(|p| p.elapsed < p.life);
    }

    /// Start every layer on the row the current state gives it, or take it off screen.
    fn start_layers(&mut self) {
        let Some(meta) = self.meta.as_ref() else {
            return;
        };
        let state = self.state.index();
        for (layer, run) in meta.layers.iter().zip(self.layers.iter_mut()) {
            let (frames, animation) = layer.states[state];
            run.frames = (frames > 0).then(|| FrameAnimation::new(animation, frames));
            run.since_move = Duration::ZERO;
            run.anchor = 0;
        }
    }

    /// A clear that chained. One pop is not a reaction worth pulling a face for - it is most
    /// clears, several a minute, and it sends no attack either - so this is only called for a
    /// clear the game itself called a combo.
    pub fn chained(&mut self) {
        self.linger = LINGER;
        self.enter(CharacterState::Winning);
    }

    /// The match was won. Terminal: the face is held for the rest of it rather than lingering.
    pub fn victory(&mut self) {
        self.enter(CharacterState::Winning);
        self.latched = true;
    }

    /// Buried. Terminal, and nothing leaves it.
    pub fn game_over(&mut self) {
        self.enter(CharacterState::Defeat);
    }

    /// The two per-frame numbers, which have no event between them: how high this player's
    /// stack is, and whether anything is waiting in their tray.
    ///
    /// `winning` beats `losing` while a match is running - a player who chains while buried is
    /// answering the nuisance, and cancelling it outright, and the face that says so is the
    /// right one. `losing` resumes when the hold runs out if its cause is still true.
    pub fn danger(&mut self, danger: f64, pending: bool) {
        // the dial is kept whatever the state, since the sweat is graded and is not the
        // `losing` face - measured: Grounder holds `losing` for a whole clip and sweats nothing
        self.danger = danger;
        if self.meta.is_none() || self.latched || self.state == CharacterState::Defeat {
            return;
        }
        let losing = pending
            || danger >= DANGER_ENTER
            || (self.state == CharacterState::Losing && danger > DANGER_LEAVE);
        match self.state {
            CharacterState::Winning => {
                if self.linger.is_zero() {
                    self.enter(if losing {
                        CharacterState::Losing
                    } else {
                        CharacterState::Idle
                    });
                }
            }
            CharacterState::Losing if !losing => {
                if self.linger.is_zero() {
                    self.enter(CharacterState::Idle);
                }
            }
            CharacterState::Losing => self.linger = LINGER,
            CharacterState::Idle if losing => {
                self.linger = LINGER;
                self.enter(CharacterState::Losing);
            }
            _ => {}
        }
    }

    /// A state ends by cutting its row off wherever it has got to, on one frame and with no
    /// blend, which is what the original does - measured on Skweel, who drops back to idle
    /// mid-sneeze. So the new row starts from its own first frame.
    fn enter(&mut self, state: CharacterState) {
        let Some(meta) = self.meta.as_ref() else {
            return;
        };
        if self.state == state {
            return;
        }
        // A game over is the one thing nothing refuses - not a state that has only just been
        // entered, not a won match, and not another game over.
        if state != CharacterState::Defeat {
            // being buried is terminal, and so is winning the match
            if self.state == CharacterState::Defeat || self.latched {
                return;
            }
            // ... and a state holds its minimum, so a face cannot be entered and left inside
            // one animation cycle
            if self.held < MIN_DWELL && self.state != CharacterState::Idle {
                return;
            }
        } else if self.state == CharacterState::Defeat {
            return;
        }
        self.state = state;
        self.frames = Some(meta.animation(state));
        self.held = Duration::ZERO;
        // a layer's clock belongs to the row, so it is cut off with it and starts again
        self.start_layers();
    }
}

impl Default for LayerRun {
    fn default() -> Self {
        Self {
            frames: None,
            anchor: 0,
            since_move: Duration::ZERO,
        }
    }
}

fn new_particle(
    emitter: &EmitterMeta,
    index: usize,
    source: &EmitterSource,
    direction: (f64, f64),
) -> CharacterParticle {
    // the captures were measured in box pixels an axis per 60 Hz frame, so that is the unit
    // the tables carry and this is the one place it becomes pixels a second
    let speed = emitter.speed * 60.0;
    CharacterParticle {
        x: source.at.0,
        y: source.at.1,
        velocity: (direction.0 * speed, direction.1 * speed),
        elapsed: Duration::ZERO,
        life: emitter.life,
        fade_last: emitter.fade_last,
        emitter: index,
        frame: 0,
        frames: emitter.frames,
        fps: emitter.fps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> CharacterMeta {
        CharacterMeta::plain([
            (2, FrameAnimationType::Static),
            (2, FrameAnimationType::Static),
            (2, FrameAnimationType::Static),
            (1, FrameAnimationType::Static),
        ])
    }

    fn dealt() -> CharacterAnimation {
        let mut c = CharacterAnimation::new();
        c.deal(meta(), 0, false);
        c
    }

    /// past `MIN_DWELL`, so the next change is allowed
    fn settle(c: &mut CharacterAnimation) {
        c.update(MIN_DWELL + Duration::from_millis(10));
    }

    #[test]
    fn a_character_that_was_never_dealt_draws_nothing() {
        let c = CharacterAnimation::new();
        assert_eq!(c.character(), None);
        // ... and takes its triggers without panicking
        let mut c = c;
        c.danger(1.0, true);
        c.chained();
        assert_eq!(c.state(), CharacterState::Idle);
    }

    #[test]
    fn a_high_stack_is_losing_and_an_empty_board_is_not() {
        let mut c = dealt();
        c.danger(0.9, false);
        assert_eq!(c.state(), CharacterState::Losing);
        settle(&mut c);
        c.update(LINGER);
        c.danger(0.0, false);
        assert_eq!(c.state(), CharacterState::Idle);
    }

    #[test]
    fn a_tray_with_anything_in_it_is_losing_however_low_the_stack() {
        let mut c = dealt();
        c.danger(0.0, true);
        assert_eq!(c.state(), CharacterState::Losing);
    }

    /// The one this is all for: a stack sitting exactly on the threshold must not strobe as
    /// each piece locks. Between the two thresholds the state is whatever it already was.
    #[test]
    fn the_danger_thresholds_do_not_flip_flop() {
        let mut c = dealt();
        let between = (DANGER_ENTER + DANGER_LEAVE) / 2.0;
        for _ in 0..50 {
            settle(&mut c);
            c.danger(between, false);
            assert_eq!(
                c.state(),
                CharacterState::Idle,
                "entered below the enter point"
            );
        }
        c.danger(DANGER_ENTER + 0.01, false);
        assert_eq!(c.state(), CharacterState::Losing);
        for _ in 0..50 {
            settle(&mut c);
            c.update(LINGER);
            c.danger(between, false);
            assert_eq!(
                c.state(),
                CharacterState::Losing,
                "left above the leave point"
            );
        }
        settle(&mut c);
        c.update(LINGER);
        c.danger(DANGER_LEAVE - 0.01, false);
        assert_eq!(c.state(), CharacterState::Idle);
    }

    #[test]
    fn a_state_holds_for_a_minimum_however_the_board_looks() {
        let mut c = dealt();
        c.danger(0.9, false);
        assert_eq!(c.state(), CharacterState::Losing);
        // the board empties immediately; the face does not follow it that fast
        c.update(Duration::from_millis(50));
        c.danger(0.0, false);
        assert_eq!(c.state(), CharacterState::Losing);
    }

    /// A player who chains while buried is answering the nuisance, and the face that says so
    /// is the winning one.
    #[test]
    fn winning_beats_losing_and_losing_resumes_after_it() {
        let mut c = dealt();
        c.danger(0.9, true);
        assert_eq!(c.state(), CharacterState::Losing);
        settle(&mut c);
        c.chained();
        assert_eq!(c.state(), CharacterState::Winning);
        // ... and while the linger runs it stays winning even though the board is still bad
        c.danger(0.9, true);
        assert_eq!(c.state(), CharacterState::Winning);
        settle(&mut c);
        c.update(LINGER);
        c.danger(0.9, true);
        assert_eq!(c.state(), CharacterState::Losing);
    }

    /// Every step of a chain fires its own clear, so a nine chain holds the face for nine
    /// steps and a bit rather than restarting it.
    #[test]
    fn each_step_of_a_chain_refreshes_the_linger_rather_than_restarting_the_face() {
        let mut c = dealt();
        c.chained();
        let frames_in = |c: &CharacterAnimation| c.frame();
        settle(&mut c);
        let before = frames_in(&c);
        c.chained();
        assert_eq!(c.state(), CharacterState::Winning);
        assert_eq!(frames_in(&c), before, "the row restarted mid-chain");
    }

    #[test]
    fn defeat_is_terminal() {
        let mut c = dealt();
        c.game_over();
        assert_eq!(c.state(), CharacterState::Defeat);
        settle(&mut c);
        c.danger(0.0, false);
        c.chained();
        assert_eq!(c.state(), CharacterState::Defeat);
    }

    /// A won match holds the face for the rest of it rather than lingering back to idle.
    #[test]
    fn victory_is_held_and_not_lingered() {
        let mut c = dealt();
        c.victory();
        assert_eq!(c.state(), CharacterState::Winning);
        for _ in 0..20 {
            settle(&mut c);
            c.update(LINGER);
            c.danger(0.0, false);
        }
        assert_eq!(c.state(), CharacterState::Winning);
    }

    fn with_extras() -> CharacterMeta {
        CharacterMeta {
            states: [
                (2, FrameAnimationType::Static),
                (4, FrameAnimationType::Linear { fps: 10 }),
                (2, FrameAnimationType::Static),
                (1, FrameAnimationType::Static),
            ],
            // drawn on idle and losing, absent on the other two
            layers: vec![LayerMeta {
                states: [
                    (3, FrameAnimationType::Linear { fps: 10 }),
                    (0, FrameAnimationType::Static),
                    (2, FrameAnimationType::Linear { fps: 10 }),
                    (0, FrameAnimationType::Static),
                ],
                anchors: &[(0, 0), (8, 0), (16, 0), (24, 0)],
                wander: Some(Duration::from_millis(100)),
            }],
            emitters: vec![
                // a burst on a clock while winning
                EmitterMeta {
                    triggers: [
                        None,
                        Some(EmitterTrigger::Every(Duration::from_millis(500))),
                        None,
                        None,
                    ],
                    sources: &[EmitterSource {
                        at: (10.0, 10.0),
                        directions: &[(-1.0, -1.0), (1.0, -1.0)],
                    }],
                    speed: 5.0,
                    life: Duration::from_millis(200),
                    fade_last: 0.5,
                    frames: 2,
                    fps: 10,
                },
                // and the cast-wide dial, on in every state
                EmitterMeta {
                    triggers: [Some(SWEAT); 4],
                    sources: &[EmitterSource {
                        at: (10.0, 10.0),
                        directions: &[(-1.0, -1.0)],
                    }],
                    speed: 1.0,
                    life: Duration::from_secs(1),
                    fade_last: 0.5,
                    frames: 1,
                    fps: 0,
                },
            ],
        }
    }

    const SWEAT: EmitterTrigger = EmitterTrigger::Danger {
        above: 0.5,
        per_second: 10.0,
    };

    fn with_extras_dealt() -> CharacterAnimation {
        let mut c = CharacterAnimation::new();
        c.deal(with_extras(), 0, false);
        c
    }

    /// A layer belongs to a *row*, not to the character, so a state it is absent from draws
    /// nothing at all - which is Coconuts' coin on idle and Sir Ffuzzy-Logik's eyes off every
    /// row but his losing one.
    #[test]
    fn a_layer_is_drawn_only_on_the_rows_that_declare_frames() {
        let mut c = with_extras_dealt();
        assert_eq!(c.layers().len(), 1, "the idle layer was not drawn");
        settle(&mut c);
        c.chained();
        assert_eq!(c.state(), CharacterState::Winning);
        assert!(c.layers().is_empty(), "a layer with no winning frames drew");
        settle(&mut c);
        c.update(LINGER);
        c.danger(0.9, false);
        assert_eq!(c.state(), CharacterState::Losing);
        assert_eq!(c.layers().len(), 1, "the losing layer was not drawn");
    }

    /// It does not travel: it appears at one of a handful of slots, flashes and goes. Two
    /// flashes in the same slot read as no flash at all, so it never repeats one.
    #[test]
    fn a_wandering_layer_moves_and_never_twice_to_the_same_slot() {
        let mut c = with_extras_dealt();
        let mut seen = vec![c.layers()[0].2];
        for _ in 0..30 {
            c.update(Duration::from_millis(110));
            let at = c.layers()[0].2;
            assert_ne!(
                at,
                *seen.last().unwrap(),
                "it flashed in the same slot twice"
            );
            seen.push(at);
        }
        let mut distinct = seen.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert!(distinct.len() >= 3, "it only ever used {distinct:?}");
    }

    /// A burst is one particle down every direction of every source, and it comes round on
    /// its own clock for as long as the row is up - Frankly's sparks, six at a time.
    #[test]
    fn a_burst_fires_on_its_clock_and_only_in_the_state_that_asks_for_it() {
        let mut c = with_extras_dealt();
        c.update(Duration::from_millis(600));
        assert!(
            c.particles().is_empty(),
            "an emitter fired on a row it is silent on"
        );
        c.chained();
        assert_eq!(c.state(), CharacterState::Winning);
        c.update(Duration::from_millis(400));
        assert!(
            c.particles().is_empty(),
            "it fired before its period was up"
        );
        c.update(Duration::from_millis(150));
        assert_eq!(
            c.particles().len(),
            2,
            "a burst is one down every direction"
        );
    }

    /// The sweat is not the losing face: it is graded, and comes on later. Measured - Grounder
    /// holds the losing face for a whole clip with a low stack and sweats nothing.
    #[test]
    fn the_dial_throws_nothing_below_its_threshold_and_more_the_worse_it_gets() {
        let mut c = with_extras_dealt();
        for _ in 0..30 {
            c.danger(0.4, false);
            c.update(Duration::from_millis(100));
        }
        assert!(c.particles().is_empty(), "it sweated on a healthy board");
        let mut count = |danger: f64| {
            let mut c = with_extras_dealt();
            for _ in 0..10 {
                c.danger(danger, false);
                c.update(Duration::from_millis(100));
            }
            c.particles().len()
        };
        let some = count(0.75);
        let lots = count(1.0);
        assert!(some > 0, "a bad board sweated nothing");
        assert!(
            lots > some,
            "a worse board sweated no more: {lots} vs {some}"
        );
    }

    /// A particle is thrown in **box coordinates** so that one flip serves the whole spray;
    /// it leaves the box, which is the whole reason it is drawn on the window.
    #[test]
    fn a_particle_travels_out_of_the_box_and_then_expires() {
        let mut c = with_extras_dealt();
        settle(&mut c);
        c.chained();
        let step = Duration::from_millis(10);
        while c.particles().is_empty() {
            c.update(step);
        }
        let start = c.particles()[0];
        assert_eq!(
            (start.x, start.y),
            (10.0, 10.0),
            "it did not start at its source"
        );
        for _ in 0..15 {
            c.update(step);
        }
        let flying = c.particles()[0];
        assert!(flying.x < 0.0 && flying.y < 0.0, "it did not leave the box");
        assert!(flying.alpha() < 255, "it is not fading out");
        for _ in 0..6 {
            c.update(step);
        }
        assert!(c.particles().len() < 2, "the first one outlived its life");
    }

    /// Humpty fires from his antenna tips at the moment they are drawn in, and his winning row
    /// runs that gesture *twice* - so the trigger is a set of frames, and each fires once
    /// however long the strip holds it.
    #[test]
    fn a_frame_trigger_fires_once_an_arrival_and_once_per_named_frame() {
        let mut meta = with_extras();
        meta.emitters[0].triggers[1] = Some(EmitterTrigger::OnFrame(&[1, 3]));
        // longer lived than a whole pass, so the bursts pile up and can be counted
        meta.emitters[0].life = Duration::from_secs(2);
        let mut c = CharacterAnimation::new();
        c.deal(meta, 0, false);
        c.chained();
        // the winning row is four frames at 10 fps; walk it once
        let mut fired = 0;
        for _ in 0..40 {
            let before = c.particles().len();
            c.update(Duration::from_millis(10));
            if c.particles().len() > before {
                fired += 1;
            }
        }
        assert_eq!(fired, 2, "one burst per named frame of a single pass");
    }

    /// `LinearWithPause` holds the *last* frame of its strip, which is why a row is cut with
    /// its action first and its rest last. If that ever stops being true every character in
    /// the compendium rests on the wrong pose, so it is pinned here rather than assumed.
    #[test]
    fn a_paused_row_rests_on_the_last_frame_of_its_strip() {
        let mut animation = FrameAnimation::new(
            FrameAnimationType::LinearWithPause {
                fps: 10,
                pause_for: Duration::from_secs(2),
                resume_from_frame: 0,
            },
            4,
        );
        // run the action out
        for _ in 0..4 {
            animation.update(Duration::from_millis(100));
        }
        assert_eq!(
            animation.frame(),
            3,
            "the action did not run to the rest frame"
        );
        // ... and it stays there for the pause
        animation.update(Duration::from_millis(900));
        assert_eq!(animation.frame(), 3, "the rest frame was not held");
    }
}
