//! A bottle and two placements for every network input: the placements that score highest and
//! lowest on it, searched with [`crate::game::ai::placement`] rather than hand picked so the pair
//! always moves the input. [`super::explain_main`] scores them and `feature_shots` draws them.
//!
//! A bottle is written as a picture, bottom rows last, in [`BOTTLE_WIDTH`] columns:
//!
//! | | |
//! |--|--|
//! | `.` | empty |
//! | `R` `B` `Y` | a virus, red / blue / yellow |
//! | `r` `b` `y` | a settled block of that colour |

use crate::game::ai::evaluator::raw_inputs;
use crate::game::ai::features::BottleAnalysis;
use crate::game::ai::imitation;
use crate::game::ai::n64::N64Ai;
use crate::game::ai::placement::{Placement, PlacementSearch, Reach};
use crate::game::block::Block;
use crate::game::bottle::{Bottle, BOTTLE_HEIGHT, BOTTLE_WIDTH};
use crate::game::geometry::BottlePoint;
use crate::game::pill::PillShape;
use crate::game::pill::VirusColor::{Blue, Red, Yellow};

/// One input, drawn: the bottle before the pill and two placements, each as landed and as
/// settled. `[0]` is the placement that moves the input furthest from zero.
pub struct FeatureScenario {
    /// which input of [`super::INPUTS`] this shows
    pub input: usize,
    /// the bottle both placements were made into
    pub before: Bottle,
    /// the bottle the instant the pill locks, before anything clears
    pub landed: [Bottle; 2],
    /// the cells the first clear takes out of [`Self::landed`]
    pub destroyed: [Vec<BottlePoint>; 2],
    /// rounds of clearing each placement sets off; only the first is in [`Self::destroyed`]
    pub rounds: [usize; 2],
    /// the bottle each leaves behind, cleared and cascaded out
    pub after: [Bottle; 2],
    /// where each placement's halves came to rest
    pub placed: [[BottlePoint; 2]; 2],
    /// the input's value for each, in its own units, before any centring or scaling
    pub value: [f64; 2],
    /// whether this had to be found in a real game because no drawn bottle separated the input
    pub found: bool,
}

impl FeatureScenario {
    /// whether the picture actually shows anything: two placements the input can tell apart
    pub fn separates(&self) -> bool {
        self.value[0] != self.value[1]
    }
}

/// Read a picture of a bottle. Rows are given top first and are padded out, so a picture only
/// has to draw the part of the bottle it is about.
fn bottle_of(rows: &[&str]) -> Bottle {
    let mut bottle = Bottle::new();
    let top = BOTTLE_HEIGHT as usize - rows.len();
    for (row, line) in rows.iter().enumerate() {
        for (column, character) in line.chars().enumerate() {
            assert!(column < BOTTLE_WIDTH as usize, "row too wide: {}", line);
            let block = match character {
                '.' => continue,
                'R' => Block::Virus(Red),
                'B' => Block::Virus(Blue),
                'Y' => Block::Virus(Yellow),
                'r' => Block::Garbage(Red),
                'b' => Block::Garbage(Blue),
                'y' => Block::Garbage(Yellow),
                other => panic!("unknown cell '{}'", other),
            };
            bottle.place(column as u32, (top + row) as u32, block);
        }
    }
    bottle
}

/// What a placement leaves behind, in the states a picture wants. [`Placement`] keeps only the
/// settled bottle, so the drop is replayed here to recover the landed one.
struct Landed {
    /// the halves in the bottle, before anything clears
    bottle: Bottle,
    /// the cells the first round of clearing takes
    destroyed: Vec<BottlePoint>,
    /// how many rounds of clearing follow
    rounds: usize,
}

fn replay(before: &Bottle, landing: [(BottlePoint, crate::game::pill::VirusColor); 2]) -> Landed {
    let mut bottle = before.clone();
    for (point, colour) in landing {
        bottle.place(point.x() as u32, point.y() as u32, Block::Garbage(colour));
    }
    let landed = bottle.clone();
    let mut destroyed = vec![];
    let mut rounds = 0;
    loop {
        let (blocks, _) = bottle.pattern();
        if blocks.is_empty() {
            break;
        }
        if rounds == 0 {
            destroyed = blocks.iter().map(|block| block.position).collect();
        }
        rounds += 1;
        bottle.destroy(blocks);
        while bottle.step_down_garbage() {}
    }
    Landed {
        bottle: landed,
        destroyed,
        rounds,
    }
}

/// the pills the search is run with: every single-colour pill and every mixed pair
const PILLS: [(crate::game::pill::VirusColor, crate::game::pill::VirusColor); 6] = [
    (Red, Red),
    (Blue, Blue),
    (Yellow, Yellow),
    (Red, Blue),
    (Blue, Yellow),
    (Yellow, Red),
];

