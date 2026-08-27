use crate::game::board::{
    compact_destroy_lines, Board, DestroyLines, Spin, BOARD_WIDTH, MAX_DESTROYED_LINES,
    TOTAL_HEIGHT,
};
use crate::game::cell::{garbage_row, placed_minos, GAME_ID};
use crate::game::geometry::Point;
use crate::game::random::{RandomTetromino, PEEK_SIZE};
use crate::game::tetromino::{Minos, TetrominoShape};
use engine::game::hold::HoldState;
use engine::game::timing::{lock_move, LockMove, LockPlacements, Timing};
use engine::game::{
    ids, Attack, Cell, GameEvent, GameId, MetricKind, PieceId, PlacedCell, StageState,
    StageTransition,
};
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
/// rows of garbage a combo adds, indexed by the combo counter. A longer combo than the table
/// keeps sending its last entry
const COMBO_GARBAGE: [u32; 12] = [0, 1, 1, 2, 2, 3, 3, 4, 4, 4, 5, 5];
/// what a T-spin is worth by the lines it cleared. A T can complete three lines at most
const T_SPIN_POINTS: [u32; 4] = [400, 800, 1_200, 1_600];
const T_SPIN_MINI_POINTS: [u32; 4] = [100, 200, 400, 400];
/// rows a T-spin sends, by the lines it cleared
const T_SPIN_GARBAGE: [u32; 4] = [0, 2, 4, 6];
const T_SPIN_MINI_GARBAGE: [u32; 4] = [0, 0, 1, 1];
/// what a perfect clear is worth on top of the lines it cleared, by line count
const PERFECT_CLEAR_POINTS: [u32; 5] = [0, 800, 1_200, 1_800, 2_000];
/// a perfect clear by a tetris that was itself back to back
const PERFECT_CLEAR_BACK_TO_BACK_TETRIS_POINTS: u32 = 3_200;
/// rows a perfect clear sends on top of the rows its lines sent
const PERFECT_CLEAR_GARBAGE: u32 = 10;
/// What a clear is worth to a player of the *other* game, in garbage blocks, since that is
/// what an attack is over there. A Dr. Rustario bottle is eight wide and its blocks are only
/// cleared by matching four of a colour, so a row of them is nothing like a row of Rustris
/// garbage: a Rustris player who sent one row per row would bury a bottle in seconds. Only the
/// clears worth working for cross at all - a tetris, a T-spin that took two lines or more, and
/// a perfect clear - and a tetris crosses as two blocks, the size of the combo a Dr. Rustario
/// player sends most often. A clear that qualifies more than one way sends the larger of them,
/// and combos and back to back stay at home
const FOREIGN_TETRIS_GARBAGE: u32 = 2;
/// blocks a T-spin sends abroad, by the lines it cleared: a single is routine, so it does not
const FOREIGN_T_SPIN_GARBAGE: [u32; 4] = [0, 0, 2, 3];
/// blocks a perfect clear sends abroad, as much as a Dr. Rustario combo ever sends
const FOREIGN_PERFECT_CLEAR_GARBAGE: u32 = 4;
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
    /// check the board for completed lines, having locked a piece that spun into place or not
    Pattern(Option<Spin>),
    /// completed lines have been emptied; drop the stack once the clear animation is done
    Settle(DestroyLines),
    GameOver,
    SpawnGarbage {
        duration: Duration,
        next_shape: TetrominoShape,
        spawned: u32,
    },
}

/// What a locked piece achieved, in the terms the guideline scores it by. It travels to the
/// themes as the game-private `detail` of a [`GameEvent::Clear`], the way an [`Attack`] carries
/// its own detail, so a theme can tell a perfect clear from an ordinary one without knowing
/// anything about tetrominoes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ClearAction {
    /// lines the piece completed, 0 to 4
    pub lines: u32,
    /// the piece was spun into place
    pub spin: Option<Spin>,
    /// the piece left the board empty
    pub perfect_clear: bool,
}

const DETAIL_LINES: u64 = 0b111;
const DETAIL_PERFECT_CLEAR: u64 = 1 << 3;
const DETAIL_SPIN: u64 = 1 << 4;
const DETAIL_SPIN_MINI: u64 = 1 << 5;

