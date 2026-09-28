//! Puyo Puyo Tsu's rules, simulated headlessly.

pub mod ai;
pub mod board;
pub mod cell;
pub mod nuisance;
pub mod pair;
pub mod random;
pub mod rules;
pub mod score;

use crate::game::board::{Board, ChainStep, COLUMNS, ROWS, SPAWN, VISIBLE_ROWS};
use crate::game::cell::{PuyoCell, PuyoSkin};
use crate::game::nuisance::Nuisance;
use crate::game::pair::Pair;
use crate::game::random::GameRandom;
use crate::game::rules::Difficulty;
use crate::game::score::step_score;
use engine::game::geometry::Point;
use engine::game::{
    ids, Attack, Cell, CellId, GameEvent, GameId, MetricKind, PieceId, PlacedCell, StageState,
    StageTransition,
};
use std::time::Duration;

pub use crate::game::cell::GAME_ID;

/// how many puyos have to go in one step before the field calls it a big clear
pub const BIG_CLEAR_PUYOS: u32 = 8;

/// the chain length that earns the background field's `CHAIN`
pub const LONG_CHAIN: u32 = 4;

/// What a [`GameEvent::Clear`] carries in its game-private `detail`, read back by the renderer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClearDetail {
    /// which step of the chain this was, counting from 1
    pub chain: u32,
    /// this step left the board empty
    pub all_clear: bool,
}

const DETAIL_CHAIN: u64 = 0xffff;
const DETAIL_ALL_CLEAR: u64 = 1 << 16;

impl From<ClearDetail> for u64 {
    fn from(detail: ClearDetail) -> Self {
        (detail.chain as u64 & DETAIL_CHAIN)
            | if detail.all_clear {
                DETAIL_ALL_CLEAR
            } else {
                0
            }
    }
}

impl From<u64> for ClearDetail {
    fn from(detail: u64) -> Self {
        Self {
            chain: (detail & DETAIL_CHAIN) as u32,
            all_clear: detail & DETAIL_ALL_CLEAR != 0,
        }
    }
}

/// Where the game is in a placement: a pair falls and locks, the chain resolves a settle and a
/// pop at a time, and then the queue drops, already offset against the chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// nothing in play; the next pair is about to appear
    Spawning(Duration),
    /// the player has the pair
    Falling {
        lock: Duration,
    },
    /// the chain loop: settle, pop, settle, pop, until nothing more goes
    Resolving {
        chain: u32,
        timer: Duration,
    },
    /// the queue emptying onto the board
    Dropping(Duration),
    GameOver,
}

pub struct Game {
    board: Board,
    random: GameRandom,
    queue: Nuisance,
    pair: Option<Pair>,
    state: State,
    events: Vec<GameEvent>,
    score: u32,
    speed_index: u32,
    completed_stages: u32,
    stage_complete: bool,
    /// puyos cleared towards the next speed step
    stage_puyos: u32,
    /// the longest chain this game has managed, for the HUD
    max_chain: u32,
    /// what the chain being resolved has scored so far
    chain_score: u32,
    soft_drop: bool,
    fall: Duration,
    /// which sprite set this player's cells are drawn from, carried on every id it reports
    skin: PuyoSkin,
}

