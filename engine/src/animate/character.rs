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

/// One 60 Hz tick, which is the unit a [`RoutineFrame`]'s hold is measured in - the same unit
/// an emitter's speed is in, and for the same reason: it is what the captures were read at.
const TICK: Duration = Duration::from_nanos(16_666_667);

/// How many particles one character may have in the air.
///
/// A burst is six and they live a fifth of a second, so this is never approached by a
/// character's own emitters; it is the sweat, which fires on a dial that a buried board holds
/// wide open, that needs a ceiling at all.
const MAX_PARTICLES: usize = 64;

/// Where a routine has to start from for the whole of it to stay in the box.
///
/// A routine carries its travel - the spin that goes twenty pixels left is the same table as
/// the one that goes twenty right, played backwards - so a character standing against the left
/// post and dealt the left-hand spin would walk out of the arch. Fitting it at the deal is
/// invisible: he simply starts a little further in. Clamping every *frame* instead would stall
/// him against the wall in the middle of a spin, which is not what the game does.
/// Whether a routine may be played from here.
///
/// The whole of it: a routine is started from exactly where the last one left the character,
/// and if its own window does not contain that, it is not dealt. **Nothing is shifted.**
/// Shifting a routine to make it fit is what put a jump between the end of one and the start
/// of the next, which is the one thing a run of animations must not do.
fn fits(home: i32, way: &RoutineWay) -> bool {
    home >= way.origins.0 && home <= way.origins.1
}

/// A well spread deterministic sequence, the golden ratio walk the nuisance stagger uses.
///
/// Deterministic rather than random on purpose: this is on the render path, and the same
/// character in the same state should look the same twice. It only has to *spread*, and a
/// low-discrepancy sequence spreads better than an RNG does over a handful of draws.
fn hashed(n: u64) -> f64 {
    (n as f64 * 0.618_033_988_749_894_9).fract()
}

/// One frame of a [`Routine`]: which pose, how long it is held, and where it goes in the box.
///
/// A pose of `None` draws nothing at all, which is a character who has **left the box** - Kirby
/// inflates and floats clean out of the arch for a second and a half in one routine, and for
/// two thirds of one in four others. A strip cannot say that and does not have to: a mugshot
/// never leaves its box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoutineFrame {
    pub pose: Option<usize>,
    /// held for this many 60 Hz ticks. A capture is 30 fps against the game's 60, so these are
    /// measured to two ticks and no better - which is why every one of Kirby's is even.
    pub ticks: u32,
    /// the pose's top left, **relative to the frame the routine opens on** rather than to the
    /// box: the order routines are dealt in is not the order a capture happened to play them,
    /// so a routine that walks nineteen pixels right has to start where the last one finished.
    pub at: (i32, i32),
}

/// A run a character plays through once and then rests on.
pub type Routine = &'static [RoutineFrame];

/// One way round a routine may be played.
///
/// A move with a direction has two of these and the direction is chosen from where the
/// character is standing, which is the whole point: a Kirby against the right post walks *left*
/// rather than sliding left and walking back.
#[derive(Clone, Copy, Debug)]
pub struct RoutineWay {
    pub frames: Routine,
    /// the inclusive range of origins this way may be started from with its **whole drawn
    /// path** staying inside the box, computed by the cutter from its own poses' widths
    pub origins: (i32, i32),
    /// when the character may be translated into position, as (tick it starts, ticks it takes).
    ///
    /// The cutter puts it on the routine's **first airborne stretch** wherever there is one, so
    /// a routine that opens by hopping is walked into *in the air*. Sliding along the floor to
    /// get somewhere is the one thing that reads as a glitch, and `ricochet` opens by lying
    /// down for two thirds of a second - exactly where a translation from tick zero would land.
    pub glide: (u32, u32),
}

/// One routine a character may be dealt: a **kind**, and how much of a thing it is.
#[derive(Clone, Copy, Debug)]
pub struct RoutineChoice {
    /// 0 for a kind that never leaves the floor, 1 for the one that goes furthest up.
    ///
    /// **Vertical reach**, because that is what reads as effort. It decides the order a bag
    /// comes out in and never whether something may come out at all.
    pub intensity: f64,
    /// the ways round it may be played, in no particular order
    pub ways: &'static [RoutineWay],
}

