//! The agent that plays a board: it thinks about the pair in play across as many frames as it
//! needs, then presses the keys to put it where it decided, at whatever rate its difficulty
//! allows.
//!
//! **Thinking is spread over frames.** The search is the expensive thing this game does, and
//! doing it all in the frame the pair spawns in means one stall of a millisecond on a desktop
//! and a tenth of a second on a handheld. But the agent has no need to answer in a frame: a
//! pair takes a second or more to fall, which is sixty frames, so [`Search`] is stepped
//! once per frame and the answer is taken when it is ready. The search is not made smaller to
//! fit a slow device - it is taken in pieces, which costs nothing at all.
//!
//! Three things follow, and all are handled below. The pair goes on **falling** while the
//! search runs, so the keys the search worked out from where the pair *was* are no longer the
//! keys to press - they are worked out again from where it is. The pair may come to **rest**
//! before the search is done, on a board too full to fall through, so the search has to be
//! interruptible: it is, because every placement is scored before the first step and only
//! sharpened after it. And the **tray** can fill in the meantime, so what is waiting to land
//! is read when the answer is taken rather than when the thinking started.

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

/// How full the spawn column has to get before the ai stops building and fires whatever it
/// has. Three rows short of the death square: enough to take the nuisance a chain of its own
/// will draw, and not so early that it gives up on every chain it starts.
const PRESSED_HEIGHT: usize = VISIBLE - 3;

/// How much taller a tray of `pending` is about to make every column.
///
/// Only [`MAX_DROP`](crate::game::nuisance::MAX_DROP) of it falls at once, and it falls full
/// rows first with the remainder scattered - so this is the part of it that is certain to
/// land on the spawn column, and the part the ai can count on being buried by. It is what
/// makes a board that is comfortable now count as pressed: a chain fired *before* the rock
/// lands is worth one fired after it, and there is no fired-after in Tsu, because the tray
/// drops the moment this pair locks.
fn incoming_rows(pending: u32) -> usize {
    (pending.min(crate::game::nuisance::MAX_DROP) / COLUMNS) as usize
}

/// What the agent is doing about the pair in play.
enum Thinking {
    /// there is no pair, or the one there is has not been looked at yet
    Idle,
    /// a search is under way, one step a frame
    Running(Box<Search>),
    /// The keys are queued. The search is kept rather than dropped, because one thing can
    /// still change this pair's mind - see [`PuyoAiAgent::act`]. It is `None` when the keys
    /// were not a search's to begin with, which is the placeholder and the board with nowhere
    /// left to put anything; neither has a mind to change.
    Decided {
        search: Option<Box<Search>>,
        /// what was in the tray when it decided, so a deeper one can be noticed
        pending: u32,
    },
}

impl Thinking {
    /// keys queued by something that was not a search: there is nothing to reconsider, so a
    /// tray that can never be deeper than this is what it remembers seeing
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
    /// a search to run in place of the row's own, which only the harness ever sets - see
    /// [`with_search`](Self::with_search)
    search: Option<SearchConfig>,
    /// how many pairs this agent has committed to a chain that cancels the tray, which is
    /// what `ga puyo duel` reports and the only visible sign that a row is answering at all
    answers: u32,
    /// how many pairs it committed with anything at all waiting in the tray. The ceiling on
    /// [`answers`](Self::answers), and the number that says whether a row that never answers
    /// is being fussy or is simply never asked
    trays: u32,
    /// how many pairs it fired on because of what the tray was about to do to the board
    /// rather than because it had the chain it wanted - see [`incoming_rows`]
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

    /// Play this row's weights with a search of someone else's choosing.
    ///
    /// Nothing in the game does this: a difficulty is a whole row, and a row is its weights
    /// and its search together. It exists so that `ga puyo duel` can move one dial of one row
    /// and play the result, which is how a [`SearchConfig`] gets a number in it at all - the
    /// alternative is editing a const and rebuilding for every point of a sweep.
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

    /// forget whatever was queued and whatever was being thought about; the board it was meant
    /// for has gone
    pub fn reset(&mut self) {
        self.keys.abandon();
        self.thinking = Thinking::Idle;
    }

