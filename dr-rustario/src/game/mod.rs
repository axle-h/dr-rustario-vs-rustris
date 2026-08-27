use crate::game::bottle::{SendGarbage, BOTTLE_HEIGHT, BOTTLE_WIDTH};
use crate::game::cell::{decode_garbage, encode_garbage, placed_vitamins, GAME_ID};
use crate::game::event::ColoredBlock;

use crate::game::pill::{PillShape, VirusColor};
use crate::game::random::GameRandom;

use std::time::Duration;
use strum::IntoEnumIterator;

use crate::game::geometry::BottlePoint;
use engine::game::hold::HoldState;
use engine::game::timing::{lock_move, LockMove, LockPlacements, Timing};
use engine::game::{
    ids, Attack, Cell, GameEvent, GameId, MetricKind, PieceId, StageState, StageTransition,
};

#[cfg(not(test))]
use crate::game::bottle::Bottle;
#[cfg(test)]
use crate::game::tests::MockBottle as Bottle;

pub mod ai;
pub mod block;
pub mod bottle;
pub mod cell;
pub mod event;
pub mod geometry;
pub mod pill;
pub mod random;
pub mod rules;

const GARBAGE_DROP_DURATION: Duration = Duration::from_millis(200);
const TIMING: Timing = Timing::new(Duration::from_millis(500), Duration::from_millis(300 / 2));
const PILLS_PER_SPEED_LEVEL: usize = 10;
pub const MAX_SCORE: u32 = 9999999;

const SPEED_TABLE: [Duration; 81] = [
    Duration::from_nanos(1166666667),
    Duration::from_nanos(1133333333),
    Duration::from_nanos(1100000000),
    Duration::from_nanos(1066666667),
    Duration::from_nanos(1033333333),
    Duration::from_nanos(1000000000),
    Duration::from_nanos(966666667),
    Duration::from_nanos(933333333),
    Duration::from_nanos(900000000),
    Duration::from_nanos(866666667),
    Duration::from_nanos(833333333),
    Duration::from_nanos(800000000),
    Duration::from_nanos(766666667),
    Duration::from_nanos(733333333),
    Duration::from_nanos(700000000),
    Duration::from_nanos(666666667),
    Duration::from_nanos(633333333),
    Duration::from_nanos(600000000),
    Duration::from_nanos(566666667),
    Duration::from_nanos(533333333),
    Duration::from_nanos(500000000),
    Duration::from_nanos(466666667),
    Duration::from_nanos(433333333),
    Duration::from_nanos(400000000),
    Duration::from_nanos(366666667),
    Duration::from_nanos(333333333),
    Duration::from_nanos(316666667),
    Duration::from_nanos(300000000),
    Duration::from_nanos(283333333),
    Duration::from_nanos(266666667),
    Duration::from_nanos(250000000),
    Duration::from_nanos(233333333),
    Duration::from_nanos(216666667),
    Duration::from_nanos(200000000),
    Duration::from_nanos(183333333),
    Duration::from_nanos(166666667),
    Duration::from_nanos(166666667),
    Duration::from_nanos(150000000),
    Duration::from_nanos(150000000),
    Duration::from_nanos(133333333),
    Duration::from_nanos(133333333),
    Duration::from_nanos(116666667),
    Duration::from_nanos(116666667),
    Duration::from_nanos(100000000),
    Duration::from_nanos(100000000),
    Duration::from_nanos(100000000),
    Duration::from_nanos(100000000),
    Duration::from_nanos(100000000),
    Duration::from_nanos(100000000),
    Duration::from_nanos(100000000),
    Duration::from_nanos(100000000),
    Duration::from_nanos(100000000),
    Duration::from_nanos(100000000),
    Duration::from_nanos(100000000),
    Duration::from_nanos(100000000),
    Duration::from_nanos(83333333),
    Duration::from_nanos(83333333),
    Duration::from_nanos(83333333),
    Duration::from_nanos(83333333),
    Duration::from_nanos(83333333),
    Duration::from_nanos(66666667),
    Duration::from_nanos(66666667),
    Duration::from_nanos(66666667),
    Duration::from_nanos(66666667),
    Duration::from_nanos(66666667),
    Duration::from_nanos(50000000),
    Duration::from_nanos(50000000),
    Duration::from_nanos(50000000),
    Duration::from_nanos(50000000),
    Duration::from_nanos(50000000),
    Duration::from_nanos(33333333),
    Duration::from_nanos(33333333),
    Duration::from_nanos(33333333),
    Duration::from_nanos(33333333),
    Duration::from_nanos(33333333),
    Duration::from_nanos(33333333),
    Duration::from_nanos(33333333),
    Duration::from_nanos(33333333),
    Duration::from_nanos(33333333),
    Duration::from_nanos(33333333),
    Duration::from_nanos(16666667),
];

const BASE_SCORE_LOW: u32 = 100;
const BASE_SCORE_MEDIUM: u32 = 200;
const BASE_SCORE_HIGH: u32 = 300;

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, strum::IntoStaticStr, strum::EnumIter, strum::EnumString,
)]
pub enum GameSpeed {
    #[strum(serialize = "low")]
    Low = 0,
    #[strum(serialize = "medium")]
    Medium = 1,
    #[strum(serialize = "high")]
    High = 2,
}

impl GameSpeed {
    pub fn names() -> Vec<&'static str> {
        Self::iter().map(|e| e.into()).collect()
    }
}

impl GameSpeed {
    const MAX_LEVEL: usize = 49;

    fn min_drop_duration(&self) -> Duration {
        self.duration_of_level(Self::MAX_LEVEL)
    }

    fn duration_of_level(&self, speed_level: usize) -> Duration {
        let index = match self {
            GameSpeed::Low => 15,
            GameSpeed::Medium => 25,
            GameSpeed::High => 31,
        } + speed_level.min(Self::MAX_LEVEL);
        SPEED_TABLE[index]
    }

