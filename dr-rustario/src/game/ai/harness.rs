//! `ga dr play`: run the built in model headless on a fixed seed and report as it goes.

use crate::game::ai::agent::DrAiAgent;
use crate::game::ai::models;
use crate::game::random::{GameRandom, RandomMode};
use crate::game::{Game, GameSpeed};
use engine::ai::Seed;
use engine::game::{Game as _, GameEvent, MetricKind};
use std::time::{Duration, Instant};

const STEP: Duration = Duration::from_millis(16);
const CLEAR_DURATION: Duration = Duration::from_millis(400);

struct Stats {
    viruses: u64,
    pills: u64,
    stages: u64,
    sim: Duration,
    started: Instant,
}

impl Stats {
    fn report(&self, tag: &str, game: &Game, level: u32) {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{:.3}\t{}\t{}",
            tag,
            self.viruses,
            self.pills,
            self.stages,
            level,
            self.viruses as f64 / self.pills.max(1) as f64,
            fmt_duration(self.sim),
            fmt_duration(self.started.elapsed())
        );
        let _ = game;
    }
}

fn fmt_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    format!(
        "{}:{:02}:{:02}",
        seconds / 3600,
        (seconds / 60) % 60,
        seconds % 60
    )
}

/// `args` are the arguments after `ga dr play`:
/// `<seed> [virus level] [pill cap] [report every n pills] [brain]`, where the brain is `n64`
/// (the default), `n64:0` to `n64:5` for one of the N64 ai's own rows of weights, `neural` for
/// the trained network or `linear` for the hand written baseline it is measured against
pub fn harness_main(args: &[String]) -> Result<(), String> {
    let seed: Seed = args
        .first()
        .ok_or("usage: ga dr play <seed> [virus level] [pill cap] [report every n pills]")?
        .clone()
        .into();
    let level: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
    let pill_cap: u64 = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .filter(|&c| c > 0)
        .unwrap_or(u64::MAX);
    let every: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(100);
    let brain = args.get(4).map(String::as_str).unwrap_or("n64");

    let random = GameRandom::from_seed(seed.into(), RandomMode::Bag);
    let mut game = Game::new(level, GameSpeed::Medium, random)?;
    let mut agent = match brain {
        "linear" => DrAiAgent::linear(),
        "neural" => DrAiAgent::new(models::virus_clear_trained()),
        "n64" => DrAiAgent::n64(),
        other => match other
            .strip_prefix("n64:")
            .and_then(|s| s.parse::<u8>().ok())
        {
            Some(skill) => DrAiAgent::n64_with_skill(skill),
            None => return Err(format!("unknown brain: {}", other)),
        },
    };

    let mut stats = Stats {
        viruses: 0,
        pills: 0,
        stages: 0,
        sim: Duration::ZERO,
        started: Instant::now(),
    };
    let mut next_report = every;
    let mut game_over = false;

    println!("tag\tviruses\tpills\tstages\tlevel\tviruses_per_pill\tsim_time\twall_time");
    loop {
        stats.sim += STEP;
        if stats.pills >= pill_cap {
            break;
        }

        let viruses_before = game.bottle().virus_count();
        agent.act(&mut game, STEP);
        let mut events = game.drain_events();
        game.update(STEP);
        events.extend(game.drain_events());
        stats.viruses += viruses_before.saturating_sub(game.bottle().virus_count()) as u64;

        for event in events {
            match event {
                GameEvent::GameOver => game_over = true,
                GameEvent::Clear { .. } => stats.sim += CLEAR_DURATION,
                GameEvent::Spawn { .. } => stats.pills += 1,
                GameEvent::StageComplete => {
                    stats.stages += 1;
                    stats.report("stage", &game, game.metric(MetricKind::Level).unwrap_or(0));
                    if game.next_stage().is_err() {
                        game_over = true;
                    }
                    agent.reset();
                }
                _ => {}
            }
        }
        if game_over {
            break;
        }
        if stats.pills >= next_report {
            stats.report(
                "progress",
                &game,
                game.metric(MetricKind::Level).unwrap_or(0),
            );
            next_report = (stats.pills / every + 1) * every;
        }
    }
    stats.report(
        if game_over { "game_over" } else { "end" },
        &game,
        game.metric(MetricKind::Level).unwrap_or(0),
    );
    Ok(())
}
