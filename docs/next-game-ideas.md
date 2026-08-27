# Next game ideas

The compendium is built to grow: the engine is genuinely game-agnostic, and the two games in
it are each a crate of rules, themes and an ai sitting on top of `engine::game::Game`. This
file is the record of which game should come next and why, so the reasoning survives the
conversation that produced it.

It is a shortlist and a judgement, not a backlog. When a game is picked, its *how* goes in a
plan document of its own and is linked from the status board below.

## Status board

This board is authoritative. Do not start a second game while one is in progress.

| Game | Status | Plan |
|---|---|---|
| **Puyo Puyo** (Tsu ruleset) | **in progress** | [puyo-puyo-plan.md](puyo-puyo-plan.md) |
| Super Puzzle Fighter II Turbo | queued | — |
| Tetris Battle Gaiden *(as a Rustris ruleset)* | queued | — |
| Bombliss *(as a Rustris ruleset)* | queued | — |
| Everything else below | rejected | — |

## Criteria

Every candidate was judged on five things.

1. **Engine fit.** A game is a board of `Cell`s with a falling piece, driven by
   left/right/rotate/soft-drop/hard-drop/hold, producing `GameEvent`s. Anything outside that
   shape means new engine concepts before the game can be written at all.
2. **2-player battle depth.** Is there a real attack economy, or does the second player just
   have to survive longer? This is the whole point of the compendium's vs. mode.
3. **Distinctness.** Does it add something Rustris and Dr. Rustario do not already have
   between them?
4. **Assets.** Can sprites and music be obtained from a decompilation, a community rip, an
   emulator or rom extraction, or original art we author ourselves?
5. **Mechanics documentation.** Can the rules and the attack maths be *sourced* rather than
   guessed at? The N64 ai port set the standard here: `aiset.c` was read, not inferred.

## Puyo Puyo — chosen

**Plan: [puyo-puyo-plan.md](puyo-puyo-plan.md).**

Puyo Puyo is the canonical 2-player falling block battle game, and it fits this codebase
better than anything else on the list. Its pair piece is mechanically Dr. Rustario's pill —
two halves, rotation about a pivot with kicks, splitting apart when it lands — so the hardest
code in that crate has a sibling to be written against rather than a blank page. What it
brings that neither existing game has is a real attack economy: a chain's score converts to
nuisance puyo through a documented power table, and the **offset rule** lets an incoming
attack be cancelled by chaining back at it. That single mechanic is what turns two people
racing into two people fighting.

The assets are unusually good because of the Western reskins. *Dr. Robotnik's Mean Bean
Machine* (Genesis, Master System, Game Gear) and *Kirby's Avalanche* (SNES) are the same game
with different art, so three retro themes are available in exactly the platform idiom the
compendium already uses, each with mascot art for the throw and victory animations. The
mechanics are the best documented of any candidate: Puyo Nexus records the chain power,
colour and group bonus tables, target points, the nuisance queue and the offset rule
precisely, and there is a work-in-progress Megadrive disassembly plus several open source ai
bots whose evaluation functions are described in the literature.

The cost is a full game crate — the existing two are about 9-10k lines each, of which the art
and audio is the real expense — plus generalising the launcher past its assumption that there
are exactly two games. That work is scoped in the plan.

## The other candidates

