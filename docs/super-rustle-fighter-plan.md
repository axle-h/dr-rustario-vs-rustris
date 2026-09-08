# Super Rustle Fighter II Turbo — implementation plan

The fourth game: Capcom's *Super Puzzle Fighter II Turbo*, as the PlayStation port plays it.

**Status: phases 1, 2 and 4 done — the game is playable.** It has a menu entry of its own, the
arcade theme, its music and effects, and a fighter row that is its character select. **Phase 3,
the ai, is what is left**, and phase 5 with it: a playlist turn and the six new crossings both
wait on it, since `ga cross` prices a game by playing it. Phase 4 was taken before phase 3
deliberately - the plan's own risk note says to play the game before building an ai against it,
and that could not be done until there was something to play.

This document is the *how*; the *what* is
[super-puzzle-fighter-rules.md](super-puzzle-fighter-rules.md), read out of the game's own
executable, and [next-game-ideas.md](next-game-ideas.md) is the status board.

| | |
|---|---|
| Name | **Super Rustle Fighter II Turbo** (Puzzle → Rustle, the house trick) |
| Crate | `rustle-fighter/` |
| `GameId` | a new id in `engine::game::ids` |
| Ruleset | the PlayStation port, `SLUS_004.18` — see the rules doc's *Provenance* |

## The one thing to read first

**The rules document is the source of truth, and it is now complete enough to build from.** Both
items that previously blocked implementation have been closed by reading the executable:

* **The break** — crash gem propagation, the chain loop, counter gem collateral and the rainbow
  gem — is fully mapped. See *The break* in the rules doc.
* **The score** — all seven accumulators are identified, including `+0x112`, which turned out to be
  a bonus for destroying gems that arrived as garbage rather than the power gem premium it was
  assumed to be.

**One question survives and it is a playtest, not a blocker.** Nothing in this code pays a power
gem more than the same gems loose and linked: forming one sets a membership bit and stamps a
parallel array, but the class nibble stays the plain colour, so every cell scores 100 either way —
and the colour flood walks a solid rectangle exactly as it walks a linked blob. StrategyWiki
reports 1410 / 2010 / 3210 points for three arrangements of the same twelve gems; this code
produces 1410 for all three. Build it as read, then judge it in play. If power gems feel valueless
the missing term is real and worth hunting; if they feel right, the guide was wrong. That is
cheaper than more reverse engineering and it is the kind of question a playtest answers better.

The remaining `[open]` entries in the rules doc — the power gem growth loop line by line, gravity
ordering, game over, the attack-animation consumer — are all things phase 2 will surface naturally
with the decompilation open. None of them gates starting.

## Scope

**In:** the versus game. Six-wide, thirteen-tall board, four gem colours, crash gems, power gems,
counter gems with their countdown, the rainbow gem, the per-character drop patterns, the measured
damage and defence formulas, an ai, **one theme — the arcade one** — and a place in the vs.
playlist.

**No particle theme, for now**, decided 2026-09-07. Worth recording what that defers rather than
settles: `next-game-ideas.md` carries a standing note that a new game should be complete and
playable on its particle theme alone, because the particle theme is original art and the retro
themes are not. Shipping arcade-only means this game has no such fallback until a particle theme is
written. That is a sequencing choice, not a reversal — the arcade look is the point of the exercise
and a second theme can follow.

**A seven-character roster, with the full fighter animation layer**, decided 2026-09-07:
Chun-Li, Felicia, Hsien-Ko, Ken, Morrigan, Ryu and Sakura — the seven we have sprite sheets for.
Akuma, Dan, Devilot and Donovan are cut; their drop patterns stay recorded in the rules doc in
case that is ever revisited. **A character select screen is in.** The arcade look and sound is the
target, not a reduced version of it: every labelled animation state on the sheets gets built.

**Out, and deliberately:** Street Puzzle mode, the intermissions and endings, the versus fighter's
super-combo finish, and the arcade's attract mode. Two-player-focused, like the rest of the
compendium.

