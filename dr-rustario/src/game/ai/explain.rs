//! `ga dr explain`: what every input of the network is, what it is for, and how much the model
//! that is embedded actually uses it.
//!
//! This is the companion to [`crate::game::ai::probe`] and asks a different question. The probe
//! asks what a *feature set* could express about the deterministic ai it was designed to
//! reproduce - a question about the features. This asks what the *trained model* does with each
//! of them - a question about the network - and the two do not agree, which is the point. The
//! model has beaten the ai it learned from, so how well an input predicts that ai has stopped
//! being the measure of whether it is worth feeding.
//!
//! Three numbers per input, weakest evidence first:
//!
//! 1. **weight** - the size of the first layer's weight column for that input. It is the only
//!    one of the three that can be read straight off the model, and an input whose column is
//!    all zeros is provably doing nothing at all. Anything else it says is weak: a large column
//!    on an input that never moves is still nothing, and the layers after it can undo whatever
//!    the first one does.
//! 2. **spread** - how far the input actually moves between the placements of one pill, in the
//!    units the network is fed. An input with no spread cannot rank anything whatever its
//!    weights are, since every candidate gets the same value.
//! 3. **mind changes** - the model is asked to play with that input *silenced*
//!    ([`engine::ai::NeuralNetwork::silence_input`]) and how often it then picks a different
//!    placement is counted. This is the one that matters: it is the model's own answer, over
//!    real pills, about what it would do differently without the input.

use crate::game::ai::evaluator::{self, Scorer};
use crate::game::ai::features::BottleFeatures;
use crate::game::ai::imitation;
use crate::game::ai::models::{self, DrNeuralNetwork};
use crate::game::ai::placement::Placement;
use crate::game::bottle::{Bottle, BOTTLE_HEIGHT, BOTTLE_WIDTH};
use engine::ai::BOTTLE_FEATURE_INPUTS;

mod scenarios;
pub use scenarios::{scenarios, FeatureScenario};

/// What every input is called and what it is for, in [`evaluator::raw_inputs`]'s order. The
/// names match the probe's so the two reports can be read side by side.
#[rustfmt::skip]
pub const INPUTS: [Input; BOTTLE_FEATURE_INPUTS] = [
    Input::comparative("delta.viruses", "how many viruses the bottle lost. Negative is good, and it is the only input that says a virus actually died rather than that one became easier to reach."),
    Input::comparative("delta.virus_work", "how the total work left changed: for every virus, the fewest matching blocks a line of four through it still needs, with a virus nothing can reach any more counted as worse than any reachable one. This is what points the agent at the viruses instead of at a tidy heap in the corner."),
    Input::comparative("delta.buried_viruses", "differently coloured blocks stacked on top of viruses. Everything of another colour above a virus has to clear before the virus can be reached from above."),
    Input::comparative("delta.buried_blocks", "the same count for everything that is not a virus: how much of the stack is trapped under other colours."),
    Input::comparative("delta.max_height", "how the tallest column moved."),
    Input::comparative("delta.entrance_height", "how high the two columns a pill spawns over stand. Not the tallest column and not the average one - the only height that can actually end a game."),
    Input::comparative("delta.holes", "empty cells with something above them in the same column. A hole is a cell a pill can no longer be dropped into."),
    Input::comparative("delta.virus_3_row", "runs of three along a row, one short of a match, that contain a virus and have somewhere left to finish. A run walled in at both ends is not counted, and neither is one whose only gap is under an overhang."),
    Input::comparative("delta.virus_3_col", "the same, down a column. Counted apart from the row because the N64 pays for them out of different tables."),
    Input::comparative("delta.virus_2_row", "runs of two along a row with a virus in them and room to reach four."),
    Input::comparative("delta.virus_2_col", "the same, down a column."),
    Input::comparative("delta.block_3_row", "runs of three along a row with no virus in them. Clearing one only tidies the bottle up, so it is worth far less than the virus version."),
    Input::comparative("delta.block_3_col", "the same, down a column."),
    Input::comparative("place.patterns_cleared", "how many runs this placement cleared, cascades included."),
    Input::comparative("place.touching", "the longest run of one colour actually touching a half of the pill - what would clear, right now, if it reached four."),
    Input::comparative("place.reach", "how long that run could still become, counting the gaps a pill could drop a half into. It is what makes the agent build towards a clear rather than only taking the ones in front of it."),
    Input::comparative("place.open_3", "halves left one short of a match with room to finish. The single thing the N64 pays most for."),
    Input::comparative("place.open_2", "the same, two short."),
    Input::comparative("place.run_viruses", "viruses in the runs the halves landed in, which is what makes building one worth doing at all."),
    Input::comparative("place.stranded", "halves left where no line through them can ever reach four. Taking this out of the N64 changes its mind on 44% of pills - it is a veto rather than a preference."),
    Input::comparative("place.stranded_on_virus", "stranded, and sitting on a virus, which is worse again."),
    Input::comparative("place.covers_virus", "halves that came to rest on top of a virus with nothing else of the sort above them."),
    Input::comparative("place.buries_virus", "the same, where the half is also stranded: a virus buried for nothing."),
    Input::comparative("place.one_away", "how many ways the next pill could clear something in the bottle this placement leaves behind. Every colour is tried at every landing cell, since the queue is not read."),
    Input::comparative("place.one_away_virus", "how many of those would take a virus with them."),
    Input::comparative("place.chains", "whether one of them would cascade into a second clear."),
    Input::context("context.viruses", "how many viruses are left in the bottle before the pill. Context, not a ranking signal: it is the same for every candidate by construction."),
    Input::context("context.virus_work", "the work the bottle still needs before the pill."),
    Input::context("context.max_height", "how high the bottle stands before the pill."),
    Input::context("context.entrance_height", "how high the spawn columns stand before the pill."),
    Input::context("context.holes", "how many holes the bottle has before the pill."),
    Input::context("context.held", "whether this candidate is a placement of the pill being *held* rather than the one in play. Zero everywhere while hold is off, which is everywhere."),
];