impl Game {
    /// `skin` is the player's sprite set; it reaches every cell id and nothing in the rules.
    pub fn new(
        difficulty: Difficulty,
        speed_index: u32,
        random: GameRandom,
        skin: PuyoSkin,
    ) -> Self {
        let mut queue = Nuisance::new(random.seed());
        let mut board = Board::new(skin);
        let rows = difficulty.starting_nuisance_rows();
        if rows > 0 {
            queue.drop_onto(&mut board, rows * nuisance::ROW, skin);
        }
        let mut game = Self {
            board,
            random,
            queue,
            pair: None,
            state: State::Spawning(Duration::ZERO),
            events: vec![],
            score: 0,
            speed_index: speed_index + difficulty.speed_bonus(),
            completed_stages: 0,
            stage_complete: false,
            stage_puyos: 0,
            max_chain: 0,
            chain_score: 0,
            soft_drop: false,
            fall: Duration::ZERO,
            skin,
        };
        game.spawn();
        game
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn pending_nuisance(&self) -> u32 {
        self.queue.pending()
    }

    pub fn max_chain(&self) -> u32 {
        self.max_chain
    }

    /// the pair in play, if the player has one
    pub fn pair(&self) -> Option<Pair> {
        self.pair
    }

    fn fall_interval(&self) -> Duration {
        let delay = rules::fall_delay(self.speed_index);
        if self.soft_drop {
            delay.min(rules::SOFT_DROP_DELAY)
        } else {
            delay
        }
    }

    fn spawn(&mut self) {
        let piece = self.random.next_pair();
        let pair = Pair::new(SPAWN, piece);
        self.events.push(GameEvent::Spawn {
            piece: piece.id(self.skin),
            cells: pair.cells(self.skin),
            is_hold: false,
        });
        self.events.push(GameEvent::Spawned);
        self.pair = Some(pair);
        self.fall = Duration::ZERO;
        self.state = State::Falling {
            lock: Duration::ZERO,
        };
    }

    /// put the pair down and start the chain loop
    fn lock_pair(&mut self, dropped: bool) {
        let Some(pair) = self.pair.take() else { return };
        let cells = pair.cells(self.skin);
        pair.lock(&mut self.board);
        // the half already resting has landed; the other reports itself from the settle below
        let landed: Vec<_> = cells
            .iter()
            .copied()
            .filter(|(at, _)| self.board.is_supported(*at))
            .collect();
        self.events.push(GameEvent::Lock { cells, dropped });
        if !landed.is_empty() {
            self.events.push(GameEvent::Landed { cells: landed });
        }
        self.chain_score = 0;
        self.state = State::Resolving {
            chain: 0,
            timer: rules::SETTLE_DELAY,
        };
    }

    /// what the board holds at each of `points`, as the engine's own placed cells
    fn placed(&self, points: &[Point]) -> Vec<PlacedCell> {
        points
            .iter()
            .filter_map(|at| self.board.get(*at).map(|cell| (*at, cell.id(self.skin))))
            .collect()
    }

    /// One turn of the chain loop: let gravity finish, then pop whatever is ready, so a pair
    /// splits and each step lands before the next is looked for.
    fn resolve(&mut self, chain: u32) {
        let settled = self.board.settle();
        if !settled.is_empty() {
            self.events.push(GameEvent::Settle);
            self.events.push(GameEvent::Landed {
                cells: self.placed(&settled),
            });
            self.state = State::Resolving {
                chain,
                timer: rules::SETTLE_DELAY,
            };
            return;
        }
        match self.board.pop() {
            Some(step) => self.pop_step(chain + 1, step),
            None => self.finish_chain(chain),
        }
    }

    fn pop_step(&mut self, chain: u32, step: ChainStep) {
        let scored = step_score(chain, &step.groups);
        self.score = self.score.saturating_add(scored);
        self.chain_score = self.chain_score.saturating_add(scored);
        self.stage_puyos += step.count();

        self.max_chain = self.max_chain.max(chain);
        let detail = ClearDetail {
            chain,
            all_clear: self.board.is_all_clear(),
        };
        let count = step.count();
        self.events.push(GameEvent::Clear {
            cells: step.cells,
            count,
            // false on a chain's first step and true after, the grammar Dr. Rustario's combos use
            is_combo: chain > 1,
            detail: detail.into(),
        });
        self.state = State::Resolving {
            chain,
            timer: rules::POP_DELAY,
        };
    }

    /// The chain is over: settle up with the tray, then let whatever still waits fall.
    fn finish_chain(&mut self, chain: u32) {
        if chain > 0 {
            // resolve before earning: the all clear pays out on the next chain, not this one
            let all_clear = self.board.is_all_clear();
            let outgoing = self.queue.resolve(self.chain_score);
            if all_clear {
                self.queue.earn_all_clear();
            }
            if outgoing.sent > 0 {
                let sent = outgoing.sent;
                self.events.push(GameEvent::AttackSent(
                    Attack::new(GAME_ID, sent)
                        .with_foreign_for(ids::DR_RUSTARIO, foreign_attack(ids::DR_RUSTARIO, sent))
                        .with_foreign_for(ids::RUSTRIS, foreign_attack(ids::RUSTRIS, sent)),
                ));
            }
            // one event per stage the chain paid for, since a big chain can clear several
            while self.stage_puyos >= rules::PUYOS_PER_STAGE {
                self.stage_puyos -= rules::PUYOS_PER_STAGE;
                self.stage_complete = true;
                self.events.push(GameEvent::StageComplete);
            }
        }
        self.chain_score = 0;

        let dropping = self.queue.take_drop();
        if dropping > 0 {
            let cells = self.queue.drop_onto(&mut self.board, dropping, self.skin);
            self.events.push(GameEvent::AttackReceived { cells });
            self.state = State::Dropping(rules::NUISANCE_DELAY);
        } else {
            self.next_pair();
        }
    }

    /// the game ends only once something rests on the death square
    fn next_pair(&mut self) {
        if self.board.is_dead() {
            self.events.push(GameEvent::GameOver);
            self.state = State::GameOver;
        } else {
            self.state = State::Spawning(rules::SPAWN_DELAY);
        }
    }

    fn tick_falling(&mut self, delta: Duration, mut lock: Duration) {
        let Some(mut pair) = self.pair else { return };
        let interval = self.fall_interval();
        self.fall += delta;
        while self.fall >= interval {
            self.fall -= interval;
            if pair.fall(&self.board) {
                self.events.push(GameEvent::Fall);
                if self.soft_drop {
                    self.events.push(GameEvent::SoftDrop);
                }
            } else {
                self.fall = Duration::ZERO;
                break;
            }
        }
        self.pair = Some(pair);

        if pair.is_resting(&self.board) {
            lock += delta;
            if lock >= rules::LOCK_DELAY {
                self.lock_pair(false);
                return;
            }
        } else {
            lock = Duration::ZERO;
        }
        self.state = State::Falling { lock };
    }

    /// run the pair through a closure, keeping the lock delay alive while it is nudged about
    fn with_pair(&mut self, f: impl FnOnce(&mut Pair, &Board) -> bool) -> bool {
        if !matches!(self.state, State::Falling { .. }) {
            return false;
        }
        let Some(mut pair) = self.pair else {
            return false;
        };
        let moved = f(&mut pair, &self.board);
        if moved {
            self.pair = Some(pair);
            if let State::Falling { lock } = &mut self.state {
                *lock = Duration::ZERO;
            }
        } else {
            // a refused rotation is still stored, for the quick turn
            self.pair = Some(pair);
        }
        moved
    }
}

impl engine::game::Game for Game {
    fn game_id(&self) -> GameId {
        GAME_ID
    }

    fn update(&mut self, delta: Duration) {
        match self.state {
            State::Spawning(left) => {
                if let Some(left) = left.checked_sub(delta).filter(|d| !d.is_zero()) {
                    self.state = State::Spawning(left);
                } else {
                    self.spawn();
                }
            }
            State::Falling { lock } => self.tick_falling(delta, lock),
            State::Resolving { chain, timer } => {
                if let Some(timer) = timer.checked_sub(delta).filter(|d| !d.is_zero()) {
                    self.state = State::Resolving { chain, timer };
                } else {
                    self.resolve(chain);
                }
            }
            State::Dropping(left) => {
                if let Some(left) = left.checked_sub(delta).filter(|d| !d.is_zero()) {
                    self.state = State::Dropping(left);
                } else {
                    self.next_pair();
                }
            }
            State::GameOver => {}
        }
    }

