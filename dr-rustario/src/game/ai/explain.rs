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
    Input::comparative("delta.virus_work", "how the work its viruses need changed. Work is the fewest blocks a line of four through a cell still needs, counting only lines no other colour blocks and whose gaps a pill could still be dropped into - the cheaper of the two axes."),
    Input::comparative("delta.virus_work_row", "the same along rows only."),
    Input::comparative("delta.virus_work_col", "and down columns only. Both are fed beside the cheaper of them because they are not the same job: finishing a column means building upward."),
    Input::comparative("delta.viruses_buried", "viruses with no line left at all - on neither axis a window of four that another colour does not block or an overhang does not shut. Not a cost that can be paid off later: the virus has to be dug back out."),
    Input::comparative("delta.block_work", "the same work count over everything that is not a virus, which is how much of what the agent has put down is on its way to clearing itself up."),
    Input::comparative("delta.blocks_buried", "blocks with no line left. Dead weight, and the thing a placement is most easily talked into making."),
    Input::comparative("place.halves_work", "the work the better of the two cells the pill just put down still needs. The bottle-wide sums cannot say this - they add up forty-odd blocks, so the two the decision is about are a twentieth of the number."),
    Input::comparative("place.halves_touching", "the longest run of one colour actually *touching* a placed half. Not work upside down: work counts a reachable gap as one block, so two with a hole and three joined are both one short, while this counts only what would clear this instant if it reached four."),
    Input::comparative("place.halves_run_viruses", "viruses already in the line the placed halves are working on, which is what makes building one worth doing rather than tidying a corner."),
    Input::comparative("place.halves_one_short", "how many of the two placed cells are exactly one block from a clear."),
    Input::comparative("place.halves_two_short", "and exactly two. The same measurement as place.halves_work told as an indicator, and worth +763 on the median taught model - neither of the pair is worth anything without the other."),
    Input::comparative("delta.viruses_at_work_1_row", "viruses one block from dying along a row."),
    Input::comparative("delta.viruses_at_work_1_col", "and down a column."),
    Input::comparative("delta.blocks_at_work_1_row", "blocks one from clearing along a row."),
    Input::comparative("delta.blocks_at_work_1_col", "and down a column."),
    Input::comparative("delta.landing_height", "how the *shortest* column moved - the lowest a pill can still be put. This is the height that constrains play where the tallest column only looks like it does: one virus on the floor makes the tallest column 1 and changes nothing, because every other column is still open to the floor. Feeding it is worth +337; feeding the tallest column instead costs 201."),
    Input::context("context.blocks_at_work_1", "how many blocks were already one from clearing before the pill. Context, not a ranking signal: it is the same for every candidate by construction, and it is there to tell the network whether the bottle it is in is one it can finish."),
    Input::context("context.viruses_at_work_1", "how many viruses were already one from dying before the pill."),
    Input::context("context.held", "whether this candidate is a placement of the pill being *held* rather than the one in play. Zero everywhere while hold is off, which is everywhere, and silencing it is measurably a no-op."),
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
