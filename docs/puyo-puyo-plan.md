# Puyo Rusto — implementation plan

The third game of the compendium, picked in [next-game-ideas.md](next-game-ideas.md). This is
the shared memory for that work: the phase board, the decisions that still bind, and the things
a future agent would otherwise have to rediscover. **It is not a log.** Anything recoverable
from the code, the tests or the `art/*.py` doc comments has been taken out — read those for the
detail and this for the *why*.

## Status

| phase | what | status |
|---|---|---|
| 0 | engine and launcher past two games | `done` |
| 1 | rules, headless | `done` |
| 2 | playable on the particle theme | `done` |
| 3 | retro themes (`genesis`, `snes`) | `done`, **except 3d: `snes` has no audio of its own** |
| 4 | the ai | step 1 (`beam.rs`) `done`; step 2 (a neural model) `todo` |
| 5 | **vs. integration and attack pricing** | **`todo` — the one phase with real work left** |
| 6 | the characters | `done` — the `genesis` cast and the `snes` Kirby |
| 7 | how it moves | `done` |

**Phase 5 is the next job.** Everything else open is a small, named debt listed under its own
phase.

## Decisions that still bind

* **Faithful Tsu between two Puyo players** — the real chain power, colour and group bonus
  tables, target points, the nuisance queue and classic offset. Cross-game attacks are tuned
  *down* and are measured, not guessed (phase 5).
* **`GameId(3)`, declared as `engine::game::ids::PUYO`.** Game ids live in the engine because a
  game pricing an attack has to name the game it crosses to, and the game crates are siblings.
* **The exact tables are Puyo Nexus's.** [puyo-nexus-rules.md](puyo-nexus-rules.md) is a local
  copy of every page carrying a rule — search it first, and search for the *mechanic* rather
  than the page you expect it on (phase 1 got the ghost-puyo row wrong because that rule is
  filed under *Gameplay Guides*). The live wiki is the authority and rejects automated fetches,
  so read it in a browser when the copy looks thin. **Do not guess a table.** Cite the live page
  in code comments, not the copy.
* **Connected puyos are a `CellId` encoding, not an engine feature.** A `CellId` carries colour,
  a four-bit mask of which orthogonal neighbours match, and a `PuyoSkin`. `board.rs` recomputes
  masks after every lock, pop and settle. The falling pair and every ghost draw unlinked;
  nuisance never links.
* **No hold; hard drop stays.** Tsu has neither. `hold()` is a no-op and a Puyo board shows no
  hold box; hard drop stays because the engine's input model, every pad mapping and the ghost
  piece are built on it.
* **The colour count is fixed for a whole match** and is *not* driven by `speed_index`. Stages
  advance per player while the colour stream is dealt from one shared seed to independent
  randomisers, so anything that changes what is *dealt* mid-match desynchronises two players who
  reach the change at different moments. The general rule for the games after this one:
  **`speed_index` may change how a game feels, never what it deals.**
* **Cross-game garbage arriving at Puyo joins the nuisance queue** — visible, offsettable,
  dropping when the chain finishes — rather than applying immediately the way the other two take
  a hit. Offset is the identity mechanic and it would be strange for it to work against one
  opponent and not another. It makes a tetris less frightening than its raw number suggests,
  which phase 5's pricing has to account for.
* **The speed ramp stays** (Alex, 2026-08-27). Tsu has no level that climbs with play, so
  `PUYOS_PER_STAGE = 30` and the twelve-step fall curve are an invented house rule — kept in
  single player and versus alike, because the whole compendium's mode structure is built on
  stages and a third game that opted out would cost the level sprint, the stage clear card and
  the speed band scenes their meaning here. Margin time lands in phase 5 *on top of* the ramp,
  not instead of it. **Nothing revisits this.**
* **Themes are `genesis`, `snes`, `particle`, oldest first**, which is the theme sprint's order
  and the retro playlist's. Every theme is named for its platform, as everywhere in this repo.
* **`pair.rs` is a sibling of `pill.rs`, not an extraction from it.** The two pieces rhyme but
  the kick tables, the quick turn and what happens to the halves all differ; a shared engine
  pair-piece would be all parameters and no substance.

## The art, and the rule about it

**No rip is in the repository and none will be.** They sit under their *verbatim* download names
in `puyo-rusto/art/` and `art/retro/`, each named by full path in the root `.gitignore` with a
comment saying which script reads it. Nobody renames a source file to something tidier — the name
is the provenance. Alex downloads them; the agent writes the cutter.

