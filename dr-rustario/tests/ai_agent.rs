//! The agent driving a real game, which the crate's own test build replaces with a mock.

use dr_rustario::game::ai::agent::{DrAiAgent, Hold};
use dr_rustario::game::ai::models;
use dr_rustario::game::random::{GameRandom, RandomMode};
use dr_rustario::game::{Game, GameSpeed};
use engine::game::geometry::Point;
use engine::game::random::Seed;
use engine::game::{Cell, Game as _, GameEvent, MetricKind, PlacedCell};
use std::collections::HashSet;
use std::time::Duration;

const STEP: Duration = Duration::from_millis(16);

fn viruses(game: &Game) -> u32 {
    game.metric(MetricKind::Viruses).unwrap_or(0)
}

/// a frame budget so a stuck agent fails the test instead of hanging it
const MAX_STEPS: u32 = 400_000;

struct Played {
    steps: u32,
    stages: u32,
    pills: u32,
    columns: HashSet<i32>,
    viruses_cleared: u32,
    game_over: bool,
    /// swaps the agent made
    holds: u32,
    /// halves that locked under something other than their partner: tucks
    tucked: u32,
}

/// Play with the hand weighted scorer, so these tests do not move when the embedded model does.
fn play(level: u32, key_delay: Duration, max_pills: u32) -> Played {
    play_with(DrAiAgent::linear(), level, key_delay, max_pills)
}

fn play_with(agent: DrAiAgent, level: u32, key_delay: Duration, max_pills: u32) -> Played {
    play_at(agent, level, GameSpeed::Medium, key_delay, max_pills)
}

fn play_at(
    agent: DrAiAgent,
    level: u32,
    speed: GameSpeed,
    key_delay: Duration,
    max_pills: u32,
) -> Played {
    let mut agent = agent.with_key_delay(key_delay);
    let random = GameRandom::from_seed(Seed::from_u64(1), RandomMode::Bag);
    let mut game = Game::new(level, speed, random).expect("could not deal a bottle");

    let mut played = Played {
        steps: 0,
        stages: 0,
        pills: 0,
        columns: HashSet::new(),
        viruses_cleared: 0,
        game_over: false,
        holds: 0,
        tucked: 0,
    };

    while played.pills < max_pills && !played.game_over && played.steps < MAX_STEPS {
        played.steps += 1;
        let viruses_before = viruses(&game);
        agent.act(&mut game, STEP);
        let mut events = game.drain_events();
        game.update(STEP);
        events.extend(game.drain_events());
        played.viruses_cleared += viruses_before.saturating_sub(viruses(&game));

        for event in events {
            match event {
                GameEvent::GameOver => played.game_over = true,
                GameEvent::Hold => played.holds += 1,
                GameEvent::Spawn { .. } => played.pills += 1,
                GameEvent::StageComplete => {
                    // the real game shows an interstitial here and waits to be dismissed
                    played.stages += 1;
                    game.next_stage().expect("could not deal the next bottle");
                    agent.reset();
                }
                GameEvent::Lock { ref cells, .. } => {
                    for cell in cells {
                        played.columns.insert(cell.0.x());
                    }
                    played.tucked += tucked_halves(&game, cells);
                }
                _ => (),
            }
        }
    }

    played
}

/// How many of the halves that just locked have a settled block over them other than their
/// partner; a [`Cell::Ghost`] or the pill in play is not cover.
fn tucked_halves(game: &Game, cells: &[PlacedCell]) -> u32 {
    let own: HashSet<(i32, i32)> = cells.iter().map(|(at, _)| (at.x(), at.y())).collect();
    cells
        .iter()
        .filter(|(at, _)| {
            (0..at.y())
                .filter(|y| !own.contains(&(at.x(), *y)))
                .any(|y| {
                    matches!(
                        game.cell(Point::new(at.x(), y)),
                        Cell::Stack(_) | Cell::Garbage(_)
                    )
                })
        })
        .count() as u32
}

#[test]
fn the_agent_tucks_under_an_overhang() {
    let played = play_with(DrAiAgent::n64(), 15, Duration::ZERO, 600);
    assert!(
        played.tucked > 0,
        "in {} pills nothing was ever walked in under an overhang, so the search offers tucks \
         the agent cannot execute",
        played.pills
    );
}

