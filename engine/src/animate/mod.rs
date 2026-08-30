//! Per-player animation state machines. They are pure timing: a theme reads their state when
//! it draws. Every slot is game-neutral; the optional ones (mascot, spawn arc, idle cells)
//! are left unset by games that have no such thing.

pub mod attack_ball;
pub mod bounce;
pub mod cell_idle;
pub mod character;
pub mod debris;
pub mod destroy;
pub mod event;
pub mod frames;
pub mod game_over;
pub mod hard_drop;
pub mod impact;
pub mod interstitial;
pub mod lock;
pub mod mascot;
pub mod next_stage;
pub mod nuisance;
pub mod popup;
pub mod spawn;
pub mod tray;
pub mod victory;

use crate::animate::bounce::BounceAnimation;
use crate::animate::cell_idle::CellIdleAnimation;
use crate::animate::character::CharacterAnimation;
use crate::animate::debris::{BurstSpec, DebrisAnimation, DebrisArt, Spread};
use crate::animate::destroy::{DestroyAnimation, DestroyStyle, PopPhase};
use crate::animate::event::{AnimationEvent, AnimationType};
use crate::animate::frames::{FrameAnimation, FrameAnimationType};
use crate::animate::game_over::{GameOverAnimation, GameOverStyle};
use crate::animate::hard_drop::HardDropAnimation;
use crate::animate::impact::ImpactAnimation;
use crate::animate::interstitial::InterstitialAnimation;
use crate::animate::lock::LockAnimation;
use crate::animate::mascot::MascotMeta;
use crate::animate::next_stage::NextStageAnimation;
use crate::animate::nuisance::NuisanceAnimation;
use crate::animate::popup::PopupAnimation;
use crate::animate::spawn::{SpawnAnimation, SpawnArc};
use crate::animate::tray::TrayAnimation;
use crate::animate::victory::VictoryAnimation;
use crate::game::geometry::Point;
use crate::game::CellId;
use crate::game::PlacedCell;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

/// Everything a theme declares about how its animations run.
#[derive(Clone, Debug)]
pub struct AnimationMeta {
    pub destroy: DestroyStyle,
    pub game_over: GameOverStyle,
    pub interstitial_frames: usize,
    /// cells that animate while idle on the board, with their frame counts
    pub cell_idle_type: FrameAnimationType,
    pub cell_idle: Vec<(CellId, usize)>,
    pub spawn_arc: Option<SpawnArc>,
    pub mascot: Option<MascotMeta>,
    /// how fast the hard drop trail falls, in rows per 4ms frame
    pub hard_drop_rows_per_frame: f64,
    /// what a cell throws off as it pops, if the theme wants a burst
    pub pop_debris: Option<PopDebris>,
    /// How hard the board shakes when a slab of nuisance lands, as a fraction of a block,
    /// and for how long.
    ///
    /// Opt-in, and off by default, because the original **does not shake** - see
    /// [`crate::animate::impact`], where that is measured rather than remembered. It is a
    /// modern flare: the particle theme takes it and neither retro theme does.
    pub nuisance_rumble: Option<(f64, Duration)>,
}

/// The pieces a popping cell throws, and when in its strip it throws them.
///
/// It is fired from [`PlayerAnimations::update`] rather than by the match screen, because
/// what it needs to know - which frame of which cell's strip is on - lives here, and because
/// the pieces have to **outlive the clear**: a chain settles and the next step starts
/// blinking while the last one's droplets are still in the air.
#[derive(Clone, Copy, Debug)]
pub struct PopDebris {
    /// which frame of the pop strip the cell bursts on
    pub at_frame: usize,
    pub pieces: usize,
    pub speed: (f64, f64),
    pub gravity: f64,
    pub life: Duration,
    pub size: f64,
}

impl AnimationMeta {
    pub fn cell_idle_frames(&self, id: CellId) -> Option<usize> {
        self.cell_idle
            .iter()
            .find(|(cell, _)| *cell == id)
            .map(|(_, frames)| *frames)
    }
}

/// The middle of a group of cells, in fractional board coordinates, and the cell id most of
/// them are - which is the group's colour for anything that wants to be drawn in it.
///
/// Ties break on the cell id, so the same group always gives the same answer.
pub fn centre_and_modal(cells: &[PlacedCell]) -> Option<((f64, f64), CellId)> {
    if cells.is_empty() {
        return None;
    }
    let column = cells.iter().map(|(p, _)| p.x as f64).sum::<f64>() / cells.len() as f64;
    let row = cells.iter().map(|(p, _)| p.y as f64).sum::<f64>() / cells.len() as f64;
    let mut counts: HashMap<CellId, usize> = HashMap::new();
    for (_, id) in cells {
        *counts.entry(*id).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(id, count)| (*count, id.0))
        .map(|(id, _)| ((column, row), id))
}

