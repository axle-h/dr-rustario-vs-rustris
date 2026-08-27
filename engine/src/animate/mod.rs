//! Per-player animation state machines. They are pure timing: a theme reads their state when
//! it draws. Every slot is game-neutral; the optional ones (mascot, spawn arc, idle cells)
//! are left unset by games that have no such thing.

pub mod cell_idle;
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
pub mod popup;
pub mod spawn;
pub mod victory;

use crate::animate::cell_idle::CellIdleAnimation;
use crate::animate::destroy::{DestroyAnimation, DestroyStyle};
use crate::animate::event::{AnimationEvent, AnimationType};
use crate::animate::frames::{FrameAnimation, FrameAnimationType};
use crate::animate::game_over::{GameOverAnimation, GameOverStyle};
use crate::animate::hard_drop::HardDropAnimation;
use crate::animate::impact::ImpactAnimation;
use crate::animate::interstitial::InterstitialAnimation;
use crate::animate::lock::LockAnimation;
use crate::animate::mascot::MascotMeta;
use crate::animate::next_stage::NextStageAnimation;
use crate::animate::popup::PopupAnimation;
use crate::animate::spawn::{SpawnAnimation, SpawnArc};
use crate::animate::victory::VictoryAnimation;
use crate::game::{CellId, GameEvent};
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
}

impl AnimationMeta {
    pub fn cell_idle_frames(&self, id: CellId) -> Option<usize> {
        self.cell_idle
            .iter()
            .find(|(cell, _)| *cell == id)
            .map(|(_, frames)| *frames)
    }
}

#[derive(Clone, Debug)]
pub struct PlayerAnimations {
    player: u32,
    mascot_idle: Option<FrameAnimation>,
    cell_idle: CellIdleAnimation,
    destroy: DestroyAnimation,
    impact: ImpactAnimation,
    lock: LockAnimation,
    hard_drop: HardDropAnimation,
    spawn: SpawnAnimation,
    game_over: GameOverAnimation,
    victory: VictoryAnimation,
    next_stage: NextStageAnimation,
    interstitial: InterstitialAnimation,
    popup: PopupAnimation,
}

impl PlayerAnimations {
    pub fn new(player: u32, meta: &AnimationMeta) -> Self {
        Self {
            player,
            mascot_idle: meta.mascot.map(|m| m.idle()),
            cell_idle: CellIdleAnimation::new(meta.cell_idle_type, &meta.cell_idle),
            destroy: DestroyAnimation::new(meta.destroy.clone()),
            impact: ImpactAnimation::new(),
            lock: LockAnimation::new(),
            hard_drop: HardDropAnimation::new(meta.hard_drop_rows_per_frame),
            spawn: SpawnAnimation::new(meta.spawn_arc, meta.mascot),
            game_over: GameOverAnimation::new(meta.game_over, meta.mascot),
            victory: VictoryAnimation::new(meta.mascot),
            next_stage: NextStageAnimation::new(),
            interstitial: InterstitialAnimation::new(meta.interstitial_frames, meta.mascot),
            popup: PopupAnimation::new(),
        }
    }

    pub fn reset(&mut self) {
        if let Some(mascot) = self.mascot_idle.as_mut() {
            mascot.reset();
        }
        self.cell_idle.reset();
        self.destroy.reset();
        self.impact.reset();
        self.lock.reset();
        self.hard_drop.reset();
        self.spawn.reset();
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
        self.destroy.update(delta);
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
        self.interstitial.update(delta);
        self.popup.update(delta);
        events
    }

    /// react to the unconditional game events; the session decides about game over, victory
    /// and stage transitions itself
    pub fn on_event(&mut self, event: &GameEvent) {
        match event {
            GameEvent::Clear { cells, .. } => self.destroy.add(cells.clone()),
            GameEvent::Lock { cells, dropped } => {
                if *dropped {
                    self.impact.impact();
                }
                self.lock.lock(cells);
            }
            GameEvent::HardDrop {
                cells,
                dropped_rows,
            } => self.hard_drop.hard_drop(cells, *dropped_rows),
            GameEvent::Spawn { piece, is_hold, .. } => {
                self.spawn.spawn(*piece, *is_hold);
            }
            _ => {}
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
