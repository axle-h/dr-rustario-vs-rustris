//! A beam search: play the pair in play and the pairs behind it, keep the best `width` boards
//! at each layer, and rank each root placement by the best board reachable under it.
//!
//! Past the visible queue it plays six fixed invented continuations (takapt's, by way of ama).
//! [`Search`] is stepped a few boards a frame, and every root placement is scored before the
//! first step, so an interrupted search still has an answer.

use crate::game::ai::eval::{self, Weights};
use crate::game::ai::field::Field;
use crate::game::ai::placement::{moves, Drop, RootMove, MAX_MOVES};
use crate::game::cell::PuyoColor;

/// How the search is run. Every field is a difficulty dial, not a speed limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchConfig {
    /// how many boards survive each step
    pub width: usize,
    /// how many of the visible pairs behind the one in play are searched
    pub queue_depth: usize,
    /// how many pairs to invent and search past the ones the player can see
    pub lookahead: usize,
    /// how many of the six continuations to try, each an independent search
    pub queues: usize,
    /// the chain score at which it stops building and fires; zero fires at anything
    pub trigger: u32,
    /// How deep the tray has to get, in nuisance puyos, before this row fires a chain to answer
    /// it; [`u32::MAX`] never does. Answering is right but rarely fires, since
    /// [`Candidate::fires`] is usually zero when a tray is seen, and the dial measures flat.
    pub answer_at: u32,
}

/// How many boards one [`Search::step`] expands before handing the frame back. Eight splits the
/// widths the rows use into two or three steps a layer.
const PARENTS_PER_STEP: usize = 8;

impl SearchConfig {
    /// An upper bound on the [`Search::step`]s a whole search takes, which is also how many
    /// frames the agent waits for an answer, so no row may need more than a pair takes to fall.
    pub fn steps(&self) -> usize {
        let queues = self.queues.clamp(1, CONTINUATIONS.len());
        let layers = self.queue_depth + queues * self.lookahead;
        (layers * self.width.div_ceil(PARENTS_PER_STEP)).max(1)
    }
}

/// The six continuations, as colour indices. Between them they cover every two-colour pair in
/// either order and never deal a same-coloured one, which makes an ai overrate its chances.
const CONTINUATIONS: [[usize; 4]; 6] = [
    [0, 3, 1, 2],
    [0, 1, 3, 2],
    [0, 2, 3, 1],
    [3, 1, 0, 2],
    [3, 2, 0, 1],
    [1, 2, 0, 3],
];

/// one board in the beam, and which of the root's placements it came from
#[derive(Clone, Copy)]
struct Node {
    field: Field,
    /// the running total of what the placements along the way cost, tears and puyos spent
    action: i32,
    /// the board's own score, as of this layer
    eval: i32,
    root: usize,
}

impl Node {
    fn score(&self) -> i32 {
        self.eval + self.action
    }
}

/// What the search made of one of the root's placements. It is ranked on the board at the
/// horizon rather than on `chain_score` as ama does, because this horizon is too short for most
/// branches to find a chain.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub root: RootMove,
    /// what the board is worth the moment this placement is made
    pub immediate: i32,
    /// the best board at the far end of the search, `None` when the beam cut every branch under it
    pub horizon: Option<i32>,
    /// the biggest chain found anywhere under it, in the game's own points
    pub chain_score: u32,
    /// the chain this placement fires itself, right now
    pub fires: u32,
    /// playing it leaves a puyo resting on the death square
    pub fatal: bool,
}

/// Which pair the search is playing next, and where it is up to in playing it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    /// the `n`th of the pairs the player can actually see
    Queue(usize),
    /// the `step`th invented pair of continuation `queue`
    Invented {
        queue: usize,
        step: usize,
    },
    Done,
}

/// A search in progress, built with the pair in play already played out.
pub struct Search {
    candidates: Vec<Candidate>,
    weights: Weights,
    config: SearchConfig,
    /// the visible pairs, cut to what this row reads
    queue: Vec<[u8; 2]>,
    /// the boards this layer is being expanded from, and how far through them it has got
    parents: Vec<Node>,
    parent: usize,
    /// what this layer has expanded into so far
    children: Vec<Node>,
    /// the beam when the visible queue ran out, which every continuation forks from
    trunk: Vec<Node>,
    stage: Stage,
}

