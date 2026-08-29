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
* **Scope.** A full citizen of the compendium: its themes, an ai offering the same four
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
* **The speed ramp stays.** Tsu has no level that climbs with play — versus fixes the drop
  speed for the whole match and answers a long one with margin time — so phase 1's
  `PUYOS_PER_STAGE = 30` and its twelve step fall curve are an invented house rule, and phase 2
  left the question open. **Alex settled it on 2026-08-27: keep the levelling intact**, in
  single player and versus alike. The whole compendium's mode structure is built on stages, and
  a third game that opted out of it would cost the level sprint, the stage clear card and the
  speed band scenes their meaning here. Margin time still arrives in phase 5, on top of the
  ramp rather than in place of it. The long form is in phase 2's amendments, where the question
  was raised.
* **The retro themes are `genesis` and `snes`** — Mean Bean Machine and Kirby's Avalanche,
  oldest first. There was a third, `3ds` (Puyo Puyo Chronicle), chosen with Alex on 2026-08-27
  over two drafts that named another Sega title in that slot; it was built and then **cut on
  2026-08-28**, for being modern art in a retro slot and for capping the cell size of every
  theme the game has. Phase 3c carries the whole reasoning. Every theme is named for its
  platform, as everywhere else in this repository. Sources are at the head of phase 3.
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

### The skin, added after phase 2

The particle theme's art turned out not to be original after all (see phase 2 below): it is a
rip that carries **eleven** usable sets of the same puyos, so every player is dealt a
different one and a two player game is never two boards of the same puyos. That went into the `CellId`
beside the link mask - bits 9-12, a `PuyoSkin` - and into the `PieceId` for the previews, since
a queue drawn from the other player's art is exactly as wrong as a board would be. The warning
above stands and was worth heeding: the retrofit was cheap only because the mask had already
established that a `CellId` may carry drawing information, and because `PuyoCell` itself stayed
skinless, so nothing in the rules can tell two players' puyos apart.

**Eleven and not fifteen, in two steps.** One of the fifteen whole skins has no downward neck
to cut - its sixteen link variants are only eight, paired so that a puyo joined below draws
exactly like one joined to nothing - so nothing can make it meet the puyo underneath, and
`rip.py` has always left it out. Three more went later for how they *look* joined rather than
for anything missing, which is the harder thing to notice: a set earns its place only if four
in a row read as one mass, and a television with antennae whose necks are stubs stays a row of
televisions, a stick figure joins into an elongated humanoid, and a small round face merges
into a gappy mesh. None of that shows a cell at a time; `python3 puyo-rusto/art/rip.py check`
draws a whole board per skin, which is the only way to judge it.

**The sheet is laid out against a texture limit, not just for tidiness.** It is loaded whole,
as one texture, and a band per skin stacked in a single column stood it 6720 pixels tall -
past the 4096 `MAX_ATLAS_WIDTH` a GLES driver will allocate in a dimension, so on a handheld
the theme could not be built at all rather than merely being slow. `rip.py`'s `BANDS_ACROSS`
lays the bands two across instead; `skin_block` in `theme/modern/mod.rs` is the same formula on
the Rust side, and those two are the only places that know it. A test holds the sheet to both
its shape and that ceiling, so a skin count that would push it back over fails rather than
shipping.

It was first built as a *slot* - two of them, resolved to art by the theme when the theme was
built - which is worth writing down because it was wrong and the reason is not obvious. A theme
is built once for a whole session and bakes its sprites into an atlas at that point, so a slot
resolved there can only be re-rolled by rebuilding the atlas: the puyos were fixed for the run,
not per match. Keying **all fifteen** instead moves the choice to `PuyoSkin::deal`, which the
game calls off the match seed, and the theme stops having an opinion. Off the seed rather than
the thread's randomness so that a playlist swapping one board onto Puyo mid-match hands that
player the puyos they already had, and so a replayed seed looks like it did.

Three consequences to know about:

* the sheet keys `PuyoSkin::COUNT × 84` cells and `PuyoSkin::COUNT × 25` previews - 1176 cells
  and 350 previews. `BlockSpriteSheet` wraps its atlas onto another row at `MAX_ATLAS_WIDTH`,
  and shelves its preview sheet the same way, since either in one line is past what a driver
  will allocate. `field_preview sheet` writes one png per theme for the same reason.
* the pre-built bank of alpha variants had to go, and its removal is what made fifteen skins
  affordable at all. It was 63 whole copies of the atlas, one per fade step, so that a `&self`
  draw could pick one without needing `&mut` for `set_alpha_mod` - roughly 106 MiB for a single
  skin. The atlas is now one texture in a `RefCell` with the fade applied at draw time, which
  is what the popup font already did for its tint: all fifteen skins come to about 27 MiB.
* the race on the title screen is the one place the whole sheet is on show - `race_themes`
  offers a pair per colour of every skin. It is not a board and owes nobody consistency.

**Cutting fifteen skins is not cutting one fifteen times.** Only six of them reach their own
cell edges; eight more were drawn on pitches of their own and laid out on the rip's common 72
pixel grid, so a neck stops one to eight pixels short and every join draws a seam. `repair`
finds a neck by *difference* - the linked tile against the same puyo unlinked - and runs the
outermost line of that difference to the cell edge. Locating it by difference rather than by
the tile's own outermost pixels is the whole trick: one skin wears antennae on the same line
as its upward neck, and repeating those paints a band of antenna up the cell. One skin is
dropped outright, because its sixteen link variants are only eight - paired so that a puyo
joined below draws exactly like one joined to nothing - so it has no downward neck to cut.
`rip.py check` is what found all of this and is the only way to see it: a seam is a hairline,
and reading the sheet a cell at a time will not show one.

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

### Rules audit, 2026-08-27

The whole crate was read back against [puyo-nexus-rules.md](puyo-nexus-rules.md) after phase 1
closed - every table, every rule, module by module. **135 unit tests, `cargo test --workspace`
688 and green.** The tables all check out: the chain power curve, the colour and group bonus
tables, the 70 target points and the carry, the 30 nuisance all clear, the five difficulty
settings and their colours, starting rows and speed bonus, the pair pool's reduced opening, and
the ghost row. Three things did not, and are fixed.

**A pair that landed flat drew unjoined.** `Board::recompute_links` documents itself as running
"after every lock, pop and settle", and it ran after two of the three: `Pair::lock` laid the two
halves down loose, `Board::settle` only recomputes when something actually moved and
`Board::pop` only when something actually popped. So every placement that landed flat and
cleared nothing left its own puyos - and whatever they landed beside - with a mask of zero, and
the signature joined look simply went missing for that placement. `Pair::lock` now recomputes.
This was invisible to phase 1 because nothing draws yet, and would have read as an art bug in
phase 2.

**The quick turn slid the pair down instead of flipping it in place.** *Rotation, collision and
push back* is explicit that after the double tap "a rotation pushes the pair's main puyo
upwards, with the slave puyo taking its place at the bottom; or the slave puyo ends up at the
top with the main puyo being pushed down by one cell" - either way the pair keeps the same two
squares and the halves swap. The old code searched for somewhere to put the flipped pair and
took the pair's own cell first, which moved it down a row in mid air; it only did the right
thing against the floor, where that candidate failed. It is now the swap, which is also why the
page can say that by this point "nothing will cancel the rotation": the two cells are the ones
the pair is already standing on, so the quick turn cannot fail and no longer returns an
`Option`.

**A pair in the ghost row could be shoved about.** The same page's *current row check* -
`if(current_row < 2) if(target_cell == bottom || target_cell == top) exit;` - refuses an upright
rotation outright when the pivot is in a ghost row, rather than kicking the pair anywhere, and
does not even arm the double tap. Phase 1 read Tsu's ceiling as falling out of the board having
no fourteenth row, and pushed the pair *down* off it instead, which handed the player a free
manoeuvre up there. Now refused. (The 13-row board is still right: the fourteenth row exists in
the real game's memory but nothing can reach it, which is what the ceiling rule is.)

Two smaller things: `NuisancePoints::reset` was dead and its doc comment described the opposite
of what it did - the leftover carries and is never reset, per *Scoring* - so it is gone; and the
crate is clippy-clean.

**Tests added**: the whole of Puyo Nexus's published *List of Chain Scores* - all nineteen chain
lengths, points and nuisance - rather than the first four; group bonuses adding over several
groups of a step; nuisance caught between two groups clearing once; the ghost row above the
death square not being the death square; garbage falling after a placement that clears nothing
(the other half of classic offset, which nothing covered); the five-row cap end to end; the
link masks after a plain lock; soft drop; and the three rotation rules above.

**Known gaps, all deliberate and all sourced**, for whoever picks up phase 5's balance work:

* **Margin time** is still out, and it is fully specified in the local copy (96 seconds, then
  target points to 3/4 and halving every 16 seconds, at most 14 iterations or until they reach
  1) - it is the game's own answer to a match that will not end, which a playlist wants.
* **The soft drop bonus** is out. *Scoring* says Tsu adds the drop bonus to the nuisance a chain
  sends, and *Special Maneuvers and Mechanics* calls it "charging up your 1 Chains" - but
  neither page gives the points per cell, so implementing it means guessing a number, which this
  document forbids. `GameEvent::SoftDrop` is already emitted if it is ever wanted.
* **Soft dropping onto a blocked cell locks the pair immediately** in Tsu, skipping the grace
  period entirely (*Soft Drop*). The game uses one `LOCK_DELAY` either way.
* **The nuisance scatter** is still the honest guess phase 1 flagged: the wiki lists
  "Distribution algorithm of ojama puyos across a row" as an open question on its own reverse
  engineering page.

---

## Phase 2 — playable on the particle theme, human players

**Status:** `done`

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

**Settled differently in the end**: `art/sprites.py` did draw the eighty variants procedurally
the way this section proposes, and then a Puyo Puyo Tetris rip turned up with the linked
sprites already in it, fifteen skins over. `art/rip.py` cuts those and `art/sprites.py` is kept
only as the description of what the sheet has to contain. The reasoning below is what it was
weighed against.

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

Done on 2026-08-27. Puyo Rusto is on the pre-menu, playable by two humans on its own particle
theme, with its own high score tables. `cargo test --workspace` is **706 tests, 0 failures**
(688 before); `puyo-rusto` is 152 of them and is clippy clean, and the workspace's clippy
warning count is unchanged at 112.

**The art and the audio are generated, and both generators are committed.** The plan said not
to draw eighty sprites by hand and this is how that was avoided:

* `puyo-rusto/art/sprites.py` draws `src/theme/modern/sprites.png` (93 KiB). A puyo is a
  signed distance field: a body circle unioned with one *neck* per linked direction, joined by
  a smooth minimum so the neck flows out of the body with a fillet rather than notching into
  it. The necks run **past the cell edge**, which is the trick that matters - the rim is the
  band just inside the field, so a shape that stops at the edge would draw a dark line across
  every join. Layout is `colour x 16` on a 16x6 grid, 64px cells at a 72px pitch, with the
  nuisance puyo and the tray's three symbols on the sixth row.
* `puyo-rusto/art/audio.py` synthesises the sixteen oggs (340 KiB) - a small chiptune
  synthesiser, everything OGG Vorbis at 44,100 Hz mono, which is what the decoder takes. The
  four `pop-*.ogg` are the same flourish a whole tone apart, so a chain is *heard* climbing,
  which is what `clear_class` grades a step for. Every one of them is decoded at theme build
  time, so `frame_shot` running is proof they all load.

The whole theme was **436 KiB** against `dr-rustario`'s 26 MiB, because none of it was a rip.

**Superseded on 2026-08-27: the puyos are now a rip.** A Puyo Puyo Tetris sprite sheet sits
in `puyo-rusto/art/` - gitignored, since 12 MiB of source for 317 KiB of output is not worth
carrying - and `art/rip.py` cuts the theme's sheet out of it instead: the first of its sixteen
skins, which is the glossy Tsu one. Everything above about the *layout* still
holds, because the rip is on the same `colour x 16` grid this was: only the cell grew from 64
to 72 and `SRC_BLOCK_SIZE` with it. The three things the rip needed fixing are in `repair`:
its right hand necks stop two pixels short of their cell, red's upward neck was cropped off by
the top of the sheet, and an upward neck starts one pixel above its own cell and so lands in
the bottom row of the puyo above. `art/sprites.py` is kept, writing `art/procedural-sprites.png`
(gitignored) rather than the theme's sheet. The sheet is 317 KiB, the theme 672 KiB.

**And the music is a rip, 2026-08-27.** `art/music.py` cuts the theme's soundtrack out of a
directory of converted Puyo Puyo Tetris tracks and a `loops.json` of their loop points, which
is not in the repository either - the same footing the sprite sheet is on. Five tracks: *It's
Main Menu!* over both Puyo Rusto menu screens, through a `MENU_SOUNDS` of its own the way
Rustris has one, and *Korobeiniki*, *Decisive Battle*, *Magical Confrontation* and
*PuyoTetroMix* as the game music, one of them dealt per match.

Two things had to give for that. The mixer takes 44,100 Hz and nothing else, so four of the
five are resampled; and it has no loop marker, so each track is *split* at its loop point into
`-intro.ogg` and `-repeat.ogg`, which is the pair `StructuredMusic::new` has always taken and
Dr. Rustario's themes have always shipped. The split is cut on raw pcm rather than by seeking
with ffmpeg, so the seam falls on the sample the loop point names.

Dealing one per match needed the engine to hold more than one: `AudioTheme` keeps a list and a
`Cell` for the pick, `MatchSettings::music` is what a match asks for (`Random`, or a `Track`
indexing that list), and the pick is made in `ThemeContext::sync_music` - the one place that is
reached only when the theme owning the music has changed, so nothing re-deals for a pause, a
stage clear or a game over, and a second Puyo theme will re-deal of its own accord. The menu's
`music` row is `random` plus `GameMusic::ALL`, whose order *is* the index into
`theme::GAME_MUSIC`. It costs 12.3 MiB, which takes the theme from 672 KiB to 13 MiB - the
first thing this crate has been charged real money for. *(Amended 2026-08-28, phase 3d: the
`music` row and everything that pinned a track are gone. A match is dealt one, always.)*

`art/audio.py`'s `music_loop` went with it, and the chord and lead tables it was the only
reader of; the two stings are still its own.

**And the sound effects are a rip, 2026-08-27, so `art/audio.py` is gone.** A dump of Puyo
Puyo Tetris 2's sound effects - twelve `.acb` banks of WAVs, 33 MiB, not in the repository and
gitignored beside the other two sources - and `art/sfx.py` cuts the fifteen the theme keys out
of it. The synthesiser is not kept the way `art/sprites.py` was: procedural art was a
description of what the sheet had to contain, but a synthesised chain is just a wrong guess at
the sound Puyo is known for, and the rip carries seven `ren` steps that are the real thing.

The mapping is mostly the filenames doing the work - `se_puy07_move` is `move.ogg`,
`se_puy08_rotate` is `rotate.ogg`, `se_puy09_down` is `lock.ogg`, `se_puy24_levelup` is
`speed-up.ogg`, `se_puy19_win` and `se_puy20_lose` are the two stings. Four of them took a
choice:

* **`pop-1..4` are `se_puy00_ren1` to `se_puy03_ren4`.** 連 is the chain, and the rip has
  seven of them, one per step. `clear_class` grades a step 0..3 - chain 1, 2, 3, then
  everything from four up - so the first four are what is wanted, and `ren4` is what the
  original plays on the step this game stops counting at. `ren5`, `ren6` and `ren7` are on the
  shelf for a `CLEAR_CLASSES` that ever grows.
* **`attack.ogg` is `oj_okuri1`, the smallest of four.** 送り is nuisance being sent, and it
  fires on top of the `ren` that earned it, so it takes the one that will not bury it.
* **`hard-drop.ogg` is `se_tet04_hdrop`,** the only borrowing from Tetris's half of the rip
  besides `settle`. Tsu has no hard drop; this game does, so it takes the sound of the
  mechanic it took.
* **`settle.ogg` is `se_tet01_fall`,** which at 0.07s and a peak of 0.21 is the quietest thing
  in the theme. It fires once per chain step under a `ren` four times its height, and a chain
  that clatters is a chain you cannot hear climbing.

Nothing is normalised - levelling them would put a landing thud on top of the win fanfare -
and everything is resampled, since a third of the rip is 48 kHz and `decode.rs` takes 44,100
and nothing else. Padding is trimmed off both ends, which is a quarter of a second of nothing
after a chain step that has to land again before the next one, and key press latency at the
head.

Two more came out of `se_sys`: **`menu/chime.ogg` and `menu/select.ogg`**, from
`se_sys05_cursor` and `se_sys02_decide`. `MENU_SOUNDS` was already Puyo Rusto's own for the
music and borrowed the engine's clicks; now only the high score music is borrowed, since the
rip has nothing that belongs under a table of names. They live in `theme/menu/` beside that
music rather than in the particle theme, for the same reason it does: a menu is not a theme
and phase 3's retro themes walk the same one.

It costs 500 KiB where the synthesiser cost 144 KiB, against a 16 MiB theme.

What else the rip has, and did not get used: the flash and burst frames a puyo pops with, which
would want `DestroyStyle::Pop` - and that is a fixed 300 ms against `rules::POP_DELAY`'s 280,
so a chain step would advance while the cells were still animating; and **fifteen more skins**,
several of them pixel art, which is what phase 3's retro themes could be built out of.

**The previews cost nothing.** A pair is two colours from five, so the plan budgeted 25 preview
sprites - but `PreviewData::Compose` builds a preview out of cells the sheet already has, and a
pair is just its two cells stacked. `theme/data.rs::previews()` is fifteen lines and there is a
test that every cell it names is keyed.

**Decisions the plan did not anticipate:**

* **`GameKind::PLAYLIST_ORDER`, a third list beside `ALL` and `RUNNING_ORDER`.** Adding
  `GameKind::Puyo` puts Puyo in every versus playlist for free, and that is wrong two phases
  early: with one theme, `PlaylistThemes::slots()` (a min over the games) would collapse the
  theme race from eight stages to three, and the **retro playlist would have no stages at
  all**, since Puyo has no retro themes until phase 3. So a game is now billed on the pre-menu
  as soon as it can be played and joins the playlists separately. `slots()`, `stage_count`,
  `first_game`, `fixed_stages` and `random_game` all read `PLAYLIST_ORDER`/`PLAYLIST_COUNT`;
  every playlist deals exactly what it dealt before, seed for seed, and the tests say so.
  **Phase 5 adds `GameKind::Puyo` to that list**, which is one line.
* **`PerGame::default()` is one value per game, not an empty collection.** The derived `Default`
  gave an empty `Vec`, which nothing noticed while `slots()` was a `min` over `values()` - it
  is an out of bounds index the moment anything looks a game up by name. `PlaylistThemes::default()`
  is used in a test, and that is what caught it.
* **`ModernThemeOptions::visible_rows` counts the buffer rows in.** It is `ROWS` (13), not
  `VISIBLE_ROWS` (12): the board frame covers `visible_rows - top_buffer_rows`, so passing 12
  drew one of the twelve *playable* rows above the frame with the ghost row nowhere. Rustris
  reads the same way (`VISIBLE_HEIGHT = BOARD_HEIGHT + VISIBLE_BUFFER`) and it is easy to get
  backwards. Phase 3's retro themes want the same 13.
* **No mascot, deliberately.** `mascot: None`, exactly as Rustris's particle theme is left, so
  the queue is a column of slots. Puyo has no mascot art of its own and there is no honest way
  to generate a four-strip character animation; **phase 3's retro themes bring the mascots**
  (Robotnik and Kirby are both ripped), and the particle field's mascot silhouettes come with
  them, since the field outlines the sprites of every theme in play rather than one.
* **`MatchThemes` is two entries** - `all` and `particle` - so `MatchThemes::count()` is 1 and
  the theme sprint is not offered. It comes back on its own the moment phase 3 adds a second
  theme; `options.rs`'s test says exactly that, and `theme_mode()` goes through
  `MatchThemes::initial_index()` rather than a hand-written match, so phase 3 adds an enum
  variant, an arm of `initial_index` and an entry in `all_themes` and nothing else.
* **`all_themes` builds the particle theme twice.** `reference_block_size` measures *built*
  themes, and the other two games take their reference off their retro themes, which are built
  first. With nothing to measure against, Puyo builds the theme once provisionally, measures it
  and builds it again. **Phase 3 should delete that and take the reference off the retro themes**
  like the other two.
* **The menu's dials are themes / mode / level / difficulty.** `level` is the starting speed
  step (0-9) and `STAGE_NOUN` is `"level"`, so the modes read "1 level sprint" as they do
  everywhere else and the HUD row is the `Level` every game shows. There is no randomiser row:
  Puyo has one pair pool and nothing to choose between. `difficulty` is the game's own five
  settings from phase 1 - the colour count and how buried you start - which is *not* the four
  ai difficulty names.

**The stub ai shipped, as the plan allowed.** `puyo_rusto::game::ai` is `PuyoAiKind::Placeholder`
and a `PuyoAiAgent` that drops the pair in a column picked at random; it is seeded from a fixed
constant, so a demo replays the same game twice. All four difficulty names and both demos are
on the menu and the gating tests pass, so **the menu surface phase 4 has to keep is already
final**: a `puyo_brain()` in `games.rs`, an arm of `VersusAi::ai_players`, and
`AiDifficulty::brain()` in `game/rules.rs` are the three places a real brain goes.

**Every generic launcher test now covers Puyo**, because phase 0 keyed them on `GameKind::ALL`:
the ai difficulty names, the players list, the high score keys, the per-game mode and the versus
ai all picked it up with no new test. Two were added: `a_game_played_on_its_own_deals_its_own_boards`
(the one thing `game_mode` could get wrong that nothing else would notice) and
`every_game_a_playlist_deals_is_a_game`.

**Examples.** `frame_shot` takes `puyo` and builds a board with groups linked up in it and an
attack left in the tray; `scale_report` and `field_preview` list it, the latter with its
palette. `menu_shot`'s theme walk used to be the literal `["all", "nes", "all"]` - it now asks
each game for its own theme names, since Puyo has no `nes` and `MatchThemes::from_str` would
have panicked. `field_preview sheet` reports Puyo as *"25 sprites, 1 used, 24 duplicates"* and
that is correct rather than a fault: every pair is the same two-cell silhouette, so the shape
bank keeps one.

**Checked without a display:** `frame_shot` at 640x480 (1 player) and 900x700 (2 players),
`menu_shot` at 960x720 walking the theme and mode rows, `field_preview sheet` and
`field_preview 3 ... puyo`, and `scale_report` (Puyo's particle theme: scale 1.0, block 48,
board 288x624, 49px of top slack cropped out of the 50 it keeps back for the tray). **Nobody
has played it with a controller yet** - that is the last line of this document's verification
list and it is Alex's.


### Amended after review, 2026-08-27

Alex played the phase 2 build and four things came back. Two were bugs, one was a decision
taken, one is still open. All of them are in the code as described here.

**The queue was ragged, and it was Rustris's bug too.** `modern_theme` lays the queue out as a
column of slots, the first 2.5 blocks and the rest 1.5, all sharing a *left* edge - and a piece
is drawn centred in its own slot, so the big slot's piece sat on a different axis from the
rest. With tetrominoes that reads as a slightly untidy column; with Puyo, where every piece is
one column wide, it is plainly wrong. The smaller slots are now centred on the big one. The big
slot is untouched and Rustris's column is tidier for it.

**The game over card did not fit a six column board**, and that was luck rather than design
everywhere else: the card font is two blocks tall, so "game over" comes out 226px against
Dr. Rustario's 256px board and 213px against Rustris's 300px one, and 340px against Puyo's
288px. `modern_theme` now measures the widest card and shrinks the font to fit the board,
never growing it - so Dr. Rustario stays at 63 and Rustris at 60 exactly as before, and Puyo
drops 96 to 75.

**The HUD is down to the score, and the chain announces itself over the puyos instead.** Tsu's
HUD is the score and the nuisance tray; a chain is a thing that *happens*, and a running
best-chain counter in the corner is not something the game has. `Level` went the same way in
the same review - the speed step is something a player feels rather than reads, and on a board
that changes it every thirty puyos the number just sat there. So Puyo's HUD is **the score,
and nothing else**, in the side column under the queue; the left column is empty, since there
is no hold box either. In place of the chain row is a new engine concept,
`engine::animate::popup`:

* `GameRender::clear_popup(&self, event) -> Option<String>` - a short caption to draw over the
  cells a clear took. It is not `clear_word`: that writes across the whole window in particles
  and is for the once-a-match moments, while this is small, local and fires on *every* clear.
  Both existing games return `None` and are unaffected.
* `PopupAnimation` on `PlayerAnimations` holds them, one per clear, each on its own clock - so
  a chain leaves a trail of captions climbing the board rather than replacing one with the
  next. It never blocks the tick; a popup is decoration.
* `Theme::popup_font` is a `PopupFont` - two renders of the same face, one for the fill and one
  near-black for the outline, because a `FontRender` bakes its colour into its texture and a
  shadow is therefore a second font rather than a second draw colour. Every theme gets one,
  sized to its own cell, since whether there are popups at all is the *game*'s decision and a
  theme should not have to opt into a game's feedback. **Phase 3's retro themes owe this
  nothing** - and their popup font comes out at source-pixel size and scales up with the rest
  of the art, which is chunky and which is right for a retro theme.
* **It is drawn last of all, on the window, after the foreground particles** -
  `ThemeContext::draw_popups`, called by the match screen after `fg_particles.draw`. The first
  attempt drew it into the board texture with everything else, which put it *under* the clear's
  own particle burst - and the burst is precisely what is happening when a caption appears, so
  it was being lost. Drawing on the window costs the clipping the board texture gave for free,
  so the caption is held inside the board's own width instead, and the theme's `Scale` and the
  board's window position are passed in so the geometry is still worked out in the theme's own
  source pixels and mapped out at the end.
* Puyo returns `"{n} chain"` from the first step, per Alex: a 1-chain saying "1 chain" is what
  makes the second step reading "2 chain" mean anything.

**The caption is drawn in the colour of what popped**, which was the second half of that
review: a white caption over a busy board was getting lost. Where the colour comes from is the
part worth writing down, because the obvious answers are both wrong. The *game* cannot say -
it knows a `CellId`, not what a theme paints it. The *theme* cannot be asked to declare one per
cell - that is eighty entries for Puyo alone and every retro theme would owe the same again. So
`BlockSpriteSheet` **reads it off its own built atlas**: one pass at build time, averaging each
cell's sprite with pixels weighted by saturation times brightness, so an outline and the white
of a puyo's eyes do not wash the answer out. `cell_color(id)` is then right on any theme,
including one sliced out of a rip, and costs a game no new contract at all. A `Popup` carries
the *modal* cell of the group it is about rather than an average, since a group that took a
nuisance puyo with it should still be its own colour.

It is also a cell and a quarter tall now rather than three quarters, held to fifteen sixteenths
of the board's width and clamped inside it - a six column board is narrow and a caption over
the first column was hanging off the edge.

**And the particle theme says it in the game's own face**, once the rip turned out to carry
one: `theme/modern/popup.png`, cut by `rip.py`'s `popup` from the digits and the word `Chain!`
that sit under the effects at the bottom of the sheet. A theme offers art through
`PopupSpriteData` on `ModernThemeOptions` - a sheet and the *tokens* it can spell, each with
its rect - and `PopupFont` uses it wherever it can and falls back to the face for anything it
cannot spell, whole rather than in part. Three things are worth remembering:

* A token is whatever the sheet drew as one piece: a digit here and a whole word there, so
  `2 chain` is spelt from a `2` and a `chain` and not from six letters. The digits are on a
  fixed pitch, because a counter climbing from 9 to 10 that shifted its digits about would
  read worse than one that does not.
* Every cell of the sheet is the same height whatever it draws, because `rip.py` cuts each
  glyph against its *row's baseline* rather than its own bounding box. The round digits
  overhang the line by two or three pixels and the word sits twelve above it, exactly as the
  game drew them - so the whole caption is one row of cells drawn at one y, with no metrics
  to carry.
* It is **not tinted**. The colour of what popped is right for a caption written in a plain
  face; modulating a gold glyph towards a blue puyo just gives a dark gold. The art carries
  its own outline and shadow, which is also why it is drawn a cell and a half tall rather than
  a cell and a quarter.

Phase 3's retro themes still owe this nothing: without a sheet a theme gets the face, which is
what every theme had.

**One thing to know if you touch `PopupFont`:** tinting is `Texture::set_color_mod`, which is a
mutation, and a theme draws through `&self` - so the fill font sits behind a `RefCell`. That
makes `Theme<'a>` **invariant** in `'a`, where it used to be covariant, and code that built
themes in a local and handed out `&'a Theme<'a>` stops compiling. `Shell` was already leaking
its texture creator for the life of the process and was unaffected; `frame_shot` now leaks both
its texture creator and its theme list the same way, which is two lines and is what that
example wants anyway.

`MetricKind::Chain` stays in the engine. Phase 0 added it for this game *and* for Puzzle
Fighter and Bombliss, both of which want the same counter; it is simply not on a HUD today.

**Still open: how the speed step should work.** Tsu has no level that climbs with play. In 2P
versus the drop speed is fixed for the whole match by the difficulty setting - 16 frames per
cell from Easiest through Hard, 8 at Hardest (*Frame Data Tables, Drop speed*) - and never
changes; the game's answer to a match that drags is **margin time**, not gravity. Solo mode
does have stages and does speed up through them (the wiki notes that in the late solo stages
the pair falls faster than soft drop, so it cannot be slowed), but a stage there is *an
opponent beaten* and Puyo Nexus's solo drop-speed table is explicitly "(table to be
completed)", with only level 1 recorded.

