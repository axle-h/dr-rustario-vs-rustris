//! The search: play the pair in play and the ones behind it, keep the best few boards at each
//! step, and see which first move the good boards came from.
//!
//! A Puyo board is far too wide to search exhaustively - twenty two placements a pair, so five
//! pairs is five million boards - and far too *shallow* to search greedily, because a chain
//! is built over a dozen pairs and no single one of them looks like progress. A beam is the
//! usual answer to both: expand every placement of the next pair from every board kept, score
//! the results, throw all but the best `width` away, and go again.
//!
//! Three things about it are worth saying out loud.
//!
//! **The search runs past the queue.** A player sees the pair in play and
//! [two more](crate::game::random::PEEK_SIZE), which is three pairs, and three pairs is not
//! enough depth to tell a chain from a heap. So the search carries on past them down an
//! invented continuation, and the continuations are not random: they are the six queues in
//! takapt's beam search - by way of ama, which found six fixed ones as good as fifty random -
//! that between them contain every kind of pair there is without caring which way round its
//! colours came. Guessing that the next pair is red-yellow costs nothing when what is being
//! asked is *whether there is room to keep building*, which is a question about the board.
//!
//! **What is chosen is not the best board.** Every node remembers which of the root's
//! placements it descends from, and a placement's worth is the best board reachable under it.
//! Separately, each root placement remembers the biggest chain found anywhere below it - so
//! the search knows both what to build towards and what it could fire right now, and the
//! decision between the two is [`Plan`]'s.
//!
//! **It is run a piece at a time, and it always has an answer.** [`Search`] is a state machine
//! rather than a function: [`Search::new`] plays the pair in play and stops, and each
//! [`Search::step`] after it plays a handful of boards forward and hands the frame back. That
//! is not a performance trick, it is what makes the same search affordable on a handheld: the
//! agent has the pair's whole fall time to think - a second or more, sixty frames - and taking
//! it in eight-board pieces turns one stall into eight ordinary frames without giving up a
//! single board of the search. And because the root placements are all scored and ranked
//! before the first [`step`](Search::step), a search that is interrupted - by the pair coming
//! to rest on a board too full to fall through - still answers, with the best board it had
//! got to.

use crate::game::ai::eval::{self, Weights};
use crate::game::ai::field::Field;
use crate::game::ai::placement::{moves, Drop, RootMove, MAX_MOVES};
use crate::game::cell::PuyoColor;

/// How the search is run. Every one of these is a difficulty dial, and none of them is a
/// speed limit: a wider, deeper search is a *better player*, which is the whole point of
/// having them - see [`crate::game::ai::skill`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchConfig {
    /// how many boards survive each step
    pub width: usize,
    /// how many of the pairs the player can see behind the one in play are searched. A row
    /// that looks at none of them is playing the pair in front of it and nothing else
    pub queue_depth: usize,
    /// how many pairs to invent and search past the ones the player can see
    pub lookahead: usize,
    /// how many of the six continuations to try, each an independent search
    pub queues: usize,
    /// the chain score at which it stops building and fires. Zero fires at the first thing it
    /// finds, which is what a beginner does
    pub trigger: u32,
}

/// How many boards one [`Search::step`] plays the next pair onto before handing the frame back.
///
/// A step costs about this many times [`MAX_MOVES`] evaluations - a couple of hundred, which is
/// well under a millisecond on a desktop and a few on a handheld - and a whole search is
/// somewhere between one step and a dozen depending on the row. Eight because it divides the
/// widths the rows actually use into two or three pieces, so no row waits many more frames for
/// an answer than the strength it is buying.
const PARENTS_PER_STEP: usize = 8;

impl SearchConfig {
    /// How many [`Search::step`]s a whole search takes at most: one layer per pair searched,
    /// and one step per [`PARENTS_PER_STEP`] boards of each layer.
    ///
    /// An upper bound rather than a count - a beam that runs out of boards finishes early -
    /// and the number to divide a measured think time by to get the cost of a frame. It is
    /// also how long the agent waits before it has an answer, in frames, which is why no row
    /// is allowed to want more of them than a pair takes to fall.
    pub fn steps(&self) -> usize {
        let queues = self.queues.clamp(1, CONTINUATIONS.len());
        let layers = self.queue_depth + queues * self.lookahead;
        (layers * self.width.div_ceil(PARENTS_PER_STEP)).max(1)
    }
}

/// The six continuations, as colour indices.
///
/// From ama, by way of takapt: searching past the queue down several invented futures is worth
/// far more than searching one further pair of the real one, and these six between them cover
/// every kind of pair - two colours in either order, and the pairs a doublet stands in for -
/// without any same-coloured pair, which makes an ai overrate its chances.
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
    /// the running total of what the placements along the way cost - tears and puyos spent.
    /// Kept apart from the board's own score because it is paid once and carried, while the
    /// board is only ever worth what it is worth now
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