/// A character whose rows are **routines** rather than one strip apiece.
///
/// The two art models sit side by side because they are answers to different games. Mean Bean
/// Machine draws a face in a fixed box at a constant rate, which is a strip; Kirby's Avalanche
/// stands a whole little character in the arch who walks about it, changes size from frame to
/// frame - 14x14 standing, 14x7 in the pancake he lands in, 6x14 when he squashes thin against
/// a wall - holds one pose for half a second and the next for two ticks, and climbs the centre
/// column right up past `STAGE`. None of that is a strip at an fps.
///
/// **And it is not chosen by state.** A mugshot has four rows and pulls one face when you chain
/// and another when you are nearly buried. This has one pool, and which of it gets dealt is
/// decided by how much is *happening* - a high stack and a big chain are both intense, and the
/// character does bigger things either way. Only being buried is a state, because that one is
/// terminal.
#[derive(Clone, Copy, Debug)]
pub struct RoutineMeta {
    /// everything the character may be dealt, and how much each moves
    pub choices: &'static [RoutineChoice],
    /// played *between* routines rather than as one of them, half the time: the blink, which is
    /// a tic rather than a thing he does, and reads as a loop if it is dealt like one
    pub filler: Option<RoutineWay>,
    /// what a buried character does, held - the one routine a state still picks
    pub defeat: Routine,
    /// how long the last frame is held before another routine is dealt
    pub rest: Duration,
    /// where a routine's opening frame goes in the box, which every routine shares because
    /// every one of them opens on the character standing
    pub home: (i32, i32),
    /// how far either way `home.0` may be walked, since routines carry a net displacement and
    /// the character would otherwise walk out of the box over a match
    pub wander: (i32, i32),
    /// how much a routine's pace is varied when it is dealt, as a fraction either way - so the
    /// same spin is not the same spin twice. Zero plays every routine at its measured pace.
    pub speed_spread: f64,
    /// whether a routine may be walked into at all. With this off, a character not standing in
    /// a way's window plays it from where he is and the arch clips whatever hangs out.
    pub approach: bool,
}

/// What a character is drawing this tick.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CharacterFrame {
    /// a frame of a strip, filling the box - the mugshot path
    Whole(usize),
    /// a pose at its own size, at a place in the box - the routine path
    Placed(usize, (i32, i32)),
    /// nothing: a routine frame with no pose, which is a character out of the box
    Hidden,
}

/// Everything a theme declares about one character: how each of its four rows plays.
///
/// Frame counts are the strip's own, in play order - a row is cut action first and rest last so
/// that [`FrameAnimationType::LinearWithPause`], which holds the *last* frame, holds the pose
/// the character rests in.
#[derive(Clone, Debug)]
pub struct CharacterMeta {
    /// (frames, how it plays) per [`CharacterState`], in `CharacterState::ALL` order.
    /// Ignored while `routines` is set, which plays its own frames.
    pub states: [(usize, FrameAnimationType); 4],
    /// routines instead of strips: `None` on a mugshot cast, which is every character here
    /// but Kirby
    pub routines: Option<RoutineMeta>,
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
            routines: None,
            layers: vec![],
            emitters: vec![],
        }
    }

    fn animation(&self, state: CharacterState) -> FrameAnimation {
        let (frames, animation_type) = self.states[state.index()];
        FrameAnimation::new(animation_type, frames)
    }
}

/// Where a routine has got to.
///
/// `rested` is `None` while the routine is playing and `Some` once it has run out, which is
/// what makes the character stand for a moment before another is dealt rather than snapping
/// straight into it - measured at two seconds, and the whole of what a state's playlist looks
/// like when nothing on the board interrupts it.
#[derive(Clone, Copy, Debug)]
struct RoutineRun {
    /// an index into `RoutineMeta::choices`, or `usize::MAX` for the filler and the defeat
    /// routine, which are not dealt from the pool
    routine: usize,
    frames: Routine,
    frame: usize,
    elapsed: Duration,
    rested: Option<Duration>,
    /// where this routine's frame 0 *ends up*, which is what its places are relative to once
    /// the character has walked into it. Explicit rather than read off `home`, because the filler replaces the run
    /// and used to take the routine's displacement with it - a walk that carried him twenty
    /// pixels put him back where he started the moment he blinked.
    origin: i32,
    /// what the routine carries him by, kept here so a filler cannot lose it
    net: i32,
    /// where the character was standing when this was dealt, and the stretch of the routine
    /// he is translated to `origin` over. Equal for one he was already in position for.
    from: i32,
    glide: (Duration, Duration),
    /// how long this run has been going, for the glide
    age: Duration,
    /// what this dealing plays at, around 1.0 - see `RoutineMeta::speed_spread`
    speed: f64,
    /// the filler goes back to resting when it ends rather than dealing another
    filler: bool,
    /// whether the rest this run settles into has already had its coin tossed
    blinked: bool,
}

