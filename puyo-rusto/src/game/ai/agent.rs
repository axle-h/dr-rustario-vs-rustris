//! The agent that plays a board: it steps a [`Search`] once a frame while the pair falls, then
//! presses the keys at its difficulty's rate. The route and the tray are read when the answer is
//! taken rather than when thinking began, since the pair falls and the tray fills meanwhile.

use crate::game::ai::beam::{Plan, Search, SearchConfig};
use crate::game::ai::field::{of_color, Field, VISIBLE};
use crate::game::ai::input_sequence::Translation;
use crate::game::ai::placement::root_moves;
use crate::game::ai::PuyoAiKind;
use crate::game::board::{COLUMNS, SPAWN};
use crate::game::cell::PuyoPiece;
use crate::game::Game;
use engine::ai::KeyPacer;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::time::Duration;

/// what a demo replays from, so two runs of the same demo look the same
const PLACEHOLDER_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// How full the spawn column gets before the ai fires whatever it has: three rows short of the
/// death square, room for the nuisance its own chain will draw.
const PRESSED_HEIGHT: usize = VISIBLE - 3;

/// How much taller a tray of `pending` is certain to make every column, counted into the spawn
/// column's height so a threatened board is pressed and fires now: Tsu offset drops the tray as
/// soon as this pair locks.
fn incoming_rows(pending: u32) -> usize {
    (pending.min(crate::game::nuisance::MAX_DROP) / COLUMNS) as usize
}

/// What the agent is doing about the pair in play.
enum Thinking {
    /// there is no pair, or the one there is has not been looked at yet
    Idle,
    /// a search is under way, one step a frame
    Running(Box<Search>),
    /// The keys are queued, and the search is kept so a deeper tray can change its mind. It is
    /// `None` for keys that were not a search's.
    Decided {
        search: Option<Box<Search>>,
        /// what was in the tray when it decided, so a deeper one can be noticed
        pending: u32,
    },
}

impl Thinking {
    /// keys queued by something that was not a search, so no tray is ever deeper than it saw
    fn blind() -> Self {
        Thinking::Decided {
            search: None,
            pending: u32::MAX,
        }
    }
}

pub struct PuyoAiAgent {
    brain: PuyoAiKind,
    keys: KeyPacer<Translation>,
    thinking: Thinking,
    rng: ChaCha8Rng,
    /// a search to run in place of the row's own, set only by the harness
    search: Option<SearchConfig>,
    /// how many pairs this agent committed to a chain that cancels the tray
    answers: u32,
    /// how many pairs it committed with anything waiting in the tray, the ceiling on `answers`
    trays: u32,
    /// how many pairs it fired on because the tray was about to bury it, see [`incoming_rows`]
    crowded: u32,
}

impl Default for PuyoAiAgent {
    fn default() -> Self {
        Self::of(PuyoAiKind::default())
    }
}

impl PuyoAiAgent {
    pub fn of(brain: PuyoAiKind) -> Self {
        Self {
            brain,
            keys: KeyPacer::new(Duration::ZERO),
            thinking: Thinking::Idle,
            rng: ChaCha8Rng::seed_from_u64(PLACEHOLDER_SEED),
            search: None,
            answers: 0,
            trays: 0,
            crowded: 0,
        }
    }

    pub fn with_key_delay(mut self, key_delay: Duration) -> Self {
        self.keys = KeyPacer::new(key_delay);
        self
    }

    /// Play this row's weights with another search, so `ga puyo duel` can sweep one dial.
    pub fn with_search(mut self, search: SearchConfig) -> Self {
        self.search = Some(search);
        self
    }

    /// how many times this agent has spent a chain on what was waiting in its tray
    pub fn answers(&self) -> u32 {
        self.answers
    }

    /// how many pairs it decided with anything waiting in its tray at all
    pub fn trays(&self) -> u32 {
        self.trays
    }

    /// how many pairs it fired on because the tray was about to bury it
    pub fn crowded(&self) -> u32 {
        self.crowded
    }

    /// forget whatever was queued or being thought about
    pub fn reset(&mut self) {
        self.keys.abandon();
        self.thinking = Thinking::Idle;
    }

    /// drive `game` for one frame
    pub fn act(&mut self, game: &mut Game, delta: Duration) {
        self.keys.tick(delta);

        let Some(pair) = game.pair() else {
            // between pairs, anything still queued belonged to a pair that has locked
            self.reset();
            return;
        };

        if matches!(self.thinking, Thinking::Idle) {
            self.begin(game);
        }
        match &mut self.thinking {
            Thinking::Running(search) => {
                // the pair is about to lock: take the best answer so far, which always exists
                let out_of_time = pair.is_resting(game.board());
                if !out_of_time {
                    search.step();
                }
                if out_of_time || search.finished() {
                    self.commit(game);
                }
            }
            // an attack landed after deciding: this pair is the only one that can answer it
            // before Tsu offset drops the tray, so re-rank the kept search
            Thinking::Decided { pending, .. } if game.pending_nuisance() > *pending => {
                self.keys.abandon();
                self.commit(game);
            }
            _ => {}
        }

        // a speed limited agent gets one key here and then has to wait; at full speed the
        // whole sequence goes in this frame
        while let Some(key) = self.keys.next_key() {
            match key {
                Translation::Left => engine::game::Game::left(game),
                Translation::Right => engine::game::Game::right(game),
                Translation::RotateClockwise => engine::game::Game::rotate(game, true),
                Translation::RotateAnticlockwise => engine::game::Game::rotate(game, false),
                Translation::HardDrop => engine::game::Game::hard_drop(game),
            }
        }
    }

