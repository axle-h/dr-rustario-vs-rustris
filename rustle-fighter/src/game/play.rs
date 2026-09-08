//! The piece lifecycle: what the engine drives, one frame at a time.
//!
//! [`crate::game::Playfield`] is the rules and knows nothing about time. This is the state
//! machine over it - a pair falls, it locks, the chain loop runs a pass at a time so it can be
//! watched, the garbage that survived the offset lands, and the next pair enters - plus the
//! [`engine::game::Game`] impl that is the whole of what the engine sees.

use crate::game::board::{self, Board, COLUMNS, ROWS, SPAWN, VISIBLE_ROWS};
use crate::game::cell::{Gem, PowerMask};
use crate::game::counter::Fighter;
use crate::game::gems::Pressure;
use crate::game::pair::{Pair, RotateOutcome};
use crate::game::random::GameRandom;
use crate::game::rules::{self, Difficulty};
use crate::game::score::Level;
use crate::game::{Playfield, GAME_ID};
use engine::game::geometry::Point;
use engine::game::{
    Attack, Cell, CellId, GameEvent, GameId, MetricKind, PieceId, PlacedCell, StageState,
    StageTransition,
};
use std::time::Duration;

/// how many gems have to go in one pass before the background field calls it a big break
pub const BIG_BREAK_GEMS: u32 = 8;

/// the chain length that earns the background field's `CHAIN`
pub const LONG_CHAIN: u32 = 4;

/// What a [`GameEvent::Clear`] carries in its game-private `detail`.
///
/// The renderer reads it back to grade the break and pick a word for it; the engine never
/// looks inside.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClearDetail {
    /// which pass of the chain this was, counting from 1
    pub chain: u32,
    /// this pass left the board empty
    pub all_clear: bool,
    /// a rainbow gem came to rest on the floor
    pub tech_bonus: bool,
}

const DETAIL_CHAIN: u64 = 0xffff;
const DETAIL_ALL_CLEAR: u64 = 1 << 16;
const DETAIL_TECH_BONUS: u64 = 1 << 17;

impl From<ClearDetail> for u64 {
    fn from(detail: ClearDetail) -> u64 {
        (detail.chain as u64 & DETAIL_CHAIN)
            | if detail.all_clear {
                DETAIL_ALL_CLEAR
            } else {
                0
            }
            | if detail.tech_bonus {
                DETAIL_TECH_BONUS
            } else {
                0
            }
    }
}

impl From<u64> for ClearDetail {
    fn from(detail: u64) -> ClearDetail {
        ClearDetail {
            chain: (detail & DETAIL_CHAIN) as u32,
            all_clear: detail & DETAIL_ALL_CLEAR != 0,
            tech_bonus: detail & DETAIL_TECH_BONUS != 0,
        }
    }
}

/// Where the game is in a placement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// nothing in play; the next pair is about to appear
    Spawning(Duration),
    /// the player has the pair
    Falling {
        lock: Duration,
    },
    /// the chain loop, a pass at a time so it can be watched
    Resolving {
        chain: u32,
        timer: Duration,
    },
    /// counter gems arriving
    Delivering(Duration),
    GameOver,
}

pub struct Game {
    play: Playfield,
    random: GameRandom,
    pair: Option<Pair>,
    state: State,
    events: Vec<GameEvent>,
    level: Level,
    speed_index: u32,
    completed_stages: u32,
    stage_complete: bool,
    /// gems destroyed towards the next speed step
    stage_gems: u32,
    /// **the round clock**, `+0x1f0`. Every attack in this game grows with it, which is a rule
    /// no other game in the compendium has
    round: Duration,
    soft_drop: bool,
    fall: Duration,
    /// the fighter on the *other* board, whose pattern this player's garbage arrives in
    opponent: Fighter,
}

impl Game {
    pub fn new(
        fighter: Fighter,
        difficulty: Difficulty,
        speed_index: u32,
        random: GameRandom,
    ) -> Game {
        // the column ordering counter gems arrive in is picked once, when the round starts
        let ordering = random.seed().bytes()[0] as usize;
        let mut play = Playfield::new(fighter, ordering);
        let rows = difficulty.starting_counter_rows();
        if rows > 0 {
            play.tray.receive(rows * COLUMNS, false);
            play.take_garbage(fighter);
        }
        let mut game = Game {
            play,
            random,
            pair: None,
            state: State::Spawning(Duration::ZERO),
            events: vec![],
            level: difficulty.level(),
            speed_index,
            completed_stages: 0,
            stage_complete: false,
            stage_gems: 0,
            round: Duration::ZERO,
            soft_drop: false,
            fall: Duration::ZERO,
            opponent: fighter,
        };
        game.spawn();
        game
    }

