@README.md

## Layout

| crate | what it is |
|---|---|
| `engine/` | everything that is not game rules: SDL app shell, menus, high scores, config, input, rendering, audio, the match session, animations, particles, and the shared AI core (`engine/src/ai/`) |
| `dr-rustario/` | Dr. Rustario's rules (bottle, pills, viruses), its four themes and its AI |
| `rustris/` | Rustris's rules (board, SRS, scoring, garbage), its four themes and its AI |
| `puyo-rusto/` | Puyo Rusto's rules (board, pairs, chains, nuisance), its three themes and its AI |
| `launcher/` | the `dr-rustario-vs-rustris` binary: `shell.rs` (screens), `games.rs` (`AnyGame`), `modes.rs` (playlists) |

Game crates are siblings and never depend on each other; anything shared goes in `engine`.

## Commands

```shell
cargo build --release            # SDL2 links the platform's own way, no flags; see README
cargo test                       # dr-rustario/tests/ai_agent.rs pins the AI difficulties
cargo fmt --all                  # stock rustfmt, no rustfmt.toml
./build-portmaster.sh            # aarch64 handheld port, `portmaster` feature
./build-browser.sh               # wasm via emscripten, `browser` feature
```

Headless render harnesses (no window needed - the way to *see* a change):

```shell
cargo run --example frame_shot -- 640 480 1 out/ [game]   # one frame on every theme
cargo run --example animation_shot                        # a scripted match, one png every N ms
cargo run --example menu_shot                             # menus, walking the theme rows
cargo run --example field_preview                         # particle field: features | sheet | <seconds>
cargo run --example character_shot / kirby_shot           # Puyo mugshot cast / Kirby routines
cargo run --example feature_shots                         # Dr. Rustario network inputs
cargo run --example scale_report                          # where every theme puts its board
```

Each example's doc comment carries its own usage. AI training and measurement is the `ga`
subcommand of the main binary, dispatched in `launcher/src/main.rs` (`ga dr auto|trial|play|
probe|explain|...`, `ga puyo rank|play|duel`, `ga auto|play|...` for Rustris); the readme's
*Training Dr. Rustario* is the walkthrough.

## Architecture

A game implements `engine::game::Game` (headless board of `Cell`s with game-private `CellId`s,
emitting `GameEvent`s) and `engine::render::GameRender`. Its themes are data handed to the
engine's `retro_theme` / `modern_theme` builders in `engine/src/render/`.

**Every match runs through `launcher/src/games.rs`'s `AnyGame` wrapper, and a defaulted trait
method it does not delegate is silently never asked of the game.** This compiles fine and fails
at runtime; the tests in that file are the only thing that catches it. Prefer a new
`GameEvent` (data on the wire, needs no arm) over a new trait method where you can.