/// What the search made of one of the root's placements.
///
/// Ama ranks its candidates on `chain_score` alone - the biggest chain found anywhere under
/// each one - because its horizon is sixteen pairs deep and by then every branch worth having
/// has found a chain. Three visible pairs and a couple of invented ones is not that horizon,
/// and on it most placements find nothing at all, so what is ranked here is the *board* at
/// the far end instead. The chain is not thrown away: it is what [`Plan`] decides on.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub root: RootMove,
    /// what the board is worth the moment this placement is made
    pub immediate: i32,
    /// the best board reachable at the far end of the search - `None` when the beam cut every
    /// branch under this placement before it got there
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

/// A search in progress.
///
/// It is built with the pair in play already played out - so it can be asked for an answer
/// from the moment it exists - and then stepped until [`finished`](Self::finished).
pub struct Search {
    candidates: Vec<Candidate>,
    weights: Weights,
    config: SearchConfig,
    /// the pairs the player can see, cut to what this row bothers to read
    queue: Vec<[u8; 2]>,
    /// the boards this layer is being expanded from, and how far through them it has got
    parents: Vec<Node>,
    parent: usize,
    /// what this layer has expanded into so far
    children: Vec<Node>,
    /// the beam as it stood when the visible queue ran out. Every continuation forks from
    /// here, because they only differ past the pairs that are real
    trunk: Vec<Node>,
    stage: Stage,
}

impl Search {
    /// Play every placement of the pair in play, score the boards, and stop.
    ///
    /// The root layer is done here rather than in a step because it is what makes the search
    /// interruptible: from this point on every placement has been scored, so there is always
    /// an answer, and every step after this only sharpens it.
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

        // The root layer is a beam layer like any other and is cut to the width like any
        // other. Expanding all twenty two of the pair's placements before the first cut costs
        // more than every layer after it put together - each of those starts from `width`
        // boards, not from twenty two. What it costs is that the placements cut here are only
        // ever ranked on the board they make, which is what `immediate` is for.
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

    /// Play the next pair onto [`PARENTS_PER_STEP`] more of the boards being held, and report
    /// whether that finished the search.
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

    /// one layer is done: work out which pair comes next, and record the horizon whenever a
    /// branch has reached its end
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
                    // this continuation ran out of boards; go to the next one
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

    /// The root placements, best first, and what playing the first of them would mean.
    ///
    /// Handing back the whole order rather than the winner is what lets the agent take the
    /// next one down when the pair has fallen too far to reach the best.
    pub fn ranking(&self, pressed: bool) -> (Vec<usize>, Plan) {
        ranking(&self.candidates, &self.config, pressed)
    }
}

/// The far end of the search: the layer the placements are ranked on, and the only one where
/// every board has had the same number of pairs played onto it.
fn record_horizon(beam: &[Node], candidates: &mut [Candidate]) {
    for node in beam {
        let horizon = &mut candidates[node.root].horizon;
        *horizon = Some(horizon.map_or(node.score(), |best: i32| best.max(node.score())));
    }
}

/// the `nth` pair of an invented continuation: the colours in the order the continuation
/// names them, two at a time, round and round
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

            // what this branch could fire is the root placement's to remember, whether or not
            // the branch itself is worth keeping
            let candidate = &mut candidates[node.root];
            candidate.chain_score = candidate.chain_score.max(chain.score);

            // a board that has just spent itself on a big chain has nothing left to say about
            // how well it was built, so it leaves the beam rather than crowding out the boards
            // that are still building
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

/// A chain big enough that a board which fired it is finished, and is dropped from the beam
/// however good it looks. Ama's `PRUNE`.
const PRUNE_CHAIN_SCORE: u32 = 5_000;

/// What to do with a pair, once the search has been run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    /// keep building: this placement leads to the best board
    Build,
    /// fire: this placement sets off a chain worth having
    Fire,
}

