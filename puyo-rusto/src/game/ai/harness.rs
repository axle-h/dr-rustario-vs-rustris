//! `ga puyo play` and `ga puyo rank`: run a brain headless and see what it can do.
//!
//! `play` is the diagnostic - one seed, one brain, reporting as it goes. `rank` is what
//! [`SKILL_ORDER`](crate::game::ai::SKILL_ORDER) is *made of*: every row plays the same seeds
//! from the same start, and the order that comes out is pasted back into
//! [`crate::game::ai::skill`]. Nothing about the ladder is asserted anywhere; it is measured
//! here and then written down.

use crate::game::ai::agent::PuyoAiAgent;
use crate::game::ai::{skill, PuyoAiKind, SKILLS};
use crate::game::random::GameRandom;
use crate::game::rules::Difficulty;
use crate::game::Game;
use engine::game::random::Seed;
use engine::game::{Game as _, GameEvent, StageState};
use std::time::{Duration, Instant};

/// the frame the headless game is stepped at; the agent is speed limited by its own key
/// pacer and not by this
const STEP: Duration = Duration::from_millis(8);

/// a headless game cannot run forever if a brain refuses to lock anything, so every run has a
/// ceiling in frames as well as in pairs
const MAX_FRAMES: u64 = 40_000_000;

struct Run {
    score: u32,
    pairs: u64,
    chains: u64,
    best_chain: u32,
    nuisance_sent: u64,
    buried: bool,
}

fn brain_of(name: &str) -> Result<PuyoAiKind, String> {
    if name == "placeholder" {
        return Ok(PuyoAiKind::Placeholder);
    }
    if let Some(row) = skill::by_name(name) {
        return Ok(PuyoAiKind::Scorer(row));
    }
    if let Some(row) = name
        .strip_prefix("row:")
        .and_then(|s| s.parse::<usize>().ok())
    {
        if row < SKILLS {
            return Ok(PuyoAiKind::Scorer(row));
        }
    }
    let names: Vec<&str> = skill::ROWS.iter().map(|row| row.name).collect();
    Err(format!(
        "unknown brain '{name}', expected placeholder, row:0..{}, or one of: {}",
        SKILLS - 1,
        names.join(", ")
    ))
}

fn play(
    brain: PuyoAiKind,
    seed: u64,
    difficulty: Difficulty,
    level: u32,
    pair_cap: u64,
    mut report: impl FnMut(&Run),
) -> Run {
    let random = GameRandom::from_seed(Seed::from_u64(seed), difficulty.colors());
    let mut game = Game::new(
        difficulty,
        level,
        random,
        crate::game::cell::PuyoSkin::FIRST,
    );
    let mut agent = PuyoAiAgent::of(brain);
    let mut run = Run {
        score: 0,
        pairs: 0,
        chains: 0,
        best_chain: 0,
        nuisance_sent: 0,
        buried: false,
    };

    for _ in 0..MAX_FRAMES {
        if run.pairs >= pair_cap || run.buried {
            break;
        }
        agent.act(&mut game, STEP);
        let mut events = game.drain_events();
        game.update(STEP);
        events.extend(game.drain_events());
        for event in events {
            match event {
                GameEvent::Lock { .. } => {
                    run.pairs += 1;
                    report(&run);
                }
                GameEvent::Clear { detail, .. } => {
                    let detail = crate::game::ClearDetail::from(detail);
                    if detail.chain == 1 {
                        run.chains += 1;
                    }
                    run.best_chain = run.best_chain.max(detail.chain);
                }
                GameEvent::AttackSent(attack) => {
                    run.nuisance_sent += attack.strength_for(crate::game::GAME_ID) as u64;
                }
                GameEvent::GameOver => run.buried = true,
                _ => {}
            }
        }
        if matches!(game.stage_state(), StageState::GameOver) {
            run.buried = true;
        }
        run.score = game.score();
    }
    run.score = game.score();
    run
}

