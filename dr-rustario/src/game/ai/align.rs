//! `ga dr align [model] [boards]`: where a brain's placed halves sit relative to the nearest cell
//! of their own colour along their row and column, against every candidate as the base rate.

use crate::game::ai::agent::DrAiAgent;
use crate::game::ai::evaluator::Scorer;
use crate::game::ai::features::BottleAnalysis;
use crate::game::ai::features::{BottleFeatures, Grid};
use crate::game::ai::genetic::{
    load_weights, training_fixture, UNSEEN_PER_LEVEL, UNSEEN_SEED_BLOCK,
};
use crate::game::ai::models::{self, DrNeuralNetwork};
use crate::game::ai::placement::{Placement, PlacementSearch, Reach};
use crate::game::ai::run::{N64_FIRE, START_LEVELS};
use crate::game::ai::N64Ai;
use crate::game::block::Block;
use crate::game::bottle::{Bottle, BOTTLE_HEIGHT, BOTTLE_WIDTH};
use crate::game::geometry::{BottlePoint, Rotation};
use crate::game::pill::VitaminOrdinal;
use engine::ai::Seed;
use rayon::prelude::*;

#[derive(Clone, Copy)]
#[allow(clippy::large_enum_variant)]
enum Brain {
    N64(N64Ai),
    Network(DrNeuralNetwork),
}

impl Brain {
    fn agent(&self) -> DrAiAgent {
        match self {
            Brain::N64(_) => DrAiAgent::n64(),
            Brain::Network(network) => DrAiAgent::new(*network),
        }
    }

    fn choose(&self, bottle: &Bottle, placements: &[Placement]) -> usize {
        match self {
            Brain::N64(ai) => ai.choose(bottle, placements).unwrap_or(0),
            Brain::Network(network) => {
                let features: Vec<BottleFeatures> =
                    placements.iter().map(|p| p.features()).collect();
                let scores = Scorer::Network(*network).rank(&features);
                (0..placements.len())
                    .max_by(|a, b| {
                        scores[*a]
                            .total_cmp(&scores[*b])
                            .then_with(|| placements[*b].inputs().cmp(placements[*a].inputs()))
                    })
                    .unwrap_or(0)
            }
        }
    }
}

/// How a placed half relates to the nearest cell of its colour, best first.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Relation {
    /// cleared something as it locked
    Cleared,
    Touching,
    /// within a window of four, every gap one a pill can fill now
    LiveGap,
    /// within a window of four, but a gap has nothing under it yet
    AirGap,
    /// within a window of four with another colour in between
    Blocked,
    /// four or more apart, nothing else in between
    FarOpen,
    /// four or more apart, another colour in between
    FarBlocked,
    /// nothing of its colour on its row or column
    Alone,
}

#[allow(dead_code)]
const RELATIONS: [Relation; 8] = [
    Relation::Cleared,
    Relation::Touching,
    Relation::LiveGap,
    Relation::AirGap,
    Relation::Blocked,
    Relation::FarOpen,
    Relation::FarBlocked,
    Relation::Alone,
];

fn in_bottle(x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && x < BOTTLE_WIDTH as i32 && y < BOTTLE_HEIGHT as i32
}

/// whether an empty cell has something under it, or the floor
fn supported(grid: &Grid, x: u32, y: u32) -> bool {
    y + 1 == BOTTLE_HEIGHT || grid.colour(x, y + 1).is_some()
}

/// the half at `at`, its partner at `partner`, read on the bottle as the pill locked
/// the half's relation, whether that is along its row, and whether the cell it found is a virus
fn relation(grid: &Grid, at: BottlePoint, partner: BottlePoint) -> (Relation, bool, bool) {
    let (x, y) = (at.x(), at.y());
    let colour = grid.colour(x as u32, y as u32).expect("a placed half");
    let mut best = (Relation::Alone, false, false);
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let (mut foreign, mut air) = (false, false);
        let mut step = 1;
        loop {
            let (nx, ny) = (x + dx * step, y + dy * step);
            if !in_bottle(nx, ny) {
                break;
            }
            let (ux, uy) = (nx as u32, ny as u32);
            match grid.colour(ux, uy) {
                // a same coloured partner is its own pill, not a target
                Some(c) if c == colour && BottlePoint::new(nx, ny) == partner => {}
                Some(c) if c == colour => {
                    let r = match (step, foreign, air) {
                        (1, _, _) => Relation::Touching,
                        (2..=3, false, false) => Relation::LiveGap,
                        (2..=3, false, true) => Relation::AirGap,
                        (2..=3, true, _) => Relation::Blocked,
                        (_, false, _) => Relation::FarOpen,
                        (_, true, _) => Relation::FarBlocked,
                    };
                    if r < best.0 {
                        best = (r, dy == 0, grid.is_virus(ux, uy));
                    }
                    break;
                }
                Some(_) => foreign = true,
                None => {
                    if !supported(grid, ux, uy) || !grid.reachable(ux, uy) {
                        air = true;
                    }
                }
            }
            step += 1;
        }
    }
    best
}

