# The ai

`engine/src/ai/` owns network shapes, genome, genetic algorithm and the `Fitness` seam; each game
supplies features, placement search and agent under `<crate>/src/game/ai/`. The search is stepped
once a frame and always interruptible (`agent.rs`), so a piece keeps falling while it thinks.

## Dr. Rustario

* Every difficulty and both demos play the Dr. Mario 64 port in `game/ai/n64/`; `params.rs` holds
  the weights and `SKILL_ORDER`. The trained network (`ai/models.rs`) is fielded by nothing: it
  wins on the numbers and is not good to watch. `DrAiKind` picks between them; `ga dr` trains it.
* A tuck soft drops: see `DrAiAgent` in `agent.rs`.
* The network's inputs are `evaluator::raw_inputs`, off the one scan in `features.rs`. Change them
  only through `ga dr screen`, and compare medians, never best-of-N.
* Measured and dropped, so do not re-add without a screen: a one ply lookahead (`one_away`),
  `halves_over_virus`, `patterns_cleared`, `delta.max_height`, any superset of the nineteen.
  `entrance_height` and `holes` are the probe's control group.
* `ga dr auto` past a well-taught seed selects on seed luck; `SEEDS_PER_GAME` is the lever.
* Also measured worse: counting tuck-reachable cells in `Grid::reachable`, a rounded fitness
  (selection falls through to the tiebreak), and the engine's default elite rate.
* `probe.rs` and `explain.rs` diagnose feature choice and trained-model behaviour.

## Rustris

A small neural network, weights embedded in `game/ai/models.rs`.

## Puyo Rusto

`game/ai/beam.rs` is a beam search over `eval.rs`'s weights with a quiescence search
(`quiet.rs`); no neural model, ever. Built from [ama](https://github.com/citrus610/ama). It is the
only ai that reads the pending-attack tray, which is why the other two games' duels are
one-sided. `ga puyo rank` measures the ladder in a solo marathon, which takes no nuisance, so it
measures building only; see `SKILL_ORDER` in `skill.rs` for the ladder under fire.