| Game | Engine fit | Battle | Assets | Docs | Verdict |
|---|---|---|---|---|---|
| **Puyo Puyo (Tsu)** | Excellent — pair piece ≈ pill | Best in class: chain power, nuisance queue, offset | Mean Bean Machine (Genesis/SMS/GG), Kirby's Avalanche (SNES), arcade | Puyo Nexus documents the formulas exactly | **Chosen** |
| Super Puzzle Fighter II Turbo | Good — pair piece, but needs multi-cell power gems | Excellent and very distinct: crash gems, countdown counter gems, per-character attack patterns | Spriters Resource (arcade CPS2, PS1, GBA) | StrategyWiki and FAQs cover it well | Strong runner-up |
| Tetris Battle Gaiden (SNES) | Excellent — it *is* Rustris plus a gauge | Excellent: crystals on pieces fill a magic gauge, four spell levels per character, offensive and defensive | SNES rip, Japan-only | GameFAQs guides list every spell | **Cheapest win — a Rustris ruleset, not a new crate** |
| Bombliss / Tetris Blast / Super Bombliss | Excellent — tetrominoes carrying bomb cells | Good: completing a line detonates the bombs in it, chains, four small bombs in a 2x2 merge into a big one | GB/SFC/FC rips | TetrisWiki and Hard Drop | **Also a Rustris ruleset, not a new crate** |
| Tetris 2 (1993) | Good, but it is essentially Dr. Rustario | Modest: "fishbowl" vs., clear the flashing blocks | NES/SNES/GB rips | Hard Drop and StrategyWiki | Skip — overlaps Dr. Rustario |
| Columns / II / III | Excellent, trivial to build | Thin: raise the opponent's floor; III adds an attack meter | Genesis/arcade rips | Sparse | Skip — shallow, overlaps Dr. Rustario |
| Panel de Pon / Tetris Attack | Poor — **rising** stack, swap cursor, a new input model | Arguably the best puzzle battle ever made | SNES rip; open source clones to reference | The clones document the mechanics | Premise mismatch — not a falling block game |
| Magical Drop | Poor — descending ceiling, pull and throw | Excellent, blisteringly fast | Neo Geo rips are first-rate | Sparse | Premise mismatch |
| Lumines | Poor — timeline sweep | Weak vs. | Licensed music, no clean route | — | Skip |
| Baku Baku Animal, Yoshi's Cookie, Wario's Woods, Money Puzzle Exchanger | Mixed | Varies | Thin | Thin | Not investigated in depth |
| Hatris, Welltris, Faces…Tris III, Wordtris, 3D Tetris, Tetrisphere, The New Tetris, Tetris Plus | Poor to none | Weak or absent | — | — | Skip — novelty spin-offs with no battle economy |

### Super Puzzle Fighter II Turbo

The strongest alternative, and the most *different* from what the compendium already has.
Gems fall in pairs, gems of a colour merge into rectangular power gems, and a crash gem
detonates every gem of its colour it touches. Attacks arrive as counter gems in a pattern
chosen by the character you picked, each carrying a countdown that ticks down one per piece
you place before it turns into an ordinary gem — so an attack is a timer as well as a mess,
and a crash gem placed against it defuses it early.

Against it: power gems are rectangles spanning several board cells, which the sprite pipeline
does not currently do (`CellSpriteData` is one snip per cell), and the per-character attack
patterns are a content surface with no obvious floor. Worth doing after Puyo Puyo if the
appetite is there.

### The official Tetris variants — the important finding

Tetris Battle Gaiden and Bombliss are **not third games. They are alternate Rustris
rulesets.** Both keep the guideline board, the SRS, the seven bag and hold, and add a layer on
top:

* **Tetris Battle Gaiden** puts magic crystals on some pieces. Clearing them fills a gauge,
  and spending one to four crystals casts that character's spell at that level — stealing the
  opponent's crystals, flattening your own bottom rows, inverting their controls, dumping five
  garbage rows, pushing every block toward the centre. It is Tetris with a fighting game's
  meter, and it is a genuinely excellent battle game.
* **Bombliss** puts bomb cells inside tetrominoes. Completing a row does not clear it —
  it detonates the bombs in it, which blow up the blocks around them, which can set off
  further bombs. Four small bombs arranged in a 2x2 merge into a big one once the chain
  settles.

Implementing either as a `GameConfig` variant inside `rustris/` reuses `board.rs` and
`tetromino.rs` wholesale and costs a fraction of a new crate, while still adding a battle
economy the compendium does not have. Both are worth doing after Puyo Puyo, and worth doing
*before* considering any further new crate.

Tetris 2 is the opposite case. Despite the name it is a colour matching game — irregular
multi-coloured tetromino-ish pieces landing on a field of fixed coloured blocks, clearing
three in a line, with a flashing block per colour that takes the rest of its colour with it.
That is Dr. Rustario's problem space, not Rustris's, so it is a lot of work to arrive
somewhere the compendium already is.

### Panel de Pon and Magical Drop

Both were seriously considered and both fail criterion 1. Panel de Pon's stack rises from the
bottom and the player swaps pairs horizontally with a cursor; Magical Drop's ceiling descends
and the player pulls and throws from a column below it. Neither has a falling piece, so
neither uses the input model, the piece queue, the hold box, the ghost piece or the hard drop
that the engine is built around. They would each need a parallel set of engine concepts.
Recorded here so the next person to have the idea can see it was weighed and why it lost.

## Sources

The Puyo Nexus pages reject automated fetches (HTTP 403) — read them in a browser.

