//! `ga puyo play`, `ga puyo rank` and `ga puyo duel`: run a brain headless and see what it can
//! do.
//!
//! `play` is the diagnostic - one seed, one brain, reporting as it goes. `rank` is what
//! [`SKILL_ORDER`](crate::game::ai::SKILL_ORDER) is *made of*: every row plays the same seeds
//! from the same start, and the order that comes out is pasted back into
//! [`crate::game::ai::skill`]. Nothing about the ladder is asserted anywhere; it is measured
//! here and then written down.
//!
//! **`rank` is a solo marathon, and a solo marathon takes no nuisance.** Nothing ever lands on
//! the board, so it measures what a row *builds* and can say nothing whatever about what a row
//! does with what is thrown at it - which is half of what a row is for. That is `duel`: two
//! rows on the same seed, each sending the other what its chains buy, routed the way the match
//! screen routes it. It is what
//! [`SearchConfig::answer_at`](crate::game::ai::beam::SearchConfig::answer_at) was set from,
//! and it is also the Puyo end of the protocol the cross-game attack prices are measured on.

use crate::game::ai::agent::PuyoAiAgent;
use crate::game::ai::beam::SearchConfig;
use crate::game::ai::{skill, PuyoAiKind, SKILLS};
use crate::game::random::GameRandom;
use crate::game::rules::Difficulty;
use crate::game::Game;
use engine::game::random::Seed;
use engine::game::{Attack, Game as _, GameEvent, StageState};
use std::time::{Duration, Instant};

/// the frame the headless game is stepped at; the agent is speed limited by its own key
/// pacer and not by this
const STEP: Duration = Duration::from_millis(8);

/// a headless game cannot run forever if a brain refuses to lock anything, so every run has a
/// ceiling in frames as well as in pairs
const MAX_FRAMES: u64 = 40_000_000;

#[derive(Default)]
struct Run {
    score: u32,
    pairs: u64,
    chains: u64,
    best_chain: u32,
    nuisance_sent: u64,
    /// what an opponent put in this board's tray. Always zero in a solo run
    nuisance_received: u64,
    /// pairs this board committed to a chain that cancels its tray - see
    /// [`PuyoAiAgent::answers`]
    answers: u32,
    /// pairs it decided with anything at all waiting - see [`PuyoAiAgent::trays`]
    trays: u32,
    /// pairs it fired on because the tray was about to bury it - see [`PuyoAiAgent::crowded`]
    crowded: u32,
    buried: bool,
}

/// A brain to play a board with: one of the rows, and optionally one of its dials moved.
///
/// The override is spelled `sharp@12` on the command line and means *that row, answering a
/// tray twelve puyos deep*. It is a measurement seam and nothing else - a difficulty is
/// always a whole row - and it is what a sweep of
/// [`answer_at`](crate::game::ai::beam::SearchConfig::answer_at) is made of, since the
/// alternative is editing a const and rebuilding for every point of it.
#[derive(Clone, Copy)]
struct Brain {
    kind: PuyoAiKind,
    answer_at: Option<u32>,
}

impl Brain {
    fn name(&self) -> String {
        let row = self
            .kind
            .skill()
            .map(|skill| skill.name)
            .unwrap_or("placeholder");
        match self.answer_at {
            None => row.to_string(),
            Some(u32::MAX) => format!("{row}@never"),
            Some(at) => format!("{row}@{at}"),
        }
    }

    fn agent(&self, key_delay: Duration) -> PuyoAiAgent {
        let agent = PuyoAiAgent::of(self.kind).with_key_delay(key_delay);
        match (self.answer_at, self.kind.skill()) {
            (Some(answer_at), Some(skill)) => agent.with_search(SearchConfig {
                answer_at,
                ..skill.search
            }),
            _ => agent,
        }
    }
}

fn brain_of(name: &str) -> Result<Brain, String> {
    let (name, answer_at) = match name.split_once('@') {
        None => (name, None),
        Some((row, "never")) => (row, Some(u32::MAX)),
        Some((row, at)) => (
            row,
            Some(
                at.parse::<u32>()
                    .map_err(|_| format!("'{at}' is not a tray depth"))?,
            ),
        ),
    };
    let kind = kind_of(name)?;
    Ok(Brain { kind, answer_at })
}

