//! A bottle and two placements for every input of the network: the placement that scores
//! *highest* on that input and the one that scores *lowest*, so the pair shows what the input
//! is measuring rather than what the author of the picture believed it measured.
//!
//! They are found rather than written. Hand picking two landings per input was tried first and
//! twenty one of the thirty two pairs turned out not to move the input they were about at all -
//! a picture that shows nothing, and one that would go quietly stale the first time a feature
//! changed. So the placements come out of the real placement search
//! ([`crate::game::ai::placement`]), every legal one of several pills over a handful of hand
//! drawn bottles, and the pair with the widest separation wins. What is drawn is then a
//! position the agent could actually have reached, and the numbers under it are the numbers the
//! network is really fed.
//!
//! They are here rather than in a test because two things need them and neither can build them:
//! [`super::explain_main`] scores them, which needs the crate's private feature code, and
//! `cargo run --example feature_shots` draws them, which needs a theme and a window. What they
//! share is the [`Bottle`], which is public.
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

/// One input, drawn: the bottle before the pill, and two placements of it, each in the two
/// states worth seeing - the moment the halves land, and what is left once everything has
/// cleared and fallen.
///
/// `[0]` is whichever placement moves the input **furthest from zero**, which is the one where
/// the thing the input measures actually happens: the virus count is a clear taking three
/// viruses against a placement taking none, rather than two placements that both take none.
pub struct FeatureScenario {
    /// which input of [`super::INPUTS`] this shows
    pub input: usize,
    /// the bottle both placements were made into
    pub before: Bottle,
    /// The bottle the instant the pill locks: the two halves are in it and **nothing has
    /// cleared yet**, which is the only state in which where they landed can be seen. A
    /// placement that clears takes its own halves with it, so in the settled bottle the cells
    /// they were in are empty.
    pub landed: [Bottle; 2],
    /// the cells the first clear takes out of [`Self::landed`], for a renderer that can draw
    /// them going. Empty where the placement clears nothing.
    pub destroyed: [Vec<BottlePoint>; 2],
    /// how many rounds of clearing each placement sets off. More than one is a cascade, and
    /// only the first round's cells are in [`Self::destroyed`].
    pub rounds: [usize; 2],
    /// the bottle each leaves behind, cleared and cascaded out
    pub after: [Bottle; 2],
    /// where each placement's halves came to rest
    pub placed: [[BottlePoint; 2]; 2],
    /// the input's value for each, in its own units, before any centring or scaling
    pub value: [f64; 2],
    /// Whether this had to be found in a real game rather than drawn. A minimal bottle is
    /// always preferred - it is the whole point of the pictures - but some inputs describe a
    /// situation that cannot be *set up* in a few blocks, only arrived at.
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

/// What a placement leaves behind, in the states a picture wants.
///
/// [`Placement`] keeps only the settled bottle, because that is all the scorers read and
/// carrying a second one per candidate would cost the search real memory. A picture needs the
/// state *before* the clear as well - a placement that clears takes its own halves with it, so
/// in the settled bottle the cells they landed in are empty and there is nothing to point at -
/// so the drop is replayed here, for the thirty odd placements that end up being drawn.
struct Landed {
    /// the halves in the bottle, before anything clears
    bottle: Bottle,
    /// The cells the *first* round of clearing takes, which is the round that happens where the
    /// pill just landed and the only one whose cells are still where a picture could ring them.
    destroyed: Vec<BottlePoint>,
    /// How many rounds of clearing follow. More than one is a cascade, and a cascade is why a
    /// picture of the first round can show one virus going while the placement's virus count
    /// says three went: the other two are taken by rounds that only happen once what the first
    /// round unsupported has fallen, by which time those cells are somewhere else.
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

/// The pills the search is run with. Two halves of one colour and two of different ones, over
/// all three colours, which between them reach every clear and every stranding these bottles
/// have to offer.
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

/// The bottle each input is drawn in, in [`super::INPUTS`] order, and for a context input the
/// two bottles it is compared across.
///
/// **One bottle per input, built to show that input and as little else as possible.** They were
/// snapshots of a real game to begin with, which guaranteed that every input could be separated
/// somewhere but made the pictures unreadable: a placement worth one virus, drawn in a bottle
/// holding thirty nine of them and a hundred loose halves, shows nothing a reader can find. A
/// virus count is best drawn with a single virus in it.
///
/// The *placements* are still searched rather than written, so a bottle only has to contain the
/// right pieces and never the right answer - `ga dr explain` says so if one of them stops
/// separating the input it is for.
const SCENARIOS: [Scene; engine::ai::BOTTLE_FEATURE_INPUTS] = [
    // ---- how the bottle moved
    // a virus and two of its colour: one half completes the line and takes the virus
    Scene::one(&["Rrr....."]),
    // one virus and nothing else: a matching half lowers the work, another colour raises it
    Scene::one(&["...R...."]),
    // one virus to bury under a different colour
    Scene::one(&["...R...."]),
    // one loose half to bury under a different colour
    Scene::one(&["...r...."]),
    // empty: a pill stood up raises the stack twice as far as one laid flat
    Scene::one(&["........"]),
    // empty: over the spawn columns against out of the way
    Scene::one(&["........"]),
    // one step: a pill laid across it leaves a hole under the overhang
    Scene::one(&["r......."]),
    // ---- runs one and two short
    // a virus and one of its colour along the floor
    Scene::one(&["Rr......"]),
    // the same standing up, so the run that grows is a column
    Scene::one(&["r.......", "R......."]),
    Scene::one(&["R......."]),
    Scene::one(&["R......."]),
    // two loose halves of a colour: no virus in the run, so it only tidies
    Scene::one(&["rr......"]),
    Scene::one(&["r......."]),
    // ---- what the placement did
    // three in a row: one half clears them
    Scene::one(&["rrr....."]),
    // two of a colour to land against
    Scene::one(&["rr......"]),
    // a run with a gap in it a pill could still drop into
    Scene::one(&["r.r....."]),
    Scene::one(&["rr......"]),
    Scene::one(&["r......."]),
    // a virus in the run the half lands in
    Scene::one(&["R......."]),
    // A roofed pit one cell wide, with the column beside it open so a half can be walked in
    // under the roof and nothing else can join it: the roof stops a line growing upwards and
    // the wall two along stops one growing sideways.
    Scene::one(&["b.b.....", "..b.....", ".bb....."]),
    // the same, with a virus under the pit to be stranded on top of
    Scene::one(&["b.b.....", "..b.....", "Rbb....."]),
    // one virus to come to rest on
    Scene::one(&["...R...."]),
    // the roofed pit again: a half walked in covers the virus *and* is stranded
    Scene::one(&["b.b.....", "..b.....", "Rbb....."]),
    // three in a row left standing: the next pill has somewhere to clear
    Scene::one(&["rr......"]),
    // the same with a virus in it
    Scene::one(&["Rr......"]),
    // Three reds along the floor and three yellows on top of them, with a fourth yellow waiting
    // over the gap: a half completing the reds clears them, and the yellows that were resting on
    // them fall into a line of their own. `chains` asks whether the bottle a placement *leaves*
    // is one half away from exactly that.
    Scene::one(&["...y....", "yyy.....", "rrr....."]),
    // ---- what kind of bottle this is: two bottles, not two placements
    Scene::two(&FULL, &EMPTY),
    Scene::two(&FULL, &EMPTY),
    Scene::two(&FULL, &EMPTY),
    Scene::two(&FULL, &EMPTY),
    Scene::two(&HOLED, &EMPTY),
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
/// the same as `FULL` with the stack lifted off the floor, so it has holes under it
const HOLED: [&str; 5] = [
    "..b.....", //
    "R.b..By.", //
    "y.b.rBy.", //
    "rrY.rRyb", //
    "r.y.y.y.", //
];

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

/// The best picture of one input: over every placement of every pill in the bottle built for
/// it, the pair the input separates most widely, drawn with whichever end is *furthest from
/// zero* first - because that is the one where the thing the input measures actually happens.
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
fn read_pair(
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

/// A context input is the same for every placement of a pill by construction - that is what
/// makes it context - so no two placements can ever separate one. What separates them is two
/// *bottles*, so these are drawn as one placement in each.
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

/// How many bottles are taken out of a real game, and how far apart. Nothing uses these unless
/// the bottle drawn for an input fails to separate it - `place.chains` is the one that does.
/// Its situation is a bottle one half away from a clear that *cascades*, and every compact way
/// of setting that up either clears on the spot or buries the cell the clear needs; it is far
/// easier found than built. The seed and the level are fixed and the ai that plays them is
/// deterministic, so the same bottles come out every time.
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
            // the bottle a game hands over has the pill in play still in it, and a picture
            // wants the stack on its own: `hold` is what takes one back out
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
                // the drawn bottle is always preferred; only one that cannot show its input at
                // all gives way to a position found in real play
                if drawn.separates() {
                    drawn
                } else {
                    found_scenario(input).unwrap_or(drawn)
                }
            }
        })
        .collect()
}
