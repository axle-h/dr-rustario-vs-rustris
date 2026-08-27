# Puyo Puyo — implementation plan

The third game for the compendium, picked in [next-game-ideas.md](next-game-ideas.md).
This document is the *how*: the phases, their status, and the notes each agent hands to the
next. It is the shared memory for this piece of work — read it top to bottom before starting.

## Why this game

Puyo Puyo is the canonical 2-player falling block battle game and it fits this codebase
better than anything else considered. Its pair piece is mechanically Dr. Rustario's pill —
two halves, rotation about a pivot with kicks, splitting apart when it lands — so the hardest
code in that crate has a sibling to be written against. What it adds that neither existing
game has is a real attack economy: a chain's score converts to nuisance puyo through a
documented power table, and the **offset rule** lets an incoming attack be cancelled by
chaining back at it, which is what turns two people racing into two people fighting.

## Decisions already taken

* **Fidelity.** Faithful Tsu *between two Puyo players* — the real chain power, colour and
  group bonus tables, target points, the nuisance queue and classic Tsu offset. Cross-game
  attacks are **tuned down** so a Puyo chain does not overwhelm a Rustris or Dr. Rustario
  player; that pricing is measured, not guessed (phase 5).
* **Scope.** A full citizen of the compendium: four themes, an ai offering the same four
  difficulties and the same two demo models as the other games, high score tables, menus, and
  three-way attack pricing.
* **Crate name.** Alex's call, not settled. `rusto-rusto` (displayed "Rusto Rusto") is the
  placeholder; `oxo-oxo` and `rusty-beans` are the alternatives. Whoever starts phase 1 asks
  before creating the directory. It takes `GameId(3)`.