fn kind_of(name: &str) -> Result<PuyoAiKind, String> {
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
        "unknown brain '{name}', expected placeholder, row:0..{}, or one of: {},          any of them with an '@<tray depth>' answering override",
        SKILLS - 1,
        names.join(", ")
    ))
}

/// One side of a match: its board, the brain playing it, and what it has managed so far.
///
/// A solo run is one of these stepped on its own; a duel is two, stepped and only then
/// delivered to each other. Both go through the same frame, so the only difference between
/// what `rank` measures and what `duel` measures is that in a duel something arrives.
struct Player {
    game: Game,
    agent: PuyoAiAgent,
    run: Run,
}

impl Player {
    fn new(
        brain: Brain,
        seed: u64,
        difficulty: Difficulty,
        level: u32,
        key_delay: Duration,
    ) -> Self {
        let random = GameRandom::from_seed(Seed::from_u64(seed), difficulty.colors());
        Self {
            game: Game::new(
                difficulty,
                level,
                random,
                crate::game::cell::PuyoSkin::FIRST,
            ),
            agent: brain.agent(key_delay),
            run: Run::default(),
        }
    }

    /// Play one frame, and report what this board fired off in it.
    ///
    /// What comes back is the strength to hand the *opponent*, which is why it is returned
    /// rather than delivered: the match screen collects a frame's events from every player
    /// before it routes any of them, so two chains that finish together cross rather than one
    /// of them cancelling the other.
    fn frame(&mut self, report: &mut impl FnMut(&Run)) -> u32 {
        let mut fired = 0;
        self.agent.act(&mut self.game, STEP);
        let mut events = self.game.drain_events();
        self.game.update(STEP);
        events.extend(self.game.drain_events());
        for event in events {
            match event {
                GameEvent::Lock { .. } => {
                    self.run.pairs += 1;
                    report(&self.run);
                }
                GameEvent::Clear { detail, .. } => {
                    let detail = crate::game::ClearDetail::from(detail);
                    if detail.chain == 1 {
                        self.run.chains += 1;
                    }
                    self.run.best_chain = self.run.best_chain.max(detail.chain);
                }
                GameEvent::AttackSent(attack) => {
                    let strength = attack.strength_for(crate::game::GAME_ID);
                    self.run.nuisance_sent += strength as u64;
                    fired += strength;
                }
                GameEvent::GameOver => self.run.buried = true,
                _ => {}
            }
        }
        if matches!(self.game.stage_state(), StageState::GameOver) {
            self.run.buried = true;
        }
        self.run.score = self.game.score();
        self.run.answers = self.agent.answers();
        self.run.trays = self.agent.trays();
        self.run.crowded = self.agent.crowded();
        fired
    }

    fn take(&mut self, strength: u32) {
        if strength > 0 {
            self.run.nuisance_received += strength as u64;
            self.game
                .receive_attack(Attack::new(crate::game::GAME_ID, strength));
        }
    }
}

fn play(
    brain: Brain,
    seed: u64,
    difficulty: Difficulty,
    level: u32,
    pair_cap: u64,
    mut report: impl FnMut(&Run),
) -> Run {
    let mut player = Player::new(brain, seed, difficulty, level, Duration::ZERO);
    for _ in 0..MAX_FRAMES {
        if player.run.pairs >= pair_cap || player.run.buried {
            break;
        }
        player.frame(&mut report);
    }
    player.run
}