impl Search {
    /// Play every placement of the pair in play, score the boards, and stop. Doing the root
    /// layer here is what makes the search interruptible.
    pub fn new(
        field: &Field,
        roots: Vec<RootMove>,
        queue: &[[u8; 2]],
        weights: Weights,
        config: SearchConfig,
    ) -> Self {
        let mut candidates: Vec<Candidate> = roots
            .into_iter()
            .map(|root| Candidate {
                root,
                immediate: i32::MIN,
                horizon: None,
                chain_score: 0,
                fires: 0,
                fatal: false,
            })
            .collect();

        let mut parents: Vec<Node> = Vec::with_capacity(candidates.len());
        for (index, candidate) in candidates.iter_mut().enumerate() {
            let mut next = *field;
            let Some((tear, chain)) = candidate.root.drop.apply(&mut next) else {
                candidate.fatal = true;
                continue;
            };
            candidate.fires = chain.score;
            candidate.chain_score = chain.score;
            candidate.fatal = next.is_dead();
            let action = eval::action(tear, chain.popped, &weights);
            let eval =
                eval::evaluate(&next, &weights) + if candidate.fatal { weights.death } else { 0 };
            candidate.immediate = eval + action;
            parents.push(Node {
                field: next,
                action,
                eval,
                root: index,
            });
        }

        // the root layer is cut to the width like any other, so placements cut here are only
        // ranked on `immediate`
        parents.sort_unstable_by_key(|node| std::cmp::Reverse(node.score()));
        parents.truncate(config.width);

        let queue: Vec<[u8; 2]> = queue.iter().take(config.queue_depth).copied().collect();

        let mut search = Self {
            candidates,
            weights,
            config,
            queue,
            parents,
            parent: 0,
            children: vec![],
            trunk: vec![],
            stage: Stage::Done,
        };
        search.stage = search.opening_stage();
        search
    }

    fn opening_stage(&mut self) -> Stage {
        if self.parents.is_empty() {
            return Stage::Done;
        }
        if !self.queue.is_empty() {
            return Stage::Queue(0);
        }
        if self.config.lookahead > 0 {
            self.trunk = self.parents.clone();
            return Stage::Invented { queue: 0, step: 0 };
        }
        Stage::Done
    }

    pub fn finished(&self) -> bool {
        self.stage == Stage::Done
    }

    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }

    /// Expand `PARENTS_PER_STEP` more boards, and report whether that finished the search.
    pub fn step(&mut self) -> bool {
        let pair = match self.stage {
            Stage::Done => return true,
            Stage::Queue(n) => self.queue[n],
            Stage::Invented { queue, step } => invented(&CONTINUATIONS[queue], step),
        };

        let end = (self.parent + PARENTS_PER_STEP).min(self.parents.len());
        expand_into(
            &self.parents[self.parent..end],
            pair,
            &self.weights,
            &mut self.children,
            &mut self.candidates,
        );
        self.parent = end;
        if self.parent < self.parents.len() {
            return false;
        }

        // the layer is complete: cut it to the width and make it the one to expand from
        self.children
            .sort_unstable_by_key(|node| std::cmp::Reverse(node.score()));
        self.children.truncate(self.config.width);
        self.parents = std::mem::take(&mut self.children);
        self.parent = 0;
        self.advance();
        self.finished()
    }

    /// one layer is done: pick the next pair, and record the horizon when a branch has ended
    fn advance(&mut self) {
        self.stage = match self.stage {
            Stage::Done => Stage::Done,
            Stage::Queue(n) if n + 1 < self.queue.len() => Stage::Queue(n + 1),
            Stage::Queue(_) => {
                if self.config.lookahead == 0 || self.parents.is_empty() {
                    record_horizon(&self.parents, &mut self.candidates);
                    Stage::Done
                } else {
                    self.trunk = self.parents.clone();
                    Stage::Invented { queue: 0, step: 0 }
                }
            }
            Stage::Invented { queue, step } if step + 1 < self.config.lookahead => {
                if self.parents.is_empty() {
                    self.next_continuation(queue)
                } else {
                    Stage::Invented {
                        queue,
                        step: step + 1,
                    }
                }
            }
            Stage::Invented { queue, .. } => {
                record_horizon(&self.parents, &mut self.candidates);
                self.next_continuation(queue)
            }
        };
    }

    fn next_continuation(&mut self, queue: usize) -> Stage {
        let queues = self.config.queues.clamp(1, CONTINUATIONS.len());
        if queue + 1 < queues && !self.trunk.is_empty() {
            self.parents = self.trunk.clone();
            self.parent = 0;
            Stage::Invented {
                queue: queue + 1,
                step: 0,
            }
        } else {
            Stage::Done
        }
    }

    /// The root placements, best first, and what playing the first would mean. The agent takes
    /// the next one down when the pair has fallen too far to reach the best.
    pub fn ranking(&self, pressed: bool, pending: u32) -> (Vec<usize>, Plan) {
        ranking(&self.candidates, &self.config, pressed, pending)
    }
}