## Engine work, before the crate

One change certainly lands in `engine/` and one may; the third is not an engine change at all but
belongs here because it touches every other game. Anything that does land goes in its own commit
with the existing test suites green either side.

### 1. Move Puyo's rotation up

Decided 2026-09-07 and recorded in the rules doc's *Deliberate deviations*: Super Rustle Fighter
uses **Puyo Puyo's rotation**, not the PlayStation game's own, which has no wall kick, no quick
turn, and no escape at all from being wedged between two columns.

`puyo-rusto/src/game/pair.rs` moves to `engine/`. Game crates are siblings that never depend on
each other, so nothing in `rustle-fighter/` may reach into `puyo-rusto/`.

**This is a relocation, not a merge.** Dr. Rustario's `pill.rs` stays exactly where it is — the
compendium keeps two rotation systems because they are genuinely different rule sets, and
unifying them behind one abstraction would fight all three. Puyo Rusto's behaviour must not shift
by a frame; that is what the commit has to prove.

### 2. Multi-cell power gems in the sprite pipeline

A power gem is a rectangle spanning several cells, and `CellSpriteData` is one snip per cell.
**Puyo Rusto already solved this** — `puyo-rusto/src/theme/data.rs:43` builds a `LinkMask` from
neighbour bits and registers a distinct snip per colour × link combination, so a joined blob is
drawn as per-cell snips chosen by which edges are interior.

A power gem is the same trick with a rectangle constraint instead of a flood fill, and the game
already computes exactly the mask needed: the power gem array carries corner codes `1`/`2`/`4`/`8`
per cell plus a shared gem id. Feed those into a `CellId` the way Puyo feeds its `LinkMask` and
nothing in the renderer needs to change.

Whether the mask type is worth hoisting into `engine/` alongside the rotation, or whether the two
games just each have one, is a judgement to make when the second use exists — not before.

### 3. The crossings go from six prices to twelve

Attacks cross games through `foreign_attack` and `ForeignPrices`, keyed on `engine::game::ids`.
Three games is six directed prices; **four games is twelve**, so six new ones, all involving
Rustle Fighter:

| new price | |
|---|---|
| Rustle Fighter → Dr. Rustario | Rustle Fighter → Rustris |
| Rustle Fighter → Puyo Rusto | Dr. Rustario → Rustle Fighter |
| Rustris → Rustle Fighter | Puyo Rusto → Rustle Fighter |

**They are measured, not guessed.** `ga cross` (`launcher/src/cross.rs`) plays each game's own ai
alone and reads every crossing as a share of what the receiving game's own opponents throw; extend
it to four games and re-run the whole table, not just the new rows. An unpriced pair is worth
nothing and drops silently, which is what `every_crossing_between_two_games_is_priced` in
`launcher/src/games.rs` exists to catch.

Note for whoever prices these: Rustle Fighter is the only game in the compendium whose attack
**cancels against the opponent's** before it lands, and whose damage grows with elapsed round
time. Both distort a naive reading of "what it throws per minute", so read `cross.rs`'s doc
comment before trusting the numbers.

## The crate

Mirror the existing three. `rustle-fighter/src/game/` for rules, `src/theme/` for the theme,
`src/game/ai/` for the ai.

| module | what |
|---|---|
| `game/board.rs` | 6×13 with one row of headroom, the cell array plus the parallel power gem array |
| `game/cell.rs` | the cell encoding — colour 1–4, crash bit, counter gem countdown and colour |
| `game/pair.rs` | thin: the piece, deferring rotation to the engine's |
| `game/gems.rs` | crash resolution, chain, and **power gem formation and merging** |
| `game/counter.rs` | the counter gem tray, countdown, and the per-character drop patterns |
| `game/score.rs` | the seven accumulators, the damage formula, the defence conversion |
| `game/random.rs` | the four 64-entry distribution tables, the rainbow schedule, the drought rule |
| `game/ai/` | board features, placement search, agent |

