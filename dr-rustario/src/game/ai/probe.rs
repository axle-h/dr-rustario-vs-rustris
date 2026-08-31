//! `ga dr probe`: ask the deterministic ai what it is actually paying for.
//!
//! [`crate::game::ai::features`] is the answer to a question this module asks: a `ga dr auto`
//! run on the features that came before got nowhere, and rather than guess at why, this plays
//! the N64 ai, records what it thought of *every* placement it was offered, and measures how
//! much of that opinion a feature set can express. The features it is pointed at now are the
//! ones it chose; re-run it after changing them and it will say whether they were an
//! improvement.
//!
//! What it asks, in the order the report answers it:
//!
//! 1. **Does an input separate one placement from another?** Scoring only ever ranks the
//!    placements of one pill against each other, so an input whose value hardly moves between
//!    them cannot rank them, however important the thing it measures is. That is the `spread`
//!    column, and it is why two inputs that differ only by a constant per pill are the same
//!    input.
//! 2. **How much of the N64's ranking can a feature set reproduce?** Fit the best scorer the
//!    inputs allow - a straight line, and then the same network shape a `ga dr` run trains -
//!    and see how often it picks what the N64 picked. That is a ceiling on what any model of
//!    those inputs can do.
//! 3. **Does the clone play?** Agreeing with the N64 is a means; the end is playing like it, so
//!    every fitted scorer is sent out to play whole games and counted on viruses and bottles.
//! 4. **Which of the N64's own terms matter?** Take one out, replay the same pills, count how
//!    often it changes its mind. And the same for the inputs: clone the network again without
//!    a group of them and see what the clone loses.
//!
//! Everything is measured on pills held back from every fit, and the report is the whole of the
//! output: nothing here is used by the game.

use crate::game::ai::evaluator::{self, Scorer, COMPARATIVE};
use crate::game::ai::features::{BottleAnalysis, BottleFeatures, Grid, TOP_ROW_RATE};
use crate::game::ai::imitation;
use crate::game::ai::models;
use crate::game::ai::models::DR_NEURAL_GENOME_SIZE;
use crate::game::ai::n64::{N64Ai, Params, Situation};
use crate::game::ai::placement::{Placement, PlacementSearch};
use crate::game::bottle::{Bottle, BOTTLE_HEIGHT, BOTTLE_WIDTH};
use crate::game::random::{viruses_at_level, GameRandom, RandomMode};
use crate::game::{Game, GameSpeed};
use engine::ai::{NeuralNetwork, Seed, Tensor, BOTTLE_FEATURE_INPUTS};
use engine::game::{Game as _, GameEvent};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaChaRng;
use std::time::Duration;

const STEP: Duration = Duration::from_millis(16);
/// pills without a virus destroyed before a game is called off as going nowhere
const STALL_PILLS: u32 = 200;

/// what the N64 charges for leaving a block high up, by column. Its `bad_point`.
const DEAD_POINT: [f64; 8] = [90., 270., 360., 900., 900., 360., 270., 90.];

// ---------------------------------------------------------------------------------------
// the corpus
// ---------------------------------------------------------------------------------------

/// one pill, and what the ai made of every placement it had
struct Decision {
    /// by candidate: every feature, already centred on the mean over the candidates of this
    /// pill, since only what separates one placement from another can rank them
    rows: Vec<Vec<f64>>,
    /// the N64's priority for each candidate, centred and scaled to unit spread
    target: Vec<f64>,
    /// which candidate the N64 played
    chosen: usize,
    /// by candidate, whether it was worth as much as the one the N64 played: a tie is settled
    /// by the original's own ordering, so any of them would have done
    top: Vec<bool>,
    situation: Situation,
    /// held back from every fit, so a scorer is only ever measured on pills it was not fitted to
    test: bool,
    /// what the embedded network and the hand written scorer made of this pill, before any
    /// centring: the mean over its placements and how far they spread out around it
    seen: [(f64, f64); 2],
}