#[derive(Clone, Debug)]
pub struct PlayerAnimations {
    player: u32,
    mascot_idle: Option<FrameAnimation>,
    cell_idle: CellIdleAnimation,
    character: CharacterAnimation,
    bounce: BounceAnimation,
    debris: DebrisAnimation,
    pop_debris: Option<PopDebris>,
    nuisance_rumble: Option<(f64, Duration)>,
    /// which cells of the clear in play have already burst, so each does so once
    burst: HashSet<Point>,
    destroy: DestroyAnimation,
    impact: ImpactAnimation,
    lock: LockAnimation,
    hard_drop: HardDropAnimation,
    spawn: SpawnAnimation,
    game_over: GameOverAnimation,
    victory: VictoryAnimation,
    next_stage: NextStageAnimation,
    nuisance: NuisanceAnimation,
    tray: TrayAnimation,
    interstitial: InterstitialAnimation,
    popup: PopupAnimation,
}

impl PlayerAnimations {
    pub fn new(player: u32, meta: &AnimationMeta) -> Self {
        Self {
            player,
            mascot_idle: meta.mascot.map(|m| m.idle()),
            cell_idle: CellIdleAnimation::new(meta.cell_idle_type, &meta.cell_idle),
            character: CharacterAnimation::new(),
            bounce: BounceAnimation::new(),
            debris: DebrisAnimation::new(),
            pop_debris: meta.pop_debris,
            nuisance_rumble: meta.nuisance_rumble,
            burst: HashSet::new(),
            destroy: DestroyAnimation::new(meta.destroy.clone()),
            impact: ImpactAnimation::new(),
            lock: LockAnimation::new(),
            hard_drop: HardDropAnimation::new(meta.hard_drop_rows_per_frame),
            spawn: SpawnAnimation::new(meta.spawn_arc, meta.mascot),
            game_over: GameOverAnimation::new(meta.game_over, meta.mascot),
            victory: VictoryAnimation::new(meta.mascot),
            next_stage: NextStageAnimation::new(),
            nuisance: NuisanceAnimation::new(),
            tray: TrayAnimation::new(),
            interstitial: InterstitialAnimation::new(meta.interstitial_frames, meta.mascot),
            popup: PopupAnimation::new(),
        }
    }

    pub fn reset(&mut self) {
        if let Some(mascot) = self.mascot_idle.as_mut() {
            mascot.reset();
        }
        self.cell_idle.reset();
        self.bounce.reset();
        self.debris.reset();
        self.burst.clear();
        self.destroy.reset();
        self.impact.reset();
        self.lock.reset();
        self.hard_drop.reset();
        self.spawn.reset();
        self.nuisance.reset();
        self.tray.reset();
        self.popup.reset();
    }

    pub fn update(&mut self, delta: Duration) -> Vec<AnimationEvent> {
        if delta.is_zero() {
            return vec![];
        }

        let mut events = vec![];
        if let Some(mascot) = self.mascot_idle.as_mut() {
            mascot.update(delta);
        }
        self.cell_idle.update(delta);
        self.character.update(delta);
        self.bounce.update(delta);
        self.debris.update(delta);
        self.destroy.update(delta);
        self.burst_popping_cells();
        self.impact.update(delta);
        self.lock.update(delta);
        self.hard_drop.update(delta);
        if self.spawn.update(delta) {
            events.push(AnimationEvent::Finished {
                animation: AnimationType::Spawn,
                player: self.player,
            });
        }
        self.game_over.update(delta);
        self.victory.update(delta);
        self.next_stage.update(delta);
        // a slab arrives one column at a time, and every bean of it bounces where it lands -
        // which is the rumble a nuisance drop actually has: nothing shakes, but the whole
        // bottom of the board is jolted at once
        let landed = self.nuisance.update(delta);
        if !landed.is_empty() {
            self.bounce.land(&landed);
            if let Some((amplitude, duration)) = self.nuisance_rumble {
                self.impact.rumble(amplitude, duration);
            }
        }
        self.tray.update(delta);
        self.interstitial.update(delta);
        self.popup.update(delta);
        events
    }