/// one input of the network: what it is called, what it measures, and whether it can rank
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub name: &'static str,
    pub purpose: &'static str,
    /// Whether it is centred on the candidates of the pill in play. The rest are context and
    /// are the same for every candidate, so they cannot separate them - they are there to say
    /// what kind of bottle this is.
    pub comparative: bool,
}

impl Input {
    const fn comparative(name: &'static str, purpose: &'static str) -> Self {
        Self {
            name,
            purpose,
            comparative: true,
        }
    }
    const fn context(name: &'static str, purpose: &'static str) -> Self {
        Self {
            name,
            purpose,
            comparative: false,
        }
    }
}

/// What the embedded model does with one input.
pub struct Influence {
    pub input: usize,
    /// the size of the first layer's weight column: zero is proof of nothing at all
    pub weight: f64,
    /// how far the input moves between the placements of one pill, as the network is fed it
    pub spread: f64,
    /// the share of pills the model plays differently when the input is silenced
    pub mind_changes: f64,
    /// how much worse it plays without it, in viruses over the games it was measured on
    pub viruses_without: u32,
}

/// The first layer's weight column for `input`, as its length.
///
/// The layer is a `Tensor<WIDTH, IN>` flattened one neuron at a time, so the weights an input
/// reaches are every `IN`th number from its own index. Reading them off the flattened genome
/// rather than through the network is deliberate: it is the same order
/// [`crate::game::ai::genetic`] prints and mutation walks, so what is measured here is what a
/// run is actually moving about.
pub fn weight_column(network: &DrNeuralNetwork, input: usize) -> f64 {
    let flat = network.flatten();
    (0..BOTTLE_FEATURE_INPUTS)
        .map(|neuron| flat[neuron * BOTTLE_FEATURE_INPUTS + input].powi(2))
        .sum::<f64>()
        .sqrt()
}

/// How far each input moves between the placements of one pill, averaged over `pills` of them,
/// in the units the network is fed.
fn spreads(corpus: &[Vec<BottleFeatures>]) -> [f64; BOTTLE_FEATURE_INPUTS] {
    let mut totals = [0.0; BOTTLE_FEATURE_INPUTS];
    for pill in corpus {
        let rows = evaluator::inputs(pill);
        for (input, total) in totals.iter_mut().enumerate() {
            let mean = rows.iter().map(|row| row[input]).sum::<f64>() / rows.len() as f64;
            let variance = rows
                .iter()
                .map(|row| (row[input] - mean).powi(2))
                .sum::<f64>()
                / rows.len() as f64;
            *total += variance.sqrt();
        }
    }
    totals.map(|total| total / corpus.len().max(1) as f64)
}

/// Every pill the model was dealt over `seeds` games, as the candidates it was choosing between.
fn corpus(network: DrNeuralNetwork, seeds: u128, level: u32) -> Vec<Vec<BottleFeatures>> {
    let mut pills: Vec<Vec<BottleFeatures>> = vec![];
    let scorer = Scorer::Network(network);
    imitation::play_games(
        seeds,
        level,
        |_bottle: &Bottle, placements: &[Placement]| {
            let features: Vec<BottleFeatures> = placements.iter().map(|p| p.features()).collect();
            if features.len() > 1 {
                pills.push(features.clone());
            }
            let scores = scorer.rank(&features);
            (0..placements.len()).max_by(|a, b| scores[*a].total_cmp(&scores[*b]))
        },
    );
    pills
}