struct Corpus {
    names: Vec<String>,
    decisions: Vec<Decision>,
    /// viruses the ai destroyed while the corpus was gathered
    killed: usize,
    /// bottles it finished, and games it lost, which is the sanity check that the probe is
    /// watching the ai play as it really plays
    stages: u32,
    games_over: u32,
    /// how often taking one of the ai's terms out changes its mind
    ablations: Vec<(&'static str, usize)>,
    pills: u64,
    games: u32,
}

/// every term of the N64 scorer that can be taken out through its weights, and how
fn ablations() -> Vec<(&'static str, fn(&mut Params))> {
    vec![
        ("chain (rensa_p + rensa_mp)", |p| {
            p.rensa_p = 0;
            p.rensa_mp = 0;
        }),
        ("chain for a colour not in this pill (rensa_mp)", |p| {
            p.rensa_mp = 0
        }),
        ("weight taken off the top rows (pri_point[7])", |p| {
            p.pri_point[7] = 0
        }),
        ("covering a virus (on_virus_p)", |p| p.on_virus_p = 0),
        ("stranding a half (alone_cap_p/wp, l_pri_p)", |p| {
            p.alone_cap_p = [0; 6];
            p.alone_cap_wp = [0; 6];
            p.l_pri_p = 0;
        }),
        ("a column run that has not gone yet (hei_lines_allp)", |p| {
            p.hei_lines_allp = [0; 9]
        }),
        ("a row run that has not gone yet (wid_lines_allp)", |p| {
            p.wid_lines_allp = [0; 9]
        }),
        ("either run that has not gone yet", |p| {
            p.hei_lines_allp = [0; 9];
            p.wid_lines_allp = [0; 9];
        }),
        ("viruses a run could still take (pri_point[4])", |p| {
            p.pri_point[4] = 0
        }),
        ("viruses the clear took (pri_point[1])", |p| {
            p.pri_point[1] = 0
        }),
        ("clearing a line at all (erase_lin_p)", |p| {
            p.erase_lin_p = [0; 9]
        }),
        ("playing away from the stacked side (wall)", |p| {
            p.wall = false
        }),
    ]
}

fn collect(seeds: u128, level: u32, decision_cap: usize, ablate: bool) -> Corpus {
    let ai = N64Ai::new();
    let variants = ablations();
    let mut corpus = Corpus {
        names: names(),
        decisions: vec![],
        killed: 0,
        stages: 0,
        games_over: 0,
        ablations: variants.iter().map(|(name, _)| (*name, 0)).collect(),
        pills: 0,
        games: 0,
    };

    for seed in 0..seeds {
        if corpus.decisions.len() >= decision_cap {
            break;
        }
        let random = GameRandom::from_seed(Seed::from(seed + 1).into(), RandomMode::Bag);
        let Ok(mut game) = Game::new(level, GameSpeed::Medium, random) else {
            continue;
        };
        corpus.games += 1;

        let mut stalled = 0;
        let mut game_over = false;
        let mut viruses = game.bottle().virus_count();
        while !game_over && stalled < STALL_PILLS && corpus.decisions.len() < decision_cap {
            // the clear animation holds the bottle up for a few frames, so what a pill killed
            // only shows up some way after it was played
            let now = game.bottle().virus_count();
            if now < viruses {
                corpus.killed += (viruses - now) as usize;
                stalled = 0;
            }
            viruses = now;

            if game.bottle().pill().is_none() {
                game.update(STEP);
                for event in game.drain_events() {
                    match event {
                        GameEvent::GameOver => game_over = true,
                        GameEvent::StageComplete => {
                            stalled = 0;
                            if game.next_stage().is_err() {
                                game_over = true;
                            }
                            viruses = 0;
                        }
                        _ => (),
                    }
                }
                continue;
            }

            let before = game.bottle().stats();
            let placements = game.bottle().placements(before);
            let reading = ai.read(game.bottle(), &placements);
            let Some(chosen) = reading.chosen else {
                break;
            };

            if let Some(mut decision) = decision(&placements, &reading, chosen) {
                // every fifth pill is held back, so nothing is ever measured on a pill it was
                // fitted to
                decision.test = corpus.decisions.len() % 5 == 4;
                corpus.decisions.push(decision);
            }
            if ablate {
                for ((_, count), (_, tweak)) in corpus.ablations.iter_mut().zip(variants.iter()) {
                    let without = ai.read_tweaked(game.bottle(), &placements, tweak);
                    if without.chosen != Some(chosen) {
                        *count += 1;
                    }
                }
            }

            imitation::press(&mut game, &placements[chosen]);
            corpus.pills += 1;
            stalled += 1;

            let mut events = game.drain_events();
            game.update(STEP);
            events.extend(game.drain_events());
            for event in events {
                match event {
                    GameEvent::GameOver => game_over = true,
                    GameEvent::StageComplete => {
                        stalled = 0;
                        if game.next_stage().is_err() {
                            game_over = true;
                        }
                        viruses = 0;
                    }
                    _ => (),
                }
            }
        }
        corpus.games_over += game_over as u32;
        corpus.stages += game.completed_stages();
    }
    corpus
}

/// Turn one pill into a row per candidate, centred on the mean over the candidates: an input
/// that is the same for every placement of a pill cannot choose between them, whatever it is
/// measuring, and centring is what makes that visible.
fn decision(
    placements: &[Placement],
    reading: &crate::game::ai::n64::Reading,
    chosen: usize,
) -> Option<Decision> {
    let scored: Vec<usize> = (0..placements.len())
        .filter(|i| reading.priorities[*i].is_some())
        .collect();
    if scored.len() < 2 {
        return None;
    }

    let scored_placements: Vec<&Placement> = scored.iter().map(|i| &placements[*i]).collect();
    let mut rows: Vec<Vec<f64>> = rows(&scored_placements);
    let mut target: Vec<f64> = scored
        .iter()
        .map(|i| reading.priorities[*i].unwrap() as f64)
        .collect();
    let best = target.iter().copied().fold(f64::MIN, f64::max);
    let top: Vec<bool> = target.iter().map(|value| *value == best).collect();
    let chosen = scored.iter().position(|i| *i == chosen)?;

    // what a scorer of the kind being trained today actually sees when it looks at this pill:
    // one number per placement, and how far apart those numbers are
    let seen = [Scorer::Network(models::survival_trained()), Scorer::Linear].map(|scorer| {
        let features: Vec<BottleFeatures> =
            scored_placements.iter().map(|p| p.features()).collect();
        let scores = scorer.rank(&features);
        let mean = scores.iter().sum::<f64>() / scores.len() as f64;
        let variance = scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / scores.len() as f64;
        (mean, variance.sqrt())
    });

    // The inputs the model is fed have already been centred on this pill's candidates by
    // `evaluator::inputs`; the columns after them have not, so they are centred here. The
    // context block is left alone deliberately - centring it would zero it, which is exactly
    // what it means for it to be unable to rank anything on its own.
    let width = rows[0].len();
    for column in 0..width {
        if (COMPARATIVE..NOW).contains(&column) {
            continue;
        }
        let mean = rows.iter().map(|row| row[column]).sum::<f64>() / rows.len() as f64;
        for row in rows.iter_mut() {
            row[column] -= mean;
        }
    }

    // the N64's priorities are on a wildly different scale from one pill to the next - the
    // weights change with the situation and the whole thing is multiplied through when the
    // bottle is lopsided - so every pill is brought to the same spread before they are pooled
    let mean = target.iter().sum::<f64>() / target.len() as f64;
    let variance = target.iter().map(|t| (t - mean).powi(2)).sum::<f64>() / target.len() as f64;
    let deviation = variance.sqrt();
    if deviation < 1e-9 {
        // every placement is worth the same, so there is nothing here to learn from
        return None;
    }
    for value in target.iter_mut() {
        // the N64's own numbers have enormous outliers in them - a chain is worth thousands, a
        // block left in the neck of the bottle costs nine - and a least squares fit chasing
        // those would tell us nothing about how it ranks the placements it actually chooses
        // between, so every pill is clipped to three deviations
        *value = ((*value - mean) / deviation).clamp(-3.0, 3.0);
    }

    Some(Decision {
        rows,
        target,
        chosen,
        top,
        situation: reading.situation,
        test: false,
        seen,
    })
}

// ---------------------------------------------------------------------------------------
// the features
// ---------------------------------------------------------------------------------------

/// the names of every column, in the order [`rows`] builds them
fn names() -> Vec<String> {
    let mut names: Vec<String> = NOW_NAMES.iter().map(|n| format!("now.{}", n)).collect();
    assert_eq!(
        names.len(),
        NOW,
        "the input names are out of step with the network"
    );
    names.extend(EXTRA.iter().map(|name| format!("new.{}", name)));
    names.extend(REFERENCE.iter().map(|name| format!("ref.{}", name)));
    names
}

/// how many inputs the model is fed today, and what they are called
const NOW: usize = BOTTLE_FEATURE_INPUTS;

#[rustfmt::skip]
const NOW_NAMES: [&str; BOTTLE_FEATURE_INPUTS] = [
    "delta.viruses", "delta.virus_work", "delta.buried_viruses", "delta.buried_blocks",
    "delta.max_height", "delta.entrance_height", "delta.holes",
    "delta.virus_3_row", "delta.virus_3_col", "delta.virus_2_row", "delta.virus_2_col",
    "delta.block_3_row", "delta.block_3_col",
    "place.patterns_cleared", "place.touching", "place.reach", "place.open_3", "place.open_2",
    "place.run_viruses", "place.stranded", "place.stranded_on_virus", "place.covers_virus",
    "place.buries_virus", "place.one_away", "place.one_away_virus", "place.chains",
    "context.viruses", "context.virus_work", "context.max_height", "context.entrance_height",
    "context.holes", "context.held",
];

/// What is measured here and left out of the model, which is the control on the feature set:
/// if a clone fed these as well plays better, the model is missing something.
///
/// It used to hold the entrance height too, on the grounds that the group as a whole added
/// nothing. That was measured again and was no longer true - a clone fed the group played 508
/// viruses against 271 without it - and of the seven, the entrance height was far the strongest
/// on its own. It is a feature now, and with it fed this group is worth *less* than nothing:
/// the same comparison run again gives 1810 viruses against 2082 without them. That is what
/// these six are here to keep saying.
const EXTRA: [&str; 6] = [
    "place_top_weight",
    "place_dead_weight",
    "place_height",
    "top_weight",
    "bumpiness",
    "imbalance",
];

/// scorers to measure the fitted ones against: the hand written baseline the model is supposed
/// to beat, and the simplest policy there is
const REFERENCE: [&str; 2] = ["hand written linear", "viruses this placement killed"];

/// One row per placement: what the network is fed for this pill - already centred on the
/// candidates and scaled, exactly as it is at play time - then the inputs left out of it, then
/// the reference scorers.
fn rows(placements: &[&Placement]) -> Vec<Vec<f64>> {
    let features: Vec<BottleFeatures> = placements.iter().map(|p| p.features()).collect();
    let fed = evaluator::inputs(&features);
    let linear = Scorer::Linear.rank(&features);

    fed.into_iter()
        .zip(placements.iter())
        .zip(linear)
        .map(|((row, placement), linear)| {
            let mut values = row.to_vec();
            values.extend(left_out(placement));
            values.push(linear);
            values.push(-placement.features().delta().viruses() as f64);
            values
        })
        .collect()
}

/// [`EXTRA`], measured: what the placement put in the neck of the bottle, and the shape of the
/// stack it left behind.
fn left_out(placement: &Placement) -> [f64; EXTRA.len()] {
    let grid = Grid::of(placement.settled());

    let mut top_weight = 0.0;
    let mut dead_weight = 0.0;
    let mut height: f64 = 0.0;
    for (point, _) in placement.landing().iter() {
        let (x, y) = (point.x(), point.y());
        if x < 0 || y < 0 || x >= BOTTLE_WIDTH as i32 || y >= BOTTLE_HEIGHT as i32 {
            continue;
        }
        let (x, y) = (x as usize, y as usize);
        height = height.max((BOTTLE_HEIGHT as usize - y) as f64);
        if y < TOP_ROW_RATE.len() {
            top_weight += TOP_ROW_RATE[y][x] as f64;
            dead_weight += DEAD_POINT[x] / (y as f64 * 2.0 + 1.0);
        }
    }

    let heights = grid.heights();
    let stack_weight: f64 = TOP_ROW_RATE
        .iter()
        .enumerate()
        .map(|(y, rates)| {
            rates
                .iter()
                .enumerate()
                .filter(|(x, _)| grid.colour(*x as u32, y as u32).is_some())
                .map(|(_, rate)| *rate as f64)
                .sum::<f64>()
        })
        .sum();
    let bumpiness: f64 = heights
        .windows(2)
        .map(|pair| (pair[0] - pair[1]).abs() as f64)
        .sum();
    let imbalance =
        (heights[..4].iter().sum::<i32>() - heights[4..].iter().sum::<i32>()).abs() as f64;

    [
        top_weight,
        dead_weight,
        height,
        stack_weight,
        bumpiness,
        imbalance,
    ]
}

// ---------------------------------------------------------------------------------------
// fitting a linear scorer, which is the ceiling a feature set puts on any model of it
// ---------------------------------------------------------------------------------------

/// the sums a least squares fit needs, over every candidate of every pill, gathered once for
/// every feature so that a fit over any subset of them is only a small solve
struct Gram {
    /// `xx[i][j]` is the sum of feature i times feature j
    xx: Vec<Vec<f64>>,
    /// `xy[i]` is the sum of feature i times the priority
    xy: Vec<f64>,
    /// the standard deviation of each feature over the whole corpus, used to standardise it
    deviation: Vec<f64>,
    /// the mean of each feature's spread within one pill: a feature with none cannot rank
    spread: Vec<f64>,
}

impl Gram {
    fn of(corpus: &Corpus) -> Self {
        Self::over(corpus, None)
    }

