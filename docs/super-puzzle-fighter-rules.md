# Super Puzzle Fighter II Turbo — rules, read from the code

This is a rules reference reconstructed by **reading the game's own code**, not by observing it.
It exists so that if Puzzle Fighter is ever built here it is sourced the way Dr. Rustario's ai was
sourced from `aiset.c`, rather than from strategy guides that disagree with each other.

It is **not** a plan. It records *what the game does*. The status board in
[next-game-ideas.md](next-game-ideas.md) still says whether the game is being built.

## Provenance

| | |
|---|---|
| Binary | `SLUS_004.18`, 583,680 bytes, from *Super Puzzle Fighter II Turbo (USA)*, PlayStation |
| Built | `CAPCOMNov 13 199615:05:56CAPCOM0000` (build stamp in the executable) |
| Format | `PS-X EXE`, no encryption, no compression. Text loads at `0x80100000`, 581,632 bytes, entry `0x8015c8e8` |
| Toolchain | Sony Psy-Q C (libgpu/libcd strings present; 2,303 functions, symmetric compiler prologues, 14.6% unfilled delay slots) |
| Analysis | Ghidra 12.1.3, raw binary loader, `MIPS:LE:32:default`, base `0x80100000`. Auto-analysis finds 2,303 functions and decompiles all of them with zero failures |

**Every address below is a PS1 virtual address in that executable.** Field offsets are into the
player struct. Reproduce the whole thing in about a minute:

```shell
chdman extractcd -i "Super Puzzle Fighter II Turbo (USA).chd" -o spf2t.cue -ob spf2t.bin
# convert the MODE2/2352 data track to a plain iso (2048 bytes at offset 24 of each 2352-byte sector)
iso-read -i spf2t.iso -e slus_004.18 -o SLUS_004.18
dd if=SLUS_004.18 of=spf2t.text bs=2048 skip=1        # strip the 0x800 header
analyzeHeadless <proj> spf2t -import spf2t.text -processor MIPS:LE:32:default \
    -loader BinaryLoader -loader-baseAddr 0x80100000
```

### Why the PS1 port and not the arcade

The arcade original is CPS2, where the 68000 program ROM is **encrypted on opcode fetches only**
— data reads return the raw ROM. Decrypt naively and the code reads correctly while every data
table is garbled, which is exactly where the drop patterns and damage values live. The PS1 port
has no such problem, is Psy-Q C rather than hand-written assembly, and loads at a fixed address
with a flat 2MB map.

**This documents the PlayStation ruleset, and that is deliberate.** Alex's decision
(2026-09-07): the port is official, the board is the same shape, and a recreation of *this*
ruleset is the goal. Where the arcade differs — if it does — the arcade is simply a different
game for our purposes. Nothing here is contingent on an arcade cross-check, and none was done.

## Confidence

Findings are marked:

* **[code]** — read directly out of the decompilation, and in several cases cross-checked against
  a number published in a strategy guide. Treat as fact.
* **[partial]** — the mechanism is located and mostly understood, with a named gap.
* **[open]** — located but not yet read, or read and not yet understood.

## Deliberate deviations

This document records what the game does. Where we have decided *not* to reproduce something,
it is listed here rather than quietly written out of the rules above, so that the difference
between "the original does this" and "we chose otherwise" is never in doubt.

| What | Decision | Why |
|---|---|---|
| **Rotation and kicks** | **Use Puyo Rusto's rotation, and move it into `engine/` to share.** Alex, 2026-09-07 | The PS1 rotation has no wall kick and no quick turn (see [Rotation](#rotation-kicks-and-locking-partial)); later ports are believed to have added kicks, and a player wedged between two columns with no escape reads it as a bug rather than a rule |

## Player struct

Two players; each holds a pointer to the other. Confirmed fields:

| Offset | Type | Meaning |
|---|---|---|
| `+0x50` | int | score |
| `+0x67` | char | character-indexed bit position for the damage-modifier masks (see below) |
| `+0x68` | char | character index, 0–10; selects the drop pattern and the damage modifiers |
| `+0x84` | ptr | **the opponent's player struct** |
| `+0x88` | ptr | the cell array; `+0x400` from it is the power gem array |
| `+0x8c` | ptr | the companion board used by the power gem passes |
| `+0x79` | char | board pressure 0/1/2, drives the fighter sprite |
| `+0x1e0` | short | rotation lifts used this piece (the climbing trick), capped at 3 |
| `+0x21b` | byte | falling pair orientation, 0-3 |
| `+0x222` | byte | **state machine index**, 0-20 |
| `+0x264` | char | power gem id allocator, wraps 255 to 1 |
| `+0x11e` | short | power gem corner-code sum for the candidate rectangle |
| `+0x10c`…`+0x118` | short ×7 | the seven score accumulators (below) |
| `+0x13e` | short | occupied cell count, recomputed each settle |
| `+0x1d2` | short | row index of the incoming counter-gem block |
| `+0x1d4` | short | column ×2, the argument to the counter-gem cell builder |
| `+0x1f0` | short | **elapsed round time in seconds**, capped at 539 |
| `+0x1f2` | short | frame counter; at 60 it resets and bumps `+0x1f0` |
| `+0x21a` | byte | warning level 0–3 |
| `+0x22a` | short | **outgoing counter gems** — the attack this player has generated, aimed at the opponent |
| `+0x229` | char | chain depth |
| `+0x265` | char | total gems destroyed |
| `+0x266` | char | best chain this round |
| `+0x28c` | char | position within the partial counter-gem row, 0–5 |
| `+0x28d` | char | which of the eight column orderings is in use |
| `+0x292` | byte | selects the gem distribution table |
| `+0x298` | char | chain counter used by the HUD |
| `+0x29c` | char | running All Clear award |
| `+0x29e` | byte | 4-bit mask of colours erased this step |
| `+0x106` / `+0x108` / `+0x10a` | short | pieces dealt / next rainbow piece number / rainbows so far |
| `+0x26c`…`+0x26f` | char ×4 | per-colour deal counters (drought rule) |

Note `+0x22a` is **outgoing**, not incoming: `FUN_80131154` reads `opponent->0x22a` to decide what
falls on *this* player, and `FUN_8012e824` copies this player's `+0x22a` into the opponent's HUD.

## Board **[code]**

Three parallel arrays, all 16-bit-cell, **8 cells per row (16-byte stride)**, of which **columns
1-6 are playable** — columns 0 and 7 are borders. `board+0xC2` is the bottom-left playable cell
and scans walk **upward** with a -16-byte step.

* `+0x88` — the cell array.
* `+0x88 + 0x400` — the **power gem array**, one entry per cell: low byte is a corner code
  (`1` `2` `4` `8`, plus sprite codes `0x30` `0x60` `0x90` `0xC0`), high byte is the **power gem
  id** that all cells of one gem share.
* `+0x8c` — a second board, used as the companion cursor throughout the power gem passes.
* `board-0xE` … `board-0x4` — the six-cell **staging row** that incoming counter gems are written
  into before they fall.

The census and gravity passes scan **13 rows**; the erase pass scans **14**, one above the visible
field. The field is therefore **6 wide x 13 tall with one row of headroom**.

**Column 3 (0-based) is the Drop Alley** — the fourth from the left, confirmed twice below.

### Cell encoding **[code]**

| Bits | Meaning |
|---|---|
| `0x0007` | colour `1`-`4`; the value `7` means **counter gem** |
| `0x0008` | crash gem (so a crash gem of colour *c* is `c+8`, i.e. `9`-`12`) |
| `0x0010` | marked for erase this step |
| `0x0020` | marked by the current propagation pass |
| `0x0040` | excluded from propagation |
| `0x0080` | **this cell belongs to a power gem.** Not "occupied" — the only writers of this bit anywhere in the executable are the three power gem formation functions |
| `0x0F00` | counter gem countdown |
| `0xF000` | counter gem colour, held here until the countdown expires |

`0` is empty; `0xFF` and `0x57` are transient markers cleared during the census. Classes `5`
(rainbow) and `6` exist and are both excluded from power gem formation.

This resolves cleanly against everything else. A counter gem is `colour<<12 | 0x507`: its low three
bits are `7`, so it is not a colour, and its `0x80` is clear, so it is not in a power gem. The merge
pass requires `0x80` and therefore only ever considers cells already in power gems, which is what
makes it a *merge*. The census clears the power gem array entry for every cell without `0x80`.

**One loose end**: the erase pass `FUN_80134A28` gates its "gems destroyed" counter `+0x265` and the
per-colour tallies on `0x80`, so those count only power gem cells. That may be deliberate — they
feed the end-of-round bonus — but it is worth confirming.

## Piece generation **[code]**

`FUN_8012fd78`. Each half of the pair is an independent draw from a **64-entry weighted table**
at `0x8016D81C`, selected by `+0x292`:

* first half uses `rng & 0x3F` — the whole table
* second half uses `rng & 0x1F` — **only the first 32 entries**

The two halves therefore have *different* distributions. Four tables exist, each biasing two of
the four colours (12 entries each) over the other two (7 each), with crash gems making up
**40.6% of the first half's draw and 34.4% of the second's** — identical rates in all four
tables, which differ only in which colours they favour. If both halves come out as the same crash gem, the first half is demoted to the
normal gem of that colour (`if (a == b && a > 8) a -= 8`), so a pair never self-destructs on
landing.