    pub fn board(&self) -> &Board {
        &self.play.board
    }

    pub fn playfield(&self) -> &Playfield {
        &self.play
    }

    pub fn pair(&self) -> Option<Pair> {
        self.pair
    }

    /// how buried this player is, which is the only thing the fighter sprite reads
    pub fn pressure(&self) -> Pressure {
        self.play.pressure()
    }

    pub fn pending_counter_gems(&self) -> u32 {
        self.play.tray.pending()
    }

    pub fn best_chain(&self) -> u32 {
        self.play.best_chain
    }

    /// the fighter playing this board
    pub fn fighter(&self) -> Fighter {
        self.play.fighter
    }

    /// **The round clock in whole seconds**, capped where the game caps it.
    ///
    /// Damage grows with this and with nothing else about how the board looks - see
    /// [`crate::game::score::time_tier`].
    pub fn round_seconds(&self) -> u32 {
        (self.round.as_secs() as u32).min(rules::MAX_ROUND_SECONDS)
    }

    fn fall_interval(&self) -> Duration {
        let delay = rules::fall_delay(self.speed_index);
        if self.soft_drop {
            delay.min(rules::SOFT_DROP_DELAY)
        } else {
            delay
        }
    }

    /// the cells of a pair, always drawn unjoined - a gem joins a power gem only once it has
    /// landed and a rectangle has been found
    fn pair_cells(pair: &Pair) -> Vec<PlacedCell> {
        let [pivot, child] = pair.points();
        let [pivot_half, child_half] = pair.halves();
        vec![
            (pivot, pivot_half.gem().loose_id()),
            (child, child_half.gem().loose_id()),
        ]
    }