So `PUYOS_PER_STAGE = 30` and the twelve step `FALL_DELAY_MS` curve are both invented, and this
document should not leave them unremarked. The reason a stage exists at all is that the shared
mode structure needs one - `MatchRules::StageSprint` and the level sprint are built on stages.
The options were to keep the ramp as a documented house rule and add margin time in phase 5 as
well; to drop the ramp for one fixed speed per difficulty, which costs the level sprint its
meaning here; or to ramp in single player only, where Tsu also ramps, and hold versus at one
speed.

**Settled 2026-08-27, by Alex: keep the levelling intact.** The ramp stays exactly as phase 1
built it, in single player and in versus alike - `PUYOS_PER_STAGE = 30` and the twelve step
curve are the house rule, and this game has a `Level` the way every other game in the
compendium has one. It is what makes the level sprint, the stage clear card and the speed band
scenes mean the same thing here as they do next door, and a compendium whose third game
quietly opted out of the shared mode structure would be the worse trade. **Nothing in phases
3, 4 or 5 revisits this**; margin time still lands in phase 5, on top of the ramp rather than
instead of it, and phase 3's retro themes each want their `scenes` and `board_snips` one per
speed band exactly as the other games' retro themes do.

---

## Phase 3 — retro themes

**Status:** `in progress` — 3a, 3b and 3e are `done` (2026-08-28); **3c was done and then
reverted** — `3ds` was cut from the repository on 2026-08-28, see its section; 3d is `in
progress`, `genesis` done and `snes` still on the particle theme's audio; **3f is `done`**
(2026-08-28). Puyo Rusto has three themes and that is the intended number. **3e was added on
2026-08-28** after Alex played the first three:
the panels were built from the rips alone and the rips disagree with the games, so every board
and every preview was out. See it for what was wrong and how each number was measured. **Split into 3a, 3b, 3c and 3d on
2026-08-27**, with Alex, because three themes is three separate slicing jobs that share one
piece of groundwork and nothing else, and there is no reason for the second to wait on the
third. 3a carries the groundwork and `genesis`; 3b is `snes`, which is the same game reskinned
and so the same job again for much less; 3c is `3ds`, which shares nothing with either and is
the most work of the three; 3d is the audio, deliberately left as placeholders until then.

**Goal.** Retro themes alongside the particle one, so Puyo can take its turn in the retro
playlist. It was three of them until `3ds` was cut; it is two.

### The three sources, and what they are called