/// Two rows, one seed, each sending the other what its chains buy.
///
/// Both boards are dealt the same pairs, which is what makes it a paired comparison: the
/// difference between the two sides is the two brains and nothing else. It runs until one of
/// them is buried or both have placed `pair_cap` pairs, and what comes back is a run apiece.
fn duel(
    brains: [Brain; 2],
    seed: u64,
    difficulty: Difficulty,
    pair_cap: u64,
    key_delay: Duration,
) -> [Run; 2] {
    let mut players = [
        Player::new(brains[0], seed, difficulty, 0, key_delay),
        Player::new(brains[1], seed, difficulty, 0, key_delay),
    ];
    let mut quiet = |_: &Run| {};

    for _ in 0..MAX_FRAMES {
        if players.iter().any(|p| p.run.buried) || players.iter().all(|p| p.run.pairs >= pair_cap) {
            break;
        }
        // every board steps, and only then is anything delivered
        let fired = [players[0].frame(&mut quiet), players[1].frame(&mut quiet)];
        players[0].take(fired[1]);
        players[1].take(fired[0]);
    }
    let [a, b] = players;
    [a.run, b.run]
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
        "seed\t{seed}\tdifficulty\t{}\tbrain\t{}",
        difficulty.name(),
        brain.name()
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
                Brain {
                    kind: PuyoAiKind::Scorer(row),
                    answer_at: None,
                },
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

/// who won a duel, or `None` if it was still going when the pairs ran out
fn winner(runs: &[Run; 2]) -> Option<usize> {
    match (runs[0].buried, runs[1].buried) {
        (false, true) => Some(0),
        (true, false) => Some(1),
        _ => None,
    }
}

/// one line of a duel table
fn print_side(tag: &str, name: &str, run: &Run) {
    println!(
        "{tag}\t{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        run.score,
        run.pairs,
        run.chains,
        run.best_chain,
        run.nuisance_sent,
        run.nuisance_received,
        run.trays,
        run.answers,
        run.crowded,
    );
}

const DUEL_COLUMNS: &str =
    "tag\tname\tscore\tpairs\tchains\tbest_chain\tsent\treceived\ttrays\tanswers\tcrowded";

/// `args` are the arguments after `ga puyo duel`:
/// `[seeds] [pair cap] [difficulty] [key delay ms] [a] [b]`.
///
/// With two brains named it is a head to head, one line a seed. With neither, it is every row
/// against every other - which is the ladder measured under fire rather than in a marathon,
/// and the table to read when [`answer_at`](crate::game::ai::beam::SearchConfig::answer_at) or
/// a [`trigger`](crate::game::ai::beam::SearchConfig::trigger) has been touched.
///
/// **A duel ends when someone is buried**, so the pair cap is a ceiling rather than a length:
/// a pairing that reaches it was a stalemate and counts as one.
pub fn duel_main(args: &[String]) -> Result<(), String> {
    let seeds: u64 = args.first().and_then(|s| s.parse().ok()).unwrap_or(6);
    let pair_cap: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(400);
    let difficulty = match args.get(2) {
        None => Difficulty::default(),
        Some(name) => {
            Difficulty::from_name(name).ok_or_else(|| format!("unknown difficulty '{name}'"))?
        }
    };
    let key_delay = Duration::from_millis(args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0));
    let named: Vec<Brain> = args[4.min(args.len())..]
        .iter()
        .map(|name| brain_of(name))
        .collect::<Result<_, _>>()?;

    match named.len() {
        0 => round_robin(seeds, pair_cap, difficulty, key_delay),
        2 => head_to_head([named[0], named[1]], seeds, pair_cap, difficulty, key_delay),
        _ => {
            return Err(
                "usage: ga puyo duel [seeds] [pair cap] [difficulty] [key delay ms] \
                 [brain a] [brain b] - name both brains or neither"
                    .to_string(),
            )
        }
    }
    Ok(())
}