| script | what it cuts |
|---|---|
| `art/rip.py` | the particle theme's puyos (`check` writes an alignment board) |
| `art/rip_retro.py` | `genesis` and `snes` sheets, panels, fonts, vignettes (`check` likewise) |
| `art/retro_audio.py`, `music.py`, `sfx.py` | music and effects |
| `art/mugshots.py`, `kirby.py` | the characters; both print the Rust table to paste back |
| `art/sprites.py` | the procedural puyos the rip replaced, kept as a description of what the sheet must contain |

Each script's doc comment carries its own archaeology — how the link variants are *found* in a
sheet that has no grid, how a loop point is measured rather than read, how the SNES layer switch
(`$212C` in a savestate) renders a background layer on its own. Re-run a script rather than
editing its output.

**Retro geometry was measured against the emulated game, not read off the rip**, and the rip was
wrong every time the two disagreed. The numbers live in each theme module beside a comment saying
what they were measured from. Keep doing it that way.

**RetroArch, if you drive it:** `--set video_vsync=false` or the session hangs a few seconds in
with no error; `--set savestate_file_compression=false` or a state is `#RZIP` rather than a plain
file. `video_driver=sdl2` does not start in this build. Both cores publish no memory map. Sessions
still die after a while on this machine — do not plan a long emulator drive.

---

## Phase 0 — engine and launcher past two games — `done`

What it left behind, all of it still load-bearing:

* **`ForeignPrices`** on `Attack` — a `[u32; 8]` keyed by `GameId`, `Copy`, **defaulting to
  zero**, with `with_foreign_for(receiver, price)` to author and `strength_for(receiver)` to
  read. An unpriced crossing is worth nothing and `Match::send_attack` drops it, rather than
  sending the wrong units silently. `GAMES` is 8: raise it rather than renumbering games.
* **`MetricKind::Chain`** and **`words::CHAIN`**. Words are outlined ahead of time by
  `ParticleRender::build_captions`, so a word that was never outlined is silently dropped.
* **`Game::pending_attacks() -> Vec<CellId>`** — the attack-queue strip. The game reports what is
  queued as its *own* `CellId`s and the theme draws them with `BlockSpriteSheet::draw_cell`, so
  **the strip costs a theme no new art**. A retro theme authors a `PendingLayout`; a particle
  theme sets `pending_max`.
* **`PerGame<T>`** and `GameKind`'s three lists: `ALL` (the order games are *numbered*, the key of
  every per-game collection), `RUNNING_ORDER` (the order they are *billed* on the pre-menu) and
  `PLAYLIST_ORDER` (which playlists deal). Three because they are three different things; a test
  holds them to being the same games.
* **`AiBrain`** — `VersusAi::brains()` returns an ai player and one brain per game rather than a
  tuple that grows a dimension per game. A brain handed a board that is not its game does
  nothing. Each game's brain plays at *its own* declared key delay.
* **`modes.rs::game_mode(game)`** is the single place naming each game's standalone mode, so a
  game cannot be added to the shell and forgotten in the tests.

## Phase 1 — rules, headless — `done`

`game/` is `mod.rs`, `board.rs`, `pair.rs`, `nuisance.rs`, `score.rs`, `random.rs`, `rules.rs`,
`cell.rs`. Every module's doc comment names the Puyo Nexus page it came from, and
`a_three_chain_scores_and_sends_what_the_published_table_says` is the check that makes "faithful"
checkable rather than asserted.

Things that are easy to get subtly wrong, and are therefore worth stating here as well:

* **Top-out is the death square**, not a blocked spawn: the game is lost when a puyo comes to rest
  on the spawn point.
* **The ghost row does not pop.** `Board::grouping_color` reports a thirteenth-row cell as having
  no colour, so a group of four with one member up there does not pop *at all* — the other reading
  would fire the chain immediately and there would be no technique to speak of. Tsu's ceiling
  falls out of there being no fourteenth row, and a rotation whose pivot is in a ghost row is
  **refused outright** rather than kicked (*current row check*).
* **The quick turn is a swap, not a search.** The pair keeps the same two squares and the halves
  exchange places, which is why the rotation cannot fail.
* **Chain power is the multiplayer table, in one player as well as two.** One table is one
  behaviour to test. A solo marathon therefore scores lower than the arcade would have shown;
  swapping the single-player curve back in for one-player modes is small and contained.