* **Sources.** The exact tables come from Puyo Nexus at the time of writing the code — see
  the links in [next-game-ideas.md](next-game-ideas.md#sources). Those pages reject automated
  fetches; read them in a browser. Do not guess a table.

## Sizing

Each existing game crate is around 9-10k lines (`dr-rustario` 10,054; `rustris` 9,053),
split roughly: rules 3,800-4,200, ai 2,000-4,000, theme Rust about 950, glue about 400. The
art and audio is the real expense — `dr-rustario` ships 26 MiB of embedded assets and
`rustris` 7.6 MiB. Launcher changes for a third game are about 300 lines plus a substantial
test rewrite. The engine needs only the three small changes in phase 0.

---

## Phase 0 — generalise the engine and launcher past two games

**Status:** `todo`

**Goal.** Nothing in the engine or launcher assumes the game count is two, with both existing
games unchanged in behaviour. Do this before the new crate exists so the crate is built on
clean foundations.

### The one silent-bug risk

`engine/src/game/mod.rs` carries a single `foreign: u32` on `Attack`, and `strength_for` is a
two-way branch — mine, or not mine:

```rust
pub fn strength_for(&self, receiver: GameId) -> u32 {
    if receiver == self.origin { self.strength } else { self.foreign }
}
```

With three games a Rustris tetris would send the same `2` to Dr. Rustario (garbage blocks)
and to Puyo (nuisance puyo), **and it compiles**. This is the only place a third game causes
a balance bug with no compiler error, so deal with it first.

Replace `foreign` with a small `Copy` price table indexed by `GameId`:
`with_foreign_for(receiver, price)` to author, `strength_for(receiver)` to read. That keeps
every existing measured number intact and makes each new pair of games a deliberate decision.
It is O(n²) authoring, which is the honest cost of this project's own principle that only the
sender knows what a clear took — a neutral "work unit" currency would be O(n) but would throw
that principle away.

Make the default price **zero**, so a forgotten pair drops the attack (the session already
drops zero-strength attacks in `engine/src/session.rs`) rather than sending wrong units.
Note that `Attack::new` currently defaults `foreign` to `strength`, which is the opposite of
what is wanted here.

Then update the two senders: `foreign_attack` in `rustris/src/game/mod.rs` and in
`dr-rustario/src/game/mod.rs`.

### Two closed enums grow

* `MetricKind` in `engine/src/game/mod.rs` is `{ Score, Level, Lines, Viruses }`, with
  `metric_label` in `engine/src/render/metrics_table.rs`. Puyo wants a puyos-cleared or
  max-chain counter — add one variant.
* `words::ALL` in `engine/src/particles/field/reaction.rs` — add `CHAIN`. Words are outlined
  ahead of time by `ParticleRender::build_captions` in `engine/src/particles/render.rs`, and
  a word that was never outlined is silently dropped, so this must be done before any game
  returns it from `clear_word`.

### Launcher — sites that break at compile time

These are the checklist; adding an enum variant surfaces them all.

* `launcher/src/games.rs` — `GameKind`, `AnyGame`, the `delegate!` macro (one new arm covers
  about thirty trait methods), `AnyGame::kind()`
* `launcher/src/modes.rs` — `Themes::range`, `Themes::race`, `PlaylistThemes::theme`,
  `VersusMode::game_seed`, `VersusMode::new_games`, the versus ai controller dispatch
* `launcher/src/shell.rs` — `ModeChoice`, its `ALL`, `mode()` and `choose_mode`

### Launcher — sites that will *not* break, and must be found by hand

* `let order = [GameKind::Rustris, GameKind::DrRustario];` in `Playlist::fixed_stages`
* `first_game`'s `_ => GameKind::Rustris` fallback
* `random_game`'s `stage_roll(..) & 1` coin flip — becomes a modulo over the game count
* `_ => Some(2 * themes.slots())` in `stage_count` — that `2` is games-per-slot

### Launcher — structural changes

* `Themes` holds one `Range<usize>` field per game plus a `race_all()` that concatenates
  exactly two calls; make it a collection keyed by `GameKind`. It is built in `Shell::new`.
* `PlaylistThemes` holds two named `Vec<usize>` fields and
  `slots() = dr_rustario.len().min(rustris.len())`; make it a collection and `slots()` a min
  over all games.
* `VersusAi::brains()` returns `Vec<(u32, Duration, DrAiKind, TetrisNeuralNetwork)>` by
  zipping two `ai_players()` lists — it becomes a three-way zip, and the controller gains a
  third brain.
* `Difficulty` fans the single 0-10 dial out to per-game settings; add the Puyo level and
  speed pair.
* The playlist tests encode two-game stage sequences throughout and are the bulk of the
  mechanical work in this phase. Expect to rewrite essentially all of that module.
* The examples enumerate games by hand too: `field_preview.rs` hardcodes `GameId(1)` and
  `GameId(2)` and a `dr|rustris|mixed` argument, `menu_shot.rs` uses `[Vec<MenuItem>; 2]`,
  and `frame_shot.rs` and `scale_report.rs` list both games.

### What needs no work

High scores are string-keyed on `{game, mode, ranking}`, so a new game simply adds top-level
keys to `high_scores.yml` and old files load unchanged. The particle field is already n-way:
`SceneContext::games` is a `Vec<GameId>`, and silhouettes are keyed by theme index and come
free from any theme's block sprites via `ShapeBank`.

### Do not rename the binary

`shell.rs` and `modes.rs` hardcode "Dr. Rustario vs. Rustris" and the config path derives from
the crate name. Renaming orphans every existing `high_scores.yml` and `config.yml` on every
installation. The third game joins the compendium; it does not retitle it.

**Done when:** `cargo test --workspace` passes, both existing games behave identically, and a
search for the game count turns up nothing that assumes two.

### Handover notes

_(to be filled in by the agent that completes this phase)_

---

## Phase 1 — rules, headless

**Status:** `todo` — blocked on phase 0

**Goal.** The rules simulate correctly with no rendering at all, checked by unit tests against
known positions.

Mirror the existing crate shape (`dr-rustario/src/lib.rs` is six lines):

| file | what |
|---|---|
| `game/mod.rs` | the `engine::game::Game` impl and the state machine |
| `game/board.rs` | the grid, connectivity, popping, settling, the chain loop |
| `game/pair.rs` | the two-puyo piece: rotation, wall and floor kicks, the double-rotate quick turn, halves splitting once locked |
| `game/nuisance.rs` | the queue, offset, and the drop pattern |
| `game/score.rs` | the chain power, colour bonus and group bonus tables |
| `game/random.rs` | the seeded colour sequence |
| `game/rules.rs` | `GameConfig`, `MatchThemes`, `AiDifficulty`, `AiMode`, `ai_players()` |
| `game/cell.rs` | `GameId(3)`, the `CellId` and `PieceId` space |

The random sequence **must be reproducible from a seed**: every player in a match is dealt the
same game from one seed, however far apart the playlist has moved them.

### The ruleset

Faithful Tsu, with the exact tables sourced rather than guessed.

* Six columns by twelve visible rows plus hidden rows above; the pair spawns in column three.
* The pair rotates about its pivot with wall kicks, floor kicks and the double-rotate quick
  turn against a wall. Once it lands the halves split and fall independently.
* Pop every orthogonally connected group of four or more of a colour, settle under gravity,
  and repeat. Each iteration of that loop is one chain step.
* Score for a step is `10 × puyos_cleared × clamp(chain_power + colour_bonus + group_bonus, 1, 999)`.
* Nuisance is score divided by 70 target points, with the remainder carried between
  placements rather than discarded.
* **Classic Tsu offset**: outgoing nuisance cancels whatever is pending against you before any
  is sent on, and whatever still waits drops as soon as your chain finishes — so you generally
  get exactly one chain to answer an attack. Cap a single drop at 30 (five rows): full rows of
  six first, the remainder scattered.
* An all clear (zenkeshi) carries its bonus.
* Leave margin time out for now; note it in the handover as a possible difficulty knob.

### Fitting the engine's stage model

Puyo has no natural level, so map stages onto speed: a stage is a speed level with
`StageTransition::Seamless`, the way Rustris advances every ten lines, triggered by a
puyos-cleared count. `speed_index` drives fall speed and the colour count (four of five by
default).

**Done when:** unit tests build known fields, fire known chains, and match the documented
score and nuisance counts exactly. That test is the thing that makes "faithful" checkable
rather than asserted.

### Handover notes

_(to be filled in by the agent that completes this phase)_

---

## Phase 2 — playable on the particle theme, human players

**Status:** `todo` — blocked on phase 1

**Goal.** Two people can sit down and play a Puyo match from the menu.

`render.rs` implementing `GameRender` (only `name` and `spawn_cells` are required; add
`clear_class` and `clear_word`), the `modern` particle theme in **original art**,
`options.rs`, a `Mode` impl in the launcher of about 110 lines modelled on `DrRustarioMode`,
a `ModeChoice` variant, and the high score tables.

Two engine contracts to honour:

* `clear_class` grades the chain 0..3, with **3 reserved for the biggest chain**. The particle
  field relies on every game grading its largest clear as class 3 for the big-clear silhouette
  interrupt to fire — see `TETRIS_CLEAR_CLASS` in `rustris/src/render.rs`. Every theme then
  supplies a sound per class it can return.
* `clear_word` returns the new `CHAIN` on a long chain and `PERFECT` on an all clear.

The particle theme needs only `sprites.png`, the mascot strips and the oggs, plus a
`particle_color` and a `particle_palette` — its background, board frame, HUD and cards are all
drawn procedurally. Template: `dr-rustario/src/theme/modern/mod.rs`.

**Done when:** Puyo is selectable from the pre-menu and playable by two humans on the particle
theme, with high scores recorded, and the particle field picks up its pieces and mascot.

### Handover notes

_(to be filled in by the agent that completes this phase)_

---

## Phase 3 — retro themes

**Status:** `todo` — blocked on phase 2

**Goal.** Three retro themes alongside the particle one, so Puyo has the same four as the
other games and can take its turn in the retro playlist.

| module | source | notes |
|---|---|---|
| `genesis` | Dr. Robotnik's Mean Bean Machine | mascot art available |
| `snes` | Kirby's Avalanche | mascot art available |
| `sms` | Mean Bean Machine on Master System / Game Gear | third retro theme |

Everything is `include_bytes!` — there is no build script, no asset manifest and no slicing
tooling. Every rect is arithmetic written by hand in the theme's `mod.rs`. Per retro theme,
in a directory beside that file:

* `sprites.png` — one `source_block_size` grid holding the cells, the idle and pop strips and
  the previews
* `background.png`, and `board.png` either as one frame or as N frames side by side selected
  by `board_snips`, one per speed band
* `background-tile*.png` if the theme uses `SceneType::Tile`, one per speed band
* `game-over.png` / `match-end.png` overlays, positioned by `game_over_points` and
  `interstitial_points`
* `font.png` — ten digits in a row for `FontRenderOptions::numeric_sprites`, or digits plus
  letters sliced by a closure the way `rustris/src/theme/data.rs` does it
* mascot strips `{idle,throw,victory,game-over}.png`
* music and about twelve sound effects as **OGG Vorbis at exactly 44,100 Hz, mono or stereo**
  — the decoder rejects anything else outright. Music may be split into `-intro.ogg` and
  `-repeat.ogg` for the intro-then-loop chaining.

Template for the 34-field `RetroThemeOptions`: `dr-rustario/src/theme/nes/mod.rs`.

Register each theme in three places: `theme/mod.rs::all_themes` (the order defines the theme
sprint), the `MatchThemes` enum in `game/rules.rs`, and `theme_mode()` in `options.rs`.

**Done when:** `frame_shot` renders every theme correctly, `menu_shot` walks the theme rows,
and `field_preview sheet` outlines the new sprites cleanly.

### Handover notes

_(to be filled in by the agent that completes this phase)_

---

## Phase 4 — the ai

**Status:** `todo` — blocked on phase 3

**Goal.** Puyo fields the same four difficulties and the same two demo models as the other
games. This is not optional: `VersusAi` cannot deal a Puyo board without it, and there is a
test asserting every mode offers identically named difficulties.

Follow the Dr. Rustario precedent — a deterministic scorer that actually plays, with a neural
model alongside it, dispatched through a `PuyoAiKind`.

1. **A hand-written evaluator with a shallow beam search** over the current pair and the
   queue. Features from the literature and the open source bots: chain potential (the largest
   chain the field could fire), height and bumpiness, edge and corner penalties (a puyo
   against the wall can link to fewer neighbours, a corner fewer still), spawn column
   clearance, buried nuisance, link counts. The difficulty ladder is weight sets × search
   depth × the engine's existing `KeyPacer` delays (500/400/300/0 ms), so that a harder
   setting is a better player and not merely a faster one — the standard `SKILL_ORDER` sets
   for Dr. Rustario. **Rank the weight sets by measurement, not assumption.**
2. **A neural model** through `feature_network!` over the same features, trained by
   `ga puyo auto` on the existing `Fitness` seam. It ships only when it beats the scorer;
   until then the scorer plays, exactly as `DrAiKind` works today.

Add `ga puyo play <seed> <level> <cap> <every> <brain>` mirroring `ga dr play`, so strength
can be measured headlessly.

**Done when:** the four difficulty names match the other games exactly, the 1- and 2-player
demos run, and the weight set ranking is recorded in the handover notes with the numbers
behind it.

### Handover notes

_(to be filled in by the agent that completes this phase)_

---

## Phase 5 — vs. integration and attack pricing

**Status:** `todo` — blocked on phase 4

**Goal.** Puyo takes its turn in every vs. playlist, and garbage crosses at sane volumes in
all six directions.

Playlists deal three games, `VersusAi` fields three brains, and — the real work — the **six
directed attack prices** are set. Measure rather than guess, the way the README's existing
table was built: run each game's own ai for the same protocol (five seeds at full speed for
fifty minutes of game time, counting what it sent), then hand-tune *down* from the measurement
so a Puyo chain does not bury a Rustris or Dr. Rustario player. Extend the README's measured
table from three rows to the six directed prices.