    /// drive `game` for one frame
    pub fn act(&mut self, game: &mut Game, delta: Duration) {
        self.keys.tick(delta);

        let Some(pair) = game.pair() else {
            // between pairs: the board is popping, settling or taking its nuisance, and
            // anything still queued belonged to a pair that has already locked
            self.reset();
            return;
        };

        if matches!(self.thinking, Thinking::Idle) {
            self.begin(game);
        }
        match &mut self.thinking {
            Thinking::Running(search) => {
                // the pair is about to lock: take the best answer the search has got to, which
                // is never nothing, because every placement was scored before the first step
                let out_of_time = pair.is_resting(game.board());
                if !out_of_time {
                    search.step();
                }
                if out_of_time || search.finished() {
                    self.commit(game);
                }
            }
            // **An attack landed while the keys were still being pressed.** Deciding once and
            // then looking away would make answering nearly useless: the search finishes a
            // dozen frames after the pair spawns, an ai opponent takes a third of a second per
            // key after that, and classic Tsu offset drops the tray the moment this pair
            // locks. So the pair in play when an attack arrives is the only pair that can
            // answer it, and it has usually been committed to something else by then. The
            // search is still here and every placement in it is still scored, so changing its
            // mind costs one re-rank and a fresh route.
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

    /// Start thinking about the pair in play.
    ///
    /// What the ai is allowed to know is exactly what a player sitting in front of the board
    /// knows: the pair in play, the two behind it, and the board. Not the pool the pairs are
    /// dealt from, and not the seed.
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
            // nowhere to put it, which means the game is over in a frame or two; press
            // something rather than freezing
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

    /// Take the search's answer and turn it into keys.
    ///
    /// The route is worked out again from where the pair is *now* rather than reused from
    /// where it was when the search began, because it has been falling all the while and a row
    /// lower is a different set of rotations. If the placement it settled on can no longer be
    /// reached at all, the next one down the order is taken instead - which is why
    /// [`Search::ranking`] hands back an order rather than a winner.
    ///
    /// **What is waiting in the tray is read here rather than at
    /// [`begin`](Self::begin)**, for the same reason the route is: a tray fills while the
    /// search runs, and an attack that arrived a frame ago is exactly the one this pair still
    /// has time to answer.
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
        // the board is fine and is about to not be
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
                // every placement in an answering order fires a chain that covers the tray,
                // so falling back down the order is still an answer
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

    /// play one board out and report what it managed, which is the measure everything in the
    /// ladder is ranked on
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

    /// the placeholder is not meant to play well, but it is meant to *play*
    #[test]
    fn the_placeholder_keeps_placing_pairs() {
        let (_, placed) = play(PuyoAiKind::Placeholder, 7, 40);
        assert!(placed > 10, "only {placed} pairs placed");
    }

    /// two runs of a demo look the same, which is what a fixed seed is for
    #[test]
    fn the_placeholder_plays_the_same_game_twice() {
        assert_eq!(
            play(PuyoAiKind::Placeholder, 7, 30),
            play(PuyoAiKind::Placeholder, 7, 30)
        );
    }

    /// the whole of phase 4 in one assertion: a brain that reads the board outscores one that
    /// does not, on the same seeds, over the same pairs
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

    /// what a tray is certain to add to every column, including the one pairs spawn in
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

    /// **The tray is read again after the keys are queued.**
    ///
    /// The search finishes a dozen frames after a pair spawns and a fielded opponent then
    /// takes a third of a second per key, so an attack almost always arrives after the ai has
    /// decided. Deciding once and looking away would let nearly all of them through: classic
    /// Tsu offset drops the tray the moment this pair locks, so the pair in play when an
    /// attack arrives is the only pair that can do anything about it.
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

    /// A step of the search happens in the middle of a frame, so it has to fit in one - and
    /// with room to spare, since a handheld is several times slower than whatever this ran on.
    ///
    /// Only meaningful with optimisations on - a debug build is an order of magnitude off and
    /// would either fail always or have to be given a bound that means nothing - so this is
    /// the one test in the crate that only runs in release.
    #[test]
    #[cfg(not(debug_assertions))]
    fn no_step_of_the_hardest_row_comes_near_a_frame() {
        use crate::game::ai::beam::Search;
        use crate::game::ai::field::{of_color, Field};
        use crate::game::ai::placement::root_moves;
        use crate::game::cell::PuyoPiece;

        let mut game = game();
        let mut agent = PuyoAiAgent::of(PuyoAiKind::best());
        // play a while first, so the board it is timed on is a real one rather than empty
        for _ in 0..4_000 {
            agent.act(&mut game, Duration::from_millis(8));
            game.update(Duration::from_millis(8));
            game.drain_events();
            if matches!(game.stage_state(), StageState::GameOver) {
                break;
            }
        }

        // run on until a pair is actually in play rather than the board mid chain
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

    /// every row plays, and none of them freezes on a board it does not like
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