impl RoutineRun {
    /// Where this run's frame 0 sits *now* - which is the origin once the character has walked
    /// into it, and a point on the way there before that.
    fn place(&self) -> i32 {
        let (at, span) = self.glide;
        if self.age <= at || span.is_zero() {
            return if self.age <= at {
                self.from
            } else {
                self.origin
            };
        }
        if self.age >= at + span {
            return self.origin;
        }
        let through = (self.age - at).as_secs_f64() / span.as_secs_f64();
        self.from + ((self.origin - self.from) as f64 * through).round() as i32
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
    /// where the dealt routine has got to; `None` on a character with strips
    run: Option<RoutineRun>,
    /// how far along the box the character has walked, since a routine carries a net
    /// displacement and the next one starts wherever this one finished
    home: i32,
    /// what has not been dealt yet this cycle, so a game shows most of what there is rather
    /// than the same three things - the seven bag's idea, and for the same reason
    bag: Vec<usize>,
    /// how much a chain has just excited the character, which decays away
    excitement: f64,
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
    /// how many routines have been dealt, which is the only way to tell one dealing of a
    /// routine from the next dealing of the same one
    deals: u64,
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
            run: None,
            home: 0,
            bag: vec![],
            excitement: 0.0,
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
            deals: 0,
        }
    }

    /// Deal this player a character. Until this is called nothing is drawn at all, which is
    /// what a theme with no cast wants.
    pub fn deal(&mut self, meta: CharacterMeta, character: usize, mirrored: bool) {
        self.frames = Some(meta.animation(CharacterState::Idle));
        self.home = meta.routines.map(|r| r.home.0).unwrap_or(0);
        self.run = None;
        self.bag.clear();
        self.excitement = 0.0;
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
        self.start_routine(None);
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
        match self.drawing() {
            CharacterFrame::Whole(frame) | CharacterFrame::Placed(frame, _) => frame,
            CharacterFrame::Hidden => 0,
        }
    }

    /// What is drawn this tick: a strip's frame filling the box, a pose at a place in it, or
    /// nothing at all.
    pub fn drawing(&self) -> CharacterFrame {
        let Some(meta) = self.meta.as_ref() else {
            return CharacterFrame::Whole(0);
        };
        let Some(routines) = meta.routines else {
            return CharacterFrame::Whole(self.frames.map(|f| f.frame()).unwrap_or(0));
        };
        let Some(frame) = self.routine_frame(&routines) else {
            return CharacterFrame::Hidden;
        };
        match frame.pose {
            // the **run's** own place and not `home`: they are the same once a routine is
            // under way and they are not while he is walking into it, nor for the filler,
            // which stands where the routine before it finished rather than where it started.
            Some(pose) => CharacterFrame::Placed(
                pose,
                (
                    self.run.map(|r| r.place()).unwrap_or(self.home) + frame.at.0,
                    routines.home.1 + frame.at.1,
                ),
            ),
            None => CharacterFrame::Hidden,
        }
    }

    fn routine_frame(&self, _routines: &RoutineMeta) -> Option<RoutineFrame> {
        let run = self.run.as_ref()?;
        run.frames
            .get(run.frame.min(run.frames.len().saturating_sub(1)))
            .copied()
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
        self.update_routine(delta);
        self.update_layers(delta);
        // the particles already in the air move first, so one born this tick starts where it
        // was thrown rather than a tick down its flight
        self.update_particles(delta);
        self.update_emitters(delta);
    }

    /// One tick of the dealt routine, which is nothing at all on a character with strips.
    ///
    /// A frame is held for its own count of ticks rather than a shared fps, so this walks the
    /// remainder forward: a routine whose first frame is held ten ticks and whose next is held
    /// two is one line of a table here and would be ten frames of a strip.
    fn update_routine(&mut self, delta: Duration) {
        let Some(routines) = self.meta.as_ref().and_then(|m| m.routines) else {
            return;
        };
        // a chain's excitement fades over about three seconds, which is a routine and a rest
        self.excitement = (self.excitement - delta.as_secs_f64() / 3.0).max(0.0);
        let Some(mut run) = self.run else {
            self.start_routine(None);
            return;
        };
        if let Some(rested) = run.rested.as_mut() {
            *rested += delta;
            // half way through the rest, half the time, he blinks - which is a tic between
            // routines and not one of them
            if !run.blinked && *rested >= routines.rest / 2 {
                run.blinked = true;
                self.tick = self.tick.wrapping_add(1);
                let at = run.origin + run.net;
                if let (Some(filler), true) = (
                    routines.filler.filter(|f| fits(at, f)),
                    hashed(self.tick) < 0.5,
                ) {
                    // the blink stands where the routine left him, not where it started
                    self.run = Some(RoutineRun {
                        routine: usize::MAX,
                        frames: filler.frames,
                        frame: 0,
                        elapsed: Duration::ZERO,
                        rested: None,
                        origin: at,
                        net: 0,
                        from: at,
                        glide: (Duration::ZERO, Duration::ZERO),
                        age: Duration::ZERO,
                        speed: 1.0,
                        filler: true,
                        blinked: true,
                    });
                    return;
                }
            }
            if *rested >= routines.rest {
                self.home = (run.origin + run.net).clamp(routines.wander.0, routines.wander.1);
                self.start_routine(None);
                return;
            }
            self.run = Some(run);
            return;
        }
        run.elapsed += delta;
        run.age += delta;
        while let Some(frame) = run.frames.get(run.frame) {
            let hold = Duration::from_nanos(
                (frame.ticks as f64 * TICK.as_nanos() as f64 * run.speed) as u64,
            );
            if run.elapsed < hold {
                break;
            }
            run.elapsed -= hold;
            if run.frame + 1 < run.frames.len() {
                run.frame += 1;
            } else if run.filler {
                // the blink is over: back to standing for the rest of the rest
                run.filler = false;
                run.frame = 0;
                run.rested = Some(routines.rest / 2);
                break;
            } else {
                run.rested = Some(Duration::ZERO);
                break;
            }
        }
        self.run = Some(run);
    }

    /// How much is happening to this player, 0 to 1.
    ///
    /// A high stack and a big chain are both intense, and there is no face here that says which
    /// - the character just does bigger things when more is going on.
    fn intensity(&self) -> f64 {
        self.danger.max(self.excitement).clamp(0.0, 1.0)
    }

    /// Deal a routine, and start it from its first frame.
    ///
    /// `prefer` is a pick a caller has already made - `kirby_shot` walking them all. With
    /// `None` it is dealt out of a **bag**: a shuffled run of every routine there is, drawn
    /// from without replacement, so a game shows most of what the character has rather than
    /// the same three things. Two things filter what may come out of it - how intense the match
    /// is, against the routine's own [`RoutineChoice::intensity`], and whether the routine
    /// **fits from where he is standing**, which is what stops a spin that travels right being
    /// dealt to a character against the right post.
    fn start_routine(&mut self, prefer: Option<usize>) {
        let Some(routines) = self.meta.as_ref().and_then(|m| m.routines) else {
            self.run = None;
            return;
        };
        if self.state == CharacterState::Defeat {
            let way = RoutineWay {
                frames: routines.defeat,
                origins: (i32::MIN, i32::MAX),
                glide: (0, 0),
            };
            self.run = Some(self.begin(usize::MAX, way, routines, false));
            return;
        }
        let choices = routines.choices;
        if choices.is_empty() {
            self.run = None;
            return;
        }
        if let Some(index) = prefer {
            // `usize::MAX` is the filler, which is not in the pool and still has to be looked
            // at next to the recording
            let (index, way) = match routines.filler {
                Some(filler) if index == usize::MAX => (usize::MAX, filler),
                _ => {
                    let at = index.min(choices.len() - 1);
                    (at, self.choose_way(choices[at].ways, routines))
                }
            };
            let run = self.begin(index, way, routines, false);
            self.run = Some(run);
            return;
        }
        let want = self.intensity();
        if self.bag.is_empty() {
            self.refill(choices.len());
        }
        // A **bag**, drained strictly: every routine plays once before any plays twice, which
        // is the seven bag's bargain and is why nothing here is rare. What the intensity does
        // is decide the *order* within a cycle - the entry closest to how much is happening
        // comes out next - rather than shutting anything out, because a band that shuts things
        // out is what made the quiet ones repeat and the big ones never turn up.
        let mut at = 0usize;
        let mut best = f64::MAX;
        for (index, choice) in self.bag.iter().enumerate() {
            let off = (choices[*choice].intensity - want).abs();
            if off < best {
                best = off;
                at = index;
            }
        }
        let pick = self.bag.remove(at);
        let way = self.choose_way(choices[pick].ways, routines);
        let run = self.begin(pick, way, routines, true);
        self.run = Some(run);
    }

    /// A fresh bag: every routine once, in an order the deterministic walk decides.
    fn refill(&mut self, len: usize) {
        self.bag = (0..len).collect();
        for i in (1..self.bag.len()).rev() {
            self.tick = self.tick.wrapping_add(1);
            let j = (hashed(self.tick) * (i + 1) as f64) as usize;
            self.bag.swap(i, j.min(i));
        }
    }

    /// Which way round to play a kind, from where the character is standing.
    ///
    /// **This is what makes a run of routines read as a character rather than a shuffle.** A
    /// way whose window already holds him is played from exactly where he is; among those, the
    /// one that leaves him nearest the middle wins, so he turns round at the walls instead of
    /// sliding back to do the same thing again. Only when neither way holds him is one walked
    /// into, and then it is the nearer.
    fn choose_way(&self, ways: &'static [RoutineWay], routines: RoutineMeta) -> RoutineWay {
        let middle = (routines.wander.0 + routines.wander.1) / 2;
        let here: Vec<&RoutineWay> = ways.iter().filter(|w| fits(self.home, w)).collect();
        if !here.is_empty() {
            return **here
                .iter()
                .min_by_key(|w| {
                    let net = w.frames.last().map(|f| f.at.0).unwrap_or(0);
                    (self.home + net - middle).abs()
                })
                .unwrap();
        }
        *ways
            .iter()
            .min_by_key(|w| (self.home.clamp(w.origins.0, w.origins.1) - self.home).abs())
            .unwrap_or(&ways[0])
    }

    fn begin(
        &mut self,
        index: usize,
        way: RoutineWay,
        routines: RoutineMeta,
        vary: bool,
    ) -> RoutineRun {
        // He is never *moved* into position: the origin is the nearest point of the way's own
        // window, and he is translated there over the stretch the cutter marked - the first
        // time the routine takes him off the ground, wherever it does. A jump between routines
        // is the one thing a run of them must not have, and a slide along the floor is the
        // next thing.
        let frames = way.frames;
        let origin = if routines.approach {
            self.home.clamp(way.origins.0, way.origins.1)
        } else {
            self.home
        };
        let tick = |ticks: u32| Duration::from_nanos(ticks as u64 * TICK.as_nanos() as u64);
        let glide = if origin == self.home {
            (Duration::ZERO, Duration::ZERO)
        } else {
            (tick(way.glide.0), tick(way.glide.1.max(1)))
        };
        // no two dealings of one routine run at quite the same pace. A routine a caller asked
        // for by name runs at its measured one, since that is what `kirby_shot` puts next to
        // the recording.
        let speed = if vary {
            self.tick = self.tick.wrapping_add(1);
            let spread = routines.speed_spread.clamp(0.0, 0.9);
            1.0 + (hashed(self.tick) * 2.0 - 1.0) * spread
        } else {
            1.0
        };
        self.deals += 1;
        RoutineRun {
            routine: index,
            frames,
            frame: 0,
            elapsed: Duration::ZERO,
            rested: None,
            origin,
            net: frames.last().map(|f| f.at.0).unwrap_or(0),
            from: self.home,
            glide,
            age: Duration::ZERO,
            speed,
            filler: false,
            blinked: false,
        }
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

    /// Put the character in a state and start one named routine of it, from its first frame.
    ///
    /// For tests and for `kirby_shot`, which walks the fifteen: a deal cannot be asked for a
    /// particular routine, and every one of them has to be looked at.
    ///
    /// `home` stands the character somewhere other than the middle of the box, which is what
    /// puts a capture of a routine next to the recording it was measured off - the recording
    /// caught it wherever the routine before it had left him.
    pub fn play(&mut self, routine: usize, home: Option<i32>) {
        self.latched = false;
        self.state = CharacterState::Idle;
        self.held = Duration::ZERO;
        self.linger = Duration::ZERO;
        if let Some(meta) = self.meta.as_ref() {
            self.frames = Some(meta.animation(CharacterState::Idle));
            self.home = home.unwrap_or_else(|| meta.routines.map(|r| r.home.0).unwrap_or(0));
        }
        self.particles.clear();
        self.start_layers();
        self.start_routine(Some(routine));
    }

    /// Which routine is playing, as an index into `RoutineMeta::choices`; `None` for the
    /// filler, the defeat routine, or a character with strips.
    pub fn routine(&self) -> Option<usize> {
        self.run
            .as_ref()
            .map(|r| r.routine)
            .filter(|i| *i != usize::MAX)
    }

    /// How many routines have been dealt, so a test can tell one dealing from the next.
    pub fn deals(&self) -> u64 {
        self.deals
    }

    /// How many routines there are, so a caller can walk them.
    pub fn routines(&self) -> usize {
        self.meta
            .as_ref()
            .and_then(|m| m.routines)
            .map(|r| r.choices.len())
            .unwrap_or(0)
    }

    /// Whether the routine that is playing has run out and the character is standing.
    pub fn resting(&self) -> bool {
        self.run.map(|r| r.rested.is_some()).unwrap_or(true)
    }

    /// A clear that chained. One pop is not a reaction worth pulling a face for - it is most
    /// clears, several a minute, and it sends no attack either - so this is only called for a
    /// clear the game itself called a combo.
    pub fn chained(&mut self) {
        self.linger = LINGER;
        // ... and on the routine path it is the *intensity* that answers, not a face: a chain
        // excites the character and the next routine dealt is a bigger one
        self.excitement = (self.excitement + 0.6).min(1.0);
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
        // A routine is **not** restarted by a state change, because it is not chosen by state:
        // a chain raises the intensity and the next dealing answers it, rather than cutting
        // whatever is playing in half. Being buried is the exception, and is terminal.
        if state == CharacterState::Defeat {
            self.start_routine(None);
        }
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
            routines: None,
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

    // ------------------------------------------------------------------ routines

    /// three frames and a rest, the shape every one of Kirby's has: a stand, a pose that
    /// travels, and a frame that draws nothing because he has left the box
    const WALK: &[RoutineFrame] = &[
        RoutineFrame {
            pose: Some(0),
            ticks: 10,
            at: (0, 0),
        },
        RoutineFrame {
            pose: Some(1),
            ticks: 4,
            at: (6, 0),
        },
        RoutineFrame {
            pose: None,
            ticks: 6,
            at: (12, 0),
        },
        RoutineFrame {
            pose: Some(0),
            ticks: 2,
            at: (12, 0),
        },
    ];
    const STAND: &[RoutineFrame] = &[RoutineFrame {
        pose: Some(2),
        ticks: 4,
        at: (0, 0),
    }];

    fn routine_meta() -> CharacterMeta {
        let mut meta = CharacterMeta::plain([(1, FrameAnimationType::Static); 4]);
        meta.routines = Some(RoutineMeta {
            choices: &[
                RoutineChoice {
                    intensity: 0.0,
                    ways: &[RoutineWay {
                        frames: WALK,
                        origins: (0, 30),
                        glide: (0, 2),
                    }],
                },
                RoutineChoice {
                    intensity: 0.0,
                    ways: &[RoutineWay {
                        frames: STAND,
                        origins: (0, 30),
                        glide: (0, 2),
                    }],
                },
            ],
            filler: None,
            defeat: STAND,
            rest: Duration::from_millis(500),
            home: (10, 20),
            wander: (0, 30),
            speed_spread: 0.0,
            approach: true,
        });
        meta
    }

    fn ticks(c: &mut CharacterAnimation, n: u32) {
        for _ in 0..n {
            c.update(Duration::from_nanos(16_666_667));
        }
    }

    /// A frame is held for its own count of ticks rather than a shared rate, which is the
    /// whole reason a routine is not a strip.
    #[test]
    fn a_routine_holds_each_frame_for_its_own_time() {
        let mut c = CharacterAnimation::new();
        c.deal(routine_meta(), 0, false);
        c.play(0, None);
        assert_eq!(c.drawing(), CharacterFrame::Placed(0, (10, 20)));
        ticks(&mut c, 9);
        assert_eq!(
            c.drawing(),
            CharacterFrame::Placed(0, (10, 20)),
            "cut short"
        );
        ticks(&mut c, 2);
        assert_eq!(c.drawing(), CharacterFrame::Placed(1, (16, 20)));
    }

    /// A frame with no pose draws nothing at all - a character out of the box - and the
    /// routine goes on running underneath it.
    #[test]
    fn a_frame_with_no_pose_draws_nothing() {
        let mut c = CharacterAnimation::new();
        c.deal(routine_meta(), 0, false);
        c.play(0, None);
        ticks(&mut c, 15);
        assert_eq!(c.drawing(), CharacterFrame::Hidden);
        ticks(&mut c, 6);
        assert_eq!(c.drawing(), CharacterFrame::Placed(0, (22, 20)));
    }

    /// The last frame is held for the rest, and then another routine is dealt from wherever
    /// this one left him - which is what keeps a walk from teleporting back.
    #[test]
    fn a_finished_routine_rests_and_then_deals_another_from_where_it_left_him() {
        let mut c = CharacterAnimation::new();
        c.deal(routine_meta(), 0, false);
        c.play(0, None);
        ticks(&mut c, 22);
        assert!(c.resting(), "the routine has not run out");
        assert_eq!(c.drawing(), CharacterFrame::Placed(0, (22, 20)));
        ticks(&mut c, 40);
        // the walk carried him twelve pixels, and whatever is dealt starts from there
        match c.drawing() {
            CharacterFrame::Placed(_, (x, _)) => assert_eq!(x, 22),
            other => panic!("nothing dealt: {other:?}"),
        }
    }

    /// A state change does **not** cut a routine off, because a routine is not chosen by
    /// state: a chain raises the intensity and the next dealing answers it.
    #[test]
    fn a_state_change_does_not_interrupt_a_routine() {
        let mut c = CharacterAnimation::new();
        c.deal(routine_meta(), 0, false);
        c.play(0, None);
        ticks(&mut c, 12);
        let before = (c.routine(), c.deals());
        settle(&mut c);
        c.chained();
        assert_eq!(c.state(), CharacterState::Winning);
        assert_eq!(
            (c.routine(), c.deals()),
            before,
            "the chain dealt a new routine instead of leaving this one alone"
        );
    }

    /// **A routine starts exactly where the last one left him.** It is never shifted into
    /// place, which is what put a jump between the end of one routine and the start of the
    /// next; if it needs him somewhere else he is translated there *inside* the routine, on
    /// the stretch the way declares.
    #[test]
    fn a_routine_starts_where_the_last_one_left_him() {
        const RIGHT: &[RoutineFrame] = &[
            RoutineFrame {
                pose: Some(0),
                ticks: 2,
                at: (0, 0),
            },
            RoutineFrame {
                pose: Some(0),
                ticks: 2,
                at: (20, 0),
            },
        ];
        const STILL: &[RoutineFrame] = &[RoutineFrame {
            pose: Some(1),
            ticks: 2,
            at: (0, 0),
        }];
        let mut meta = CharacterMeta::plain([(1, FrameAnimationType::Static); 4]);
        meta.routines = Some(RoutineMeta {
            choices: &[
                RoutineChoice {
                    intensity: 0.0,
                    ways: &[RoutineWay {
                        frames: RIGHT,
                        origins: (0, 10),
                        glide: (0, 2),
                    }],
                },
                RoutineChoice {
                    intensity: 0.0,
                    ways: &[RoutineWay {
                        frames: STILL,
                        origins: (0, 30),
                        glide: (0, 2),
                    }],
                },
            ],
            filler: None,
            defeat: STILL,
            rest: Duration::from_millis(10),
            home: (10, 0),
            wander: (0, 30),
            speed_spread: 0.0,
            approach: true,
        });
        let mut c = CharacterAnimation::new();
        c.deal(meta, 0, false);
        // the drawn place at the moment a routine is dealt is the drawn place the tick before
        let mut before = None;
        let mut seen = 0u64;
        for _ in 0..900 {
            let now = match c.drawing() {
                CharacterFrame::Placed(_, at) => Some(at.0),
                _ => None,
            };
            if c.deals() != seen {
                seen = c.deals();
                if let (Some(a), Some(b)) = (before, now) {
                    assert_eq!(a, b, "he was moved as a routine was dealt");
                }
            }
            before = now;
            ticks(&mut c, 1);
        }
    }

    /// A routine he is not in position for is **walked into**, not jumped into and not
    /// refused: nothing is rare. The translation rides the stretch the way declares - which the
    /// cutter puts on the routine's first hop - so it is never a slide along the floor.
    #[test]
    fn a_routine_out_of_reach_is_walked_into_on_its_own_hop() {
        const NARROW: &[RoutineFrame] = &[
            // forty ticks on the ground, then a hop: the translation belongs to the hop
            RoutineFrame {
                pose: Some(0),
                ticks: 40,
                at: (0, 0),
            },
            RoutineFrame {
                pose: Some(0),
                ticks: 20,
                at: (0, -20),
            },
            RoutineFrame {
                pose: Some(0),
                ticks: 4,
                at: (0, 0),
            },
        ];
        let mut meta = CharacterMeta::plain([(1, FrameAnimationType::Static); 4]);
        meta.routines = Some(RoutineMeta {
            choices: &[RoutineChoice {
                intensity: 0.0,
                // three pixels of window in a thirty pixel box, which is `ricochet`'s shape
                ways: &[RoutineWay {
                    frames: NARROW,
                    origins: (8, 10),
                    glide: (40, 20),
                }],
            }],
            filler: None,
            defeat: NARROW,
            rest: Duration::from_millis(50),
            home: (30, 0),
            wander: (0, 30),
            speed_spread: 0.0,
            approach: true,
        });
        let mut c = CharacterAnimation::new();
        c.deal(meta, 0, false);
        let mut seen = vec![];
        for _ in 0..64 {
            if let CharacterFrame::Placed(_, at) = c.drawing() {
                seen.push(at.0);
            }
            ticks(&mut c, 1);
        }
        assert_eq!(seen[0], 30, "he did not start where he was standing");
        // nothing moves while he is on the ground
        assert!(
            seen[..40].iter().all(|x| *x == 30),
            "he slid along the floor: {:?}",
            &seen[..40]
        );
        assert_eq!(*seen.last().unwrap(), 10, "he never walked into the window");
    }

    /// The intensity decides the **order** a cycle comes out in, not whether a routine may
    /// come out at all - a band that shut things out is what made the quiet ones repeat.
    #[test]
    fn intensity_orders_the_bag_and_a_chain_raises_it() {
        const STILL: &[RoutineFrame] = &[RoutineFrame {
            pose: Some(0),
            ticks: 4,
            at: (0, 0),
        }];
        const BIG: &[RoutineFrame] = &[RoutineFrame {
            pose: Some(1),
            ticks: 4,
            at: (0, 0),
        }];
        let mut meta = CharacterMeta::plain([(1, FrameAnimationType::Static); 4]);
        meta.routines = Some(RoutineMeta {
            choices: &[
                RoutineChoice {
                    intensity: 0.0,
                    ways: &[RoutineWay {
                        frames: STILL,
                        origins: (0, 30),
                        glide: (0, 2),
                    }],
                },
                RoutineChoice {
                    intensity: 1.0,
                    ways: &[RoutineWay {
                        frames: BIG,
                        origins: (0, 30),
                        glide: (0, 2),
                    }],
                },
            ],
            filler: None,
            defeat: STILL,
            rest: Duration::from_millis(50),
            home: (10, 20),
            wander: (0, 30),
            speed_spread: 0.0,
            approach: true,
        });
        let mut c = CharacterAnimation::new();
        c.deal(meta.clone(), 0, false);
        // a quiet board opens on the still one ...
        c.danger(0.0, false);
        assert!(
            matches!(c.drawing(), CharacterFrame::Placed(0, _)),
            "opened too big"
        );
        // ... and both are played over a cycle, because nothing is shut out
        let mut both = [false; 2];
        for _ in 0..400 {
            if let CharacterFrame::Placed(pose, _) = c.drawing() {
                both[pose] = true;
            }
            ticks(&mut c, 1);
        }
        assert!(
            both[0] && both[1],
            "the bag did not deal everything: {both:?}"
        );
        // a chain raises the intensity, so the big one comes out of the next fresh bag first
        let mut c = CharacterAnimation::new();
        c.deal(meta, 0, false);
        c.danger(0.0, false);
        ticks(&mut c, 40); // drain the still one
        c.chained();
        let mut saw_big = false;
        for _ in 0..40 {
            if matches!(c.drawing(), CharacterFrame::Placed(1, _)) {
                saw_big = true;
            }
            ticks(&mut c, 1);
        }
        assert!(saw_big, "a chain changed nothing");
    }

    /// The filler is played *between* routines and is never one of them.
    #[test]
    fn the_filler_goes_between_routines() {
        const STILL: &[RoutineFrame] = &[RoutineFrame {
            pose: Some(0),
            ticks: 4,
            at: (0, 0),
        }];
        const BLINK: &[RoutineFrame] = &[RoutineFrame {
            pose: Some(9),
            ticks: 4,
            at: (0, 0),
        }];
        let mut meta = CharacterMeta::plain([(1, FrameAnimationType::Static); 4]);
        meta.routines = Some(RoutineMeta {
            choices: &[RoutineChoice {
                intensity: 0.0,
                ways: &[RoutineWay {
                    frames: STILL,
                    origins: (0, 30),
                    glide: (0, 2),
                }],
            }],
            filler: Some(RoutineWay {
                frames: BLINK,
                origins: (0, 30),
                glide: (0, 0),
            }),
            defeat: STILL,
            rest: Duration::from_millis(400),
            home: (10, 20),
            wander: (0, 30),
            speed_spread: 0.0,
            approach: true,
        });
        let mut c = CharacterAnimation::new();
        c.deal(meta, 0, false);
        let mut blinks = 0;
        let mut was = false;
        for _ in 0..3000 {
            let now = matches!(c.drawing(), CharacterFrame::Placed(9, _));
            if now && !was {
                blinks += 1;
            }
            was = now;
            ticks(&mut c, 1);
        }
        // it is a coin toss on each rest, so this is "sometimes" and not "always"
        assert!(blinks > 3, "the filler never played: {blinks}");
    }

    /// A bag deals everything before it deals anything twice, so a game shows most of what
    /// there is rather than the same three things.
    #[test]
    fn the_bag_deals_everything_before_repeating() {
        const A: &[RoutineFrame] = &[RoutineFrame {
            pose: Some(0),
            ticks: 2,
            at: (0, 0),
        }];
        let mut meta = CharacterMeta::plain([(1, FrameAnimationType::Static); 4]);
        meta.routines = Some(RoutineMeta {
            choices: &[
                RoutineChoice {
                    intensity: 0.0,
                    ways: &[RoutineWay {
                        frames: A,
                        origins: (0, 30),
                        glide: (0, 2),
                    }],
                },
                RoutineChoice {
                    intensity: 0.0,
                    ways: &[RoutineWay {
                        frames: A,
                        origins: (0, 30),
                        glide: (0, 2),
                    }],
                },
                RoutineChoice {
                    intensity: 0.0,
                    ways: &[RoutineWay {
                        frames: A,
                        origins: (0, 30),
                        glide: (0, 2),
                    }],
                },
                RoutineChoice {
                    intensity: 0.0,
                    ways: &[RoutineWay {
                        frames: A,
                        origins: (0, 30),
                        glide: (0, 2),
                    }],
                },
            ],
            filler: None,
            defeat: A,
            rest: Duration::from_millis(20),
            home: (10, 20),
            wander: (0, 30),
            speed_spread: 0.0,
            approach: true,
        });
        let mut c = CharacterAnimation::new();
        c.deal(meta, 0, false);
        let mut dealt = vec![];
        let mut seen = 0;
        for _ in 0..4000 {
            if c.deals() != seen {
                seen = c.deals();
                if let Some(index) = c.routine() {
                    dealt.push(index);
                }
            }
            ticks(&mut c, 1);
        }
        assert!(dealt.len() >= 16, "only {} dealings", dealt.len());
        for cycle in dealt.chunks(4) {
            if cycle.len() < 4 {
                continue;
            }
            let mut seen = cycle.to_vec();
            seen.sort_unstable();
            seen.dedup();
            assert_eq!(seen.len(), 4, "a bag repeated inside a cycle: {dealt:?}");
        }
    }

    /// A character with strips is untouched by any of it    /// A character with strips is untouched by any of it: the mugshot path draws a frame
    /// filling its box and never a placed pose.
    #[test]
    fn a_character_with_strips_still_draws_whole_frames() {
        let c = dealt();
        assert!(matches!(c.drawing(), CharacterFrame::Whole(_)));
        assert_eq!(c.routines(), 0);
    }
}