/// every placement of every one of [`PILLS`] in this bottle
fn all_placements(before: &Bottle) -> Vec<Placement> {
    let stats = before.stats();
    PILLS
        .iter()
        .flat_map(|(left, right)| {
            let mut with_pill = before.clone();
            if with_pill.try_spawn(PillShape::new(*left, *right)).is_none() {
                return vec![];
            }
            with_pill.placements_within(Reach::Tuck, stats)
        })
        .collect()
}

/// The minimal bottle each input is drawn in, in [`super::INPUTS`] order, or the two bottles a
/// context input is compared across. `ga dr explain` reports any that stop separating their input.
const SCENARIOS: [Scene; engine::ai::BOTTLE_FEATURE_INPUTS] = [
    // how the bottle moved
    // a lone virus: a matching half lowers the work, another colour raises it
    Scene::one(&["...R...."]),
    // the row's work moves
    Scene::one(&["Rr......"]),
    // the column's work moves
    Scene::one(&["r.......", "R......."]),
    // a virus whose row is dead, so another colour on top buries it
    Scene::one(&["Rb......"]),
    // two loose halves: a third lowers the work on all three
    Scene::one(&["rr......"]),
    // a roofed pit a half can be tucked into, where no line can grow
    Scene::one(&["b.b.....", "..b.....", ".bb....."]),
    // what the placement did
    // two of a colour, so a half can land one short of a match
    Scene::one(&["rr......"]),
    // a gapped run: work says one short, touching says two
    Scene::one(&["r.r....."]),
    // a virus in the run a half lands in
    Scene::one(&["Rr......"]),
    // a half lands exactly one short, or exactly two
    Scene::one(&["rr......"]),
    Scene::one(&["r......."]),
    // what is one block from going, by axis
    Scene::one(&["Rr......"]),
    Scene::one(&["r.......", "R......."]),
    Scene::one(&["rr......"]),
    Scene::one(&["r.......", "r......."]),
    // empty: a pill stood up raises its column twice as far as one laid flat
    Scene::one(&["........"]),
    // a virus two blocks short: a matching half on top kills it
    Scene::one(&["r.......", "r.......", "R......."]),
    // two runs one short, stacked: a pill stood up between them clears both at once
    Scene::one(&["rrr.....", "bbb....."]),
    // empty: a pill in the middle raises the spawn columns, one at the side does not
    Scene::one(&["........"]),
    // context: two bottles, not two placements
    Scene::two(&FULL, &EMPTY),
    Scene::two(&FULL, &EMPTY),
    Scene::two(&FULL, &EMPTY),
    Scene::two(&FULL, &EMPTY),
    // hold is off, so this is zero in any bottle at all
    Scene::two(&FULL, &EMPTY),
];

/// the two ends of every context input: a bottle with a game left in it, and one nearly done
const FULL: [&str; 5] = [
    "..b.....", //
    "R.b..By.", //
    "y.b.rBy.", //
    "rrY.rRyb", //
    "ryybyRyb", //
];
const EMPTY: [&str; 1] = ["...R...."];

/// which bottle or bottles one input is drawn in
struct Scene {
    high: &'static [&'static str],
    /// the second bottle, for a context input. `None` where both placements share one.
    low: Option<&'static [&'static str]>,
}

impl Scene {
    const fn one(rows: &'static [&'static str]) -> Self {
        Self {
            high: rows,
            low: None,
        }
    }
    const fn two(high: &'static [&'static str], low: &'static [&'static str]) -> Self {
        Self {
            high,
            low: Some(low),
        }
    }
}

/// The pair of placements an input separates most widely in its bottle, furthest from zero first.
fn scenario(input: usize, before: &Bottle, placements: &[Placement]) -> FeatureScenario {
    let values: Vec<f64> = placements
        .iter()
        .map(|p| raw_inputs(&p.features())[input])
        .collect();
    let high = (0..values.len())
        .max_by(|a, b| values[*a].total_cmp(&values[*b]))
        .expect("the bottle offered no placement");
    let low = (0..values.len())
        .min_by(|a, b| values[*a].total_cmp(&values[*b]))
        .expect("the bottle offered no placement");
    let (first, second) = if values[high].abs() >= values[low].abs() {
        (high, low)
    } else {
        (low, high)
    };
    read_pair(input, before, placements, first, second)
}