impl ClearAction {
    pub fn to_detail(self) -> u64 {
        let mut detail = self.lines as u64 & DETAIL_LINES;
        if self.perfect_clear {
            detail |= DETAIL_PERFECT_CLEAR;
        }
        match self.spin {
            None => {}
            Some(Spin::Full) => detail |= DETAIL_SPIN,
            Some(Spin::Mini) => detail |= DETAIL_SPIN | DETAIL_SPIN_MINI,
        }
        detail
    }

    pub fn from_detail(detail: u64) -> Self {
        Self {
            lines: (detail & DETAIL_LINES) as u32,
            spin: match (detail & DETAIL_SPIN != 0, detail & DETAIL_SPIN_MINI != 0) {
                (false, _) => None,
                (true, false) => Some(Spin::Full),
                (true, true) => Some(Spin::Mini),
            },
            perfect_clear: detail & DETAIL_PERFECT_CLEAR != 0,
        }
    }
}

/// What a clear sends to a player of `receiver`, in that game's units: see
/// [`FOREIGN_TETRIS_GARBAGE`]. Nothing else a Rustris player does crosses.
///
/// Only the sender knows what the clear took, so only it can price the crossing - and it
/// prices each game it can reach separately, since a row, a garbage block and a nuisance puyo
/// are not the same thing. A game nothing here prices is worth nothing and never gets hit.
fn foreign_attack(receiver: GameId, action: ClearAction) -> u32 {
    if receiver != ids::DR_RUSTARIO {
        return 0;
    }
    let spin_index = (action.lines as usize).min(FOREIGN_T_SPIN_GARBAGE.len() - 1);
    let tetris = if action.lines as usize == MAX_DESTROYED_LINES {
        FOREIGN_TETRIS_GARBAGE
    } else {
        0
    };
    let spin = match action.spin {
        // a mini is not the trick the full spin is, so it stays at home with the rest
        Some(Spin::Full) => FOREIGN_T_SPIN_GARBAGE[spin_index],
        Some(Spin::Mini) | None => 0,
    };
    let perfect_clear = if action.perfect_clear {
        FOREIGN_PERFECT_CLEAR_GARBAGE
    } else {
        0
    };
    tetris.max(spin).max(perfect_clear)
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
    /// the guideline combo counter: `None` until a piece clears a line, then counting the
    /// clears *after* the first, and broken by a piece that clears nothing
    combo: Option<u32>,
    /// whether the last line clear was a difficult one, which is what a difficult clear has to
    /// follow to be worth back to back. Only a line clear can change it: a piece that clears
    /// nothing leaves it alone
    back_to_back: bool,
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
            back_to_back: false,
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
            GameState::Pattern(spin) => self.pattern(spin),
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
            // ask while the piece is still the board's: locking it takes it away
            let spin = self.board.t_spin();
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
                GameState::Pattern(spin)
            }
        } else {
            // otherwise must've moved over empty space so start a new fall
            GameState::Fall(Duration::ZERO)
        }
    }

    /// completed lines leave the board at once and the theme animates them from the event;
    /// the stack above only drops once that animation has held the game
    fn pattern(&mut self, spin: Option<Spin>) -> GameState {
        let lines = self.board.pattern();
        let rows = compact_destroy_lines(lines);
        if rows.is_empty() {
            self.update_score_and_send_attack(ClearAction {
                lines: 0,
                spin,
                perfect_clear: false,
            });
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
        // the completed rows have been emptied and the stack above them has not dropped yet, so
        // nothing left on the board now means nothing will be left once it settles either
        let action = ClearAction {
            lines: rows.len() as u32,
            spin,
            perfect_clear: self.board.is_empty(),
        };
        // a combo is already running, so this clear continues it
        let is_combo = self.combo.is_some();
        self.events.push(GameEvent::Clear {
            cells,
            count: action.lines,
            is_combo,
            detail: action.to_detail(),
        });
        self.update_score_and_send_attack(action);
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

    fn update_score_and_send_attack(&mut self, action: ClearAction) {
        let line_count = action.lines;
        // a T can complete three lines at most, so the spin tables are indexed by a clamped
        // count rather than trusting the caller
        let spin_index = (line_count as usize).min(T_SPIN_POINTS.len() - 1);

        let (action_score, action_difficult, garbage_lines) = match (action.spin, line_count) {
            (spin, 0) => {
                // a piece that clears nothing breaks the combo. It does not break back to
                // back: only a line clear that is not difficult can do that
                self.combo = None;
                // a spin that cleared nothing still scores, it just scores nothing else
                if let Some(spin) = spin {
                    let points = match spin {
                        Spin::Full => T_SPIN_POINTS[0],
                        Spin::Mini => T_SPIN_MINI_POINTS[0],
                    };
                    let score_delta = points * (self.level + 1);
                    self.score = (self.score + score_delta).min(MAX_SCORE);
                }
                return;
            }
            // every T-spin that clears a line is difficult, mini or not
            (Some(Spin::Full), _) => (T_SPIN_POINTS[spin_index], true, T_SPIN_GARBAGE[spin_index]),
            (Some(Spin::Mini), _) => (
                T_SPIN_MINI_POINTS[spin_index],
                true,
                T_SPIN_MINI_GARBAGE[spin_index],
            ),
            (None, 1) => (SINGLE_POINTS, false, 0),
            (None, 2) => (DOUBLE_POINTS, false, 1),
            (None, 3) => (TRIPLE_POINTS, false, 2),
            (None, 4) => (TETRIS_POINTS, true, 4),
            (None, _) => unreachable!(),
        };

        // update the combo counter: 0 for the first clear of a chain, and one more for each
        // clear that follows it
        let combo = self.combo.map_or(0, |count| count + 1);
        self.combo = Some(combo);

        // a difficult clear straight after another difficult clear is worth back to back
        let back_to_back = action_difficult && self.back_to_back;
        self.back_to_back = action_difficult;

        // calculate score delta
        let level_multiplier = self.level + 1;
        let (difficult_score_multiplier, difficult_garbage_lines) = if back_to_back {
            (DIFFICULT_MULTIPLIER, 1)
        } else {
            (1.0, 0)
        };
        let combo_score = COMBO_POINTS * combo * level_multiplier;
        // an emptied board pays a bonus on top of the lines that emptied it
        let perfect_clear_score = if action.perfect_clear {
            let points = if line_count as usize == MAX_DESTROYED_LINES && back_to_back {
                PERFECT_CLEAR_BACK_TO_BACK_TETRIS_POINTS
            } else {
                PERFECT_CLEAR_POINTS[line_count as usize]
            };
            points * level_multiplier
        } else {
            0
        };
        let score_delta =
            action_score as f64 * level_multiplier as f64 * difficult_score_multiplier
                + combo_score as f64
                + perfect_clear_score as f64;

        // update score
        self.score = (self.score + score_delta.round() as u32).min(MAX_SCORE);

        let combo_garbage = COMBO_GARBAGE[(combo as usize).min(COMBO_GARBAGE.len() - 1)];
        let perfect_clear_garbage = if action.perfect_clear {
            PERFECT_CLEAR_GARBAGE
        } else {
            0
        };
        let attack =
            garbage_lines + difficult_garbage_lines + combo_garbage + perfect_clear_garbage;
        if attack > 0 {
            self.events.push(GameEvent::AttackSent(
                Attack::new(GAME_ID, attack)
                    .with_foreign_for(ids::DR_RUSTARIO, foreign_attack(ids::DR_RUSTARIO, action)),
            ));
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
            MetricKind::Viruses | MetricKind::Chain => None,
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
        self.send_garbage(attack.strength_for(GAME_ID));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::block::BlockState;
    use crate::game::random::RandomMode;

    fn game() -> Game {
        Game::new(0, RandomTetromino::new(RandomMode::Bag, 10, 7.into()))
    }

    /// a pattern of n completed lines from the bottom of the board up
    fn lines(n: u32) -> DestroyLines {
        let mut result: DestroyLines = [None; MAX_DESTROYED_LINES];
        for (y, slot) in result.iter_mut().enumerate().take(n as usize) {
            *slot = Some(y as u32);
        }
        result
    }

    /// score a locked piece that cleared n lines on a fixed level, returning the points it
    /// earned. The level is pinned so a test may clear as much as it likes without the level
    /// multiplier moving underneath it
    fn clear_at(game: &mut Game, level: u32, n: u32) -> u32 {
        game.level = level;
        game.stage_lines = 0;
        let before = game.score;
        game.update_score_and_send_attack(ClearAction {
            lines: n,
            spin: None,
            perfect_clear: false,
        });
        game.score - before
    }

    fn clear(game: &mut Game, n: u32) -> u32 {
        clear_at(game, 0, n)
    }

    /// score a locked piece that cleared n lines and emptied the board with them
    fn perfect_clear_at(game: &mut Game, level: u32, n: u32) -> u32 {
        game.level = level;
        game.stage_lines = 0;
        let before = game.score;
        game.update_score_and_send_attack(ClearAction {
            lines: n,
            spin: None,
            perfect_clear: true,
        });
        game.score - before
    }

    /// the rows of garbage the events drained so far would send
    fn attack_sent(game: &mut Game) -> u32 {
        std::mem::take(&mut game.events)
            .into_iter()
            .filter_map(|event| match event {
                GameEvent::AttackSent(attack) => Some(attack.strength),
                _ => None,
            })
            .sum()
    }

    #[test]
    fn line_clears_score_the_guideline_values() {
        assert_eq!(clear(&mut game(), 1), SINGLE_POINTS);
        assert_eq!(clear(&mut game(), 2), DOUBLE_POINTS);
        assert_eq!(clear(&mut game(), 3), TRIPLE_POINTS);
        assert_eq!(clear(&mut game(), 4), TETRIS_POINTS);
    }

    #[test]
    fn the_level_multiplies_the_line_score() {
        // levels are 0-based so the multiplier is level + 1
        assert_eq!(clear_at(&mut game(), 4, 1), SINGLE_POINTS * 5);
        assert_eq!(clear_at(&mut game(), 9, 4), TETRIS_POINTS * 10);
    }

    #[test]
    fn a_tetris_after_a_tetris_is_worth_back_to_back() {
        let mut game = game();
        assert_eq!(clear(&mut game, 4), TETRIS_POINTS);
        // back to back tetrises are also a combo, so the second is worth the combo too
        assert_eq!(
            clear(&mut game, 4),
            (TETRIS_POINTS as f64 * 1.5) as u32 + COMBO_POINTS
        );
    }

    #[test]
    fn back_to_back_survives_a_piece_that_clears_nothing() {
        let mut game = game();
        clear(&mut game, 4);
        clear(&mut game, 0);
        clear(&mut game, 0);
        // only a line clear that is not difficult can break back to back
        assert_eq!(clear(&mut game, 4), (TETRIS_POINTS as f64 * 1.5) as u32);
    }

    #[test]
    fn a_clear_that_is_not_difficult_breaks_back_to_back() {
        let mut game = game();
        clear(&mut game, 4);
        clear(&mut game, 1);
        // the third clear is combo 2, but earns no back to back multiplier
        assert_eq!(clear(&mut game, 4), TETRIS_POINTS + COMBO_POINTS * 2);
    }

    #[test]
    fn a_combo_scores_fifty_a_clear_times_the_level() {
        let mut game = game();
        // the first clear of a chain is combo 0 and scores no combo points
        assert_eq!(clear_at(&mut game, 3, 1), SINGLE_POINTS * 4);
        assert_eq!(
            clear_at(&mut game, 3, 1),
            SINGLE_POINTS * 4 + COMBO_POINTS * 4
        );
        assert_eq!(
            clear_at(&mut game, 3, 1),
            SINGLE_POINTS * 4 + COMBO_POINTS * 2 * 4
        );
    }

    #[test]
    fn a_piece_that_clears_nothing_breaks_the_combo() {
        let mut game = game();
        clear(&mut game, 1);
        clear(&mut game, 1);
        clear(&mut game, 0);
        assert_eq!(game.combo, None);
        assert_eq!(clear(&mut game, 1), SINGLE_POINTS);
    }

    #[test]
    fn line_clears_send_the_guideline_garbage() {
        for (count, expected) in [(1, 0), (2, 1), (3, 2), (4, 4)] {
            let mut game = game();
            clear(&mut game, count);
            assert_eq!(attack_sent(&mut game), expected, "{count} lines");
        }
    }

    #[test]
    fn back_to_back_sends_an_extra_row() {
        let mut game = game();
        clear(&mut game, 4);
        // break the combo so the extra row is the back to back one alone
        clear(&mut game, 0);
        attack_sent(&mut game);
        clear(&mut game, 4);
        assert_eq!(attack_sent(&mut game), 4 + 1);
    }

    #[test]
    fn a_combo_sends_garbage_from_the_table() {
        let mut game = game();
        for (combo, expected) in COMBO_GARBAGE.iter().enumerate() {
            clear(&mut game, 1);
            assert_eq!(attack_sent(&mut game), *expected, "combo {combo}");
        }
    }

    #[test]
    fn a_long_combo_keeps_sending_the_last_row_of_the_table() {
        let mut game = game();
        for _ in 0..COMBO_GARBAGE.len() + 5 {
            clear(&mut game, 1);
            attack_sent(&mut game);
        }
        clear(&mut game, 1);
        assert_eq!(
            attack_sent(&mut game),
            COMBO_GARBAGE[COMBO_GARBAGE.len() - 1]
        );
    }

    #[test]
    fn a_perfect_clear_pays_a_bonus_on_top_of_its_lines() {
        for (lines, action_points) in [
            (1, SINGLE_POINTS),
            (2, DOUBLE_POINTS),
            (3, TRIPLE_POINTS),
            (4, TETRIS_POINTS),
        ] {
            let observed = perfect_clear_at(&mut game(), 0, lines);
            assert_eq!(
                observed,
                action_points + PERFECT_CLEAR_POINTS[lines as usize],
                "{lines} lines"
            );
        }
    }

    #[test]
    fn a_back_to_back_tetris_perfect_clear_is_worth_more() {
        let mut game = game();
        clear(&mut game, 4);
        // break the combo so only the back to back tetris bonus is left to see
        clear(&mut game, 0);
        let observed = perfect_clear_at(&mut game, 0, 4);
        assert_eq!(
            observed,
            (TETRIS_POINTS as f64 * 1.5) as u32 + PERFECT_CLEAR_BACK_TO_BACK_TETRIS_POINTS
        );
    }

    #[test]
    fn the_level_multiplies_the_perfect_clear_bonus() {
        let observed = perfect_clear_at(&mut game(), 4, 2);
        assert_eq!(observed, (DOUBLE_POINTS + PERFECT_CLEAR_POINTS[2]) * 5);
    }

    #[test]
    fn a_perfect_clear_sends_ten_rows_on_top_of_its_lines() {
        let mut game = game();
        perfect_clear_at(&mut game, 0, 2);
        assert_eq!(attack_sent(&mut game), 1 + PERFECT_CLEAR_GARBAGE);
    }

    #[test]
    fn clear_detail_survives_the_round_trip() {
        for lines in 0..=4 {
            for spin in [None, Some(Spin::Full), Some(Spin::Mini)] {
                for perfect_clear in [false, true] {
                    let action = ClearAction {
                        lines,
                        spin,
                        perfect_clear,
                    };
                    assert_eq!(ClearAction::from_detail(action.to_detail()), action);
                }
            }
        }
    }

    #[test]
    fn emptying_the_board_is_a_perfect_clear() {
        let mut game = game();
        having_completed_rows(&mut game, 2);
        game.pattern(None);
        assert!(ClearAction::from_detail(clear_event_detail(&mut game).unwrap()).perfect_clear);
    }

    #[test]
    fn leaving_a_block_behind_is_not_a_perfect_clear() {
        let mut game = game();
        having_completed_rows(&mut game, 2);
        game.board
            .set_block(Point::from_u32(0, 4), BlockState::Garbage);
        game.pattern(None);
        assert!(!ClearAction::from_detail(clear_event_detail(&mut game).unwrap()).perfect_clear);
    }

    /// score a locked piece that spun into place and cleared n lines
    fn spin_clear_at(game: &mut Game, level: u32, spin: Spin, n: u32) -> u32 {
        game.level = level;
        game.stage_lines = 0;
        let before = game.score;
        game.update_score_and_send_attack(ClearAction {
            lines: n,
            spin: Some(spin),
            perfect_clear: false,
        });
        game.score - before
    }

    #[test]
    fn t_spins_score_the_guideline_values() {
        for lines in 0..=3 {
            assert_eq!(
                spin_clear_at(&mut game(), 0, Spin::Full, lines),
                T_SPIN_POINTS[lines as usize],
                "{lines} lines"
            );
            assert_eq!(
                spin_clear_at(&mut game(), 0, Spin::Mini, lines),
                T_SPIN_MINI_POINTS[lines as usize],
                "mini, {lines} lines"
            );
        }
    }

    #[test]
    fn the_level_multiplies_a_t_spin() {
        assert_eq!(
            spin_clear_at(&mut game(), 4, Spin::Full, 2),
            T_SPIN_POINTS[2] * 5
        );
        assert_eq!(
            spin_clear_at(&mut game(), 4, Spin::Full, 0),
            T_SPIN_POINTS[0] * 5
        );
    }

    #[test]
    fn every_t_spin_that_clears_a_line_is_difficult() {
        for spin in [Spin::Full, Spin::Mini] {
            let mut game = game();
            spin_clear_at(&mut game, 0, spin, 1);
            // break the combo so only the back to back multiplier is left to see
            clear(&mut game, 0);
            assert_eq!(
                clear(&mut game, 4),
                (TETRIS_POINTS as f64 * 1.5) as u32,
                "{spin:?}"
            );
        }
    }

    #[test]
    fn a_t_spin_that_clears_nothing_breaks_the_combo_but_not_back_to_back() {
        let mut game = game();
        clear(&mut game, 4);
        clear(&mut game, 1);
        spin_clear_at(&mut game, 0, Spin::Full, 0);
        assert_eq!(game.combo, None);
        // the single broke back to back, and the spin that cleared nothing did not restore it
        assert_eq!(clear(&mut game, 4), TETRIS_POINTS);
    }

    /// the garbage the events drained so far would send to a player of the *other* game
    fn foreign_attack_sent(game: &mut Game) -> u32 {
        std::mem::take(&mut game.events)
            .into_iter()
            .filter_map(|event| match event {
                GameEvent::AttackSent(attack) => Some(attack.strength_for(ids::DR_RUSTARIO)),
                _ => None,
            })
            .sum()
    }

    #[test]
    fn only_the_clears_worth_working_for_cross_to_the_other_game() {
        for (count, expected) in [(1, 0), (2, 0), (3, 0), (4, FOREIGN_TETRIS_GARBAGE)] {
            let mut game = game();
            clear(&mut game, count);
            assert_eq!(foreign_attack_sent(&mut game), expected, "{count} lines");
        }
    }

    #[test]
    fn a_t_spin_crosses_to_the_other_game_from_two_lines_up() {
        for lines in 0..=3 {
            let mut full = game();
            spin_clear_at(&mut full, 0, Spin::Full, lines);
            assert_eq!(
                foreign_attack_sent(&mut full),
                FOREIGN_T_SPIN_GARBAGE[lines as usize],
                "{lines} lines"
            );

            // a mini is not the trick a full spin is, and never crosses
            let mut mini = game();
            spin_clear_at(&mut mini, 0, Spin::Mini, lines);
            assert_eq!(foreign_attack_sent(&mut mini), 0, "mini, {lines} lines");
        }
    }

    #[test]
    fn a_perfect_clear_crosses_whatever_cleared_it() {
        for lines in 1..=4 {
            let mut game = game();
            perfect_clear_at(&mut game, 0, lines);
            assert_eq!(
                foreign_attack_sent(&mut game),
                FOREIGN_PERFECT_CLEAR_GARBAGE,
                "{lines} lines"
            );
        }
    }

    #[test]
    fn combos_and_back_to_back_stay_at_home() {
        let mut game = game();
        // a combo of singles sends rows at home and nothing abroad
        for _ in 0..4 {
            clear(&mut game, 1);
        }
        assert!(attack_sent(&mut game) > 0);
        assert_eq!(foreign_attack_sent(&mut game), 0);

        // ... and back to back tetrises cross as a plain tetris each
        let mut game = super::tests::game();
        clear(&mut game, 4);
        assert_eq!(foreign_attack_sent(&mut game), FOREIGN_TETRIS_GARBAGE);
        clear(&mut game, 4);
        assert_eq!(foreign_attack_sent(&mut game), FOREIGN_TETRIS_GARBAGE);
    }

    #[test]
    fn a_foreign_attack_lands_in_the_receivers_own_units() {
        let mut game = game();
        engine::game::Game::receive_attack(
            &mut game,
            Attack::new(GAME_ID, 8).with_foreign_for(GAME_ID, 2),
        );
        assert_eq!(game.garbage_buffer, 8, "another Rustris player sends rows");

        let mut game = super::tests::game();
        engine::game::Game::receive_attack(
            &mut game,
            Attack::new(GameId(u16::MAX), 8).with_foreign_for(GAME_ID, 2),
        );
        assert_eq!(game.garbage_buffer, 2, "another game sends what it says");
    }

    #[test]
    fn t_spins_send_the_guideline_garbage() {
        for lines in 0..=3 {
            let mut full = game();
            spin_clear_at(&mut full, 0, Spin::Full, lines);
            assert_eq!(
                attack_sent(&mut full),
                T_SPIN_GARBAGE[lines as usize],
                "{lines} lines"
            );

            let mut mini = game();
            spin_clear_at(&mut mini, 0, Spin::Mini, lines);
            assert_eq!(
                attack_sent(&mut mini),
                T_SPIN_MINI_GARBAGE[lines as usize],
                "mini, {lines} lines"
            );
        }
    }

    /// a T spawned into a nook that has three of its corners filled either way it is turned,
    /// and resting on a block so that it locks
    fn having_t_at_spawn(game: &mut Game) {
        assert!(game.board.try_spawn_tetromino(TetrominoShape::T).is_some());
        // clear of the skyline, so that locking it up here is not a lock out
        assert!(game.board.step_down());
        assert!(game.board.step_down());
        for (x, y) in [(5, 19), (5, 17), (3, 17), (4, 16)] {
            game.board
                .set_block(Point::from_u32(x, y), BlockState::Garbage);
        }
    }

    fn locked_state(game: &mut Game) -> GameState {
        game.state = GameState::Lock(TIMING.lock);
        game.update(Duration::ZERO);
        game.state
    }

    #[test]
    fn locking_a_t_that_spun_into_place_carries_the_spin_to_the_pattern() {
        let mut game = game();
        having_t_at_spawn(&mut game);
        assert!(game.board.rotate(true));
        assert_eq!(
            locked_state(&mut game),
            GameState::Pattern(Some(Spin::Full))
        );
    }

    #[test]
    fn locking_a_t_that_never_rotated_carries_no_spin() {
        let mut game = game();
        // the same three corners are filled, but the piece only ever fell into them
        having_t_at_spawn(&mut game);
        assert_eq!(locked_state(&mut game), GameState::Pattern(None));
    }

    /// fill n rows from the bottom of the board so the next pattern completes them
    fn having_completed_rows(game: &mut Game, n: u32) {
        for y in 0..n {
            for x in 0..BOARD_WIDTH {
                game.board
                    .set_block(Point::from_u32(x, y), BlockState::Garbage);
            }
        }
    }

    fn clear_event_detail(game: &mut Game) -> Option<u64> {
        std::mem::take(&mut game.events)
            .into_iter()
            .find_map(|event| match event {
                GameEvent::Clear { detail, .. } => Some(detail),
                _ => None,
            })
    }

    fn clear_event_is_combo(game: &mut Game) -> Option<bool> {
        std::mem::take(&mut game.events)
            .into_iter()
            .find_map(|event| match event {
                GameEvent::Clear { is_combo, .. } => Some(is_combo),
                _ => None,
            })
    }

    #[test]
    fn the_first_clear_of_a_chain_is_not_a_combo() {
        let mut game = game();
        having_completed_rows(&mut game, 1);
        game.pattern(None);
        assert_eq!(clear_event_is_combo(&mut game), Some(false));
    }

    #[test]
    fn the_second_clear_of_a_chain_is_a_combo() {
        let mut game = game();
        having_completed_rows(&mut game, 1);
        game.pattern(None);
        clear_event_is_combo(&mut game);

        game.state = GameState::Settle(lines(1));
        game.board.destroy(lines(1));
        having_completed_rows(&mut game, 1);
        game.pattern(None);
        assert_eq!(clear_event_is_combo(&mut game), Some(true));
    }
}