* **The all clear pays out on the *next* chain**, not the one that earned it.
* **The event grammar is Dr. Rustario's combo grammar**: one `GameEvent::Clear` per chain *step*,
  `is_combo` false on the first and true after, a `Settle` between steps. That is what the
  particle field, the clear wave and the words are already listening for, so none of them has to
  learn what a chain is. `detail` carries `ClearDetail { chain, all_clear }`.
* **`clear_class` grades 0..3 and 3 is reserved for the biggest clear**, or the particle field's
  silhouette interrupt never fires.
* **`Difficulty` is the game's own five settings** (colours, starting rows of nuisance, a speed
  bonus on the hardest) — *not* the four ai difficulty names, which are a different thing wearing
  the same word.

**Known gaps, all deliberate and all sourced:**

* **Margin time** — specified (96 s, then target points to 3/4 and halving every 16 s, at most 14
  iterations or until they reach 1) and not built. It is the game's own answer to a match that
  will not end, which is exactly what a playlist wants. Phase 5's to take.
* **The soft drop bonus** is out: no page gives the points per cell, and implementing it means
  guessing a number. `GameEvent::SoftDrop` is emitted if it is ever wanted.
* **Soft dropping onto a blocked cell locks immediately** in Tsu; this uses one `LOCK_DELAY`.
* **The nuisance scatter is a documented guess** — Puyo Nexus lists the distribution algorithm as
  an open question on its own reverse-engineering page. Full rows first, then the remainder over
  distinct columns, which honours the sourced parts and guesses only the undocumented one.

## Phase 2 — the particle theme — `done`

The theme is a rip (`art/rip.py`, `music.py`, `sfx.py`) rather than the procedural art it started
as. What is worth carrying forward:

* **The skins.** The sheet carries several usable sets of the same puyos and `PuyoSkin::deal`
  hands each player a different one off the **match seed** — off the seed so a playlist swapping a
  board onto Puyo mid-match returns the puyos that player already had. `PuyoSkin::COUNT` is the
  number and a test holds the sheet to it. A set earns its place only if four in a row read as
  **one mass**, which nothing shows a cell at a time: `rip.py check` draws a whole board per skin
  and is the only way to judge it.
* **The sheet is laid out against a texture limit.** One band per skin in a single column stood it
  past the 4096 `MAX_ATLAS_WIDTH` a GLES driver will allocate, so on a handheld the theme could
  not be built at all. `BANDS_ACROSS` in `rip.py` and `skin_block` in `theme/modern/mod.rs` are
  the only two places that know the layout; a test holds the sheet to both its shape and the
  ceiling.
* **There is no pre-built bank of alpha variants.** It was a whole copy of the atlas per fade step
  — roughly 106 MiB for one skin. The atlas is one texture in a `RefCell` with the fade applied at
  draw time, which is what the popup font already did.
* **The HUD is the score and nothing else** (Alex, 2026-08-27). Tsu's HUD is the score and the
  tray; a chain is a thing that *happens*, so it announces itself over the puyos instead through
  `engine::animate::popup` — `GameRender::clear_popup`, one popup per clear each on its own clock,
  drawn **last of all on the window** after the foreground particles, because drawn into the board
  texture it lands under the clear's own particle burst.
* **A popup is drawn in the colour of what popped**, and `BlockSpriteSheet` works that colour out
  by reading its own built atlas — averaging each cell's sprite weighted by saturation × brightness
  so an outline and the white of a puyo's eyes do not wash it out. The game cannot say (it knows a
  `CellId`, not what a theme paints it) and a theme cannot be asked to declare eighty of them.
* **`PopupSpriteData`** lets a theme spell a caption out of its own art in *tokens* — a digit here
  and a whole word there — falling back to the plain face for anything it cannot spell, whole
  rather than in part. A theme owes this nothing.
* **`ModernThemeOptions::visible_rows` counts the buffer rows in** — `ROWS` (13), not
  `VISIBLE_ROWS` (12). Passing 12 draws a playable row above the frame. Easy to get backwards.
* **`MetricKind::Chain` stays in the engine** even though no HUD draws it: Puzzle Fighter and
  Bombliss both want the same counter.

## Phase 3 — retro themes — `done`, bar the `snes` audio

`genesis` (Dr. Robotnik's Mean Bean Machine) and `snes` (Kirby's Avalanche) — the two western
reskins of one Compile original, so both boards are exactly this game's board and the rips carry
the sixteen link variants already, because the originals drew connected beans the same way.