/// Play the model with `input` silenced and count how often it plays a different pill.
fn mind_changes(network: DrNeuralNetwork, corpus: &[Vec<BottleFeatures>], input: usize) -> f64 {
    let mut silenced = network;
    silenced.silence_input(input);
    let (full, cut) = (Scorer::Network(network), Scorer::Network(silenced));
    let changed = corpus
        .iter()
        .filter(|pill| {
            let best = |scorer: &Scorer| {
                let scores = scorer.rank(pill);
                (0..pill.len()).max_by(|a, b| scores[*a].total_cmp(&scores[*b]))
            };
            best(&full) != best(&cut)
        })
        .count();
    changed as f64 / corpus.len().max(1) as f64
}

/// How the model plays with `input` silenced, in viruses over `seeds` whole games.
fn plays_without(network: DrNeuralNetwork, input: usize, seeds: u128, level: u32) -> u32 {
    let mut silenced = network;
    silenced.silence_input(input);
    let scorer = Scorer::Network(silenced);
    imitation::play_games(seeds, level, |_: &Bottle, placements: &[Placement]| {
        let features: Vec<BottleFeatures> = placements.iter().map(|p| p.features()).collect();
        let scores = scorer.rank(&features);
        (0..placements.len()).max_by(|a, b| scores[*a].total_cmp(&scores[*b]))
    })
    .viruses
}

/// `ga dr explain [pills] [seeds] [level]`
pub fn explain_main(args: &[String]) -> Result<(), String> {
    let pills: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(2000);
    let seeds: u128 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(3);
    let level: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
    let network = models::survival_trained();

    println!(
        "the embedded model over {} games at virus level {}",
        seeds, level
    );
    let mut gathered = corpus(network, seeds, level);
    gathered.truncate(pills);
    let played = imitation::play_games(seeds, level, |_: &Bottle, placements: &[Placement]| {
        let features: Vec<BottleFeatures> = placements.iter().map(|p| p.features()).collect();
        let scores = Scorer::Network(network).rank(&features);
        (0..placements.len()).max_by(|a, b| scores[*a].total_cmp(&scores[*b]))
    });
    println!(
        "{} pills of choices, {} viruses, {} bottles\n",
        gathered.len(),
        played.viruses,
        played.bottles
    );

    let spread = spreads(&gathered);
    let mut rows: Vec<Influence> = (0..BOTTLE_FEATURE_INPUTS)
        .map(|input| Influence {
            input,
            weight: weight_column(&network, input),
            spread: spread[input],
            mind_changes: mind_changes(network, &gathered, input),
            viruses_without: plays_without(network, input, seeds, level),
        })
        .collect();
    rows.sort_by(|a, b| b.mind_changes.total_cmp(&a.mind_changes));

    println!(
        "{:<26} {:>7} {:>8} {:>8} {:>9}",
        "input", "weight", "spread", "changes", "viruses"
    );
    println!(
        "{:<26} {:>7} {:>8} {:>8} {:>9}",
        "", "", "", "", played.viruses
    );
    for row in &rows {
        println!(
            "{:<26} {:>7.2} {:>8.3} {:>7.1}% {:>9}",
            INPUTS[row.input].name,
            row.weight,
            row.spread,
            100.0 * row.mind_changes,
            row.viruses_without
        );
    }

    // The pictures are checked here rather than in a test because the crate's own test build
    // swaps the bottle for a mock, so nothing that builds one can run in it.
    println!("\n== the scenarios the shots are drawn from ==");
    let all = scenarios();
    assert_eq!(all.len(), BOTTLE_FEATURE_INPUTS, "an input has no picture");
    let mut flat = vec![];
    for (i, scenario) in all.iter().enumerate() {
        assert_eq!(scenario.input, i, "the pictures are in input order");
        let separates = scenario.separates();
        if !separates {
            flat.push(INPUTS[i].name);
        }
        println!(
            "{:<26} {:>8.1} vs {:>8.1}  {}",
            INPUTS[i].name,
            scenario.value[0],
            scenario.value[1],
            if separates {
                ""
            } else {
                "<- shows the same value twice"
            }
        );
    }
    if !flat.is_empty() {
        println!(
            "\n{} of {} pictures do not separate the input they are about: {}",
            flat.len(),
            all.len(),
            flat.join(", ")
        );
    }
    Ok(())
}

/// every cell of a bottle, for a renderer that has no other way in
pub fn cells(bottle: &Bottle) -> Vec<(u32, u32, crate::game::block::Block)> {
    let mut cells = vec![];
    for y in 0..BOTTLE_HEIGHT {
        for x in 0..BOTTLE_WIDTH {
            let block = bottle.block_at(x, y);
            if !block.is_empty() {
                cells.push((x, y, block));
            }
        }
    }
    cells
}