**Rainbow gem.** `+0x106` counts pairs dealt and is never reset. When it reaches `+0x108` the
second half of the pair becomes class `5` and `+0x108` is reloaded from the schedule at
`0x8016DB3C`. So the guides are right: **every 25th pair**, and now it is sourced rather than
counted. The schedule runs `25, 50, …, 600` and then jumps: `800, 850, 900, 950`, and then
`9999`, which is to say never again. All twenty-eight entries are transcribed in
`rustle-fighter/src/game/tables.rs`.

**All four distribution tables are transcribed**, in that same module, along with the eight
column orderings and the 11 × 12 × 6 drop pattern table. Every published description of them
checks out against the bytes: Ryu is six straight columns, Chun-Li six 2×2 blocks, Ken's rows
alternate colours, Dan's board is one colour, and the Drop Alley is last in all eight orderings.

**The drought rule — undocumented anywhere.** `FUN_8012FF2C` counts every gem dealt per colour in
`+0x26C`…`+0x26F`. When any colour's counter passes 12, it resets and the *next* piece's first
half is forced to the crash gem of that colour (class 9/10/11/12). You are guaranteed a crash gem
for whatever colour you have been flooded with.

## Piece lifecycle — the state machine **[code]**

`+0x222` is a state index dispatched every frame through the jump table at **`PTR_FUN_8016D76C`**
(21 states, 0-20) by `FUN_8012E5F4`. Almost every handler is a stub that increments `+0x222` and
calls one worker, so the settle sequence is a straight pipeline rather than a graph:

| State | Worker | What it is |
|---|---|---|
| 0 | `FUN_8012E8DC` | round/piece reset; picks the counter-gem column ordering (`rng & 7` into `+0x28d`) |
| 1, 3, 10 | `FUN_80131400` | shared step, sets `DAT_8019FCAF` |
| 5, 6, 7 | `FUN_80132964`, `FUN_80133024`, `FUN_801336F0` | **power gem formation**, three corner scans |
| 8 | `FUN_80131AE0`, `FUN_8013190C`, `FUN_80131C00` | gravity / settling |
| 9 | `FUN_80134808` | — |
| 12, 13, 14 | `FUN_801343A4`, `FUN_801340B4`, `FUN_80133DC4` | **power gem merging**, three corner scans |
| 19 | `FUN_8012ECA8` (688 bytes) | the large mid-pipeline step |
| 20 | `FUN_8012F154` | terminal state; the round timer stops when the *opponent* reaches it |

`FUN_80132450` and `FUN_80133D54` run after most steps. Entries 21-23 of that table are the three
HUD message drawers (`CHAIN!`, `TECH BONUS`, `ALL CLEAR`) and are **not** states.

## Rotation, kicks and locking **[partial]**

> **Decision: do not implement what this section describes.** The rules below are the record of
> what `SLUS_004.18` actually does, kept because it is evidence and because a future reader will
> want to know what was traded away. **The implementation should use Puyo Rusto's rotation
> instead** — `puyo-rusto/src/game/pair.rs`, whose rules come from Puyo Nexus. See
> *Deliberate deviations* above.
>
> What that changes, concretely:
>
> * **gains a wall kick and a floor kick** — a blocked rotation is pushed away from whatever the
>   child was turning into rather than refused;
> * **gains the quick turn** — wedged between two columns, a second press flips the pair end over
>   end in place. The PS1 game has no escape from that position at all;
> * **loses the three-lift cap** on rotations that raise the pair (`+0x1e0`), and with it the
>   "climbing gems" trick as the guides describe it. Puyo floor-kicks as often as the board allows;
> * **swaps the ceiling rule** — Puyo's current-row check, which refuses an upright turn in the
>   ghost row outright, replaces the PS1's `+0x12a` spawn-area guard.
>
> **Where the code lives — decided, 2026-09-07.** The compendium keeps **two** rotation systems for
> its two-halves-and-a-pivot games, because they are genuinely different rule sets and unifying them
> behind one abstraction would fight all three for no gain:
>
> * **Dr. Mario's** — a candidate kick list, four escapes turning upright and two turning flat, and
>   no quick turn because its four orientations are only two shapes, so the flip falls out of
>   ordinary rotation. Used by Dr. Rustario alone. **Stays where it is**, in
>   `dr-rustario/src/game/pill.rs`.
> * **Puyo Puyo's** — one derived kick, then the quick turn as a guaranteed escape, plus the
>   ghost-row current-row check. Used by Puyo Rusto **and Puzzle Fighter**, so it **moves up into
>   `engine/`**. Game crates are siblings that never depend on each other, and no Puzzle Fighter
>   crate may reach into `puyo-rusto`.
>
> That move is a **relocation, not a merge**: lift `pair.rs` into the engine and leave `pill.rs`
> alone. It touches a shipped game, so Puyo Rusto's behaviour must not shift by a frame — the move
> wants its own commit, with the existing tests green before and after and nothing else in it.