**Data the crate needs, already extracted** and sitting in the rules doc: the drop pattern table
(11 × 12 × 6), the eight column orderings, the chain bonus table, the level table, the height/time
tier thresholds, the gem distribution tables, and the rainbow schedule. Transcribe them from the
doc rather than re-deriving them.

**Two compendium rules this game must respect.** `speed_index` may change how the game *feels*,
never what it *deals* — so the gem distribution table and the drop pattern are fixed for a whole
match, exactly as Puyo Rusto's colour count is. And every match runs through `AnyGame`, where a
defaulted trait method that is not delegated is silently never called; prefer a new `GameEvent`
over a new trait method, and trust the tests in `launcher/src/games.rs` to catch the rest.

## Phases

Each phase ends somewhere playable or measurable. Do not start the next until the current one is.

**1 — engine changes. Done.** `engine/src/game/pair.rs` holds `PairMotion` and a `PairBoard`
trait with two questions - is this cell free, and is this row the ceiling - so a game supplies
its board and keeps its own piece type. Puyo Rusto's `Pair` is now the colours and the lock
around one; its 253 tests pass unchanged, which is the proof the move asked for. The ceiling
became a trait method rather than a hard-coded ghost row: Puyo answers yes for its thirteenth,
and a board with no such row says nothing and gets the ordinary kick.

**2 — headless rules. Done.** `rustle-fighter/src/game/` — `board`, `cell`, `pair`, `gems`,
`counter`, `score`, `random`, and `tables`, which is the extracted data on its own because it is
evidence. All four numbers are pinned by tests: a 4-gem crash sends 4, twelve loose gems score
1410, the first All Clear sends 6 (and the second 12), a 24-gem defended break cancels fully.

Three things it turned up, all recorded in the rules doc:

* **The tables are extracted, not paraphrased.** The plan said they were "already sitting in the
  rules doc" and they were not — the doc gives their addresses and describes their shape. They
  are now read out of `SLUS_004.18` directly into `tables.rs`, and every published description
  of them checks out: Ryu is six straight columns, Chun-Li six 2x2 blocks, Ken's rows alternate,
  the Drop Alley is last in all eight column orderings, and every distribution table is 26 crash
  gems in 64 and 11 in its first 32.
* **The time tier's comparison is `<=`.** The rules doc gives it two ways and they disagree at
  the two boundaries it quotes in prose, 75 and 405 seconds. Inclusive is the reading that makes
  both forms agree everywhere.
* **The corner-code sum needs saying as what it means.** Summing the corners inside a candidate
  rectangle is a compact way of saying "every power gem this touches is wholly inside it, and
  there are at most two" — but read as a bare sum it also accepts a rectangle sitting in the
  *middle* of a large gem, where there are no corners to count, and restamps it for ever. The
  original never asks that question because its scan starts at a gem's own corner. `gems.rs`
  tests the meaning and asserts the sum agrees with it.

**A defaulted rule is recorded rather than guessed**: what sets `+0x292`, the gem distribution
table, is not read, so it is drawn from the match seed and fixed for the whole match — which is
the compendium's own rule that `speed_index` may change how a game feels but never what it deals.

**3 — the ai.** Placement search and evaluation. Power gems make this genuinely different from the
other three: the interesting move is often *not* to break. Puyo Rusto's beam search over a hand
written evaluation is the closest sibling and the place to start; no neural model.
*Done when:* a difficulty ladder exists and is **measured**, the way `ga puyo rank` measures Puyo's
rather than assuming it.

**4 — arcade theme. Done, and the fighter layer is not.** `rustle-fighter/src/theme/arcade/`,
cut by `art/rip.py`, `art/music.py` and `art/sfx.py`, plus the `AnyGame` arm and menu entry that
phase 5 was going to add - because a theme nobody can reach is not a theme anyone can judge.
What went in: the gems, the playfield frame, the NEXT box, the score plate and face, the brick
wall, seven arcade stage themes as loop pairs, the character select tune over the menus, and
thirteen effects. What did not: the seven fighters and their animation states, and the arcade's
own CAUTION / WARNING / DANGER plates, which are cut but unused - the engine's tray of icons
says the same thing more precisely and the plates are left for the fighter layer.