/// counts by key: chosen, then every candidate weighted to one pill
type Tally = std::collections::BTreeMap<String, [f64; 2]>;

fn bump(tally: &mut Tally, key: String, slot: usize, weight: f64) {
    tally.entry(key).or_default()[slot] += weight;
}

/// each half's relation and whether it is along a row, read as the pill locked
fn halves(bottle: &Bottle, placement: &Placement) -> Vec<(Relation, bool, bool)> {
    let landing = placement.landing();
    let mut landed = bottle.clone();
    for (point, colour) in landing {
        if in_bottle(point.x(), point.y()) {
            landed.place(
                point.x() as u32,
                point.y() as u32,
                Block::Stack(colour, Rotation::North, VitaminOrdinal::Left),
            );
        }
    }
    let grid = Grid::of(&landed);
    let cleared = placement.features().placement().patterns_cleared() > 0;
    (0..2)
        .filter(|i| in_bottle(landing[*i].0.x(), landing[*i].0.y()))
        .map(|i| {
            if cleared {
                (Relation::Cleared, false, false)
            } else {
                relation(&grid, landing[i].0, landing[1 - i].0)
            }
        })
        .collect()
}

/// The halves' best window as the scan reads it, but each empty cell costing itself and every
/// empty cell under it, since nothing can rest there until those are filled.
fn grounded_work(bottle: &Bottle, placement: &Placement) -> i32 {
    let landing = placement.landing();
    let mut landed = bottle.clone();
    for (point, colour) in landing {
        if in_bottle(point.x(), point.y()) {
            landed.place(
                point.x() as u32,
                point.y() as u32,
                Block::Stack(colour, Rotation::North, VitaminOrdinal::Left),
            );
        }
    }
    let grid = Grid::of(&landed);
    let mut best = i32::MAX;
    for (point, _) in landing {
        let (x, y) = (point.x(), point.y());
        if !in_bottle(x, y) {
            continue;
        }
        let colour = grid.colour(x as u32, y as u32).unwrap();
        for (dx, dy) in [(1i32, 0i32), (0, 1)] {
            for offset in 0..4 {
                let (sx, sy) = (x - dx * offset, y - dy * offset);
                if !in_bottle(sx, sy) || !in_bottle(sx + dx * 3, sy + dy * 3) {
                    continue;
                }
                let mut cost = 0;
                let mut live = true;
                for step in 0..4 {
                    let (cx, cy) = ((sx + dx * step) as u32, (sy + dy * step) as u32);
                    match grid.colour(cx, cy) {
                        Some(c) if c == colour => {}
                        Some(_) => live = false,
                        None if grid.reachable(cx, cy) => {
                            cost += 1;
                            // the column under a gap, down to whatever holds it up
                            if dx != 0 {
                                let mut below = cy + 1;
                                while below < BOTTLE_HEIGHT && grid.colour(cx, below).is_none() {
                                    cost += 1;
                                    below += 1;
                                }
                            }
                        }
                        None => live = false,
                    }
                }
                if live {
                    best = best.min(cost);
                }
            }
        }
    }
    best
}

fn productive(r: Relation) -> bool {
    r <= Relation::LiveGap
}

fn read(bottle: &Bottle, placement: &Placement, tally: &mut Tally, slot: usize, weight: f64) {
    let read = halves(bottle, placement);
    for (r, row, virus) in &read {
        bump(tally, format!("half {:?}", r), slot, weight);
        if *r >= Relation::Blocked && *r != Relation::Alone {
            let axis = if *row { "row" } else { "col" };
            let target = if *virus { "virus" } else { "vitamin" };
            bump(
                tally,
                format!("half {:?} {} {}", r, axis, target),
                slot,
                weight,
            );
        }
    }
    let best = read.iter().map(|h| h.0).min().unwrap_or(Relation::Alone);
    bump(tally, format!("pill {:?}", best), slot, weight);
    let unrelated = read
        .iter()
        .filter(|h| h.0 >= Relation::Blocked && h.0 != Relation::Alone)
        .count();
    bump(
        tally,
        format!("pill with {} halves aligned to nothing useful", unrelated),
        slot,
        weight,
    );
}

struct Example {
    before: Bottle,
    chosen: Bottle,
    relations: Vec<(Relation, bool, bool)>,
}