/// The far end of the search, the only layer where every board has had the same number of
/// pairs played onto it.
fn record_horizon(beam: &[Node], candidates: &mut [Candidate]) {
    for node in beam {
        let horizon = &mut candidates[node.root].horizon;
        *horizon = Some(horizon.map_or(node.score(), |best: i32| best.max(node.score())));
    }
}

/// the `nth` pair of an invented continuation, cycling through its colours two at a time
fn invented(continuation: &[usize; 4], nth: usize) -> [u8; 2] {
    let first = continuation[(nth * 2) % continuation.len()];
    let second = continuation[(nth * 2 + 1) % continuation.len()];
    [
        crate::game::ai::field::of_color(PuyoColor::from_index(first)),
        crate::game::ai::field::of_color(PuyoColor::from_index(second)),
    ]
}

/// every placement of `pair` on every board of `parents`, scored and added to `children`
fn expand_into(
    parents: &[Node],
    pair: [u8; 2],
    weights: &Weights,
    children: &mut Vec<Node>,
    candidates: &mut [Candidate],
) {
    let mut buffer = [Drop::new([0, 0], pair); MAX_MOVES];

    for node in parents {
        let n = moves(&node.field, pair, &mut buffer);
        for drop in &buffer[..n] {
            let mut next = node.field;
            let Some((tear, chain)) = drop.apply(&mut next) else {
                continue;
            };

            // the root remembers what any branch under it could fire, kept or not
            let candidate = &mut candidates[node.root];
            candidate.chain_score = candidate.chain_score.max(chain.score);

            // a board that has just fired a big chain leaves the beam rather than crowding out
            // boards still building
            if chain.score >= PRUNE_CHAIN_SCORE || next.is_dead() {
                continue;
            }

            let action = node.action + eval::action(tear, chain.popped, weights);
            let eval = eval::evaluate(&next, weights);
            children.push(Node {
                field: next,
                action,
                eval,
                root: node.root,
            });
        }
    }
}

/// A chain big enough that a board which fired it is dropped from the beam. Ama's `PRUNE`.
const PRUNE_CHAIN_SCORE: u32 = 5_000;

/// What to do with a pair, once the search has been run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    /// keep building towards the best board
    Build,
    /// fire a chain worth having
    Fire,
    /// fire the smallest chain that cancels the whole tray
    Answer,
}