/// gather the two placements a scenario is built from, in the order they are to be drawn
pub(crate) fn read_pair(
    input: usize,
    before: &Bottle,
    placements: &[Placement],
    first: usize,
    second: usize,
) -> FeatureScenario {
    let read = |at: usize| {
        let placement = &placements[at];
        let landed = replay(before, placement.landing());
        (
            landed.bottle,
            landed.destroyed,
            landed.rounds,
            placement.settled().clone(),
            placement.landing().map(|(point, _)| point),
            raw_inputs(&placement.features())[input],
        )
    };
    let (a_landed, a_destroyed, a_rounds, a_after, a_placed, a_value) = read(first);
    let (b_landed, b_destroyed, b_rounds, b_after, b_placed, b_value) = read(second);
    FeatureScenario {
        input,
        before: before.clone(),
        landed: [a_landed, b_landed],
        destroyed: [a_destroyed, b_destroyed],
        rounds: [a_rounds, b_rounds],
        after: [a_after, b_after],
        placed: [a_placed, b_placed],
        value: [a_value, b_value],
        found: false,
    }
}

/// A context input is the same for every placement of a pill, so it is drawn as one placement
/// in each of two bottles.
fn context_scenario(input: usize, high: &Bottle, low: &Bottle) -> FeatureScenario {
    let bottles = [high, low];
    let placements: Vec<Placement> = bottles
        .iter()
        .map(|bottle| {
            all_placements(bottle)
                .into_iter()
                .next()
                .expect("the bottle offered no placement")
        })
        .collect();
    let read = |at: usize| {
        let placement = &placements[at];
        let landed = replay(bottles[at], placement.landing());
        (
            landed.bottle,
            landed.destroyed,
            landed.rounds,
            placement.settled().clone(),
            placement.landing().map(|(point, _)| point),
            raw_inputs(&placement.features())[input],
        )
    };
    let (a_landed, a_destroyed, a_rounds, a_after, a_placed, a_value) = read(0);
    let (b_landed, b_destroyed, b_rounds, b_after, b_placed, b_value) = read(1);
    FeatureScenario {
        input,
        before: high.clone(),
        landed: [a_landed, b_landed],
        destroyed: [a_destroyed, b_destroyed],
        rounds: [a_rounds, b_rounds],
        after: [a_after, b_after],
        placed: [a_placed, b_placed],
        value: [a_value, b_value],
        found: false,
    }
}

/// How many bottles are taken out of a real game, and how far apart, for an input whose drawn
/// bottle fails to separate it. The seed, level and ai are fixed, so the bottles are too.
const SNAPSHOT_SEED: u128 = 7;
const SNAPSHOT_LEVEL: u32 = 12;
const SNAPSHOT_EVERY: usize = 5;
const SNAPSHOTS: usize = 40;

/// bottles out of a real game, one every [`SNAPSHOT_EVERY`] pills
fn snapshots() -> Vec<Bottle> {
    let ai = N64Ai::new();
    let mut taken = vec![];
    let mut pill = 0;
    imitation::play_games(SNAPSHOT_SEED, SNAPSHOT_LEVEL, |bottle, placements| {
        if pill % SNAPSHOT_EVERY == 0 && taken.len() < SNAPSHOTS {
            // `hold` takes the pill in play back out of the bottle
            let mut snapshot = bottle.clone();
            snapshot.hold();
            taken.push(snapshot);
        }
        pill += 1;
        ai.choose(bottle, placements)
    });
    taken
}

/// The widest pair over bottles from a real game, for an input no drawn bottle could separate.
fn found_scenario(input: usize) -> Option<FeatureScenario> {
    let mut best: Option<(f64, FeatureScenario)> = None;
    for bottle in snapshots() {
        let placements = all_placements(&bottle);
        if placements.len() < 2 {
            continue;
        }
        let drawn = scenario(input, &bottle, &placements);
        let separation = (drawn.value[0] - drawn.value[1]).abs();
        if separation > 0.0 && best.as_ref().is_none_or(|(widest, _)| separation > *widest) {
            best = Some((separation, drawn));
        }
    }
    best.map(|(_, mut drawn)| {
        drawn.found = true;
        drawn
    })
}

/// Every input, in [`super::INPUTS`] order.
pub fn scenarios() -> Vec<FeatureScenario> {
    SCENARIOS
        .iter()
        .enumerate()
        .map(|(input, scene)| match scene.low {
            Some(low) => context_scenario(input, &bottle_of(scene.high), &bottle_of(low)),
            None => {
                let before = bottle_of(scene.high);
                let placements = all_placements(&before);
                let drawn = scenario(input, &before, &placements);
                // a drawn bottle is preferred unless it cannot show its input
                if drawn.separates() {
                    drawn
                } else {
                    found_scenario(input).unwrap_or(drawn)
                }
            }
        })
        .collect()
}