**A third theme, `3ds` (Puyo Puyo Chronicle), was built and then cut** on 2026-08-28. Two reasons:
it is modern art in a retro slot, and — the one that cost something — **its panel sized the board
for every other theme**, since every theme of a game is drawn at the largest cell all of them can
hold. `git log` has it. `SceneType::Cover` was written for it and is kept: it is a general thing
for a scene that is one painted picture, and the vignettes use it now. If the slot is ever filled
again it wants something 16-bit, and something whose sheet carries the frames a bean needs to pop.

What a future theme here owes, and the traps:

* **A retro theme's background needs a hole in it.** The board frame is drawn *under* the
  background, so a panel carrying its own well covers the board and every cell on it — a perfect
  empty field with the queue, tray and score all correct beside it, which is a very convincing way
  to look broken. Both other games' retro themes cut the same hole and nothing says so anywhere
  but the art. `rip_retro.py` punches it and each theme has a test that the hole and the board
  agree.
* **`board_snips` are into the *padded* board texture** — add `top_padding` to their height or the
  bottom row is left outside the copy.
* **All skin slots key to the same art.** `data::cells` walks `PuyoSkin::all()` and a retro theme
  hands back the same points for every slot, paying only for duplicate keys. So **on a retro theme
  both players draw the same puyos**, because the original drew one set. That is not a bug.
* **The thirteenth row is open and has nothing behind it.** Closing it was tried first, at Alex's
  ask, and reversed on sight: the row above the field is *played in*, and hiding it reads as a bug
  the moment a board fills. Both panels are cut level with the top of their own field and the
  thirteenth row is `top_padding` above panel and board alike. The happy accident is that **a
  point in either padded background is a point on that console's screen**, which every coordinate
  in both themes now relies on.