Orientation is `+0x21b`, values `0`-`3`, stepped `&3`. `FUN_801301E4` rotates anticlockwise
(`(o-1)&3`) and `FUN_8013033C` is its clockwise twin. Both delegate the legality test and the
kick to **`FUN_80130494`**.

Six 4-byte tables from `0x8016D804`, all indexed by target orientation, give the second half's
cell offset relative to the pivot and the kick to apply:

| Table | Values | Meaning |
|---|---|---|
| `0x8016D804` | `0, 1, 0, -1` | second-half column offset |
| `0x8016D808` | `-8, 0, 8, 0` | second-half row offset (8 cells = one row) |
| `0x8016D80C` | `0, -1, 0, 1` | the opposing probe |
| `0x8016D810` | `8, 0, -8, 0` | " |
| `0x8016D814` | `0, -1, 0, 1` | x adjustment applied on success (**the kick**), into `+0x3c` |
| `0x8016D818` | `1, 0, -1, 0` | y adjustment applied on success, into `+0x40` |

So orientation 0 puts the second half one row up, 1 one column right, 2 one row down, 3 one
column left — an ordinary four-state pivot. The kick is not an SRS-style candidate list: rotation
always applies exactly one fixed offset, so the pair pivots about a point that shifts by a cell
rather than trying a sequence of fallbacks. A rotation that fails the probe is simply refused
(`+0x248 = 2`).

**The climbing trick is real and it is capped.** When the y adjustment is negative — a rotation
that lifts the pair, which is the `y = -1` entry for target orientation 2 — `FUN_80130494` counts
the lifts in `+0x1e0`, and on the **third** it sets
`+0x1e0 = 5`, `+0x21f = 1` and `+0x1e2 = 1`, which locks further rotation. StrategyWiki's "tap
the rotation buttons and you can actually lift the gems" is a real mechanic with a hard limit of
**three lifts per piece**.

**[partial]**: the sense of the collision probe in `FUN_80130494` is inverted from what the cell
encoding implies (it fails when the probe reads `0`), so `param_3` is a companion array with
opposite polarity rather than the cell array. Resolve before implementing.

## Power gems **[partial]**

Six functions, in two groups of three. Each group scans from a different corner — the formation
group starts at `board+0xC2` (bottom-left), `+0xCC` and `+0xCC`; the merge group likewise — which
is how a rectangle is found regardless of which way it grew.

**Formation** (`FUN_80132964`, `FUN_80133024`, `FUN_801336F0`, ~1,700 bytes each). From each
anchor cell the scan extends upward while the colour matches, to a maximum of **10**, then extends
rightward, testing each new column to the established height. Cells are rejected when the colour
differs, when `0x80` is clear, or when the class is `6` or `7`.

**The acceptance rule is the elegant part, and it is undocumented anywhere.** While scanning, the
power gem corner codes inside the candidate rectangle are summed into `+0x11e`. The rectangle is
accepted only when that sum is **0, 15, or 30**:

* **0** — no existing power gem inside it; a fresh rectangle of plain gems.
* **15** — exactly one existing power gem (`1+2+4+8`) wholly inside it; it is absorbed.
* **30** — exactly two.

Any other sum means a power gem is *partially* overlapped, and the candidate is rejected. This is
the mechanism behind the guides' observation that a power gem gets harder to extend as it grows:
you are not adding a row to a gem, you are finding a larger rectangle that swallows it whole.

On acceptance the region is rewritten: the power gem array is cleared across it, every cell is
stamped with a new id from the allocator at `+0x264` (which wraps 255 to 1), and the four corners
get codes `0x30`/`1`, `0x60`/`2`, `0x90`/`4`, `0xC0`/`8`.

**Merging** (`FUN_801343A4`, `FUN_801340B4`, `FUN_80133DC4`, ~750 bytes each) is the same idea
applied to two existing gems: it walks from a `1` corner to a `4` corner, requires the corner sum
to be exactly **30**, and rewrites the union as one gem.

**The sum has to be read as what it means, not as a sum.** Implementing it (2026-09-07) turned
this up: "the corner codes inside the rectangle sum to 0, 15 or 30" is a compact way of saying
*every power gem this rectangle touches is wholly inside it, and there are at most two of them* —
because a whole gem contributes all four of its corners and a partly-covered one contributes some
other number. Taken literally as a sum it also accepts a rectangle sitting entirely in the
**middle** of a large power gem, where there are no corners to count at all, and a search that
does that restamps the same gem for ever. The original never asks that question because its scan
anchors on a gem's own corner and grows outward. `rustle-fighter/src/game/gems.rs` implements the
meaning and asserts the sum agrees with it.

**[partial]**: the structure, the caps and the acceptance rule are certain. The exact
column-by-column growth loop is ~1,700 bytes of pointer arithmetic per function and has not been
transcribed line by line; do that with the decompilation open at implementation time rather than
trusting a paraphrase. **Whether a power gem scores more per cell than a loose gem is an open
question** — see *Score* and the gap list.