    /// Start thinking about the pair in play. The ai sees what a player sees: the pair, the two
    /// behind it and the board, never the pool or the seed.
    fn begin(&mut self, game: &Game) {
        let PuyoAiKind::Scorer(row) = self.brain else {
            self.place_at_random(game);
            self.thinking = Thinking::blind();
            return;
        };
        let skill = &crate::game::ai::skill::ROWS[row % crate::game::ai::SKILLS];
        let Some(pair) = game.pair() else { return };
        let roots = root_moves(game.board(), pair);
        if roots.is_empty() {
            // nowhere to put it: press something rather than freezing
            self.place_at_random(game);
            self.thinking = Thinking::blind();
            return;
        }

        let queue: Vec<[u8; 2]> = engine::game::Game::queue(game)
            .into_iter()
            .map(|id| {
                let piece: PuyoPiece = id.into();
                [of_color(piece.pivot), of_color(piece.child)]
            })
            .collect();

        self.thinking = Thinking::Running(Box::new(Search::new(
            &Field::from_board(game.board()),
            roots,
            &queue,
            skill.weights,
            self.search.unwrap_or(skill.search),
        )));
    }

    /// Take the search's answer and turn it into keys, routed from where the pair is now. An
    /// unreachable placement falls back to the next one down the order.
    fn commit(&mut self, game: &mut Game) {
        let search = match std::mem::replace(&mut self.thinking, Thinking::Idle) {
            Thinking::Running(search)
            | Thinking::Decided {
                search: Some(search),
                ..
            } => search,
            other => {
                self.thinking = other;
                return;
            }
        };
        let Some(pair) = game.pair() else { return };
        let field = Field::from_board(game.board());
        let pending = game.pending_nuisance();
        let height = field.height(SPAWN.x as usize) as usize;
        let pressed = height >= PRESSED_HEIGHT;
        let crowded = !pressed && height + incoming_rows(pending) >= PRESSED_HEIGHT;
        self.trays += u32::from(pending > 0);
        self.crowded += u32::from(crowded);
        let (ranked, plan) = search.ranking(pressed || crowded, pending);
        let routes = root_moves(game.board(), pair);

        let chosen = ranked.iter().find_map(|candidate| {
            let wanted = search.candidates()[*candidate].root.drop;
            routes.iter().find(|route| route.drop == wanted)
        });
        match chosen {
            Some(route) => {
                // every placement in an answering order covers the tray, so falling back is
                // still an answer
                if plan == Plan::Answer {
                    self.answers += 1;
                }
                self.keys.queue(route.inputs.clone())
            }
            None => self.place_at_random(game),
        }
        self.thinking = Thinking::Decided {
            search: Some(search),
            pending,
        };
    }