* **The scenes are vignettes** (Alex's pick of five candidates) — a 96x54 png of the backdrop's
  own colour drawn through `SceneType::Cover`, so it is smooth at 4k for a couple of kilobytes —
  **and the panel casts a shadow on them**, which is what lifts the panel off the scene.
  `PanelShadow` falls down and right only (a light over the top-left shoulder), takes a `margin`
  on all four sides so a trim or a pad does not hang the shadow out in the scene, is composited
  rather than painted into the art (a margin round the art comes straight off the cell size), and
  does **not** move with the hard-drop ricochet — that offsets the board inside a panel that has
  not moved.
* **Panel size is the whole board's size.** Two levers, both in `genesis/mod.rs` and mirrored in
  `rip_retro.py`: `SIDE_TRIM` (the outer rock becomes scene; the png stays 208 wide, so no
  coordinate changes) and `BOTTOM_PADDING` (transparent rows under the background art, bought out
  of the fit like any source pixel). Alex took `(8, 4)` and 8 rows: the gap between two boards
  goes from 16 px to 66 and the panel gains 42 px under it, with the two-player cell unchanged at
  73. Both are one constant each if it ever reads as tight — past eleven rows the height binds and
  the cell comes down with it.
  `the_panel_art_stops_where_the_trim_says_it_does` reads the columns off the png and is the only
  thing that can hold art and constant together.

### Outstanding: phase 3d, the `snes` audio

`genesis` has its own music and effects (`retro_audio.py genesis`, 4.4 MiB, a test decoding all
twenty three). **`snes` still plays the particle theme's sounds and `theme::GAME_MUSIC`** — which
is a deliberate placeholder, not an oversight. To finish it: `retro_audio.py` grows a `snes`
subcommand, the theme writes its own `mod sound` and its own music table the way `genesis/mod.rs`
does, and it needs **four tracks** (`theme::GAME_MUSIC_TRACKS` — every theme's table is the same
length, or a theme's tracks would be heard less often than another's). Effects must be **OGG
Vorbis at exactly 44,100 Hz**; the decoder rejects anything else and a third of any rip is 48 kHz.
Trim padding off both ends. Whether the rip is laid out the way Mean Bean Machine's is is the
first thing to find out, and `split`'s assertion is what will say.

**Also open: seven of `genesis`'s effects are inferred, not heard.** The rip names only the sounds
whoever made it recognised and numbers the rest `sfx_N`; RetroArch would not stay up long enough
to trigger them in-game. Each is one line of `GENESIS_SFX` in `retro_audio.py` and a re-run away
from being corrected:

| slot | taken from | the doubt |
|--|--|--|
| `rotate` | `puyo_sine` | a 16 ms blip; `sfx_11` is the other candidate |
| `lock` / `settle` | `puyo_blob` / `puyo_blob_2` | which way round, and whether either is the settle |
| `garbage` | `bad_puyos` | the plural, read as the shower landing |
| `attack` | `bad_puyo_1` | reading the singulars as sizes of a *send*; there may be no send sound |
| `pause` | `select` | most likely the menu confirm |
| `hard-drop` | `short_noise` | the game has no hard drop, so this is a substitute either way |

Not in doubt: `move`, the four `chain_N` steps and `level_start`. Also wanted: whether `Victory!`
and `Continue` are the right tracks for a win and a burial.

## Phase 4 — the ai — step 1 `done`, step 2 `todo`

`game/ai/` is `field.rs`, `quiet.rs`, `eval.rs`, `placement.rs`, `beam.rs`, `skill.rs`,
`harness.rs`. There is no decomp to port, which is the one way this differs from Dr. Rustario, so
the shape came from the open literature — mostly [ama](https://github.com/citrus610/ama) (MIT),
whose evaluation is fifteen weights in one file where `mayah`'s is about a hundred; plus takapt's
beam-search idea and Ikeda et al.'s *Playing PuyoPuyo*.

* **The quiescence search is the thing.** `quiet.rs` is what separates a bot that plays from one
  that tidies: a building player almost never fires anything, so a placement's own chain says
  nothing about nearly every placement on offer. What matters is the chain the field is *holding*.
* **The search is a state machine, not a function.** A pair takes a second to fall and nothing is
  waiting on the answer until it lands, so `Search::new` scores every root placement and stops,
  and each `step` expands more and hands the frame back. The strongest row costs 0.88 ms a frame
  instead of 10.6 ms in one lump — worth roughly a twelvefold budget, at no cost in strength,
  which is why the ladder was **not** scaled down under `portmaster`. `width` is the dial if a
  device still cannot afford it, and `ga puyo rank` is compiled into the handheld build so
  measuring on the device is one command.
* **Two things follow from thinking slowly**, both handled in `agent.rs`: the pair goes on falling,
  so the keys are recomputed at the end from where it *is*, and if the chosen placement can no
  longer be reached the next one down is taken (which is why `beam::ranking` returns an order, not
  a winner); and the pair may come to rest before the search is done, so there is always an answer
  after `Search::new` and every step only sharpens it.
* **The ghost row is worth two features.** In `field.rs` it is the whole of the `NEIGHBOURS` table
  — the ghost row is nobody's neighbour, so the rule is stated once. In `eval.rs` it is the
  `ghost` weight, counting cells of that row walled off from the spawn column: a puyo resting up
  there is a *door closed*, because a pair moves sideways with one half in it.
* **Read colours through the mask, not around it.** A `CellId` is colour *and* link mask, so a
  feature comparing raw `CellId`s sees sixteen different reds and finds no chains at all.
  `Field::from_board` drops the mask on the way in.

The ladder (`skill.rs`) is **measured, not assumed** — `ga puyo rank [seeds] [pair cap]
[difficulty]` plays every row over the same seeds and prints the `SKILL_ORDER` to paste back, and
`ga puyo play <seed> ...` plays one brain headless. The three rows sharing the `build` weights
differ in *how long they hold a chain* rather than only in how hard they think, which is what came
out of the first ranking run. **The measure is a solo marathon**, where no nuisance ever arrives,
so it ranks what a row builds and not how it takes a hit; ranking the rows against each other is
phase 5's to want.

**Step 2, the neural model, is `todo` and deliberately not provisioned for.** Nothing in
`game/ai/` is shaped around one; adding it means a `PuyoAiKind` variant beside `Scorer`, the way
`DrAiKind` carries both, trained by `ga puyo auto` over the existing `Fitness` seam. It ships only
if it beats the scorer.

---

## Phase 5 — vs. integration and attack pricing — `todo`

**Goal.** Puyo takes its turn in every vs. playlist, and garbage crosses at sane volumes in all
six directions.

**Joining the playlists is one line**: add `GameKind::Puyo` to `PLAYLIST_ORDER` in
`launcher/src/games.rs`. Phase 2 introduced that list precisely so this could be a deliberate
step — a game is billed on the pre-menu as soon as it is playable and deals a playlist turn only
once it has the themes and the ai to take one. Everything that sequences a playlist already reads
it, so nothing else in the launcher changes.

**The six directed prices are the real work.** Today `puyo_rusto::game::foreign_attack` returns
zero for every receiver, so a Puyo attack is *dropped* at the border rather than mispriced. Each
sending game's `foreign_attack(receiver, ...)` gains an arm per receiver and the caller adds a
`with_foreign_for(receiver, price)`; phase 0 made the default zero, so a crossing this phase
forgets is silent but harmless and shows up as nothing arriving.

Measure rather than guess, the way the README's existing table was built: run each game's own ai
on one protocol (five seeds at full speed for fifty minutes of game time, counting what it sent),
then hand-tune *down* so a Puyo chain does not bury a Rustris or Dr. Rustario player. Extend the
README's measured table from three rows to six.

**Price the two directions asymmetrically, because they are not symmetric.** Attacks *into* Puyo
land in the tray and can be answered — a player who chains back cancels them outright, so a number
that looks brutal on paper is often absorbed for free. Attacks *out of* Puyo land on a player with
no offset at all. The same raw measurement means different things in each direction, and tuning
both ends off one table will get one of them wrong. Sanity check by playing each pairing.

Starting intuitions to test, not to ship: a four-chain is roughly the work of a tetris; routine
two-chains are what a Puyo player throws constantly and should cross for little or nothing.

**Margin time is the knob to reach for if matches drag** — phase 1 sourced it and left it out
(96 s, then the 70 target points to 3/4 and halving every 16 s). It makes every chain send more as
a match wears on, which is what an endless playlist needs and what nothing else here provides.

This is also the phase that sets **the Puyo half of `Difficulty`** — the level the 0-10 vs. dial
maps to, as an arm of `Difficulty::level(game)`, plus a speed dial of its own if it wants one the
way `dr_rustario_speed()` is Dr. Rustario's. The arm exists and returns the dial unchanged.

**Done when:** a 2-player vs. match on each playlist has the three games taking turns, garbage
crosses sensibly in all six directions, and the README table carries the measurements.

---

## Phase 6 — the characters — `done`

`genesis` deals every player one of thirteen Mean Bean Machine faces; `snes` stands Kirby in the
arch. Both are dealt off the match seed the way `PuyoSkin` is, and both are reviewed *moving* —
`cargo run --example character_shot` and `kirby_shot` are the harnesses. `art/mugshots.py` and
`art/kirby.py` carry the cast tables, every calibration already measured, and the routines that
read a capture; both print the Rust table to paste back, so a strip's geometry is derived from the
art rather than typed twice. **The per-character reading lives in those scripts and in
`genesis/mugshots.rs` and `snes/kirby.rs`**, whose doc comments say what was measured and how.

What is design rather than data, and would otherwise be re-argued:

* **The box holds the *player's own* face**, not the opponent's. The original drew one box for you
  and one for the opponent; here a panel belongs to one player, which is the same move the two
  `NEXT` boxes already made.
* **Four states, read per player from that player's own board**: `idle`, `winning` (a chain, or a
  won match), `losing` (nuisance waiting, or the stack high), `defeat`. `winning` beats `losing` —
  a player who chains while buried is *answering* the nuisance. `defeat` and victory are terminal.
* **A single pop does not count as `winning`** (Alex, 2026-08-28): `Clear` fires once per chain
  step, so a one-step clear would enter it several times a minute and sends nothing. Two steps or
  more.
* **Three rules stop it flip-flopping**, and all three are needed: a `MIN_DWELL` that holds a state
  whatever happens short of game over; hysteresis on the height trigger (`DANGER_ENTER` /
  `DANGER_LEAVE` with a real gap, or a stack sitting on one threshold strobes as each pair locks);
  and a `LINGER` that outlives the last `Clear`, refreshed rather than restarted by each step.
* **An overlay and a palette cycle are the same thing** — one `Layer`: a small sprite at an anchor
  in box coordinates on a clock of its own, whose variants merely happen to be recolours. So
  **palette cycling stays out of the renderer**. A layer's strip is per *row*, and a row with zero
  frames is a layer not drawn in that state at all; a cycle is cut as **only** the cycled pixels,
  so it is safe over a portrait animating underneath.
* **A cycled element is rarely only the cycled colour** — a light has a halo, and the halo cycles
  with it on the hardware and cannot here. So the portrait frame under a cycle layer has to sit at
  the *same* cycle phase as every other frame the sheet was drawn at. **A layer's strip looks
  correct on its own when it is wrong**: check a cycle in the composite, never in the cut art.
* **The sweat belongs to nobody.** It is one authored drop, one emitter, appended to every
  character, gated twice — the `losing` row says whether they are worried at all and
  `stack_danger` says how much (threshold 0.55, above `DANGER_ENTER`, because the sweat comes on
  *later* than the losing face).
* **The danger flash is deliberately not implemented** (Alex, 2026-08-29). Every character goes
  white in bursts near death, but it is the whole *sprite plane* brightening — character and puyos
  together, while the wall and the labels do not move — so it is a palette flash, not an animation.
  Recorded here so nobody adds it back thinking it was missed, and so the next agent does not spend
  a capture explaining a character who suddenly turns white.
* **`FrameAnimationType::YoYo` is the wrong shape and nothing here uses it.** It runs `0..n` and
  back, repeating each end (`0 1 2 2 1 0`), where every ping-pong measured off the game holds each
  end once. A ping-pong is cut unrolled and played as a plain `Linear`. Two rate formulas went with
  it — a `YoYo` pass is `2n` steps, and **`LinearWithPause` gives its last frame a whole frame of
  its own *and then* the pause**, so a pass is `n/fps + pause`.
* **`LinearWithPause` holds the *last* frame**, so a row is cut **action first, rest last**. The
  sheet's order is not the play order; diff the row, then check the capture for which pose has the
  long dwell.
* **Emitters are drawn on the window**, not into the panel, since their particles leave the box —
  `ThemeContext::draw_character_particles`, anchored on the panel rather than the board. Speeds are
  declared in **box pixels an axis per 60 Hz frame**, the unit every capture was measured in, and
  converted in exactly one place.
* **Mirroring reaches three things**, not one: the portrait, a layer's anchor and a particle's box
  x. Nothing else in the compendium flips a sprite.
* **A face's texture is built when it is *dealt***, out of a `RefCell<HashMap<_, _>>` behind the
  `&self` draw, so most of the cast is never turned into a texture. One png per character rather
  than one sheet for the cast is what makes that worth anything.
* **The deal lives on `PlayerAnimations`**, not on the theme: `draw_board` is `&self` on a
  `&'static` theme. Same constraint that put `PuyoSkin` on the `CellId`, arrived at from the other
  end.

**Open, and small:** how long a defeat pose holds before the halved last frame is shown is still
unmeasured — bounded at ≥3.96 s and off the critical path, since the halved frame is not drawn at
all. One game over capture left running would close it. The sweat's *rate* is eyeballed; its
threshold and velocity are measured.

## Phase 7 — how it moves — `done`

Read off two captures of Mean Bean Machine, pixel by pixel. Five new engine primitives and two
retimed ones; three of the five are *decoration* in the sense `popup.rs` establishes — the board
carries on underneath them, so they stay out of `blocks_tick()` and change no gameplay timing.

| what | where | blocks tick? |
|---|---|---|
| the landing squash | `animate/bounce.rs` | no |
| debris thrown off something | `animate/debris.rs` | no |
| the attack ball | `animate/attack_ball.rs` | no |
| the tray's hold and slide | `animate/tray.rs` | no |
| the rumble, opt-in | `animate/impact.rs` | no |
| the pop's blink | `animate/destroy.rs` | **yes** |
| nuisance under gravity | `animate/nuisance.rs` | **yes** |

* **A landing is a new event, `GameEvent::Landed { cells }`, not a change to `Settle`.** `Settle`
  fires once for a whole board and only when something *moved*, so a pair landing flat produces
  none at all. An event is data on the wire, so it needs no `AnyGame` delegation and no pinning
  test — the decisive advantage over a `GameRender` method, and the pattern CLAUDE.md asks for.
* **Debris is measured in board cells and is unbounded** — a droplet leaves its cell and often the
  board, which is why it is drawn on the window rather than into the board texture. It is fired
  from `PlayerAnimations::update` off the cells crossing the burst frame, so droplets outlive the
  destroy state with no match-screen wiring.
* **The attack ball belongs to no player**, so it lives on `ThemeContext` and is drawn unclipped. A
  flight is held in cells and player numbers, never pixels, and both ends resolve through whichever
  theme each player is on *at draw time* — so a theme change mid-flight moves the endpoints rather
  than leaving the ball flying to where a board used to be. Its colour is the **sending** player's.
* **The tray has to be held back.** An attack is routed the moment the chain ends and the receiving
  game trays it there and then, so without `animate/tray.rs` the icons appear a third of a second
  before the ball carrying them lands. `match_screen` snapshots each tray's depth before the update
  loop and does not draw the new icons until the ball arrives.
* **A pop is a tell and then a strip** — the group flashes about three times over ~0.3 s **starting
  lit** (starting dark reads as a cell deleted rather than a warning), then a held surprised face,
  then the balls. The face is the long beat, not an even third, which is what `holding_first` on
  `DestroyStyle` is for.
* **An animation under `POP_DELAY` does not cost nothing.** `match_screen` skips `game.update`
  outright while an animation blocks the tick, so a strip and the delay **add**. `POP_DELAY` is
  90 ms and each theme carries its own beat: `genesis` is ~770 ms a chain step (the measured
  figure, Alex's choice of "genesis plays slower"), `snes` and the particle theme 290 ms.
* **Nothing shakes.** Cross-correlated over 294 frames including a two-row drop: zero displacement,
  every frame. What reads as a rumble is every refugee bean bouncing at once. The rumble is opt-in
  and the particle theme is the only taker; `impact.rs`'s module doc records the measurement so
  nobody adds one to a retro theme again.
* **The per-column stagger in the nuisance fall is a golden-ratio hash of the column index**, not
  an RNG: neighbours get very different offsets so the row visibly breaks up rather than tilting,
  the same board falls the same way twice, and there is no randomness on the render path.
* **The tray anchors at the well's right edge and fills leftwards** on `genesis`, which is the one
  thing on that theme the original does not do — it is drawn in `TOP_PADDING`, the band a pair
  spawns in, and the pair is drawn *over* it, so a left-anchored strip sat behind every spawn.
  `the_whole_tray_stands_clear_of_the_spawn_column` pins it, since it reads as a tidy row either
  way round. The particle theme's tray went down the left of the board for the same reason.
* **An icon is drawn at three quarters of a cell**, on a pitch of the same number. The symbols are
  cut as whole cells and the art inside runs 12-16 px across, so drawing the cell at a half-cell
  pitch put a 12 px blob out at 6. Four rocks is 120 nuisance against a field that holds 72, so
  capacity was never what a tray ran out of.
* **The ball flies to the tray**, aiming at `PendingLayout::origin`, because `draw_pending` slides
  every arriving icon out of exactly that point.
  `an_arriving_icon_starts_where_the_ball_burst` pins the two, which are worked out in different
  files and would drift apart silently.

**Worth a look with human eyes:** whether a `genesis` chain step at ~770 ms reads as faithful or as
slow with several steps back to back, and whether droplets thrown from an edge column stray onto
the panel's stonework. One constant each — `POP_HOLD` and `POP_DEBRIS.speed` in
`genesis/mod.rs`. Two things were never in scope: the `y = 51` strips (harder squashes, dizzy
faces) are uncut, and the attack ball fires for any pair of players including a Rustris /
Dr. Rustario match, which was the recommendation but has not been *watched*.

---

## Verification

* `cargo test --workspace`. `every_mode_offers_the_same_ai_opponents_and_demos` and
  `ai_difficulties_agree` in `launcher/src/modes.rs` gate the menu surface.
* `cargo run --example frame_shot -- 640 480 1 out/ puyo` — one frame on every theme, which is how
  theme geometry is checked without a display.
* `cargo run --example animation_shot -- 1920 1080 out/ genesis 50 80` — a scripted match stepped a
  frame at a time, one PNG every 50 ms, which is how anything that *moves* is checked. Run it with
  `SDL_VIDEODRIVER=dummy SDL_RENDER_DRIVER=software`.
* `cargo run --example menu_shot -- 960 720 out/`; `field_preview sheet`; `character_shot` and
  `kirby_shot` for the cast.
* `ga puyo rank` / `ga puyo play` for ai strength, and the five-seed protocol for the attack prices.
* Finally, play it. A 2-player match on each playlist.

## Working agreement

Work on this repository is **synchronous. One agent at a time. Never in parallel.**

* **This document is the shared memory.** Conversations do not carry over between agents; this file
  does. Read it before starting.
* **Phase status lives here**, updated in the same commit as the work it describes, so the document
  and the code never disagree.
* **One phase at a time, in order.** If blocked, mark it `blocked` with the reason and stop — do
  not route around it and do not start a later one instead. Surface it to Alex.
* **Every phase records what a reader could not recover from the code**: decisions the plan did not
  anticipate, measured numbers with no home in a module, and anything that cost time. Not a diary —
  if the code, a test or a script's doc comment already says it, leave it there and say nothing.
* **Amend, do not append contradictions.** A document that argues with itself is worse than no
  document.
* **Stay inside this game.** While these phases are open, nobody starts another game from
  [next-game-ideas.md](next-game-ideas.md) — see the status board there.