    /// the same, over the pills the ai read as one situation, so a scorer can be fitted for
    /// that situation alone and asked whether it does better than the one fitted over all of it
    fn of_situation(corpus: &Corpus, situation: Situation) -> Self {
        Self::over(corpus, Some(situation))
    }

    fn over(corpus: &Corpus, only: Option<Situation>) -> Self {
        let width = corpus.names.len();
        let mut sum = vec![0.0; width];
        let mut squares = vec![0.0; width];
        let mut spread = vec![0.0; width];
        let mut rows = 0.0;

        for decision in corpus.decisions.iter().filter(|d| !d.test) {
            if only.is_some_and(|situation| situation != decision.situation) {
                continue;
            }
            for column in 0..width {
                let values: Vec<f64> = decision.rows.iter().map(|row| row[column]).collect();
                // the rows are already centred on the pill, so this is the spread within it
                let variance = values.iter().map(|v| v * v).sum::<f64>() / values.len() as f64;
                spread[column] += variance.sqrt();
                for value in values {
                    sum[column] += value;
                    squares[column] += value * value;
                }
            }
            rows += decision.rows.len() as f64;
        }

        let deviation: Vec<f64> = (0..width)
            .map(|i| {
                let mean = sum[i] / rows;
                ((squares[i] / rows - mean * mean).max(0.0))
                    .sqrt()
                    .max(1e-9)
            })
            .collect();
        let pills = corpus
            .decisions
            .iter()
            .filter(|d| !d.test && only.is_none_or(|situation| situation == d.situation))
            .count()
            .max(1);
        let spread: Vec<f64> = spread.iter().map(|total| total / pills as f64).collect();

        let mut xx = vec![vec![0.0; width]; width];
        let mut xy = vec![0.0; width];
        for decision in corpus.decisions.iter().filter(|d| !d.test) {
            if only.is_some_and(|situation| situation != decision.situation) {
                continue;
            }
            for (row, target) in decision.rows.iter().zip(decision.target.iter()) {
                let scaled: Vec<f64> = (0..width).map(|i| row[i] / deviation[i]).collect();
                for i in 0..width {
                    if scaled[i] == 0.0 {
                        continue;
                    }
                    for j in 0..width {
                        xx[i][j] += scaled[i] * scaled[j];
                    }
                    xy[i] += scaled[i] * target;
                }
            }
        }

        Self {
            xx,
            xy,
            deviation,
            spread,
        }
    }

