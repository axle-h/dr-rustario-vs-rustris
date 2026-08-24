use crate::game::board::{compact_destroy_lines, Board, DestroyLines, BOARD_WIDTH, TOTAL_HEIGHT};
use crate::game::cell::{garbage_row, placed_minos, GAME_ID};
use crate::game::geometry::Point;
use crate::game::random::{RandomTetromino, PEEK_SIZE};
use crate::game::tetromino::{Minos, TetrominoShape};
use engine::game::hold::HoldState;
use engine::game::timing::{lock_move, LockMove, LockPlacements, Timing};
use engine::game::{
    Attack, Cell, GameEvent, GameId, MetricKind, PieceId, PlacedCell, StageState,
    StageTransition,
};
use std::cmp::max;
use std::time::Duration;

pub mod ai;
pub mod block;
pub mod board;
pub mod cell;
pub mod geometry;
pub mod random;
pub mod rules;
pub mod tetromino;

pub const LINES_PER_LEVEL: u32 = 10;
/// levels are 0-based: the guideline plays 15 levels with the fall speed curve ending on the
/// 15th, so the level (and with it the score multiplier of level + 1) caps together with [STEPS]
pub const MAX_LEVEL: u32 = STEPS.len() as u32 - 1;
pub const MAX_SCORE: u32 = 999_999_999;
pub const MAX_LINES: u32 = 9_999;
/// rows shown above the skyline
pub const VISIBLE_BUFFER: u32 = 2;
pub const VISIBLE_HEIGHT: u32 = board::BOARD_HEIGHT + VISIBLE_BUFFER;
/// tetrominoes slam down: the hard drop trail covers a row per frame
pub const HARD_DROP_ROWS_PER_FRAME: f64 = 1.0;

const TIMING: Timing = Timing::new(Duration::from_millis(500), Duration::from_millis(500 / 2))
    .with_spawn_delay_cap(Duration::from_millis(500));
const GARBAGE_WAIT: Duration = Duration::from_millis(50);

const SINGLE_POINTS: u32 = 100;
const DOUBLE_POINTS: u32 = 300;
const TRIPLE_POINTS: u32 = 500;
const TETRIS_POINTS: u32 = 800;
const COMBO_POINTS: u32 = 50;
const DIFFICULT_MULTIPLIER: f64 = 1.5;
const SOFT_DROP_POINTS_PER_ROW: u32 = 1;
const HARD_DROP_POINTS_PER_ROW: u32 = 2;