    fn spawn(&mut self) {
        let piece = self.random.next_pair();
        let pair = Pair::new(SPAWN, piece);
        if pair.points().iter().any(|p| !self.play.board.is_free(*p)) {
            // the Drop Alley is blocked, and there is nowhere for a piece to enter
            self.events.push(GameEvent::GameOver);
            self.state = State::GameOver;
            return;
        }
        self.events.push(GameEvent::Spawn {
            piece: PieceId::from(piece),
            cells: Game::pair_cells(&pair),
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
        let cells = Game::pair_cells(&pair);
        pair.lock(&mut self.play.board);
        // whichever half is already resting on something has landed; the other is about to
        // come apart from it and reports itself out of the settle instead
        let landed: Vec<PlacedCell> = cells
            .iter()
            .copied()
            .filter(|(at, _)| !self.play.board.is_free(at.translate(0, 1)))
            .collect();
        self.events.push(GameEvent::Lock { cells, dropped });
        if !landed.is_empty() {
            self.events.push(GameEvent::Landed { cells: landed });
        }
        // a piece landing is what ripens every counter gem on the board by one
        self.play.board.tick_countdowns();
        self.state = State::Resolving {
            chain: 0,
            timer: rules::SETTLE_DELAY,
        };
    }

    /// what the board holds at each of `points`, as the engine's own placed cells
    fn placed(&self, points: &[Point]) -> Vec<PlacedCell> {
        points
            .iter()
            .filter_map(|at| {
                self.play
                    .board
                    .get(*at)
                    .map(|gem| (*at, gem.id(self.play.board.power_mask(*at))))
            })
            .collect()
    }

    /// One pass of the chain loop, or the end of it.
    fn resolve(&mut self, chain: u32) {
        let round = self.round_seconds();
        let Some(step) = self.play.resolve_step(chain, round, self.level) else {
            self.finish_chain(chain);
            return;
        };
        self.stage_gems += step.erased.count();
        let detail = ClearDetail {
            chain: step.chain,
            all_clear: step.all_clear,
            tech_bonus: step.erased.tech_bonus,
        };
        let cells: Vec<PlacedCell> = step
            .erased
            .cells
            .iter()
            .map(|(at, gem)| (*at, gem.id(PowerMask::NONE)))
            .collect();
        let count = step.erased.count();
        self.events.push(GameEvent::Clear {
            cells,
            count,
            // the same grammar the other games use: false on the first pass of a chain, true
            // on every one after, so nothing else has to know what a chain is
            is_combo: step.chain > 1,
            detail: detail.into(),
        });
        self.state = State::Resolving {
            chain: step.chain,
            timer: rules::ERASE_DELAY,
        };
    }

    /// The chain is over: settle up with the tray, then let whatever is still owed land.
    fn finish_chain(&mut self, chain: u32) {
        if chain > 0 {
            let sent = self.play.offset_outgoing();
            if sent > 0 {
                self.events
                    .push(GameEvent::AttackSent(Attack::new(GAME_ID, sent)));
            }
            // one event per stage the chain paid for, since a long chain can be worth several
            while self.stage_gems >= rules::GEMS_PER_STAGE {
                self.stage_gems -= rules::GEMS_PER_STAGE;
                self.stage_complete = true;
                self.events.push(GameEvent::StageComplete);
            }
        }

        if self.play.tray.pending() > 0 {
            let before = self.board_snapshot();
            self.play.take_garbage(self.opponent);
            let cells = self.arrived(&before);
            if !cells.is_empty() {
                self.events.push(GameEvent::AttackReceived { cells });
                self.state = State::Delivering(rules::DELIVERY_DELAY);
                return;
            }
        }
        self.next_pair();
    }

    fn board_snapshot(&self) -> Vec<Point> {
        self.play.board.occupied().map(|(at, _)| at).collect()
    }

    /// the cells that are on the board now and were not before
    fn arrived(&self, before: &[Point]) -> Vec<PlacedCell> {
        let points: Vec<Point> = self
            .play
            .board
            .occupied()
            .map(|(at, _)| at)
            .filter(|at| !before.contains(at))
            .collect();
        self.placed(&points)
    }

    fn next_pair(&mut self) {
        self.state = State::Spawning(rules::SPAWN_DELAY);
    }

    fn tick_falling(&mut self, delta: Duration, mut lock: Duration) {
        let Some(mut pair) = self.pair else { return };
        let interval = self.fall_interval();
        self.fall += delta;
        while self.fall >= interval {
            self.fall -= interval;
            if pair.fall(&self.play.board) {
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

        if pair.is_resting(&self.play.board) {
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
        let moved = f(&mut pair, &self.play.board);
        // a refused rotation still has to be remembered, for the quick turn
        self.pair = Some(pair);
        if moved {
            if let State::Falling { lock } = &mut self.state {
                *lock = Duration::ZERO;
            }
        }
        moved
    }
}

impl engine::game::Game for Game {
    fn game_id(&self) -> GameId {
        GAME_ID
    }

    fn update(&mut self, delta: Duration) {
        // the round clock runs whatever the board is doing, because damage grows with it
        if !matches!(self.state, State::GameOver) {
            self.round += delta;
        }
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
            State::Delivering(left) => {
                if let Some(left) = left.checked_sub(delta).filter(|d| !d.is_zero()) {
                    self.state = State::Delivering(left);
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
        if self.with_pair(|pair, board| pair.rotate(board, clockwise) != RotateOutcome::Blocked) {
            self.events.push(GameEvent::Rotate);
        }
    }

    /// Taking soft drop up or letting it go carries the pair's *position* across the change,
    /// not the time it has banked towards the next row - the same trap Puyo Rusto documents,
    /// and for the same reason: this game slides between cells too.
    fn set_soft_drop(&mut self, soft_drop: bool) {
        if soft_drop == self.soft_drop {
            return;
        }
        let travelled = self.fall.as_secs_f64() / self.fall_interval().as_secs_f64();
        self.soft_drop = soft_drop;
        self.fall = self.fall_interval().mul_f64(travelled.clamp(0.0, 1.0));
    }

    /// Zero while the pair is resting: the fall timer goes on accumulating there, and drawing
    /// it would sink a settled pair into the stack and snap it back once a fall interval.
    fn fall_progress(&self) -> f64 {
        let Some(pair) = self.pair else { return 0.0 };
        if !matches!(self.state, State::Falling { .. }) || pair.is_resting(&self.play.board) {
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
        let cells = Game::pair_cells(&pair);
        let dropped_rows = pair.hard_drop(&self.play.board);
        self.pair = Some(pair);
        self.events.push(GameEvent::HardDrop {
            cells,
            dropped_rows,
        });
        self.lock_pair(true);
    }

    /// Puzzle Fighter has no hold box and this game does not add one. The pair in play and the
    /// one NEXT box are the whole of what a player is given to plan with, and the game's
    /// difficulty is built around that.
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
            for (at, id) in Game::pair_cells(&pair) {
                if at == point {
                    return Cell::Active(id);
                }
            }
            for (at, id) in Game::pair_cells(&pair.ghost(&self.play.board)) {
                if at == point {
                    return Cell::Ghost(id);
                }
            }
        }
        match self.play.board.get(point) {
            None => Cell::Empty,
            // a counter gem is the one thing on this board the player did not put there
            Some(gem @ Gem::Counter { .. }) => Cell::Garbage(gem.loose_id()),
            Some(gem) => Cell::Stack(gem.id(self.play.board.power_mask(point))),
        }
    }

    fn queue(&self) -> Vec<PieceId> {
        // `peek` fills lazily, and `queue` only has `&self` - so the NEXT box is read off the
        // pairs already dealt into the buffer, which `next_pair` keeps topped up
        self.random
            .peeked()
            .into_iter()
            .map(PieceId::from)
            .collect()
    }

    fn held(&self) -> Option<PieceId> {
        None
    }

    fn metric(&self, kind: MetricKind) -> Option<u32> {
        match kind {
            MetricKind::Score => Some(self.play.score),
            // there is no level in this game, so the speed step stands in for one
            MetricKind::Level => Some(self.speed_index),
            MetricKind::Chain => Some(self.play.best_chain),
            MetricKind::Lines | MetricKind::Viruses => None,
        }
    }

    fn score(&self) -> u32 {
        self.play.score
    }

    fn set_score(&mut self, score: u32) {
        self.play.score = score;
    }

    fn speed_index(&self) -> u32 {
        self.speed_index
    }

    fn set_speed_index(&mut self, index: u32) {
        self.speed_index = index;
    }

    fn stage_state(&self) -> StageState {
        match self.state {
            State::GameOver => StageState::GameOver,
            _ if self.stage_complete => StageState::StageComplete,
            _ => StageState::Playing,
        }
    }

    /// The board is never cleared away between stages: a speed step is something that happens
    /// to a round in progress, not a new round. Dr. Rustario's bottle is the odd one out here.
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
        self.events.push(GameEvent::SpeedUp);
        Ok(())
    }

    /// Garbage arriving does not land immediately: it joins the tray, where this player's own
    /// next break can still cancel it. That is the offset this game shares with Puyo, and it
    /// is why the tray is worth showing.
    fn receive_attack(&mut self, attack: Attack) {
        let gems = attack.strength_for(GAME_ID);
        if gems > 0 {
            // an attack that arrived having been cancelled down lands on a shorter fuse; this
            // is the plain case, so it lands on the full five
            self.play.tray.receive(gems, false);
        }
    }

    fn pending_attacks(&self) -> Vec<CellId> {
        // one icon per counter gem still to fall, drawn in the colour the sender will send
        let pending = self.play.tray.pending().min(board::VISIBLE_CELLS);
        (0..pending)
            .map(|i| {
                let color = self.opponent.drop_color(0, (i % COLUMNS) as usize);
                Gem::counter(color, crate::game::cell::COUNTER_COUNTDOWN).loose_id()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::board::tests::board;
    use crate::game::cell::{GemColor, GemPair, Half};
    use crate::game::random::Seed;
    use engine::game::Game as _;

    fn game() -> Game {
        Game::new(
            Fighter::Ryu,
            Difficulty::Normal,
            0,
            GameRandom::from_seed(Seed::from_u64(42)),
        )
    }

    /// run the game until `f` says so, or give up - so a test cannot hang on a state that
    /// never ends
    fn run_until(game: &mut Game, mut f: impl FnMut(&Game) -> bool) -> bool {
        for _ in 0..20_000 {
            if f(game) {
                return true;
            }
            game.update(Duration::from_millis(16));
        }
        false
    }

    #[test]
    fn a_pair_enters_over_the_drop_alley_and_falls() {
        let mut game = game();
        assert!(run_until(&mut game, |g| g.pair.is_some()));
        let pair = game.pair.expect("a pair");
        assert_eq!(pair.pivot(), SPAWN);
        let start = pair.pivot().y;
        assert!(run_until(&mut game, |g| g
            .pair
            .is_some_and(|p| p.pivot().y > start)));
    }

    /// the queue shows the pair that comes next, and it is the one that arrives
    #[test]
    fn the_next_box_shows_the_pair_that_arrives() {
        let mut game = game();
        let next = game.queue();
        assert_eq!(next.len(), crate::game::random::PEEK_SIZE);
        let first = game.pair.expect("a pair").piece();
        game.hard_drop();
        assert!(run_until(&mut game, |g| g
            .pair
            .is_some_and(|p| p.piece() != first)));
        assert_eq!(vec![PieceId::from(game.pair.unwrap().piece())], next);
    }

    /// A crash gem landing on its own colour breaks, even straight out of the deal.
    ///
    /// The "a pair never self-destructs" rule is narrower than it looks: it demotes only when
    /// **both** halves are the same crash gem. A plain gem under a crash gem of that colour is
    /// an ordinary two-gem break, and one arrives within a pair or two of any seed.
    #[test]
    fn a_gem_under_its_own_crash_gem_breaks_where_it_lands() {
        let mut game = game();
        game.play.board = crate::game::board::Board::new();
        game.pair = Some(Pair::new(
            SPAWN,
            GemPair::new(Half::Plain(GemColor::Blue), Half::Crash(GemColor::Blue)),
        ));
        game.state = State::Falling {
            lock: Duration::ZERO,
        };
        game.drain_events();
        game.hard_drop();
        run_until(&mut game, |g| matches!(g.state, State::Spawning(_)));
        assert!(game
            .drain_events()
            .iter()
            .any(|e| matches!(e, GameEvent::Clear { .. })));
    }

    /// a hard drop locks the pair and the chain loop takes over
    #[test]
    fn a_hard_drop_locks_the_pair_and_resolves() {
        let mut game = game();
        game.hard_drop();
        let events = game.drain_events();
        assert!(events
            .iter()
            .any(|e| matches!(e, GameEvent::HardDrop { .. })));
        assert!(events.iter().any(|e| matches!(e, GameEvent::Lock { .. })));
        assert!(matches!(game.state, State::Resolving { .. }));
        assert!(
            game.pair.is_none(),
            "the player has nothing while it resolves"
        );
    }

    /// **the chain is watched, not computed.** One pass fires, then the game waits out the
    /// forty frame erase animation before the next
    #[test]
    fn a_chain_is_stepped_one_pass_at_a_time() {
        let mut game = game();
        game.play.board = board(&["..G.", "rrrR", "gg.."]);
        game.pair = None;
        game.state = State::Resolving {
            chain: 0,
            timer: Duration::ZERO,
        };
        let mut passes = vec![];
        run_until(&mut game, |g| {
            matches!(g.state, State::Spawning(_) | State::GameOver)
        });
        for event in game.drain_events() {
            if let GameEvent::Clear { detail, .. } = event {
                passes.push(ClearDetail::from(detail).chain);
            }
        }
        assert_eq!(passes, vec![1, 2], "two passes, in order");
    }

    /// a break's attack is offset against what is already falling towards this player, and
    /// only the remainder is sent
    #[test]
    fn an_attack_is_offset_against_the_tray_before_it_is_sent() {
        let mut game = game();
        game.play.board = board(&["rrg", "rRg"]);
        game.play.tray.receive(2, false);
        game.pair = None;
        game.state = State::Resolving {
            chain: 0,
            timer: Duration::ZERO,
        };
        run_until(&mut game, |g| matches!(g.state, State::Spawning(_)));
        let sent: Vec<u32> = game
            .drain_events()
            .into_iter()
            .filter_map(|e| match e {
                GameEvent::AttackSent(attack) => Some(attack.strength_for(GAME_ID)),
                _ => None,
            })
            .collect();
        assert_eq!(sent, vec![2], "four thrown, two cancelled on the way out");
        assert_eq!(game.pending_counter_gems(), 0);
    }

    /// garbage that survives the offset lands, and the engine is told so it can be watched in
    ///
    /// Driven straight into the chain loop with nothing on the board, so that the pair dealt
    /// cannot break and cancel the tray on its way past - which is what a hard drop here
    /// happens to do on this seed, and is the rule working rather than the test.
    #[test]
    fn garbage_that_survives_the_offset_lands() {
        let mut game = game();
        game.play.tray.receive(6, false);
        game.pair = None;
        game.state = State::Resolving {
            chain: 0,
            timer: Duration::ZERO,
        };
        game.drain_events();
        assert!(run_until(&mut game, |g| matches!(
            g.state,
            State::Delivering(_)
        )));
        let received: usize = game
            .drain_events()
            .into_iter()
            .filter_map(|e| match e {
                GameEvent::AttackReceived { cells } => Some(cells.len()),
                _ => None,
            })
            .sum();
        assert_eq!(received, 6);
    }

    /// **game over is the Drop Alley blocking.** Nothing else ends a round.
    #[test]
    fn the_game_ends_when_the_drop_alley_is_blocked() {
        let mut game = game();
        for y in 0..ROWS as i32 {
            game.play.board.set(
                Point::new(board::DROP_ALLEY, y),
                Some(Gem::plain(GemColor::Green)),
            );
        }
        game.pair = None;
        game.state = State::Spawning(Duration::ZERO);
        game.update(Duration::from_millis(16));
        assert_eq!(game.stage_state(), StageState::GameOver);
    }

    /// the round clock is what every attack in this game grows with, so it has to run
    #[test]
    fn the_round_clock_runs_and_stops_at_the_cap() {
        let mut game = game();
        assert_eq!(game.round_seconds(), 0);
        game.update(Duration::from_secs(2));
        assert_eq!(game.round_seconds(), 2);
        game.round = Duration::from_secs(10_000);
        assert_eq!(game.round_seconds(), rules::MAX_ROUND_SECONDS);
    }

    /// a counter gem draws as garbage rather than as part of the stack, and a power gem cell
    /// draws joined to the rest of its rectangle
    #[test]
    fn the_board_reports_cells_the_way_a_theme_draws_them() {
        let mut game = game();
        game.play.board = board(&["rr", "rr", "4."]);
        game.pair = None;
        crate::game::gems::form_power_gems(&mut game.play.board, &mut game.play.ids);
        let floor = ROWS as i32 - 1;
        assert!(matches!(game.cell(Point::new(0, floor)), Cell::Garbage(_)));
        let Cell::Stack(id) = game.cell(Point::new(0, floor - 1)) else {
            panic!("the power gem is stack")
        };
        let crate::game::cell::GemSprite::Plain { mask, .. } = id.into() else {
            panic!("a plain gem")
        };
        assert!(
            mask.has(PowerMask::UP) && mask.has(PowerMask::RIGHT),
            "the bottom left of the rectangle is joined up and right"
        );
    }

    /// the pair in play always draws unjoined, whatever it is standing over
    #[test]
    fn the_pair_in_play_never_draws_joined() {
        let mut game = game();
        game.play.board = board(&["rr", "rr"]);
        crate::game::gems::form_power_gems(&mut game.play.board, &mut game.play.ids);
        let pair = Pair::new(
            SPAWN,
            GemPair::new(Half::Plain(GemColor::Red), Half::Plain(GemColor::Red)),
        );
        game.pair = Some(pair);
        let Cell::Active(id) = game.cell(SPAWN) else {
            panic!("the pair is active")
        };
        assert_eq!(id, Gem::plain(GemColor::Red).loose_id());
    }

    /// a speed step is something that happens to a round in progress: the board is not cleared
    /// away and the gems keep counting
    #[test]
    fn a_speed_step_leaves_the_board_alone() {
        let mut game = game();
        game.stage_gems = rules::GEMS_PER_STAGE;
        game.play.board = board(&["rrg", "rRg"]);
        game.pair = None;
        game.state = State::Resolving {
            chain: 0,
            timer: Duration::ZERO,
        };
        run_until(&mut game, |g| g.stage_complete);
        assert_eq!(game.stage_state(), StageState::StageComplete);
        assert_eq!(game.stage_transition(), StageTransition::Seamless);
        game.next_stage().expect("a next stage");
        assert_eq!(game.completed_stages(), 1);
        assert!(
            !game.play.board.is_empty(),
            "and the greens are still there"
        );
    }
}