/// The root placements in the order this player would rather make them, and what making the
/// first would mean. It builds unless a chain reaches [`SearchConfig::trigger`], the board is
/// `pressed`, every other placement is fatal, or `pending` reaches `answer_at`.
///
/// An answer takes the smallest chain covering all of `pending`, since Tsu offset drops whatever
/// is still waiting and a partial answer buys nothing; with none big enough it keeps building.
pub fn ranking(
    candidates: &[Candidate],
    config: &SearchConfig,
    pressed: bool,
    pending: u32,
) -> (Vec<usize>, Plan) {
    if candidates.is_empty() {
        return (vec![], Plan::Build);
    }

    // every placement kills: play the best-scoring one rather than freezing
    let survivable: Vec<usize> = (0..candidates.len())
        .filter(|i| !candidates[*i].fatal)
        .collect();
    let mut allowed: Vec<usize> = if survivable.is_empty() {
        (0..candidates.len()).collect()
    } else {
        survivable
    };

    let worth = |i: usize| candidates[i].horizon.unwrap_or(candidates[i].immediate);

    let fires = allowed
        .iter()
        .map(|i| candidates[*i].fires)
        .max()
        .unwrap_or(0);
    if fires > 0 && (fires >= config.trigger || pressed) {
        allowed.sort_by(|a, b| {
            candidates[*b]
                .fires
                .cmp(&candidates[*a].fires)
                .then_with(|| candidates[*a].root.inputs.cmp(&candidates[*b].root.inputs))
        });
        return (allowed, Plan::Fire);
    }

    // the tray is deep enough to answer and a chain here covers the whole of it
    if pending > 0 && pending >= config.answer_at {
        let wanted = crate::game::ai::skill::nuisance(pending);
        let mut answers: Vec<usize> = allowed
            .iter()
            .copied()
            .filter(|i| candidates[*i].fires >= wanted)
            .collect();
        if !answers.is_empty() {
            answers.sort_by(|a, b| {
                worth(*b)
                    .cmp(&worth(*a))
                    // on a tie, the one that spent less keeps the rest for next turn
                    .then_with(|| candidates[*a].fires.cmp(&candidates[*b].fires))
                    .then_with(|| candidates[*a].root.inputs.cmp(&candidates[*b].root.inputs))
            });
            return (answers, Plan::Answer);
        }
    }

    // rank the beam's survivors; only if every branch was cut does the board as it stands decide
    let reached: Vec<usize> = allowed
        .iter()
        .copied()
        .filter(|i| candidates[*i].horizon.is_some())
        .collect();
    let mut ranked = if reached.is_empty() { allowed } else { reached };

    ranked.sort_by(|a, b| {
        worth(*b)
            .cmp(&worth(*a))
            // a tie goes to the shorter input sequence
            .then_with(|| candidates[*a].root.inputs.cmp(&candidates[*b].root.inputs))
    });
    (ranked, Plan::Build)
}