    fn left(&mut self) {
        if self.with_pair(|pair, board| pair.shift(board, -1)) {
            self.events.push(GameEvent::Move);
        }
    }

    fn right(&mut self) {
        if self.with_pair(|pair, board| pair.shift(board, 1)) {
            self.events.push(GameEvent::Move);
        }
    }

    fn rotate(&mut self, clockwise: bool) {
        use crate::game::pair::RotateOutcome;
        if self.with_pair(|pair, board| pair.rotate(board, clockwise) != RotateOutcome::Blocked) {
            self.events.push(GameEvent::Rotate);
        }
    }

    /// Toggling soft drop carries the pair's position across the change but not the time banked
    /// towards the next row, or a gravity bank would be spent several rows at once at the soft
    /// drop rate.
    fn set_soft_drop(&mut self, soft_drop: bool) {
        if soft_drop == self.soft_drop {
            return;
        }
        let travelled = self.fall.as_secs_f64() / self.fall_interval().as_secs_f64();
        self.soft_drop = soft_drop;
        self.fall = self.fall_interval().mul_f64(travelled.clamp(0.0, 1.0));
    }

    /// The pair slides between cells. Zero while it is resting, since the fall timer keeps
    /// running through the lock delay and would sink a settled pair.
    fn fall_progress(&self) -> f64 {
        let Some(pair) = self.pair else { return 0.0 };
        if !matches!(self.state, State::Falling { .. }) || pair.is_resting(&self.board) {
            return 0.0;
        }
        let interval = self.fall_interval().as_secs_f64();
        if interval <= 0.0 {
            return 0.0;
        }
        (self.fall.as_secs_f64() / interval).clamp(0.0, 1.0)
    }

    fn hard_drop(&mut self) {
        if !matches!(self.state, State::Falling { .. }) {
            return;
        }
        let Some(mut pair) = self.pair else { return };
        // where it started, not where it lands: the trail smears down from these cells
        let cells = pair.cells(self.skin);
        let dropped_rows = pair.hard_drop(&self.board);
        self.pair = Some(pair);
        self.events.push(GameEvent::HardDrop {
            cells,
            dropped_rows,
        });
        self.lock_pair(true);
    }

    /// Tsu has no hold.
    fn hold(&mut self) {}

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    fn board_width(&self) -> u32 {
        COLUMNS
    }

    fn board_height(&self) -> u32 {
        ROWS
    }

    fn visible_height(&self) -> u32 {
        VISIBLE_ROWS
    }

    fn cell(&self, point: Point) -> Cell {
        if let Some(pair) = self.pair {
            for (at, id) in pair.cells(self.skin) {
                if at == point {
                    return Cell::Active(id);
                }
            }
            for (at, id) in pair.ghost(&self.board).cells(self.skin) {
                if at == point {
                    return Cell::Ghost(id);
                }
            }
        }
        match self.board.get(point) {
            None => Cell::Empty,
            // nuisance is not something the player put there
            Some(PuyoCell::Nuisance) => Cell::Garbage(PuyoCell::Nuisance.id(self.skin)),
            Some(cell) => Cell::Stack(cell.id(self.skin)),
        }
    }

    fn queue(&self) -> Vec<PieceId> {
        self.random
            .peek()
            .into_iter()
            .map(|piece| piece.id(self.skin))
            .collect()
    }

    fn held(&self) -> Option<PieceId> {
        None
    }

    fn metric(&self, kind: MetricKind) -> Option<u32> {
        match kind {
            MetricKind::Score => Some(self.score),
            // the speed step stands in for a level
            MetricKind::Level => Some(self.speed_index),
            MetricKind::Chain => Some(self.max_chain),
            MetricKind::Lines | MetricKind::Viruses => None,
        }
    }

    fn score(&self) -> u32 {
        self.score
    }

    fn set_score(&mut self, score: u32) {
        self.score = score;
    }

    fn speed_index(&self) -> u32 {
        self.speed_index
    }

    fn set_speed_index(&mut self, index: u32) {
        self.speed_index = index;
    }

    fn stage_state(&self) -> StageState {
        if matches!(self.state, State::GameOver) {
            StageState::GameOver
        } else if self.stage_complete {
            StageState::StageComplete
        } else {
            StageState::Playing
        }
    }

    /// play carries straight on into the next speed step; there is no stage clear card
    fn stage_transition(&self) -> StageTransition {
        StageTransition::Seamless
    }

    fn completed_stages(&self) -> u32 {
        self.completed_stages
    }

    fn set_completed_stages(&mut self, stages: u32) {
        self.completed_stages = stages;
    }

    fn next_stage(&mut self) -> Result<(), String> {
        self.stage_complete = false;
        self.completed_stages += 1;
        self.speed_index += 1;
        self.events.push(GameEvent::SpeedUp);
        Ok(())
    }

    /// An attack joins the tray rather than landing, whichever game it came from, so offset
    /// can answer it.
    fn receive_attack(&mut self, attack: Attack) {
        self.queue.receive(attack.strength_for(GAME_ID));
    }

