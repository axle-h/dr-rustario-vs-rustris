//! The agent that plays a board: it picks a placement for each pair as it spawns and then
//! presses the keys to reach it, at whatever rate its difficulty allows.

use crate::game::ai::PuyoAiKind;
use crate::game::board::COLUMNS;
use crate::game::Game;
use engine::ai::KeyPacer;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::time::Duration;

/// how the agent asks for one board input
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Key {
    Left,
    Right,
    RotateClockwise,
    HardDrop,
}

/// what a demo replays from, so two runs of the same demo look the same
const PLACEHOLDER_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

pub struct PuyoAiAgent {
    brain: PuyoAiKind,
    keys: KeyPacer<Key>,
    /// whether the pair in play has already been given a plan
    decided: bool,
    rng: ChaCha8Rng,
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
            decided: false,
            rng: ChaCha8Rng::seed_from_u64(PLACEHOLDER_SEED),
        }
    }

    pub fn with_key_delay(mut self, key_delay: Duration) -> Self {
        self.keys = KeyPacer::new(key_delay);
        self
    }

    /// forget whatever was queued; the board it was meant for has gone
    pub fn reset(&mut self) {
        self.keys.abandon();
        self.decided = false;
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

        if !self.decided {
            self.decide(pair.pivot().x);
            self.decided = true;
        }

        // a speed limited agent gets one key here and then has to wait; at full speed the
        // whole sequence goes in this frame
        while let Some(key) = self.keys.next_key() {
            match key {
                Key::Left => engine::game::Game::left(game),
                Key::Right => engine::game::Game::right(game),
                Key::RotateClockwise => engine::game::Game::rotate(game, true),
                Key::HardDrop => engine::game::Game::hard_drop(game),
            }
        }
    }

    fn decide(&mut self, from_column: i32) {
        match self.brain {
            PuyoAiKind::Placeholder => {
                let column = self.rng.random_range(0..COLUMNS as i32);
                let turns = self.rng.random_range(0..4);
                let mut keys = vec![Key::RotateClockwise; turns];
                let step = if column < from_column {
                    Key::Left
                } else {
                    Key::Right
                };
                keys.extend(std::iter::repeat_n(
                    step,
                    (column - from_column).unsigned_abs() as usize,
                ));
                keys.push(Key::HardDrop);
                self.keys.queue(keys);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::random::GameRandom;
    use crate::game::rules::Difficulty;
    use engine::game::random::Seed;
    use engine::game::{Game as _, StageState};

    fn game() -> Game {
        Game::new(
            Difficulty::Normal,
            0,
            GameRandom::from_seed(Seed::from_u64(7), Difficulty::Normal.colors()),
            crate::game::cell::PuyoSkin::FIRST,
        )
    }

    /// the placeholder is not meant to play well, but it is meant to *play*: it has to keep
    /// locking pairs rather than sitting on its hands, or the menu offers an opponent that
    /// does nothing at all
    #[test]
    fn the_placeholder_keeps_placing_pairs() {
        let mut game = game();
        let mut agent = PuyoAiAgent::default();
        let mut locked = 0;
        for _ in 0..20_000 {
            if matches!(game.stage_state(), StageState::GameOver) {
                break;
            }
            agent.act(&mut game, Duration::from_millis(4));
            game.update(Duration::from_millis(4));
            locked += game
                .drain_events()
                .iter()
                .filter(|e| matches!(e, engine::game::GameEvent::HardDrop { .. }))
                .count();
        }
        assert!(locked > 10, "only {locked} pairs placed");
    }

    /// two runs of a demo look the same, which is what a fixed seed is for
    #[test]
    fn the_placeholder_plays_the_same_game_twice() {
        let play = || {
            let mut game = game();
            let mut agent = PuyoAiAgent::default();
            for _ in 0..2_000 {
                agent.act(&mut game, Duration::from_millis(4));
                game.update(Duration::from_millis(4));
                game.drain_events();
            }
            game.score()
        };
        assert_eq!(play(), play());
    }
}
