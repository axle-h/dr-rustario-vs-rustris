# The ai

`engine/src/ai/` owns network shapes, genome, genetic algorithm and the `Fitness` seam; each game
supplies features, placement search and agent under `<crate>/src/game/ai/`. The search is stepped
once a frame and always interruptible (`agent.rs`), so a piece keeps falling while it thinks.

## Dr. Rustario

* Every difficulty plays the Dr. Mario 64 port in `game/ai/n64/`; `params.rs` holds the weights
  and `SKILL_ORDER`. The demos field the trained network (`ai/models.rs`), the two player one
  against the port's best row. `DrAiKind` picks between them; `ga dr` trains it.
* A tuck soft drops: see `DrAiAgent` in `agent.rs`.
* The network's inputs are `evaluator::raw_inputs`, off the one scan in `features.rs`. Change them
  only through `ga dr screen`, and compare medians, never best-of-N.
* `delta.viruses_killed` is fed because a kill otherwise reads as its last block of work, and the
  model tidies loose vitamins over a lone virus. `Choices` in `agent.rs` counts that, and every
  `ga dr` report prints it.
* Measured and dropped, so do not re-add without a screen: a one ply lookahead (`one_away`),
  `halves_over_virus`, `delta.max_height`, the 26 input superset. `holes` is the probe's
  control group.
* A screen teaches the n64 port's play and judges it solo from bottle 0, so it cannot credit an
  input for finishing, combos or surviving garbage. `patterns_cleared`, `entrance_height` and
  `context.viruses` are fed for those, though screens had dropped the first two.
* Stage two's fitness is `run::merit`: viruses and `BOTTLE_BONUS` a bottle first, garbage sent
  second at `GARBAGE_PRICE` times `COMBO_DISCOUNT`, and a buried game forfeits its garbage and pays a flat
  `BURIAL_CHARGE`. Games start from every one of `START_LEVELS` under `N64_FIRE`, so the network
  has seen garbage land. Watch vitamins over a kill: it climbed when garbage was worth too much
  or the charge grew with the bottle.
  `ga dr garbage` measures the price and the fire off the n64 port; never tune them by hand.
* A row gap is work only if one pill can leave a half in it now (`Grid::fillable`); counted over
  a well, the network built lines across valleys it could not fill.
* The `place.*` inputs read the halves as the pill locks, before anything clears; read after,
  a kill's halves are gone and it outbids nothing. The rest describe the settled bottle.
* A placement that kills the last virus settles to an empty bottle (`drop_and_settle`), as the
  game ends the bottle there; scored on its leftovers, a finish read as any one-virus kill and
  the network passed on it. `ga dr passes` shows the boards behind both `Choices` counts, and
  behind pills that cleared nothing with a kill on offer.
* `ga dr auto` past a well-taught seed selects on seed luck; `SEEDS_PER_LEVEL` is the lever.
  A game's merit varies by about 130, so compare models with `ga dr compare` over thousands of
  games, never 40. A seeded run mutates gently: stronger rates sink the median every generation.
  The seed sits in the final playoff, so a run never returns worse than it started.
* A generation is raced (`Phase::with_racing`): everyone plays a sixth of the block and only the
  best go on, so the games go where selection decides something.
* Training never meets an opponent, so solo merit can rise while a model fights worse.
  `ga dr duel` plays two brains head to head, in a sprint to clear one bottle or a marathon until
  one is buried, with garbage crossing; checkpoints report duels with the n64 and the playoff
  goes to the most marathons won.
* `ga dr tune [weights] [generations]` saves each checkpoint as `gen-N.weights`; a `STOP` file in
  its working directory ends it after the next generation, playoff and all.
* Also measured worse: counting tuck-reachable cells in `Grid::reachable`, a rounded fitness
  (selection falls through to the tiebreak), and the engine's default elite rate.
* `probe.rs` and `explain.rs` diagnose feature choice and trained-model behaviour; `ga dr align`
  shows where placed halves sit against their colour, chosen against offered.

## Rustris

A small neural network, weights embedded in `game/ai/models.rs`.

## Puyo Rusto

`game/ai/beam.rs` is a beam search over `eval.rs`'s weights with a quiescence search
(`quiet.rs`); no neural model, ever. Built from [ama](https://github.com/citrus610/ama). It is the
only ai that reads the pending-attack tray, which is why the other two games' duels are
one-sided. `ga puyo rank` measures the ladder in a solo marathon, which takes no nuisance, so it
measures building only; see `SKILL_ORDER` in `skill.rs` for the ladder under fire.