## The break — propagation and chains **[code]**

The verb of the game, in three functions.

### Propagation — `FUN_80131C00`, state 8

A four-neighbour flood from each crash gem, over the 6x14 scan. A neighbour joins the break when
**all** of these hold:

* its low three bits are not `7` — counter gems never join a break by colour;
* neither `0x10` nor `0x40` is set — not already erasing, not excluded;
* `neighbour & 7 == crash_gem & 7` — **same colour**.

Both cells are then marked `0x20`. A cell that matched nothing has `0x20` and `0x40` cleared again
(`&= 0x9F`). When any cell was marked the function sets state 3, `+0x21d = 1`, **`+0x21e = 0x28`**
and `+0x224 = 1`.

**Counter gems are taken as collateral, not as matches.** `FUN_801320B0` runs on every marked cell
and turns any of its four neighbours whose low byte is exactly `7` into `0x37` — that is
`0x20 | 0x10 | 7`, marked and erasing. This is StrategyWiki's "if a gem that touches a counter gem
is destroyed, even if that gem is a different colour, the counter gem will be shattered", and
`FUN_8013495C` implements the same rule again during the erase pass.

**Power gems need no special case.** A power gem is a solid rectangle of one colour with `0x80` set
per cell, so the ordinary colour flood walks the whole of it. Nothing in the propagation treats it
specially, and nothing needs to.

### The chain loop — `FUN_801346A0`

`+0x21e` is an erase-animation delay in frames, set to **40** whenever a pass marks anything. This
function is stepped each frame:

```
if (p->0x21e != 0):
    if (--p->0x21e == 0):
        reset +0x110, +0x114, +0x116, +0x118 and the six class flags
        p->0x229 += 1                  # chain depth
        FUN_8012F378(); FUN_80134A28(p) # the erase and score pass
        p->0x23f = 0
        p->0x222 = 3                   # back round the pipeline
else:
    opponent->0xfe += chain * chain    # then >>2, floored at 1
    p->0x222 += 1                      # nothing left to erase; move on
```

So a chain is **a loop, not a search**: mark, wait 40 frames, erase and score, look again. The chain
counter increments once per pass and is zeroed by the end-of-settle reset in `FUN_8012ECA8`.

The `chain * chain` accumulated into the **opponent's** `+0xfe`, quartered with a floor of 1, is
almost certainly what drives how hard your fighter hits theirs — the attack-animation magnitude
that the fighter section lists as open.

### The rainbow gem — `FUN_8013190C`

It destroys every gem of the colour it lands on, marking with `0x20`. Two special cases on the cell
directly beneath: a counter gem becomes `0x27`, a cell without `0x80` becomes `0x26`, and if it is
the floor marker `0xFF` the game sets `+0x29a`, which is worth **+10,000 points**.

That is the **Tech Bonus**, and it settles what the guides only gesture at: dropping a rainbow gem
down an empty lane so it lands on the floor pays ten thousand points, which is why they tell you
not to spend it.

## Score **[code]**

Seven accumulators are summed into `base`, which is added to the score **and** is the sole input
to the damage calculation. This is why points and counter gems move together.

| Field | Value |
|---|---|
| `+0x10c` | chain bonus — `CHAIN[min(chain-1, 10)]` from `0x8016E644`: `0, 200, 400, 1000, 1600, 2200, 2800, 3400, 4000, 4000, 4000` |
| `+0x10e` | colour bonus — 200 per distinct colour erased beyond the first **[partial]**, the exact off-by-one is not certain |
| `+0x110` | `10 × max(0, gems_erased − 2)` |
| `+0x112` | **the reclaimed-garbage bonus** — see below |
| `+0x114` | `100` per erased cell of class < 7 (normal gems) |
| `+0x116` | `100` per erased cell of class 8–13 (crash gems) |
| `+0x118` | `10` per erased cell of class 7 (counter gems) |

A counter gem is therefore worth **a tenth** of a normal gem, not the half the FAQs guess.

### `+0x112`, read from the disassembly

`FUN_80134808` scans the board and counts every cell that is marked for erase, is **not** a counter
gem or class `0xF`, and **whose high byte is non-zero**. It then computes

```
n     = that count
step  = DAT_8016E584[min(n, 51) - 1]      # 100 for n<=8, 150 to 15, 200 to 24,
                                          # 250 to 35, 300 to 48, 350 beyond
if (step >= 151 && p->0x2a6) step = 150   # a cap under one condition
+0x112 = step * n
```

The high-byte test is the interesting part. A gem the player placed has nothing in bits 8-15; a
**counter gem carries its colour in bits 12-15 and its countdown in bits 8-11**, and the colour
survives when the countdown expires and the gem turns normal. So this accumulator pays out for
destroying gems that **arrived as garbage** — which is exactly what StrategyWiki tells players to
do: "the reason you want to wait for the Counter Gems to become normal Gems is because when
they're detonated, you'll be awarded a higher score and Counter Gem attack count."