/// two named brains over every seed, one line each way and a total
fn head_to_head(
    brains: [Brain; 2],
    seeds: u64,
    pair_cap: u64,
    difficulty: Difficulty,
    key_delay: Duration,
) {
    let names = [brains[0].name(), brains[1].name()];
    println!(
        "{} against {} over {seeds} seeds, at most {pair_cap} pairs, on {}, {:?} a key",
        names[0],
        names[1],
        difficulty.name(),
        key_delay
    );
    println!("{DUEL_COLUMNS}");

    let started = Instant::now();
    let mut wins = [0u32; 2];
    let mut draws = 0;
    let mut totals = [Run::default(), Run::default()];
    for seed in 0..seeds {
        let runs = duel(brains, seed, difficulty, pair_cap, key_delay);
        match winner(&runs) {
            Some(side) => wins[side] += 1,
            None => draws += 1,
        }
        for side in 0..2 {
            print_side(&format!("seed {seed}"), &names[side], &runs[side]);
            let total = &mut totals[side];
            total.score += runs[side].score;
            total.pairs += runs[side].pairs;
            total.chains += runs[side].chains;
            total.best_chain = total.best_chain.max(runs[side].best_chain);
            total.nuisance_sent += runs[side].nuisance_sent;
            total.nuisance_received += runs[side].nuisance_received;
            total.answers += runs[side].answers;
            total.trays += runs[side].trays;
            total.crowded += runs[side].crowded;
        }
    }
    println!();
    for side in 0..2 {
        print_side("total", &names[side], &totals[side]);
    }
    println!(
        "{} {} - {} {}, {draws} unfinished, in {:?}",
        names[0],
        wins[0],
        wins[1],
        names[1],
        started.elapsed()
    );
}

/// every row against every other, and the order that comes out of it
fn round_robin(seeds: u64, pair_cap: u64, difficulty: Difficulty, key_delay: Duration) {
    println!(
        "every row against every other over {seeds} seeds, at most {pair_cap} pairs, on {}, \
         {key_delay:?} a key",
        difficulty.name()
    );
    println!("pairing\twins\tlosses\tunfinished\tsent\treceived\tanswers");

    let started = Instant::now();
    let mut wins = [0u32; SKILLS];
    let mut losses = [0u32; SKILLS];
    let mut sent = [0u64; SKILLS];
    let mut answers = [0u32; SKILLS];

    for a in 0..SKILLS {
        for b in (a + 1)..SKILLS {
            let brains = [
                Brain {
                    kind: PuyoAiKind::Scorer(a),
                    answer_at: None,
                },
                Brain {
                    kind: PuyoAiKind::Scorer(b),
                    answer_at: None,
                },
            ];
            let (mut a_wins, mut b_wins, mut unfinished) = (0u32, 0u32, 0u32);
            let (mut a_sent, mut b_sent, mut a_answers, mut b_answers) = (0u64, 0u64, 0u32, 0u32);
            for seed in 0..seeds {
                let runs = duel(brains, seed, difficulty, pair_cap, key_delay);
                match winner(&runs) {
                    Some(0) => a_wins += 1,
                    Some(_) => b_wins += 1,
                    None => unfinished += 1,
                }
                a_sent += runs[0].nuisance_sent;
                b_sent += runs[1].nuisance_sent;
                a_answers += runs[0].answers;
                b_answers += runs[1].answers;
            }
            wins[a] += a_wins;
            wins[b] += b_wins;
            losses[a] += b_wins;
            losses[b] += a_wins;
            sent[a] += a_sent;
            sent[b] += b_sent;
            answers[a] += a_answers;
            answers[b] += b_answers;
            println!(
                "{} v {}\t{a_wins}-{b_wins}\t{unfinished}\t{a_sent}/{b_sent}\t\
                 {a_answers}/{b_answers}",
                skill::ROWS[a].name,
                skill::ROWS[b].name
            );
        }
    }

    println!();
    println!("row\tname\twins\tlosses\tsent\tanswers");
    for row in 0..SKILLS {
        println!(
            "{row}\t{}\t{}\t{}\t{}\t{}",
            skill::ROWS[row].name,
            wins[row],
            losses[row],
            sent[row],
            answers[row]
        );
    }

    // worst first, the way SKILL_ORDER reads. Wins decide it and what a row sent breaks the
    // ties, which are common between rows that never bury each other
    let mut order: Vec<usize> = (0..SKILLS).collect();
    order.sort_by_key(|row| (wins[*row], sent[*row]));
    let order: Vec<String> = order.iter().map(|row| row.to_string()).collect();
    println!();
    println!(
        "// the ladder as measured under fire, worst first, in {:?}",
        started.elapsed()
    );
    println!("// SKILL_ORDER is rank's to set - read this against it rather than pasting over it");
    println!("// [{}]", order.join(", "));
}