Starting intuitions to test, not to ship: a four-chain is roughly the work of a tetris;
routine two-chains are what a Puyo player throws constantly and should cross for little or
nothing.

**Done when:** a 2-player vs. match on each playlist has the three games taking turns, garbage
crossing sensibly in all six directions, and the README table updated with the measurements.

### Handover notes

_(to be filled in by the agent that completes this phase)_

---

## Verification

Across all phases:

* `cargo test --workspace` throughout. The playlist tests in `launcher/src/modes.rs` are the
  canary for phase 0; `every_mode_offers_the_same_ai_opponents_and_demos` and
  `ai_difficulties_agree` gate the menu surface from phase 2 onward.
* Headless rule tests against known Puyo positions for phase 1 — the check that "faithful"
  actually holds.
* `cargo run --example frame_shot -- 640 480 1 out/ puyo` — one frame of a match on every
  theme, which is how theme geometry is checked without a display.
* `cargo run --example menu_shot -- 960 720 out/` — walks the theme and mode rows.
* `cargo run --example field_preview -- sheet out/` — outlines every sprite of every theme, to
  confirm the new ones silhouette properly; `features` and `<seconds>` for the routines.
* `ga puyo play ...` for ai strength, and the five-seed protocol for the attack prices.
* Finally, play it. A 2-player match on each playlist.