The scaling is `step * n`, confirmed against the instruction stream — the decompiler renders the
loop with a `goto` into its own body and cannot be trusted here.

**Test vector, and it matches exactly.** StrategyWiki's worked example destroys 12 linked gems
with a crash gem and reports **1410 points**. This formula gives
`12×100 + 1×100 + 10×(13−2) = 1200 + 100 + 110 = 1410`. Its other two examples — the same twelve
gems with a 6-cell power gem (2010 points) and as one 12-cell power gem (3210 points) — are
**+600** and **+1800** over the plain case, and **nothing in these seven accumulators produces that
difference**. All three break the same thirteen cells, and a power gem cell's class nibble is its
plain colour, so it scores through `+0x114` at 100 like any other. See the gap list.

## Damage **[code]**

`FUN_80134D80`, called once per erase step. All divisions are floor; the original does them by
repeated subtraction.

```
tier  = count of [90,120,150,...,420] that are < (round_seconds + 15), capped at 12
      = clamp(floor((round_seconds - 75) / 30) + 1, 0, 12)
v     = base + tier * floor(base / 10)

L     = LEVEL[cfg & 7]                       # 0x8016E634 = [-2, 0, 3, 5, 5, 5, 5, 5]
n     = floor( floor(v / 10) * (L + 10) / 100 )

if (halve_flag)      n >>= 1                 # +0x23f, see below
                     n = character_and_mode_adjust(n)
if (opp_handicap > my_handicap)
                     n += floor(n / 10) * (opp_handicap - my_handicap)

outgoing = min(250, outgoing + n)
```

**Check it.** Crash three normal gems with a crash gem: `base = 3×100 + 1×100 + 10×(4−2) = 420`;
`tier = 0` early in a round; `floor(420/10) × 10 / 100 = 4`. **Four counter gems** — exactly what
every FAQ reports for that case.

Three things fall out that no guide states correctly:

* **Damage grows with elapsed round time, not board height.** `+0x1f0` is a seconds counter. Every
  30 seconds past 75 adds another 10% of `base` to `v`, to a maximum of +120% at 405 seconds.
  **The threshold comparison is `<=`**, settled 2026-09-07: the two forms written above disagree
  at exactly the two boundaries this sentence quotes — under a strict `<` the count gives 0 at 75
  seconds and 11 at 405 — and inclusive is the only reading under which they agree everywhere. The
  guides' "how late in the game" is this, and their "destroying gems high on the screen is worth
  more" is *not* in this function at all.
* **`L` is a global difficulty/level setting**, worth ×0.8 at its lowest and ×1.5 at its highest.
  Any measurement taken without pinning it is not reproducible.
* **The pending attack caps at 250.** The HUD separately clamps its *displayed* copy to 99, and in
  one path (`FUN_8012F1B8`) clamps the field itself to 99 — which path runs when is **[partial]**.

Character and mode adjustment, from the same function: characters 8 and 9 have their damage
**halved** when `+0x6c == 0`, and characters 4 and 5 get **+25%** when their bit is set in the
masks at `DAT_801AE4D8` / `DAT_801AE4D9`. Sirlin records that Akuma and Devilot had the best drop
patterns in the original, which is consistent with two characters carrying a damage penalty.

The `+0x23f` halving flag is **[open]**. The diamond is documented by Sirlin as dealing 50% of
what the same break would deal without it, so this is very likely the diamond flag.

## Defence and offset **[code]**

`FUN_80134A28` resolves the two players' pools against each other the moment an erase completes:

```
mine   = my->outgoing        # already includes the attack just generated
theirs = opponent->outgoing

if (theirs < mine):                       # I am the larger attacker
    my->outgoing       = mine - theirs    # straight subtraction, no conversion
    opponent->outgoing = 0
else:                                     # I am defending
    opponent->outgoing = max(0, theirs - defence(mine))
    my->outgoing       = 0
```

The conversion applied to the **smaller** attack, `FUN_80134CF8`:

```
defence(n) = floor(n / 2)                             if n <= 11
             (floor(n / 8) + 1) * m                   otherwise
where m = 4 if n <= 17, 5 if n <= 23, 6 if n <= 29, 7 if n >= 30
```

This is the single most misreported rule in the game. StrategyWiki's "two counter gems cancel one"
is only true up to 11. Above that defence gets progressively better, reaching **1:1 at n = 24**
and exceeding it beyond — a 40-gem defensive break cancels 42.

| n | 8 | 12 | 16 | 18 | 24 | 30 | 40 | 60 |
|---|---|---|---|---|---|---|---|---|
| cancels | 4 | 8 | 12 | 15 | 24 | 28 | 42 | 56 |