/// The root placements in the order this player would rather make them, and what making the
/// first of them would mean.
///
/// Building is the default and firing is the exception, which is the right way round: a
/// placement that clears nothing but leaves a bigger chain behind beats one that takes four
/// puyos off the board now. Three things make it fire anyway - a chain at or over the
/// difficulty's [`SearchConfig::trigger`], a board with nowhere left to build (`pressed`), or
/// a placement that has to be made because every other one is fatal.
///
/// It is an order and not a winner because the pair goes on falling while the search runs, and
/// by the time there is an answer the best placement may be out of reach - see
/// [`crate::game::ai::agent`].
pub fn ranking(
    candidates: &[Candidate],
    config: &SearchConfig,
    pressed: bool,
) -> (Vec<usize>, Plan) {
    if candidates.is_empty() {
        return (vec![], Plan::Build);
    }

    // every placement kills: play the ones that score most on the way out rather than
    // freezing, which would look like the ai giving up
    let survivable: Vec<usize> = (0..candidates.len())
        .filter(|i| !candidates[*i].fatal)
        .collect();
    let mut allowed: Vec<usize> = if survivable.is_empty() {
        (0..candidates.len()).collect()
    } else {
        survivable
    };

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

    // the beam's survivors are ranked against each other; only if it cut every branch does
    // the board as it stands right now have to decide it
    let reached: Vec<usize> = allowed
        .iter()
        .copied()
        .filter(|i| candidates[*i].horizon.is_some())
        .collect();
    let mut ranked = if reached.is_empty() { allowed } else { reached };
    let worth = |i: usize| candidates[i].horizon.unwrap_or(candidates[i].immediate);

    ranked.sort_by(|a, b| {
        worth(*b)
            .cmp(&worth(*a))
            // a tie goes to the simpler sequence, so the agent does not walk the long way
            // round to the same board
            .then_with(|| candidates[*a].root.inputs.cmp(&candidates[*b].root.inputs))
    });
    (ranked, Plan::Build)
}

/// the placement this player would rather make, and what making it would mean
pub fn choose(
    candidates: &[Candidate],
    config: &SearchConfig,
    pressed: bool,
) -> Option<(usize, Plan)> {
    let (ranked, plan) = ranking(candidates, config, pressed);
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
        }
    }

    /// run a whole search out, the way a caller with all the time in the world would
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

    /// the search knows what each placement would set off, even the ones it would rather not
    /// make yet
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

    /// and left to itself it builds instead: with the trigger set out of reach, the placement
    /// chosen is not the one that spends the board
    #[test]
    fn it_builds_rather_than_taking_the_chain_in_front_of_it() {
        let (candidates, mut config) = run(
            &[".g....", "rg....", "rrgg.."],
            (PuyoColor::Red, PuyoColor::Blue),
        );
        config.trigger = u32::MAX;
        let (patient, plan) = choose(&candidates, &config, false).expect("a placement");
        assert_eq!(plan, Plan::Build);

        config.trigger = 0;
        let (greedy, plan) = choose(&candidates, &config, false).expect("a placement");
        assert_eq!(plan, Plan::Fire);
        assert_ne!(
            patient, greedy,
            "the two ends of the trigger dial chose the same placement"
        );
    }

    /// with the trigger down at nothing it takes whatever is there, which is the beginner's
    /// game and one end of the difficulty ladder
    #[test]
    fn a_trigger_of_nothing_fires_at_the_first_chain_it_sees() {
        let (candidates, mut config) = run(
            &[".g....", "rg....", "rrgg.."],
            (PuyoColor::Red, PuyoColor::Blue),
        );
        config.trigger = 0;
        let (chosen, plan) = choose(&candidates, &config, false).expect("a placement");
        assert_eq!(plan, Plan::Fire);
        assert!(candidates[chosen].fires > 0);
    }

    /// a placement that buries the player is never chosen while any other one exists
    #[test]
    fn a_fatal_placement_is_the_last_resort() {
        // the spawn column stacked to one below the death square: dropping a pair standing up
        // in it rests a puyo on the square and ends the game
        let rows = vec!["..o..."; 11];
        let (candidates, config) = run(&rows, (PuyoColor::Red, PuyoColor::Blue));
        let fatal = candidates.iter().filter(|c| c.fatal).count();
        assert!(fatal > 0, "some placement in that column has to be fatal");
        let (chosen, _) = choose(&candidates, &config, false).expect("a placement");
        assert!(!candidates[chosen].fatal);
    }

    /// the continuations are what searching past the queue is made of, and they carry every
    /// kind of pair without ever dealing one of a single colour
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

    /// A search that is stopped part way still answers, and answers sensibly: the placements
    /// are all scored before the first step, so the worst it can do is fall back on the board
    /// each one makes. Without that a pair coming to rest before the search finished would
    /// have nothing to play.
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
            let (ranked, _) = search.ranking(false);
            assert!(!ranked.is_empty(), "no answer after {steps} steps");
            if search.step() {
                break;
            }
            steps += 1;
        }
        assert!(steps > 0, "this row was meant to take more than one step");
    }

    /// every row finishes in few enough steps that a pair has time to fall through them, and
    /// in no more than the number `steps()` promises - which is what a measured think time is
    /// divided by to get the cost of a frame
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