// pre-calculated step durations in ms: 1000 * (0.8 - (level as f64 * 0.007)).powi(level as i32)
// doing it like this as hashmaps cannot be constant and fp logic is not yet supported at compile time
const STEPS: [Duration; 15] = [
    Duration::from_millis(1000),
    Duration::from_millis(793),
    Duration::from_millis(618),
    Duration::from_millis(473),
    Duration::from_millis(355),
    Duration::from_millis(262),
    Duration::from_millis(190),
    Duration::from_millis(135),
    Duration::from_millis(94),
    Duration::from_millis(64),
    Duration::from_millis(43),
    Duration::from_millis(28),
    Duration::from_millis(18),
    Duration::from_millis(11),
    Duration::from_millis(7),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameState {
    Spawn(Duration, TetrominoShape),
    Fall(Duration),
    Lock(Duration),
    HardDropLock,
    /// check the board for completed lines
    Pattern,
    /// completed lines have been emptied; drop the stack once the clear animation is done
    Settle(DestroyLines),
    GameOver,
    SpawnGarbage {
        duration: Duration,
        next_shape: TetrominoShape,
        spawned: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Combo {
    count: u32,
    difficult: bool,
}

/// The reasons a game ends, as named by the Tetris guideline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameOverCondition {
    /// an opponent's attacks force existing blocks past the top of the buffer zone
    TopOut,
    /// the player locks a whole tetromino down above the skyline
    LockOut,
    /// one of the starting cells of the next tetromino is blocked by an existing block
    BlockOut,
}

pub struct Game {
    board: Board,
    random: RandomTetromino,
    level: u32,
    /// lines cleared since the start of the current stage
    stage_lines: u32,
    lines: u32,
    score: u32,
    combo: Option<Combo>,
    state: GameState,
    soft_drop: bool,
    skip_next_spawn_delay: bool,
    hold: Option<HoldState<TetrominoShape>>,
    garbage_buffer: u32,
    events: Vec<GameEvent>,
    completed_stages: u32,
    stage_complete: bool,
    game_over: Option<GameOverCondition>,
}

/// engine rows count down from the top, board rows count up from the floor
fn flip(point: Point) -> Point {
    Point::new(point.x, TOTAL_HEIGHT as i32 - 1 - point.y)
}

fn flip_minos(minos: Minos) -> Minos {
    minos.map(flip)
}

impl Game {
    pub fn new(level: u32, mut random: RandomTetromino) -> Game {
        let first_shape = random.next();
        Game {
            board: Board::new(),
            random,
            level,
            stage_lines: 0,
            lines: 0,
            score: 0,
            combo: None,
            state: GameState::Spawn(Duration::ZERO, first_shape),
            soft_drop: false,
            skip_next_spawn_delay: false,
            hold: None,
            garbage_buffer: 0,
            events: Vec::new(),
            completed_stages: 0,
            stage_complete: false,
            game_over: None,
        }
    }

    pub fn level(&self) -> u32 {
        self.level
    }

    pub fn lines(&self) -> u32 {
        self.lines
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn state(&self) -> GameState {
        self.state
    }

    pub fn game_over_condition(&self) -> Option<GameOverCondition> {
        self.game_over
    }

    pub fn queue(&self) -> [TetrominoShape; PEEK_SIZE] {
        self.random.peek_buffer()
    }

    pub fn held(&self) -> Option<TetrominoShape> {
        self.hold.map(|h| h.piece)
    }

    pub fn hold(&mut self) -> bool {
        if !(matches!(self.state, GameState::Fall(_))
            || matches!(self.state, GameState::Lock(duration) if duration < TIMING.lock))
            || HoldState::is_locked(&self.hold)
        {
            // hold is blocked
            return false;
        }

        let held_shape = match self.board.hold() {
            None => return false,
            Some(shape) => shape,
        };

        let next_shape = match self.hold {
            None => self.random.next(), // just spawn next random shape
            Some(HoldState { piece, .. }) => piece,
        };

        self.state = GameState::Spawn(Duration::from_millis(500), next_shape);
        self.hold = Some(HoldState::locked(held_shape));
        self.events.push(GameEvent::Hold);
        true
    }

    pub fn set_soft_drop(&mut self, soft_drop: bool) -> bool {
        self.soft_drop = soft_drop;
        if soft_drop {
            self.events.push(GameEvent::SoftDrop);
        }
        soft_drop
    }

    fn active_cells(&self, minos: Minos) -> Vec<PlacedCell> {
        match self.board.tetromino() {
            Some(t) => placed_minos(t.shape(), t.rotation(), flip_minos(minos)),
            None => vec![],
        }
    }

    pub fn hard_drop(&mut self) -> bool {
        if let Some((hard_dropped_rows, minos)) = self.board.hard_drop() {
            self.state = GameState::HardDropLock;
            self.score = (self.score + hard_dropped_rows * HARD_DROP_POINTS_PER_ROW).min(MAX_SCORE);
            self.skip_next_spawn_delay = true;
            self.events.push(GameEvent::HardDrop {
                cells: self.active_cells(minos),
                dropped_rows: hard_dropped_rows,
            });
            true
        } else {
            false
        }
    }

    pub fn left(&mut self) -> bool {
        if self.with_checking_lock(|board| board.left()) {
            self.events.push(GameEvent::Move);
            true
        } else {
            false
        }
    }

    pub fn right(&mut self) -> bool {
        if self.with_checking_lock(|board| board.right()) {
            self.events.push(GameEvent::Move);
            true
        } else {
            false
        }
    }

    pub fn rotate(&mut self, clockwise: bool) -> bool {
        if self.with_checking_lock(|board| board.rotate(clockwise)) {
            self.events.push(GameEvent::Rotate);
            true
        } else {
            false
        }
    }

    pub fn send_garbage(&mut self, rows: u32) {
        self.garbage_buffer += rows;
    }

    fn with_checking_lock<F>(&mut self, f: F) -> bool
    where
        F: FnMut(&mut Board) -> bool,
    {
        match self.state {
            GameState::Lock(lock_duration) => {
                match lock_move(&TIMING, lock_duration, &mut self.board, f) {
                    LockMove::Blocked => false,
                    LockMove::Exhausted => {
                        self.state = GameState::Lock(TIMING.lock);
                        false
                    }
                    LockMove::Moved { last_placement } => {
                        self.state = if last_placement {
                            GameState::Lock(TIMING.lock)
                        } else {
                            GameState::Fall(Duration::ZERO)
                        };
                        true
                    }
                }
            }
            _ => {
                let mut f = f;
                f(&mut self.board) // not in lock state, pass through closure
            }
        }
    }

    pub fn update(&mut self, delta: Duration) {
        self.state = match self.state {
            GameState::Spawn(duration, shape) => self.spawn(duration + delta, shape),
            GameState::Fall(duration) => self.fall(duration + delta),
            GameState::Lock(duration) => self.lock(duration + delta, false),
            GameState::HardDropLock => self.lock(TIMING.lock, true),
            GameState::Pattern => self.pattern(),
            GameState::Settle(lines) => self.settle(lines),
            GameState::SpawnGarbage {
                duration,
                next_shape,
                spawned,
            } => self.spawn_garbage(duration + delta, next_shape, spawned),
            GameState::GameOver => GameState::GameOver,
        };
    }

    fn end_game(&mut self, condition: GameOverCondition) -> GameState {
        self.game_over = Some(condition);
        self.events.push(GameEvent::GameOver);
        GameState::GameOver
    }

    fn spawn(&mut self, duration: Duration, shape: TetrominoShape) -> GameState {
        if self.garbage_buffer > 0 {
            return GameState::SpawnGarbage {
                duration: Duration::ZERO,
                next_shape: shape,
                spawned: 0,
            };
        }

        if !self.skip_next_spawn_delay && duration < self.spawn_delay() {
            return GameState::Spawn(duration, shape);
        }

        self.skip_next_spawn_delay = false;
        if let Some(minos) = self.board.try_spawn_tetromino(shape) {
            self.events.push(GameEvent::Spawn {
                piece: shape.into(),
                cells: self.active_cells(minos),
                is_hold: false,
            });
            GameState::Fall(Duration::ZERO)
        } else {
            self.end_game(GameOverCondition::BlockOut)
        }
    }

    fn fall(&mut self, duration: Duration) -> GameState {
        if duration < self.step_delay() {
            return GameState::Fall(duration);
        }

        if !self.board.step_down() {
            // cannot step down, start lock
            return GameState::Lock(Duration::ZERO);
        }

        // has stepped down one row, update score if soft dropping
        if self.soft_drop {
            self.score = (self.score + SOFT_DROP_POINTS_PER_ROW).min(MAX_SCORE);
        }

        self.events.push(GameEvent::Fall);
        if self.board.is_collision() {
            // step has caused a collision, start a lock
            if self.board.lock_placements() >= TIMING.max_lock_placements {
                GameState::Lock(TIMING.lock)
            } else {
                GameState::Lock(Duration::ZERO)
            }
        } else {
            GameState::Fall(Duration::ZERO)
        }
    }

    fn lock(&mut self, duration: Duration, hard_dropped: bool) -> GameState {
        if !hard_dropped && duration < TIMING.lock_duration(self.soft_drop) {
            GameState::Lock(duration)
        } else if self.board.is_collision() {
            // lock timeout and still colliding so lock the piece now
            // but before locking, need to check for a game over event.
            let is_lock_out = self.board.is_tetromino_above_skyline();
            let cells = self
                .board
                .tetromino()
                .map(|t| placed_minos(t.shape(), t.rotation(), flip_minos(t.minos())))
                .unwrap_or_default();

            self.board.lock().expect("we must've locked");
            HoldState::unlock(&mut self.hold);

            if is_lock_out {
                self.end_game(GameOverCondition::LockOut)
            } else {
                self.events.push(GameEvent::Lock {
                    cells,
                    dropped: hard_dropped || self.soft_drop,
                });
                GameState::Pattern
            }
        } else {
            // otherwise must've moved over empty space so start a new fall
            GameState::Fall(Duration::ZERO)
        }
    }

    /// completed lines leave the board at once and the theme animates them from the event;
    /// the stack above only drops once that animation has held the game
    fn pattern(&mut self) -> GameState {
        // TODO t-spin garbage
        let lines = self.board.pattern();
        let rows = compact_destroy_lines(lines);
        if rows.is_empty() {
            self.update_score_and_send_attack(lines);
            return GameState::Spawn(Duration::ZERO, self.random.next());
        }
        let cells = rows
            .iter()
            .flat_map(|y| {
                (0..BOARD_WIDTH).map(|x| {
                    let point = Point::from_u32(x, *y);
                    (flip(point), Cell::from(self.board.block(point)))
                })
            })
            .filter_map(|(point, cell)| cell.id().map(|id| (point, id)))
            .collect::<Vec<PlacedCell>>();
        self.board.clear_lines(lines);
        let is_combo = matches!(self.combo, Some(Combo { count, .. }) if count > 0);
        self.events.push(GameEvent::Clear {
            cells,
            count: rows.len() as u32,
            is_combo,
        });
        self.update_score_and_send_attack(lines);
        GameState::Settle(lines)
    }

    fn settle(&mut self, lines: DestroyLines) -> GameState {
        self.board.destroy(lines);
        self.events.push(GameEvent::Settle);
        GameState::Spawn(Duration::ZERO, self.random.next())
    }

    fn spawn_garbage(
        &mut self,
        duration: Duration,
        next_shape: TetrominoShape,
        spawned: u32,
    ) -> GameState {
        if duration < GARBAGE_WAIT {
            return GameState::SpawnGarbage {
                duration,
                next_shape,
                spawned,
            };
        }

        let hole = self.random.next_garbage_hole();
        self.board.send_garbage(hole);
        self.events.push(GameEvent::AttackReceived {
            cells: garbage_row(TOTAL_HEIGHT - 1, BOARD_WIDTH, hole),
        });

        if self.board.is_stack_above_skyline() {
            return self.end_game(GameOverCondition::TopOut);
        }

        self.garbage_buffer -= 1;
        if self.garbage_buffer == 0 {
            self.skip_next_spawn_delay = true;
            GameState::Spawn(Duration::ZERO, next_shape)
        } else {
            GameState::SpawnGarbage {
                duration: Duration::ZERO,
                next_shape,
                spawned: spawned + 1,
            }
        }
    }

    fn update_score_and_send_attack(&mut self, pattern: DestroyLines) {
        // TODO test
        // todo t-spin
        // todo perfect clear

        let line_count = pattern.iter().filter(|y| y.is_some()).count() as u32;

        let (action_score, action_difficult, garbage_lines) = match line_count {
            0 => {
                self.combo = None;
                return;
            }
            1 => (SINGLE_POINTS, false, 0),
            2 => (DOUBLE_POINTS, false, 1),
            3 => (TRIPLE_POINTS, false, 2),
            4 => (TETRIS_POINTS, true, 4),
            _ => unreachable!(),
        };

        // update combo
        self.combo = match self.combo {
            None => Some(Combo {
                count: 0,
                difficult: action_difficult,
            }),
            Some(Combo { count, difficult }) => Some(Combo {
                count: count + 1,
                difficult: difficult && action_difficult,
            }),
        };

        // calculate score delta
        let level_multiplier = self.level + 1;
        let (difficult_score_multiplier, difficult_garbage_lines) = match self.combo {
            // back to back difficult clears get a 1.5x multiplier
            Some(Combo { count, difficult }) if count > 0 && difficult => (DIFFICULT_MULTIPLIER, 1),
            _ => (1.0, 0),
        };
        let combo_score = match self.combo {
            Some(Combo { count, .. }) if count > 0 => COMBO_POINTS * count,
            _ => 0,
        };
        let score_delta = action_score as f64 * level_multiplier as f64 * difficult_score_multiplier
            + combo_score as f64;

        // update score
        self.score = (self.score + score_delta.round() as u32).min(MAX_SCORE);

        let attack = garbage_lines + difficult_garbage_lines;
        if attack > 0 {
            self.events
                .push(GameEvent::AttackSent(Attack::new(GAME_ID, attack)));
        }

        // update level: every ten lines is a level and a stage
        self.lines = (self.lines + line_count).min(MAX_LINES);
        self.stage_lines += line_count;
        if self.stage_lines >= LINES_PER_LEVEL {
            self.stage_lines -= LINES_PER_LEVEL;
            self.level = (self.level + 1).min(MAX_LEVEL);
            self.stage_complete = true;
            self.events.push(GameEvent::SpeedUp);
            self.events.push(GameEvent::StageComplete);
        }
    }

    fn spawn_delay(&self) -> Duration {
        TIMING.spawn_delay(self.base_delay(), self.soft_drop, STEPS[STEPS.len() - 1])
    }

    fn step_delay(&self) -> Duration {
        TIMING.step_delay(self.base_delay(), self.soft_drop, STEPS[STEPS.len() - 1])
    }

    fn base_delay(&self) -> Duration {
        STEPS[(self.level as usize).min(STEPS.len() - 1)]
    }
}

impl LockPlacements for Board {
    fn lock_placements(&self) -> u32 {
        Board::lock_placements(self)
    }

    fn register_lock_placement(&mut self) -> u32 {
        Board::register_lock_placement(self)
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
        Game::left(self);
    }

    fn right(&mut self) {
        Game::right(self);
    }

    fn rotate(&mut self, clockwise: bool) {
        Game::rotate(self, clockwise);
    }

    fn set_soft_drop(&mut self, soft_drop: bool) {
        Game::set_soft_drop(self, soft_drop);
    }

    fn hard_drop(&mut self) {
        Game::hard_drop(self);
    }

    fn hold(&mut self) {
        Game::hold(self);
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    fn board_width(&self) -> u32 {
        BOARD_WIDTH
    }

    fn board_height(&self) -> u32 {
        TOTAL_HEIGHT
    }

    fn visible_height(&self) -> u32 {
        VISIBLE_HEIGHT
    }

    fn cell(&self, point: Point) -> Cell {
        self.board.block(flip(point)).into()
    }

    fn queue(&self) -> Vec<PieceId> {
        self.random
            .peek_buffer()
            .into_iter()
            .map(Into::into)
            .collect()
    }

    fn held(&self) -> Option<PieceId> {
        self.hold.map(|h| h.piece.into())
    }

    fn metric(&self, kind: MetricKind) -> Option<u32> {
        match kind {
            MetricKind::Score => Some(self.score),
            MetricKind::Level => Some(self.level),
            MetricKind::Lines => Some(self.lines),
            MetricKind::Viruses => None,
        }
    }

    fn score(&self) -> u32 {
        self.score
    }

    fn set_score(&mut self, score: u32) {
        self.score = score.min(MAX_SCORE);
    }

    fn speed_index(&self) -> u32 {
        self.level
    }

    fn set_speed_index(&mut self, index: u32) {
        self.level = index.min(MAX_LEVEL);
    }

    fn stage_state(&self) -> StageState {
        if self.state == GameState::GameOver {
            StageState::GameOver
        } else if self.stage_complete {
            StageState::StageComplete
        } else {
            StageState::Playing
        }
    }

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
        if self.stage_complete {
            self.stage_complete = false;
            self.completed_stages += 1;
        }
        Ok(())
    }

    fn receive_attack(&mut self, attack: Attack) {
        self.send_garbage(max(attack.strength, 0));
    }
}