**Puyo Puyo**

- Puyo Nexus Wiki: [Scoring](https://puyonexus.com/wiki/Scoring),
  [Nuisance queue](https://puyonexus.com/wiki/Nuisance_queue),
  [Offset rule](https://puyonexus.com/wiki/Offset_rule),
  [Tsu (rule)](https://puyonexus.com/wiki/Tsu_(rule)),
  [Puyo Puyo Tsu reverse engineering](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Reverse_Engineering)
- [Nasina7/puyodisasm](https://github.com/Nasina7/puyodisasm) — work in progress Megadrive
  disassembly of Puyo Puyo 1
- [Puyo Tools](https://github.com/memerdot/puyotools-1) — extracts Puyo game file formats
- Ai: [citrus610/ama](https://github.com/citrus610/ama),
  [mbrown1413/Puyo-AI](https://github.com/mbrown1413/Puyo-AI),
  [Applying Artificial Intelligence to Famicom Puyo Puyo](https://meatfighter.com/puyopuyoai/),
  and Ikeda, Tomizawa, Viennot and Tanaka, *Playing PuyoPuyo: Two search algorithms for
  constructing chain and tactical heuristics*
  ([IEEE](https://ieeexplore.ieee.org/document/6374140/))
- Sprites: [Mean Bean Machine — Genesis](https://www.spriters-resource.com/genesis_32x_scd/drrobmbm/),
  [Master System](https://www.spriters-resource.com/master_system/drrobmbm/),
  [Game Gear](https://www.spriters-resource.com/game_gear/drrobotniksmeanbeanmachine/)

**The others**

- Super Puzzle Fighter II Turbo:
  [arcade sprites](https://www.spriters-resource.com/arcade/superpuzfightiiturb/),
  [PlayStation sprites](https://www.spriters-resource.com/playstation/superpuzzlefighteriiturbo/),
  rules on [StrategyWiki](https://strategywiki.org/wiki/Super_Puzzle_Fighter_II_Turbo/Gameplay)
  and [Arcade Quartermaster](https://www.arcadequartermaster.com/spf2t_rules.html)
- Tetris Battle Gaiden: [TetrisWiki](https://tetris.wiki/Tetris_Battle_Gaiden),
  [strategy guide](https://gamefaqs.gamespot.com/snes/580018-tetris-battle-gaiden/faqs/61493)
- Bombliss: [TetrisWiki](https://tetris.wiki/Bombliss),
  [Tetris Blast on Hard Drop](https://harddrop.com/wiki/Tetris_Blast),
  [Tetris 2 + Bombliss](https://tetris.wiki/Tetris_2_+_Bombliss)
- Tetris 2: [Hard Drop](https://harddrop.com/wiki/Tetris_2),
  [StrategyWiki](https://strategywiki.org/wiki/Tetris_2)
- Panel de Pon clones to reference: [sharkwouter/panel-pop](https://github.com/sharkwouter/panel-pop),
  [a544jh/panel-pop](https://github.com/a544jh/panel-pop)

## A note on assets

Ripped sprites and music stay copyrighted whatever the extraction method — the same position
the existing NES, SNES, Game Boy and N64 themes are already in. The particle theme is original
art and carries no such risk, which is why a new game should be built so it is complete and
playable on its particle theme alone, with the retro themes as a layer on top.

## Working agreement

Work on this repository is **synchronous. One agent at a time. Never in parallel.**

* **The documents are the shared memory.** Conversations do not carry over between agents;
  these git-tracked files do. An agent picking up work reads this file and then the relevant
  plan document, top to bottom, before touching anything.
* **One game at a time.** The status board above is authoritative. While Puyo Puyo is
  `in progress`, nobody starts Puzzle Fighter, Battle Gaiden or Bombliss — not as a "quick
  parallel branch", not as a subagent, not at all. Two half-finished games in a compendium
  that hardcodes its game count in a dozen places is a mess nobody can land.
* **Finishing a game is not the same as picking the next one.** Move it to `done` on the
  board, then pick the next one *with Alex*, not unilaterally.
* **Where things belong.** This file records *which* game and *why* — the criteria, the
  verdicts, the sources, the status board. A per-game plan document records *how* — its
  phases, their status, and the handover notes between them. Do not duplicate content across
  the two; link instead.
* **Amend, do not append contradictions.** If something here turns out to be wrong, edit it
  and say so. A document that argues with itself is worse than no document.