**One caveat, and it needs checking.** The whole exchange is gated on
`DAT_8019C3F4 == 0 && round_seconds < 180`. Taken literally the two pools stop cancelling three
minutes into a round. That would be a sudden-death rule nobody has ever written down, so it is
more likely that `DAT_8019C3F4` distinguishes a mode and the gate means something narrower.
**[open]** — verify before relying on it.

## Counter gem delivery **[code]**

`FUN_80131154`, run when the receiving player settles a piece.

* If six or more remain, a **full row of six** is written to the staging row at once and the pool
  drops by 6.
* Otherwise gems are placed **one per column**, in an order taken from `DAT_8016E524`: eight
  permutations of the six columns, selected by `+0x28d`.

**Column 3 is last in all eight orderings**, which is the code's version of the FAQ's rule that
the Drop Alley only fills once every other column has taken a gem. Occupied columns are skipped
(`if (cell_high_byte == 0)`), and gems with nowhere to go are discarded.

### Colour: the per-character drop patterns

`FUN_801350E4` builds the cell:

```
idx  = *(short*)(0x8016E684 + character*0xC0 + min(row,11)*0x10 + column*2)   # colour 1..4
cell = COLOUR[idx] + 0x507        # COLOUR[1..4] = 0x1000,0x2000,0x3000,0x4000
                                  # 0x507 => countdown 5, class 7
```

so a counter gem is `colour << 12 | 0x507`, and in the alternate path it is `+ 0x307` instead —
**countdown 3**. StrategyWiki says counter gems arrive at 5 normally and 3 when they came from a
defended attack; the two constants are right there.

**Colour numbering, decoded 2026-09-07:** `1` = blue, `2` = yellow, `3` = green, `4` = red. Read
off Ryu's in-game pattern panel, whose six columns are red-green-blue-yellow-red-green against a
table row of `4,3,1,2,4,3`, and confirmed against five other characters.

The table at **`0x8016E684`** is **11 characters × 12 rows × 6 columns**, one colour index per
cell, 0xC0 bytes per character. It identifies itself against the published descriptions:

| # | pattern | matches |
|---|---|---|
| 1 | 2×2 blocks, `4433 11` repeated in pairs of rows | Chun-Li — "six groups of 2x2" |
| 2 | every row identical, `4 3 1 2 4 3` | Ryu — "six straight columns", "easiest to counter" |
| 3 | solid rows, colour changing per row | Ken — "rows alternate colors" |
| 4 | staircase | — |
| 8, 9 | diagonal, shifting one column per row | "diagonal pattern prevents large power gems" |
| 10 | every cell the same colour | Dan — "all gems of one color" |

### Character index → name **[code]**

Characters are indexed by `+0x68`. The mapping was resolved **without an emulator**, from the art:
every arcade character sheet carries the in-game "COUNTER GEM" panel showing that character's own
pattern, on an amber `(240,160,0)` field. Reading the 6×4 grid off six sheets and matching it
against this table pins six directly; the FAQ's per-character top-row strings and two eliminations
close the rest. **The panel lists its rows in reverse**, which only the diagonal patterns reveal —
the symmetric ones match either way.

| `+0x68` | character | | `+0x68` | character |
|---|---|---|---|---|
| 0 | Morrigan | | 6 | Felicia |
| 1 | Chun-Li | | 7 | Sakura |
| 2 | Ryu | | 8 | Devilot |
| 3 | Ken | | 9 | Akuma |
| 4 | Hsien-Ko | | 10 | Dan |
| 5 | Donovan | | | |

This closes a loop: the damage code halves characters **8 and 9** when `+0x6c == 0`, and 8 and 9 are
**Devilot and Akuma** — the two hidden bosses, whose drop patterns Sirlin records as the best in the
game. The penalty and the account agree.

Felicia's sheet is the one that carries no panel (it is a smaller, green-keyed gif from a different
ripper); her index comes from the FAQ's `gbbrry`, which is index 6's row 2.

## All Clear **[code]**

`FUN_80132118`. When the census finds the board empty:

```
my->0x29c += 6
score     += 600
outgoing  += my->0x29c
```

`+0x29c` accumulates across the round, so the first All Clear sends **6**, the second **12**, the
third **18**. The FAQ guessed a flat 12 and said it was unsure.

The same function sets a board-pressure level from the occupied count: `0` below 37 cells, `1` at
37–54, `2` above 54, out of 78.

## Warning display **[code]**

`FUN_8012E88C`, on the outgoing pool: `0` none, `1` at 1–10 (CAUTION), `2` at 11–30 (WARNING),
`3` at 31+ (DANGER). Matches every guide.

## The fighter sprites **[partial]**

The fighters are separate objects, not part of the player struct: `+0x30` points back at the
player, `+0x2c` at the other fighter. `FUN_80135FB4` sets the animation state:

```
state = player->0x79            # board pressure: 0 below 37 cells, 1 at 37-54, 2 above 54, of 78
if (other_fighter->0x89 == 2 && state == 0 && self->0x74 == 9 && self->0xb != 0)
        state = 0x10            # the opponent is buried and you are clear
self->0x89 = state
FUN_801364AC(self, state)       # drive the animation
```

So the **posture** — winning, pressured, losing — is a pure function of how full your own board is,
recomputed every settle by `FUN_80132118`, with one extra state for gloating over a buried
opponent. It reads no other game state.

**The sprite sheets confirm this independently.** The arcade character rips are annotated by row,
and their first four rows are **Idle**, **Advantage**, **Disadvantage 1** and **Disadvantage 2** —
precisely the three `+0x79` values plus the gloat. The art and the code agree on the state list, so
the posture layer can be considered fully specified:

| `+0x79` | board | sheet row |
|---|---|---|
| 0 | below 37 cells | Idle |
| 1 | 37-54 | Disadvantage 1 |
| 2 | above 54 | Disadvantage 2 |
| `0x10` | opponent buried, you clear | Advantage |

The **attack** animations are driven separately and are **[open]**. The inputs are all identified
and set by the erase pass `FUN_80134A28`: `+0x265` total gems destroyed, `+0x266` best chain,
`+0x229` current chain, and the six class flags `+0x237`-`+0x23c`, one per erased gem class in the
8-13 range. Finding the consumer of those flags is the remaining work.

## Confirmed by the art

The arcade sprite rips settled one thing the disassembly had already said, and it is worth
recording because the two are independent sources:

**The playfield frame's top row is hatched tabs over every column except column 3, which is
left open.** That is the Drop Alley - the column pieces enter down - arrived at by measuring
the frame off `Miscellaneous - HUD.png` rather than by reading `DAT_8016E524`. The frame's
interior is also exactly six columns and thirteen rows at sixteen pixels to the cell, which is
the board shape this document derives from the census and gravity scans.

**The gem sheets do not carry per-cell power gem art.** They carry a tiled body texture in four
shine frames plus a top edge row with rounded corner notches, which says the game composites a
border over a tiled fill at draw time rather than indexing a sprite per corner code. That does
not change any rule here - the corner codes are a data structure, not a sprite table - but it
does mean the corner codes are *only* the acceptance rule's arithmetic.

## What is not mapped yet

The economy is complete and validated. The board mechanics are mapped in structure, with these
gaps, in order of how much they would cost an implementer:

* **Does a power gem score more per cell?** All seven accumulators are now identified and **none of
  them pays a power gem premium**: forming one only sets bit `0x80` and stamps the parallel array,
  leaving the class nibble as the plain colour, so every cell still scores 100 through `+0x114`.
  StrategyWiki's 2010 and 3210 cannot be reproduced from this code.

  The propagation is now read too, and it offers no premium either: a power gem is a solid
  same-colour rectangle, so the ordinary colour flood takes all of it, exactly as it would take the
  same gems loose and linked. **On this code a power gem and an equivalent linked blob score
  identically.**

  So either StrategyWiki's 2010 and 3210 come from a different version or are wrong, or there is a
  scoring pass still unfound. Note what a power gem *does* buy that a loose blob does not: it is a
  guaranteed rectangle that survives partial breaks as a unit and is the only structure the merge
  pass can grow. Build it as read and judge it in play — that is cheaper than more reverse
  engineering, and it is the one question left that a playtest answers better than a disassembler.
* **The power gem growth loop**, line by line (see above).
* **Gravity and settling order** — `FUN_80131AE0`, `FUN_8013190C`, `FUN_80131C00`.
* **Game over** — the Drop Alley blocking condition.
* **The attack animation consumer.** The *magnitude* is found — the chain loop adds `chain * chain`
  to the opponent's `+0xfe`, then quarters it with a floor of 1 — but which sheet row that selects,
  and where Minor vs Major Damage is decided, is unread.
* **The collision probe polarity** in `FUN_80130494`, and whether `+0x3c`/`+0x40` are cell or
  pixel coordinates. **Deprioritised** — these only matter for reproducing the PS1 rotation, which
  we have decided not to do; see *Deliberate deviations*. They are worth resolving only if that
  decision is ever revisited.
* **The AI.** Entirely untouched, and a real question for a two-player game.

## Scope note

Every number here is from `SLUS_004.18`. No arcade comparison was made and none is planned; see
*Provenance*. The one thing worth confirming if the opportunity arises is the **board shape**,
since that is the anchor the decision rests on: this code gives 6 columns x 13 rows with one row
of headroom, while Arcade Quartermaster describes the arcade field as "12x6". That is likely a
guide counting visible rows loosely rather than a real difference, but it is the single
discrepancy on record.