| module | menu row | source | year |
|---|---|---|---|
| `genesis` | `genesis` | Dr. Robotnik's Mean Bean Machine — [rip](https://www.spriters-resource.com/sega_genesis/drrobmbm/) | 1993 |
| `snes` | `snes` | Kirby's Avalanche — [rip](https://www.spriters-resource.com/snes/kirbysavalanche/) | 1995 |
| ~~`3ds`~~ | ~~`3ds`~~ | ~~Puyo Puyo Chronicle — [rip](https://www.spriters-resource.com/3ds/puyopuyochronicle/)~~ — built, then cut, see 3c | 2016 |

Two things about that table were decided with Alex on 2026-08-27 and are not open:

* **Every theme is named for its platform**, which is the convention the whole repository keeps
  (`gameboy`, `nes`, `snes`, `n64`, `particle`): `genesis`, `snes`. Two earlier drafts put two
  Sega titles in the list and needed a region split to tell them apart; that problem does not
  exist any more and the reasoning behind it has been deleted rather than left lying about.

A third row was decided that day and has since gone: **Puyo Puyo Chronicle on the 3DS**, which
the slot wanted because it is *actually Puyo Puyo* rather than a reskin and is drawn for a
screen anything like the one this runs on. Both of those were true and neither survived being
played — 3c has what happened. If the slot is ever filled again, it wants something 16-bit or
thereabouts, and something whose sheet carries the frames a bean needs to pop.
* **`all_themes` runs them oldest first** — `genesis`, `snes`, `particle` — which
  is the order the other two games use and which makes the theme sprint a walk forwards through
  the hardware. That order *is* the theme sprint's, and it is the order the retro playlist
  alternates through, so it is a user-visible decision and not an implementation detail.

**"Retro" here means the engine's `ThemeFamily`, not the calendar.** What the retro playlist
and `Theme::family` sort on is *art-based theme* against *particle theme*, and nothing in the
code knows what year a game came out. That was written here to justify putting a 2016 handheld
game in this phase; it did not survive being played — see 3c, where `3ds` was cut partly for
being the theme nearest the particle one. The warning it carried is still worth keeping for
whatever fills that slot next: a theme cut from modern glossy art will read as a second
particle theme unless everything *around* the board separates it, and if it still does not,
raise it with Alex rather than quietly restyling one of them.

**The sheets are not in the repository and will not be.** Spriters-resource rejects automated
fetches (403 to anything that is not a browser), and megabytes of source for a few hundred KiB
of output is the same trade the Puyo Puyo Tetris sheet already lost. **Alex downloads them into
`puyo-rusto/art/`**; the agent writes the cutter.

Keep the existing convention exactly: the three sources already there sit under the *verbatim*
download name spriters-resource gives them, and the repository's root `.gitignore` names each
one by full path with a comment saying which script reads it. So each new sheet lands under
whatever it is called when it arrives and gains a `.gitignore` line to match, in the phase that
cuts it. Nobody renames a source file to something tidier — the name is the provenance, and it
is the only record of where the art came from.

Which means **every retro theme's sprite work is a script**, `puyo-rusto/art/rip_retro.py`, in
the shape `art/rip.py` already has: source path in, `src/theme/<module>/sprites.png` out, one
subcommand per theme and a `check` that draws the alignment board the way `rip.py check` writes
`art/alignment.png`. Re-run it rather than editing its output — that rule is in CLAUDE.md and it
applies here for the same reason it applies to the particle theme.

### What every one of the three owes

Everything is `include_bytes!` — there is no build script, no asset manifest and no slicing
tooling in the crate. Every rect is arithmetic written by hand in the theme's `mod.rs`.
Per theme, in a directory beside that file:

* `sprites.png` — one `source_block_size` grid holding the cells, the idle and pop strips and
  the previews. **The cells are the full `colour × 16` link grid** from phase 1, not one
  sprite per colour, plus nuisance and the three tray symbols. Unlike the particle theme these
  do not have to be authored: the rips carry them, because the original games drew connected
  puyos the same way — Kirby's Avalanche is ripped as a sheet literally called "Blobs &
  Boulders". Budget the time in slicing and arithmetic rather than in drawing. The previews
  cost nothing at all: `data.rs::previews()` composes all twenty five pairs out of cells the
  sheet already has, and phase 2 already wrote it.
* **All eleven skin slots, keyed to the same art.** `data::cells` walks `PuyoSkin::all()`
  because the particle theme has eleven sets of puyos in one sheet; a retro theme hands back
  the same points for every slot and pays only for the duplicate keys, which is exactly what
  that function's doc comment says it is for. The consequence is worth stating plainly so
  nobody reads it as a bug: **on a retro theme both players draw the same puyos**, because the
  original drew one set. `PuyoSkin::deal` still deals, and the deal simply does not show.
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
  retro theme passes `None`, so the field is there and empty to copy from. Two of the three
  rips draw the tray in their own art and the third does not; match the original where it has
  one.
* mascot strips `{idle,throw,victory,game-over}.png`. **This is where Puyo gets mascots at
  all** — phase 2 shipped `mascot: None` deliberately, having no art and no honest way to
  generate a four-strip character animation, and both of these rips carry characters (Robotnik
  and Kirby; Chronicle's cast went with its theme in 3c, and with it the one source here that
  had the series' own Arle and Carbuncle). The particle field's mascot silhouettes come with
  them for free, since it
  outlines the sprites of every theme in play rather than one.
* the twelve or so sound effects, and the music — **which phase 3d does, not 3a–3c.** See below.

Template for the 34-field `RetroThemeOptions`: `dr-rustario/src/theme/nes/mod.rs`.

Register each theme in three places: `theme/mod.rs::all_themes` (the order defines the theme
sprint), the `MatchThemes` enum in `game/rules.rs`, and that enum's `initial_index()`.
`options.rs::theme_mode()` reads `initial_index` rather than matching the variants itself, so
it needs no arm — that is a difference from the other two games, which do match by hand.

**Popups owe this nothing.** A theme without a `PopupSpriteData` gets the plain face, which is
what every theme had before the particle one grew a sheet; a retro theme's popup font comes out
at source-pixel size and scales up with the rest of the art, which is chunky and which is right.
If a rip turns out to carry a `Chain!` graphic, it is a bonus and not an obligation.

### Phase 3a — the groundwork, and `genesis`

**Status:** `done` — 2026-08-28

Everything above that is done once, plus the first of the three themes. The groundwork is two
debts phase 2 left here on purpose and one new file:

* **`all_themes` builds the particle theme twice.** `reference_block_size` measures themes that
  are already built, and with no retro themes there was nothing to measure. Delete the double
  build and take the reference off the retro themes, the way `dr-rustario` and `rustris` do.
  This can only be done once a retro theme exists, which is why it is 3a's and not phase 2's.
* **`visible_rows` is `ROWS` (13), not `VISIBLE_ROWS` (12).** The frame covers
  `visible_rows - top_buffer_rows`, so the hidden thirteenth row floats above it. Passing 12
  puts a playable row outside the frame, which is what happened first time.
* **`art/rip_retro.py`**, with its first subcommand. One cutter for all three themes rather than
  three scripts: they are the same job — find the puyo grid, cut `colour × 16`, cut the tray,
  the mascot strips and the font — and the differences are a table of offsets per source.

**`genesis` goes first because `snes` is the same game again.** Mean Bean Machine and Kirby's
Avalanche are the two western reskins of one Compile original, released two years apart and
built on the same board, the same beans in the same sixteen link states and the same layout —
so whatever the cutter learns in 3a it reuses almost whole in 3b, and any misunderstanding of a
bean's link grid shows up while it is still cheap to fix. It is also the oldest, so `all_themes`
is built front to back rather than assembled out of order.

**Done when:** `genesis` is on the theme row, the theme sprint is offered again (it comes back
on its own the moment `MatchThemes::count()` passes 1 — `options.rs`'s test says exactly that),
`frame_shot` renders it, and matching puyos join up on it the way they do on the particle theme.

### Phase 3b — `snes`

**Status:** `done` — 2026-08-28, and the *last* of the three rather than the second. It is not
"the cheapest of the three" the paragraph below promised: Kirby's Avalanche has no board, no
background and no font on spriters-resource, so it is the one theme the sheets alone cannot
finish. Everything but the blobs came out of the ROM instead, by rendering the SNES's own
background layers on their own — see the handover notes, and `SNES_LAYERS_BOTH` in
`art/rip_retro.py`.

Kirby's Avalanche, through the cutter 3a wrote and the theme 3a laid out — the cheapest of the
three, and second for that reason. Kirby is the mascot and its cells are the sheet literally
called "Blobs & Boulders". Nothing of the groundwork repeats; this is a rip, a `mod.rs` of
arithmetic, and an `all_themes` entry. Where it *does* differ from `genesis` is the palette and
the board furniture — the two games are the same code wearing different art, and the theme
should not end up a recolour of 3a's by accident.

**Done when:** `frame_shot` renders it, `menu_shot` walks the grown theme row, and the puyos join.

### Phase 3c — `3ds`

**Status:** `done`, then **reverted** — built on 2026-08-28 and cut from the repository the
same day, with Alex. The theme, its art and its whole section of `rip_retro.py` are gone;
`git log` has them if it is ever wanted back.

Two reasons, and the second is the one that cost something:

* **It is modern art in a retro slot.** Chronicle is 2016 and looks it. The note at the head
  of this phase already worried that it was the theme nearest the particle one; playing it,
  Alex's reading was that it is not a retro theme at all, and that the animation work the
  other two are getting (see 3f) would want sprites the Chronicle rip does not carry and
  nobody has cut.
* **Its panel sized the board for every other theme.** Every theme of a game is drawn at the
  largest cell *all* of them can hold, so the tallest panel decides it for the rest.
  Chronicle's was 290 source pixels tall for a 216 pixel field - a frame with air over it and
  a score strip under it - against `genesis`'s 224 for 192. It held Puyo's cell at 66 pixels
  where the two 16-bit panels could manage 76. Cutting it, and then taking back the spawn
  cell in 3f, moved the board from 858 pixels tall on a 1080p screen to 949.

What went with it: the `MatchThemes::ThreeDs` row, `three_ds` and its art, `THREE_DS_*` and
`three_ds()` in `rip_retro.py`, and the `THEME_DIRS` indirection that existed only because
`3ds` cannot begin a Rust identifier. `SceneType::Cover` was written for it and is left in
the engine - it is a general thing for a scene that is one painted picture, and the next
theme like that will want it.

### Phase 3e — the alignment pass (2026-08-28)

**Status:** `done`. Alex played 3a–3c and reported that the puyos did not line up with the
boards, worst on `genesis` where they sat "way above the bottom", and that the previews were
not aligned either. All three were wrong and all three are fixed. This is here because *how*
they were found is the reusable part: **every number below was measured against the emulated
game, not read off the rip**, and the rip was wrong every time the two disagreed.

* **`genesis` was missing the well's floor.** The boards sheet keeps the screen as the two
  planes the Genesis drew it on, side by side: the left one is the dungeon wall with the wells
  sunk into it, the right one is every stone border, every well floor and the boxes down the
  middle, over a flat `(0, 64, 64)` key. Cutting the left plane alone left sixteen pixels of
  open well under the last row of beans. `genesis_screen` composites the two before anything is
  cut from it; against a live frame the result agrees to within the rip's own colour rounding,
  and the back plane alone does not. The geometry was never wrong — the art under it was.
* **The frame plane's holes are the panel's boxes, exactly.** Flood filling the key gives
  `(16, 16, 96, 192)` and `(208, 16, 96, 192)` for the two wells, `(120, 32, 32, 48)` and
  `(168, 32, 32, 48)` for the two `NEXT` boxes and `(120, 96, 80, 56)` for the mugshot. Those
  are what `genesis/mod.rs` names now. A pair goes in each `NEXT` box — the game gives the
  second to the opponent, but a panel here belongs to one player, so the queue runs through
  both — and the tray goes in the mugshot box, five icons at a whole cell each.
* **`snes` was one pixel high.** A blob's eyes sit three rows into its cell; in a frame with the
  field full they land on 99, 115, 131 … 195, so the bottom row is 192 and the field starts at
  16, not the 15 the BG1 render read. Its queue goes under the two name plates (`(104, 38)` and
  `(128, 38)`, both 24 by 41, the two runs of white at row 31) and its tray across the mouth of
  the arch, `(104, 186, 48, 14)` — the only clear run in a forty-eight pixel column that is as
  wide as six icons, at half a cell each. The stage number is *filled* with the recess's own
  colour now rather than painted out with the black around it, which had left a black hole
  under `STAGE`.
* **`3ds` had the panel and the scene the wrong way round.** Chronicle stands both fields on one
  painted scene; the theme was giving each player a slice of that scene as a panel and filling
  the rest of the window with flat colour, so at 1080p a strip of scenery sat in a blue void.
  The panel is transparent now and the backdrop is the new `SceneType::Cover` — one picture
  scaled until it covers the window, centred, drawn with linear filtering. See below for the
  size it is written at.
* **The Chronicle skin sheet is not on one pitch.** Most of the top row is on nineteen, but a
  gap partway along shifts everything after it by a pixel and then by three, so the nuisance and
  the tray were cut three pixels off and the tray drew slivers of the tiles next door.
  `three_ds_row_tiles` finds the tiles by the page showing between them and the two constants
  are ordinals along that row rather than column numbers.

**`SceneType::Cover` is new engine, and it is the only scene here that is a painting rather than
a tile** — which is why it is the only one drawn with linear filtering: a tiled retro backdrop
scales by whole pixels and has to keep its hard edges, and a painted one has none to keep. The
picture is written at 1200x720: three times the 3DS top screen it is cut from, with Lanczos and
a light unsharp, and the last stretch to 1080p and beyond is `Cover`'s own filter. Three rather
than five because the art is painted — all soft gradients, no fine detail, so the interpolation
has nothing to lose — and 1200x720 is 685 KiB in the binary and 3.4 MiB of texture against 1.5
MiB and 8.6 MiB at full size. There is no neural upscaler in this pipeline and it does not need
one; a straight Lanczos blow-up of this art holds up at 1080p.

**A second round, the same day**, after Alex played the first: four more, all of them Kirby's
Avalanche except the last.

* **The wooden platform at the foot of the arch was gone**, and a black gap stood where Kirby
  stands in the original. Not a paint-out - the *grass*: the game's own score number is
  covered with a course of grass taken from beside it, and the course is sixty four pixels
  wide where what was left to cover was four, so laying a whole one down ran sixty pixels past
  the patch and over the platform. The last course is clipped to the patch now. Two more
  pixels of the game's number survived past the patch's right edge, so it runs to the foot of
  the column; and the grass is taken from clear of the `SC` label, because a course cut
  through it repeated a sliver of the label across the border.
* **The two name plates are painted out.** They label one queue for each player and this panel
  has both boxes to itself, so they were somebody else's. What is behind them is the plain
  dark of the column, read off the row underneath - which is what `snes_fill_flat` grew a
  `donor` argument for, since the plates fill most of their own region and it is the region's
  commonest colour that is normally the answer.

  Painting them out flat left a band of nothing under the `NEXT` sign, though, and a flat dark
  band in a wooden column reads as a hole. So the column's own woodwork closes both of the
  holes this panel is left with: the three posts that frame the queues are run *up* to meet
  the plank the `NEXT` sign is nailed to, so the boxes simply get taller and nothing is
  invented, and a whole course of plank is laid across the mouth of the arch, where the game
  stands Kirby and this one stands nothing. Both copy from the panel itself - the posts from
  the seven rows below the hole, the plank from the course between `STAGE` and the arch - so
  the wood is the game's own and no two courses are alike by accident. The tray stands on the
  new course. With the plates gone the two `NEXT` boxes are the gaps between the posts,
  `(108, 32, 16, 47)` and `(130, 32, 18, 47)`, which is what the posts actually frame.
* **The numbers were the wrong font, half the height they should be.** VRAM holds two numeric
  faces and the first pass took the wrong one: tile 896 is the small eight row face the game
  sets its menus in, and the one it prints a score in is **sixteen rows** - two tiles stacked,
  the top at 769 and the bottom sixteen further on, because the font is laid out sixteen
  glyphs to a VRAM row. Beside the fourteen row `SC` the panel keeps, an eight row number
  reads as a mistake, which is how Alex spotted it.

  Which tiles those are was not guessed either. The layer render happens to carry two of the
  game's own digits - the `0` of its score and the `1` in the `STAGE` recess - so both were
  masked off it and matched against a decode of all 2048 tiles: the `0` is 769 over 785 to the
  pixel and the `1` is 770 over 786, and nothing else in the sheet comes close. The four inks
  came out of the same pairing, index by index: 1 is the black outline, 5 the red `#E75163`,
  6 the pink highlight and 15 white. That palette is the *player's* - the left panel draws its
  numbers in the red its `SC` is drawn in and the right one in white - and this panel is the
  left one. The digits sit on an eight pixel pitch with no gap, so the font's spacing is zero
  and the score right aligns where the game's own does.
* **The level is a HUD row now, on every theme.** `MAX_LEVEL` was already there "so the HUD can
  size the digits" and no theme ever drew it. All three source games print it: Kirby's
  Avalanche and Mean Bean Machine both call it a *stage* and give it a box, and it is the
  same number the menu offers as the `level` to start on. It goes in the recess under `STAGE`
  on `snes`, where Mean Bean Machine prints its own on `genesis`, at the far end of the score
  strip on `3ds` (Chronicle prints no stage number anywhere, so there is no box to put it
  back in), and as a labelled row on the particle theme, which builds its own table. Placing
  it needed `data::hud`: every retro theme mapped one snip over `HUD_MAX`, which was fine
  while there was one row and drew both numbers on top of each other the moment there were
  two.
* **`genesis` gets its two words and its two faces.** Mean Bean Machine prints `NEXT`, `1P`,
  `DR R`, `STAGE` and `SCORE` down the middle of the screen and every one is a *text sprite*,
  so the frame plane carries none of them and the panel had the boxes and no words at all - a
  bare digit on stone says nothing. The fonts sheet has all of it, and reading it once serves
  the lot: every face on it is eight pixels wide on a nine pixel pitch, thirty glyphs to a row
  - the ten digits and then A to T - so a word is a lookup into that alphabet.

  Which face is which was measured, not picked. The game sets `NEXT`, `SCORE` and *both
  players' scores* in one bold sixteen row face, and `STAGE`, `1P`, `DR R` and the stage
  number in a smaller plain white one whose ink is nine rows inside its cell. The bold face is
  **green** on the sheet because green is the palette the labels take; a score is the same
  glyphs in the player's own. Matching the sheet's green `0` against the game's own
  `00007536` puts it beyond doubt - identical silhouettes to the pixel - and pairs each ink
  with the red it comes out as, which is written back in the sheet's own scaling since the
  sheet and a frame of the game are the same eight levels per channel scaled differently.
  A first pass had taken the big white `0123456789FINALSTAGE` row for the score face; that
  one is `FINAL STAGE`'s and a different shape, and it drew the stage number in it too.

  The swap asserts that **nothing green survives it**. The bold face has seven shades and two
  of them are a couple of dozen pixels across the whole row, so a shade left off the table
  lights up an edge on three of the ten digits and nowhere else - which is exactly how the
  seventh was found.

**Done when:** the bottom row of puyos rests on the floor of every retro board, the queue and
the tray sit in furniture the source game actually drew, every panel says what its numbers
are, and the `3ds` backdrop fills the window at 1080p. All three check out in `frame_shot` at
1920x1080 and 640x480, one and two players.

### Phase 3d — the retro soundtracks and sound effects

**Status:** `in progress` — 2026-08-28. **`genesis` is cut and wired, with seven of its sound
effects waiting on a listen** (see below); `snes` still plays the placeholder, and `3ds` is
gone (see 3c). Until 2026-08-28 all three shipped placeholders, which was Alex's call of
2026-08-27 and not a shortcut taken quietly.

**The placeholder is the particle theme's audio, referenced rather than copied.** A retro theme
hands `data::audio` the same `Sounds` the particle theme does. The music half is already free:
`theme::GAME_MUSIC` sits at crate level in `theme/mod.rs` precisely because "phase 3's retro
themes are the same game's music and walk the same menus", it is `pub`, and `modern/mod.rs`
passes `music: &GAME_MUSIC` today — a retro theme writes the same line.

The effects half costs **one line of groundwork in 3a**: the fifteen `include_bytes!` live in a
*private* `mod sound` inside `theme/modern/mod.rs`, so a sibling module cannot see them. Make
that module `pub(crate)`, or lift the effects up beside `GAME_MUSIC` the way the menu clicks
were already lifted. Either way `include_bytes!` of one path embeds one copy however many
modules name it, so the placeholder costs nothing but the wrong period sound.

One trap for a theme that later takes its own soundtrack: `GAME_MUSIC` is an array whose type
carries a fixed length, so the tables cannot drift apart — but `Sounds::music` is a *slice*, so
a theme handing over its own table of a different length would compile. If a retro theme gets
its own tracks, they are four. *(Amended: this was written while a `music` menu row pinned a
track by index. That row is gone — see below — and the length is `theme::GAME_MUSIC_TRACKS`
now, but the constraint is the same one.)*

What 3d then does, per theme, is what `art/music.py` and `art/sfx.py` already do for the
particle theme:

* about twelve sound effects as **OGG Vorbis at exactly 44,100 Hz, mono or stereo** — the
  decoder rejects anything else outright, and a third of any rip will be 48 kHz. Trim the
  padding off both ends; do **not** normalise.
* the music, if the theme is to have its own. `theme::GAME_MUSIC` is shared and a retro theme
  may simply keep handing that table back — the four tracks are this game's whatever it is
  drawn as. A theme wanting its own soundtrack passes its own table instead; the menu's `music`
  row indexes whatever it is given, **so the two tables have to stay the same length** or the
  row means different things on different themes. Each track splits into `-intro.ogg` and
  `-repeat.ogg` for the intro-then-loop chaining — `art/music.py` does the splitting, and the
  split is cut on raw pcm so the seam lands on the sample the loop point names.

The sources are three more rips that are not in the repository and will not be, on the same
footing as everything else in `puyo-rusto/art/`.

**Done when:** every theme plays its own period audio and `frame_shot` still runs (which is
proof every ogg decodes, since the theme builder decodes them all).

#### What `genesis` actually did, 2026-08-28

`puyo-rusto/art/retro_audio.py genesis`, cutting `src/theme/genesis/` out of a FLAC rip of the
soundtrack and a WAV rip of the effects under `art/retro/genesis-{music,sfx}`. 4.4 MiB, ogg
q6, and a test in the theme that decodes all twenty three.

* **The loop points are measured, not read.** The rip renders intro + loop *twice* + fade and
  its `meta.md` gives the loop only in whole seconds, which is a third of a bar out at this
  tempo. So `loop_length` cross-correlates the render against itself - the minimum is sharp,
  a sample either side and the residual doubles - and `loop_start` takes the longest run of
  quarter seconds that match a loop later, at 0.9 rather than at 1 because the two passes are
  the same performance and not the same samples. The rip's two numbers are the *assertion*.
  Landing late on the loop start is harmless and landing early is not, which is why the search
  never reaches back before the run it is sure of.
* **The game writes each stage's lead-in as its own track**, which is exactly the pair
  `StructuredMusic::new(intro, repeat)` takes, so the four tracks are the four stage tunes with
  their four lead-ins. Victory is `Victory!`; there is no track called game over, so a burial
  gets one pass of `Continue`, which is what the game plays when you lose.
* **The effects had to be levelled, which contradicts `sfx.py` on purpose.** All sixty files in
  that rip peak at the same sample value - it is peak-normalised, so the mix is gone and a bean
  settling would arrive as loud as a fanfare. Each effect is scaled to the peak of the particle
  theme's sound for the same slot, read off `src/theme/sfx/` at run time rather than written
  down. Mean Bean Machine has no hard drop, so that slot takes the nearest noise the game owns.
* **The `music` menu row is gone, and so is the pinning seam behind it**, which is Alex's call
  of 2026-08-28. The row named the particle theme's four tracks (`korobeiniki`, `decisive` ...)
  and those names could only ever be right on one theme; with a second soundtrack in the game
  they were wrong on `genesis` and would be wrong again on `snes` and `3ds`. With nothing left
  to pin, `GameMusic`, `Options::music_choice`, `MatchSettings::music`,
  `ThemeContext::set_music_choice` and the engine's whole `MusicChoice` enum went with it;
  `AudioTheme::choose_game_music` is `deal_game_music(rng)` now and `with_game_music_choice` is
  `with_game_music_track`. What is left is `theme::GAME_MUSIC_TRACKS`, one number every theme's
  table is as long as - so a theme with a shorter soundtrack would be heard less often rather
  than differently, and a retro theme cutting its own knows it has four to find.

**Open: `genesis`'s sound effects are waiting on Alex's ear.** The music is settled - the rip
names its tracks and the loop split is measured - but the effect rip names only the sounds
whoever made it recognised and numbers the other twenty six `sfx_N`, so seven slots were filled
by inference from the names and the spectra rather than by hearing the game. Playing it was
tried and abandoned: RetroArch freezes on this machine about fifteen seconds into a session, so
the sounds could not be triggered and recorded. **These are the guesses, and each is one line
of `GENESIS_SFX` in `retro_audio.py` and a re-run away from being corrected:**

| slot | taken from | the doubt |
|--|--|--|
| `rotate` | `puyo_sine.wav` | a 16 ms 744 Hz blip; `sfx_11.wav` (88 ms, 1.9 kHz) is the other candidate |
| `lock` | `puyo_blob.wav` | which way round `puyo_blob` and `puyo_blob_2` go |
| `settle` | `puyo_blob_2.wav` | ... the same question, and whether either is really the settle |
| `garbage` | `bad_puyos.wav` | the plural, read as the shower of them landing |
| `attack` | `bad_puyo_1.wav` | reading the three singulars as sizes of a *send*; the game may have no send sound at all |
| `pause` | `select.wav` | most likely the menu confirm rather than the pause |
| `hard-drop` | `short_noise.wav` | the game has no hard drop, so this is a substitute either way |

Not in doubt: `move`, the four `chain_N` steps (they are a rising ladder - 1456 Hz, 2018, 2139,
2280 - which is the sound Puyo is known for) and `level_start`. Also wanted: whether `Victory!`
and `Continue` are the right tracks for a win and a burial.

**What is left after that:** `snes` (Kirby's Avalanche), and only that one now `3ds` has been
cut. It writes its tables into its own `mod sound` and its own `GAME_MUSIC` the way
`genesis/mod.rs` does, and needs four tracks. `retro_audio.py` grows a subcommand for it;
whether its rip is laid out the way Mean Bean Machine's is is the first thing to find out, and
`split`'s assertion is what will say.

### Phase 3f — how a bean pops, and how tall the board is (2026-08-28)

**Status:** `done` — 2026-08-28. Added after Alex played the retro themes. Four things he
asked for, in his words: close the top of the `genesis` board so beans spawn behind it; pop
the beans the way the game does; blink the refugee bean; and make the board fill more of the
screen's height on every theme. The fourth is why `3ds` was cut - see 3c. **The first he
reversed after seeing it**, and the reversal took two more goes; that is the first note below.

**The board is thirteen rows now, and the thirteenth has nothing behind it.** This took four
goes and all four are worth recording, because Alex asked for the first, saw it, and reversed
himself, and then narrowed the reversal twice.

1. **Closed.** A `covered_top` on `RetroThemeOptions` - transparent rows over the board art
   alone, which the background is then drawn over - put the spawning row behind Mean Bean
   Machine's course of stone and Kirby's hedge, and a pair was revealed as it fell in. It
   looked right and it is wrong: the row above the field is *played in*. A puyo up there is
   still in the game, still part of a group, still what a stack dies against, and hiding it
   reads as a bug the moment a board fills. `covered_top` is gone; nothing else used it.
2. **Open, with the board's own art grown a course.** The rip grew a row of each board's own
   texture over the top - found rather than picked, by asking which course of the art best
   continues into its top row (Genesis came out at row 20, one cell short of the wall's 36 row
   period; Kirby at row 8, deep in the canopy). It read as a *taller well* rather than as room
   above the board.
3. **Open, art stopping at the field, panel still a row taller.** The board art carried an
   empty cell and the panel's hole was cut a row taller, so the spawning row was a notch in
   the panel with stone either side of it. Nearly right, and the sides gave it away.
4. **Open, panel cut level with the field** - which is where it landed, and is exactly what
   the retro Rustris themes do. Both panels are cut at the top of their own field, both boards
   are the twelve rows their games drew, and the thirteenth row is `top_padding` above the
   panel and the board alike: a cell of bare scene with the panel below it and nothing to
   either side. Mean Bean Machine's course of stone and Kirby's hedge go with the cut, the
   hedge over the queue's column as well as over the field, so the panel is level all the way
   across.

The happy accident of (4) is that **a point in either padded background is a point on that
console's screen**, which `genesis` always relied on and `snes` now gets for free: every
`+ TOP_PADDING` that `snes/mod.rs` used to add to its HUD, queue and tray coordinates is gone.

**The scenes are vignettes and were tiles**, which is the other half of the same problem: with
the row open, a puyo spawning above the board had the same hand-scattered stone behind it that
the panel below it is made of, and neither plane read as being in front of the other. Four
backdrops were rendered and shown to Alex - the game's own tile, that tile dimmed, a flat
colour, a `Checkerboard` of two close shades, and a vignette - and he took the **vignette**.
`rip_retro.py`'s `vignette` writes it: a 96x54 png of the backdrop's own colour, lifted in the
middle and falling away to the corners, drawn through `SceneType::Cover`, which scales one
picture over the window with linear filtering - so it is smooth at 4k for a couple of
kilobytes, and `Cover` has a second user now `3ds` has gone. `background-tile.png` and the code
that cut it are gone from both themes.

**And the panel casts a shadow on it**, which is what actually lifts it: `PanelShadow` on
`RetroThemeOptions`, declared once for both themes in `theme::data::panel_shadow` (3 px down
and right, 5 px of spread, black at 0xa0). Four things about it are worth knowing.

It falls **down and to the right only**, which is a light over the panel's top left shoulder
and was Alex's call on seeing the first version. Every ring keeps the body's own top left
corner rather than being centred on it, so the top and left edges of the shadow are inside the
panel and never drawn. A centred ring puts a band of shadow along the panel's top edge, which
is exactly where a spawning pair is the only thing standing on the scene.

It does **not** move with the hard drop's ricochet, and should not: the impact offsets the
*board* texture inside a panel that stays where it is - which is what every retro theme in the
repository has always done - so a shadow that shook with it would be the shadow of something
that had not moved.

It is **not painted into the panel art**, which is where it would naturally go. A panel is
measured in source pixels and every theme of a game is drawn at the largest cell all of them
can hold, so a margin round the art comes straight off the board: in one player the panels are
height-limited and there are 11 free pixels, but in two players they are *width*-limited at
exactly 73, and eight pixels of margin takes the cell to 68. Drawn at composite time it costs
the layout nothing and is free to fall outside the player's own area, which is what a shadow
should do.

And its `skip_top` is the theme's `top_padding`. Cast from the whole padded box, the shadow put
a dark rectangle behind the spawning row - the one row the whole exercise was about giving a
clear backdrop to.

**Kirby lost its `NEXT` sign to the level cut and got it back, twice.** The game nails the sign
*above* the top edge of the field, so cutting the panel there took the top half of it. The
first fix moved the letters and the hedge behind them down onto the plank - and sawed the plank
in half, because the plank was still where it was. What moves now is the **whole assembly**,
letters and plank together, rows 7 to 31 of the screen, down by nine. The plank lands exactly
over the game's two name plates; they named one queue per player and this panel has both boxes
to itself, so they had to go whichever way, and the plank covers them outright. The paint-out
that used to erase them and the woodwork that used to fill the hole are both gone from the
script - checked by re-cutting the panel and diffing it, which came out identical.

**What the height is worth.** At 1920x1080, one player: the cell was 66 pixels and is 73, the
board 858 pixels tall and 949. Two separate steps. Cutting `3ds` took the cap from 66 to 71
(its panel was 290 source pixels for a 216 pixel field); the two retro panels are 208 rows plus
the spawning cell, which is 224 - their console's own screen height - and that is 76. It does
not reach 76 because the *particle* theme is then the tallest: it is built at whatever the
retro themes can reach, and its own frame, border and pending strip need about 1.7 cells more
than the board, so `BoardLayout` settles everyone at 73. Getting past that means the modern
theme's furniture, not the retro panels'.

**The pop is four beats and the sheet had all of them.** Mean Bean Machine's beans sheet
carries, per colour band: a surprised face (the *last* frame of the animation strip at the
top, not the first - the first is the white flash), a whole-cell ball, an eight pixel ball and
a six pixel droplet, the last three sitting in a block under the arrangements beside the halo
and wings the same bean wears as an angel. `genesis_animations` in `rip_retro.py` composes
them into `animations.png`, one strip per row, the droplet frames being four copies of the one
droplet flown out from the middle at two distances. The refugee bean has no ball of its own,
so its pop is the sheet's white-outlined refugee and then the shrunken one; and the shrunken
one is also its **blink**, on a three frame idle strip with a two second pause on the
eyes-open frame.

Three things in the engine, all small:

* `DestroyStyle::Pop` gained a `duration`, because it was a fixed 300 ms and Puyo's own
  `POP_DELAY` is 280 - so the animation would have set the pace of a chain rather than fitting
  inside it. `genesis` asks for 240.
* A strip's frames have to be **edge to edge**: `non_exclusive_linear` addresses frame *i* by
  counting frame widths from the strip's start. Only the rows of the sheet are spaced apart.
* Nuisance is a `Cell::Garbage` and that arm drew the still sprite, so it could never idle.
  It now goes through `draw_stack_cell`, which without an idle strip is the identical draw -
  which is every other theme in the repository.

**Verified:** `cargo test --workspace` green; `frame_shot` on all three themes; the pop and the
blink were stepped frame by frame through a throwaway example (drawn, looked at, deleted) since
`frame_shot` replays only the caption of a clear and not the clear's own animation. The pop and
the blink have been played by Alex and are what he wanted; the open board has not been played
at the time of writing, only shot.

### Handover notes

Worked on 2026-08-28. **3a, 3b and 3c were all `done` at the time this was written; 3c has
since been reverted and Puyo Rusto has three themes, not four** — read 3c and 3f before this
paragraph. `cargo test --workspace` is green (753 tests, up from 746); `puyo-rusto` is 180 of
them and is clippy clean. `cargo run --example frame_shot -- 900 700 2 out/ puyo` draws
`genesis`, `snes` and `particle`, and `menu_shot` walks the grown theme row.

**The order the phase was done in changed, and the plan above is amended to match.** 3a was
written as "groundwork plus `genesis`" and that is what it is; but `snes` turned out to be the
one theme whose rips carry no board, no background and no font, so it is **not** the cheap
second theme this document promised - it is the one that needed the ROM. `3ds` was done second
because Chronicle's rips carry everything, and `snes` last. Do not read 3b's "cheapest of the
three" against what actually happened.

#### `art/rip_retro.py`, and the one idea in it

One cutter, one subcommand per theme, in the shape `rip.py` already had. What is worth knowing
is how the sixteen link variants are *found*, because no two of the three sheets agree and none
of them says which sprite is which:

* **Mean Bean Machine's sheet has no grid at all.** PicsAndPixels arranged the beans as
  *groups* - a page of two-, three-, four- and five-bean shapes drawn as they appear in play.
  So `link_grid` reads the arrangement: a group is a connected run of non-background pixels
  whose bounding box is a whole number of cells, a cell in that box is occupied or it is not,
  and a bean's mask is which of its four neighbours are occupied. One pass yields all sixteen
  of all five with no coordinates written down by hand. It also refuses to write a sheet with
  holes in it, which is the check that matters.
* **Kirby's and Chronicle's sheets are grids in the game's own order**, and that order is not
  this game's. Both were read rather than guessed, by the same trick: a neck runs to its cell
  edge - that is what makes two joined puyos meet flush - so which sides a variant is joined on
  is which edges its art touches *in the middle*. Kirby's sheet indexes bit 1 up, 2 right, 4
  down, 8 left. Chronicle's is `THREE_DS_ORDER`, and it is not separable into a row part and a
  column part; what says it is right is that **all five colours decode to the identical
  permutation**, which is five independent confirmations of a sixteen entry table.
* **`assert_no_marker` is the guard that earns its keep.** Mean Bean Machine's sheet has three
  flat fills painted behind the beans - a green, a pale green and a pink - and nothing on it
  says so. Two were listed, the sheet was cut, and the third was still there in the output. So:
  puyos of different colours share only their black rim and their white eyes, and any *other*
  colour dominant in cells of three or more colour rows is a marker rather than art. It names
  the colour so the next sheet's marker is one line to fix.

`python3 puyo-rusto/art/rip_retro.py check` writes `art/retro-alignment.png` (gitignored): every
theme's cells drawn as a board that uses all sixteen masks, side by side. It is `rip.py check`'s
twin and it is the only way to see a seam.

#### The gotcha that cost the most: **a retro theme's background needs a hole in it**

The board frame is drawn *under* the background - `draw_players` composites the board texture
first and the background over it - so a panel that carries its own well covers the board and
every cell on it. The first `genesis` build drew a perfect empty field with the queue, the tray
and the score all correct beside it, and that is a very convincing way to look like the board
is broken. Both of the other games' retro themes cut the same hole and **nothing says so
anywhere but the art**: `dr-rustario/src/theme/nes/background.png` has 18,044 transparent
pixels and `rustris`'s has 23,838. `rip_retro.py` now punches it, and there is a test in each
theme that the hole and the board agree.

The other one, smaller: **`board_snips` are into the *padded* board texture.** A theme that
supplies them (as `3ds` does, one frame per speed band) must add `top_padding` to their height,
or the bottom row of the board is left outside the copy - which looks exactly like a board
whose stack is sitting one row too low.

#### `genesis` - Dr. Robotnik's Mean Bean Machine

Sixteen pixel beans, six columns, twelve rows: the Genesis board *is* this game's board, so
nothing is scaled. The panel is cut straight out of the third of the four 320x224 boards on the
boards sheet - **measured, not picked by eye**: the four were scored against a live frame of the
emulated game and that one is 82.6% identical to it, where the next best is 47.8%. The well is
at (16, 16), 96 by 192, which is the geometry the emulator gave.

The rest: the refugee bean is the nuisance puyo; the tray's three symbols are the refugee at
three weights, since Mean Bean Machine lands an attack the moment it is sent and has no tray to
draw. The font is the big white face the game sets FINAL STAGE in. The scene is a course of the
dungeon wall taken out of the left border, tiled - there is no seamless tile in the board art,
the stone is hand scattered and repeats on nothing, so it tiles with a visible horizontal band
and that is fine: courses of stone are what a wall has.

#### `3ds` - Puyo Puyo Chronicle

The rip carries a whole theme: four field frames, the field's own dark blue puyo print, the
game's own digits, and twenty two 400x240 in-game backgrounds. Two things are scaled, and both
are noted in the module:

* The frame's interior is 100 by 180 where twelve rows of an eighteen pixel puyo is 216, so the
  frame is scaled up **uniformly** until its interior is twelve rows tall, and the six columns
  sit centred in the width that leaves. A rounded corner stretched on one axis reads as a
  mistake; six pixels of slack either side reads as padding.
* The background is one of the game's own top screens, scaled to cover a player's panel and
  cropped from the middle.

**The four frames are `board_snips`, one per speed band**, so the field changes colour as the
pairs come down faster. That is a liberty - Chronicle does not do it - and it is the only one.
The score goes *under* the field rather than beside it, because seven digits of the game's own
face are wider than the column beside the board, and because that is where Chronicle puts it.

Chronicle's cells are 18 wide by 17 tall on the sheet - not square - so the cut squares them off
by repeating the last row of art once. That is what closes a downward neck against the puyo
below, and it is `rip.py`'s `repair` argument at one pixel instead of eight.

#### `snes` - Kirby's Avalanche, and the layer switch

Kirby's Avalanche is Compile's Puyo Puyo again, so its board is this game's board exactly and
the blobs cut like the others. **Everything else had to come out of the ROM**: Blobs & Boulders
is the only playfield art on spriters-resource for this game - no board, no background, no font
among the five sheets.

Not by screenshotting the board, which has the blobs, Kirby, the opponent's portrait and both
players' HUDs drawn into it. By **the SNES's own layer switch**, which is Alex's suggestion and
is much better than what was being attempted before it:

* `$212C` is the main screen designation - a bit per background layer and one for the sprites.
  snes9x keeps it in `Memory.FillRAM`, which is a block of every savestate it writes. So take a
  state mid-match, poke that one byte, load it back, and the emulator renders whichever layers
  you asked for and nothing else. **Set `savestate_file_compression=false`** and the state is a
  plain file with `FIL` in it; otherwise it is `#RZIP` and has to be inflated first.
* Kirby's Avalanche plays in **mode 2** with `$212C = 0x13` - BG1, BG2 and the sprites. `0x03`
  gives both backgrounds without the sprites, `0x02` gives BG2 alone. That separates the forest,
  the grass border and the score line (BG2) from the wooden centre column, the flower border and
  the blobs in play (BG1).
* **The field is on neither layer.** It is the backdrop colour, so the forest simply shows
  through it - which is why `board.png` is a crop of the BG2 render rather than a frame of its
  own, and why the field's geometry could be measured by looking for the one flat run of the
  backdrop colour in the BG1 render. It is at (8, 15), 96 by 192: six columns and twelve rows of
  sixteen, the same board Mean Bean Machine has.

Three things are painted out of the panel, all of them the game's own HUD rather than art: its
score number (the `SC` label is kept, and this game's score is drawn right after it), its stage
number, and two blobs standing in the arch at the foot of the centre column - which survive
punching the field out because they are not in the field. The last two are *found* rather than
measured: a blob is the only saturated thing on the arch's black floor, so the patch is the
bounding box of whatever is saturated there filled with the colour around it.

The font came out of VRAM. The savestate's `VRA` block is all 65,536 bytes of it, the game's
whole tile set is legible in `ra.py tiles --format snes4 --file`, and the digits are ten 8x8
tiles using three indices - nothing, a dark outline and a white fill, which is exactly how they
are drawn on screen. So `snes_font` decodes them straight out of the state and inks them; no
palette needed, because a two-colour font does not have one worth finding.

**What was tried first and did not work**, so nobody repeats it: reading CGRAM out of the
savestate to colour the tiles by hand. snes9x publishes no memory map at all (`READ_CORE_MEMORY`
is empty; only 128 KB of WRAM through `--raw`), so the palette has to come from the state, and
no 512 byte window of the `PPU` block decodes as 256 plain BGR555 words in either byte order.
The layer switch made the whole question moot - the emulator colours the layers itself.

#### Groundwork, and the debts phase 2 left

* **The sound effects moved up out of the particle theme**, to `src/theme/sfx/`, and their
  `include_bytes!` sit in `theme/mod.rs`'s `sound` module beside `GAME_MUSIC`. They are the
  *game*'s sounds, not that theme's - a bean settling and a puyo settling are the same event -
  and a retro theme plays them until it has a rip of its own. `art/sfx.py` writes there now;
  re-run it rather than moving files back. One copy is embedded however many modules name it.
* **`all_themes` no longer builds the particle theme twice.** The reference block size comes off
  the retro themes, which are built first, exactly as `dr-rustario` and `rustris` do it.
* **`visible_rows` is `ROWS`,** and both new themes pass it, with one transparent cell of
  `top_padding` for the thirteenth row to float in.
* **The theme sprint came back on its own** the moment there was a second theme, as phase 2 said
  it would. `options.rs`'s two theme tests were rewritten to say so rather than deleted.
* **`MatchThemes` is `all`, `genesis`, `3ds`, `particle`** - oldest first, particle last. `3ds`
  is the menu row only: the variant is `ThreeDs` and the module `three_ds`, because an
  identifier cannot begin with a digit and no raw identifier rescues one.

#### RetroArch, for whoever drives it next

**`--set video_vsync=false` or the session hangs.** Started with vsync on, RetroArch answers the
command port for a few seconds and then stops replying to everything - status, screenshot, input
- while the process stays alive and asleep in `poll`. It is the GL swap blocking the main loop
once the display idles, and there is no error anywhere to say so. `--set video_threaded=true`
alongside it did no harm. This is worth adding to the skill's own gotchas.

`video_driver=sdl2` does not start at all in this build - it dies before the command port opens.
`genesis_plus_gx` and `snes9x` both publish no memory map; only `--raw` WRAM.

#### 3d is untouched

Both new themes play the particle theme's sound effects and `theme::GAME_MUSIC`, which is the
placeholder 3d replaces. Nothing about that changed.

---

## Phase 4 — the ai

**Status:** step 1 `done` — 2026-08-28. Step 2, the neural model, is `todo` and deliberately
not provisioned for: nothing in `game/ai/` is shaped around one, and adding it means adding a
`PuyoAiKind` variant beside `Scorer`, the way `DrAiKind` carries both.

**Goal.** Puyo fields the same four difficulties and the same two demo models as the other
games — for real this time. Phase 2 already put the four names on the menu behind a
placeholder that plays at random, because the tests demanded it; this phase makes the menu
honest. `VersusAi` cannot deal a Puyo board worth playing against until it does.

Follow the Dr. Rustario precedent — a deterministic scorer that actually plays, with a neural
model alongside it, dispatched through a `PuyoAiKind`.

1. **A hand-written evaluator with a shallow beam search** — `done`. `game/ai/` is seven
   modules: `field.rs` (the board the search thinks on), `quiet.rs` (chain potential),
   `eval.rs` (fifteen weights), `placement.rs` (two move generators), `beam.rs` (the search),
   `skill.rs` (the six rows) and `harness.rs` (`ga puyo play` and `ga puyo rank`).
2. **A neural model** through `feature_network!` over the same features, trained by
   `ga puyo auto` on the existing `Fitness` seam. `todo`. It ships only when it beats the
   scorer; until then the scorer plays, exactly as `DrAiKind` works today.

`ga puyo play <seed> [difficulty] [pair cap] [report every n pairs] [brain]` plays one brain
headless; `ga puyo rank [seeds] [pair cap] [difficulty]` plays every row over the same seeds
and prints the `SKILL_ORDER` to paste back.

### What was built, and what came from where

There is no decomp to port here, which is the one way this differs from Dr. Rustario. Mean
Bean Machine *has* a disassembly (`DevsArchive/mean-bean-machine-disassembly`) and its cpu is
findable in it — `sub_56C0` calls `sub_12E6C` when `control_player_1` says the player is not
human, and that walks the pair towards a target column and rotation decided behind `sub_12F82`
— but it is unlabelled 68000 rather than the readable C `aiset.c` was, and the game's cpu is a
beginner's opponent besides. Puyo VS has an ai (`Puyolib/AI.cpp`, GPL-3.0) and it is two
hundred lines: take the biggest chain on offer, and if the best is nothing or a single pop,
place at random away from the spawn column. That is the `greedy` row, roughly, and it is the
bottom of the ladder.

So the shape came from the open literature instead, and mostly from
[ama](https://github.com/citrus610/ama) (MIT), the strongest open Puyo Puyo Tsu ai and small
enough to read end to end — its evaluation is fifteen weights in one file, where
[puyoai](https://github.com/puyoai/puyoai)'s `mayah` has about a hundred. Also
takapt's beam search idea (searching past the queue down invented continuations, by way of
ama's six fixed ones) and Ikeda, Tomizawa, Viennot and Tanaka's *Playing PuyoPuyo*, which both
of those cite.

**The quiescence search is the thing.** `quiet.rs` is what separates a bot that plays from one
that tidies: for every column a pair can still be carried over and every colour already on the
board, drop puyos one at a time until a group of four forms, run the chain out, and report what
it would have been. A placement's *own* chain is easy to see, but a building player almost
never fires anything, so scoring that says nothing about nearly every placement on offer. What
matters is the chain the field is holding, and this is how the evaluation is told about it.

### The ladder, as measured

`ga puyo rank 12 600` on 2026-08-28 — twelve seeds, six hundred pairs each, on `normal`,
ranked on score banked:

| row | weights | width | queue | ahead | queues | fires at | score/pair | best chain | steps | ms/pair | ms/step |
|--|--|--|--|--|--|--|--|--|--|--|--|
| greedy  | greedy    |  1 | 0 | 0 | 1 | anything |  48.4 |  4 |  1 |  0.02 | 0.02 |
| tidy    | freestyle |  6 | 1 | 0 | 1 |  6 nuisance | 191.1 |  7 |  1 |  0.53 | 0.53 |
| swift   | fast      | 12 | 2 | 0 | 1 | 12 nuisance | 284.4 |  8 |  4 |  1.97 | 0.49 |
| builder | build     | 16 | 2 | 1 | 1 | 18 nuisance | 433.0 | 10 |  6 |  4.71 | 0.79 |
| patient | build     | 20 | 2 | 2 | 1 | 30 nuisance | 571.9 | 12 | 12 |  8.38 | 0.70 |
| sharp   | build     | 16 | 2 | 2 | 2 | 48 nuisance | 761.8 | 12 | 12 | 10.56 | 0.88 |

The first run of it had `patient` and `builder` within four percent of each other for twice
the search, which is what made the three rows sharing the `build` weights differ in *how long
they hold a chain* rather than only in how hard they think. `easy` and `normal` play the two
weakest, `hard` the runner up and `impossible` the best, which is the shape Dr. Rustario's
four difficulties already had.

**One thing is known to be left.** The measure is a solo marathon, where no nuisance ever
arrives, so it ranks what a row *builds* and not how it takes a hit; ranking the rows against
each other is phase 5's to want.

### Thinking across frames

The hardest row takes 10.6 ms to decide a pair on a desktop and would take a tenth of a second
on a handheld, which is a stall you can see. The answer is not a smaller search — it is that
**the agent has no need to answer in a frame**. A pair takes a second or more to fall, which is
sixty frames, and nothing is waiting on the answer until it lands.

So `beam::Search` is a state machine rather than a function. `Search::new` plays every
placement of the pair in play, scores them and stops; each `step` after it plays the next pair
onto eight more of the boards being held and hands the frame back. The agent calls it once per
frame. What that buys is the whole of the difference between the last two columns above: the
same search, the same strength, at 0.88 ms a frame instead of 10.6 ms in one lump. It is worth
roughly a twelvefold budget, and it costs no board of the search at all — which is why the
ladder was **not** also scaled down under the `portmaster` feature. If a device still cannot
afford 0.88 ms a frame, `width` is the dial and `SearchConfig::steps` is what a measured think
time is divided by to find out; `ga puyo rank` prints all three columns and is compiled into
the handheld build, so measuring it on the device is one command.

Two things follow from thinking slowly, and both are handled in `agent.rs`. The pair goes on
**falling** while the search runs, so the keys are worked out again at the end from where the
pair *is* rather than reused from where it was — `root_moves` costs no evaluations, so running
it twice is free — and if the placement it settled on can no longer be reached, the next one
down the order is taken, which is why `beam::ranking` hands back an order rather than a winner.
And the pair may come to **rest** before the search is done, on a board too full to fall
through, so the search has to be interruptible. It is, and that is what putting the root layer
in `Search::new` is for: every placement is scored before the first `step`, so there is always
an answer and every step after only sharpens it.

**The ghost row is worth a feature of its own** — it got two. A puyo in the hidden thirteenth
row cannot pop and does not count towards a group, so a chain with a foot up there is *held
back* until it drops. In `field.rs` that is the whole of the `NEIGHBOURS` table: the ghost row
is nobody's neighbour, so nothing there groups, pops, or is dragged out beside a group, and
the rule is stated once. In `eval.rs` it is the `ghost` weight, which counts the cells of that
row walled off from the spawn column — a puyo resting up there is a *door closed*, because a
pair moves sideways with one half in it, and everything past it is unreachable however empty
the column below looks. That is also what bounds the quiescence search and the move
generator: `quiet::reachable_columns`.

**Read colours through the mask, not around it.** Every board feature above is about colour,
and a `CellId` here is colour *and* link mask, so a feature that compares raw `CellId`s sees
sixteen different reds and finds no chains at all. `Field::from_board` is where that is
handled: it drops the mask on the way in and keeps one byte a cell, because connectivity for
popping is worked out from the colours themselves and a mask is only ever drawing information.

**What made it fast enough to run in a frame.** The first version took 190 ms a pair, which is
twelve dropped frames. Four things fixed it, in the order they were worth:

* **a neighbour table.** `[[u8; 5]; 78]`, worked out at compile time, in place of the divide
  and modulo the chain loop was doing three hundred times a scan. On its own: 10.9 µs an
  evaluation down to 4.9 µs.
* **cutting the root layer.** A beam cuts every layer to its width, and this one was expanding
  all twenty two of the pair's placements before its first cut — one layer costing more than
  every layer after it put together.
* **not walking the visible queue once per continuation.** The invented continuations only
  differ *past* the pairs the player can see, so forking before them searches the same three
  real pairs over and over and calls it a wider search.
* **popping before settling.** A `Field` is always settled by construction, so the settle the
  game's own chain loop opens with is a scan of the whole board to move nothing — a dozen
  times over per placement, because the quiescence search resolves a dozen probes.

Together: 4.1 µs an evaluation, and the strongest row went *up* as well as getting cheaper,
because the budget freed up bought it depth. Spreading the search over frames — above — took
the same work from 10.6 ms in one frame to 0.88 ms in each of twelve.

## Phase 5 — vs. integration and attack pricing

**Status:** `todo` — no longer blocked: phase 4 step 1 is done, so `VersusAi` has a Puyo brain
worth dealing a board to

**Goal.** Puyo takes its turn in every vs. playlist, and garbage crosses at sane volumes in
all six directions.

Playlists deal three games, `VersusAi` fields three brains, and — the real work — the **six
directed attack prices** are set.

**Puyo joins the playlists by being added to `GameKind::PLAYLIST_ORDER`** (`launcher/src/games.rs`),
which is one line. Phase 2 introduced that list precisely so this could be a deliberate step:
a game is billed on the pre-menu as soon as it is playable and deals a playlist turn only once
it has the themes and the ai to take one. Everything that sequences a playlist -
`PlaylistThemes::slots`, `stage_count`, `first_game`, `fixed_stages`, `random_game` and the
tests - already reads that list, so nothing else in the launcher changes. `Difficulty::level`
already has a Puyo arm; it returns the dial unchanged as a placeholder and this is the phase
that measures what it should be. Measure rather than guess, the way the README's existing
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

## Phase 6 — the characters

**Status:** `todo` — raised by Alex on 2026-08-28, after `genesis` got its own audio. **Nothing
is built.** What is done is the *reading*, which is the part that cannot be recovered from the
code and is why this section is as long as it is. Start at *Resume here*, immediately below.

### Resume here — 2026-08-29

A session ended mid-task; this is where it got to and what to do next. **Read the handover notes
at the foot of this phase before starting.**

**State of play.** All thirteen characters are read off the sheet. Ten are read off the emulated
game and written up under their own headings below. Three are not: **Davy Sprocket, Scratch and
Dr. Robotnik**, whose captures exist but were never processed, because the shell in that session
stopped working. **Nothing has been built.** One file was added and nothing imports it:
[`puyo-rusto/art/mugshots.py`](../puyo-rusto/art/mugshots.py), the analysis script — assembled
from working code but **never run in one piece**, so its first run is a shakedown.

**Also gone with that session:** the extracted frames, the `*_cal.json` files and the strip pngs,
which all lived in a scratch directory. None of that matters — the calibrations are in the table
in the method section and re-extracting a clip is one `ffmpeg` call, which `extract()` does for
you. Nothing needs recovering.

**Do these in order.**

1. **Shake down the script.** `python3 puyo-rusto/art/mugshots.py prep grounder` should print
   his frame diffs and reproduce what his section says — 128 px at rows 13-29 on the first idle
   step, a `halving` of 1.000 on the defeat pair, no teal and no enclosed navy. If it does, the
   sheet half works. Then `Clip('grounder', 'idle', CALIBRATION['grounder']['idle'])` and
   `against_ncc(c, cut('grounder')['idle'], (19, 8, 46, 32))` should give a blink every ~2.33 s.
   That exercises the capture half against a known answer. Fix whatever typos surface.
2. **Read Davy Sprocket, then Scratch, then Dr. Robotnik**, following *How to read a capture*
   below. Their own section has the sheet prep and what each row draws; the **regions to point
   the classifier at** are the last three rows of the region table in step 2 of the method,
   worked out from their sheet diffs but never tried against a capture. So this is timing and
   confirmation rather than discovery. Davy first: he is the simplest and he is what the
   plumbing gets built against. None of the three has a calibration yet — fit one, and
   **checkerboard it before trusting it**. Write each one up under its own heading beside the
   other ten, and update the two summary tables (*the timers* and *what each row is*), the
   calibration table and the region table.
3. **Answer the two questions their sections raise** — Davy's silver/gold cap studs (an
   unlabelled palette cycle?) and Dr. Robotnik's 48 px of enclosed key (real gaps, or detail a
   colour key would punch through?).
4. **Take one more game over capture and leave it running for ten seconds.** The single number
   missing from this whole phase is how long a defeat pose holds before the halved frame — five
   captures, the longest 2.32 s, none of them long enough. Any character will do.
5. **Then build**, starting from *What the engine needs, and what it already has*.

**Two decisions of Alex's that are already made**, so do not re-open them: the **danger flash is
not being implemented**, and the **sweat is one shared effect on a dial**, not per-character art.
Both are written up in *Two things that are not the character*.

**One question still standing that is Alex's, not a capture's:** is Dr. Robotnik in the deal at
all, or is he the boss of the set and out of it?

---

**It is numbered after phase 5 but does not depend on it**, which is the one place the working
agreement's "in order" needs a word: this is theme art and per-player animation, and it touches
nothing the vs. playlists or the attack prices touch. If Alex wants the faces before the
crossings, take it out of order deliberately rather than by drift.

**Goal.** Every retro theme draws a **character** beside each player's board, in the box its
own game drew one in, and the face it wears answers what is happening on that board. `genesis`
first, because Mean Bean Machine's sheet is the one that has been read.

### The source, and the grid it is on

`puyo-rusto/art/retro/Sega Genesis - Dr. Robotnik's Mean Bean Machine - Miscellaneous -
Mugshots.png`, 1095x971, keyed on teal `(0, 108, 108)` — the same rip directory as everything
else in phase 3 and, like the rest of it, **not in the repository**.

Thirteen characters, and the ripper wrote the reading down on the sheet itself:

> Note: All animations go in the same order (from top to bottom): -Idle -Winning -Losing -Defeat

Which is Alex's reading exactly, and is now sourced rather than assumed.

**Every frame is 80x56 on an 81x57 pitch**, without exception — one pixel of key between
frames, four rows of frames per character, each character block starting wherever its name
label ends. So the whole sheet is `origin + (81 * col, 57 * row)` and a character is an origin
and four frame counts. This was measured by connected components over the keyed background,
not read off by eye.

**80x56 is exactly `MUGSHOT` in `theme/genesis/mod.rs`.** The box the panel already names —
`(120, 96, 80, 56)`, a hole in the frame plane, so its rect is the game's own to the pixel —
takes a portrait with nothing to scale and nothing to centre. That is not a coincidence; it is
the box these were drawn for.

**Key the background out and draw the portrait over the wall.** ~~Draw the frame opaque,
including its background.~~ **Corrected 2026-08-28 against the Arms captures**, which settle it:
brighten the box interior in any of them and the dungeon stone is plainly there behind the
character, the same speckled dark course the ripper already cut into `background.png`'s hole.
The uniform `(0, 0, 96)` behind every face on the sheet is the *ripper's* key, not the Genesis's
backdrop. So the box stays a hole and the character is composited into it.

Two things follow. The key colour appears **enclosed** inside some characters — 253 pixels of it
in a Frankly frame, 108 in a Humpty, 60 in a Robotnik — and that is right rather than wrong:
they are the gaps his bent springs leave, wall showing through. So key by colour and do not try
to be clever about interior holes. But a character with a dark navy *detail* the same colour
would be punched through by the same rule, so the cut wants eyeballing per character, which is
what `rip_retro.py check` already exists to do.

| character | origin | idle | winning | losing | defeat | extras |
|---|---|--:|--:|--:|--:|---|
| Arms | (1, 14) | 4 | 3 | 2 | 2 | palette cycle |
| Frankly | (327, 14) | 2 | **1** | 2 | 2 | 4 bolts |
| Humpty | (493, 14) | 2 | 4 | 3 | 2 | 2 + 3 + 3 |
| Coconuts | (846, 14) | 2 | 2 | 3 | 2 | palette cycle ×2 |
| Davy Sprocket | (1, 257) | 2 | 2 | 2 | 2 | — |
| Skweel | (202, 257) | 3 | 3 | **6** | 2 | — |
| Dynamight | (690, 257) | 2 | **5** | 2 | 2 | — |
| Grounder | (1, 500) | 3 | 2 | 2 | 2 | — |
| Spike | (246, 500) | 3 | 2 | 2 | 2 | — |
| Sir Ffuzzy-Logik | (491, 500) | 3 | 3 | 3 | **4** | palette cycle, 3 eyes |
| Dragon Breath | (835, 500) | 3 | 2 | 3 | 2 | — |
| Scratch | (1, 743) | 2 | 2 | 2 | **1** | — |
| Dr. Robotnik | (169, 743) | 3 | 2 | 2 | **1** | — |

127 frames in all. `Border (Ending Only)` at (414, 743) is an empty box outline for the ending
cutscene and is **not** a character.

**A frame count is not a pose count**, which Arms showed and which the table above cannot say.
Two frames of a row can differ *only* in colour, because the ripper grabbed them at different
points in a palette cycle — Arms' idle frames 1 and 2 are the same face and differ solely in the
rim rows the lights are on. And the **last frame of a defeat row is never a pose at all**: it is
some earlier frame at exactly half brightness, pixel for pixel.

**Which frame it halves was checked across the whole cast** (2026-08-29), and there is one
exception worth the checking:

| character | last defeat frame is the halving of | match |
|---|---|--:|
| Frankly, Arms, Humpty, Skweel, Grounder, Spike, Dragon Breath | its own `defeat[0]` | 1.000 |
| Davy Sprocket, Coconuts | its own `defeat[0]` | 0.992, 0.988 |
| Sir Ffuzzy-Logik | its own `defeat[2]` — three poses, then the halving | 1.000 |
| **Dynamight** | **`losing[1]`** — because his `defeat[0]` is the explosion | 1.000 |
| Scratch, Dr. Robotnik | — one defeat frame each, so nothing to halve | |

Coconuts' 0.988 and Davy's 0.992 are not exceptions: the pixels that miss are Coconuts' cycling
coin, caught at two different points of its ramp. So the rule is **the last defeat frame is the
halving of the pose the character comes to rest in**, which for everyone but Dynamight is the
frame before it.

So before treating a row as N poses, **diff its frames**: what differs only in one band of rows
is a palette state, and what is an exact halving is a fade.

**And the defeat row does not play through.** Alex, 2026-08-29, from watching several: *the game
over animation is the first frame for a short period, then the final frame displayed statically
after that.* **Five game over captures now say what the first part of that means**, and one of
them sharpens it:

| capture | what the box does from the moment of death | for |
|---|---|--:|
| Dynamight | holds `defeat[0]`, the explosion, dead still | 0.92 s |
| Spike | holds `defeat[0]` dead still, residual flat at 31.2 | 1.73 s |
| Dragon Breath | holds `defeat[0]` dead still, box mean flat at 78.2 | 1.75 s |
| Grounder | holds `defeat[0]` dead still, box mean flat at 76.1 | 1.84 s |
| **Sir Ffuzzy-Logik** | **runs `[0] → [1] → [2] → [1]`, a ping-pong at 36 frames round** | 2.32 s |

So the rule is not "the first frame": it is **the defeat row's poses, animated the way that
character animates**, which for twelve of them is a single pose held and for Sir Ffuzzy-Logik is
his own three-frame fur dither still ticking over at a rate of its own. **Alex confirmed that
reading on 2026-08-29** having watched the set — *the game over animation is different per
character, with most, maybe all but Ffuzzy, being static* — so a static defeat is the default and
Ffuzzy is the exception rather than the first of a pattern. Then, on Alex's word, the halved
frame is held. **No capture has reached the halving yet** — the longest is Sir Ffuzzy-Logik's
2.32 s, over which `defeat[3]`'s residual sits flat at 52 while the other three swing between 19
and 26 — so how long the poses run before the fade is **at least 2.4 s** and otherwise
unmeasured. It is the one number still missing from this phase, and the next game over capture
should simply be left running.

**What that means for the implementation, and it is the cheap answer:** `defeat` is a
`FrameAnimationType::Static` on the row's first frame for every character but Sir Ffuzzy-Logik,
who wants `YoYo` over his first three at 36 frames round. Since the fade has never been seen to
arrive, **the halved frame need not be drawn at all to start with** — cut it or ignore it, and
add it later if it ever turns out to matter. That removes the only piece of the defeat row
nobody has measured from the critical path.

*(Separating the last two frames of a defeat row wants an **absolute** comparison, not a
normalised one: a halving correlates perfectly with what it halves, so normalised correlation
reports the two as the same frame. This is what made Sir Ffuzzy-Logik's row look at first as
though it were reaching the fade and coming back.)*

**Scratch and Dr. Robotnik's one frame each is not a gap in the rip** — first and last are the
same frame, so their defeat is simply that pose held, with no fade, which answers the question
the per-character notes raise for them.

### Whose face it is

In Mean Bean Machine the box holds the **opponent** — the stage's boss, reacting to you. Here a
panel belongs to one player and there may be one of them, so the character is that **player's
own**, and the states are read from that player's board. Alex's call, and it is the same move
the `NEXT` boxes already made: the game drew one box for you and one for the opponent, and this
panel runs one player's queue through both.

**Which character is dealt at random**, per player, per match, and it goes through the seed the
way `PuyoSkin::deal` does and for the same reason — a playlist that swaps a player's board onto
Puyo mid-match must hand them back the character they had, and the two players of a two player
game must not be dealt the same face. Thirteen characters and at most two players, so unlike
the skins there is no cycling to worry about.

### The four states, and what moves between them

The sheet's four rows map onto the match like this. **State is per player**, read from that
player's own board and nobody else's.

| state | row | entered when | left when |
|---|---|---|---|
| `idle` | 1 | the resting state | something below happens |
| `winning` | 2 | a chain, and a won match | its hold runs out |
| `losing` | 3 | nuisance is waiting, or the stack is high | neither is true any more |
| `defeat` | 4 | game over | never — the match is finished |

**`winning` runs the whole chain and lingers.** The trigger is `GameEvent::Clear`; a chain is a
run of them, one per step, with a `Settle` between, so the rule is *enter on a `Clear`, and
hold until `LINGER` after the last one*. Every further step of the same chain refreshes the
hold, so a nine chain holds the face for nine steps and a bit rather than restarting it. The
open question is whether a **single** pop counts — see below.

**`losing` has two triggers and no event for either.** Nuisance waiting is
`Game::pending_attacks()`, which is non-empty exactly when an opponent has sent something that
has not landed — the tray this theme already draws. Board height is
`MatchScreen::stack_danger`, which is already computed every frame for the particle field:
0 when the board is empty, 1 when the stack is at the top. Note what it measures — **the
highest column as a fraction of the visible height**, not the fraction of cells filled — so
"60% full" reads as *the stack is 60% of the way up*, which is the number that matters and is
the one already on hand. Both are read per frame; neither is an event.

**Do not flip-flop.** Three rules, all of them needed:

* **A minimum dwell.** Once entered, a state holds for `MIN_DWELL` whatever else happens short
  of game over, so a state cannot be entered and left inside one animation cycle.
* **Hysteresis on the height trigger.** Enter `losing` above `DANGER_ENTER`, leave it only
  below `DANGER_LEAVE`, with a real gap between them (0.60 and 0.45 to start). A stack sitting
  exactly on one threshold otherwise strobes as each pair locks.
* **A linger, not an edge.** `winning` outlives its last `Clear` by `LINGER`, and `losing`
  outlives the last nuisance landing by the same, so a chain that clears the tray does not
  snap the face back mid-pop.

**Precedence, while a match is running: `winning` beats `losing`.** A player who chains while
buried is answering the nuisance — cancelling it outright, which is what `nuisance.rs` does —
and the face that says so is the right one. `losing` resumes when the hold runs out, if
whatever put it there is still true.

**`defeat` and victory are terminal.** `GameEvent::GameOver` enters `defeat` and nothing leaves
it; `GameEvent::Victory` enters `winning` and holds it for the rest of the match rather than
lingering. Alex's call, and it means `winning` needs a *held* mode as well as a lingering one.

### Two things that are not the character — the sweat and the danger flash

Both were raised by Alex on 2026-08-29 after watching several captures, and both were then
measured. Neither belongs to any character, and they pull in opposite directions: one is worth
generalising and building, the other is worth leaving out.

#### The sweat is a shared effect on a dial, not anybody's art

**Blue drops fly off a character's head when their board is going badly**, and they are the
same drops on everybody. Three separate pieces of evidence say so:

* **Six characters sweat identically** — Frankly, Dynamight, Grounder, Spike, Sir Ffuzzy-Logik
  and Dragon Breath — and *none of their sheet blocks carries a drop*. Frankly's carries four
  gold bolts and Sir Ffuzzy-Logik's three eyes; the other four carry no extras at all. There is no drop sprite anywhere in the Mugshots rip,
  in `Opponents.png` or in `Has Bean.png`, and Alex could not find one either (2026-08-28). So
  it cannot be per-character art.
* **The same character sweats in one clip and not the other.** Counting round blue blobs in the
  band of wall *above* the box — which no character's art reaches, so nothing of his own can be
  counted — over the same framing:

  | clip | drops per frame | frames with any |
  |---|--:|--:|
  | Grounder, losing (his stack still low) | **0.00** | **0%** |
  | Spike, losing | 0.08 | 7% |
  | Sir Ffuzzy-Logik, losing | 0.20 | 12% |
  | Spike, after the death | 0.27 | |
  | Dragon Breath, game over | 0.32 | 23% |
  | Grounder, game over (the same clip framing, deeper in) | 0.45 | 35% |
  | Dragon Breath, losing | 0.47 | 41% |
  | Sir Ffuzzy-Logik, game over | 0.64 | 42% |
  | Dynamight, game over | 1.04 | 61% |
  | Dynamight, losing | 1.05 | 74% |
  | Frankly, losing | 1.13 | 52% |

  Grounder is in the `losing` row for the whole of his losing clip — worried face, mouth
  working — and sweats **nothing**. So the drops are not the `losing` state; they are a
  separate, graded thing that comes on later than it does.
* **It is graded, and there are three thresholds, in this order:** the losing *face*, then the
  sweat, then the flash below. Dynamight's losing clip has drops from the first frame and the
  flash only from t≈1.4 s; Grounder's has neither, and his game over clip has both.

**What a drop is**, measured off Dynamight's and Frankly's losing clips: a round blue blob with
a lighter blue core, about **3-4 screen pixels across** (≈4 box px). It leaves from the **upper
corners of the head** — first seen around box `(14-17, 13-20)` on the left and box `(70, 20)` on
the right, which is just clear of Dynamight's own silhouette (his art spans cols 22-59) — and
travels **up and outward on the 45° diagonals** at about **1.0-1.5 box px an axis a frame**.
That is the same speed as Frankly's winning sparks (1.5-1.7), which is a point in favour of one
emitter serving both. **It is not clipped to the box**: the drops cross the stone of the centre
column and go on over the boards, and they are last seen fading rather than stopping, on Alex's
word of 2026-08-28. One to three are in the air above the box at a time at the levels captured.

**So: one drop sprite, one emitter, driven by a number the engine already has.** The dial is
`MatchScreen::stack_danger` — 0 when the board is empty, 1 when the stack is at the top —
which the state table above already reads for the `losing` trigger, and which is already
computed every frame for the particle field. The sweat wants **a rate that scales with it**
above some threshold higher than `DANGER_ENTER`, rather than an on/off. The drop still has to
be cut out of a video, since no rip carries it — the routine sketched in the Frankly section is
the way, taking a colour and a clip and taking the median of every drop registered on its
centre — but it is cut **once for the whole cast**, not per character, and that is the whole of
the change.

**This supersedes the Frankly write-up's reading**, which had the sweat as his own oddity and
"shared art the game keeps somewhere these rips do not go". The second half of that was right;
the first half was not.

#### The danger flash is the screen, and Alex's call is to leave it out

**Every character goes white in bursts when their board is nearly full**, and **three separate
losing clips taken while the stack was still low show no flash at all** — Grounder, Sir
Ffuzzy-Logik and Dragon Breath, all within 3% of their own quiet level over more than a hundred
frames each — so it is a danger threshold and not part of the losing animation. It is not a
character animation either: the whole **sprite plane** brightens — the character *and* the puyos on both boards
go pale together — while the wall, the `STAGE` text and the `SCORE` text do not move at all,
which is a palette flash on the palettes the sprites share rather than anything the mugshot is
doing. Measured as the box's mean brightness against its own quiet level:

| clip | quiet | flashed | when |
|---|--:|--:|---|
| Grounder, losing (stack still low) | 63.1 | 63.8 (+1%) | never — 0 of 103 frames |
| Sir Ffuzzy-Logik, losing | 95.8 | 98.6 (+3%) | never — 0 of 133 frames |
| Dragon Breath, losing | 70.5 | 71.7 (+2%) | never — 0 of 148 frames |
| Grounder, game over | 76.1 | 89.6 (+18%) | 19 of 75 frames, in two bursts before death |
| Sir Ffuzzy-Logik, game over | 97.8 | 131.6 (+35%) | 62 of 163 frames |
| Dragon Breath, game over | 78.1 | 107.8 (+38%) | 25 of 73 frames |
| Spike, losing into game over | 98.6 | 138.4 (+40%) | 46 of 143 frames |
| Dynamight, losing | 47.1 | 73.2 (+55%) | 20 of 82 frames, from t≈1.4 s on |

**Alex's decision, 2026-08-29: do not implement it.** It is recorded here so the next agent
does not spend a capture trying to explain a character who suddenly turns white, and so nobody
adds it back thinking it was missed.

### What the engine needs, and what it already has

The engine already has a character beside the board — `animate/mascot.rs`, which is Dr. Mario —
and it is **close but not this**. `MascotMeta` has four strips too, but they are `idle`,
`spawn`, `victory` and `game_over`, and which one plays is decided entirely by the animation
phase in `Theme::draw_board`: a piece is spawning, the match is over, the match was won,
otherwise idle. There is no strip for *how the board is going* and no seam that reads the
board at all. So this is a new thing beside the mascot rather than a fifth `MascotKind`:

* **`engine/src/animate/character.rs`** — a `CharacterAnimation` holding the current state, its
  `FrameAnimation`, the dwell and linger clocks and the hysteresis latch. It goes in
  `PlayerAnimations`, which is per player, is already `update(delta)`d every frame and already
  takes `on_event(&GameEvent)` — which is where `Clear`, `GameOver` and `Victory` arrive. The
  two per-frame numbers (`danger`, and whether anything is pending) have no event, so they need
  one new call alongside `update`, fed from the match screen the same way `SceneContext` is fed
  `Self::stack_danger(game)` today.
* **`CharacterLayout` and character sprites on `RetroThemeOptions`**, both `Option`, beside
  `mascot`/`mascot_animations`. A layout is a `Rect` and nothing else here, since the art is
  the size of the box.
* **One `AnimationSpriteSheetData` per state**, which
  `AnimationSpriteSheetData::non_exclusive_linear(file, start, frames, 80, 56)` expresses
  exactly — one PNG per character, four rows, start at `(0, 57 * row)`. Thirteen PNGs of at
  most 480x228 each; nowhere near `MAX_ATLAS_WIDTH`, and about 2 MB of texture for the lot if
  all thirteen are built, which they must be, since which one is dealt is not known until a
  match starts.
* **The deal has to reach the theme.** `Theme::draw_board(&self, canvas, game, animations)` is
  `&self` and a theme is `&'static`, so the character index cannot live on the theme — it
  belongs on `PlayerAnimations`, dealt when the match screen builds one. This is the same
  constraint that put `PuyoSkin` on the `CellId` rather than on the theme, arrived at from the
  other end.

### The one conflict on `genesis`: the tray is in that box

`PendingLayout` currently fills the mugshot box with the nuisance tray — five icons at a whole
cell each, right to left — because it is the only hole on the panel big enough and Mean Bean
Machine, which lands an attack the moment it is sent, drew nothing that needs it. A character
wants the whole box. **These cannot both have it**, and the tray is a rule this game has and
the original did not, so it cannot simply be dropped.

**Resolved by phase 7 on 2026-08-29 — the mugshot box is free.** The premise above is wrong:
Mean Bean Machine *does* draw a nuisance tray, on the wall immediately above the board, and
the paragraph above is left standing only so this is not re-argued from the same mistaken
start. The tray has gone to where the game draws it and **the whole box is the character's**.
See [phase 7](#phase-7--how-it-moves) for the measurement.

What was on the table when this looked like a conflict, none of which is needed now:

* the two `NEXT` boxes are 32x48 and hold one pair each — full;
* the strip under `SCORE` at the foot of the panel is stone, unboxed, and about 80x16 clear;
* the tray could be drawn **over** the character, along the bottom of the box, which is 80x16
  of a face's chin;
* or the character could be dropped on `genesis` and the theme keep its tray.

### Per-character notes, and the questions to ask

The rip carries loose sprites beside some rows that are **not** part of any 80x56 frame. They
are drawn to the right of the row they belong to, at no stated offset, so *where they go over
the portrait is not on the sheet* — this is the part that has to come off the video, one
character at a time. Their rects, and what they appear to be:

* **Arms** — 4 idle frames, the most of any character. `Palette cycle: replace (224,224,0) with`
  a ramp of eight, `(96,64,0)` → `(128,64,0)` → `(160,96,32)` → `(192,128,64)` → `(224,160,96)`
  → `(224,192,128)` → `(224,224,160)` → `(224,224,224)`. That is the bright yellow of the lights
  round his saucer's rim pulsing dark-to-white. ~~*Which row does it apply to?*~~ **Read off the
  game on 2026-08-28: all of them, at two different speeds — see *Arms — reconstructed* below.**
* **Frankly** — **one** winning frame and four 8x16 lightning bolts at `(408, 71)`, `(417, 71)`,
  `(426, 71)`, `(435, 71)`. So his win is a single pose with the bolts flashing over it, which
  is the clearest evidence on the sheet of how the loose sprites are meant to work: they are
  extra frames of an overlay, not extra frames of the face. **Read off the game on 2026-08-28 —
  see *Frankly — reconstructed* below**, which confirms it.
* **Humpty** — three sets. Two 24x8 yellow squiggles at `(655, 14)` and `(655, 23)` on the idle
  row; three 8x16 bolts at `(817, 71)`, `(826, 71)`, `(835, 71)` on the winning row, where he is
  already pointing a finger; three 24x16 green bent limbs at `(736, 128)`, `(736, 145)`,
  `(736, 162)` on the losing row, where his hands are already wringing. *The idle pair is the
  unclear one* — antennae, or a sparkle on the antenna balls?
* **Coconuts** — the only character with **two** palette cycles, and the only one where the rip
  says which row each is for. Replace the pair `(224,128,0)` / `(224,192,96)`;
  **winning** with eight pairs running `(64,32,0)` up to `(224,224,160)`/`(224,224,224)`,
  **losing** with six running `(224,192,128)`/`(224,224,224)` down through two reds to
  `(32,0,0)`. Bright flash to win, red flush to lose. His fur, by the look of the swatch.
* **Davy Sprocket** — two frames in every row, no extras. The simplest character on the sheet
  and therefore the one to build the plumbing against.
* **Skweel** — **six** losing frames, three times any other row he has. ~~Worth watching for
  whether it is a one-shot that settles on a pose rather than a loop.~~ **Read off the game on
  2026-08-29: a loop, and a straight one.** See his section.
* **Dynamight** — **five** winning frames, and his defeat frame is an explosion. No extras.
  **Read off the game on 2026-08-29**: the five frames are one held portrait with the plunger
  handle waving over it, so the row is Frankly's shape drawn into the frames. See his section.
* **Grounder**, **Spike**, **Dragon Breath** — three idle frames each, otherwise unremarkable
  and extra-free. All three **read off the game on 2026-08-29**; unremarkable was right, and
  their value was in the timings and in the two game over captures. See their sections.
* **Sir Ffuzzy-Logik** — three 32x24 sprites at `(734, 614)`, `(767, 614)`, `(800, 614)` on the
  losing row, which are **his eyes**: yellow open, yellow narrowed, and shut. Plus a palette
  cycle replacing the pair `(192,192,0)` / `(160,128,0)` — the same eye yellow — with five
  pairs from `(96,96,0)`/`(64,32,0)` up to `(224,224,64)`/`(192,160,32)`. So his eyes are
  animated twice over, by sprite and by palette. He is also the only character with **four**
  defeat frames. **Read off the game on 2026-08-29 — see his section**, which places the eye
  overlay at box `(24, 8)`, times all three of his clocks per row, and settles what a defeat
  row does.
* **Scratch** and **Dr. Robotnik** — one defeat frame each, so their defeat is a still and they
  never fade, which the game over rule above now explains rather than leaving as an oddity: the
  first frame and the last frame are the same frame. Robotnik is the player character of the
  original and the odd one out of thirteen: *should he be in the deal at all, or is he the boss
  of the set and out of it?*

Questions that are not per-character:

1. ~~**Does a single pop count as `winning`?**~~ **Settled with Alex, 2026-08-28: it does
   not.** `GameEvent::Clear` fires once per chain *step*, so a one-step clear — which is most
   clears — would enter `winning` several times a minute, and it sends no nuisance either. It
   takes **two chain steps or more**; a one-chain leaves the face alone.
2. **Does incoming nuisance mean `pending`, or `landing`?** The tray means an attack is coming;
   the hit is when it drops. Reacting to the tray is the earlier and more useful signal and is
   what the state table above says, but the original reacted to the hit.
3. **Does the character animate over the pause, the stage clear card, and the game over
   curtain?** The mascot does not.
4. ~~**What frame rate?**~~ **Answered by Arms, 2026-08-28.** A face frame runs **6 to 11
   frames at 60 Hz** (0.10-0.18 s), and `FrameAnimationType`'s existing vocabulary covers every
   row seen so far: Arms' idle is a **ping-pong** (a blink on a timer), his winning row a
   ping-pong (arms pumping), his losing row a plain two-frame alternation, and Frankly's rows are
   `Static`. Skweel adds the two the list was missing — a **straight loop** (`Linear`, his
   six-frame sneeze) and a continuous **yo-yo** with no rest (`YoYo`, his sway). ~~What the
   vocabulary does *not* cover is a held pose with a short action on a long timer, which is what
   an idle turns out to be.~~ **It does**, and it is already implemented: the refugee bean's
   `LinearWithPause` in `theme/genesis/mod.rs`, over a strip cut with the *action first and the
   rest last*, so the frame the pause holds is the resting one. Skweel's idle is that exactly —
   sheet order open, half, shut, cut as `1, 2, 1, 0`, `resume_from_frame: 0`.
5. **~~Palette cycling is not a thing the renderer does.~~** Three characters need it. Baking
   each cycle step as an extra frame in the ripper is cheap — a cycle is a colour swap over one
   pose and the ripper already does palette swaps for the `genesis` fonts — but **it cannot be
   baked into the face frames**, which Arms showed and Coconuts settles: the cycle and the pose
   run on clocks that do not divide (Coconuts' winning row is a 1.36 s wink over a 0.60 s cycle,
   his losing row a 0.33 s shake over a 0.40 s one), so a frame carrying both would need the
   product of the two. So the cycled element is **cut as its own small overlay sprite, one
   variant per ramp step, drawn over the portrait on its own clock** — which is the cheapest of
   the three answers, keeps the renderer free of a palette feature exactly three sprites want,
   and is the same mechanism the overlay kind already needs for Humpty. Coconuts is the easy
   case (one coin, box rows 0-11, cols 44-60, fourteen variants over two ramps); Arms' ring of
   lights and Sir Ffuzzy-Logik's eyes are the two to size it against.

### Reading the animations off the game, one character at a time

The loose sprites, the palette cycles and the timings are not on the sheet, and neither is what
any of it *does*. So Alex records the emulated game a character at a time and it is read off the
clips here. This is the backlog; work down it, and write each one up under its own heading as it
is done. **How to read a clip is written out below**, and so is **what the sheet already says about
the last three** — do the prep before opening a video, it is what tells you where to look.

**All thirteen have captures and all thirteen are read off the sheet. Ten are read off the game
as well**; the last three are the sheet only, because the shell died mid-session on 2026-08-29
and their clips were never processed. Their section carries what the frames say and what the
other ten predict, clearly marked; **finishing them is an hour and it is the first job here.**

| # | character | status |
|--:|---|---|
| 1 | **Frankly** | **done** — 2026-08-28, two captures. See below. |
| 2 | **Arms** | **done** — 2026-08-28, three captures. See below. |
| 3 | **Humpty** | **done** — 2026-08-28, three captures. See below. |
| 4 | **Coconuts** | **done** — 2026-08-29, three captures. See below. |
| 5 | **Skweel** | **done** — 2026-08-29, three captures. See below. |
| 6 | **Dynamight** | **done** — 2026-08-29, four captures including a game over. See below. |
| 7 | **Sir Ffuzzy-Logik** | **done** — 2026-08-29, four captures including a game over. Taken out of turn at Alex's word; he was the one to leave until last and repaid it. See below. |
| 8 | **Grounder** | **done** — 2026-08-29, four captures including a game over. See below. |
| 9 | **Spike** | **done** — 2026-08-29, three captures, one of them losing straight into the game over. See below. |
| 10 | **Dragon Breath** | **done** — 2026-08-29, four captures including a game over. See below. |
| 11 | Davy Sprocket | **sheet only** — captures taken, never processed. **Do him first**: 2/2/2/2, no extras, and he is what the plumbing gets built against |
| 12 | Scratch | **sheet only** — captures taken, never processed. His one defeat frame is now explained rather than open |
| 13 | Dr. Robotnik | **sheet only** — captures taken, never processed. Two things need a human: his 48 px of enclosed key, and whether he is in the deal at all |

**~~The gap Frankly left.~~** Closed by Arms: a face frame runs 6-11 frames at 60 Hz, an idle is
a held pose with a blink on a ~1.2 s timer, and a row's frame count is not its pose count.

**~~The gap Arms left.~~** Closed by Humpty: an idle does have loose sprites, and they *flicker
in place* rather than being emitted — which split the loose sprites into two kinds. See his
section for what that does to the design.

**~~The gap the sheet's labels left.~~** Closed by Coconuts: a *labelled* cycle uses every step
of its ramp, in order, on the rows the label names — but the label says nothing about shape or
rate, and his two ramps differ on both. A cycle's clock never divides its row's pose clock, so
the cycled element is a layer and not a baked frame. See his section.

**~~The gap a long row left.~~** Closed by Skweel: a six-frame row is a **straight loop**, not
a one-shot — the first row on the sheet that is neither a ping-pong nor an alternation. And his
sneeze is drawn *into* the 80x56 poses, which makes a third home for an effect beside the
emitter and the overlay, and much the cheapest of the three. See his section.

**~~The gap the losing row left.~~** Closed by Dynamight and Grounder, and by Alex watching
them: the **sweat is not a character's** and the **flash is not a character at all**. Grounder
is the proof — the same framing, the same `losing` face throughout, no drops and no flash in his
losing clip and both in his game over clip. See *Two things that are not the character*, above
the character sections, which is where both now live.

**~~The gap the defeat row left.~~** Closed by Sir Ffuzzy-Logik: a defeat row's poses **keep
animating** rather than holding, at a rate of that row's own. His four frames are three dither
phases ping-ponging at 36 frames round, still going 2.32 s after the death, plus the halving,
which no capture has yet reached. See *the game over* in the sheet section.

**Ten of thirteen are read**, and the shapes have stopped being new. Every row seen is one of
five: a held pose (Frankly), a continuous alternation or ping-pong (Arms, Dynamight's idle,
Skweel, Sir Ffuzzy-Logik's fur), a straight loop (Skweel's sneeze, Dynamight's winning), a held
pose with **one** action on a timer (Humpty, Grounder's losing, Dragon Breath's winning), or a
held pose with the action **twice** (Coconuts' idle, Grounder's winning, all three of Spike's) —
with Dragon Breath's losing row a three-times variant of the last. The three still to read are
unlikely to add a sixth.

**The timers, all measured, so nothing shares a constant it should not:**

| character | idle | winning | losing |
|---|--:|--:|--:|
| Dynamight | 0.69 s (continuous) | 0.45 s (loop of 5) | 0.25 s (continuous) |
| Arms | 1.19 s | 0.56 s (continuous) | 0.36 s (continuous) |
| Skweel | 1.27 s | 0.63 s (continuous) | 0.66 s (loop of 6) |
| Humpty | 1.35 s | ~0.70 s ×2 then ~2 s | 2 poses alternating, hands flickering |
| Coconuts | 1.68 s (**double**) | 1.36 s | 0.33 s (continuous) |
| Dragon Breath | 1.93 s | 1.54 s | 1.60 s (flutter ×3) |
| Grounder | 2.33 s | 1.56 s (**double**) | 1.62 s |
| Spike | 2.40 s | 2.34 s (**double**) | 1.99 s (**double**) |
| Sir Ffuzzy-Logik | 0.54 s fur, ~1.2 s eyes | 0.67 s fur, 0.52 s eyes | 0.39 s fur, ~0.3 s eyes, ~1.2 s blink |
| Frankly | static | static + emitter | static + emitter |
| *Davy Sprocket, Scratch, Dr. Robotnik* | *unread* | *unread* | *unread* |

A blink period alone runs from 1.19 s to 2.40 s across the cast, two to one, so **take it off
every capture**.

**And what each row actually is**, since a timing without its gesture is no use:

| character | idle | winning | losing | defeat |
|---|---|---|---|---|
| Frankly | static | static, sparks off both antenna balls | static, sweating | pose + fade |
| Arms | blink, 3 poses | arms pump | 2-frame alternation | pose + fade; lights cycle on every row |
| Humpty | blink, 2 poses | antennae flex and fire bolts | ducks and wrings his hands | pose + fade; arc between antennae when idle |
| Coconuts | **double** blink | one eye winks | shakes a raised fist | pose + fade; coin cycles on win and lose |
| Skweel | blink, 3 poses | sways his whole body | **sneezes** — snout up, mouth open, puff from the nostrils | pose + fade |
| Dynamight | his grin works, continuously | face still; the plunger handle waves | hammers the plunger | **explosion**, then the halving of `losing[1]` |
| Grounder | blink, 3 poses | **double** eyebrow-raise and grin | gulps, tongue out | covers his face; pose + fade |
| Spike | blink, 3 poses | **double** laugh | hands to his cheeks, mouth clamping **twice** | pose + fade |
| Sir Ffuzzy-Logik | fur dither; no blink | fur dither; no blink | fur dither **and** the eyes blink | three dither poses, **still animating**, + the halving |
| Dragon Breath | blink, 3 poses | narrows his eyes, widens the grin | lip-flutter **×3** | pose + fade |
| Davy Sprocket | blink (eyes vanish) | rocket swings; cap studs silver↔gold | beak opens and shuts | pose + fade (0.992) |
| Scratch | small change at the beak | the red comb shifts | beak gapes, tongue showing | **one frame, no fade** |
| Dr. Robotnik | blink of 2-3 px — the smallest on the sheet | raises a gloved finger | clenches a gloved fist | **one frame, no fade** |

**Which frame is the rest is not consistent, and it decides how the strip is cut.**
`LinearWithPause` holds the *last* frame of its strip, so a row wants cutting **action first,
rest last**. Grounder's losing row is already that way round (`losing[0]` is the gulp,
`losing[1]` the rest) and Skweel's idle is not (his `idle[0]` is the open eye, so his strip is
cut `1, 2, 1, 0`). Both of Spike's animated rows hold `[0]`, so both need reordering. Diff the
row, then check the capture for which pose has the long dwell — do not assume the sheet's order
is the play order.

**~~Frame the next captures wider if you can.~~** Not needed — **Alex settled it on 2026-08-29**:
the travelling effects (Frankly's sparks, his sweat, Humpty's bolts) **leave the box and fade
away, like particles**. So the question every zoomed capture leaves open is already answered for
the whole cast, and a capture cropped to the box costs nothing but the confirmation.

That makes the **emitter** kind unambiguously a particle source and not a sprite strip: emitted,
free of the box, and dying by fading rather than by being clipped. Note what is measured and what
is not — Frankly's sparks were *last seen* about 40 box pixels out and Humpty's bolts about 10,
but no capture resolves a fade, at these scales, against dark stone. So **fade them out over the
last of their life** on Alex's word, and treat the distance as when they are gone rather than as a
hard edge. The **overlay** kind is untouched by this: an arc between two antennae and a pair of
wrung hands stay where the character is.

**Where the captures live.** `~/Videos/Screencasts/`, named for the character and the row —
`frankly-winning.mp4`, `frankly-losing.mp4`. Alex leaves them in place; they are not in the
repository and are not meant to be, on the same footing as every other rip source here. Keep
the naming for the ones that follow.

**What a capture can and cannot settle.** Both of Frankly's are screen recordings of an
upscaled window at a *variable* frame rate — 161 frames over 8.71 s and 132 over 6.07 s, with
the second panning mid-clip. Positions and directions come out clean, because they are
geometry; **timings do not**, because a capture frame's timestamp is when it was captured and
not when the Genesis drew it. So take speeds and periods below as measured to about ±20%.
**That is good enough** — Alex's call of 2026-08-28: nothing here has to match the Genesis
frame for frame, it has to read as the same effect. Do not go back to the emulator for a
number; take the measured one and tune it by eye.

#### How to read a capture — the method

Alex records short clips of the emulated game, one per row that does anything, and drops them in
`~/Videos/Screencasts/` as `<character>-<row>.mp4`. What follows is how the ten characters below
were read off them. It is written out because the context that produced them does not survive,
and because the same dozen steps answered every question for all ten.

**~~If the remaining characters are done in one sitting, all of this wants to be a script.~~**
**It is one: [`puyo-rusto/art/mugshots.py`](../puyo-rusto/art/mugshots.py)**, written 2026-08-29
out of the code that produced every number below. It carries the `CAST` table of origins and
frame counts, the `CALIBRATION` table of every box fit already measured, `prep` and `strips` as
a two-command CLI, and the rest as a library — `Clip`, `against`, `against_ncc`, `autofit`,
`ncc_fit`, `checkerboard`, `motion`, `palette_swapped`, `find_overlay`, `sweat_rate`. **It was
assembled after the shell died and has not been re-run since**, so expect to fix a typo; the
routines themselves are the proven ones. The steps below are still the method — the script only
saves retyping them.

**Nothing here needs to be exact.** Alex's call of 2026-08-29: these have to *read* as the same
effect, not match the Genesis frame for frame. Take the measurement and tune by eye.

**0. Prep from the sheet before opening the video.** `mugshots.py prep <character>` does it, and
the table further down carries it for the three still unread: how many frames each row has, and
**what actually differs between them**. Do this first, because it tells you where to look. A pair
that differs only in rows 17-27 is an eye change; a pair that differs over the whole 80x56 is a
different animal.

**1. Extract.** Frames and their timestamps, separately:

```
ffmpeg -v error -i <clip>.mp4 -vsync 0 out/f%03d.png
ffprobe -v error -select_streams v -show_entries frame=pts_time -of csv=p=0 <clip>.mp4 > pts.txt
```

`-vsync 0` matters: these are **variable frame rate** captures, roughly 20-30 fps against the
Genesis's 60, and the timestamps are when the frame was *captured*, not when it was drawn. Frames
repeat. Expect ±20% on any period and do not chase better.

**2. Calibrate to box coordinates.** Every number worth writing down is in the 80x56 box, so find
the mapping first. Three ways, best first:

* **The clip is the box.** Humpty's three are cropped to it, so `box = video / (width / 80)`.
  Check by comparing the character's outline against the sheet frame.
* **The `STAGE` and `SCORE` labels.** Their ink starts at screen y=85 and y=161 — measured off
  `background.png`, and `GENESIS_LABELS` in `rip_retro.py` has their nominal positions. The gap is
  76 screen rows, which gives the vertical scale from one subtraction. `STAGE 2` spans screen
  x 128-184 for the horizontal.
* **Fit the sheet frame itself**, scaled, against the video, over the frame's own non-key pixels.
  This is the best of the four when the character has little or no key in his frames — Coconuts
  has none at all — because it uses the whole 80x56 rather than two landmarks, and it gives both
  axes and the origin at once. Search the scale on each axis separately.
* **The box edges.** Find where the character's colours stop against the stone. Least reliable —
  the character rarely fills the box.

Sanity check the two axes against each other. **They are not always equal**: Coconuts' three
clips are 4.225 across against 4.53 down, a 7% difference, so a disagreement is a *non-square
capture* at least as often as it is a bad reading. Fit the two axes separately and only distrust
a reading when the two ways of measuring one axis disagree. And beware the `STAGE` label
specifically — the plain face is the same white as the speckles in the stone, and a column
projection over it can read the left edge fifty video pixels early. `SCORE`'s green is safe.

**The label method, written out**, since it is the cross-check for the fit and it was verified
against three known-good fits to within 1.3 px. Find the top row of `STAGE`'s white ink and the
top row and left column of `SCORE`'s green, then

```
sy   = (score_ink_y - stage_ink_y) / 76      # 76 screen rows between the two
box_y0 = stage_ink_y + 11 * sy               # STAGE's ink is 11 rows above the box top
box_x0 = score_ink_x -  8 * sx               # SCORE starts at screen x=128, the box at 120
```

Checked on Grounder it gives (98.7, 69.3) against a fit of (99, 68); on Dynamight (91.7, 66.3)
against (91, 65).

**The calibrations already measured**, so a re-run need not refit. Box origin in video pixels,
then video pixels per box pixel across and down:

| character | clip | video | box origin | sx | sy |
|---|---|---|---|--:|--:|
| Coconuts | all three | 585x430 | (135, 70) | 4.225 | 4.53 |
| Skweel | idle | 475x318 | (86, 61) | 3.79 | 3.77 |
| Skweel | winning, losing | 407x305 | (53, 56) | 3.79 | 3.81 |
| Dynamight | idle, winning | 494x350 | (99, 68) | 3.79 | 3.81 |
| Dynamight | losing, game over | 490x337 | (91, 65) | 3.79 | 3.81 |
| Grounder | all four | 494x350 | (99, 68) | 3.79 | 3.81 |
| Spike | all three | 490x337 | (97, 61) | 3.64 | 3.62 |
| Sir Ffuzzy-Logik | idle, winning | 490x337 | (92, 86) | 3.76 | 3.84 |
| Sir Ffuzzy-Logik | losing | 490x337 | (91, 86) | 3.79 | 3.84 |
| Sir Ffuzzy-Logik | game over | 490x337 | (91, 87) | 3.80 | 3.80 |
| Dragon Breath | all four | 490x337 | (77, 59) | 3.71 | 3.80 |

Two things to read off that. **A 490x337 capture is not always the same framing** — Dynamight,
Spike, Sir Ffuzzy-Logik and Dragon Breath share the size and differ by twenty pixels of origin
and 4% of scale — so the size is a hint and never a substitute for a fit. And **only Coconuts'
session was meaningfully anisotropic**; everything since has been square to within 1%.

**Fit it as a pyramid.** A full two-axis search at full resolution is minutes a clip. A coarse
*isotropic* sweep on a quarter-size copy of the frame, then a fine anisotropic refine at full
resolution within a few pixels of that answer, is seconds — and gave the same answer every time
it was checked against the slow version.

**And the fit routines return `(score, sx, sy, x0, y0)` while `Clip` takes `(x0, y0, sx, sy)`.**
`mugshots.py`'s `calibration()` converts; getting it wrong silently gives a plausible-looking
crop of the wrong thing.

**The regions each character was classified on**, in box coordinates as `(x0, y0, x1, y1)`.
These are the answer to *"choosing the region is the whole game"* and they are otherwise lost, so
they are written down: each one contains what that row's sheet diff says changes and excludes
everything else.

| character | idle | winning | losing | other |
|---|---|---|---|---|
| Coconuts | (16, 22, 38, 33) eyes | (27, 24, 38, 32) the winking eye | (0, 0, 32, 26) the fist | (42, 0, 64, 13) the coin |
| Skweel | (20, 8, 50, 32) eyes | whole frame — he sways | whole frame | |
| Dynamight | (20, 36, 56, 56) mouth | (40, 0, 80, 20) the handle | (18, 0, 60, 18) the plunger | (18, 18, 62, 56) proves the winning face static |
| Grounder | (19, 8, 46, 32) | (26, 6, 60, 48) | (20, 4, 55, 52) | |
| Spike | (26, 12, 56, 36) | (24, 14, 79, 56) | (30, 28, 64, 56) | |
| Sir Ffuzzy-Logik | (0, 34, 80, 56) fur | same | same | (24, 8, 56, 32) the eye overlay; the 37 px of `(192,192,0)` for the cycle |
| Dragon Breath | (25, 4, 54, 28) | (11, 19, 56, 48) | (22, 30, 52, 52) | |
| **Davy Sprocket** | **(20, 19, 47, 30)** | **(22, 5, 79, 44)** | **(20, 26, 52, 56)** | *derived from the sheet diff; unverified* |
| **Scratch** | **(32, 23, 58, 42)** | **(25, 2, 50, 17)** | **(11, 33, 52, 56)** | *derived from the sheet diff; unverified* |
| **Dr. Robotnik** | **(38, 19, 54, 29)** | whole frame | **(0, 28, 30, 56)** | *derived; his idle needs a threshold well under 4.0* |

The last three rows are worked out from their sheet diffs and have **not** been tried against a
capture — they are a starting point, not a measurement.

**3. Classify the face over time.** The trick that did all the work: keep a list of reference
crops, and for each frame either match it to one or add it as a new state.

```python
refs, labels = [], []
for f in frames:
    reg = np.array(Image.open(f).convert('RGB')).astype(float)[y0:y1, x0:x1]
    lab = next((i for i, r in enumerate(refs) if np.abs(reg - r).mean() < 4.0), None)
    if lab is None: refs.append(reg); lab = len(refs) - 1
    labels.append(lab)
```

A threshold of 4-5 absorbs compression noise and keeps real frames apart. Then print only the
*transitions* with their timestamps and the animation reads itself out.

**Choosing the region is the whole game**, and getting it wrong is the one mistake that cost time
here. It must contain what the sheet's diff says changes and **exclude the loose sprites**, or
every flicker becomes a state — Humpty came out as 11 states until the region was pulled down off
his antennae. Use the sheet diff's row range, converted to video coordinates.

**3b. An overlay's anchor may be on the sheet after all.** The loose sprites are drawn beside
their row at no stated offset, which is why this was filed as a job for the video — but when the
overlay's *rest* state is what the row's frames already draw, matching that sprite back into its
own frames finds the offset exactly. Sir Ffuzzy-Logik's open-eyes sprite lands at box `(24, 8)`
on all three of his losing frames with a residual of 4.4. Try this before opening the clip; it
works for an overlay that replaces part of the portrait, and not for an effect the portrait does
not contain.

**4. Find the loose sprites.** They are one or two flat Genesis colours, so a colour mask finds
them, and a **temporal** mask removes the character:

```python
M = np.array([mask(frame) for frame in frames])     # e.g. gold: r>150, g>110, b<110, r-b>90
static = M.mean(axis=0) > 0.6                       # anything present most of the time is him
loose = M & ~static
```

Then connected components per frame, with a minimum size, gives centroids and bounding boxes.
Sort them and print per frame with the timestamp.

**5. Composite every frame's loose sprites over one still.** This one picture answered more than
all the numbers:

```python
acc = (M & ~static).sum(axis=0)
still = frames[len(frames)//2].copy(); still[acc > 0] = [255, 0, 255]
```

Frankly's six rays leapt out of it; so did Humpty's arc filling the gap between his antennae, and
his bolts leaving from the tips. **Draw this before measuring anything.**

**6. Decide which kind it is** — see Humpty's section for why this is the question. A composite
that shows **rays** is an emitter. One that shows a **band or a few blobs** is an overlay. Confirm
by tracking one sprite across consecutive frames: an emitter's moves in a straight line at a
steady speed, an overlay's teleports between a handful of anchors.

**7. Check the character is not moving.** Track something small and bright — the eye whites work
on everything so far:

```python
m = (r>170)&(g>170)&(b>170); cx, cy = xs.mean(), ys.mean()
```

Held within a pixel across a clip means nothing shakes, which is what all three showed. And check
the **stone in a corner** against frame one at the same time: a diff over about 5 means the
capture is panning, and Frankly's losing clip does exactly that halfway through.

**8. For a palette cycle**, sample a 7x7 patch at the centre of one lit element and match it to the
nearest entry of the sheet's `With:` ramp each frame. Print the transitions. Sample **two adjacent**
elements as well: same shade every frame means a pulse, different shades means a chase. Arms is a
pulse. Beware of picking a patch that is not on the element — a flat answer for a whole clip means
the patch missed, not that nothing happened.

**Ask for a game over capture too.** Dynamight's and Grounder's are the first, and they earn
their place twice over: the moment of death is the only place the *defeat* row can be read at
all, and the run-up to it is where the shared sweat and the danger flash turn on, so a game over
clip is the one that shows a character at his worst. **Record for several seconds after the
death** — all five of them hold their defeat poses to the end of the recording and none reaches
the halving, so how long that first hold lasts is still unmeasured. A losing clip that runs
straight into the game over works perfectly well and saves a recording: Spike's and Dr.
Robotnik's are cut that way, and labelling the whole clip against every frame of every row makes
the transition fall out.

**Find the loose sprites' anchors on the sheet first.** `find_overlay` brute-forces a loose
sprite against a row's own frames, and lands it exactly whenever the overlay's *rest* state is
what those frames already draw. That is one fewer thing the capture has to answer.

**What to write down, per row:** the number of *poses* (not frames — see below), what is held and
what moves, the dwell of each pose in seconds and in 60 Hz frames, the period of the whole thing,
and for each loose sprite its kind, its anchor or source in box coordinates, its directions, its
speed in box pixels an axis a frame, and its lifetime.

**Pitfalls, all of them paid for:**

* **A frame count is not a pose count.** Two frames can differ only in a palette state, and the
  last frame of a defeat row is usually a fade. The table below has this worked out per row.
* **An automated "palette only" test lies.** Colours mapping *to* the key `(0, 0, 96)` or to black
  is a sprite shrinking or an eye closing, not a swap. Only a map among shades of one ramp, in
  place, is a palette cycle. Read the map before believing it.
* **A one-frame flash is invisible** at 20-30 fps. Humpty's arc was caught eight times in four
  seconds and certainly fired more often. Never report a rate as an upper bound.
* **The static mask hides a moving character part.** Humpty's antenna balls move in his winning
  row, so they survive `~static` and turn up among the loose sprites — which was useful, but only
  once it was noticed. Check the blob sizes: his balls are ~300 px and his bolts ~120.
* **Do not measure speed from two adjacent capture frames.** Fit across a whole burst instead;
  adjacent frames gave 1.5 and 1.7 px for the same spark.
* **The game's own beans fly over the box.** Coconuts' winning clip has a green bean cross it at
  t≈1.8 s, which is the attack animation and not a loose sprite of his. A one-off event that
  never repeats over a whole clip is the game, not the character; a loose sprite of his recurs.
* **Two clips of one character need calibrating once, not three times.** Coconuts' three share a
  framing to the pixel — the label rows are identical in all of them — so measure it on one and
  check the other two against it rather than fitting each.
* **Sample the whole element, not a patch, when checking a cycle.** A patch answers whether one
  colour moved; matching the *whole* element against the frame palette-swapped through the ramp
  answers whether the pair moved together, which is the pulse-or-chase question, and costs the
  same.
* **A capture can outlive the state it was taken of.** Skweel's losing clip goes back to *idle*
  a second before it ends, because his stack came down. So label the tail against **all four
  rows**, not against the row you asked for, and watch the residual: a stretch that matches its
  own row noticeably worse than the rest of the clip is a different row, not noise. That one
  transition is also what proved his losing row loops rather than settling — the row was cut off
  mid-pass.
* **Brighten by more on a late stage.** The wall behind the box takes the stage's palette. Stage
  6 reads as flat black raw and wanted ×3.2 before the stone was visible, where stage 4 wanted
  none.
* **Calibrate once per session, not once per clip.** Grounder's four clips share a framing to
  the pixel and Dynamight's four share two; fit the first and check the rest against it. The fit
  itself wants doing as a **pyramid** — a coarse *isotropic* sweep on a quarter-size copy of the
  frame, then a fine anisotropic refine at full resolution near the answer — because a full
  two-axis search at full resolution is minutes a clip and this is seconds.
* **A colour mask cannot find a blue drop on a blue character.** Counting sweat over the whole
  frame gave Grounder twenty drops a frame, all of them his own face. Count in the **band of
  wall above the box**, which no character's art reaches: it gave 0.00 for the same clip, and
  the visual check agrees.
* **Normalised correlation cannot separate a frame from its halving.** A fade correlates
  perfectly with what it fades, so an NCC classifier reports the two as the same frame — which
  made Sir Ffuzzy-Logik's defeat row look as though it were reaching the fade and coming back
  out of it. Use NCC where the danger flash would otherwise ruin the labelling, and an
  **absolute** difference wherever a halved frame is one of the candidates.
* **A high residual is not a bad fit.** Spike's best alignment sits at 25 where Grounder's sits
  at 13, because he is a saturated orange and the capture spends more of its bitrate elsewhere.
  Check an alignment by **checkerboarding** the crop against the sheet frame — 8-pixel squares
  of each, alternating — which shows a one-pixel error instantly. Two pictures side by side do
  not; that is what cost the time here.

If the remaining three are done in one sitting, all of the above wants to be a script in
`puyo-rusto/art/` beside the ripper rather than retyped — it is about a hundred lines.

#### Frankly — reconstructed, 2026-08-28

Two captures: `Screencast From 2026-08-28 15-41-17.mp4` (winning) and
`Screencast From 2026-08-28 15-49-00.mp4` (losing, running on into defeat). Both are of the
in-match centre column at stage 2 — `STAGE`, the mugshot box, `SCORE` — so the box in them is
the game's own `(120, 96, 80, 56)` and every number below is in **box coordinates**, measured
by calibrating each capture against the `STAGE` and `SCORE` labels, whose screen positions the
ripper already knows.

**Winning: one pose, and six sparks thrown from the antenna balls.**

* **The face does not animate.** The sheet gives the winning row one frame and the capture
  agrees: the portrait is one held pose for the whole clip. Everything that moves is the loose
  sprites.
* **Six sparks a burst, three from each antenna tip ball.** In the winning frame the balls are
  at box `(6, 15)` and `(77, 15)` — found as the gold blobs in the art, and confirmed by
  back-projecting the tracks, which meet them.
* **Each ball throws three, on the 45° diagonals, and omits the one that would go into his
  body**: the left ball throws ↖ ↗ ↙, the right ball ↗ ↖ ↘. Composite every frame's sparks over
  one still and the six rays are unmistakable.
* **They travel out at about 1.5–1.7 box pixels an axis per 60 Hz frame** (≈100 px/s an axis,
  ≈140 px/s along the diagonal). Whether the original steps one whole pixel an axis a frame or
  two is exactly the thing a variable-rate capture cannot tell you.
* **They live about 40 box pixels of travel** — roughly 28 px on each axis, so about 17 frames —
  and are last seen at box x from `-22` to `+98` and y from `-18` down to `+48`.
* **Which means they are drawn well outside the box, and are not clipped to it.** On the Genesis
  they cross the stone of the centre column *and go over the playfield* — the captures show them
  over both players' beans. Our panel belongs to one player, so the same thing here means the
  sparks cross that player's own well. **This is a decision for Alex** and the first real one
  the characters raise: keep it, or clip the sparks to the centre column.
* **A burst about every 0.67 s** (~40 frames at 60 Hz) — but the ten intervals measured ran from
  0.19 s to 1.28 s, which is either a randomised delay or the capture lying. Treat the mean as
  the number to implement and the spread as a question.
* **The four bolt sprites are 8x16 and a single colour, `(192, 160, 0)`** — a dark gold, about
  24 lit pixels each, four different little crackle shapes. Four sprites and six sparks means
  they are *not* one per spark. The likely reading is that each spark cycles through all four as
  it flies, which is the ordinary Genesis idiom and is what four frames are for; the capture is
  too soft at this scale to prove it.

**Losing: the face is held too, and something sweats that is not on the sheet.**

* **The portrait does not change.** Over a steady six-second stretch the box region is
  pixel-identical bar compression noise, even though the sheet gives the losing row two frames.
  So either the second frame is used somewhere this capture never reached, or it alternates far
  more slowly than the clip is long. **Worth a second look on the emulator.**
* **Small round blue drops fly up and out**, one or two at a time, from the **upper corners of
  his head** — not from the antenna balls, which hang low in this pose. Same 45° diagonals as
  the sparks, up-left and up-right, about 3-4 screen pixels across.
* **They carry on into the defeat state**, which the same clip runs into.
* **There is no drop sprite anywhere in the rips**, and Alex could not find one either
  (2026-08-28). Frankly's block on the Mugshots sheet carries the four gold bolts and nothing
  else; `Opponents.png` is full-body cutscene art and `Has Bean.png` is the victory bean. ~~So
  the sweat is shared art the game keeps somewhere these rips do not go.~~ **Half right, and
  corrected on 2026-08-29** — it *is* art these rips do not carry, and it is **not Frankly's**:
  every character sweats the same drops, on a dial, and Grounder's losing clip has none at all
  while his game over clip has them. See *Two things that are not the character* above; what is
  written here is a description of the shared effect, taken off his clip.
* **So cut it out of the video instead** — **once, for the whole cast**. `frankly-losing.mp4`
  has dozens of drops crossing plain dark stone, an upscaled 3-4 pixel sprite each. The job is a
  small one and belongs in `rip_retro.py` beside everything else that reads art: find the drops
  by colour outside the box, take the *median* of every one after registering them on their
  centres — which is what kills the compression noise, since the sprite is the same every time
  and the noise is not — and quantise the result back onto the Genesis's own palette steps
  (multiples of 32). Write it out as one small png beside the character sheets. Write it to take
  a colour and a clip rather than to know about sweat, so it serves any other effect that turns
  out not to be on a sheet.

**What Frankly settles for every other character**, and what it opens:

* A row with **one** frame is a held pose with an overlay doing the moving. That is now shown,
  not guessed, and it is how Frankly's single winning frame was always going to have to work.
* Overlay sprites travel on **45° diagonals from a point on the character**, outward, and are
  **not clipped to the box**. So the character surface needs to be able to draw outside its own
  rect — which the `CharacterLayout` sketched above, a bare `Rect`, cannot do. It needs a
  **canvas** to draw into (the panel) and an anchor within it, not a clip.
* The loose sprites are an **emitter**, not a strip: a source point, a set of directions, a
  speed, a lifetime and a period. That is `particles/source.rs`'s model exactly — fire and
  forget, a group emitted and then left alone — and it is worth asking whether the character's
  extras should be a particle source rather than bespoke code, since the engine already has one
  and it is already what every foreground effect uses.
* And at least one character wants art the rip does not carry.


#### Arms — reconstructed, 2026-08-28

Three captures — `arms-idle.mp4`, `arms-winning.mp4`, `arms-losing.mp4` — which is what a
character costs when every row does something. He is the red saucer with a ring of lights round
its rim, and he answers most of what Frankly could not.

**The lights: one palette cycle, every row, at two speeds.** The sheet's note is right and
under-specified. One colour index cycles — the `(224, 224, 0)` of the lights — through the eight
`With:` shades, and **every light moves together**; adjacent lights sampled frame by frame are
always the same shade, so it is a pulse and not a chase. What the sheet cannot say is that the
*rate and the shape both change with the row*:

| row | shape | a step | a cycle |
|---|---|--:|--:|
| idle | **ping-pong**, bright → dark → bright | ~0.12 s (7 frames) | ~1.65 s |
| winning | **sawtooth**, bright → dark, then snap back | ~0.04 s (2-3 frames) | ~0.33 s |
| losing | sawtooth, the same | ~0.04 s | ~0.32 s |

So the lights *breathe* while nothing is happening and *spin* once the match is going somewhere,
five times faster and one way. That is a free piece of characterisation and it costs one number.

**Idle: three poses, and a blink on a timer.** The row's four frames are not four poses. Frames
1 and 2 differ **only** in the rim rows the lights are on — the ripper caught the same face at
two points in the cycle — and frames 2, 3 and 4 differ only in the eye rows. So it is three
poses: eyes open, half, shut. The capture agrees exactly, finding three states and nothing else.

What it does with them is the useful part:

* **open is held for about 0.82 s**, then **half ~0.10 s → shut ~0.18 s → half ~0.09 s** and back
  to open. A ping-pong blink, about 0.37 s of it.
* **and it comes round every 1.19 s**, measured over five blinks at 1.185, 1.201, 1.184 and 1.200
  seconds. That is as regular as anything here has been.

**An idle is therefore a held pose with an action on a timer, not a loop** — which is a shape
`FrameAnimationType` does not have. Frankly's idle was a single frame, so it did not show; Arms'
does. Every character with 2-4 idle frames is probably this: a rest, and a blink or a twitch
every second or so.

**Winning: the arms pump.** Three frames, and the metal arms rise from below the rim and bend
over him. The capture ping-pongs 1 → 2 → 3 → 2 → 1 continuously, fully-up recurring every
**0.56 s**, which over the four steps of a three-frame ping-pong is about **0.14 s (8 frames) a
step**. It never rests.

**Losing: two frames alternating**, about **0.18 s (11 frames) each**, a 0.36 s cycle, measured
over twelve cycles. No rest and no ping-pong — just the two, back and forth.

**Defeat: one pose and a fade.** Frame 2 is frame 1 at exactly half brightness, every pixel. See
the correction above — this holds for ten of the thirteen characters, so it is a property of the
sheet rather than of Arms.

**What Arms settles for every other character:**

* **The box is a hole.** The most important thing either character has shown, and it is written
  up above where the sheet is described, because it changes how every one of them is cut.
* **Face frames run 6-11 frames at 60 Hz.** That is the number the whole phase was missing.
* **Diff a row's frames before counting poses.** A palette state and a fade both look like frames.
* **The palette cycle is worth having** and baking it as frames is now clearly right: eight shades
  over one pose, and the *rate* is the theme's to choose per row. Baking is cheap because the
  cycling index is one colour and the poses are few — Arms needs 8 shades over 3 idle poses only
  if the blink and the cycle have to stay independent, and they do: 1.19 s and 1.65 s do not
  divide. **So the lights cannot be baked into the face frames at all** — they are a second,
  independently timed layer over the same portrait. Either the renderer gets a palette swap after
  all, or the lights are cut as their own small overlay sprite with eight colour variants and
  drawn on top. The overlay is the cheaper of the two and matches how Frankly's sparks already
  have to work.


#### Humpty — reconstructed, 2026-08-28

Three captures again — `humpty-idle.mp4`, `humpty-winning.mp4`, `humpty-losing.mp4` — and this
time **zoomed to the box**, which is worth knowing before reading the numbers: unlike Frankly's,
these cannot say whether anything leaves the 80x56 rect, because nothing outside it was filmed.
Everything below is in box coordinates, which here is simply the video divided by 4.65.

He is the green frog in a gold pot with two antennae, and he is the character with loose sprites
on **three** rows, which is why he went next.

**Idle: a blink, and an arc that crackles between his antennae.**

* Two poses, differing **only** in the eye rows (16-26, cols 24-48). Open is held about **1.0 s**,
  shut about **0.33 s**, and it comes round every **~1.35 s** — 1.369 and 1.335 measured. The same
  shape as Arms at 1.19 s, so this is now a pattern rather than a coincidence: **an idle is a held
  pose with a blink on a timer of about a second and a third.**
* The two 24x8 gold zigzags are an **electric arc between the two antenna balls**. The balls sit at
  box `(16, 8)` and `(63, 8)`; composite every frame's arcs over one still and they fill the band
  between them, box x 17-60 at y 5-14, and nothing outside it.
* **It does not travel.** It appears at one of about **four positions on an ~8 pixel pitch** across
  that gap, flashes for a frame or two, and goes. The gaps between flashes were 0.12, 0.25, 0.43,
  0.63, 0.72, 0.80 and 0.82 s — irregular enough to read as random rather than periodic, and a
  20-30 fps capture will miss a one-frame flash outright, so the true rate is higher than the eight
  flashes seen in four seconds.

**Winning: he flexes his antennae, and fires from the tips.**

The antennae are part of the 80x56 frame, not an overlay, and the row's four frames are their
positions. Tracking the gold balls gives the whole gesture:

| what | balls at box x |
|---|---|
| rest | 16 and 60 |
| drawn **in** | 28 and 48 |
| flung **out** | 11.6 and 64.5 |

and the cycle is **in → fire → back → rest → out → rest**, about **0.70 s**, run **twice**, then
about **2 s of rest** before it goes again. So the winning row is a gesture on a timer too, not a
loop — the same shape as the idle, with a bigger action.

The three 8x16 bolts are fired **at the moment the antennae are drawn in**, from the tips: two of
them, left and right (occasionally a third up the middle), travelling up and out at about 45° and
**~2.2 box pixels an axis a frame**, alive about 0.1 s. That is faster than Frankly's sparks and
much shorter lived. Whether they carry on past the box is exactly what this capture cannot say.

**Losing: he ducks into the pot and wrings his hands.**

The face alternates two poses, and the three 24x16 green sprites are his **hands**. They are not
projectiles and they do not travel: they appear at a handful of fixed slots either side of him —
box `(18, 24)`, `(18, 27)`, `(50, 32)`, `(56, 34)`, `(57, 27)`, `(63, 31)` — and flicker between
them every 0.1-0.2 s, which reads as wringing.

**The character does not move.** Tracking his eye whites across all three clips puts them within a
pixel of the same place throughout, so nothing here shakes, and the apparent movement in the
losing row is entirely the antennae and hands.

**What Humpty settles, and it is the biggest thing yet:**

**There are three kinds of loose sprite, not one.** Frankly's sparks *travel* — emitted, given a
direction and a speed, and left to fly. Humpty's idle arc *flickers in place*, at one of a few
slots on a pitch. Humpty's losing hands *flicker between fixed anchors*. Two of those three are not
an emitter at all; they are **an overlay slot holding a sprite index, switched on a clock**, which
is much simpler than what the Frankly write-up above reaches for.

So the character surface wants **both**, and the overlay is the commoner and cheaper case:

* an **overlay**: a list of anchor points in box coordinates, a set of sprites, and a rule for
  which is showing when (a flicker, a sequence, or off). Humpty's idle and losing rows, and
  Sir Ffuzzy-Logik's eyes, are all this.
* an **emitter**: a source point, directions, a speed, a lifetime, a period. Frankly's sparks and
  Humpty's winning bolts are this, and only this.

And **a row can be ordinary frame animation as well** — Humpty's winning row animates the
character's own antennae inside the 80x56 frame *and* fires an emitter off them, at a
frame-accurate moment in the gesture. So the two are not alternatives: a row is frames, plus
whichever of the two overlays it wants, and the emitter needs to be able to fire on a nominated
frame rather than on a timer of its own.

#### Coconuts — reconstructed, 2026-08-29

Three captures — `coconuts-idle.mp4`, `coconuts-winning.mp4`, `coconuts-losing.mp4` — of the
in-match centre column at stage 4, so `STAGE`, the box and `SCORE` are all in shot. He is the
monkey in the lab coat with a **gold coin stuck to the top of his head**, and the coin is the
whole of what his two labelled palette cycles act on.

**Calibration, and the thing it caught.** All three clips share a framing, so one calibration
serves the lot: the box is video `(135, 70)` and a box pixel is **4.225 video px across and 4.53
down**. The capture is **anisotropic** — an isotropic fit is 4.6% out on one axis, which the
method's own 3% sanity check would have reported as a bad reading rather than as a non-square
capture, so add that to the list of things step 2 can go wrong on. Two independent confirmations
of the fit: `SCORE`'s ink starts at video x=168, which is screen x=127.8 against the ripper's
nominal 128; and the sheet's idle frame 0, scaled, lands on the box to within a pixel. The
`STAGE` label is *not* usable here, because the plain face is the same white as the speckles in
the stone and a column projection reads its left edge fifty pixels early. Coconuts' frames carry
no key colour at all, so the sheet frame itself is the template — the cleanest calibration any
of these has had, and worth reaching for on any other character whose frames are solid.

**Frame count is pose count, in every row** — the first character on the sheet of which that is
true. Nothing folds away: idle 2, winning 2, losing 3 are all distinct poses, and defeat is one
pose and a fade.

**He has no loose sprites and nothing outside the box.** A motion composite over each clip —
per-pixel maximum deviation from the median frame — lights up the coin and the eyes and nothing
else inside the box. The one moving thing that crosses the box and is not him is a **green bean
flying past** on the winning clip at t≈1.8 s, which is the game's own attack animation over the
centre column. Worth knowing before it is mistaken for an emitter on some other character:
whatever is happening on the boards travels over this box.

**He does not move.** The eye whites hold within 0.6 box px across the idle and losing clips
(the ±0.3 in winning is the wink changing which pixels are white), and the stone in a corner
does not drift, so none of the three captures pans.

**Idle: a double blink on a ~1.68 s timer.** Two poses, differing only in the eye rows (24-29,
cols 19-34) — open, and a narrowed glare rather than a full close. The classifier finds exactly
two states and the shape is:

| | dwell | at 60 Hz |
|---|--:|--:|
| open | 1.17 s | 70 |
| narrowed | 0.14 s | 8 |
| open | 0.23 s | 14 |
| narrowed | 0.12 s | 7 |

and round again — the two measured periods are 1.686 and 1.671 s, so **~1.68 s, 101 frames**. He
blinks *twice* in quick succession and then rests, where Arms (1.19 s) and Humpty (1.35 s) blink
once. An idle is still a held pose with an action on a timer; the action is not always one beat.

**Winning: one eye winks, on a ~1.36 s timer.** Two poses, the diff being his screen-left eye
(rows 25-30, cols 29-36). Open is held **0.85 s (51 frames)** and the wink **0.53 s (32
frames)**, four periods measured at 1.335, 1.385, 1.385 and 1.351 s. Nothing below row 33 moves:
the raised finger and the coat are held for the whole clip.

**Losing: he shakes his raised fist**, and that is all that moves. The three frames run as a
strict ping-pong 0 → 1 → 2 → 1 → 0, **5 frames (0.083 s) a step**, so **0.333 s round** —
fifteen cycles measured, mean 0.333 s. It never rests and there is no pause between cycles. The
face and everything below row 26 are static.

**The two labelled cycles do exactly what the sheet says — and each has its own shape and
rate, which the sheet does not say.** The ramps below are read off the swatch pixels rather than
by eye, and the capture uses every step of both, in order, on the rows named and on no other:

| | replace `(224,128,0)` rim | replace `(224,192,96)` face |
|--:|---|---|
| **winning** 1 | `(64,32,0)` | `(64,32,0)` |
| 2 | `(128,64,0)` | `(128,64,0)` |
| 3 | `(160,96,32)` | `(160,96,32)` |
| 4 | `(160,96,32)` | `(192,128,64)` |
| 5 | `(192,128,64)` | `(224,160,96)` |
| 6 | `(224,160,96)` | `(224,192,128)` |
| 7 | `(224,192,128)` | `(224,224,160)` |
| 8 | `(224,224,160)` | `(224,224,224)` |
| **losing** 1 | `(224,192,128)` | `(224,224,224)` |
| 2 | `(160,96,64)` | `(192,128,96)` |
| 3 | `(64,32,0)` | `(64,32,0)` |
| 4 | `(224,0,0)` | `(224,0,0)` |
| 5 | `(128,0,0)` | `(128,0,0)` |
| 6 | `(32,0,0)` | `(32,0,0)` |

* **Winning is a ping-pong, 36 frames (0.601 s) round**, measured over ten cycles at 0.583 to
  0.634 s. Phase-folded it is unambiguous: **2 frames a step** through the six middle steps in
  each direction, with the **bright end held about 7 frames and the dark end about 5** —
  12×2 + 7 + 5 = 36. It reads as the coin catching the light and flaring white.
* **Losing is a sawtooth, 24 frames (0.400 s) round**, measured over fourteen cycles at 0.350 to
  0.451 s: **4 frames a step**, one pass 1 → 6, then a snap back to 1. Uniform — no step is
  held. Since the ramp runs pale white → brown → bright red → dark red → near black, the snap
  back to white is part of the effect and not an artefact of it: it flushes red and resets. **It
  must not be ping-ponged**, or the flush loses its beat.
* **Idle does not cycle at all.** The coin patch's colour has a standard deviation of 0.2 over
  the whole idle clip. The base pair is what idle holds.
* **Both colours of the pair move together.** Matching the *whole* coin against the sheet frame
  palette-swapped through the ramp gives the same step sequence as matching a patch of the face
  alone, so it is a pulse over one two-colour element and not a chase — the same answer Arms
  gave for his lights.

**The base pair is not on either ramp, and the coin is different art per row.** Idle and defeat
draw it with a silver engraved face — `(224,224,224)` whites and `(160,192,224)` /
`(128,128,160)` blues inside the gold rim — while winning and losing draw a flat gold disc, 69
and 82 px of `(224,192,96)`. So the cycle is not the only thing that changes about the coin
between rows: cut all eight frames as they are rather than recolouring one coin for all four
rows.

**Defeat is a fade**, and it is the one thing about him still open. 98.8% of the frame is exactly
halved; the 1.2% that is not is the coin, where 24 px of `(224,128,0)` map to `(112,80,16)` —
the halving of `(224,160,32)`, not of `(224,128,0)`. So **the ripper caught the two defeat
frames at different points of a cycle**, and whatever the defeat row does with the coin it is
not holding the base pair. No defeat capture was taken. If one is, that is the question to
answer, along with whether the fade plays once and stops.

**Cutting him.** No teal key anywhere in his eight frames, and flood-filling the `(0, 0, 96)`
from the border leaves no enclosed key either, bar a single stray pixel at (40, 51) in each
losing frame and one more elsewhere in losing frame 2 — one-pixel holes that will show a speck
of stone. So he keys cleanly on the navy and wants none of the per-character eyeballing the
sheet section warns about.

**What Coconuts settles:**

* **A labelled cycle does what its label says**, which is what he was for. Both ramps are used
  in full, in order, on the pair named, on the rows named, and nowhere else.
* **But a label says nothing about shape or rate, and his two differ on both** — a ping-pong at
  36 frames against a sawtooth at 24, on the same element in the same character. Arms showed the
  same for an *unlabelled* cycle. So shape and rate are always the capture's to give and never
  the sheet's, and there is no reason to expect any two of them alike.
* **A row's cycle is independent of its frame animation**, again: 1.36 s against 0.60 s in the
  winning row, 0.33 s against 0.40 s in the losing one, and none of those divide. Which is Arms'
  finding a second time and now settled — **the cycled element is its own layer over the
  portrait, on its own clock, and cannot be baked into the poses.** Coconuts is the cheapest
  possible case of it: one small sprite, the coin at box rows 0-11, cols 44-60, entirely inside
  the box and never overlapped by anything.
* **An idle need not blink once.** So the idle shape wants to be a short *sequence* on a timer
  rather than a single blink beat.

#### Skweel — reconstructed, 2026-08-29

Three captures — `skweel-idle.mp4`, `skweel-winning.mp4`, `skweel-losing.mp4` — at **stage 6**,
which matters for one thing below. He is the pink slug with the pig's snout, and he is the
character with six losing frames.

**Calibration.** The idle clip is framed differently from the other two, so it wants its own:
the box is video `(86, 60)` on the idle clip and `(53, 56)` on the winning and losing ones, and
a box pixel is **3.77 across and 3.79 down** — near enough square, where Coconuts' three were 7%
out. So a capture's anisotropy is per-session and not a property of the recorder. Refitting the
box on a late frame of each clip moves it by at most a pixel, so none of the three pans.

**Stage 6's wall is much darker than stage 4's.** Raw, the box interior reads as flat black and
looks like it might be opaque after all; brightened ×3.2 the same speckled dungeon course is
plainly there behind him. The box is still a hole — but the check needs a harder brighten on a
late stage than it did on Arms and Coconuts, because the wall takes the stage's palette.

**No palette cycle, no loose sprites, and nothing outside the box.** A motion composite over each
clip lights the eyes (idle), the whole character (winning) and the snout and the sneeze (losing),
and nothing else; everything moving outside the box is the two boards, which the framing catches
at both edges. He never reaches the box's own edges either: his art sits in cols 19-63 at rest
and swings to col 5 and col 75 at the extremes of the winning sway.

**He does not move.** The losing clip holds his eye whites within **0.19 box px** across every
pass. The movement in the winning row is the sway, which is frame animation and not a shake.

**Idle: a three-pose blink, yo-yo, on a ~1.27 s timer.** The frames differ only in the eye rows
(11-30, cols 22-48) — open, half, shut — and they run **0 → 1 → 2 → 1** and back to a long hold
on 0:

| | dwell | at 60 Hz |
|---|--:|--:|
| open | 0.97 s | 58 |
| half | 0.10 s | 6 |
| shut | 0.10 s | 6 |
| half | 0.09 s | 5 |

Three blinks measured at 1.236, 1.319 and 1.249 s, so **~1.27 s, 76 frames**, of which the blink
itself is about 18. Which is Arms (1.19 s) and Humpty (1.35 s) again; Coconuts' double blink
remains the odd one.

**Winning: he sways his whole body, and never rests.** Three poses — leaning left, upright,
leaning right — run as a **continuous yo-yo 0 → 1 → 2 → 1** with no pause between passes over
six cycles. Frame 0 is held about **8 frames** and frames 1 and 2 about **10** each, period
measured at 0.600, 0.667, 0.619, 0.634 and 0.635 s, so **~0.63 s, 38 frames**. The sway is
worth a number: his snout's centroid crosses box x **25.5 → 39.6 → 54.0**, a **28-pixel swing**,
dipping about 5 rows in the middle — so nearly half the box wide, which is the largest movement
any character has shown.

**Losing: a sneeze, and it is a LOOP.** That was the question he was for, and the answer is
unambiguous: the six frames run **0 → 1 → 2 → 3 → 4 → 5 and straight round again**, four
complete passes back to back with no rest and no settling, at **~6-7 frames a step** — period
measured 0.667, 0.636 and 0.667 s, so **~0.66 s, 40 frames**. It is **not** a ping-pong, which
makes it the first straight loop on the sheet.

What it draws: his **snout tips up and his mouth opens**, and a grey-white puff blows out of the
nostrils, expands up and to the left, and is gone; frames 4 and 5 are the snout settling back
with the mouth closing, which is why three of the six differ from each other only in the rows
under the snout. The puff, measured with his own greys masked out by column:

| frame | puff | centroid | reach |
|--:|--:|---|---|
| 1 | 82 px | (20.7, 25.7) | cols 12-27 |
| 2 | 74 px | (18.3, 21.9) | cols 6-27, rows 12-33 |
| 3 | 50 px | (20.8, 22.2) | cols 8-27, rows 10-33 |
| 4 | — | — | gone |

**The puff is drawn into the 80x56 poses.** It is not a loose sprite and not an overlay: it is
part of the frames, it never leaves the box — its furthest reach is box col 3 on frame 3, three
pixels short of the edge — and the motion composite finds nothing of his outside. So it costs no
machinery at all.

**And the clip outlived the state, which is what proves the loop.** At **t = 2.721 s** he stops
mid-pass and goes back to **idle** — the frames from there match the idle row's frame 0 and he
blinks at 3.72 s — because his stack came down. So the losing row does not play out and settle
on a pose; it loops for as long as the state lasts and is **cut off** when it ends, on one frame,
with no blend.

**Defeat is an exact fade** — 100% of the frame halved, pixel for pixel, the cleanest on the
sheet (Coconuts' was 98.8%, and only because of his cycling coin).

**Cutting him.** No teal key in any of his fourteen frames, and flood-filling the `(0, 0, 96)`
from the border leaves nothing enclosed but 9 px inside the sneeze puff on losing frame 1 (rows
24-27, cols 17-21) and one stray pixel in idle frame 2 — the puff's own gaps, which correctly
show the wall. He keys cleanly on the navy.

**What Skweel settles:**

* **A row can be a straight loop.** Every row read before him was a ping-pong, a two-frame
  alternation or a held pose, and six frames looked like it had to be a one-shot. It is not. So
  a frame count says nothing about the *shape* either, and `Linear` earns its place beside
  `YoYo`.
* **An effect can live inside the frames** — a third place, beside the emitter and the overlay,
  and by far the cheapest: no anchor, no clock, no machinery. Ask which of the three an effect is
  before reaching for either of the other two. Frankly's sparks and Humpty's bolts leave the box
  and so cannot be this; Skweel's sneeze never gets within three pixels of the edge and so can.
* **`FrameAnimationType` covers him whole**, which extends question 4's answer past Arms. His
  winning row is `YoYo { fps: 6 }` and his losing row `Linear { fps: 9 }`. His idle is the
  **refugee bean's own trick** already in `theme/genesis/mod.rs` — `LinearWithPause` over a strip
  cut with the blink first and the rest last, so the frame the pause holds is the open eye. His
  sheet order is open, half, shut, so the strip is cut `1, 2, 1, 0` with `resume_from_frame: 0`.
  That is the shape of every idle read so far, and it is already implemented.
* **A state ends by cutting the row off, not by letting it finish.** Worth knowing before any of
  this is built, and free — it fell out of a capture that ran a second too long.
* **Brighten harder on a late stage.** The wall behind the box takes the stage's palette, so the
  "is the stone really there" check wants ×3 on stage 6 where it wanted nothing on stage 4.

#### Dynamight — reconstructed, 2026-08-29

Four captures — `dynamight-idle.mp4`, `-winning.mp4`, `-losing.mp4` and, for the first time,
`-game-over.mp4` — at **stage 7**. He is the red stick of dynamite with a **T-handle plunger
detonator** standing on his head, and everything he does he does with the plunger.

**Calibration.** The idle and winning clips have the box at video `(99, 68)` and the losing and
game over clips at `(91, 65)`; all four are **3.79 across and 3.81 down**, the same near-square
scale as Skweel's. Grounder's four clips are at `(99, 68)` and the same scale, so that framing
is now the common one and is worth trying first on any capture that follows.

**Idle: his grin works, and it never rests.** Two poses differing only in the mouth rows
(39-55, cols 24-52), alternating flat out at **~20 frames each**, period **0.69 s (41 frames)**
over four cycles. **The first idle on the sheet that is neither a blink nor a held pose** — no
long dwell, no action on a timer, just two frames back and forth. So "an idle is a rest with a
blink" is a pattern and not a rule.

**Winning: the face does not move at all — the plunger does.** All five frames are **pixel-
identical below row 16**; every difference in the row is the T-handle waving across the top
right of the box. They run as a **straight loop 0 → 1 → 2 → 3 → 4**, ~5.4 frames a step, period
**0.446 s (27 frames)**, measured 0.400 to 0.500 s over seven cycles. The handle swings in from
the right edge (art to col 78 on frame 0), left to col 64, and back out — touching the box's
right edge and never crossing it, which the motion composite confirms.

So this is **Frankly's shape** — a held portrait with the moving thing over it — but drawn
*into* the frames the way Skweel's sneeze is, rather than as a loose overlay sprite. Five frames
of an 80x56 portrait to animate a stick, which is what the sheet's block-per-character layout
costs and is also why it needs no machinery.

**Losing: he hammers the plunger, and it is the fastest row on the sheet.** Two poses — handle
**down held ~10 frames**, **up ~5** — period **0.246 s (15 frames)** over fourteen pumps. Four
pumps a second. Nothing else in the frame moves.

**Defeat: the explosion, and the one exception to the fade rule.** `defeat[0]` is a full-box
blast of smoke, rubble and red shards — the only defeat frame on the sheet that is not a
portrait — and `defeat[1]` is the exact halving of **`losing[1]`**, his own plunger-down pose,
not of the explosion. So he blows himself up and what is left is his losing face, dimmed. He is
the only character for whom the last defeat frame halves a frame from another row; see the table
under *The source, and the grid it is on*.

**The game over capture** runs 6.55 s of the losing pump — keeping its 0.25 s beat right to the
last frame of it — and then holds the explosion, unchanged, for the remaining 0.92 s of the
recording. It never reaches the halved frame. See *Two things that are not the character* for
what that does and does not settle.

**Sweat and flash.** He sweats from the first frame of the losing clip (1.05 drops a frame in
the band above the box) and starts flashing at t≈1.4 s, +55% on the box's mean brightness. Both
are the shared effects, not his.

#### Grounder — reconstructed, 2026-08-29

Four captures — `grounder-idle.mp4`, `-winning.mp4`, `-losing.mp4`, `-game-over.mp4` — at
**stage 8**, all four framed identically: box at video `(99, 68)`, **3.79 across and 3.81 down**.
He is the blue drill robot, and he is the one character so far whose **art fills the box edge to
edge** — cols 0-79 in his winning and losing rows, cols 8-79 in his idle — so there is no spare
margin at his sides at all.

**Idle: a three-pose blink, and much the slowest on the sheet.** Ping-pong **0 → 1 → 2 → 1**,
each blink step about **6 frames**, the open pose held about **120 frames (2.0 s)**, and the
whole thing round every **~2.33 s (140 frames)** — measured 2.319 and 2.335 s, which is as
regular as Arms was. Set beside the others: Arms 1.19 s, Skweel 1.27 s, Humpty 1.35 s, Coconuts
1.68 s (a double blink), Grounder 2.33 s. **A blink period is a per-character number and the
spread is nearly two to one**, so it is worth taking off every capture rather than sharing one
constant.

**Winning: a double eyebrow-raise on a timer.** Two poses — the diff is his **brow and his
grin**, which both lift — run as **raise, drop, raise, then rest**: about 12 frames up, 9 down,
13 up, then **~60 frames** held, round every **~1.56 s (94 frames)**, measured 1.552 and 1.568 s.
Which is Coconuts' double blink again, on a different row: **a two-beat action then a long rest
is now a shape that recurs**, and it is the second thing after the blink that
`LinearWithPause` over an unrolled strip covers exactly.

**Losing: a gulp on a timer.** Two poses — mouth shut is the rest, held **~81 frames (1.34 s)**,
then the mouth opens with the tongue out for **~17 frames (0.28 s)** — round every **1.62 s (97
frames)**, measured 1.601 and 1.635. Note the sheet's order: `losing[0]` is the *action* and
`losing[1]` is the rest, so his strip is already cut action-first and wants the pause on the
second frame.

**Defeat** is `defeat[0]` — he covers his face with his drill hand — and its exact halving.

**The game over capture** is the clean one of the two. The losing gulp keeps its timer to the
moment of death at **t = 1.366 s**, and from that frame the box is **`defeat[0]` held perfectly
still** for the remaining **1.84 s**: box mean flat at 76.1, the match against `defeat[0]` flat
at 25.0, the match against `defeat[1]` flat at 42.1. Nothing fades. That is the longest look
anyone has had at a Mean Bean Machine game over and it still does not reach the halved frame.

**Sweat and flash: he is the control.** His losing clip has **neither** — zero drops in 103
frames and a box brightness flat to 1% — and his game over clip has **both**. Same character,
same framing, same `losing` row on his face throughout. That pair is what proves both effects
are graded by how bad the board is rather than being part of the losing animation, and it is
written up above.

#### Spike — reconstructed, 2026-08-29

Three captures — `spike-idle.mp4`, `-winning.mp4`, `-losing-and-game-over.mp4` (one clip for
both, which works well) — at **stage 9**. He is the orange blob with the quiff, and his art
**fills the box in every frame**, rows 0-55 and cols 0-79 throughout, so there is no margin
anywhere.

**Calibration.** Box at video `(97, 61)`, **3.64 across and 3.62 down**. Worth a note: the
absolute-difference fit and the normalised-correlation fit agree exactly here, but the *residual*
is 25 where Grounder's is 13 — because Spike is a saturated orange against dark stone and the
capture's compression costs more on him. **A high residual is not a bad fit**; check with a
checkerboard of the crop against the sheet frame rather than by eye on two separate pictures,
which is what wasted the time here.

**Every one of his three rows is the same shape: a short action, twice, then a long rest.**
That is the shape Coconuts' idle and Grounder's winning row each showed once; Spike does it
three times over, which makes it the commonest shape on the sheet.

| row | what it is | action | rest | period |
|---|---|--:|--:|--:|
| idle | a three-pose blink, ping-pong 0→1→2→1 | ~6-8 fr a step, ~21 fr | 123 fr | **2.40 s (144 fr)** |
| winning | he laughs — mouth open, twice | 14 fr, gap 12, 12 fr | 102 fr | **2.34 s (140 fr)** |
| losing | hands to his cheeks, mouth clamping shut twice | 11 fr, gap 8, 10 fr | 90 fr | **1.99 s (119 fr)** |

Note which frame is the rest: in the losing row the sheet's `losing[0]` — mouth **open** — is
what he holds, and `losing[1]` is the brief clamp. In the winning row `winning[0]` is the held
grin. So both are already cut action-last, which is the opposite of Grounder's, and the strip
wants reordering before it can go through `LinearWithPause`.

**Game over:** the losing timer runs to the last frame of it, then `defeat[0]` is held dead
still for the remaining **1.73 s**, its residual flat at 31.2 and the box's mean at 98.9-99.0.
No fade. **Flash** +40% on the box mean, 46 of 143 frames. **Sweat** 0.08 a frame while losing
and 0.27 after the death — the lightest sweat of anybody who sweats at all, which fits where his
stack was.

#### Sir Ffuzzy-Logik — reconstructed, 2026-08-29

Four captures — `sir-ffuzzy-logic-idle.mp4`, `-winning.mp4`, `-losing.mp4`, `-game-over.mp4`.
He was the one to leave until last and he earns it: **three things animate at once on three
different clocks**, and all three change rate per row. He is the orange furry thing in a
diving helmet.

**Calibration.** Box at video `(92, 86)`, **3.76 across and 3.84 down**; the game over clip is
`(91, 87)` at 3.80/3.80, fitted on a *late* frame because its first frame is mid-match and fits
badly.

**1. The fur is an animated dither, and it is not a palette cycle.** The three frames of every
row differ over the whole 80x56 by 1386-1463 px, and the map between them runs *both ways*
between the same pairs of oranges — `(224,128,96)` ↔ `(192,96,64)` 244 and 211 px, and so on
through five shades. A palette swap is a function; this is not. So all three frames are real and
none folds away, exactly as the sheet prep guessed.

**It runs as a three-frame ping-pong 0 → 1 → 2 → 1, continuously, at a rate set by the row:**

| row | a step | a cycle |
|---|--:|--:|
| winning | ~10 fr | **40 fr (0.665 s)** — measured 0.651-0.670 over five |
| defeat | ~9 fr | **36 fr (0.595 s)** — measured 0.584-0.601 over four |
| idle | ~8 fr | **32 fr (0.536 s)** — measured 0.503-0.567 over four |
| losing | ~6 fr | **24 fr (0.392 s)** — measured 0.350-0.418 over eight |

So his fur settles when he is winning and ripples fastest when he is losing, which is the same
free characterisation Arms' lights gave and is again a thing the sheet cannot say.

**2. The eye yellow cycles, on its own clock, also per row.** The ramp, read off the swatch
pixels: replace `(192,192,0)` / `(160,128,0)` with

| | light | dark |
|--:|---|---|
| 1 | `(96,96,0)` | `(64,32,0)` |
| 2 | `(128,128,0)` | `(96,64,0)` |
| 3 | `(160,160,0)` | `(128,96,0)` |
| 4 | `(192,192,0)` | `(160,128,0)` — **the base pair itself** |
| 5 | `(224,224,64)` | `(192,160,32)` |

The base pair being step 4 of its own ramp is worth knowing: the eyes pulse *about* their rest
colour rather than away from it. Sampling the 37 light-eye pixels frame by frame gives five flat
plateaus at 95, 123, 150, 179 and 205 — evenly spaced, the ramp's own 96/128/160/192/224 pulled
down at the top by blending — running as a clean **ping-pong 1 ↔ 5**. Rate: **~1.2 s** on idle,
**0.518 s (31 fr)** on winning (measured 0.500-0.550 over six, very tight), and faster still
while losing, around 0.3 s, where the blink below makes it hard to measure cleanly. None of
those divides the fur's period on the same row, so it is a third independent clock.

**3. The loose eyes blink, and only on the losing row.** The three 32x24 sprites are his two
eyes — wide, narrowed, shut — and **their anchor is box `(24, 8)`**, which did not need the
video at all: matching the *open* sprite against each of his own losing frames finds the same
offset three times over with a residual of 4.4, because the open eyes are what those frames
already draw. **That is the way to place any overlay whose rest state is in the frame**, and it
is a good deal cheaper than reading it off a capture.

Classified against the three sprites at that anchor, the losing clip blinks
open → half → shut → half → open, **shut for 2-7 frames**, at intervals of 1.234, 0.419, 1.202,
0.784 and 1.267 s. Read that as **about every 1.2 s with shorter ones between**, and remember
the pitfall: a two-frame shut is easy to miss at 22 fps, so the true rate is at least this. The
idle and winning clips show no blink at all — his eyes only close when he is losing.

**4. Defeat: his four frames are three poses and a fade, and the three poses keep animating.**
This is the one that matters beyond him. From the moment of death at t = 4.570 s the box runs
**`defeat[0]` → `[1]` → `[2]` → `[1]` → `[0]`**, a ping-pong at ~9 frames a step, **36 frames
(0.595 s) round**, measured 0.600, 0.601 and 0.584 s over four cycles — and it was still going
2.32 s later when the recording stopped. `defeat[3]`, the halving, is never reached: its
residual sits flat at 52-53 while the other three swing between 19 and 26, and the box's mean
holds at 99.5. Checked with an *absolute* comparison, not a normalised one, because the halving
correlates perfectly with what it halves and only the absolute measure can separate them.

So the defeat row is his own dither still running, at a rate of its own. See *the game over* in
the sheet section, which this settles.

**Flash and sweat.** The losing clip: no flash (+3%, 0 of 133 frames) and 0.20 drops a frame.
The game over clip: +35% and 0.64 drops a frame. The same ladder as everyone else.

#### Dragon Breath — reconstructed, 2026-08-29

Four captures — `dragon-breath-idle.mp4`, `-winning.mp4`, `-losing.mp4`, `-game-over.mp4`. The
spiked silver-and-gold dragon head, and the most metronomic character on the sheet.

**Calibration.** Box at video `(77, 59)`, **3.71-3.74 across and 3.78-3.80 down** across the
four clips, which agree to a pixel.

**Idle: a three-pose blink** — open, half, shut, ping-pong 0 → 1 → 2 → 1 — **~8 frames a step**
(about 24 frames of blink), the open pose held **~91 frames (1.52 s)**, round every
**1.93 s (116 frames)**: measured 1.952 and 1.901.

**Winning: one beat, not two.** He narrows his eyes and widens the grin for **21 frames
(0.35 s)**, holds the rest for **71 frames (1.18 s)**, and comes round every **1.536 s (92
frames)** — measured 1.537, 1.535 and 1.535 s. Three intervals inside 2 ms of each other is the
tightest timing measured anywhere in this phase, and it is worth saying because it shows the
*capture* is not the limit: where the game is regular, a 24 fps recording says so.

**Losing: a lip-flutter, three times, then a rest.** The three poses are his lower jaw and
tongue. He holds `losing[0]` for **~62 frames (1.03 s)**, then runs **1 → 2 → 1 → 2 → 1 → 2** at
about 5-6 frames a step — six steps, roughly 34 frames — and drops back. Round every **1.60 s
(96 frames)**, measured 1.618, 1.601 and 1.584. So the prep's guess of "a three-frame loop
rather than a one-shot" was close but not right: it is a rest plus a **repeated two-frame
flutter**, which is a fourth shape and the one nearest to Spike's double.

**Game over:** `defeat[0]` held dead still for **1.75 s**, residual flat at 24.7-24.9 and box
mean at 78.2. No fade. **Flash:** none at all while losing (+2%, 0 of 148 frames), +38% on the
game over clip. **Sweat:** 0.47 drops a frame while losing.

#### Davy Sprocket, Scratch, Dr. Robotnik — read off the sheet, 2026-08-29; **timings outstanding**

**Captures exist for all three** and are in `~/Videos/Screencasts/` with the rest —
`davy-sprocket-{idle,winning,losing,game-over}.mp4`,
`scratch-{idle,winning,losing-and-game-over}.mp4`,
`dr-robotnik-{idle,winning,losing-and-game-over}.mp4` — but **the shell died before they could be
read** (2026-08-29; every command including `echo` failing, so no ffmpeg, no classifier, no
calibration). What follows is the sheet, the frames themselves, and what the other ten say to
expect. **Nothing below marked *expected* is measured.** Whoever picks this up should run the
three clips through the method above and replace those lines with numbers; it is an hour's work
and it is the only thing standing between this section and the rest.

Two of the three have their losing row buried inside the game over capture rather than in a clip
of its own, which is fine — Spike's was too, and it worked: label the whole clip against every
frame of every row and the transition falls out. Note that means the *clip* name is
`losing-and-game-over` rather than either row; `extract()` builds the filename from whatever
string it is handed, so pass the clip name and not a row name.

**Where to point the classifier** is the last three rows of the region table in step 2 of the
method — worked out from the sheet diffs below and never tried against a capture, so treat them
as a starting point. None of the three has a calibration yet either; fit one per clip and
checkerboard it before trusting it.

**Davy Sprocket** — `(1, 257)`, 2/2/2/2, no extras and no cycle. The purple robot in a studded
cap with a rocket on his shoulder.

* **idle** — 79 px, rows 22-26, cols 23-43: his **eyes**, open then gone. A blink, and one of the
  cleanest on the sheet.
* **winning** — 465 px, rows 7-41, cols 24-79: the **rocket swings** on his shoulder *and* the
  four studs on his cap change from silver to gold. The studs are gold in every idle frame, so
  the winning row is where they differ. **Check whether that is a strict colour map**: if it is,
  it is a palette cycle the ripper did not label, which would make it the second after Arms'.
* **losing** — 279 px, rows 28-55, cols 23-48: his **beak**, open then shut, under half-lidded
  eyes.
* **defeat** — 2 frames, halving **0.992**. Not the exact 1.000 the others give; 8 px in a
  thousand differ. For Coconuts that shortfall was his cycling coin caught at two points of its
  ramp, so **find out what Davy's is** — it may be the studs, which would confirm the cycle
  above.
* No teal key and **no enclosed key anywhere**, so he cuts clean.
* *Expected*: a blink on a timer somewhere between 1.2 s and 2.4 s; the other two rows a held
  pose with one or two beats. **He is still the one to build the plumbing against** — 2/2/2/2,
  no overlay, no emitter, no cycle — so take his numbers first.

**Scratch** — `(1, 743)`, 2/2/2/1. The white-and-red rooster.

* **idle** — 69 px, rows 26-39, cols 35-54: a small change around the beak and cheek. Subtle
  enough that the classifier wants pointing exactly there.
* **winning** — 86 px, rows 4-14, cols 28-46: high up, in the **red comb**, while he grins. The
  second-smallest winning step on the sheet.
* **losing** — 333 px, rows 36-55, cols 14-48: his **beak gapes open**, red tongue showing, then
  shuts. His biggest diff, and the one to classify on.
* **defeat** — **one frame**: beak wide open, wailing. **No fade, and that is now explained
  rather than odd** — the game over is the defeat row's poses and then the final frame held, and
  when the row has one frame those are the same frame. So Scratch's game over is that pose,
  held, at full brightness, for ever. The question the sheet raised for him is answered without
  a capture.
* No teal key and one stray enclosed pixel. Cuts clean.

**Dr. Robotnik** — `(169, 743)`, 3/2/2/1. The moustache in close-up, and the odd one out twice
over.

* **idle** — 14 px then 9 px, rows 22-25, cols 41-50. **The smallest animation on the entire
  sheet**: two or three pixels of eye, and the second step maps `(64,64,96)` and `(160,128,192)`
  to black, which is a lid closing rather than a palette step. So it is a blink of almost
  nothing. Worth knowing before the capture: **a 9 px change is inside the compression noise of
  these recordings**, so the classifier must be pointed at those ten columns and nothing else,
  and the threshold dropped well under the 4.0 the others used.
* **winning** — 875 px, rows 1-41, the whole width: he raises a **gloved finger** and his hat and
  goggles shift with it. A big, obvious step.
* **losing** — 373 px, rows 31-55, cols 0-26: his **gloved fist** at the bottom left, and his
  face unchanged.
* **defeat** — **one frame**: mouth wide open, wailing, hands up. Held at full brightness, like
  Scratch's.
* **He is the character the cut has to be eyeballed for.** Flood-filling `(0, 0, 96)` from the
  border leaves **48 enclosed pixels in every idle frame, 55-60 in each losing frame and 20 in
  the defeat frame** — by far the most on the sheet, where nine of the thirteen have none at all
  and the rest have one to thirteen. Some of that will be wall showing between his moustache
  strands, which is right; some may be dark detail in his goggles, which a plain colour key
  would punch a hole through. **Look at him keyed before shipping him**, which is what
  `rip_retro.py check` is for.
* And the standing question, which is Alex's rather than the capture's: he is the player
  character of the original, so **is he in the deal at all, or is he the boss of the set and out
  of it?**

**The enclosed-key census**, since it only matters for the cut and it is now complete: Grounder,
Spike, Dragon Breath, Davy Sprocket, Arms, Frankly and Humpty have **none**; Coconuts, Skweel and
Scratch have one or two stray pixels; Dynamight has 13, inside the explosion's smoke; Sir
Ffuzzy-Logik has a steady 8 and 2 on the same two frames of every row; **Dr. Robotnik has 48 to
60**. Key by colour for all of them and eyeball only the last two.

### `snes` and `3ds` afterwards

Kirby's Avalanche has the same idea and its own sheet — `SNES - Kirby's Avalanche -
Miscellaneous - Battle Faces.gif`, 662x1742, unread so far — and its panel has the arch at the
foot of the centre column, where the game stands Kirby and this one currently stands nothing.
That arch is the box on that theme, and it is why the plank was laid across its mouth in
phase 3e rather than the hole being left. Chronicle stands both fields on one painted scene
and has no box at all; whether it gets a character, and where, is open.

Do `genesis` first and completely, and do not generalise the plumbing across the other two
until it has been played.

### Done when

`genesis` deals every player a character, the four states move as the table says without
flip-flopping in a real match, both players of a two player match get different faces, the
nuisance tray still has somewhere to live, and `frame_shot` still runs.

### Handover notes

_(to be filled in by the agent that completes this phase. What follows covers the **reading**,
which is all that has happened so far — no code has been written for this phase bar the
analysis script.)_

**2026-08-29, the reading.** Ten of the thirteen characters were read off the emulated game over
two days, on top of Frankly, Arms and Humpty from the 28th. Everything measured is in their
sections; what follows is only what a reader would not otherwise find.

* **Nothing was built.** One file was added — `puyo-rusto/art/mugshots.py`, the analysis script
  the method section had been asking for — and it is not wired to anything and nothing imports
  it. It was assembled from the working code *after* the shell died and has **not been re-run**,
  so treat a first run as a shakedown.
* **Three characters are captured but unprocessed**: Davy Sprocket, Scratch and Dr. Robotnik.
  The shell in that session stopped working — `echo` returning a non-zero exit — so no ffmpeg,
  no classifier, no calibration. Their section carries the sheet reading, the frames described,
  and what the other ten predict, all marked. **That is the first job, and it is an hour.**
* **Two findings changed the design rather than adding to it**, and both were Alex's
  observations first, confirmed against the captures afterwards. The sweat is one shared graded
  effect, not per-character art — which deletes the bespoke per-character rip the Frankly
  section used to call for. The danger flash is the sprite plane and is deliberately **not**
  being implemented.
* **The defeat row is simpler than feared.** `Static` on the first frame for twelve of thirteen,
  `YoYo` over three frames for Sir Ffuzzy-Logik, and the halved last frame has never been seen
  to arrive in any capture — so it can be ignored to start with.
* **The whole reading maps onto `FrameAnimationType` with nothing new**, which is the most
  build-relevant thing to come out of it. Every row of every character read is one of:

  | what the row does | how to build it |
  |---|---|
  | a held pose | `Static` |
  | a continuous alternation or ping-pong | `YoYo` (two frames: either) |
  | a straight loop | `Linear` |
  | a held pose with an action on a timer, once or twice or thrice | `LinearWithPause` over a strip cut **action first, rest last**, `resume_from_frame: 0` |
  | defeat | `Static`, except Sir Ffuzzy-Logik's `YoYo` |

  The fourth is the refugee bean's existing trick in `theme/genesis/mod.rs`, so it is already
  implemented. What is *not* in the vocabulary is the second layer some characters need — a
  palette-cycled element on its own clock (Arms, Coconuts, Sir Ffuzzy-Logik), an overlay sprite
  (Sir Ffuzzy-Logik's eyes, Humpty's hands), and an emitter (Frankly's sparks, Humpty's bolts).
  A third kind of effect, drawn *into* the frames, needs nothing at all — Skweel's sneeze and
  Dynamight's plunger are both that, and it is worth checking which kind an effect is before
  reaching for machinery.
* **One number is still missing from the whole phase**: how long the defeat poses run before the
  halved frame is held. Five game over captures, the longest 2.32 s, none of them long enough.
  The next one just needs leaving to run.
* **A question the reading raised and did not answer**, for Alex: Davy Sprocket's cap studs
  change from silver to gold between his two winning frames, and his defeat halving is 0.992
  rather than 1.000. If those are the same pixels it is an **unlabelled palette cycle**, which
  the sheet claims only Arms, Coconuts and Sir Ffuzzy-Logik have. Worth ten minutes when his
  clips are processed.

---

## Phase 7 — how it moves

**Status:** `done` — 2026-08-29. Raised by Alex off two screen captures of Mean Bean Machine
(`~/Videos/Screencasts/genesis-puyo-animation.mp4`, the whole 320x224 screen, and
`...-zoom.mp4`, one board), with the brief: work out how the original animates, find the gaps,
close them, and leave room for modern flare. Characters were explicitly out of scope — they
are phase 6, and this phase *frees the box they want* (see above).

Everything under "what the original does" was read off pixels. Both captures are ~24 fps
effective against the Genesis's 60, so the **phases and durations are reliable and exact frame
counts are not**; where a claim was checked numerically it says so.

### What the original does

1. **A pop is a tell and then a strip.** The group flashes on and off about three times over
   ~0.3 s - **starting lit**, and still drawn exactly as it sits on the board, joined to its
   neighbours and all - and only then pulls a face, curls into a ball, the ball shrinks and it
   bursts into droplets. The droplets **outlive the chain step**: the board settles and the
   next step starts blinking while they are still in the air.

   The **face is the long beat**, not an even third of the strip: measured over two separate
   pops it is held ~0.26 s while the two ball frames go by in under a tenth each, and the
   beans are drawn *unlinked* from the moment it appears. Split evenly it reads as a flicker
   on the way to the balls, which is what it did until [`DestroyStyle::holding_first`].

   Frame counts for the two pops measured, at the capture's 23.25 fps: the blue group is lit
   160-162, flashes 163-169, is surprised 170-174, balls 175-176; the red group is lit
   193-195, flashes 196-202, is surprised 203-208, balls 209 on.
2. **Every puyo squashes where it lands**, and the two halves of a pair squash independently
   because they land at different moments.
3. **There is a nuisance tray**, on the wall immediately above the board, anchored at its left
   edge and growing rightwards. An arriving icon slides in from over the middle of the board;
   the tray empties on the frame the drop begins.
4. **An attack crosses the window as a ball**, arcing up and over to just above the top of the
   opponent's board, where it bursts into shards and leaves the tray icon behind. One ball per
   attack, not per chain step.

   It is a **sprite of its own and not a puyo**: a white core inside a coloured rim, on the
   beans sheet below the refugee bean at (624, 98) and (659, 101) in red and (624, 134) and
   (659, 137) in blue - **22x20** for the big one and 16x16 for the small, so the big one is
   1.375 cells across against a bean's one. Its colour is the **sending player's palette**,
   red for player one and blue for player two, which is the same rule the score font follows;
   it is *not* the popped group's colour. In the capture player one sent it and it was red,
   and its white core measured 1.07 cells at its peak, **strobing** three times as it formed
   rather than fading smoothly.
5. **Nuisance falls under gravity**, and raggedly: the beans appear level under the lintel and
   the level row breaks up on the way down.
6. **Nothing shakes.** The stone wall between the two boards was cross-correlated against the
   first frame over all 294 of them, including a two row drop: zero displacement, every frame.
   What reads as a rumble is every refugee bean bouncing at once.
7. The chain score goes in the HUD. **Alex decided to keep our `clear_popup` caption instead**,
   so no HUD readout was built and there is no `hud_readout` trait method.
8. **The tray has three symbols and they are the classic 1 / 6 / 30.** Measured: a run of four
   **small pale** blobs stood for four nuisance, and one then two **black** beans with white
   outlines stood for six then twelve. Both are on the sheet - the small eyeless blob at
   (665, 32), 12x9, and the black bean at (627, 50), 14x12. `NuisanceIcon::{Small, Large,
   Rock}` already decomposed correctly; what was wrong was only which sprite each got.
9. **The droplets are big.** Measured off the capture at a little under half a bean across,
   where they were being drawn at a quarter of one.

### What was built

Five new engine primitives and two retimed ones. Three of the five are *decoration* in the
sense `popup.rs` establishes - the board carries on underneath them - so they stay out of
`blocks_tick()` and change no gameplay timing at all.

| what | where | blocks tick? |
|---|---|---|
| the landing squash | `engine/src/animate/bounce.rs` | no |
| debris thrown off something | `engine/src/animate/debris.rs` | no |
| the attack ball | `engine/src/animate/attack_ball.rs` | no |
| the tray's hold and slide | `engine/src/animate/tray.rs` | no |
| the rumble, opt-in | `engine/src/animate/impact.rs` (`State::Rumble`) | no |
| the pop's blink | `engine/src/animate/destroy.rs` (`DestroyStyle::Pop`'s `blink`) | **yes** |
| nuisance under gravity | `engine/src/animate/nuisance.rs` (`NuisanceFall`) | **yes** |

**The seam for a landing is a new event, `GameEvent::Landed { cells }`, and not a change to
`Settle`.** `Settle` fires once for a whole board and only when a settle *moved* something, so
a pair landing flat on the stack produces none at all and a half resting on a ledge comes to
rest a lock earlier than its partner. An event is data on the wire rather than a trait method,
so it needs **no `AnyGame` delegation and no pinning test** - which is the decisive advantage
over a `GameRender` method here. `Board::settle()` went from `-> bool` to returning the points
that moved, read after `recompute_links()` so a bounce never draws a stale link mask.

**Debris is measured in board cells, not pixels**, and is unbounded - a droplet leaves its
cell, and often the board. That is exactly why it is drawn on the window after the foreground
particles rather than into the board texture, which would clip it at the board's edge. It is
fired from `PlayerAnimations::update` itself, off the cells that cross the burst frame of the
destroy strip, so **the droplets outlive the destroy state** with no match-screen wiring.
`DebrisArt::Cell` always resolves, so a burst needs no art at all - the trick `PendingLayout`
already uses to draw a tray from a theme's own cells.

**The attack ball is the one thing here that belongs to no player**, so it lives on
`ThemeContext` and is drawn unclipped: every offset a player owns is applied inside that
player's own panel. A flight is held in cells and player numbers, never pixels, and both ends
are resolved through whichever theme each player is on at draw time - so a theme change
mid-flight moves the endpoints instead of leaving the ball flying to where a board used to be.
`ThemeContext::animate_destroy` remembers the centre and modal cell of each clear, because an
attack is routed *after* the chain that earned it has ended, by which time the group that paid
for it is off the board.

**The tray had to be held back.** An attack is routed the moment the chain ends and the
receiving game trays it there and then, so without `animate/tray.rs` the icons appear a third
of a second before the ball carrying them lands. `match_screen` snapshots every tray's depth
*before* the update loop, and the new icons are simply not drawn until the ball arrives.

### The numbers, and where they came from

* **The genesis tray is at `(BOARD.0 + 2, 4)`, half a cell, stepping right.** Measured off the
  emulated game, not off any sheet - the tray is drawn in sprites and the frame plane carries
  none of it. In the full-screen capture the right well is at source x 208..304 and its one
  icon spans x ~210..219 and y ~4..12: **8x8 source pixels**, half a cell, inset two from the
  well's left edge. That band is `TOP_PADDING`, and a point in the padded background is a
  point on the Genesis screen, so the move needed no new plumbing.
* **The landing squash is the `y = 70` pair of each colour band**, not the top strip. The plan
  this phase was approved from said the bounce was five frames of the `y = 32` strip; on
  re-reading the sheet that is wrong - those middle frames are the faces a bean pulls on its
  way out and belong to the pop. Every colour band carries, on a row of its own and used for
  nothing else, a **flat** bean and a **tall** one. That is squash and stretch, and it is
  what `rip_retro.py`'s `genesis_bounce` rows now cut. The refugee bean has a flat of its own
  (the same art its blink shuts its eyes with) and no tall, so it settles straight back.
* **`POP_DELAY` went from 280 ms to 90 ms** and each theme's animation now carries the beat.
  Alex chose "genesis plays slower": a genesis chain step is `300 (blink) + 380 (strip) +
  90 = ~770 ms`, the measured figure, where `snes` and the particle theme are `200 + 90 =
  290 ms`. Three doc comments in the Puyo themes claimed a pop animation under `POP_DELAY`
  "costs nothing"; **it does not**, and they now say so. `match_screen.rs` skips `game.update`
  outright while an animation blocks the tick, so a strip and the delay **add**.
* **`GENESIS_POP_FRAMES` went from 5 to 3.** The two frames that went drew four droplets
  *inside* the cell at two spreads, which is as far as a sprite can throw anything; they are
  thrown as debris now and leave the cell, the board and the panel behind them.
* **The nuisance fall is `initial_speed 7.0, acceleration 26.0, max_speed 26.0,
  column_jitter 60 ms`.** The longest drop - a rock into an empty well - takes ~0.79 s, close
  to the 0.86 s the constant-speed fall took, so this costs the tick nothing. The per-column
  stagger is a **golden-ratio hash of the column index**, not an RNG: neighbouring columns get
  very different offsets so the row visibly breaks up rather than tilting, the same board
  falls the same way twice, and there is no randomness on the render path.
* **The rumble is opt-in and the particle theme is the only taker**, at a twelfth of a block
  over 280 ms. `animate/impact.rs`'s module doc now records that the original has none,
  measured, so nobody adds one to a retro theme again.

### What this cost everything else

* **Dr. Rustario and Rustris: nothing.** Neither emits `Landed`, neither returns `Some` from
  `attack_fall`, neither declares a bounce strip, debris or a rumble. Both could adopt the
  landing squash later for free - it is one strip on a `CellAnimationData`.
* **`AnyGame` delegation**, the trap CLAUDE.md names: one arm was touched, and only as a
  rename - `attack_fall_speed() -> Option<f64>` became `attack_fall() -> Option<NuisanceFall>`,
  with its pinning test renamed alongside it. **No new defaulted trait method was introduced
  anywhere in this phase.**
* `AnimationSpriteSheet`'s texture went into a `RefCell` so a frame can be drawn faded, which
  is the same trick the block atlas and the popup font's fill already use and for the same
  reason: `set_alpha_mod` is a mutation behind a `&self` draw. The faded draw puts the alpha
  back afterwards, since every other draw takes the sheet as it finds it.
* `CellAnimationData` is `Default` now, so a theme names only the strips it has art for and
  gains a new one without being touched. That is what made adding `bounce` and `debris` a
  one-line change per theme rather than eleven.

### Seeing it move

`frame_shot` renders one frame and replays only a clear's caption, which says nothing about
how anything moves - the third time that has bitten, so this phase added
**[`launcher/examples/animation_shot.rs`](../launcher/examples/animation_shot.rs)**:

```shell
cargo run -p dr-rustario-vs-rustris --example animation_shot -- 1920 1080 out/ genesis 50 80
```

It drives a scripted two-player Puyo match - stack, chain, send, take the drop - and writes one
PNG every N ms, reimplementing the small part of `match_screen` that matters: drain the events,
fan them out, and **skip the game's update while an animation holds the tick**. Run it with
`SDL_VIDEODRIVER=dummy SDL_RENDER_DRIVER=software`. Phase 6 will want it for the characters.

### The second pass, 2026-08-29

Alex watched the capture again against the build and named four things. All four were real;
the measurements above were amended rather than appended to.

* **The surprised face is a held beat, not a frame.** The *order* was right - flash, then
  face, then balls - but split evenly over three frames the face lasted 127 ms against the
  original's 260. `DestroyStyle` gained `holding_first`, which gives the strip's first frame
  a slot of its own and shares what is left among the rest; asking for less than an even
  share changes nothing, so every other theme is untouched. And the blink now starts
  **lit**: it began dark, so a clear opened by taking the group away, which reads as a cell
  deleted rather than as a warning.
* **The droplets were a quarter of a cell and should be about half.** The cut droplet is
  small and centred in a whole cell, so `BurstSpec::size` is the *cell's* size and the piece
  inside it comes out at half that - 0.5 gave a quarter-cell droplet. It is 0.9 now, and the
  attack ball's arrival burst matches it.
* **The attack ball is its own sprite.** It was being drawn as the popped puyo at one cell
  with a white square core. It is now the game's own art (`theme/genesis/attack.png`, cut by
  `rip_retro.py`'s `genesis_attack_balls`): four frames, player-major and big-first, drawn at
  1.375 blocks, picked by the **sending player** and by whether the attack is a whole row or
  more. `AttackBallData` is optional on both theme builders and a theme without it falls back
  to the popped cell exactly as before - which is what the particle theme does.
* **The tray's three symbols were mapped to the wrong art.** Small was right; Large was the
  plain board refugee where the game uses the black one, and Rock had the black one. Small
  and Large are now measured off the game. **Rock is a placeholder**: Alex has seen a red
  symbol for it, and there is no red refugee anywhere on the beans rip and none in the
  capture - the only red thing in the tray band is the attack ball bursting. It borrows the
  white-outlined bean, which is at least distinct and heavier-looking. `GENESIS_TRAY` in
  `rip_retro.py` is the one line to change when a rip of the red one turns up.

### Handover notes

* **The plan's frame order for the landing squash did not survive contact with the sheet.**
  It is the `y = 70` pair, not the top strip; see above. Nothing else in the approved plan
  changed materially.
* **`docs/puyo-puyo-plan.md` was being edited by the phase 6 agent while this landed.** Only
  two things here touch their text: this section, and the resolution note in *The one conflict
  on `genesis`*. Their `puyo-rusto/art/mugshots.py` was not touched.
* **What was not done, and was never in scope.** The `y = 51` strips (harder squashes, dizzy
  faces) are still uncut; the attack ball fires for any pair of players, including Rustris and
  Dr. Rustario in a vs. match, drawn with whatever cell their clear was - which was the
  recommendation but has not been *watched*; and the particle theme has no bespoke ball art,
  drawing its own cell sprite like everything else.
* **Worth a look with human eyes**: whether a genesis chain step at ~770 ms reads as faithful
  or as slow when several steps run back to back, and whether the droplets thrown from an edge
  column stray too far onto the panel's stonework. Both are one constant each - `POP_HOLD` and
  `POP_DEBRIS.speed` in `puyo-rusto/src/theme/genesis/mod.rs`.

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
* `cargo run --example animation_shot -- 1920 1080 out/ genesis 50 80` — a scripted match
  stepped a frame at a time, one PNG every 50 ms, which is how anything that *moves* is
  checked without one. See phase 7.
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