    /// the best linear scorer over `columns`, in standardised units so the weights compare
    fn fit(&self, columns: &[usize]) -> Vec<f64> {
        let n = columns.len();
        let ridge =
            1e-4 * columns.iter().map(|c| self.xx[*c][*c]).sum::<f64>() / columns.len() as f64;
        let mut a = vec![vec![0.0; n + 1]; n];
        for (i, ci) in columns.iter().enumerate() {
            for (j, cj) in columns.iter().enumerate() {
                a[i][j] = self.xx[*ci][*cj];
            }
            a[i][i] += ridge;
            a[i][n] = self.xy[*ci];
        }
        solve(a)
    }
}

/// Gaussian elimination with partial pivoting, on an augmented matrix
fn solve(mut a: Vec<Vec<f64>>) -> Vec<f64> {
    let n = a.len();
    for i in 0..n {
        let pivot = (i..n)
            .max_by(|x, y| a[*x][i].abs().total_cmp(&a[*y][i].abs()))
            .unwrap();
        a.swap(i, pivot);
        if a[i][i].abs() < 1e-12 {
            continue;
        }
        for row in i + 1..n {
            let factor = a[row][i] / a[i][i];
            for column in i..=n {
                a[row][column] -= factor * a[i][column];
            }
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        if a[i][i].abs() < 1e-12 {
            continue;
        }
        let mut value = a[i][n];
        for j in i + 1..n {
            value -= a[i][j] * x[j];
        }
        x[i] = value / a[i][i];
    }
    x
}

/// how well a scorer reproduces the N64: how often it picks the same placement, how often it
/// orders a pair of them the same way, and how much of the priority it accounts for
struct Fit {
    agreement: f64,
    pairs: f64,
    r2: f64,
    /// where the n64's own pick lands in this scorer's ordering, 100% being top of the list
    percentile: f64,
}

fn evaluate(
    corpus: &Corpus,
    gram: &Gram,
    columns: &[usize],
    weights: &[f64],
    only: Option<Situation>,
) -> Fit {
    let mut agreed: f64 = 0.0;
    let mut decisions: f64 = 0.0;
    let mut ordered: f64 = 0.0;
    let mut pairs: f64 = 0.0;
    let mut residual: f64 = 0.0;
    let mut total: f64 = 0.0;
    let mut percentile: f64 = 0.0;

    for decision in corpus.decisions.iter().filter(|d| d.test) {
        if only.is_some_and(|situation| situation != decision.situation) {
            continue;
        }
        let scores: Vec<f64> = decision
            .rows
            .iter()
            .map(|row| {
                columns
                    .iter()
                    .zip(weights.iter())
                    .map(|(c, w)| row[*c] / gram.deviation[*c] * w)
                    .sum()
            })
            .collect();

        let best = (0..scores.len())
            .max_by(|a, b| scores[*a].total_cmp(&scores[*b]))
            .unwrap();
        decisions += 1.0;
        if decision.top[best] {
            agreed += 1.0;
        }
        let beaten = scores
            .iter()
            .filter(|score| **score < scores[decision.chosen])
            .count() as f64;
        percentile += beaten / (scores.len() - 1).max(1) as f64;
        for i in 0..scores.len() {
            residual += (scores[i] - decision.target[i]).powi(2);
            total += decision.target[i].powi(2);
            for j in i + 1..scores.len() {
                let ours = scores[i] - scores[j];
                let theirs = decision.target[i] - decision.target[j];
                if theirs != 0.0 {
                    pairs += 1.0;
                    if ours * theirs > 0.0 {
                        ordered += 1.0;
                    }
                }
            }
        }
    }

    Fit {
        agreement: agreed / decisions.max(1.0),
        pairs: ordered / pairs.max(1.0),
        r2: 1.0 - residual / total.max(1e-9),
        percentile: percentile / decisions.max(1.0),
    }
}

/// how often you would pick the N64's placement by chance
fn chance(corpus: &Corpus) -> f64 {
    let held: Vec<&Decision> = corpus.decisions.iter().filter(|d| d.test).collect();
    held.iter()
        .map(|d| d.top.iter().filter(|top| **top).count() as f64 / d.rows.len() as f64)
        .sum::<f64>()
        / held.len().max(1) as f64
}

/// How a scorer gets on when it has to play rather than only agree: the point of imitating
/// the N64 is to play like it, and agreeing with it on nine pills in ten would still be worth
/// nothing if the tenth is the one that buries you.
struct Played {
    viruses: u32,
    bottles: u32,
    pills: u32,
    buried: u32,
}

impl Played {
    fn line(&self, name: &str) -> String {
        format!(
            "{:<34} {:>8} {:>9} {:>8} {:>10.2}",
            name,
            self.viruses,
            self.bottles,
            self.buried,
            self.viruses as f64 / self.pills.max(1) as f64
        )
    }
}

/// Play `games` whole games from the first bottle, choosing with `choose`.
fn play(
    seeds: u128,
    level: u32,
    choose: impl Fn(&Bottle, &[Placement]) -> Option<usize>,
) -> Played {
    let mut played = Played {
        viruses: 0,
        bottles: 0,
        pills: 0,
        buried: 0,
    };

    for seed in 0..seeds {
        let random = GameRandom::from_seed(Seed::from(seed + 1).into(), RandomMode::Bag);
        let Ok(mut game) = Game::new(level, GameSpeed::Medium, random) else {
            continue;
        };
        let mut stalled = 0;
        let mut game_over = false;
        let mut viruses = game.bottle().virus_count();

        while !game_over && stalled < STALL_PILLS {
            let now = game.bottle().virus_count();
            if now < viruses {
                played.viruses += viruses - now;
                stalled = 0;
            }
            viruses = now;

            if game.bottle().pill().is_none() {
                game.update(STEP);
                for event in game.drain_events() {
                    match event {
                        GameEvent::GameOver => game_over = true,
                        GameEvent::StageComplete => {
                            stalled = 0;
                            viruses = 0;
                            played.bottles += 1;
                            if game.next_stage().is_err() {
                                game_over = true;
                            }
                        }
                        _ => (),
                    }
                }
                continue;
            }

            let before = game.bottle().stats();
            let placements = game.bottle().placements(before);
            let Some(chosen) = choose(game.bottle(), &placements) else {
                break;
            };
            imitation::press(&mut game, &placements[chosen]);
            played.pills += 1;
            stalled += 1;

            let mut events = game.drain_events();
            game.update(STEP);
            events.extend(game.drain_events());
            for event in events {
                match event {
                    GameEvent::GameOver => game_over = true,
                    GameEvent::StageComplete => {
                        stalled = 0;
                        viruses = 0;
                        played.bottles += 1;
                        if game.next_stage().is_err() {
                            game_over = true;
                        }
                    }
                    _ => (),
                }
            }
        }
        played.buried += game_over as u32;
    }
    played
}

/// the placement a set of fitted weights likes best, over the probe's own columns
fn choose_by_weights(
    placements: &[Placement],
    columns: &[usize],
    deviation: &[f64],
    weights: &[f64],
) -> Option<usize> {
    let all: Vec<&Placement> = placements.iter().collect();
    rows(&all)
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            let score: f64 = columns
                .iter()
                .zip(weights.iter())
                .map(|(c, w)| row[*c] / deviation[*c] * w)
                .sum();
            (index, score)
        })
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(index, _)| index)
}