/// `args` are the arguments after `ga puyo play`:
/// `<seed> [difficulty] [pair cap] [report every n pairs] [brain]`
pub fn harness_main(args: &[String]) -> Result<(), String> {
    let seed: u64 = args
        .first()
        .ok_or("usage: ga puyo play <seed> [difficulty] [pair cap] [report every n pairs] [brain]")?
        .parse()
        .map_err(|_| "the seed is a number".to_string())?;
    let difficulty = match args.get(1) {
        None => Difficulty::default(),
        Some(name) => {
            Difficulty::from_name(name).ok_or_else(|| format!("unknown difficulty '{name}'"))?
        }
    };
    let pair_cap: u64 = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .filter(|c| *c > 0)
        .unwrap_or(u64::MAX);
    let every: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(100);
    let brain = brain_of(args.get(4).map(String::as_str).unwrap_or("sharp"))?;

    println!(
        "seed\t{seed}\tdifficulty\t{}\tbrain\t{brain:?}",
        difficulty.name()
    );
    println!("tag\tpairs\tscore\tchains\tbest_chain\tnuisance_sent\twall_time");
    let started = Instant::now();
    let mut next = every;
    let run = play(brain, seed, difficulty, 0, pair_cap, |run| {
        if run.pairs >= next {
            next += every;
            println!(
                "at\t{}\t{}\t{}\t{}\t{}\t{:?}",
                run.pairs,
                run.score,
                run.chains,
                run.best_chain,
                run.nuisance_sent,
                started.elapsed()
            );
        }
    });
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{:?}",
        if run.buried { "buried" } else { "capped" },
        run.pairs,
        run.score,
        run.chains,
        run.best_chain,
        run.nuisance_sent,
        started.elapsed()
    );
    Ok(())
}

/// `args` are the arguments after `ga puyo rank`: `[seeds] [pair cap] [difficulty]`.
///
/// Every row plays every seed. What it prints is the table to paste into
/// [`crate::game::ai::skill::SKILL_ORDER`], worst first.
pub fn rank_main(args: &[String]) -> Result<(), String> {
    let seeds: u64 = args.first().and_then(|s| s.parse().ok()).unwrap_or(8);
    let pair_cap: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(400);
    let difficulty = match args.get(2) {
        None => Difficulty::default(),
        Some(name) => {
            Difficulty::from_name(name).ok_or_else(|| format!("unknown difficulty '{name}'"))?
        }
    };

    println!(
        "ranking {SKILLS} rows over {seeds} seeds, {pair_cap} pairs each, on {}",
        difficulty.name()
    );
    println!(
        "row\tname\tscore\tscore_per_pair\tpairs\tburied\tbest_chain\tnuisance_sent\twall_time"
    );

    let mut totals: Vec<(usize, u64)> = vec![];
    for row in 0..SKILLS {
        let started = Instant::now();
        let (mut score, mut pairs, mut buried, mut best, mut sent) = (0u64, 0u64, 0u64, 0u32, 0u64);
        for seed in 0..seeds {
            let run = play(
                PuyoAiKind::Scorer(row),
                seed,
                difficulty,
                0,
                pair_cap,
                |_| {},
            );
            score += run.score as u64;
            pairs += run.pairs;
            buried += u64::from(run.buried);
            best = best.max(run.best_chain);
            sent += run.nuisance_sent;
        }
        // the search is stepped once a frame, so what a frame costs on this machine is the
        // whole think divided by the steps a search takes - which is the number to look at on
        // a handheld, not the one per pair
        let steps = skill::ROWS[row].search.steps();
        let per_pair = started.elapsed().as_secs_f64() * 1000.0 / pairs.max(1) as f64;
        println!(
            "{row}\t{}\t{score}\t{:.1}\t{pairs}\t{buried}\t{best}\t{sent}\t{steps}\t\
             {per_pair:.2}\t{:.2}",
            skill::ROWS[row].name,
            score as f64 / pairs.max(1) as f64,
            per_pair / steps as f64
        );
        totals.push((row, score));
    }

    totals.sort_by_key(|(_, score)| *score);
    let order: Vec<String> = totals.iter().map(|(row, _)| row.to_string()).collect();
    println!();
    println!("// the measured ranking: paste over SKILL_ORDER in game/ai/skill.rs");
    println!(
        "pub const SKILL_ORDER: [usize; SKILLS] = [{}];",
        order.join(", ")
    );
    Ok(())
}