    /// Throw the droplets of every cell that has just reached the burst frame of its strip.
    ///
    /// Once each, and only while a clear is playing - the pieces then live on in the debris
    /// pool, on their own clock, long after the clear that threw them is over.
    fn burst_popping_cells(&mut self) {
        let Some(spec) = self.pop_debris else {
            self.burst.clear();
            return;
        };
        let Some(state) = self.destroy.state() else {
            self.burst.clear();
            return;
        };
        let mut bursting = vec![];
        for (point, id) in state.cells().iter().copied() {
            if self.burst.contains(&point) {
                continue;
            }
            if let Some(PopPhase::Strip { frame }) = self.destroy.pop_phase(id) {
                if frame >= spec.at_frame {
                    bursting.push((point, id));
                }
            }
        }
        for (point, id) in bursting {
            self.burst.insert(point);
            self.debris.burst(BurstSpec {
                // the middle of the cell it came out of, in the board's own coordinates
                origin: (point.x as f64 + 0.5, point.y as f64 + 0.5),
                count: spec.pieces,
                speed: spec.speed,
                spread: Spread::AllDirections,
                gravity: spec.gravity,
                life: spec.life,
                fade_last: 0.5,
                size: spec.size,
                art: DebrisArt::Debris(id),
            });
        }
    }

    /// the game must not tick while one of these plays. A popup is not among them: it is
    /// decoration, and the board carries on underneath it
    pub fn blocks_tick(&self) -> bool {
        self.destroy.state().is_some()
            || self.lock.state().is_some()
            || self.hard_drop.state().is_some()
            || self.spawn.state().is_some()
            || self.game_over.state().is_some()
            || self.victory.state().is_some()
            || self.next_stage.state().is_some()
            || self.nuisance.state().is_some()
            || self.interstitial.state().is_some()
    }

    /// whether the player is between stages or out of the match, when a sprint clock stops.
    /// In-play animations (spawn, lock, clears) are part of playing and keep the clock running.
    pub fn stops_clock(&self) -> bool {
        self.game_over.state().is_some()
            || self.victory.state().is_some()
            || self.next_stage.state().is_some()
            || self.interstitial.state().is_some()
    }

    pub fn mascot_idle_frame(&self) -> Option<usize> {
        self.mascot_idle.map(|m| m.frame())
    }

    pub fn cell_idle(&self) -> &CellIdleAnimation {
        &self.cell_idle
    }

    pub fn character(&self) -> &CharacterAnimation {
        &self.character
    }

    pub fn character_mut(&mut self) -> &mut CharacterAnimation {
        &mut self.character
    }

    pub fn bounce(&self) -> &BounceAnimation {
        &self.bounce
    }

    pub fn bounce_mut(&mut self) -> &mut BounceAnimation {
        &mut self.bounce
    }

    pub fn debris(&self) -> &DebrisAnimation {
        &self.debris
    }

    pub fn debris_mut(&mut self) -> &mut DebrisAnimation {
        &mut self.debris
    }

    pub fn destroy(&self) -> &DestroyAnimation {
        &self.destroy
    }

    pub fn destroy_mut(&mut self) -> &mut DestroyAnimation {
        &mut self.destroy
    }

    pub fn impact(&self) -> &ImpactAnimation {
        &self.impact
    }

    pub fn impact_mut(&mut self) -> &mut ImpactAnimation {
        &mut self.impact
    }

    pub fn lock(&self) -> &LockAnimation {
        &self.lock
    }

    pub fn lock_mut(&mut self) -> &mut LockAnimation {
        &mut self.lock
    }

    pub fn hard_drop(&self) -> &HardDropAnimation {
        &self.hard_drop
    }

    pub fn hard_drop_mut(&mut self) -> &mut HardDropAnimation {
        &mut self.hard_drop
    }

    pub fn spawn(&self) -> &SpawnAnimation {
        &self.spawn
    }

    pub fn spawn_mut(&mut self) -> &mut SpawnAnimation {
        &mut self.spawn
    }

    pub fn game_over(&self) -> &GameOverAnimation {
        &self.game_over
    }

    pub fn game_over_mut(&mut self) -> &mut GameOverAnimation {
        &mut self.game_over
    }

    pub fn victory(&self) -> &VictoryAnimation {
        &self.victory
    }

    pub fn victory_mut(&mut self) -> &mut VictoryAnimation {
        &mut self.victory
    }

    pub fn next_stage(&self) -> &NextStageAnimation {
        &self.next_stage
    }

    pub fn next_stage_mut(&mut self) -> &mut NextStageAnimation {
        &mut self.next_stage
    }

    pub fn nuisance(&self) -> &NuisanceAnimation {
        &self.nuisance
    }

    pub fn nuisance_mut(&mut self) -> &mut NuisanceAnimation {
        &mut self.nuisance
    }

    pub fn tray(&self) -> &TrayAnimation {
        &self.tray
    }

    pub fn tray_mut(&mut self) -> &mut TrayAnimation {
        &mut self.tray
    }

    pub fn interstitial(&self) -> &InterstitialAnimation {
        &self.interstitial
    }

    pub fn interstitial_mut(&mut self) -> &mut InterstitialAnimation {
        &mut self.interstitial
    }

    pub fn popup(&self) -> &PopupAnimation {
        &self.popup
    }

    pub fn popup_mut(&mut self) -> &mut PopupAnimation {
        &mut self.popup
    }
}