---

## Working agreement

Work on this repository is **synchronous. One agent at a time. Never in parallel.**

* **This document is the shared memory.** Conversations do not carry over between agents; this
  file does. Read it top to bottom before starting, including every handover note.
* **Phase status lives here.** Each phase carries `todo`, `in progress`, `done` or `blocked`,
  updated **in the same commit as the work it describes**, so the document and the code never
  disagree.
* **One phase at a time, in order.** Do not start a phase whose predecessor is not `done`.
  They are sequenced because each one's assumptions are the previous one's output — phase 1's
  crate cannot compile before phase 0's launcher accepts a third game, and phase 5's pricing
  is meaningless before phase 4's ai can play well enough to measure it.
* **If blocked, mark it `blocked` with the reason and stop.** Do not route around a blocked
  phase and do not start a later one instead. Surface it to Alex.
* **Every phase ends with handover notes** in its own section: what actually changed, what
  deviated from this plan and why, decisions taken that the plan did not anticipate, and any
  measured numbers. The next agent's first job is to read them.
* **Amend, do not append contradictions.** If a phase's plan turns out to be wrong, edit that
  section and note in the handover that it was edited. A document that argues with itself is
  worse than no document.
* **Stay inside this game.** While these phases are open, nobody starts another game from
  [next-game-ideas.md](next-game-ideas.md) — see the status board there.