/// Every input there is, which is the widest network the probe trains.
const ALL_INPUTS: usize = NOW + EXTRA.len();

/// Train a network of the shape the model is trained in - two hidden layers as wide as its
/// input - to reproduce the N64's opinion of a placement, and report how well it does on the
/// pills it never saw. A linear fit says whether the features carry the signal *linearly*;
/// this says whether they carry it at all, and it is the same architecture a `ga dr` run has
/// to find by mutation, so it is a fair ceiling on that run's best case.
fn imitate<const IN: usize>(
    corpus: &Corpus,
    columns: &[usize],
    gram: &Gram,
    epochs: usize,
    seed: u64,
) -> (Fit, NeuralNetwork<IN, 2, 1, IN>) {
    assert_eq!(IN, columns.len());
    // where a network starts moves the answer about by a good deal, so every clone in the
    // report starts from a named place and the ones that are compared are averaged over
    // several of them
    let mut rng = ChaChaRng::seed_from_u64(seed);
    let weights: Vec<f64> = (0..NeuralNetwork::<IN, 2, 1, IN>::TOTAL_SIZE)
        .map(|_| rng.random::<f64>() * 2.0 - 1.0)
        .collect();
    let mut network = NeuralNetwork::<IN, 2, 1, IN>::new(&weights);

    let row_of = |row: &[f64]| {
        let mut input = [0.0; IN];
        for (at, column) in columns.iter().enumerate() {
            input[at] = row[*column] / gram.deviation[*column];
        }
        Tensor::vector(input)
    };

    for epoch in 0..epochs {
        let rate = 0.05 / (1.0 + 0.6 * epoch as f64);
        for decision in corpus.decisions.iter().filter(|d| !d.test) {
            for (row, target) in decision.rows.iter().zip(decision.target.iter()) {
                network.train_step(&row_of(row), &Tensor::vector([*target]), rate);
            }
        }
    }

    let mut agreed: f64 = 0.0;
    let mut pills: f64 = 0.0;
    let mut ordered: f64 = 0.0;
    let mut pairs: f64 = 0.0;
    let mut percentile: f64 = 0.0;
    let mut residual: f64 = 0.0;
    let mut total: f64 = 0.0;
    for decision in corpus.decisions.iter().filter(|d| d.test) {
        let scores: Vec<f64> = decision
            .rows
            .iter()
            .map(|row| network.forward(&row_of(row)).value())
            .collect();
        let best = (0..scores.len())
            .max_by(|a, b| scores[*a].total_cmp(&scores[*b]))
            .unwrap();
        pills += 1.0;
        if decision.top[best] {
            agreed += 1.0;
        }
        percentile += scores
            .iter()
            .filter(|score| **score < scores[decision.chosen])
            .count() as f64
            / (scores.len() - 1).max(1) as f64;
        for i in 0..scores.len() {
            residual += (scores[i] - decision.target[i]).powi(2);
            total += decision.target[i].powi(2);
            for j in i + 1..scores.len() {
                let theirs = decision.target[i] - decision.target[j];
                if theirs != 0.0 {
                    pairs += 1.0;
                    if (scores[i] - scores[j]) * theirs > 0.0 {
                        ordered += 1.0;
                    }
                }
            }
        }
    }
    (
        Fit {
            agreement: agreed / pills.max(1.0),
            pairs: ordered / pairs.max(1.0),
            r2: 1.0 - residual / total.max(1e-9),
            percentile: percentile / pills.max(1.0),
        },
        network,
    )
}