Five things it settled:

* **The sheets do not carry per-cell power gem art**, which this plan assumed they did ("the
  multi-cell rectangle has real art behind it and is not something we have to synthesise").
  What they carry is a tiled body texture in four shine frames and a top edge row with rounded
  corner notches: the arcade composites a border over a tiled fill at draw time, at whatever
  size the rectangle is. Our renderer is one snip per cell, so `art/rip.py` builds a 3x3
  template out of the sheet's own 2x2 tile - its four quadrants are the four corners, the
  cornerless middles of its edges are the edge cells - and cuts the nine masks from that. Every
  pixel is the arcade's; the arrangement is ours.
* **Nine masks, not sixteen.** Every cell of a rectangle at least two on a side is
  (top | middle | bottom) x (left | middle | right). `PowerMask::REACHABLE` is that list, and a
  test decodes the cut sheet to hold the script and the theme to it.
* **The frame confirms the Drop Alley.** Its top row is hatched tabs over every column *except*
  column 3, which is open - the same fact the disassembly gave, arrived at independently.
* **The panel layout is composed, not measured**, and that is a departure from the house rule.
  Every other retro theme's geometry was measured against the emulated game; this one could not
  be, so the theme arranges the arcade's own furniture rather than reproducing where the arcade
  puts it. The *board* is exact, straight off the frame. The theme module says so at the top.
* **Character select is a menu row**, which closes the one shell question left open. A row is
  what the choice actually is, and the arcade's select screen is as much about picking an
  opponent - a Street Puzzle mode thing, and out of scope.

**4b — the fighter layer**, still to do. The ripped sprites, panel geometry measured against
the real thing, and the full character animation layer: seven fighters, every labelled state on the
sheets, driven by `engine/src/animate/character.rs`. Both halves are specified: board fill drives
Idle / Disadvantage 1 / Disadvantage 2 with Advantage for a buried opponent, and the attack
magnitude is the `chain * chain` the chain loop accumulates into the opponent's `+0xfe`, quartered
with a floor of 1. *Done when:* `character_shot` renders
all seven casts, and `frame_shot` and `animation_shot` render a match that reads as the arcade game
does. This is now the *only* theme, so it is also the phase that makes the game playable at all.

**5 — the playlist.** The `AnyGame` arm, the `GameId` and the high score table came forward into
phase 4, because a game with no menu entry cannot be played. What is left of this phase is the
part that needs an ai: a place in `GameKind::PLAYLIST_ORDER`, and the twelve crossings
re-measured with `ga cross`.

`every_crossing_between_two_games_is_priced` now walks `PLAYLIST_ORDER` rather than `ALL`, and
the difference is the point: a crossing only exists between two games a playlist can deal into
one match. The moment this game joins that list the test asks for all six of its prices.

## Art and audio

Alex's drop, `~/Downloads/Super Puzzle Fighter Art/`, 54 files, 11MB. Arcade rips throughout,
except the sound effects, which are PlayStation.

### Gems — four sheets, one layout

| file | size | |
|---|---|---|
| `Miscellaneous - Gems.png` | 612×568, RGB | **red**, green chroma key rather than alpha |
| `Miscellaneous - Blue Gems.png` | 612×568, RGBA | blue |
| `Miscellaneous - Green Gems.png` | 612×568, RGBA | green |
| `Miscellaneous - Yellow Gems.png` | 612×568, RGBA | yellow |

All four share a layout, and it contains everything the rules need:

* the plain gem and the crash gem;
* **power gem tiles** — the run of progressively wider and taller pieces along the top. These are
  what the corner codes index, so the multi-cell rectangle has real art behind it and is not
  something we have to synthesise;
* **counter gems carrying the digits 0–9**, three rows of them at the bottom right — the countdown
  numerals, matching the `0x0F00` field in the cell encoding;
* a long destruction animation, dozens of frames of shattering fragments;
* a joined-state block bottom left.

The red sheet keys on green rather than carrying alpha, so the ripper has to handle both.

### Characters — seven sheets, and they are labelled

| file | size |
|---|---|
| `Characters - Chun-Li.png` | 1256×1710 |
| `Characters - Felicia.gif` | 700×1092, **gif not png** |
| `Characters - Hsien-Ko.png` | 2100×2091, RGBA |
| `Characters - Ken.png` | 1020×1319 |
| `Characters - Morrigan.png` | 1368×1902 |
| `Characters - Ryu.png` | 1004×1473 |
| `Characters - Sakura.png` | 1416×1806 |

Each sheet is annotated by the ripper with the state each row belongs to. Ryu's, which is
representative:

| row | frames | drives |
|---|---|---|
| **Idle** | 7 | board pressure 0 |
| **Advantage** | 8 | the gloat state — opponent buried, you clear |
| **Disadvantage 1** | 8 | board pressure 1 (37–54 cells) |
| **Disadvantage 2** | 8 | board pressure 2 (above 54) |
| **Fight Start & Taunt** | 8 | round start, and the once-per-round taunt |
| **Special Move 1** | ~10 + projectile | attacking |
| **Super Combo** | ~10 + projectile | a large attack |
| **Minor Damage** | 2 | taking a small attack |
| **Major Damage** | 2 | taking a large one |
| **Victory & Selected** | 5 | round won, and the character select |
| **Lose** | 3 | round lost |
| **Intermission Cutscenes** | 5 | out of scope |
| **Misc / Custom** | 2 | additions by the ripper, not original |

**The first four rows are exactly the four states the code drives.** `FUN_80135FB4` sets the
animation from `+0x79` — 0, 1, 2 by board fill — with one extra value for gloating over a buried
opponent, and the sheet has precisely those four rows. Independent confirmation that the posture layer is
fully understood — and the attack rows are driven by the chain-squared magnitude the chain loop
sends to the opponent.

Each sheet also carries the large character-select portrait, alternate palettes, a name plate,
NEXT boxes, a win icon, and **the "COUNTER GEM" panel showing that character's drop pattern** —
the HUD element that tells the opponent what is coming.

### HUD and screens

| file | size | contents |
|---|---|---|
| `Miscellaneous - HUD.png` | 408×266 | score plates, two digit sets, EASY/NORMAL/HARD, the playfield frame with drop-alley tabs, two NEXT boxes, the VS marker, **CAUTION / WARNING / DANGER plates in two colourways**, and two logo variants |
| `Miscellaneous - Text.png` | 1616×1319, RGBA | fonts and the game messages |
| `Miscellaneous - Character Portraits.png` | 702×591 | select-screen portraits |
| `Miscellaneous - Player Select, Mode Select & Level Select Screens.png` | 804×661, RGBA | the select screens, which the character select needs |
| `Miscellaneous - Title Screen.png` | 1366×630, RGBA | title |
| `Miscellaneous - Character-Specific Background Tiles.png` | 392×366 | per-character playfield backgrounds |
| `Miscellaneous - Super Combo Finish Background.png` | 1180×1393 | out of scope |

The three warning plates match the three levels in `FUN_8012E88C` exactly, and the blue digit set
is the block count. The HUD sheet is where the arcade theme's panel geometry gets measured from —
**measured against the real thing, not read off the rip**, as every other retro theme here was, and
the numbers go in the theme module beside a comment saying what they were measured from.

### Sound effects — complete, and from the build we decompiled

Thirteen zips of plain WAVs: eleven fighters, an announcer, and a common set. The filenames
(`SE_PL02.EMI_00014.wav`, `SE_COMN.EMI_00029.wav`) are extractions from the PlayStation disc's own
`data/*.emi` containers — the same files in `~/Downloads/spf2t-re/disc_listing.txt`. Voice clips
exist for all eleven fighters, including the four whose sprites we do not have.

### Music — 23 VGZ, and the tool is a build

The 23 tracks are gzipped VGM logs of the **arcade** QSound hardware, so music and effects have
different provenance. That is acceptable — the effects are the ones tied to gameplay events.

**Nothing in the Fedora repositories plays VGM.** There is no `vgmplay` and no `libvgm`;
`audacious-plugins` only helps inside Audacious, and `ffmpeg` cannot decode it. The route is Alex's
checkout at **`~/projects/vgmplay-libvgm`**, which carries `libvgm/audio/AudDrv_WaveWriter.c` and a
`LogSound` config option, so it renders to WAV headlessly without a sound device — exactly what a
ripper script needs.

Wrap it in `rustle-fighter/art/music.py`, following `puyo-rusto/art/music.py`: render each VGZ to
WAV, trim and loop, encode, and print what it did. Track 03 is *Versus* and is the one the versus
mode wants.

One PlayStation CD audio track already sits at `~/Downloads/spf2t-re/audio/spf2t_track02.ogg`,
levelled to −22.1 dBFS RMS. It is a single 4m15s tune and is not a substitute for the 23.

### Levelling

Music at **−22 dBFS RMS**, effects within about four decibels of it, nothing peaking over
−0.5 dBFS. Run `engine/art/audio_levels.py` after cutting anything. The effects are per-fighter
voice clips and will want `with_effects_at` rather than `with_gain` if they sit wrong against the
music. Never level a set slot by slot on peaks.

**Every ripper is a script, committed, re-runnable.** The rips themselves stay out of the
repository, exactly as the existing themes do.

## Open decisions

None. Character select, the seven-character roster, the full animation layer and the arcade-only
theme were all decided 2026-09-07 and are folded into *Scope*; **where character select lives**
was the last one and phase 4 answered it — a row on the menu, for the reasons above.

### Resolved while planning, recorded so it is not re-litigated

**Character index → name is done, and it needed no emulator.** Every arcade character sheet carries
the in-game "COUNTER GEM" panel showing that character's own drop pattern; reading the 6×4 grid off
the sheets and matching it against the extracted table pins the mapping, cross-checked against the
FAQ's per-character strings. The table is in the rules doc. Our seven are indices **0 Morrigan,
1 Chun-Li, 2 Ryu, 3 Ken, 4 Hsien-Ko, 6 Felicia, 7 Sakura**.

It also decoded the colour numbering the whole pattern table is written in — **1 blue, 2 yellow,
3 green, 4 red** — which nothing else had pinned.

## Risks

* **Power gems may be worth nothing, and that is the one thing to watch.** Everything about them is
  read — formation, the 0/15/30 corner-sum acceptance rule, merging, and the fact that the code
  gives them no scoring or destruction premium over the same gems loose. If they turn out to feel
  pointless in play, a term is missing and the hunt resumes; phase 2's tests will not catch it,
  because they test what the code does, and the code is what we implemented. **Play the game at the
  end of phase 2 before starting the ai** — an ai built against a broken incentive is wasted work.
* **The ai is a research problem, not a port.** Dr. Rustario had a scorer to port and Puyo had open
  literature. This has neither, and the interesting decision — build or break — has no published
  treatment. Budget for phase 3 taking longer than phase 2.
* **Twelve crossings is a re-measurement of the whole table**, not an append. Expect the existing
  six to move.
* **The power gem growth loop is the highest-risk transcription.** ~1,700 bytes of pointer
  arithmetic per function, three of them, and Ghidra's decompiler has already been caught mangling
  control flow twice in this codebase. Read the disassembly, not the pseudo-C, and expect it to be
  the slowest part of phase 2.
* **Scope.** Seven characters, thirteen animation states each, a full arcade HUD and 23 music
  tracks is a content surface with no natural floor. The rules and the board are the deliverable;
  the fighter layer is where an unbounded amount of time can go.
* **One theme means no fallback.** With the particle theme deferred, there is no version of this
  game that carries only original art — see *Scope*.
