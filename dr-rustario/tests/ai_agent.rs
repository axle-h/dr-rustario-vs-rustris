//! The agent driving a real game. This has to be an integration test: the crate's own test build
//! swaps the bottle for a mock, so `Game` is not the real thing inside it.

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
    /// halves that locked with something above them that was not their own partner. Nothing
    /// falling into a bottle can land there, so every one of these is a tuck the agent walked
    /// in after the pill came to rest.
    tucked: u32,
}

/// These exercise the agent, not the model: they run the hand written scorer, which does not
/// change when a training run replaces the embedded weights. The model itself is checked by
/// `the_trained_model_clears_bottles`, which needs a trained one to pass.
fn play(level: u32, key_delay: Duration, max_pills: u32) -> Played {
    play_with(DrAiAgent::linear(), level, key_delay, max_pills)
}

fn play_with(agent: DrAiAgent, level: u32, key_delay: Duration, max_pills: u32) -> Played {
    let mut agent = agent.with_key_delay(key_delay);
    let random = GameRandom::from_seed(Seed::from_u64(1), RandomMode::Bag);
    let mut game = Game::new(level, GameSpeed::Medium, random).expect("could not deal a bottle");

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

/// How many of the halves that just locked have a settled block over them that is not the
/// other half of the same pill. A pill dropped into a bottle can only ever come to rest with
/// open space above it, so a covered half is one that was walked in sideways under an overhang.
/// A [`Cell::Ghost`] over it is not cover - it is the drop shadow of the pill after this one -
/// and neither is the pill in play itself.
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
    // the deterministic ai on a full bottle, which is where an overhang is worth getting under
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
    // one key every 400 ms against a 500 ms lock delay: a tuck is a rest and then one move, and
    // the move has that whole delay to happen in
    let played = play_with(DrAiAgent::n64(), 15, Duration::from_millis(400), 600);
    assert!(
        played.tucked > 0,
        "the speed limited agent never got a half under an overhang in {} pills",
        played.pills
    );
}

#[test]
fn an_agent_offered_the_held_pill_still_plays_and_does_swap() {
    // the hand written scorer with hold on: it has no learned opinion about a swap, so what
    // this checks is that the agent can execute one at all - press hold, wait for the swapped
    // pill to spawn, and then play the plan it made for that pill rather than throwing it away
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
    // if the agent's key presses were not reaching the game every pill would land where it spawned
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
    // clearing a bottle has to hand over to the next one, which is what a training run counts
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

/// The deterministic ai is the one every difficulty and the demo actually play, so unlike the
/// network it is worth holding to a standard here.
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
#[ignore = "needs a trained model: run with --ignored after a ga dr auto run"]
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