/// Clone the N64 onto `columns` and then send the clone out to play, which is the only test
/// of a feature set that counts: agreeing with the N64 is a means, and playing like it is the
/// end.
fn clone_and_play<const IN: usize>(
    corpus: &Corpus,
    columns: &[usize],
    gram: &Gram,
    epochs: usize,
    games: u128,
    level: u32,
    clones: u64,
) -> (Fit, Played) {
    let mut fit = Fit {
        agreement: 0.0,
        pairs: 0.0,
        r2: 0.0,
        percentile: 0.0,
    };
    let mut played = Played {
        viruses: 0,
        bottles: 0,
        pills: 0,
        buried: 0,
    };
    for clone in 0..clones {
        let (one, network) = imitate::<IN>(corpus, columns, gram, epochs, clone);
        fit.agreement += one.agreement / clones as f64;
        fit.pairs += one.pairs / clones as f64;
        fit.r2 += one.r2 / clones as f64;
        fit.percentile += one.percentile / clones as f64;
        let this = play(games, level, |_bottle, placements| {
            choose_by_network(placements, columns, &gram.deviation, &network)
        });
        played.viruses += this.viruses;
        played.bottles += this.bottles;
        played.pills += this.pills;
        played.buried += this.buried;
    }
    played.viruses /= clones as u32;
    played.bottles /= clones as u32;
    played.pills /= clones as u32;
    played.buried /= clones as u32;
    (fit, played)
}

/// the placement a cloned network likes best, over the probe's own columns
fn choose_by_network<const IN: usize>(
    placements: &[Placement],
    columns: &[usize],
    deviation: &[f64],
    network: &NeuralNetwork<IN, 2, 1, IN>,
) -> Option<usize> {
    let all: Vec<&Placement> = placements.iter().collect();
    let built = rows(&all);
    if built.is_empty() {
        return None;
    }
    // the columns the model is fed arrive centred on the pill's candidates already; the rest
    // are centred here, exactly as the corpus was, so the clone sees what it was trained on
    let means: Vec<f64> = columns
        .iter()
        .map(|c| {
            if (COMPARATIVE..NOW).contains(c) {
                0.0
            } else {
                built.iter().map(|row| row[*c]).sum::<f64>() / built.len() as f64
            }
        })
        .collect();

    built
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let mut input = [0.0; IN];
            for (at, column) in columns.iter().enumerate() {
                input[at] = (row[*column] - means[at]) / deviation[*column];
            }
            (index, network.forward(&Tensor::vector(input)).value())
        })
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(index, _)| index)
}

// ---------------------------------------------------------------------------------------
// the report
// ---------------------------------------------------------------------------------------