    fn base_score(&self) -> u32 {
        match self {
            GameSpeed::Low => BASE_SCORE_LOW,
            GameSpeed::Medium => BASE_SCORE_MEDIUM,
            GameSpeed::High => BASE_SCORE_HIGH,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameState {
    Spawn(Duration),
    SpawnHold(Option<PillShape>),
    Fall(Duration),
    Lock(Duration),
    /// check the bottle for patterns to destroy
    Pattern(Combo),
    /// destroy marked patterns
    Destroy(Vec<ColoredBlock>, Combo),
    DropGarbage(Duration, Combo),
    GameOver,
    LevelComplete,
}

impl GameState {
    const NEW_LOCK: Self = Self::Lock(Duration::ZERO);
    const LOCK_NOW: Self = Self::Lock(TIMING.lock);
    const NEW_FALL: Self = Self::Fall(Duration::ZERO);
    const NEW_SPAWN: Self = Self::Spawn(Duration::ZERO);
    const NEW_PATTERN: Self = Self::Pattern(Combo::empty());

    fn drop_garbage(combo: Combo) -> Self {
        Self::DropGarbage(Duration::ZERO, combo)
    }
}

/// What a combo is worth to a player of the *other* game, in rows of garbage, since that is
/// what an attack is over there. Most combos are two patterns - one pill finishing two lines
/// at once - which is nowhere near the work a Rustris player puts into a row, so the first
/// pattern of a combo buys nothing abroad and the rest buy a row each. A real chain still
/// hurts, up to the four rows a tetris sends
const MAX_FOREIGN_GARBAGE_ROWS: u32 = 4;

/// what `blocks` of garbage is worth to a player of `receiver`, in that game's own units.
/// Only the sender knows what the combo took, so only it can price the crossing; a game
/// nothing here prices is worth nothing and the attack never leaves.
fn foreign_attack(receiver: GameId, blocks: u32) -> u32 {
    if receiver == ids::RUSTRIS {
        blocks.saturating_sub(1).min(MAX_FOREIGN_GARBAGE_ROWS)
    } else {
        0
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Combo {
    patterns: Vec<VirusColor>,
    viruses: u32,
}

impl Combo {
    pub fn new(patterns: Vec<VirusColor>, viruses: u32) -> Self {
        Self { patterns, viruses }
    }

    const fn empty() -> Self {
        Self {
            patterns: vec![],
            viruses: 0,
        }
    }

    fn into_updated(mut self, patterns: Vec<VirusColor>, viruses: u32) -> Self {
        for color in patterns {
            self.patterns.push(color);
        }
        self.viruses += viruses;
        self
    }

    fn is_combo(&self) -> bool {
        self.patterns.len() > 1
    }

    fn score(&self, speed: GameSpeed) -> u32 {
        if self.viruses == 0 {
            return 0;
        }
        // |NUMBER OF VIRUSES |   LOW   |   MED   |   HIGH   |
        // |   ELIMINATED     |  SPEED  |  SPEED  |  SPEED   |
        // |¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯|¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯¯|
        // |	    1		  |   100   |   200   |   300    |
        // |        2         |   200   |   400   |   600    |
        // |        3         |   400   |   800   |  1200    |
        // |        4         |   800   |  1600   |  2400    |
        // |        5         |  1600   |  3200   |  4800    |
        // |        6         |  3200   |  6400   |  9600    |
        let base_score = speed.base_score();
        (0..self.viruses).map(|i| base_score * 2_u32.pow(i)).sum()
    }

    fn garbage(&self) -> SendGarbage {
        if self.is_combo() {
            self.patterns.clone()
        } else {
            vec![]
        }
    }
}

pub struct Game {
    virus_level: u32,
    level_count: u32,
    speed: GameSpeed,
    random: GameRandom,
    events: Vec<GameEvent>,
    bottle: Bottle,
    state: GameState,
    score: u32,
    total_pills: usize,
    soft_drop: bool,
    hard_dropped: bool,
    hold: Option<HoldState<PillShape>>,
    garbage_buffer: Vec<SendGarbage>,
}

impl Game {
    pub fn new(virus_level: u32, speed: GameSpeed, mut random: GameRandom) -> Result<Self, String> {
        let bottle = Bottle::from_seed(random.bottle_seed(virus_level)?, random.garbage_rng());
        Ok(Self::from_bottle(virus_level, speed, random, bottle))
    }

    pub fn from_bottle(
        virus_level: u32,
        speed: GameSpeed,
        random: GameRandom,
        bottle: Bottle,
    ) -> Self {
        Self {
            virus_level,
            level_count: 0,
            speed,
            random,
            events: vec![],
            bottle,
            state: GameState::NEW_SPAWN,
            score: 0,
            total_pills: 0,
            soft_drop: false,
            hard_dropped: false,
            hold: None,
            garbage_buffer: vec![],
        }
    }

    pub fn next_level(&mut self) -> Result<(), String> {
        assert_eq!(self.state, GameState::LevelComplete);
        self.virus_level += 1;
        self.level_count += 1;
        self.events.clear();
        self.bottle = Bottle::from_seed(
            self.random.bottle_seed(self.virus_level)?,
            self.random.garbage_rng(),
        );
        self.state = GameState::NEW_SPAWN;
        self.total_pills = 0;
        self.soft_drop = false;
        self.hard_dropped = false;
        // the held pill carries into the next bottle
        HoldState::unlock(&mut self.hold);
        self.garbage_buffer.clear();
        Ok(())
    }

    /// the bottle the AI reads and simulates placements on
    pub(crate) fn bottle(&self) -> &Bottle {
        &self.bottle
    }

    /// the shape holding, if any: what pressing hold would swap the pill in play for
    pub(crate) fn held_shape(&self) -> Option<PillShape> {
        self.hold.map(|h| h.piece)
    }

    /// the shape at the front of the queue, which is what hold takes when nothing is held
    pub(crate) fn next_shape(&self) -> PillShape {
        self.random.peek()[0]
    }

    /// hold is locked until the pill in play locks
    pub(crate) fn can_hold(&self) -> bool {
        !HoldState::is_locked(&self.hold)
    }

    pub fn viruses(&self) -> Vec<ColoredBlock> {
        self.bottle.viruses()
    }

    pub fn hold(&mut self) {
        if HoldState::is_locked(&self.hold) {
            return;
        }

        let held_shape = match self.bottle.hold() {
            None => return,
            Some(shape) => shape,
        };

        self.state = GameState::SpawnHold(self.hold.map(|h| h.piece));
        self.hold = Some(HoldState::locked(held_shape));
        self.events.push(GameEvent::Hold);
    }

    pub fn set_soft_drop(&mut self, soft_drop: bool) {
        self.soft_drop = soft_drop;
        if soft_drop {
            self.events.push(GameEvent::SoftDrop);
        }
    }

    pub fn hard_drop(&mut self) {
        if let Some((dropped_rows, vitamins)) = self.bottle.hard_drop() {
            self.state = GameState::LOCK_NOW;
            self.hard_dropped = true;
            self.events.push(GameEvent::HardDrop {
                cells: placed_vitamins(vitamins),
                dropped_rows,
            });
        }
    }

    pub fn left(&mut self) {
        if self.with_checking_lock(|bottle| bottle.left()) {
            self.events.push(GameEvent::Move);
        }
    }

    pub fn right(&mut self) {
        if self.with_checking_lock(|bottle| bottle.right()) {
            self.events.push(GameEvent::Move);
        }
    }

    pub fn rotate(&mut self, clockwise: bool) {
        if self.with_checking_lock(|bottle| bottle.rotate(clockwise)) {
            self.events.push(GameEvent::Rotate);
        }
    }

    pub fn send_garbage(&mut self, garbage: SendGarbage) {
        self.garbage_buffer.push(garbage);
    }

    pub fn attack(garbage: &SendGarbage) -> Attack {
        let blocks = garbage.len() as u32;
        Attack::new(GAME_ID, blocks)
            .with_foreign_for(ids::RUSTRIS, foreign_attack(ids::RUSTRIS, blocks))
            .with_detail(encode_garbage(garbage))
    }

    fn garbage_of(&mut self, attack: Attack) -> SendGarbage {
        if attack.origin == GAME_ID {
            decode_garbage(attack.detail)
        } else {
            // another game attacked: make up the colours
            (0..attack.strength_for(GAME_ID))
                .map(|_| self.random.random_color())
                .collect()
        }
    }

    pub fn update(&mut self, delta: Duration) {
        self.state = match &self.state {
            GameState::Spawn(duration) => self.next_spawn(*duration + delta),
            GameState::SpawnHold(Some(shape)) => self.spawn_shape(*shape, true),
            GameState::SpawnHold(None) => {
                let shape = self.random.next_pill();
                self.spawn_shape(shape, false)
            }
            GameState::Fall(duration) => self.next_fall(*duration + delta),
            GameState::Lock(duration) => self.next_lock(*duration + delta),
            GameState::Pattern(combo) => self.next_pattern(combo.clone()),
            GameState::Destroy(blocks, combo) => self.next_destroy(blocks.clone(), combo.clone()),
            GameState::GameOver => GameState::GameOver,
            GameState::DropGarbage(duration, combo) => {
                self.next_drop_garbage(*duration + delta, combo.clone())
            }
            GameState::LevelComplete => GameState::LevelComplete,
        };
    }

    pub fn consume_events(&mut self, target: &mut Vec<GameEvent>) {
        for event in self.events.iter().cloned() {
            target.push(event);
        }
        self.events.clear();
    }

    fn next_spawn(&mut self, duration: Duration) -> GameState {
        if let Some(next_garbage) = self.garbage_buffer.pop() {
            let garbage = self.bottle.send_garbage(next_garbage);
            self.events.push(GameEvent::AttackReceived {
                cells: garbage.into_iter().map(Into::into).collect(),
            });
            return GameState::drop_garbage(Combo::empty());
        }

        if !self.hard_dropped && duration < self.spawn_delay() {
            return GameState::Spawn(duration);
        }
        self.hard_dropped = false;
        let shape = self.random.next_pill();
        self.spawn_shape(shape, false)
    }

    fn spawn_shape(&mut self, shape: PillShape, is_hold: bool) -> GameState {
        if let Some(vitamins) = self.bottle.try_spawn(shape) {
            self.events.push(GameEvent::Spawn {
                piece: shape.into(),
                cells: placed_vitamins(vitamins),
                is_hold,
            });
            self.total_pills += 1;
            if self.total_pills % PILLS_PER_SPEED_LEVEL == 0 {
                self.events.push(GameEvent::SpeedUp);
            }
            GameState::NEW_FALL
        } else {
            // cannot spawn a pill is a game over event
            self.events.push(GameEvent::GameOver);
            GameState::GameOver
        }
    }

    fn next_fall(&mut self, duration: Duration) -> GameState {
        if duration < self.step_delay() {
            return GameState::Fall(duration);
        }

        if !self.bottle.step_down_pill() {
            // cannot step down, start lock
            return GameState::NEW_LOCK;
        }

        self.events.push(GameEvent::Fall);
        if self.bottle.is_collision() {
            // step has caused a collision, start a lock
            if self.bottle.lock_placements() >= TIMING.max_lock_placements {
                GameState::LOCK_NOW
            } else {
                GameState::NEW_LOCK
            }
        } else {
            // no collisions, start a new fall step
            GameState::NEW_FALL
        }
    }

    fn next_lock(&mut self, duration: Duration) -> GameState {
        if !self.hard_dropped && duration < TIMING.lock_duration(self.soft_drop) {
            GameState::Lock(duration)
        } else if self.bottle.is_collision() {
            // lock timeout and still colliding so lock the piece now
            // but before locking, need to check for a game over event.
            let vitamins = self.bottle.lock().expect("we must've locked");

            HoldState::unlock(&mut self.hold);

            self.events.push(GameEvent::Lock {
                cells: placed_vitamins(vitamins),
                dropped: self.hard_dropped || self.soft_drop,
            });
            GameState::NEW_PATTERN
        } else {
            // otherwise must've moved over empty space so start a new fall
            GameState::NEW_FALL
        }
    }

    fn next_pattern(&mut self, combo: Combo) -> GameState {
        let (blocks, patterns) = self.bottle.pattern();
        if !blocks.is_empty() {
            let viruses = blocks.iter().filter(|b| b.is_virus).count() as u32;
            return GameState::Destroy(blocks, combo.into_updated(patterns, viruses));
        }

        // combo over so update the score
        self.score = (self.score + combo.score(self.speed)).min(MAX_SCORE);
        let garbage = combo.garbage();
        if !garbage.is_empty() {
            self.events
                .push(GameEvent::AttackSent(Self::attack(&garbage)));
        }

        GameState::NEW_SPAWN
    }

    fn next_destroy(&mut self, blocks: Vec<ColoredBlock>, combo: Combo) -> GameState {
        self.bottle.destroy(blocks.clone());
        self.events.push(GameEvent::Clear {
            cells: blocks.into_iter().map(Into::into).collect(),
            count: combo.patterns.len() as u32,
            is_combo: combo.is_combo(),
            // Dr. Rustario grades its clears by the count and the combo alone
            detail: 0,
        });

        if self.bottle.virus_count() == 0 {
            self.events.push(GameEvent::StageComplete);
            GameState::LevelComplete
        } else {
            GameState::drop_garbage(combo)
        }
    }

    fn next_drop_garbage(&mut self, duration: Duration, combo: Combo) -> GameState {
        if duration < GARBAGE_DROP_DURATION {
            return GameState::DropGarbage(duration, combo);
        }

        if self.bottle.step_down_garbage() {
            // garbage dropped so try again
            self.events.push(GameEvent::Settle);
            GameState::drop_garbage(combo)
        } else {
            // no garbage to drop so check for patterns
            GameState::Pattern(combo)
        }
    }

    fn with_checking_lock<F>(&mut self, f: F) -> bool
    where
        F: FnMut(&mut Bottle) -> bool,
    {
        if let GameState::Lock(lock_duration) = self.state {
            match lock_move(&TIMING, lock_duration, &mut self.bottle, f) {
                LockMove::Blocked => false,
                LockMove::Exhausted => {
                    self.state = GameState::LOCK_NOW;
                    false
                }
                LockMove::Moved { last_placement } => {
                    self.state = if last_placement {
                        GameState::LOCK_NOW
                    } else {
                        GameState::NEW_FALL
                    };
                    true
                }
            }
        } else {
            // not in lock state, pass through closure
            let mut f = f;
            f(&mut self.bottle)
        }
    }

    fn spawn_delay(&self) -> Duration {
        TIMING.spawn_delay(
            self.base_delay(),
            self.soft_drop,
            self.speed.min_drop_duration(),
        )
    }

    fn step_delay(&self) -> Duration {
        TIMING.step_delay(
            self.base_delay(),
            self.soft_drop,
            self.speed.min_drop_duration(),
        )
    }

    fn base_delay(&self) -> Duration {
        self.speed
            .duration_of_level(self.total_pills / PILLS_PER_SPEED_LEVEL)
    }
}

impl LockPlacements for Bottle {
    fn lock_placements(&self) -> u32 {
        Bottle::lock_placements(self)
    }

    fn register_lock_placement(&mut self) -> u32 {
        Bottle::register_lock_placement(self)
    }
}

impl engine::game::Game for Game {
    fn game_id(&self) -> GameId {
        GAME_ID
    }

    fn update(&mut self, delta: Duration) {
        Game::update(self, delta)
    }

    fn left(&mut self) {
        Game::left(self)
    }

    fn right(&mut self) {
        Game::right(self)
    }

    fn rotate(&mut self, clockwise: bool) {
        Game::rotate(self, clockwise)
    }

    fn set_soft_drop(&mut self, soft_drop: bool) {
        Game::set_soft_drop(self, soft_drop)
    }

    fn hard_drop(&mut self) {
        Game::hard_drop(self)
    }

    fn hold(&mut self) {
        Game::hold(self)
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    fn board_width(&self) -> u32 {
        BOTTLE_WIDTH
    }

    fn board_height(&self) -> u32 {
        BOTTLE_HEIGHT
    }

    fn visible_height(&self) -> u32 {
        BOTTLE_HEIGHT
    }

    fn cell(&self, point: BottlePoint) -> Cell {
        self.bottle.block(point).into()
    }

    fn queue(&self) -> Vec<PieceId> {
        self.random.peek().into_iter().map(Into::into).collect()
    }

    fn held(&self) -> Option<PieceId> {
        self.hold.map(|h| h.piece.into())
    }

    fn metric(&self, kind: MetricKind) -> Option<u32> {
        match kind {
            MetricKind::Score => Some(self.score),
            MetricKind::Level => Some(self.virus_level),
            MetricKind::Viruses => Some(self.bottle.virus_count()),
            MetricKind::Lines | MetricKind::Chain => None,
        }
    }

    fn score(&self) -> u32 {
        self.score
    }

    fn set_score(&mut self, score: u32) {
        self.score = score.min(MAX_SCORE);
    }

    fn speed_index(&self) -> u32 {
        self.speed as u32
    }

    /// a shared difficulty index: 0 is low, a few is medium, anything higher is high
    fn set_speed_index(&mut self, index: u32) {
        self.speed = match index {
            0 => GameSpeed::Low,
            1..=3 => GameSpeed::Medium,
            _ => GameSpeed::High,
        };
    }

    fn stage_state(&self) -> StageState {
        match self.state {
            GameState::GameOver => StageState::GameOver,
            GameState::LevelComplete => StageState::StageComplete,
            _ => StageState::Playing,
        }
    }

    fn stage_transition(&self) -> StageTransition {
        StageTransition::Interstitial
    }

    fn completed_stages(&self) -> u32 {
        self.level_count
    }

    fn set_completed_stages(&mut self, stages: u32) {
        self.level_count = stages;
    }

    fn next_stage(&mut self) -> Result<(), String> {
        self.next_level()
    }

    fn receive_attack(&mut self, attack: Attack) {
        let garbage = self.garbage_of(attack);
        if !garbage.is_empty() {
            self.send_garbage(garbage);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::pill::{Garbage, Vitamins};
    use super::random::{BottleSeed, RandomMode};
    use super::*;
    use crate::game::block::Block;
    use crate::game::geometry::BottlePoint;
    use crate::game::pill::Pill;
    use crate::game::pill::Vitamin;
    use mockall::mock;
    use mockall::predicate::*;
    use rand_chacha::ChaChaRng;

    mock! {
        pub Bottle {
            pub fn from_seed(seed: BottleSeed, rng: ChaChaRng) -> Self;
            pub fn pill(&self) -> &Pill;
            pub fn virus_count(&self) -> u32;
            pub fn viruses(&self) -> Vec<ColoredBlock>;
            pub fn row(&self, y: u32) -> &[Block];
            pub fn block(&self, point: BottlePoint) -> Block;
            pub fn left(&mut self) -> bool;
            pub fn right(&mut self) -> bool;
            pub fn rotate(&mut self, clockwise: bool) -> bool;
            pub fn hold(&mut self) -> Option<PillShape>;
            pub fn hard_drop(&mut self) -> Option<(u32, Vitamins)>;
            pub fn register_lock_placement(&mut self) -> u32;
            pub fn lock_placements(&self) -> u32;
            pub fn is_collision(&self) -> bool;
            pub fn send_garbage(&mut self, garbage: SendGarbage) -> Vec<Garbage>;
            pub fn try_spawn(&mut self, shape: PillShape) -> Option<Vitamins>;
            pub fn step_down_pill(&mut self) -> bool;
            pub fn lock(&mut self) -> Option<Vitamins>;
            pub fn pattern(&self) -> (Vec<ColoredBlock>, Vec<VirusColor>);
            pub fn destroy(&mut self, points: Vec<ColoredBlock>);
            pub fn step_down_garbage(&mut self) -> bool;
        }
    }

    #[test]
    fn left_success() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_left().return_once(|| true);
        });
        game.left();
        game.should_have_events(&[GameEvent::Move]);
    }

    #[test]
    fn left_fail() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_left().return_once(|| false);
        });
        game.left();
        game.should_have_no_events();
    }

    #[test]
    fn right_success() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_right().return_once(|| true);
        });
        game.right();
        game.should_have_events(&[GameEvent::Move]);
    }

    #[test]
    fn right_fail() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_right().return_once(|| false);
        });
        game.right();
        game.should_have_no_events();
    }

    #[test]
    fn rotate_success_when_falling() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_rotate().with(eq(true)).return_once(|_| true);
        });
        game.state = GameState::NEW_FALL;
        game.rotate(true);
        game.should_have_events(&[GameEvent::Rotate]);
    }

    #[test]
    fn rotate_success_and_reset_lock_when_locking() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_rotate().with(eq(true)).return_once(|_| true);
            bottle.expect_lock_placements().return_once(|| 0);
            bottle.expect_register_lock_placement().return_once(|| 1);
        });
        game.state = GameState::Lock(Duration::from_millis(10));
        game.rotate(true);
        game.should_have_events(&[GameEvent::Rotate]);
        assert_eq!(game.state, GameState::NEW_FALL);
    }

    #[test]
    fn rotate_success_and_lock_with_last_lock_placement() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_rotate().with(eq(true)).return_once(|_| true);
            bottle
                .expect_lock_placements()
                .return_once(|| TIMING.max_lock_placements - 1);
            bottle
                .expect_register_lock_placement()
                .return_once(|| TIMING.max_lock_placements);
        });
        game.state = GameState::Lock(Duration::from_millis(10));
        game.rotate(true);
        game.should_have_events(&[GameEvent::Rotate]);
        assert_eq!(game.state, GameState::LOCK_NOW);
    }

    #[test]
    fn rotate_fail() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_rotate().with(eq(true)).return_once(|_| false);
        });
        game.rotate(true);
        game.should_have_no_events();
    }

    #[test]
    fn rotate_fail_when_locked() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_rotate().with(eq(true)).return_once(|_| true);
        });
        game.state = GameState::LOCK_NOW;
        game.rotate(true);
        game.should_have_no_events();
        assert_eq!(game.state, GameState::LOCK_NOW);
    }

    #[test]
    fn rotate_fail_when_no_lock_placements_left() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_rotate().with(eq(true)).return_once(|_| true);
            bottle
                .expect_lock_placements()
                .return_once(|| TIMING.max_lock_placements);
        });
        game.state = GameState::NEW_LOCK;
        game.rotate(true);
        game.should_have_no_events();
        assert_eq!(game.state, GameState::LOCK_NOW);
    }

    #[test]
    fn holds_for_first_time() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_hold().return_once(|| Some(PillShape::RB));
        });
        game.hold();
        game.should_have_events(&[GameEvent::Hold]);
        assert_eq!(game.state, GameState::SpawnHold(None));
        assert_eq!(game.hold, Some(HoldState::locked(PillShape::RB)))
    }

    #[test]
    fn holds_for_second_time() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_hold().return_once(|| Some(PillShape::RB));
        });
        game.hold = Some(HoldState {
            piece: PillShape::RR,
            locked: false,
        });
        game.hold();
        game.should_have_events(&[GameEvent::Hold]);
        assert_eq!(game.state, GameState::SpawnHold(Some(PillShape::RR)));
        assert_eq!(game.hold, Some(HoldState::locked(PillShape::RB)))
    }

    #[test]
    fn cannot_hold_when_hold_locked() {
        let mut game = having_bottle(|_| {});
        game.state = GameState::NEW_FALL;
        game.hold = Some(HoldState::locked(PillShape::RB));
        game.hold();
        game.should_have_no_events();
        assert_eq!(game.state, GameState::NEW_FALL);
    }

    #[test]
    fn cannot_hold_when_bottle_rejected() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_hold().return_once(|| None);
        });
        game.state = GameState::NEW_FALL;
        game.hold();
        game.should_have_no_events();
        assert_eq!(game.state, GameState::NEW_FALL);
    }

    #[test]
    fn soft_drop_on() {
        let mut game = having_bottle(|_| {});
        game.set_soft_drop(true);
        game.should_have_events(&[GameEvent::SoftDrop]);
    }

    #[test]
    fn soft_drop_off() {
        let mut game = having_bottle(|_| {});
        game.set_soft_drop(false);
        game.should_have_no_events();
    }

    #[test]
    fn hard_drop_success() {
        let mut game = having_bottle(|bottle| {
            bottle
                .expect_hard_drop()
                .return_once(|| Some((10, Vitamin::vitamins(PillShape::RB))));
        });
        game.hard_drop();
        game.should_have_events(&[GameEvent::HardDrop {
            cells: placed_vitamins(Vitamin::vitamins(PillShape::RB)),
            dropped_rows: 10,
        }]);
        assert_eq!(game.state, GameState::LOCK_NOW);
        assert!(game.hard_dropped)
    }

    #[test]
    fn hard_drop_fail() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_hard_drop().return_once(|| None);
        });
        game.state = GameState::NEW_FALL;
        game.hard_drop();
        game.should_have_no_events();
        assert_eq!(game.state, GameState::NEW_FALL);
    }

    #[test]
    fn send_garbage() {
        let mut game = having_bottle(|_| {});
        game.send_garbage(vec![VirusColor::Red, VirusColor::Blue]);
        assert_eq!(
            game.garbage_buffer,
            vec![vec![VirusColor::Red, VirusColor::Blue]]
        );
    }

    #[test]
    fn update_spawn_into_spawn() {
        let mut game = having_bottle(|_| {});
        game.state = GameState::Spawn(Duration::from_nanos(1));
        game.update(Duration::from_nanos(2));
        assert_eq!(game.state, GameState::Spawn(Duration::from_nanos(3)));
        game.should_have_no_events();
    }

    #[test]
    fn update_spawn_into_fall() {
        let mut game = having_bottle(|bottle| {
            bottle
                .expect_try_spawn()
                .with(eq(PillShape::BR))
                .return_once(|_| Some(Vitamin::vitamins(PillShape::BR)));
        });
        game.state = GameState::Spawn(GameSpeed::Low.duration_of_level(0));
        game.update(Duration::from_nanos(1));
        assert_eq!(game.state, GameState::NEW_FALL);
        game.should_have_events(&[GameEvent::Spawn {
            piece: PillShape::BR.into(),
            cells: placed_vitamins(Vitamin::vitamins(PillShape::BR)),
            is_hold: false,
        }]);
    }

    #[test]
    fn update_hard_dropped_spawn_into_fall() {
        let mut game = having_bottle(|bottle| {
            bottle
                .expect_try_spawn()
                .with(eq(PillShape::BR))
                .return_once(|_| Some(Vitamin::vitamins(PillShape::BR)));
        });
        game.hard_dropped = true;
        game.state = GameState::NEW_SPAWN;
        game.update(Duration::from_nanos(1));
        assert_eq!(game.state, GameState::NEW_FALL);
        assert!(!game.hard_dropped);
        game.should_have_events(&[GameEvent::Spawn {
            piece: PillShape::BR.into(),
            cells: placed_vitamins(Vitamin::vitamins(PillShape::BR)),
            is_hold: false,
        }]);
    }

    #[test]
    fn update_spawn_into_game_over() {
        let mut game = having_bottle(|bottle| {
            bottle
                .expect_try_spawn()
                .with(eq(PillShape::BR))
                .return_once(|_| None);
        });
        game.state = GameState::Spawn(GameSpeed::Low.duration_of_level(0));
        game.update(Duration::from_nanos(1));
        assert_eq!(game.state, GameState::GameOver);
        game.should_have_events(&[GameEvent::GameOver]);
    }

    #[test]
    fn update_spawn_into_garbage() {
        let mut game = having_bottle(|bottle| {
            bottle
                .expect_send_garbage()
                .with(eq(vec![VirusColor::Red, VirusColor::Yellow]))
                .return_once(|_| vec![Garbage::new(VirusColor::Yellow, BottlePoint::new(1, 2))]);
        });
        game.garbage_buffer
            .push(vec![VirusColor::Red, VirusColor::Yellow]);
        game.state = GameState::NEW_SPAWN;
        game.update(Duration::from_nanos(1));
        assert_eq!(game.state, GameState::drop_garbage(Combo::empty()));
        game.should_have_events(&[GameEvent::AttackReceived {
            cells: vec![Garbage::new(VirusColor::Yellow, BottlePoint::new(1, 2)).into()],
        }]);
    }

    #[test]
    fn update_hold_spawn_into_fall() {
        let mut game = having_bottle(|bottle| {
            bottle
                .expect_try_spawn()
                .with(eq(PillShape::RB))
                .return_once(|_| Some(Vitamin::vitamins(PillShape::RB)));
        });
        game.state = GameState::SpawnHold(Some(PillShape::RB));
        game.update(Duration::from_nanos(1));
        assert_eq!(game.state, GameState::NEW_FALL);
        game.should_have_events(&[GameEvent::Spawn {
            piece: PillShape::RB.into(),
            cells: placed_vitamins(Vitamin::vitamins(PillShape::RB)),
            is_hold: true,
        }]);
    }

    #[test]
    fn update_fall_into_fall() {
        let mut game = having_bottle(|_| {});
        game.state = GameState::Fall(Duration::from_nanos(1));
        game.update(Duration::from_nanos(2));
        assert_eq!(game.state, GameState::Fall(Duration::from_nanos(3)));
        game.should_have_no_events();
    }

    #[test]
    fn update_fall_into_next_fall() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_step_down_pill().return_once(|| true);
            bottle.expect_is_collision().return_once(|| false);
        });
        game.state = GameState::Fall(GameSpeed::Low.duration_of_level(0));
        game.update(Duration::from_nanos(1));
        assert_eq!(game.state, GameState::NEW_FALL);
        game.should_have_events(&[GameEvent::Fall]);
    }

    #[test]
    fn update_fall_into_lock_by_fail() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_step_down_pill().return_once(|| false);
        });
        game.state = GameState::Fall(GameSpeed::Low.duration_of_level(0));
        game.update(Duration::from_nanos(1));
        assert_eq!(game.state, GameState::NEW_LOCK);
        game.should_have_no_events();
    }

    #[test]
    fn update_fall_into_lock_by_collision() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_step_down_pill().return_once(|| true);
            bottle.expect_is_collision().return_once(|| true);
            bottle.expect_lock_placements().return_once(|| 0);
        });
        game.state = GameState::Fall(GameSpeed::Low.duration_of_level(0));
        game.update(Duration::from_nanos(1));
        assert_eq!(game.state, GameState::NEW_LOCK);
        game.should_have_events(&[GameEvent::Fall]);
    }

    #[test]
    fn update_fall_into_lock_asap_by_collision() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_step_down_pill().return_once(|| true);
            bottle.expect_is_collision().return_once(|| true);
            bottle
                .expect_lock_placements()
                .return_once(|| TIMING.max_lock_placements);
        });
        game.state = GameState::Fall(GameSpeed::Low.duration_of_level(0));
        game.update(Duration::from_nanos(1));
        assert_eq!(game.state, GameState::LOCK_NOW);
        game.should_have_events(&[GameEvent::Fall]);
    }

    #[test]
    fn update_lock_into_lock() {
        let mut game = having_bottle(|_| {});
        game.state = GameState::Lock(Duration::from_nanos(1));
        game.update(Duration::from_nanos(2));
        assert_eq!(game.state, GameState::Lock(Duration::from_nanos(3)));
        game.should_have_no_events();
    }

    #[test]
    fn update_lock_into_pattern() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_is_collision().return_once(|| true);
            bottle
                .expect_lock()
                .return_once(|| Some(Vitamin::vitamins(PillShape::RB)));
        });
        game.state = GameState::LOCK_NOW;
        game.update(Duration::from_nanos(1));
        assert_eq!(game.state, GameState::NEW_PATTERN);
        game.should_have_events(&[GameEvent::Lock {
            cells: placed_vitamins(Vitamin::vitamins(PillShape::RB)),
            dropped: false,
        }]);
    }

    #[test]
    fn update_hard_drop_lock_into_pattern() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_is_collision().return_once(|| true);
            bottle
                .expect_lock()
                .return_once(|| Some(Vitamin::vitamins(PillShape::RB)));
        });
        game.hard_dropped = true;
        game.state = GameState::Lock(Duration::from_nanos(1));
        game.update(Duration::from_nanos(1));
        assert_eq!(game.state, GameState::NEW_PATTERN);
        game.should_have_events(&[GameEvent::Lock {
            cells: placed_vitamins(Vitamin::vitamins(PillShape::RB)),
            dropped: true,
        }])
    }

    #[test]
    fn update_lock_into_fall() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_is_collision().return_once(|| false);
        });
        game.state = GameState::LOCK_NOW;
        game.update(Duration::from_nanos(1));
        assert_eq!(game.state, GameState::NEW_FALL);
        game.should_have_no_events();
    }

    #[test]
    fn update_pattern_into_destroy() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_pattern().return_once(|| {
                (
                    vec![ColoredBlock::virus(1, 2, VirusColor::Yellow)],
                    vec![VirusColor::Yellow],
                )
            });
        });
        game.state = GameState::Pattern(Combo::new(vec![VirusColor::Blue], 0));
        game.update(Duration::from_nanos(1));
        assert_eq!(
            game.state,
            GameState::Destroy(
                vec![ColoredBlock::virus(1, 2, VirusColor::Yellow)],
                Combo::new(vec![VirusColor::Blue, VirusColor::Yellow], 1)
            )
        );
        game.should_have_no_events();
    }

    #[test]
    fn update_pattern_into_spawn() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_pattern().return_once(|| (vec![], vec![]));
            bottle.expect_virus_count().return_once(|| 1);
        });
        game.state = GameState::NEW_PATTERN;
        game.update(Duration::from_nanos(1));
        assert_eq!(game.state, GameState::NEW_SPAWN);
        game.should_have_no_events();
    }

    #[test]
    fn update_pattern_into_spawn_with_garbage() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_pattern().return_once(|| (vec![], vec![]));
            bottle.expect_virus_count().return_once(|| 1);
        });
        game.state = GameState::Pattern(Combo::new(vec![VirusColor::Blue, VirusColor::Red], 2));
        game.update(Duration::from_nanos(1));
        assert_eq!(game.state, GameState::NEW_SPAWN);
        assert_eq!(game.score, 300);
        game.should_have_events(&[GameEvent::AttackSent(Game::attack(&vec![
            VirusColor::Blue,
            VirusColor::Red,
        ]))]);
    }

    #[test]
    fn a_combo_crosses_to_the_other_game_a_row_smaller() {
        // the two pattern combo a pill finishes most often buys one row abroad, not two
        for (blocks, expected) in [(0, 0), (1, 0), (2, 1), (3, 2), (4, 3), (8, 4)] {
            let garbage: SendGarbage = vec![VirusColor::Blue; blocks];
            let attack = Game::attack(&garbage);
            assert_eq!(attack.strength, blocks as u32, "{blocks} blocks");
            assert_eq!(
                attack.strength_for(ids::RUSTRIS),
                expected,
                "{blocks} blocks to Rustris"
            );
        }
    }

    #[test]
    fn a_foreign_attack_lands_in_the_receivers_own_units() {
        let mut game = having_bottle(|_| {});
        let colors = vec![VirusColor::Blue, VirusColor::Red];
        // another Dr. Rustario player sends its own blocks, colours and all
        assert_eq!(game.garbage_of(Game::attack(&colors)), colors);
        // another game sends what it says it is worth here, in made up colours
        assert_eq!(
            game.garbage_of(Attack::new(GameId(u16::MAX), 8).with_foreign_for(GAME_ID, 2))
                .len(),
            2
        );
    }

    #[test]
    fn update_destroy_into_drop_garbage() {
        let mut game = having_bottle(|bottle| {
            bottle
                .expect_destroy()
                .with(eq(vec![ColoredBlock::virus(1, 2, VirusColor::Yellow)]))
                .return_once(|_| ());
            bottle
                .expect_block()
                .with(eq(BottlePoint::new(1, 2)))
                .return_once(|_| Block::Garbage(VirusColor::Yellow));
            bottle.expect_virus_count().return_once(|| 1);
        });
        game.state = GameState::Destroy(
            vec![ColoredBlock::virus(1, 2, VirusColor::Yellow)],
            Combo::new(vec![VirusColor::Blue], 2),
        );
        game.update(Duration::from_nanos(1));
        assert_eq!(
            game.state,
            GameState::drop_garbage(Combo::new(vec![VirusColor::Blue], 2))
        );
        game.should_have_events(&[GameEvent::Clear {
            cells: vec![ColoredBlock::virus(1, 2, VirusColor::Yellow).into()],
            count: 1,
            is_combo: false,
            detail: 0,
        }]);
    }

    #[test]
    fn update_destroy_into_drop_garbage_with_combo() {
        let mut game = having_bottle(|bottle| {
            bottle
                .expect_destroy()
                .with(eq(vec![ColoredBlock::virus(1, 2, VirusColor::Yellow)]))
                .return_once(|_| ());
            bottle
                .expect_block()
                .with(eq(BottlePoint::new(1, 2)))
                .return_once(|_| Block::Garbage(VirusColor::Yellow));
            bottle.expect_virus_count().return_once(|| 1);
        });
        let combo = Combo::new(vec![VirusColor::Red, VirusColor::Blue], 1);
        game.state = GameState::Destroy(
            vec![ColoredBlock::virus(1, 2, VirusColor::Yellow)],
            combo.clone(),
        );
        game.update(Duration::from_nanos(1));
        assert_eq!(game.state, GameState::drop_garbage(combo));
        game.should_have_events(&[GameEvent::Clear {
            cells: vec![ColoredBlock::virus(1, 2, VirusColor::Yellow).into()],
            count: 2,
            is_combo: true,
            detail: 0,
        }]);
    }

    #[test]
    fn update_drop_garbage_into_drop_garbage() {
        let mut game = having_bottle(|_| {});
        let combo = Combo::new(vec![VirusColor::Blue], 2);
        game.state = GameState::DropGarbage(Duration::from_nanos(2), combo.clone());
        game.update(Duration::from_nanos(1));
        assert_eq!(
            game.state,
            GameState::DropGarbage(Duration::from_nanos(3), combo)
        );
        game.should_have_no_events();
    }

    #[test]
    fn update_drop_garbage_into_next_drop_garbage() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_step_down_garbage().return_once(|| true);
        });
        let combo = Combo::new(vec![VirusColor::Blue], 2);
        game.state = GameState::DropGarbage(GARBAGE_DROP_DURATION, combo.clone());
        game.update(Duration::from_nanos(1));
        assert_eq!(game.state, GameState::DropGarbage(Duration::ZERO, combo));
        game.should_have_events(&[GameEvent::Settle])
    }

    #[test]
    fn update_drop_garbage_into_pattern() {
        let mut game = having_bottle(|bottle| {
            bottle.expect_step_down_garbage().return_once(|| false);
        });
        let combo = Combo::new(vec![VirusColor::Blue], 2);
        game.state = GameState::DropGarbage(GARBAGE_DROP_DURATION, combo.clone());
        game.update(Duration::from_nanos(1));
        assert_eq!(game.state, GameState::Pattern(combo));
        game.should_have_no_events();
    }

    #[test]
    fn score_0_when_empty() {
        assert_eq!(Combo::empty().score(GameSpeed::Low), 0);
    }

    #[test]
    fn score_low() {
        let score = Combo::new(vec![VirusColor::Blue], 1).score(GameSpeed::Low);
        assert_eq!(score, 100);
    }

    #[test]
    fn score_combo_low() {
        let score = Combo::new(vec![VirusColor::Blue, VirusColor::Red], 2).score(GameSpeed::Low);
        assert_eq!(score, 100 + 200);
    }

    #[test]
    fn score_medium() {
        let score = Combo::new(vec![VirusColor::Blue], 1).score(GameSpeed::Medium);
        assert_eq!(score, 200);
    }

    #[test]
    fn score_combo_medium() {
        let score = Combo::new(vec![VirusColor::Blue, VirusColor::Red], 3).score(GameSpeed::Medium);
        assert_eq!(score, 200 + 400 + 800);
    }

    #[test]
    fn score_high() {
        let score = Combo::new(vec![VirusColor::Blue], 1).score(GameSpeed::High);
        assert_eq!(score, 300);
    }

    #[test]
    fn score_combo_high() {
        let score = Combo::new(vec![VirusColor::Blue, VirusColor::Red], 4).score(GameSpeed::High);
        assert_eq!(score, 300 + 600 + 1200 + 2400);
    }

    fn having_bottle<F>(mut f: F) -> Game
    where
        F: FnMut(&mut MockBottle),
    {
        let mut bottle = MockBottle::new();
        f(&mut bottle);
        Game::from_bottle(
            10,
            GameSpeed::Low,
            GameRandom::from_u64_seed(12345, RandomMode::Bag),
            bottle,
        )
    }

    trait GameTestHarness {
        fn should_have_no_events(&self);
        fn should_have_events(&self, events: &[GameEvent]);
    }

    impl GameTestHarness for Game {
        fn should_have_no_events(&self) {
            assert!(self.events.is_empty(), "{:?}", self.events);
        }

        fn should_have_events(&self, events: &[GameEvent]) {
            assert_eq!(self.events, events.to_vec());
        }
    }
}
