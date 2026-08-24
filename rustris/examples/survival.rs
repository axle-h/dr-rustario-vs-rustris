//! Plays a Rustris neural network model headless on a fixed seed for as long as it can, printing
//! progress as it goes.
//!
//! `cargo run --release -p rustris --example survival -- <seed> [line cap] [report every n lines] [survival|tetris]`
//!
//! Mirrors the private `HeadlessGame` in `game::ai::headless_game`: a 60 Hz step with the agent
//! pressing keys as fast as it likes, and a simulated 750 ms line clear animation.
//!
//! The game caps its own counters (`MAX_LINES` = 9,999 and `MAX_SCORE` = 999,999,999) so lines are
//! counted here from the clear events, and the score is banked and reset whenever it gets close to
//! the cap. Neither counter feeds back into the game rules (level comes from `stage_lines`).

use engine::game::{Game as _, GameEvent};
use rustris::game::ai::action_evaluator::ActionEvaluator;
use rustris::game::ai::agent::AiAgent;
use rustris::game::ai::neural::TetrisNeuralNetwork;
use rustris::game::random::{RandomMode, RandomTetromino, Seed, MIN_GARBAGE_PER_HOLE};
use rustris::game::Game;
use std::time::{Duration, Instant};

const STEP: Duration = Duration::from_millis(16);
const LINE_CLEAR_DURATION: Duration = Duration::from_millis(750);
const BANK_SCORE_AT: u32 = 500_000_000;
/// the line counts from the last post, worth a row of their own
const MILESTONES: [u64; 2] = [10_840, 20_000];

fn fmt_duration(d: Duration) -> String {
    let s = d.as_secs();
    format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}

struct Stats {
    lines: u64,
    banked_score: u64,
    pieces: u64,
    clears: [u64; 5],
    sim: Duration,
    started: Instant,
}

impl Stats {
    fn score(&self, game: &Game) -> u64 {
        self.banked_score + game.score() as u64
    }

    fn report(&self, tag: &str, game: &Game) {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            tag, self.lines, self.score(game), game.level(), self.pieces,
            self.clears[4] * 4, self.clears[1], self.clears[2], self.clears[3], self.clears[4],
            fmt_duration(self.sim), fmt_duration(self.started.elapsed())
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seed: Seed = args.first().expect("seed").clone().into();
    let line_cap: u64 = args.get(1).and_then(|s| s.parse().ok()).filter(|&c| c > 0).unwrap_or(u64::MAX);
    let every: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
    let model = match args.get(3).map(String::as_str) {
        Some("tetris") => TetrisNeuralNetwork::tetris_clear_trained(),
        _ => TetrisNeuralNetwork::survival_trained(),
    };

    let rng = RandomTetromino::new(RandomMode::Bag, MIN_GARBAGE_PER_HOLE, seed);
    let mut agent = AiAgent::new(ActionEvaluator::NeuralNetwork(model), 0);
    let mut game = Game::new(0, rng);

    let mut stats = Stats {
        lines: 0,
        banked_score: 0,
        pieces: 0,
        clears: [0; 5],
        sim: Duration::ZERO,
        started: Instant::now(),
    };
    let mut next_report = every;
    let mut milestones = MILESTONES.iter().copied().peekable();
    let mut game_over = false;

    println!("tag\tlines\tscore\tlevel\tpieces\ttetris_lines\tsingles\tdoubles\ttriples\ttetrises\tsim_time\twall_time");
    loop {
        stats.sim += STEP;
        if stats.lines >= line_cap {
            break;
        }
        agent.act(&mut game, STEP);
        let mut events = game.drain_events();
        game.update(STEP);
        events.extend(game.drain_events());
        for event in events {
            match event {
                GameEvent::GameOver => game_over = true,
                GameEvent::Clear { count, .. } => {
                    stats.sim += LINE_CLEAR_DURATION;
                    stats.lines += count as u64;
                    stats.clears[(count as usize).min(4)] += 1;
                }
                GameEvent::Spawn { .. } => stats.pieces += 1,
                _ => {}
            }
        }
        if game_over {
            break;
        }
        if game.score() >= BANK_SCORE_AT {
            stats.banked_score += game.score() as u64;
            game.set_score(0);
        }
        if let Some(&milestone) = milestones.peek() {
            if stats.lines >= milestone {
                stats.report("milestone", &game);
                milestones.next();
            }
        }
        if stats.lines >= next_report {
            stats.report("progress", &game);
            next_report = (stats.lines / every + 1) * every;
        }
    }
    stats.report(if game_over { "game_over" } else { "end" }, &game);
}