/// `args` are the arguments after `ga dr probe`:
/// `[seeds] [virus level] [decisions] [skip ablation]`
pub fn probe_main(args: &[String]) -> Result<(), String> {
    let seeds: u128 = args.first().and_then(|s| s.parse().ok()).unwrap_or(4);
    let level: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
    let decisions: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1500);
    let ablate = args.get(3).map(String::as_str) != Some("no-ablation");
    /// whole games each fitted scorer is asked to play
    const GAMES: u128 = 5;
    /// passes over the corpus a cloned network is trained for
    const EPOCHS: usize = 10;
    /// random genomes played to see what the first generation of a training run looks like
    const GENOMES: usize = 24;
    /// how many clones of each feature set are trained and played, since where a network starts
    /// moves the answer about
    const CLONES: u64 = 3;
    let games = GAMES;

    println!(
        "playing the n64 ai over {} seeds at virus level {}, recording up to {} pills",
        seeds, level, decisions
    );
    let corpus = collect(seeds, level, decisions, ablate);
    if corpus.decisions.len() < 50 {
        return Err(format!(
            "only {} usable pills, not enough to measure anything",
            corpus.decisions.len()
        ));
    }
    let gram = Gram::of(&corpus);
    let width = corpus.names.len();
    let scorers = width - REFERENCE.len();
    let now: Vec<usize> = (0..NOW).collect();
    let new: Vec<usize> = (NOW..scorers).collect();
    let all: Vec<usize> = (0..scorers).collect();

    println!(
        "\n{} pills over {} games ({} of them lost, {} bottles finished), {:.1} placements \
         each, {} viruses killed",
        corpus.decisions.len(),
        corpus.games,
        corpus.games_over,
        corpus.stages,
        corpus
            .decisions
            .iter()
            .map(|d| d.rows.len() as f64)
            .sum::<f64>()
            / corpus.decisions.len() as f64,
        corpus.killed
    );

    let mut situations: Vec<(Situation, usize)> = vec![];
    for decision in &corpus.decisions {
        match situations
            .iter_mut()
            .find(|(s, _)| *s == decision.situation)
        {
            Some((_, count)) => *count += 1,
            None => situations.push((decision.situation, 1)),
        }
    }
    situations.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    print!("situations:");
    for (situation, count) in &situations {
        print!(
            " {:?} {:.0}%",
            situation,
            100.0 * *count as f64 / corpus.decisions.len() as f64
        );
    }
    println!();

    // ---- inputs that cannot rank anything, because they never differ between candidates
    println!("\n== inputs that say the same thing ==");
    println!("two inputs that move together over the placements of a pill rank them the same,");
    println!("however different the things they measure are");
    let mut duplicated = vec![];
    for i in 0..scorers {
        for j in 0..i {
            let correlation = gram.xx[i][j] / (gram.xx[i][i] * gram.xx[j][j]).sqrt().max(1e-12);
            if correlation.abs() > 0.999 {
                println!(
                    "{:<32} is {:<32} ({:+.3})",
                    corpus.names[i], corpus.names[j], correlation
                );
                duplicated.push(i);
                break;
            }
        }
    }
    let distinct: Vec<usize> = all
        .iter()
        .copied()
        .filter(|c| !duplicated.contains(c))
        .collect();

    // ---- what each feature is worth on its own
    println!("\n== every feature on its own ==");
    println!("spread is how far it moves between the placements of one pill: an input with");
    println!("none of that cannot rank them. agreement is how often that feature alone picks");
    println!(
        "what the n64 picked, chance being {:.0}%; percentile is where the n64's own pick",
        100.0 * chance(&corpus)
    );
    println!("lands in the ordering it gives.");
    println!(
        "\n{:<32} {:>8} {:>10} {:>11} {:>8}",
        "feature", "spread", "agreement", "percentile", "weight"
    );
    let full = gram.fit(&distinct);
    let mut rows: Vec<(f64, String)> = vec![];
    for column in 0..scorers {
        let weights = gram.fit(&[column]);
        let fit = evaluate(&corpus, &gram, &[column], &weights, None);
        let weight = distinct
            .iter()
            .position(|c| *c == column)
            .map(|at| format!("{:+.2}", full[at]))
            .unwrap_or_else(|| "-".into());
        rows.push((
            fit.agreement,
            format!(
                "{:<32} {:>8.3} {:>9.1}% {:>10.1}% {:>8}",
                corpus.names[column],
                gram.spread[column],
                100.0 * fit.agreement,
                100.0 * fit.percentile,
                weight
            ),
        ));
    }
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (_, line) in &rows {
        println!("{}", line);
    }

    // ---- what a whole feature set can reach
    println!("\n== the ceiling on a feature set ==");
    println!("the best linear scorer over those inputs, against the n64's own choices");
    println!(
        "\n{:<34} {:>10} {:>8} {:>11} {:>7}",
        "features", "agreement", "pairs", "percentile", "r2"
    );
    let report = |name: &str, columns: &[usize], weights: &[f64]| {
        let fit = evaluate(&corpus, &gram, columns, weights, None);
        println!(
            "{:<34} {:>9.1}% {:>7.1}% {:>10.1}% {:>7.2}",
            name,
            100.0 * fit.agreement,
            100.0 * fit.pairs,
            100.0 * fit.percentile,
            fit.r2
        );
    };
    for (name, columns) in [
        ("what the model is fed today", &now),
        ("what it is missing", &new),
        ("both", &all),
    ] {
        report(name, columns, &gram.fit(columns));
    }
    for (offset, name) in REFERENCE.iter().enumerate() {
        report(name, &[scorers + offset], &[1.0]);
    }
    println!(
        "{:<34} {:>9.1}% {:>7.1}% {:>10.1}% {:>7.2}",
        "chance",
        100.0 * chance(&corpus),
        50.0,
        50.0,
        0.0
    );

    // ---- and what the same features reach with a network rather than a straight line
    println!("\n== the same, with a network rather than a straight line ==");
    println!("two hidden layers as wide as the input, trained by gradient descent to reproduce");
    println!("the n64's own priorities: a linear fit only says whether the signal is there in a");
    println!("straight line, and this says whether it is there at all");
    println!(
        "\n{:<34} {:>10} {:>8} {:>11} {:>7}",
        "features", "agreement", "pairs", "percentile", "r2"
    );
    let report_fit = |name: &str, fit: Fit| {
        println!(
            "{:<34} {:>9.1}% {:>7.1}% {:>10.1}% {:>7.2}",
            name,
            100.0 * fit.agreement,
            100.0 * fit.pairs,
            100.0 * fit.percentile,
            fit.r2
        );
    };
    let (fit, _) = imitate::<NOW>(&corpus, &now, &gram, EPOCHS, 0);
    report_fit("what the model is fed today", fit);
    let (fit, _) = imitate::<ALL_INPUTS>(&corpus, &all, &gram, EPOCHS, 0);
    report_fit("both", fit);

    // ---- what one pill actually looks like to a scorer of the kind being trained
    println!("\n== what a scorer of the kind being trained actually sees ==");
    println!("a scorer only ever has to separate the placements of one pill from each other.");
    println!("spread is how far apart it puts them; drift is how far its numbers move from one");
    println!("pill to the next. drift the scorer cannot use, and spread lost under it is a");
    println!("decision made by rounding error.");
    println!(
        "\n{:<24} {:>12} {:>12} {:>10}",
        "scorer", "spread", "drift", "ratio"
    );
    for (at, name) in ["the embedded network", "hand written linear"]
        .iter()
        .enumerate()
    {
        let spread = corpus.decisions.iter().map(|d| d.seen[at].1).sum::<f64>()
            / corpus.decisions.len() as f64;
        let means: Vec<f64> = corpus.decisions.iter().map(|d| d.seen[at].0).collect();
        let mean = means.iter().sum::<f64>() / means.len() as f64;
        let drift =
            (means.iter().map(|m| (m - mean).powi(2)).sum::<f64>() / means.len() as f64).sqrt();
        println!(
            "{:<24} {:>12.5} {:>12.5} {:>10.3}",
            name,
            spread,
            drift,
            spread / drift.max(1e-12)
        );
    }

    // ---- which features a scorer would take first
    println!("\n== what it would take, in order ==");
    println!("the input that adds most to the agreement, added one at a time");
    let mut taken: Vec<usize> = vec![];
    let mut best_so_far: f64 = 0.0;
    for _ in 0..14 {
        let mut best: Option<(f64, usize)> = None;
        for column in distinct.iter() {
            if taken.contains(column) {
                continue;
            }
            let mut trial = taken.clone();
            trial.push(*column);
            let weights = gram.fit(&trial);
            let fit = evaluate(&corpus, &gram, &trial, &weights, None);
            if best.is_none_or(|(score, _)| fit.agreement > score) {
                best = Some((fit.agreement, *column));
            }
        }
        let Some((score, column)) = best else { break };
        taken.push(column);
        println!(
            "{:>2}. {:<32} {:>6.1}%  (+{:.1})",
            taken.len(),
            corpus.names[column],
            100.0 * score,
            100.0 * (score - best_so_far)
        );
        best_so_far = score;
    }

    // ---- what each input is worth on top of everything else
    println!("\n== what each input costs to remove ==");
    println!("the fitted scorer's agreement with every input but this one. A feature that costs");
    println!("nothing to remove is not earning the thirty two weights the first layer spends on");
    println!("it, however well it does on its own - what matters is what it adds to the rest.");
    let fed: Vec<usize> = distinct.iter().copied().filter(|c| *c < NOW).collect();
    let whole = {
        let weights = gram.fit(&fed);
        evaluate(&corpus, &gram, &fed, &weights, None).agreement
    };
    println!("  everything: {:.1}%", 100.0 * whole);
    let mut without: Vec<(f64, usize)> = fed
        .iter()
        .map(|dropped| {
            let rest: Vec<usize> = fed.iter().copied().filter(|c| c != dropped).collect();
            let weights = gram.fit(&rest);
            let fit = evaluate(&corpus, &gram, &rest, &weights, None);
            (whole - fit.agreement, *dropped)
        })
        .collect();
    without.sort_by(|a, b| a.0.total_cmp(&b.0));
    println!("{:<34} {:>10}", "input", "costs");
    for (cost, column) in &without {
        println!("{:<34} {:>+9.1}%", corpus.names[*column], 100.0 * cost);
    }

    // ---- inputs that nearly say the same thing
    println!("\n== inputs that nearly say the same thing ==");
    println!("the most correlated pairs of what the model is fed, over the placements of a pill.");
    println!("a pair near 1 is two inputs the scorer cannot tell apart, at twice the weights.");
    let mut pairs: Vec<(f64, usize, usize)> = vec![];
    for i in fed.iter().copied() {
        for j in fed.iter().copied().take_while(|j| *j < i) {
            let correlation = gram.xx[i][j] / (gram.xx[i][i] * gram.xx[j][j]).sqrt().max(1e-12);
            pairs.push((correlation.abs(), i, j));
        }
    }
    pairs.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (correlation, i, j) in pairs.iter().take(8) {
        println!(
            "{:<32} {:<32} {:.3}",
            corpus.names[*i], corpus.names[*j], correlation
        );
    }

    // ---- how much the situation matters
    println!("\n== how much the situation matters ==");
    println!("one scorer fitted over every bottle, against one fitted for that situation alone");
    let shared = gram.fit(&distinct);
    for (situation, count) in &situations {
        if *count < 100 {
            continue;
        }
        let one = evaluate(&corpus, &gram, &distinct, &shared, Some(*situation));
        let own = Gram::of_situation(&corpus, *situation);
        let fitted = own.fit(&distinct);
        let apart = evaluate(&corpus, &own, &distinct, &fitted, Some(*situation));
        println!(
            "{:<14} {:>5} pills   one scorer {:>5.1}%   its own {:>5.1}%",
            format!("{:?}", situation),
            count,
            100.0 * one.agreement,
            100.0 * apart.agreement
        );
    }

    // ---- and how those weights get on when they have to play
    println!("\n== how a fitted scorer gets on when it has to play ==");
    println!(
        "{} whole games from the first bottle, at virus level {}",
        games, level
    );
    println!(
        "\n{:<34} {:>8} {:>9} {:>8} {:>10}",
        "scorer", "viruses", "bottles", "buried", "per pill"
    );
    let n64 = play(games, level, |bottle, placements| {
        N64Ai::new().choose(bottle, placements)
    });
    println!("{}", n64.line("the n64 itself"));
    for (name, columns) in [
        ("fitted to what it is fed today", &now),
        ("fitted to both", &all),
    ] {
        let weights = gram.fit(columns);
        let played = play(games, level, |_bottle, placements| {
            choose_by_weights(placements, columns, &gram.deviation, &weights)
        });
        println!("{}", played.line(name));
    }
    let hand = play(games, level, |_bottle, placements| {
        placements
            .iter()
            .enumerate()
            .map(|(index, placement)| (index, Scorer::Linear.rank(&[placement.features()])[0]))
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(index, _)| index)
    });
    println!("{}", hand.line("the hand written linear baseline"));

    // ---- what a run of the genetic algorithm has to select between
    println!("\n== what generation zero looks like ==");
    println!(
        "{} random genomes, each playing the one game a `ga dr auto` run scores it on: bottle",
        GENOMES
    );
    println!("0 upwards, with the inputs fed raw as they are today. A generation the algorithm");
    println!("cannot tell apart is a generation it cannot select from.");
    let mut scored: Vec<u32> = (0..GENOMES)
        .map(|_| {
            let weights: Vec<f64> = (0..DR_NEURAL_GENOME_SIZE)
                .map(|_| rand::random::<f64>() * 2.0 - 1.0)
                .collect();
            let network = models::DrNeuralNetwork::new(&weights);
            play(1, 0, |_bottle, placements| {
                let features: Vec<BottleFeatures> =
                    placements.iter().map(|p| p.features()).collect();
                let scores = Scorer::Network(network).rank(&features);
                (0..placements.len()).max_by(|a, b| scores[*a].total_cmp(&scores[*b]))
            })
            .viruses
        })
        .collect();
    scored.sort_unstable();
    println!(
        "\nviruses cleared: worst {}, median {}, best {}, out of the {} in the first bottle",
        scored[0],
        scored[scored.len() / 2],
        scored[scored.len() - 1],
        viruses_at_level(0)
    );
    println!(
        "{} of {} of them cleared nothing at all",
        scored.iter().filter(|v| **v == 0).count(),
        scored.len()
    );

    // ---- is anything left out of the model worth putting back
    if ablate {
        println!("\n== is anything left out of the model worth putting back ==");
        println!("the same network cloned onto what the model is fed and then onto that plus");
        println!(
            "the inputs left out of it, each sent out to play {} whole games, averaged over {}",
            games, CLONES
        );
        println!(
            "\n{:<34} {:>10} {:>8} {:>9} {:>9}",
            "inputs", "agreement", "pairs", "viruses", "bottles"
        );
        let row = |name: &str, (fit, played): (Fit, Played)| {
            println!(
                "{:<34} {:>9.1}% {:>7.1}% {:>9} {:>9}",
                name,
                100.0 * fit.agreement,
                100.0 * fit.pairs,
                played.viruses,
                played.bottles
            );
        };
        row(
            "what the model is fed",
            clone_and_play::<NOW>(&corpus, &now, &gram, EPOCHS, games, level, CLONES),
        );
        row(
            "and the inputs left out of it",
            clone_and_play::<ALL_INPUTS>(&corpus, &all, &gram, EPOCHS, games, level, CLONES),
        );
    }

    // ---- which of the n64's own terms matter
    if ablate {
        println!("\n== taking the n64's own terms out ==");
        println!("how often it changes its mind without one of them");
        let mut ablations = corpus.ablations.clone();
        ablations.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        for (name, count) in &ablations {
            println!(
                "{:>5.1}%  {}",
                100.0 * *count as f64 / corpus.decisions.len() as f64,
                name
            );
        }
    }

    Ok(())
}