fn watch(brain: Brain, index: usize, boards: usize) -> (Tally, Vec<Example>) {
    let block = Seed::from(UNSEEN_SEED_BLOCK);
    let fixture = training_fixture(block, START_LEVELS.len() * UNSEEN_PER_LEVEL, Some(N64_FIRE));
    let (mut headless, _) = fixture.headless(brain.agent(), block, index);
    let mut tally = Tally::new();
    let mut examples = vec![];

    loop {
        let before = headless.choices();
        let bottle = headless.game().bottle().clone();
        let over = headless.update();
        if headless.choices().pills > before.pills {
            let placements = bottle.placements_within(Reach::default(), bottle.stats());
            if !placements.is_empty() {
                let chosen = brain.choose(&bottle, &placements);
                read(&bottle, &placements[chosen], &mut tally, 0, 1.0);
                let weight = 1.0 / placements.len() as f64;
                for p in &placements {
                    read(&bottle, p, &mut tally, 1, weight);
                }
                let mine = halves(&bottle, &placements[chosen]);
                let useless = |r: Relation| r >= Relation::Blocked && r != Relation::Alone;
                if mine.len() == 2
                    && mine.iter().filter(|h| useless(h.0)).count() == 1
                    && mine.iter().any(|h| productive(h.0))
                {
                    let both = placements.iter().any(|p| {
                        let h = halves(&bottle, p);
                        h.len() == 2 && h.iter().all(|x| productive(x.0))
                    });
                    bump(
                        &mut tally,
                        format!("one useless half, both useful on offer {}", both),
                        0,
                        1.0,
                    );
                }
                // what the scan credits the halves against what the gaps would really take
                for (slot, p) in placements.iter().enumerate() {
                    let work = p.features().placement().halves_work();
                    if (1..=2).contains(&work) {
                        let grounded = grounded_work(&bottle, p).min(4);
                        let key = format!("halves_work {} credited, grounded {}", work, grounded);
                        if slot == chosen {
                            bump(&mut tally, key.clone(), 0, 1.0);
                        }
                        bump(&mut tally, key, 1, weight);
                    }
                }
                bump(&mut tally, "pills".into(), 0, 1.0);
                bump(&mut tally, "pills".into(), 1, 1.0);
                if mine.iter().all(|h| !productive(h.0)) {
                    let offered = placements
                        .iter()
                        .any(|p| halves(&bottle, p).iter().any(|h| productive(h.0)));
                    bump(
                        &mut tally,
                        format!("unproductive pill, productive on offer {}", offered),
                        0,
                        1.0,
                    );
                }
                if examples.len() < boards
                    && mine.iter().any(|h| h.0 == Relation::Blocked)
                    && mine.iter().all(|h| !productive(h.0))
                {
                    let mut landed = bottle.clone();
                    for (point, colour) in placements[chosen].landing() {
                        if in_bottle(point.x(), point.y()) {
                            landed.place(
                                point.x() as u32,
                                point.y() as u32,
                                Block::Stack(colour, Rotation::North, VitaminOrdinal::Left),
                            );
                        }
                    }
                    examples.push(Example {
                        before: bottle.clone(),
                        chosen: landed,
                        relations: mine,
                    });
                }
            }
        }
        if over.is_some() {
            return (tally, examples);
        }
    }
}

pub fn ga_main_align(args: &[String]) -> Result<(), String> {
    let name = args.first().map(String::as_str).unwrap_or("embedded");
    let brain = match name {
        "n64" => Brain::N64(N64Ai::new()),
        "embedded" => Brain::Network(models::survival_trained()),
        path => Brain::Network(load_weights(path)?.into()),
    };
    let boards: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let games = START_LEVELS.len() * UNSEEN_PER_LEVEL;
    let watched: Vec<(Tally, Vec<Example>)> = (0..games)
        .into_par_iter()
        .map(|index| watch(brain, index, boards))
        .collect();
    let mut tally = Tally::new();
    for (t, _) in &watched {
        for (key, value) in t {
            let entry = tally.entry(key.clone()).or_default();
            entry[0] += value[0];
            entry[1] += value[1];
        }
    }
    let pills = tally["pills"][0];
    let halves: f64 = tally
        .iter()
        .filter(|(k, _)| k.starts_with("half ") && k.split(' ').count() == 2)
        .map(|(_, v)| v[0])
        .sum();
    println!("{name}: {pills} pills over {games} games");
    println!("  {:<48}{:>9}{:>9}{:>7}", "", "chosen%", "offered%", "lift");
    for (key, [c, o]) in &tally {
        let per = if key.starts_with("half") {
            halves
        } else {
            pills
        };
        let (c, o) = (c / per * 100.0, o / per * 100.0);
        println!(
            "  {:<48}{:>9.2}{:>9.2}{:>7.2}",
            key,
            c,
            o,
            if o > 0.0 { c / o } else { 0.0 }
        );
    }
    for example in watched.iter().flat_map(|w| &w.1).take(boards) {
        println!("\n{:?}", example.relations);
        let left: Vec<String> = format!("{:?}", example.before)
            .lines()
            .map(str::to_string)
            .collect();
        let right: Vec<String> = format!("{:?}", example.chosen)
            .lines()
            .map(str::to_string)
            .collect();
        for (l, r) in left.iter().zip(&right) {
            println!("    {:<16}{}", l, r);
        }
    }
    Ok(())
}