    fn place_at_random(&mut self, game: &Game) {
        let from_column = game.pair().map(|pair| pair.pivot().x).unwrap_or(SPAWN.x);
        let column = self.rng.random_range(0..COLUMNS as i32);
        let turns = self.rng.random_range(0..4);
        let mut keys = vec![Translation::RotateClockwise; turns];
        let step = if column < from_column {
            Translation::Left
        } else {
            Translation::Right
        };
        keys.extend(std::iter::repeat_n(
            step,
            (column - from_column).unsigned_abs() as usize,
        ));
        keys.push(Translation::HardDrop);
        self.keys.queue(keys);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::random::GameRandom;
    use crate::game::rules::Difficulty;
    use engine::game::random::Seed;
    use engine::game::{Game as _, StageState};
    #[cfg(not(debug_assertions))]
    use std::time::Instant;

    fn game_of(seed: u64) -> Game {
        Game::new(
            Difficulty::Normal,
            0,
            GameRandom::from_seed(Seed::from_u64(seed), Difficulty::Normal.colors()),
            crate::game::cell::PuyoSkin::FIRST,
        )
    }

    /// only the release-only timing test wants a game of its own
    #[cfg(not(debug_assertions))]
    fn game() -> Game {
        game_of(7)
    }

    /// play one board out and report what it managed
    fn play(brain: PuyoAiKind, seed: u64, pairs: u32) -> (u32, u32) {
        let mut game = game_of(seed);
        let mut agent = PuyoAiAgent::of(brain);
        let mut placed = 0;
        for _ in 0..400_000 {
            if placed >= pairs || matches!(game.stage_state(), StageState::GameOver) {
                break;
            }
            agent.act(&mut game, Duration::from_millis(8));
            game.update(Duration::from_millis(8));
            placed += game
                .drain_events()
                .iter()
                .filter(|e| matches!(e, engine::game::GameEvent::Lock { .. }))
                .count() as u32;
        }
        (game.score(), placed)
    }

    #[test]
    fn the_placeholder_keeps_placing_pairs() {
        let (_, placed) = play(PuyoAiKind::Placeholder, 7, 40);
        assert!(placed > 10, "only {placed} pairs placed");
    }

    /// a demo replays identically from its fixed seed
    #[test]
    fn the_placeholder_plays_the_same_game_twice() {
        assert_eq!(
            play(PuyoAiKind::Placeholder, 7, 30),
            play(PuyoAiKind::Placeholder, 7, 30)
        );
    }

    /// a brain that reads the board outscores one that does not, on the same seeds and pairs
    #[test]
    fn the_scorer_outplays_the_placeholder() {
        let scorer: u32 = (0..2)
            .map(|seed| play(PuyoAiKind::best(), seed, 25).0)
            .sum();
        let placeholder: u32 = (0..2)
            .map(|seed| play(PuyoAiKind::Placeholder, seed, 25).0)
            .sum();
        assert!(
            scorer > placeholder * 2,
            "scorer {scorer} against placeholder {placeholder}"
        );
    }

    #[test]
    fn only_a_drops_worth_of_the_tray_counts_towards_being_pressed() {
        assert_eq!(incoming_rows(0), 0);
        assert_eq!(
            incoming_rows(5),
            0,
            "a scattered remainder misses most columns"
        );
        assert_eq!(incoming_rows(6), 1);
        assert_eq!(incoming_rows(30), 5, "a rock is five rows");
        assert_eq!(
            incoming_rows(400),
            5,
            "and no more than a rock ever falls at once"
        );
    }

    /// a rock arriving after the keys are queued is still read
    #[test]
    fn an_attack_that_lands_after_it_has_decided_is_still_looked_at() {
        use engine::game::Attack;

        let mut game = game_of(7);
        let mut agent =
            PuyoAiAgent::of(PuyoAiKind::best()).with_key_delay(Duration::from_millis(400));
        for _ in 0..200 {
            agent.act(&mut game, Duration::from_millis(8));
            game.update(Duration::from_millis(8));
            game.drain_events();
            if matches!(agent.thinking, Thinking::Decided { .. }) {
                break;
            }
        }
        assert!(
            matches!(agent.thinking, Thinking::Decided { .. }),
            "the ai never settled on a placement"
        );
        assert_eq!(agent.trays(), 0, "nothing was waiting when it decided");
        assert!(game.pair().is_some(), "the pair is still being walked over");

        game.receive_attack(Attack::new(crate::game::GAME_ID, 30));
        agent.act(&mut game, Duration::from_millis(8));
        assert_eq!(
            agent.trays(),
            1,
            "the rock that arrived after it decided was never looked at"
        );
    }

    /// A step of the search fits well inside one frame. Release only, since a debug build is an
    /// order of magnitude slower.
    #[test]
    #[cfg(not(debug_assertions))]
    fn no_step_of_the_hardest_row_comes_near_a_frame() {
        use crate::game::ai::beam::Search;
        use crate::game::ai::field::{of_color, Field};
        use crate::game::ai::placement::root_moves;
        use crate::game::cell::PuyoPiece;

        let mut game = game();
        let mut agent = PuyoAiAgent::of(PuyoAiKind::best());
        // play a while first so the board is not empty
        for _ in 0..4_000 {
            agent.act(&mut game, Duration::from_millis(8));
            game.update(Duration::from_millis(8));
            game.drain_events();
            if matches!(game.stage_state(), StageState::GameOver) {
                break;
            }
        }

        // run on until a pair is in play rather than a chain resolving
        let mut pair = game.pair();
        while pair.is_none() {
            game.update(Duration::from_millis(8));
            game.drain_events();
            pair = game.pair();
        }
        let pair = pair.expect("a pair in play");
        let skill = crate::game::ai::skill::nth_weakest(crate::game::ai::SKILLS - 1);
        let queue: Vec<[u8; 2]> = engine::game::Game::queue(&game)
            .into_iter()
            .map(|id| {
                let piece: PuyoPiece = id.into();
                [of_color(piece.pivot), of_color(piece.child)]
            })
            .collect();

        let started = Instant::now();
        let mut search = Search::new(
            &Field::from_board(game.board()),
            root_moves(game.board(), pair),
            &queue,
            skill.weights,
            skill.search,
        );
        let mut worst = started.elapsed();
        let mut steps = 0;
        loop {
            let started = Instant::now();
            let done = search.step();
            worst = worst.max(started.elapsed());
            steps += 1;
            if done {
                break;
            }
        }
        assert!(
            worst < Duration::from_millis(4),
            "the worst of {steps} steps took {worst:?}"
        );
    }

    #[test]
    fn every_row_plays_a_board_out() {
        for row in 0..crate::game::ai::SKILLS {
            let (_, placed) = play(PuyoAiKind::Scorer(row), 3, 12);
            assert_eq!(
                placed,
                12,
                "row {} placed only {placed} pairs",
                crate::game::ai::skill::ROWS[row].name
            );
        }
    }
}