Attacks between players cross games: the sender prices the clear in the receiver's units via a
`ForeignPrices` table (`foreign_attack` in each game's `game/mod.rs`), keyed on the ids in
`engine::game::ids`. An unpriced pair is worth nothing and drops.

### AI

`engine/src/ai/` owns the network shapes, genome, genetic algorithm and its `Fitness` seam.
Each game supplies board features, placement search and agent under `<crate>/src/game/ai/`:

* **Dr. Rustario** - every difficulty plays `game/ai/n64/`, a port of Dr. Mario 64's
  deterministic scorer (`params.rs` holds the weights and `SKILL_ORDER`). There is also a
  trained neural model (`ai/models.rs`, `imitation.rs` then `genetic.rs`). `DrAiKind` picks
  between them. `probe.rs` and `explain.rs` are the diagnostics for feature choice and
  trained-model behaviour.

  **Its features are one scan.** For a settled cell, take the four windows of four that
  contain it on each axis; a window is *live* when nothing in it is another colour and every
  empty cell in it is `Grid::reachable`. `work` is the fewest empties any live window has, so
  1, 2 or 3; `buried` is no live window on either axis - buried is work with no answer, which
  is why they are one measurement and why nothing can report a cost it cannot pay. A 4096
  entry `OnceLock` table (six neighbours, two bits each) does it in one index, which is what
  buys measuring *every* occupied cell rather than only the viruses. Nineteen inputs come off
  it, in `evaluator::raw_inputs`.

  **They were selected, not designed**, by `ga dr screen` - a 15 second harness that teaches
  fifty clones in parallel off one corpus, reports the **median** of what they play, and can
  *silence* an input throughout training so a feature is taken away without rebuilding the
  network around a smaller `BOTTLE_FEATURE_INPUTS`. It is deterministic, so every variant is a
  paired comparison. Its statistics are the point: best-of-N reversed the ordering of two
  candidate sets twice, and held-out agreement moves the *opposite* way to play often enough
  to be useless on its own - the set that finally worked has lower agreement than one that
  did not. `ga dr pretrain` is the same thing when you want the weights.

  Four things it measured that are each the opposite of what they look like:

  * **A number is not an indicator.** `place.halves_work` says the better placed half is one
    block short; `halves_one_short`/`halves_two_short` say it as counts. Feeding the pair as
    well as the number is worth **+763** on the median, and neither is worth anything without
    the other - drop either alone and nothing moves, drop both and it all goes.
  * **The tallest column is the wrong height.** `delta.max_height` costs 201 to feed;
    `delta.landing_height` - the *shortest* column, the lowest a pill can still be put - is
    worth **+337**. One virus on the floor makes the tallest column 1 and changes nothing at
    all, because every other column is still open to the floor.
  * **More inputs are worse.** A 26 input superset of these same measurements scores a median
    of 3222 against the nineteen's 3814; an earlier one scored 644 where its own best eight
    scored 2275. An input the network has to learn to ignore is not free.
  * **The two cells the pill put down have to be fed on their own.** The bottle-wide work sums
    add up forty-odd blocks, so the two the decision is about are a twentieth of the number
    and are conflated with every other block the placement touched.

  **What was tried and dropped**, all measured the same way: a one ply lookahead (`one_away`,
  `one_away_virus`, and its `chains`) cost 433 and its bottle clone per candidate with it;
  `halves_over_virus` cost 1788; `patterns_cleared` cost 18, because the board deltas already
  say what a clear did; `entrance_height` and `holes` never earned a place and are the probe's
  control group instead. Two hypotheses about *why* the small sets were losing were also tested
  and are both false: they are not under-trained (10, 20, 40 and 80 epochs are flat past 20),
  and the thirty two input set they replaced is not merely winning because it was reverse
  engineered from the same N64 scorer it is taught by - with the N64 out of the loop entirely,
  `ga dr trial 60 25 scratch` had it ahead 672 viruses to 444.

  **A `ga dr auto` over these features gains nothing, and the reason is the fitness's own
  noise.** 100 generations from a taught seed of 4008 viruses on the unseen block went 4085,
  4080, 4173, 3558 at its four checkpoints - no trend, and the last one below the seed it
  started from. `ga dr diagnose` plays one model on five seeds and gets 723, 873, 854, 833,
  875, so a seed is worth about +-8%; `SEEDS_PER_GAME` is 4, which puts the standard error on
  a candidate's fitness near 4%, and 4008 to 4173 *is* 4%. Selection is picking the luckiest
  four seed draw rather than the better player, which is what the end-of-run playoff and
  `Fitness::confirm` exist to catch and cannot fix mid-run. It looks different from the old
  runs only because stage one now lands where a whole training run used to: the previous model
  entered stage two at 2163 with 100% of headroom, far above the noise floor, and this one
  enters at 4008 with a few percent left under it. Raising `SEEDS_PER_GAME` is the lever -
  8 halves the error and roughly doubles a generation, with `PROBE_SEEDS` clawing some back.

  `BOTTLE_FEATURE_WIDTH` is separate from the input count and is 21, which is where a sweep
  from 9 to 32 flattens. Watch it against [`engine::ai::NEURAL_GENOME_SIZE`]: `Genome` is keyed
  on nothing but its length, so if the two `feature_network!` shapes ever total the same number
  of weights the second will not compile. Rustris is 1281 and this is 1366.

* **Rustris** - a small neural network, weights embedded in `game/ai/models.rs`.
* **Puyo Rusto** - `game/ai/beam.rs`, a beam search over `eval.rs`'s fifteen weights with a
  quiescence search (`quiet.rs`); no neural model, and none ever - the search is the ai. Its
  difficulty ladder (`skill.rs`) is *measured* by `ga puyo rank`, not assumed. Built from the open
  literature, mostly [ama](https://github.com/citrus610/ama).

  **It is the one ai here that reads what is being thrown at it**, and what that turned out to
  be worth is not what it looks like. Two rules came out of `ga puyo duel`, and only one of
  them earns its place:

  * **Firing because the tray is about to bury you is worth having.** `agent.rs`'s
    `incoming_rows` adds what is queued to the spawn column's height, so a comfortable board
    with a rock hanging over it counts as `pressed` and fires now - there is no *later*, since
    classic Tsu offset drops the tray the moment this pair locks. It fires on **half** the
    trays a row sees, and both sides of a duel last longer and chain more for it.
  * **Firing the smallest chain that cancels the tray is nearly dead code.** It is right - a
    partial answer buys nothing, because the remainder drops anyway - but `Candidate::fires`,
    what the pair in play can set off *right now*, is zero at most of the moments a tray is
    seen and far short of the 2,000-9,000 points a real tray costs at the rest. Over three
    thousand duelled pairs it fired **never**. `SearchConfig::answer_at` is the dial and it
    measures flat from 1 to `u32::MAX`.

  **The ladder under fire is not the ladder in a marathon**, and roughly reverses at the top:
  see `SKILL_ORDER`'s doc comment. `ga puyo rank` is a solo marathon and takes no nuisance at
  all, so it can only ever measure building.

The search is stepped once a frame and always interruptible (`agent.rs`), so a piece keeps
falling while it thinks. **Only Puyo Rusto's ai reads the pending-attack tray**; Dr. Rustario's
and Rustris's do not, which is why their duels are one-sided.

### Rendering and themes

Every theme of every game is built at startup in `Shell::new` and stays built (the title
screen's sprite race draws from all of them), behind `engine::app::loading`'s progress bar.
That memory bill is why `dr-rustario/build.rs` halves the particle Dr.'s sheets for the
`portmaster` and `browser` builds. Cell size is set by the largest all of a game's themes can
hold, so panel dimensions are load-bearing.

Two particle models: `engine/src/particles/source.rs` is fire-and-forget (every foreground
effect, every menu); `engine/src/particles/field/` is a retained pool that owns particles for
the life of a match and reacts to a `SceneContext`, driven by `field/director.rs`. It never
touches game state.

`engine/src/animate/` holds the things that move that a board does not - `bounce`, `debris`,
`popup`, `tray`, `nuisance`, `attack_ball`, `character`. Each module's doc comment explains
what it is and what feeds it.

## Art and audio

**The rips these are cut from are not in the repository.** Re-run the script rather than
hand-editing its output:

* `puyo-rusto/art/rip.py` - particle theme puyos; `check` writes an alignment board
* `puyo-rusto/art/rip_retro.py` - `genesis` and `snes` sheets, panels, vignettes
* `puyo-rusto/art/retro_audio.py`, `music.py`, `sfx.py` - music and effects
* `puyo-rusto/art/mugshots.py`, `kirby.py` - characters; both print the Rust table to paste back
* `puyo-rusto/art/sprites.py` - the procedural art the rip replaced, kept as a description of
  what the sheet must contain
* `dr-rustario/art/build_doc.py` - the feature-reference page
* `engine/art/audio_levels.py` - the whole app's audio meter; see below

Retro theme geometry was measured against the emulated games, not read off the rips; the
numbers live in each theme module beside a comment saying what they were measured from.

### The house audio levels

**Loudness is RMS, and every theme in the app is levelled to one baseline**: its music at
**-22 dBFS RMS** with its effects within about four decibels of that (`rustris/gb`, the balance
Alex reads as right, is -2.0), and no file peaking over -0.5 dBFS. `engine/art/audio_levels.py`
decodes every embedded ogg, applies the gains the Rust adds, and prints what is out of band -
run it after cutting *any* new audio.

Two knobs, and they are different things. `AudioTheme::with_gain` levels a whole theme, music
and effects together, so the mix its source was mastered with survives; `with_effects_at` says
that a theme's effects sit wrong **against its own music**. Puyo Rusto's rips are mastered some
eight decibels hotter than the rest of the app and are trimmed with the first
(`puyo-rusto/src/theme/data.rs`); Rustris's particle theme was five decibels quiet against its
own tune and is lifted with the second. A gain over 100 scales the decoded samples, since the
config's volume dial has no headroom above it.

**Never level a set slot by slot on peaks.** Peak is one sample: two effects that both peak at
-0.5 dBFS are ten decibels apart if one is a click and the other a chord. That is what made Mean
Bean Machine's effects measure right and sound hot, and `retro_audio.py`'s `slot_gain` now
matches RMS with the peak only as a cap.

## Docs

* [docs/puyo-puyo-plan.md](docs/puyo-puyo-plan.md) - shared memory for Puyo Rusto, which is
  built bar three named items. **Read it before touching that crate.**
* [docs/puyo-nexus-rules.md](docs/puyo-nexus-rules.md) - local copy of the wiki's rule pages.
  Search it before implementing a Puyo rule, and search for the mechanic rather than the page
  you expect it on.
* [docs/particle-scenes.md](docs/particle-scenes.md), [docs/next-game-ideas.md](docs/next-game-ideas.md)