    fn pending_attacks(&self) -> Vec<CellId> {
        self.queue.tray(self.skin)
    }
}

/// How much nuisance one row of Rustris garbage or one Dr. Rustario garbage block is worth:
/// two rocks.
const NUISANCE_PER_FOREIGN_UNIT: u32 = 2 * nuisance::MAX_DROP;

/// the most a single chain sends abroad, the biggest thing either receiving game can do to itself
const MAX_FOREIGN: u32 = 4;

/// What a chain worth `nuisance` puyos is worth to a player of `receiver`, in their units. The
/// price is measured by `ga cross` and set well below Puyo's own output, since what leaves here
/// lands on a player with no offset; routine two-chains cross as nothing.
pub fn foreign_attack(receiver: GameId, nuisance: u32) -> u32 {
    match receiver {
        ids::DR_RUSTARIO | ids::RUSTRIS => (nuisance / NUISANCE_PER_FOREIGN_UNIT).min(MAX_FOREIGN),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::board::tests::{board as build_board, render};
    use crate::game::cell::{LinkMask, PuyoColor};
    use crate::game::random::Seed;
    use engine::game::Game as _;

    const STEP: Duration = Duration::from_millis(16);

    fn game() -> Game {
        game_at(Difficulty::Normal)
    }

    fn game_at(difficulty: Difficulty) -> Game {
        Game::new(
            difficulty,
            0,
            GameRandom::from_seed(Seed::from_u64(42), difficulty.colors()),
            PuyoSkin::FIRST,
        )
    }

    /// a game with a board of our choosing and no pair in play
    fn game_with(rows: &[&str]) -> Game {
        let mut game = game();
        game.drain_events();
        game.board = build_board(rows);
        game.pair = None;
        game.chain_score = 0;
        game.state = State::Resolving {
            chain: 0,
            timer: Duration::ZERO,
        };
        game
    }

    /// run until the game is waiting for the player again, or `frames` have gone by
    fn run(game: &mut Game, frames: usize) {
        for _ in 0..frames {
            game.update(STEP);
        }
    }

    /// run the chain loop out, collecting the events it produced
    fn resolve(game: &mut Game) -> Vec<GameEvent> {
        let mut events = vec![];
        for _ in 0..2000 {
            game.update(STEP);
            events.extend(game.drain_events());
            if matches!(game.state, State::Falling { .. } | State::GameOver) {
                break;
            }
        }
        events
    }

    /// a pair locking with one half over a well lands only the resting half at once
    #[test]
    fn only_the_half_that_is_resting_lands_with_the_lock() {
        let floor = ROWS as i32 - 1;
        let mut game = game_with(&["......", "......", "......", "......", "......", "r....."]);
        // the pivot on the red bean's ledge, the child over the well
        let piece = game.random.next_pair();
        let mut pair = Pair::new(Point::new(0, floor - 1), piece);
        pair.rotate(&game.board, true);
        assert_eq!(
            pair.child(),
            Point::new(1, floor - 1),
            "laid flat, to the right"
        );
        game.pair = Some(pair);
        game.events.clear();
        game.lock_pair(true);

        let landed: Vec<Vec<Point>> = game
            .events
            .iter()
            .filter_map(|e| match e {
                GameEvent::Landed { cells } => Some(cells.iter().map(|(p, _)| *p).collect()),
                _ => None,
            })
            .collect();
        assert_eq!(
            landed,
            vec![vec![Point::new(0, floor - 1)]],
            "the half on the ledge landed; the one over the well is still in the air"
        );

        // ... and the other half reports itself when gravity finishes with it
        game.drain_events();
        let settled: Vec<Point> = resolve(&mut game)
            .iter()
            .filter_map(|e| match e {
                GameEvent::Landed { cells } => Some(cells.iter().map(|(p, _)| *p)),
                _ => None,
            })
            .flatten()
            .collect();
        assert_eq!(settled, vec![Point::new(1, floor)], "and then it lands too");
    }

    /// a settle names every cell it moved, at the point it came to rest on
    #[test]
    fn a_settle_says_where_everything_came_to_rest() {
        let floor = ROWS as i32 - 1;
        let mut game = game_with(&["......", "......", "......", "......", "r.....", "......"]);
        let landed: Vec<_> = resolve(&mut game)
            .iter()
            .filter_map(|e| match e {
                GameEvent::Landed { cells } => Some(cells.clone()),
                _ => None,
            })
            .flatten()
            .collect();
        assert_eq!(
            landed,
            vec![(
                Point::new(0, floor),
                PuyoCell::puyo(PuyoColor::Red, LinkMask::NONE).id(game.skin)
            )],
            "the red bean fell to the floor, and says so at the point it landed on"
        );
    }

    fn clears(events: &[GameEvent]) -> Vec<(u32, bool, ClearDetail)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Clear {
                    count,
                    is_combo,
                    detail,
                    ..
                } => Some((*count, *is_combo, ClearDetail::from(*detail))),
                _ => None,
            })
            .collect()
    }

    fn attacks(events: &[GameEvent]) -> Vec<u32> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::AttackSent(attack) => Some(attack.strength),
                _ => None,
            })
            .collect()
    }

    /// how many times a game has announced it reached the stage goal
    fn stage_completions(game: &Game) -> usize {
        game.events
            .iter()
            .filter(|e| matches!(e, GameEvent::StageComplete))
            .count()
    }

    #[test]
    fn a_new_game_puts_a_pair_on_the_board() {
        let mut game = game();
        assert!(game.pair.is_some());
        assert_eq!(game.pair.unwrap().pivot(), SPAWN);
        let events = game.drain_events();
        assert!(matches!(events[0], GameEvent::Spawn { .. }));
        assert_eq!(game.board_width(), COLUMNS);
        assert_eq!(game.board_height(), ROWS);
        assert_eq!(game.visible_height(), VISIBLE_ROWS);
    }

    #[test]
    fn the_queue_shows_the_next_pairs_and_there_is_no_hold() {
        let game = game();
        assert_eq!(game.queue().len(), random::PEEK_SIZE);
        assert_eq!(game.held(), None, "Tsu has no hold");
    }

    #[test]
    fn hold_does_nothing_at_all() {
        let mut game = game();
        let before = game.pair;
        game.hold();
        assert_eq!(game.pair, before);
        assert!(game
            .drain_events()
            .iter()
            .all(|e| !matches!(e, GameEvent::Hold)));
    }

    #[test]
    fn a_hard_drop_locks_the_pair_at_the_bottom() {
        let mut game = game();
        game.drain_events();
        game.hard_drop();
        let events = game.drain_events();
        assert!(events
            .iter()
            .any(|e| matches!(e, GameEvent::HardDrop { .. })));
        assert!(events.iter().any(|e| matches!(e, GameEvent::Lock { .. })));
        assert!(game.pair.is_none());
        // both halves are on the board, stacked in the spawn column
        resolve(&mut game);
        assert_eq!(game.board.height(SPAWN.x), 2);
    }

    /// the trail's cells are where the pair started, not where it landed
    #[test]
    fn a_hard_drop_reports_where_the_pair_fell_from() {
        let mut game = game();
        let before = game.pair.expect("a pair").cells(PuyoSkin::FIRST);
        game.drain_events();
        game.hard_drop();
        let hard_drop = game
            .drain_events()
            .into_iter()
            .find_map(|e| match e {
                GameEvent::HardDrop {
                    cells,
                    dropped_rows,
                } => Some((cells, dropped_rows)),
                _ => None,
            })
            .expect("a hard drop");
        assert_eq!(hard_drop.0, before);
        assert!(hard_drop.1 > 0);
    }

    /// a placement that clears nothing hands over to the next pair
    #[test]
    fn a_placement_that_clears_nothing_brings_the_next_pair() {
        let mut game = game();
        game.hard_drop();
        let events = resolve(&mut game);
        assert!(clears(&events).is_empty());
        assert!(game.pair.is_some(), "a new pair is in play");
    }

    /// one Clear per step, `is_combo` false on the first and true after, a Settle between
    #[test]
    fn a_chain_reports_one_clear_per_step() {
        let mut game = game_with(&[
            "b.....", "b.....", "b.....", "r.....", "r.....", "r.....", "r.....", "b.....",
        ]);
        let events = resolve(&mut game);
        let clears = clears(&events);
        assert_eq!(clears.len(), 2, "a two chain is two clears");
        assert_eq!(
            clears[0],
            (
                4,
                false,
                ClearDetail {
                    chain: 1,
                    all_clear: false
                }
            )
        );
        assert!(clears[1].1, "the second step is a combo");
        assert_eq!(clears[1].2.chain, 2);
        assert!(clears[1].2.all_clear, "and it emptied the board");

        // a Settle separates the two pops
        let kinds: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                GameEvent::Clear { .. } => Some("clear"),
                GameEvent::Settle => Some("settle"),
                _ => None,
            })
            .collect();
        let first = kinds.iter().position(|k| *k == "clear").unwrap();
        let second = kinds.iter().rposition(|k| *k == "clear").unwrap();
        assert!(
            kinds[first + 1..second].contains(&"settle"),
            "no settle between the two steps: {kinds:?}"
        );
    }

    #[test]
    fn a_chain_scores_the_documented_points() {
        let mut game = game_with(&[
            "b.....", "b.....", "b.....", "r.....", "r.....", "r.....", "r.....", "b.....",
        ]);
        resolve(&mut game);
        // a two chain of four-links: 40 + 320
        assert_eq!(game.score(), 360);
        assert_eq!(game.max_chain(), 2);
        assert_eq!(game.metric(MetricKind::Chain), Some(2));
    }

    /// ... and the nuisance that buys, through the tray
    #[test]
    fn a_chain_sends_the_nuisance_its_score_buys() {
        let mut game = game_with(&[
            "b.....", "b.....", "b.....", "r.....", "r.....", "r.....", "r.....", "b.....",
        ]);
        let events = resolve(&mut game);
        // 360 points is five nuisance, and an all clear is earned but not yet spent
        assert_eq!(attacks(&events), vec![5]);
        assert!(game.queue.all_clear_owed());
    }

    /// a chain cancels the tray before it sends anything
    #[test]
    fn a_chain_answers_what_is_waiting_before_it_attacks() {
        let mut game = game_with(&["rrrr.."]);
        game.receive_attack(Attack::new(GAME_ID, 3));
        assert_eq!(game.pending_nuisance(), 3);
        // a single group of four is 40 points, not yet a whole nuisance puyo
        let events = resolve(&mut game);
        assert!(attacks(&events).is_empty(), "nothing crossed");
        // it could not answer at all, so all three land as the chain finishes
        assert_eq!(game.pending_nuisance(), 0, "the tray emptied");
        assert_eq!(game.board.occupied(), 3, "onto the board");
    }

    #[test]
    fn a_big_enough_chain_cancels_the_tray_outright() {
        let mut game = game_with(&[
            "b.....", "b.....", "b.....", "r.....", "r.....", "r.....", "r.....", "b.....",
        ]);
        game.receive_attack(Attack::new(GAME_ID, 3));
        let events = resolve(&mut game);
        // five nuisance: three cancel what was waiting, two cross
        assert_eq!(attacks(&events), vec![2]);
        assert_eq!(game.pending_nuisance(), 0);
    }

    /// whatever still waits falls as soon as the chain finishes
    #[test]
    fn what_is_not_answered_drops_when_the_chain_finishes() {
        let mut game = game_with(&["rrrr.."]);
        game.receive_attack(Attack::new(GAME_ID, 6));
        let events = resolve(&mut game);
        assert!(events
            .iter()
            .any(|e| matches!(e, GameEvent::AttackReceived { .. })));
        assert_eq!(
            game.pending_nuisance(),
            0,
            "the tray emptied onto the board"
        );
        assert_eq!(game.board.occupied(), 6, "a whole row of it");
    }

    /// garbage falls at the end of the turn whether or not you chained, per Puyo Nexus *Tsu (rule)*
    #[test]
    fn what_is_waiting_falls_even_after_a_placement_that_clears_nothing() {
        let mut game = game_with(&["r....."]);
        game.receive_attack(Attack::new(GAME_ID, 6));
        let events = resolve(&mut game);
        assert!(clears(&events).is_empty(), "nothing popped");
        assert!(events
            .iter()
            .any(|e| matches!(e, GameEvent::AttackReceived { .. })));
        assert_eq!(game.pending_nuisance(), 0, "and it all landed anyway");
        assert_eq!(game.board.occupied(), 7, "a row of it on top of the red");
    }

    /// no more than a rock lands in one turn, and the rest waits for the next
    #[test]
    fn only_five_rows_of_an_attack_land_in_one_turn() {
        let mut game = game_with(&["......"]);
        game.receive_attack(Attack::new(GAME_ID, 40));
        resolve(&mut game);
        assert_eq!(game.board.occupied(), nuisance::MAX_DROP);
        assert_eq!(game.pending_nuisance(), 10, "the rest still hangs over you");

        // ... and the next placement takes the remainder
        game.chain_score = 0;
        game.state = State::Resolving {
            chain: 0,
            timer: Duration::ZERO,
        };
        resolve(&mut game);
        assert_eq!(game.board.occupied(), nuisance::MAX_DROP + 10);
        assert_eq!(game.pending_nuisance(), 0);
    }

    /// the lock recomputes link masks, since a pair landing flat triggers nothing else that would
    #[test]
    fn the_puyos_a_lock_lays_down_are_joined_to_what_they_land_beside() {
        use crate::game::cell::LinkMask;
        let mut game = game_with(&["r....."]);
        game.pair = Some(pair::Pair::new(
            Point::new(0, 1),
            cell::PuyoPiece::new(PuyoColor::Red, PuyoColor::Blue),
        ));
        game.state = State::Falling {
            lock: Duration::ZERO,
        };
        game.hard_drop();
        resolve(&mut game);

        let floor = ROWS as i32 - 1;
        let links = |y: i32| game.board.get(Point::new(0, y)).unwrap().links();
        assert_eq!(links(floor), LinkMask::UP, "the red on the floor joined up");
        assert_eq!(links(floor - 1), LinkMask::DOWN, "and the red that landed");
        assert_eq!(links(floor - 2), LinkMask::NONE, "the blue joined nothing");
    }

    /// a falling pair is drawn between cells, and sits still once resting
    #[test]
    fn a_falling_pair_slides_between_cells_and_settles_still() {
        let mut game = game();
        game.set_soft_drop(true);
        let mut seen = vec![];
        for _ in 0..400 {
            if game.pair.is_none_or(|p| p.is_resting(&game.board)) {
                break;
            }
            game.update(STEP);
            seen.push(game.fall_progress());
        }
        assert!(
            seen.iter().all(|p| (0.0..=1.0).contains(p)),
            "a pair was slid outside its own cell: {seen:?}"
        );
        assert!(
            seen.iter().any(|p| *p > 0.0),
            "the pair stepped whole cells rather than sliding: {seen:?}"
        );

        assert!(game.pair.is_some_and(|p| p.is_resting(&game.board)));
        // twenty five frames of lock delay, with the fall timer still running
        for frame in 0..25 {
            assert_eq!(
                game.fall_progress(),
                0.0,
                "frame {frame}: a resting pair was slid off its cell"
            );
            game.update(STEP);
        }
    }

    /// a tap of soft drop moves the pair at most one row, however much gravity has banked
    #[test]
    fn a_tap_of_soft_drop_is_one_row_however_long_gravity_has_been_banking() {
        for banked in 0..48 {
            let mut game = game();
            for _ in 0..banked {
                game.update(STEP);
            }
            let before = game.pair.unwrap().pivot().y;

            game.set_soft_drop(true);
            game.update(STEP);
            game.set_soft_drop(false);

            let moved = game.pair.unwrap().pivot().y - before;
            assert!(
                moved <= 1,
                "a one frame tap after {banked} frames of gravity moved the pair {moved} rows"
            );
        }
    }

    /// holding soft drop makes a pair fall faster and changes nothing else
    #[test]
    fn soft_drop_hurries_the_pair_along() {
        let mut drifting = game();
        let start = drifting.pair.unwrap().pivot().y;
        run(&mut drifting, 20);
        let drifted = drifting.pair.unwrap().pivot().y - start;

        let mut hurried = game();
        hurried.set_soft_drop(true);
        run(&mut hurried, 20);
        assert!(
            hurried.pair.unwrap().pivot().y - start > drifted,
            "soft drop did not hurry it along"
        );
        assert!(
            hurried.fall_interval() < drifting.fall_interval(),
            "the soft drop step is not {:?}",
            rules::SOFT_DROP_DELAY
        );
    }

    /// an attack that arrives with nothing in play still waits rather than landing
    #[test]
    fn an_attack_waits_in_the_tray_and_is_shown_there() {
        let mut game = game();
        game.receive_attack(Attack::new(GAME_ID, 7));
        assert_eq!(game.pending_nuisance(), 7);
        assert_eq!(game.board.occupied(), 0, "nothing landed yet");
        let tray: Vec<PuyoCell> = game
            .pending_attacks()
            .into_iter()
            .map(PuyoCell::from)
            .collect();
        assert_eq!(tray.len(), 2, "a large icon and a small one");
    }

    /// an attack from an unpriced game is worth nothing here
    #[test]
    fn an_unpriced_foreign_attack_lands_as_nothing() {
        let mut game = game();
        game.receive_attack(Attack::new(GameId(u16::MAX), 8));
        assert_eq!(game.pending_nuisance(), 0);
        // ... and one that has been priced lands in its own units
        game.receive_attack(Attack::new(GameId(u16::MAX), 8).with_foreign_for(GAME_ID, 3));
        assert_eq!(game.pending_nuisance(), 3);
        // ... and a game nobody priced still gets nothing
        assert_eq!(foreign_attack(GameId(u16::MAX), 600), 0);
    }

    /// only a chain worth digging out of crosses, at two rocks a unit
    #[test]
    fn only_a_chain_worth_digging_out_of_crosses_to_another_game() {
        for receiver in [ids::RUSTRIS, ids::DR_RUSTARIO] {
            for (nuisance, expected) in [
                (0, 0),
                (12, 0),
                (nuisance::MAX_DROP, 0),
                (2 * nuisance::MAX_DROP, 1),
                (2 * nuisance::MAX_DROP - 1, 0),
                (4 * nuisance::MAX_DROP, 2),
                (8 * nuisance::MAX_DROP, 4),
                // never worth more than the receiving game can do to itself
                (100 * nuisance::MAX_DROP, MAX_FOREIGN),
            ] {
                assert_eq!(
                    foreign_attack(receiver, nuisance),
                    expected,
                    "{nuisance} nuisance to {receiver:?}"
                );
            }
        }
    }

    /// ... and a chain that crosses carries both games' units on its attack
    #[test]
    fn a_chain_prices_itself_for_both_of_the_other_games() {
        let attack = Attack::new(GAME_ID, 4 * nuisance::MAX_DROP)
            .with_foreign_for(
                ids::DR_RUSTARIO,
                foreign_attack(ids::DR_RUSTARIO, 4 * nuisance::MAX_DROP),
            )
            .with_foreign_for(
                ids::RUSTRIS,
                foreign_attack(ids::RUSTRIS, 4 * nuisance::MAX_DROP),
            );
        assert_eq!(attack.strength_for(GAME_ID), 4 * nuisance::MAX_DROP);
        assert_eq!(attack.strength_for(ids::DR_RUSTARIO), 2);
        assert_eq!(attack.strength_for(ids::RUSTRIS), 2);
    }

    /// the all clear bonus rides on the next chain, not the one that emptied the board
    #[test]
    fn an_all_clear_pays_out_on_the_following_chain() {
        let mut game = game_with(&["rrrr.."]);
        let events = resolve(&mut game);
        assert!(game.board.is_all_clear());
        assert!(attacks(&events).is_empty(), "40 points buys nothing yet");
        assert!(game.queue.all_clear_owed());

        // the next chain carries a whole rock extra
        game.board = build_board(&["bbbb.."]);
        game.chain_score = 0;
        game.state = State::Resolving {
            chain: 0,
            timer: Duration::ZERO,
        };
        let events = resolve(&mut game);
        assert_eq!(attacks(&events), vec![score::ALL_CLEAR_NUISANCE + 1]);
    }

    /// the death square, not a blocked spawn: the game ends when a puyo rests on it
    #[test]
    fn resting_on_the_death_square_ends_the_game() {
        let mut game = game();
        game.pair = None;
        game.board = Board::new(PuyoSkin::FIRST);
        // fill the spawn column to the death square
        for _ in 0..VISIBLE_ROWS {
            game.board.drop_into(SPAWN.x, PuyoCell::Nuisance);
        }
        assert!(game.board.is_dead());
        game.state = State::Resolving {
            chain: 0,
            timer: Duration::ZERO,
        };
        let events = resolve(&mut game);
        assert!(events.iter().any(|e| matches!(e, GameEvent::GameOver)));
        assert_eq!(game.stage_state(), StageState::GameOver);
    }

    /// ... and a column filled to the very top somewhere else does not
    #[test]
    fn filling_another_column_to_the_top_is_survivable() {
        let mut game = game();
        game.pair = None;
        game.board = Board::new(PuyoSkin::FIRST);
        for _ in 0..ROWS {
            game.board.drop_into(0, PuyoCell::Nuisance);
        }
        game.state = State::Resolving {
            chain: 0,
            timer: Duration::ZERO,
        };
        let events = resolve(&mut game);
        assert!(!events.iter().any(|e| matches!(e, GameEvent::GameOver)));
        assert_eq!(game.stage_state(), StageState::Playing);
    }

    #[test]
    fn a_stage_is_a_speed_step_and_play_carries_straight_on() {
        let mut game = game();
        assert_eq!(game.stage_transition(), StageTransition::Seamless);
        game.stage_puyos = rules::PUYOS_PER_STAGE;
        game.chain_score = 10;
        game.finish_chain(1);
        assert_eq!(game.stage_state(), StageState::StageComplete);
        // the flag alone does not speed a seamless game up; the event carries it on
        assert_eq!(
            stage_completions(&game),
            1,
            "reaching the goal has to announce itself"
        );

        let speed = game.speed_index();
        game.next_stage().unwrap();
        assert_eq!(game.speed_index(), speed + 1, "pairs fall faster");
        assert_eq!(game.completed_stages(), 1);
        assert_eq!(game.stage_state(), StageState::Playing);
    }

    #[test]
    fn a_chain_worth_several_stages_owes_a_step_for_each_of_them() {
        let mut game = game();
        game.stage_puyos = rules::PUYOS_PER_STAGE * 3 + 1;
        game.chain_score = 10;
        game.finish_chain(1);
        assert_eq!(stage_completions(&game), 3);
        assert_eq!(game.stage_puyos, 1, "the remainder carries into the next");
    }

    #[test]
    fn a_chain_short_of_the_goal_says_nothing() {
        let mut game = game();
        game.stage_puyos = rules::PUYOS_PER_STAGE - 1;
        game.chain_score = 10;
        game.finish_chain(1);
        assert_eq!(stage_completions(&game), 0);
        assert_eq!(game.stage_state(), StageState::Playing);
    }

    /// one seed deals one game to every player
    #[test]
    fn one_seed_deals_every_player_the_same_game() {
        let seed = Seed::from_u64(2024);
        let boards: Vec<Vec<String>> = (0..3)
            .map(|_| {
                let mut game = Game::new(
                    Difficulty::Hard,
                    0,
                    GameRandom::from_seed(seed, Difficulty::Hard.colors()),
                    PuyoSkin::FIRST,
                );
                for _ in 0..20 {
                    game.hard_drop();
                    run(&mut game, 200);
                }
                render(&game.board)
            })
            .collect();
        assert_eq!(boards[0], boards[1]);
        assert_eq!(boards[0], boards[2]);
    }

    /// the harder settings start you buried
    #[test]
    fn the_harder_settings_start_you_buried() {
        assert_eq!(game_at(Difficulty::Normal).board.occupied(), 0);
        assert_eq!(
            game_at(Difficulty::Easy).board.occupied(),
            2 * nuisance::ROW
        );
        assert_eq!(
            game_at(Difficulty::VeryHard).board.occupied(),
            2 * nuisance::ROW
        );
        // and the hardest also drops faster from the off
        assert_eq!(game_at(Difficulty::VeryHard).speed_index(), 2);
        assert_eq!(game_at(Difficulty::Hard).speed_index(), 0);
    }

    #[test]
    fn a_falling_pair_is_active_and_its_landing_is_a_ghost() {
        let game = game();
        let pair = game.pair.unwrap();
        assert!(matches!(game.cell(pair.pivot()), Cell::Active(_)));
        let ghost = pair.ghost(&game.board);
        assert!(matches!(game.cell(ghost.pivot()), Cell::Ghost(_)));
        assert!(matches!(game.cell(Point::new(0, 0)), Cell::Empty));
    }

    /// nuisance reports as Garbage to the engine
    #[test]
    fn nuisance_reads_as_garbage_and_puyos_as_stack() {
        let mut game = game_with(&["ro...."]);
        game.state = State::Falling {
            lock: Duration::ZERO,
        };
        let floor = ROWS as i32 - 1;
        assert!(matches!(game.cell(Point::new(0, floor)), Cell::Stack(_)));
        assert!(matches!(game.cell(Point::new(1, floor)), Cell::Garbage(_)));
    }

    #[test]
    fn a_pair_moves_and_rotates_under_the_player() {
        let mut game = game();
        game.drain_events();
        let start = game.pair.unwrap().pivot();
        game.left();
        assert_eq!(game.pair.unwrap().pivot().x, start.x - 1);
        game.right();
        game.right();
        assert_eq!(game.pair.unwrap().pivot().x, start.x + 1);
        game.rotate(true);
        assert_eq!(
            game.pair.unwrap().rotation(),
            engine::game::geometry::Rotation::East
        );
        let events = game.drain_events();
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, GameEvent::Move))
                .count(),
            3
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, GameEvent::Rotate))
                .count(),
            1
        );
    }

    /// nothing the player presses does anything while a chain is resolving
    #[test]
    fn the_player_has_no_say_while_a_chain_resolves() {
        let mut game = game_with(&["rrrr.."]);
        game.left();
        game.rotate(true);
        game.hard_drop();
        assert!(game.drain_events().is_empty());
    }

    #[test]
    fn the_clear_detail_survives_the_round_trip() {
        for chain in [1, 2, 19, 65535] {
            for all_clear in [false, true] {
                let detail = ClearDetail { chain, all_clear };
                assert_eq!(ClearDetail::from(u64::from(detail)), detail);
            }
        }
    }

    /// Two players firing on the same frame both get hit: every board updates before any attack
    /// is delivered, so neither chain offsets the other.
    #[test]
    fn two_chains_fired_on_the_same_frame_both_land() {
        let staircase = [
            "g.....", "g.....", "g.....", "b.....", "b.....", "b.....", "r.....", "r.....",
            "rg....", "rb....",
        ];
        let mut games = [game_with(&staircase), game_with(&staircase)];
        let mut fired = [0u32; 2];
        let mut frames_with_both = 0;

        for _ in 0..2000 {
            // every board steps, and only then is anything delivered
            let mut routed = vec![];
            for (player, game) in games.iter_mut().enumerate() {
                game.update(STEP);
                for strength in attacks(&game.drain_events()) {
                    fired[player] += strength;
                    routed.push((1 - player, strength));
                }
            }
            if routed.len() > 1 {
                frames_with_both += 1;
            }
            for (victim, strength) in routed {
                games[victim].receive_attack(Attack::new(GAME_ID, strength));
            }
            if fired[0] > 0 && fired[1] > 0 {
                break;
            }
        }

        assert_eq!(
            frames_with_both, 1,
            "the two identical boards fired together"
        );
        assert_eq!(fired, [14, 14], "and neither chain was spent cancelling");
        assert_eq!(
            [games[0].pending_nuisance(), games[1].pending_nuisance()],
            [14, 14],
            "both trays took the other's attack, and it waits there for an answer"
        );
    }

    /// A two-column staircase three chain of four-links scores 40 + 320 + 640 = 1000 points,
    /// buys 14 nuisance and leaves the board empty, as Puyo Nexus publishes.
    #[test]
    fn a_three_chain_scores_and_sends_what_the_published_table_says() {
        let mut game = game_with(&[
            "g.....", "g.....", "g.....", "b.....", "b.....", "b.....", "r.....", "r.....",
            "rg....", "rb....",
        ]);
        let events = resolve(&mut game);
        let clears = clears(&events);

        assert_eq!(clears.len(), 3, "a three chain");
        assert_eq!(clears[0].2.chain, 1);
        assert_eq!(clears[1].2.chain, 2);
        assert_eq!(clears[2].2.chain, 3);
        assert!(!clears[0].1, "the first step is not a combo");
        assert!(clears[1].1 && clears[2].1, "the rest are");
        assert!(
            clears.iter().all(|(count, _, _)| *count == 4),
            "four to a link"
        );

        assert_eq!(game.score(), 1000, "40 + 320 + 640");
        assert_eq!(game.max_chain(), 3);
        assert_eq!(attacks(&events), vec![14], "1000 / 70 target points");
        assert!(game.board.is_all_clear());
        assert!(
            game.queue.all_clear_owed(),
            "and the bonus is owed on the next one"
        );
    }
}
