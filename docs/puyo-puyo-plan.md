# Puyo Puyo — implementation plan

The third game for the compendium, picked in [next-game-ideas.md](next-game-ideas.md).
This document is the *how*: the phases, their status, and the notes each agent hands to the
next. It is the shared memory for this piece of work — read it top to bottom before starting.

Reviewed and amended on 2026-08-27, before phase 0 began. That pass added the connected-puyo
sprite decision, the pending-nuisance surface in phase 0, the top-out rule and the chain event
grammar in phase 1, and the stub `ai_players()` in phase 2; it corrected the colour count,
which an earlier draft had driven by `speed_index`, and narrowed phase 0 to work that can be
done before the crate exists. Each of those is amended in place, in the phase it belongs to.

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
* **Crate name: `puyo-rusto`**, displayed "Puyo Rusto". Settled with Alex at the start of
  phase 1, out of `puyo-rusto` and `rusto-puyo`: the other two names in the compendium keep the
  original title recognisable and let the rust land on the second beat, and "Puyo Puyo" is a
  reduplication, so replacing the echo is where the joke goes. It takes `GameId(3)`, declared
  as `engine::game::ids::PUYO`.
* **Sources.** The exact tables are Puyo Nexus's. **[puyo-nexus-rules.md](puyo-nexus-rules.md)
  is a local copy of every page of that wiki that carries a rule** — search it first. It exists
  because the browser-only route below only finds the pages you think to look for: phase 1
  implemented the hidden thirteenth row wrong because the ghost puyo rule is filed under
  *Gameplay Guides* rather than `Category:Rules`, and Alex had to catch it.
  The live wiki is still the authority and those pages reject automated fetches, so read them
  in a browser when the local copy looks stale or thin — see the links in
  [next-game-ideas.md](next-game-ideas.md#sources). Do not guess a table.
  **That local copy is scaffolding and comes out once the rules are implemented**, so cite the
  live page in code comments rather than the copy.
* **Connected puyos are a `CellId` encoding, not an engine change.** Puyos of a colour that
  are orthogonally adjacent are drawn joined — the signature look of the game, and the thing
  that tells a player at a glance what is linked to what. That is a *sprite* concern and the
  engine already supports it: a `CellId` is a game-private key the game recomputes whenever it
  likes, and Dr. Rustario already rewrites one in place when a pill half is orphaned
  (`set_garbage` in `dr-rustario/src/game/bottle.rs`). So a Puyo `CellId` carries its colour
  *and a four-bit mask of which neighbours match*, and `board.rs` recomputes the masks of
  every affected cell after each lock, pop and settle. Details in phase 1; art cost in phases
  2 and 3. The falling pair draws unlinked (mask 0) and nuisance never links.
* **No hold; hard drop stays.** Tsu has neither, but the engine's input model, every pad
  mapping and the ghost piece are built around hard drop, and a Puyo without it would feel
  broken next to the other two games. `hold()` is a no-op — adding it would change the
  balance of the game and widen the ai's search for no fidelity gain. Both are legal against
  `engine::game::Game`; this is a decision, not an accident, and the menu should not offer a
  hold box on a Puyo board.
* **The colour count is fixed for a whole match.** Not driven by `speed_index` — see the
  divergence trap in phase 1. It is set once, from the vs. difficulty dial or the options
  menu, both of which are match-wide.
* **Cross-game garbage arriving at Puyo joins the nuisance queue.** A Rustris or Dr. Rustario
  attack lands in the tray like any other, is visible, is offsettable by chaining back, and
  drops when the chain finishes — rather than applying immediately the way the other two games
  take a hit. Offset is the identity mechanic of this game and it would be strange for it to
  work against one opponent and not another. It does make a tetris less frightening than its
  raw number suggests, which phase 5's pricing has to account for.
* **`pair.rs` is a sibling of `pill.rs`, not an extraction from it.** The two pieces rhyme —
  two halves, a pivot, kicks, splitting on lock — but the kick tables, the double-rotate quick
  turn and what happens to the halves afterwards all differ. A shared engine pair-piece would
  be all parameters and no substance. Read `pill.rs` for the shape, then write the other one.

## Sizing

Each existing game crate is around 9-10k lines (`dr-rustario` 10,054; `rustris` 9,053),
split roughly: rules 3,800-4,200, ai 2,000-4,000, theme Rust about 950, glue about 400. The
art and audio is the real expense — `dr-rustario` ships 26 MiB of embedded assets and
`rustris` 7.6 MiB. Launcher changes for a third game are about 300 lines plus a substantial
test rewrite. The engine needs the changes in phase 0: three small ones — the attack price
table and the two closed enums — and one that is only small if it is answered the cheap way,
the surface that draws the pending nuisance.

---

## Phase 0 — generalise the engine and launcher past two games

**Status:** `done`

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
  `metric_label` in `engine/src/render/metrics_table.rs`. Puyo wants a max-chain counter — add
  one variant. **Name it `Chain`, not anything Puyo-flavoured**: this is a closed engine enum,
  and Puzzle Fighter and Bombliss — both queued behind this game — want the same counter.
* `words::ALL` in `engine/src/particles/field/reaction.rs` — add `CHAIN`. Words are outlined
  ahead of time by `ParticleRender::build_captions` in `engine/src/particles/render.rs`, and
  a word that was never outlined is silently dropped, so this must be done before any game
  returns it from `clear_word`.

### The pending nuisance indicator — an engine decision to take here

Puyo's battle play depends on *seeing* what is hanging over you: the row of nuisance icons
above the board is how a player knows whether to answer an attack or take it. The engine has
nowhere to put that. `GameRender` offers only `name`, `clear_class`, `clear_word`,
`spawn_cells` and `stage_intro_cells`, and the only number surface a game has is `MetricKind`.
The queue itself lives inside the game — `receive_attack` hands it over and the game holds it
— but nothing draws it.

Two ways out, and this phase picks one rather than leaving phase 2 to discover the problem:

* **A "Pending" `MetricKind` row.** Nearly free, and honest. It is a number in the HUD rather
  than the icons the game is known for, which undersells the mechanic.
* **A small engine concept for an attack-queue strip** — a game reports a pending-attack count
  and the theme draws it from its own sprites. More work, but Puzzle Fighter's countdown
  counter gems want exactly this surface, so it would not be built once and used once.

Take the second if the appetite is there; the first is not a wrong answer. Either way, record
which in the handover notes, because phase 2's theme work depends on it.

**Decided (Alex, before the work started): the second.** It is built - see the handover notes
for the shape it took and for the one thing that turned out cheaper than this section expected,
which is that it costs the themes no new art at all.

### What belongs to later phases, not this one

Two bullets that read like phase 0 work cannot be done until the crate exists, and the agent
doing this phase should not go hunting for them:

* `VersusAi::brains()` becomes a three-way zip and its controller gains a third brain — but
  the third brain is phase 4's. Here, make the zip and the controller *n*-way over whatever
  games exist, with two entries in the list.
* `Difficulty` gains the Puyo level and speed pair — but what those values should be is phase
  5's measurement. Here, make the fan-out keyed by game rather than hardcoded to two.

Same principle throughout this phase: **shape now, third entry later.** Nothing in phase 0
should mention Puyo by name.

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
  zipping two `ai_players()` lists — make it *n*-way over the games that exist, so phase 4
  adds a brain rather than a dimension. See "what belongs to later phases" above.
* `Difficulty` fans the single 0-10 dial out to per-game settings — key that fan-out by game
  instead of hardcoding two. The Puyo numbers themselves are phase 5's.
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

Done on 2026-08-27. `cargo test --workspace` passes (553 tests, 0 failures) and the four
examples all still run and render as they did. Nothing in the engine or launcher names Puyo.

**The attack price table.** `Attack::foreign` is now a `ForeignPrices` - a flat `[u32; 8]`
keyed by `GameId`, `Copy`, defaulting to zero - with `with_foreign_for(receiver, price)` to
author and the unchanged `strength_for(receiver)` to read. `Attack::new` no longer copies
`strength` into it, so an attack is worth *nothing* abroad until somebody prices the crossing,
and `Match::send_attack` drops it. `ForeignPrices::GAMES` is 8, not 3: raise it rather than
renumbering games. An id past the end trips a `debug_assert` when authored and reads back as
zero, which is the same safe default as an unpriced pair.

Both senders now take the receiver: `foreign_attack(receiver, ...)` in
`rustris/src/game/mod.rs` and `dr-rustario/src/game/mod.rs`, each an early return for a game
it does not price. Every existing measured number is intact - the tests that pin them were
rewritten to read `strength_for` rather than the old field, not relaxed.

**Game ids moved to the engine**, as `engine::game::ids::{DR_RUSTARIO, RUSTRIS}`; each crate's
`GAME_ID` re-exports its own, so nothing else changed. This was forced rather than chosen: a
game pricing an attack has to *name* the game it is crossing to, and the game crates are
siblings that do not depend on each other, so `dr-rustario` cannot say `rustris::…::GAME_ID`.
**Puyo adds `PUYO: GameId(3)` there**, and `dr-rustario` and `rustris` each gain an arm in
their `foreign_attack` for it - which is phase 5's number, but phase 1 will want the id.

**Two closed enums grew:** `MetricKind::Chain` (labelled `"Chain"` in `metrics_table.rs`; both
existing games return `None` for it) and `words::CHAIN` in `reaction.rs`, with `ALL` widened to
6 so it is outlined ahead of time.

**The pending nuisance indicator is an engine attack-queue strip**, per Alex. It came out much
smaller than this phase expected, because of one decision: the game reports what is queued as
its own `CellId`s -

```rust
fn pending_attacks(&self) -> Vec<CellId> { vec![] }   // on engine::game::Game
```

— and the theme draws them with `BlockSpriteSheet::draw_cell`, the same call the board uses.
So **the strip costs the themes no new art**: a Puyo theme already has a nuisance puyo sprite,
and that is the icon. Phases 2 and 3 owe it nothing beyond deciding where it goes. The default
is an empty `Vec`, so neither existing game changed and no game is obliged to have one.

Where it goes:

* a retro theme authors a `PendingLayout { point, step, size, max }` in background source
  pixels (`step` may be negative to fill leftwards or upwards) - a new `pending` field on
  `RetroThemeOptions`, `None` in all six existing retro themes.
* a particle theme just says how many fit, `pending_max` on `ModernThemeOptions` (`0` in both
  existing ones), and `modern_theme` places the strip itself: a cell to an icon, along the top
  of the playfield in the gap above the board. That gap is `top_slack` - room the window is
  allowed to crop - so a theme with a strip now keeps back exactly the strip's height and lets
  the rest go as before. A theme with `pending_max: 0` gets the identical `top_slack` it did
  before, which is why the existing frames are pixel-for-pixel unchanged.

`Theme::draw_pending` runs inside `draw_background`, next to the queue and hold. There is no
game exercising it until phase 2, so the arithmetic is unit tested directly
(`PendingLayout::slots`) rather than left to be discovered later.

**Launcher.** `GameKind` gained `ALL`, `COUNT`, `index()`, `name()` and a second list,
`RUNNING_ORDER`. Two lists because they are two different things and a third game joins both:

* `ALL` is the order games are *numbered* - the key of every per-game collection, and the
  order the themes are built into the shared list.
* `RUNNING_ORDER` is the order they are *billed* - the pre-menu's list and the turns a fixed
  playlist takes. It opens on Rustris, which is presentation, not numbering.

A test holds them to being the same games. Everything that named the two games by hand is now
`PerGame<T>` (`launcher/src/games.rs`): `Themes`' ranges, `PlaylistThemes`' slots, and the
`Shell`'s modes. `Playlist::stage_count` is `GameKind::COUNT * slots`, `first_game`'s fallback
is `RUNNING_ORDER[0]`, `fixed_stages`' order is `RUNNING_ORDER`, and `random_game` is a modulo
over `COUNT` rather than a coin flip - all of which deal exactly what they dealt before, seed
for seed, because `RUNNING_ORDER` preserves the old index-to-game mapping. `game_seed`'s salt
is `kind.index() + 1`, which is the same 1 and 2 as the hand-written match.

`Difficulty` fans out through `level(game)` - one arm per game - and keeps
`dr_rustario_speed()` as its own thing, because a fall speed dial separate from the level is
Dr. Rustario's own idea and Rustris has no equivalent. **Puyo's arm of `level` and its own
speed dial are phase 5's to measure.**

**The ai went n-way through a trait rather than a wider tuple**, which is the one place this
phase did more than the plan asked. `VersusAi::brains()` returned
`Vec<(u32, Duration, DrAiKind, TetrisNeuralNetwork)>` - a tuple that grows a *dimension* per
game. It now returns `Vec<(u32, Vec<Box<dyn AiBrain>>)>`: an ai player and one brain per game.

```rust
pub trait AiBrain {
    fn act(&mut self, game: &mut AnyGame, delta: Duration);
    fn reset(&mut self);
}
```

A brain handed a board that is not its game does nothing; the controller resets them all when
the playlist swaps the board and then offers the board to each in turn. **So phase 4 adds a
`puyo_brain()` beside `dr_rustario_brain()` and `rustris_brain()` in `games.rs` and one arm to
`VersusAi::ai_players`, and touches the controller not at all.** One small behaviour change
falls out of it: each game's brain now plays at *its own* declared key delay rather than at
Dr. Rustario's, which the old zip used for both. The two games declare identical delays for
every difficulty, so nothing differs today - but it is what the README already promised.

`modes.rs` gained `game_mode(game) -> Box<dyn Mode>`, the single place that names each game's
standalone mode; the shell and the tests both go through it, so a game cannot be added to one
and forgotten in the other. `ModeChoice` is now `Game(GameKind) | Versus`.

**Tests.** `launcher/src/modes.rs`'s test module was rewritten as the plan expected: helpers
`same_themes`, `turns(slot)`, `all_modes()` and `ai_difficulty_names(game)`, and constants
`THEME_SLOTS` / `FIXED_STAGES = GameKind::COUNT * THEME_SLOTS`, so the stage sequences are
asserted as *every game takes a turn per slot* rather than as literal 8s and named pairs. Four
tests were renamed off "both games". Two new ones: `a_versus_ai_player_carries_a_brain_for_every_game`
and, in `games.rs`, `every_game_is_numbered_and_billed_exactly_once`.

**Examples.** Each now has one list to add a game to: `PREVIEW_GAMES` in `field_preview.rs`
(which also killed the hardcoded `GameId(1)`/`GameId(2)` and the `dr|rustris|mixed` argument's
two-game assumption - one game named puts both players on it, anything else deals the first
two), `all_games()` and a local `MenuOptions` trait in `menu_shot.rs` (the games' `Options` are
separate types with no trait between them), and a `GAMES` constant in `scale_report.rs`.
`frame_shot.rs` already dispatched on a game-name argument, so a third game is one arm and one
`fn` there.

**Not done, deliberately:** nothing in the engine or launcher mentions Puyo, per this phase's
own rule. `ForeignPrices::GAMES` has room for it, `engine::game::ids` is where its id goes, and
the four places above each take one entry.

**Watch out for:** another agent was rewriting `dr-rustario/src/game/ai/` throughout this
phase. Nothing here touched those files - the only Dr. Rustario changes are `game/mod.rs`
(`foreign_attack`, the `MetricKind::Chain` arm), `game/cell.rs` (`GAME_ID` re-export) and
`pending: None` in the four themes. `cargo fmt --all` was run and rewrote only this phase's own
files.

---

## Phase 1 — rules, headless

**Status:** `done`

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
| `game/rules.rs` | the dials: `Difficulty` (how many colours), the fall speed curve, the stage length and the timings |
| `game/cell.rs` | the `CellId` and `PieceId` space, including the link mask |

`rules.rs` was **narrowed** while phase 1 was worked: it holds the game's own dials, and
`GameConfig`, `MatchThemes`, `AiDifficulty`, `AiMode` and `ai_players()` moved to phase 2. None
of them can be written here - `MatchThemes` names themes that do not exist, and `ai_players()`
returns brains that do not either - and phase 2 is where the menu first needs all four.

`GameId(3)` is **not** declared in `cell.rs`: phase 0 moved the ids to `engine::game::ids`,
because pricing an attack means naming the game it crosses to and the game crates are siblings.
Add `PUYO: GameId(3)` there and re-export it as this crate's `GAME_ID`, the way the other two
do.

`nuisance.rs`'s queue is also what `engine::game::Game::pending_attacks` reports - a `Vec<CellId>`,
soonest first, which phase 0 added for exactly this and which both existing games leave empty.
Return the cells the tray should be drawn with (one icon may stand for a single puyo or a whole
row, as the game likes); the themes draw them from their own sprites and owe no new art.

`random.rs` is mostly *not* new machinery, and should not be written as if it were. The engine
already owns the seed and the randomiser: `engine::game::random` has `Seed` over `ChaChaRng`
and a `BagRandom<T>` with a look-ahead queue, which both existing games wrap in about the space
it takes to name their piece type. What Puyo adds on top is small — a pair is two draws from
the colour set, and Tsu deals the opening pairs from a reduced set so the first few placements
cannot be a fourth colour. Source that opening rule rather than guessing it.

The sequence **must be reproducible from a seed**: every player in a match is dealt the same
game from one seed, however far apart the playlist has moved them. Note *how* that is achieved
today, because it constrains what the sequence may depend on: `from_seed` builds `count`
**independent** randomisers from one seed (`dr_rustario::game::random::from_seed`), one per
player, and they stay in step only because nothing they draw depends on player-local state.
`BagRandom` even fixes its piece set `all: &'static [T]` at construction. So anything that
changes the *content* of the stream mid-match — a colour count that grows, a piece set that
swaps — permanently desynchronises two players who reach the change at different moments. See
the trap below.

### The link mask

Per the decision above, a `CellId` is colour plus a four-bit mask of which orthogonal
neighbours share its colour. Reserve the bits in `cell.rs` from the start — retrofitting an
encoding that every theme's sprite table is keyed on is miserable. `dr-rustario/src/game/cell.rs`
is the model for the bit-packing: an enum, a `From<Cell> for CellId` that packs it and a
`From<CellId>` that unpacks.

Rules worth writing down because they are easy to get subtly wrong:

* The **falling pair draws unlinked** — mask 0 for both halves, even when the two halves are
  the same colour and even when one is resting against a matching puyo. Linking happens on
  lock, not before.
* **Nuisance never links**, to anything, including other nuisance.
* Masks are recomputed after **every** lock, pop and settle — a pop changes the mask of every
  survivor that was touching the group, and a settle changes the masks of everything the
  fallen puyo left behind as well as everything it arrives next to.
* Ghost cells follow the active piece: unlinked.

There are `5 colours × 16 masks` plus nuisance to key, which is a big sprite table but a
mechanical one. Phase 1 added three more on top of those: the nuisance tray's `Small`, `Large`
and `Rock` symbols, standing for 1, 6 and 30 puyos. A theme may draw all three as its plain
nuisance sprite and lose nothing but the shorthand.

The **previews** are a separate table and bigger than it looks: a pair is two colours drawn
from five, so `PuyoPiece::all()` is **25 pieces**, not five.

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
* **Top-out is the death square, not a blocked spawn.** Tsu marks one square — the spawn
  point, the third column of the top visible row — and the game is lost when a puyo comes to
  rest *there*, which is not the same rule as "the new pair has nowhere to go" and not the
  same rule as either existing game's. Getting this subtly wrong is the classic way a Puyo
  implementation ends up feeling off, so confirm it against Puyo Nexus's
  [Basic rules](https://puyonexus.com/wiki/Basic_rules) rather than inferring it from play.
* **No hold, hard drop yes** — per the decision above. `hold()` is a no-op and Puyo boards
  show no hold box.
* Leave margin time out for now; note it in the handover as a possible difficulty knob.

### Emit the events Dr. Rustario already emits

The chain loop should report itself in the same grammar `bottle.rs` uses for its combos: one
`GameEvent::Clear` **per chain step** — not one for the whole chain — with `is_combo` false on
the first step and true on every step after it, and a `Settle` between steps. That is the
shape the rest of the engine is already listening for, so the particle field's clear wave, its
big-clear silhouette interrupt and its words all work on a Puyo board without any of it
learning what a chain is. `count` is the puyos cleared in that step and `detail` is the game's
own grading, which phase 2's `clear_class` and `clear_word` read back out.

### Fitting the engine's stage model

Puyo has no natural level, so map stages onto speed: a stage is a speed level with
`StageTransition::Seamless`, the way Rustris advances every ten lines, triggered by a
puyos-cleared count. **`speed_index` drives fall speed and nothing else.**

That last part is a correction to an earlier draft of this plan, which had `speed_index` drive
the colour count as well (four of five by default). It cannot: it would break the promise that
every player in a match is dealt the same game. Stages advance **per player** — a playlist
starts each player's game as their own board reaches it — while the colour stream is dealt
from one shared seed to independent randomisers. So the moment player 1 crosses the
puyos-cleared threshold that adds a fifth colour, they draw a colour player 2 is not drawing
yet, and from there the two are playing different games for the rest of the match. Rustris
never hits this because its level touches its speed and its score but never its bag.

The colour count is therefore **fixed for the whole match**, set from the vs. difficulty dial
or the options menu — both match-wide, both known before the first draw. It stays a difficulty
knob, just not a mid-match one.

The general rule, worth keeping in mind for the games after this one: `speed_index` may change
how a game *feels*, never what it *deals*.

**Done when:** unit tests build known fields, fire known chains, and match the documented
score and nuisance counts exactly. That test is the thing that makes "faithful" checkable
rather than asserted.

### Handover notes

Done on 2026-08-27. The crate is `puyo-rusto`, a workspace member, **116 unit tests**, no
warnings; `cargo test --workspace` is 669 tests and green. Nothing outside the crate changed
except `engine::game::ids::PUYO` and the workspace member list - the launcher does not know
this game exists yet, which is phase 2's job.

**The tables are sourced, and the tests check them.** Everything came off Puyo Nexus in a
browser on 2026-08-27, per this document's instruction not to guess: *Scoring*, *List of attack
powers*, *Basic rules*, *Nuisance queue*, *Offset rule*, *All clear*, *Tsu (rule)*, *Rotation*,
*Margin time* and *Puyo Puyo Tsu/Upcoming Pair Randomizer*. Each module's doc comment names the
page it came from. The check that matters is
`a_three_chain_scores_and_sends_what_the_published_table_says`: a staircase in two columns
fires a real three chain and has to score **1000** points and send **14** nuisance, which are
the published figures.

**Decisions taken that the plan did not anticipate:**

* **Chain power: the multiplayer table, in one player as well as two.** Tsu publishes two
  (`0, 8, 16, 32, 64, 96, ...` and a stiffer single player `4, 20, 24, 32, 48, 96, ...`). One
  table is one behaviour to test, and the attack economy is why the compendium took this game
  on. The cost is that a solo marathon scores lower than the arcade would have shown. Swapping
  in the single player curve for one-player modes is a small, contained change if it is wanted.
* **The 999 ceiling cannot be reached.** The biggest step a 6x13 board can hold is five colours
  in groups of eleven: `672 + 24 + 50 = 746`. `MAX_MULTIPLIER` is carried for fidelity to the
  formula, and the test says so rather than pretending a match will meet it.
* **The tray is three symbols, not one.** `NuisanceIcon::{Small, Large, Rock}` for 1, 6 and 30
  puyos, which is the game's own tray. That is three cell keys beyond the `5 colours x 16
  masks` plus nuisance. **A theme with no art for them may draw all three as its plain nuisance
  sprite** and the tray still reads correctly - so this costs phases 2 and 3 nothing they do
  not want to spend.
* **`PUYOS_PER_STAGE = 30`** - a stage is a speed step, roughly seven or eight groups, chosen
  to be a comparable stretch of play to Rustris's ten lines. Nothing measured it; it is a knob.

**The ghost puyo rule is in.** The first pass through this phase left the hidden thirteenth row
behaving like any other, because the "row 13 does not pop" rule was not on any page it had read;
Alex pointed at [Special Maneuvers and
Mechanics](https://puyonexus.com/wiki/Special_Maneuvers_and_Mechanics#The_13th_Row_and_Beyond),
which has it, and it is now implemented and tested. It settles a question that would otherwise
have had to be guessed:

> "Puyo in the 13th row can't be cleared even if they 'connect' in a group of four... You can
> use the 13th row's properties to make chains that **won't pop** until the Puyo in the 13th row
> drops down."

So a group of four with one member in the ghost row does not pop **at all** - rather than the
three visible ones popping and leaving the ghost behind. The other reading would fire the chain
immediately and there would be no technique to speak of. It is one function,
`Board::grouping_color`, which reports a ghost as having no colour: a ghost is then neither the
start of a group nor reachable from one, so it can neither pop nor make up the numbers. Ghost
nuisance is not cleared either, and nothing draws itself joined across the boundary, because the
link mask is the game telling a player what will pop together.

The same section gives Tsu's **ceiling**: "there is a ceiling above the 13th row that prevents
rotation into the 14th row". That falls out of the board having no such row - a rotation needing
one is pushed back down, the way a floor kick pushes up - and there is a test for it.

**Left out, deliberately, and each one is a real gap rather than an oversight:**

* **Margin time.** Sourced but not built: Tsu's is 96 seconds, after which target points fall
  to 3/4 and then halve every 16 seconds, for at most 14 iterations or until they reach 1.
  It is the game's answer to matches that go on too long, and it is a *good* candidate for the
  vs. difficulty dial in phase 5 - a match that cannot end is a real problem for a playlist.
* **The nuisance scatter is not sourced, because it is not documented.** Puyo Nexus lists
  "Distribution algorithm of ojama puyos across a row" as an open question on its own reverse
  engineering page. What is built is full rows first, then the remainder over *distinct*
  columns from a dedicated RNG - which honours the sourced parts (rows of six, a 30 cap) and
  guesses only the part nobody has written down. Flagged rather than hidden.
* **Tsu's opening-pair quirk.** The real game deals its first three pairs in reverse pool
  order. It is an artefact of a triple buffer with no effect on what you are dealt overall, and
  it is skipped.

**One rules bug the tests caught**, and the kind this document warned about: the all clear
bonus was being spent by the very chain that earned it. Tsu pays it out on the *next* chain, so
`finish_chain` resolves against the tray first and earns afterwards. It would have looked like
a tuning problem - every board-clearing chain sending 30 more than it should.

**Shapes phase 2 will want:**

* `ClearDetail { chain, all_clear }` is what `GameEvent::Clear`'s `detail` carries, with
  `From`/`Into<u64>` both ways. `clear_class` and `clear_word` read it back out;
  `BIG_CLEAR_PUYOS` and `LONG_CHAIN` are in `game/mod.rs` waiting to be used by them.
  **Remember `clear_class` reserves 3 for the biggest clear**, or the particle field's
  silhouette interrupt never fires.
* The chain loop emits exactly the grammar this plan asked for: one `Clear` per step,
  `is_combo` false on the first and true after, a `Settle` between steps, and `count` is the
  puyos that went including nuisance.
* `Game::pending_attacks` returns the tray, so a theme only has to declare where the strip goes
  (`pending_max` on a particle theme, a `PendingLayout` on a retro one). Phase 0 built the rest.
* `queue()` is two pairs; `held()` is always `None` and `hold()` is a no-op, so **no hold box**.
* A preview needs **25 pieces** (five colours by five), not five: `PuyoPiece::all()`.
* `Difficulty` is the game's own five settings (very easy to very hard) and sets the colour
  count, the rows of nuisance you start buried under, and a speed bonus on the hardest. It is
  *not* the four ai difficulty names, which are a different thing with the same word.

**The colour count is fixed for a whole match**, as this plan insisted. `GameRandom` builds the
whole 128-pair pool at construction from one seed, so nothing drawn later can put two players
out of step - `one_seed_deals_every_player_the_same_game` plays three games twenty placements
deep and compares the boards. `speed_index` drives fall speed and nothing else.

---

## Phase 2 — playable on the particle theme, human players

**Status:** `todo` — phase 1 is `done`, so this is next

**Goal.** Two people can sit down and play a Puyo match from the menu.

`render.rs` implementing `GameRender` (only `name` and `spawn_cells` are required; add
`clear_class` and `clear_word`), the `modern` particle theme in **original art**,
`options.rs`, a `Mode` impl in the launcher of about 110 lines modelled on `DrRustarioMode`,
a `ModeChoice` variant, and the high score tables.

Also here, moved down from phase 1 because nothing there could use them: `GameConfig`,
`MatchThemes`, `AiDifficulty`, `AiMode` and the stub `ai_players()` in `game/rules.rs`, beside
the `Difficulty` that phase 1 left there. Note the two are different things wearing the same
word - `Difficulty` is the *game's* five settings (how many colours, how buried you start),
while `AiDifficulty` is the four names every game's menu offers.

`clear_class` and `clear_word` read back `ClearDetail { chain, all_clear }` out of the event's
`detail`; `BIG_CLEAR_PUYOS` and `LONG_CHAIN` in `game/mod.rs` are there for them. The launcher
`Mode` gets its brains through `puyo_brain()` in `games.rs`, beside the other two - see phase
0's handover for the `AiBrain` shape. And a Puyo board has **no hold box**: `held()` is always
`None`.

Two engine contracts to honour:

* `clear_class` grades the chain 0..3, with **3 reserved for the biggest chain**. The particle
  field relies on every game grading its largest clear as class 3 for the big-clear silhouette
  interrupt to fire — see `TETRIS_CLEAR_CLASS` in `rustris/src/render.rs`. Every theme then
  supplies a sound per class it can return.
* `clear_word` returns the new `CHAIN` on a long chain and `PERFECT` on an all clear.

The particle theme needs only `sprites.png`, the mascot strips and the oggs, plus a
`particle_color` and a `particle_palette` — its background, board frame, HUD and cards are all
drawn procedurally. Template: `dr-rustario/src/theme/modern/mod.rs`.

**The nuisance tray is one number here.** Phase 0 built the attack-queue strip and put the
placement inside `modern_theme`, and phase 1 filled it in, so this theme sets `pending_max` to
how many icons fit and nothing else: the strip is drawn a cell to an icon along the top of the playfield, from this
theme's own nuisance sprite, and the builder keeps the room back out of the board's top slack.
Set it to 0 and no strip is drawn at all, which is how both existing games' particle themes
are left.

### The link mask is an art problem here

Phase 1's `CellId` carries a four-bit neighbour mask, so `sprites.png` is keyed on
`colour × 16` — and unlike the retro themes in phase 3, which have the linked sprites in the
rip, **this theme's art is original**, so somebody has to author eighty variants. Do not do
that by hand.

Draw a **body** per colour and four **bridge** overlays — up, down, left, right — and
composite the sixteen masks out of them: five sprites per colour rather than sixteen. The four
bridges are one shape rotated, so what actually has to be drawn is a body and a bridge per
colour. This also keeps the sheet small, which matters — `dr-rustario` already ships 26 MiB of
embedded assets.

Whether the compositing happens ahead of time into a generated sheet or at draw time is the
author's call; ahead of time keeps `CellSpriteData`'s one-snip-per-cell contract intact and
costs nothing at runtime, which is the reason to prefer it.

### The ai gating tests fire two phases early

`every_mode_offers_the_same_ai_opponents_and_demos` and `ai_difficulties_agree` in
`launcher/src/modes.rs` hold every mode to the same four difficulty names and the same two
demos. They start applying the moment Puyo appears on the menu — which is *this* phase, while
the ai is phase 4. So this phase ships a **stub `ai_players()`**: the four difficulty names
and the two demo modes, all backed by a placeholder brain that picks a legal placement at
random. The tests pass, the menu tells the truth about what it offers, and phase 4 replaces
the brain without touching the menu surface.

If the placeholder feels too dishonest to ship even briefly, the alternative is to swap phases
3 and 4 — they have no dependency on each other, and doing the ai first shortens the window in
which the menu offers an opponent that cannot play. That is a reordering of the plan, so it is
Alex's call and not an agent's: if it is taken, renumber the phases and their `blocked on`
lines here in the same commit, so the document never disagrees with the order being worked.
Otherwise ship the stub and say so in the handover.

**Done when:** Puyo is selectable from the pre-menu and playable by two humans on the particle
theme, with high scores recorded, the particle field picks up its pieces and mascot, and
matching puyos are visibly joined.

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
  the previews. **The cells are the full `colour × 16` link grid** from phase 1, not one
  sprite per colour, plus nuisance and the three tray symbols; the previews are 25 pairs. Unlike the particle theme these do not have to be authored: the rips
  carry them, because the original games drew connected puyos the same way — Kirby's Avalanche
  is ripped as a sheet literally called "Blobs & Boulders". Budget the time in slicing and
  arithmetic rather than in drawing.
* `background.png`, and `board.png` either as one frame or as N frames side by side selected
  by `board_snips`, one per speed band
* `background-tile*.png` if the theme uses `SceneType::Tile`, one per speed band
* `game-over.png` / `match-end.png` overlays, positioned by `game_over_points` and
  `interstitial_points`
* `font.png` — ten digits in a row for `FontRenderOptions::numeric_sprites`, or digits plus
  letters sliced by a closure the way `rustris/src/theme/data.rs` does it
* a `pending: Some(PendingLayout { point, step, size, max })` in the theme's `mod.rs` — where
  the nuisance tray goes in *this* background, in source pixels, `step` negative to fill
  leftwards or upwards. No sprite work: it draws the theme's own nuisance cell. Every existing
  retro theme passes `None`, so the field is there and empty to copy from
* mascot strips `{idle,throw,victory,game-over}.png`
* music and about twelve sound effects as **OGG Vorbis at exactly 44,100 Hz, mono or stereo**
  — the decoder rejects anything else outright. Music may be split into `-intro.ogg` and
  `-repeat.ogg` for the intro-then-loop chaining.

Template for the 34-field `RetroThemeOptions`: `dr-rustario/src/theme/nes/mod.rs`.

Register each theme in three places: `theme/mod.rs::all_themes` (the order defines the theme
sprint), the `MatchThemes` enum in `game/rules.rs`, and `theme_mode()` in `options.rs`.

**Done when:** `frame_shot` renders every theme correctly, `menu_shot` walks the theme rows,
`field_preview sheet` outlines the new sprites cleanly, and matching puyos join up on all
three themes the way they do on the particle one.

### Handover notes

_(to be filled in by the agent that completes this phase)_

---

## Phase 4 — the ai

**Status:** `todo` — blocked on phase 3

**Goal.** Puyo fields the same four difficulties and the same two demo models as the other
games — for real this time. Phase 2 already put the four names on the menu behind a
placeholder that plays at random, because the tests demanded it; this phase makes the menu
honest. `VersusAi` cannot deal a Puyo board worth playing against until it does.

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

**The ghost row is worth a feature of its own.** A puyo in the hidden thirteenth row cannot
pop and does not count towards a group, so a chain with a foot up there is *held back* until it
drops - which is a real technique and something a scorer can either exploit or blunder into.
`Board::is_ghost` is the predicate; the top of the board is not worth what a naive height
feature would say it is.

**Read colours through the mask, not around it.** Every board feature above is about colour,
and a `CellId` here is colour *and* link mask, so a feature that compares raw `CellId`s sees
sixteen different reds and finds no chains at all. It will not fail loudly — it will train to
a mediocre plateau and look like a tuning problem. Unpack to the colour enum first, the way
`DrCell::color()` does, and keep the mask for the one place it is genuinely useful: link
counts come straight off it for free, since that is exactly what it counts.

If phase 2 shipped the stub `ai_players()`, this phase is where the placeholder brain goes and
the real one takes its slot. Nothing on the menu surface should change.

In the launcher that is smaller than it sounds. Phase 0 replaced the versus mode's brain tuple
with an `AiBrain` trait (`launcher/src/games.rs`): an ai player carries one boxed brain per
game, a brain handed a board that is not its game does nothing, and the controller offers the
board to each in turn. So this phase adds a `puyo_brain()` beside `dr_rustario_brain()` and
`rustris_brain()`, and one arm to `VersusAi::ai_players` — and touches the versus controller
not at all.

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
nothing. Phase 1 leaves `puyo_rusto::game::foreign_attack` returning zero for every receiver,
so today a Puyo attack is dropped at the border rather than mispriced - this phase is where it
starts crossing at all.

**Margin time is the knob to reach for if matches drag.** Phase 1 sourced it but left it out:
Tsu's is 96 seconds, after which the 70 target points fall to 3/4 and then halve every 16
seconds. It makes every chain send more as a match wears on, which is exactly what an endless
marathon playlist needs and what nothing else in this game provides.

**Price the two directions asymmetrically, because the two directions are not symmetric.**
Attacks *into* Puyo land in the nuisance tray and can be answered — a Puyo player who chains
back cancels them outright, so a number that looks brutal on paper is often absorbed for free.
Attacks *out of* Puyo land on a Rustris or Dr. Rustario player who has no offset at all and
simply takes them. So the same raw measurement means different things in each direction, and
tuning both ends from one table will get one of them wrong. Measure the six prices, then sanity
check by playing each pairing rather than trusting the numbers.

Where the six prices go: each sending game's `foreign_attack(receiver, ...)` in its own
`game/mod.rs` gains an arm for each receiver, and the caller adds a `with_foreign_for(receiver,
price)`. Phase 0 made the default zero, so any crossing this phase forgets is *dropped* rather
than sent in the wrong units — silent, but harmless and easy to spot by playing a pairing and
seeing nothing arrive.

This is also the phase that sets the Puyo half of `Difficulty` — the level the 0-10 vs. dial
maps to, as an arm of `Difficulty::level(game)`, plus a speed dial of Puyo's own if it wants
one the way `dr_rustario_speed()` is Dr. Rustario's. Left as a shape in phase 0.

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