/// the placement this player would rather make, and what making it would mean
pub fn choose(
    candidates: &[Candidate],
    config: &SearchConfig,
    pressed: bool,
    pending: u32,
) -> Option<(usize, Plan)> {
    let (ranked, plan) = ranking(candidates, config, pressed, pending);
    ranked.first().map(|best| (*best, plan))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ai::field::of_color;
    use crate::game::ai::placement::root_moves;
    use crate::game::board::tests::board;
    use crate::game::board::SPAWN;
    use crate::game::cell::PuyoPiece;
    use crate::game::pair::Pair;

    fn config() -> SearchConfig {
        SearchConfig {
            width: 24,
            queue_depth: 2,
            lookahead: 2,
            queues: 2,
            trigger: 3_000,
            answer_at: 30,
        }
    }

    /// run a whole search to the end
    fn run(rows: &[&str], piece: (PuyoColor, PuyoColor)) -> (Vec<Candidate>, SearchConfig) {
        let board = board(rows);
        let pair = Pair::new(SPAWN, PuyoPiece::new(piece.0, piece.1));
        let config = config();
        let mut search = Search::new(
            &Field::from_board(&board),
            root_moves(&board, pair),
            &[],
            Weights::BUILD,
            config,
        );
        let mut steps = 0;
        while !search.step() {
            steps += 1;
            assert!(steps < 1_000, "the search never finished");
        }
        (search.candidates().to_vec(), config)
    }

    /// every placement knows what it would set off, chosen or not
    #[test]
    fn a_placement_that_fires_a_chain_says_so() {
        let (candidates, _) = run(
            &[".g....", "rg....", "rrgg.."],
            (PuyoColor::Red, PuyoColor::Blue),
        );
        assert!(
            candidates.iter().any(|c| c.fires > 0),
            "dropping a red on column 0 sets the whole thing off"
        );
    }

    /// with the trigger out of reach it builds rather than spending the board
    #[test]
    fn it_builds_rather_than_taking_the_chain_in_front_of_it() {
        let (candidates, mut config) = run(
            &[".g....", "rg....", "rrgg.."],
            (PuyoColor::Red, PuyoColor::Blue),
        );
        config.trigger = u32::MAX;
        let (patient, plan) = choose(&candidates, &config, false, 0).expect("a placement");
        assert_eq!(plan, Plan::Build);

        config.trigger = 0;
        let (greedy, plan) = choose(&candidates, &config, false, 0).expect("a placement");
        assert_eq!(plan, Plan::Fire);
        assert_ne!(
            patient, greedy,
            "the two ends of the trigger dial chose the same placement"
        );
    }

    /// with the trigger at zero it takes whatever is there
    #[test]
    fn a_trigger_of_nothing_fires_at_the_first_chain_it_sees() {
        let (candidates, mut config) = run(
            &[".g....", "rg....", "rrgg.."],
            (PuyoColor::Red, PuyoColor::Blue),
        );
        config.trigger = 0;
        let (chosen, plan) = choose(&candidates, &config, false, 0).expect("a placement");
        assert_eq!(plan, Plan::Fire);
        assert!(candidates[chosen].fires > 0);
    }

    /// a rock in the tray makes a board that would build fire instead
    #[test]
    fn a_chain_that_covers_the_tray_is_fired_at_a_trigger_it_could_never_reach() {
        let (candidates, mut config) = run(
            &[".g....", "rg....", "rrgg.."],
            (PuyoColor::Red, PuyoColor::Blue),
        );
        config.trigger = u32::MAX;
        config.answer_at = 1;
        let covers = candidates.iter().map(|c| c.fires).max().unwrap_or(0)
            / crate::game::score::TARGET_POINTS;
        assert!(covers > 0, "some placement here fires something");

        let (_, plan) = choose(&candidates, &config, false, 0).expect("a placement");
        assert_eq!(
            plan,
            Plan::Build,
            "with an empty tray there is nothing to answer"
        );

        let (chosen, plan) = choose(&candidates, &config, false, covers).expect("a placement");
        assert_eq!(plan, Plan::Answer);
        assert!(
            candidates[chosen].fires >= crate::game::ai::skill::nuisance(covers),
            "the placement chosen does not cover the tray it was chosen to answer"
        );
    }

    /// a tray shallower than `answer_at` is eaten and the row keeps building
    #[test]
    fn a_tray_shallower_than_the_rows_own_depth_is_ignored() {
        let (candidates, mut config) = run(
            &[".g....", "rg....", "rrgg.."],
            (PuyoColor::Red, PuyoColor::Blue),
        );
        config.trigger = u32::MAX;
        config.answer_at = 30;
        let (_, plan) = choose(&candidates, &config, false, 29).expect("a placement");
        assert_eq!(plan, Plan::Build);
    }

    /// a chain covering only half the tray is not fired; the row keeps building
    #[test]
    fn a_tray_nothing_can_cover_is_not_half_answered() {
        let (candidates, mut config) = run(
            &[".g....", "rg....", "rrgg.."],
            (PuyoColor::Red, PuyoColor::Blue),
        );
        config.trigger = u32::MAX;
        config.answer_at = 1;
        let (_, plan) = choose(&candidates, &config, false, 500).expect("a placement");
        assert_eq!(
            plan,
            Plan::Build,
            "no chain here is worth five hundred puyos"
        );
    }

    /// answering takes the smallest chain that covers the tray, firing the biggest
    #[test]
    fn answering_spends_less_than_firing_does() {
        let (candidates, mut config) = run(
            &["......", "bg....", "bg....", "brrb..", "grrg..", "ggbb.."],
            (PuyoColor::Red, PuyoColor::Green),
        );
        config.trigger = 0;
        let (biggest, plan) = choose(&candidates, &config, false, 0).expect("a placement");
        assert_eq!(plan, Plan::Fire);

        // a tray one puyo deep: anything that fires at all covers it
        config.trigger = u32::MAX;
        config.answer_at = 1;
        let (answer, plan) = choose(&candidates, &config, false, 1).expect("a placement");
        assert_eq!(plan, Plan::Answer);
        assert!(
            candidates[answer].fires < candidates[biggest].fires,
            "answering spent {} where firing would have spent {}",
            candidates[answer].fires,
            candidates[biggest].fires
        );
    }

    /// a pressed board fires the biggest thing it has whatever is in the tray
    #[test]
    fn being_pressed_still_fires_biggest_first() {
        let (candidates, mut config) = run(
            &[".g....", "rg....", "rrgg.."],
            (PuyoColor::Red, PuyoColor::Blue),
        );
        config.trigger = u32::MAX;
        config.answer_at = 1;
        let (chosen, plan) = choose(&candidates, &config, true, 1).expect("a placement");
        assert_eq!(plan, Plan::Fire);
        let biggest = candidates.iter().map(|c| c.fires).max().unwrap_or(0);
        assert_eq!(candidates[chosen].fires, biggest);
    }

    /// a placement that buries the player is never chosen while any other one exists
    #[test]
    fn a_fatal_placement_is_the_last_resort() {
        // the spawn column stacked to one below the death square
        let rows = vec!["..o..."; 11];
        let (candidates, config) = run(&rows, (PuyoColor::Red, PuyoColor::Blue));
        let fatal = candidates.iter().filter(|c| c.fatal).count();
        assert!(fatal > 0, "some placement in that column has to be fatal");
        let (chosen, _) = choose(&candidates, &config, false, 0).expect("a placement");
        assert!(!candidates[chosen].fatal);
    }

    /// the continuations cover every kind of pair and never deal a single colour
    #[test]
    fn the_invented_pairs_are_never_a_doublet() {
        for continuation in CONTINUATIONS.iter() {
            for nth in 0..8 {
                let [a, b] = invented(continuation, nth);
                assert_ne!(a, b, "{continuation:?} dealt a doublet at {nth}");
            }
        }
    }

    #[test]
    fn every_continuation_is_a_permutation_of_four_colours() {
        for continuation in CONTINUATIONS.iter() {
            let mut sorted = *continuation;
            sorted.sort();
            assert_eq!(sorted, [0, 1, 2, 3]);
        }
    }

    /// nothing in the search touches the field it was handed
    #[test]
    fn searching_leaves_the_field_alone() {
        let board = board(&[".g....", "rg....", "rrgg.."]);
        let field = Field::from_board(&board);
        let before = field;
        let pair = Pair::new(SPAWN, PuyoPiece::new(PuyoColor::Red, PuyoColor::Blue));
        let mut search = Search::new(
            &field,
            root_moves(&board, pair),
            &[],
            Weights::BUILD,
            config(),
        );
        while !search.step() {}
        assert!(before == field);
        let _ = of_color(PuyoColor::Red);
    }

    /// a search stopped part way still answers from the scored root placements
    #[test]
    fn a_search_interrupted_at_any_point_still_has_an_answer() {
        let board = board(&[".g....", "rg....", "rrgg.."]);
        let pair = Pair::new(SPAWN, PuyoPiece::new(PuyoColor::Red, PuyoColor::Blue));
        let build = Search::new(
            &Field::from_board(&board),
            root_moves(&board, pair),
            &[],
            Weights::BUILD,
            config(),
        );
        let mut steps = 0;
        let mut search = build;
        loop {
            let (ranked, _) = search.ranking(false, 0);
            assert!(!ranked.is_empty(), "no answer after {steps} steps");
            if search.step() {
                break;
            }
            steps += 1;
        }
        assert!(steps > 0, "this row was meant to take more than one step");
    }

    /// every row finishes within `steps()`, and in few enough steps for a pair to fall through
    #[test]
    fn no_row_takes_more_steps_than_a_pair_has_frames() {
        let board = board(&["......", "rg....", "rrgg.."]);
        let pair = Pair::new(SPAWN, PuyoPiece::new(PuyoColor::Red, PuyoColor::Blue));
        let queue = [[1u8, 2], [3, 4]];
        for row in crate::game::ai::skill::ROWS.iter() {
            let mut search = Search::new(
                &Field::from_board(&board),
                root_moves(&board, pair),
                &queue,
                row.weights,
                row.search,
            );
            let mut steps = 0;
            while !search.step() {
                steps += 1;
                assert!(steps < 60, "{} took over a second to decide", row.name);
            }
            assert!(
                steps < row.search.steps(),
                "{} took {} steps, over the {} it promised",
                row.name,
                steps + 1,
                row.search.steps()
            );
        }
    }
}