#[test]
fn a_speed_limited_agent_tucks_too() {
    // one key every 400 ms fits the one move a tuck needs inside the 500 ms lock delay
    let played = play_with(DrAiAgent::n64(), 15, Duration::from_millis(400), 600);
    assert!(
        played.tucked > 0,
        "the speed limited agent never got a half under an overhang in {} pills",
        played.pills
    );
}

#[test]
fn an_agent_offered_the_held_pill_still_plays_and_does_swap() {
    // checks the agent can press hold, wait for the swapped pill, and keep the plan for it
    let played = play_with(
        DrAiAgent::linear().with_hold(Hold::On),
        0,
        Duration::ZERO,
        200,
    );
    assert!(
        played.steps < MAX_STEPS,
        "the agent stalled with the held pill on offer"
    );
    assert!(played.pills > 1, "the agent never got a second pill");
    assert!(
        played.holds > 0,
        "hold was on offer for {} pills and never once taken",
        played.pills
    );
}

#[test]
fn the_agent_places_pills_across_the_bottle() {
    let played = play(0, Duration::ZERO, 30);
    assert!(
        played.steps < MAX_STEPS,
        "the agent stopped making progress"
    );
    assert!(played.pills > 1, "the agent never got a second pill");
    assert!(
        played.columns.len() > 2,
        "pills only ever landed in columns {:?}, so the agent is not steering them",
        played.columns
    );
}

#[test]
fn the_agent_clears_viruses() {
    let played = play(0, Duration::ZERO, 300);
    assert!(
        played.steps < MAX_STEPS,
        "the agent stopped making progress"
    );
    assert!(
        played.viruses_cleared > 0,
        "the agent cleared no viruses in 300 pills"
    );
}

#[test]
fn the_agent_moves_on_to_the_next_bottle() {
    let played = play(0, Duration::ZERO, 300);
    assert!(
        played.stages >= 1,
        "no bottle was cleared, so the run never reached a second one"
    );
}

#[test]
fn a_speed_limited_agent_still_plays() {
    let played = play(0, Duration::from_millis(400), 10);
    assert!(played.pills > 1, "the speed limited agent stalled");
}

/// The N64 port every difficulty and both demos play clears five bottles from level 10.
#[test]
fn the_deterministic_ai_clears_several_bottles() {
    let played = play_with(DrAiAgent::n64(), 10, Duration::ZERO, 3_000);
    assert!(
        played.stages >= 5,
        "the N64 ai cleared {} bottles at virus level 10 before it was buried",
        played.stages
    );
}

#[test]
#[ignore = "needs a model trained by ga dr auto"]
fn the_trained_model_clears_bottles() {
    let played = play_with(
        DrAiAgent::new(models::survival_trained()),
        0,
        Duration::ZERO,
        3_000,
    );
    assert!(
        played.stages >= 5,
        "the embedded model cleared {} bottles before it was buried",
        played.stages
    );
}

/// A tuck soft drops to its rest waypoint, from the slowest fall speed on a near empty bottle.
#[test]
fn a_tuck_soft_drops_rather_than_waiting_out_gravity() {
    let played = play_at(DrAiAgent::n64(), 5, GameSpeed::Low, Duration::ZERO, 300);
    assert!(
        played.tucked > 0,
        "nothing tucked, so this measures nothing"
    );
    let frames_per_pill = played.steps as f64 / played.pills as f64;
    assert!(
        frames_per_pill < 80.0,
        "{frames_per_pill:.1} frames a pill over {} pills ({} tucked): the agent is sitting \
         through the fall at its rest waypoints again",
        played.pills,
        played.tucked
    );
}

/// A 400 ms agent still tucks, so soft drop is released before the tuck's move.
#[test]
fn a_speed_limited_agent_still_tucks_after_the_soft_drop() {
    let played = play_at(
        DrAiAgent::n64(),
        5,
        GameSpeed::Low,
        Duration::from_millis(400),
        300,
    );
    assert!(
        played.tucked > 50,
        "only {} tucks in {} pills: the pill is locking before the agent can walk it in",
        played.tucked,
        played.pills
    );
}
