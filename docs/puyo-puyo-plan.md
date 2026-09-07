# Puyo Rusto — implementation plan

The third game of the compendium, picked in [next-game-ideas.md](next-game-ideas.md). This is the
shared memory for that work: what is left, the decisions that still bind, and the traps a future
agent would otherwise have to rediscover. **It is not a log.** Anything recoverable from the code,
the tests or a script's doc comment has been taken out — read those for the detail and this for
the *why*.

## Status

The game is built, themed, charactered and animated, and plays on its own menu against its own ai.
The table below is what is left and what has been done; the rows marked `done` are kept for
what they found:

| # | what | where |
|---|---|---|
| 1 | the ai answers what is thrown at it — **`done` 2026-09-04**, kept below for what it found | `puyo-rusto/src/game/ai/` |
| 2 | the defeat drain — **`done` 2026-09-04**, kept below for what it found | `engine/src/animate/game_over.rs` |
| 3 | **vs. integration** — a playlist that picks its games, and six directed attack prices — **built 2026-09-07, one play test outstanding** | `launcher/`, every `game/mod.rs` |
| 4 | audio levels across the whole app — **`done` 2026-09-04**, kept below for what it found | `engine/art/audio_levels.py` |

**Every item is built.** What is left of item 3 is a *play test*, which nobody but Alex can do:
the compendium has never been sat down in front of with three games in the playlist, and the
attack ball has never been watched crossing between a Rustris board and a Dr. Rustario one at
all. The six prices are measured and land where they were aimed; whether they *feel* right is
the thing a measurement cannot answer.

Everything else — the rules, the three themes, their music and effects, the ai's beam search and
its measured ladder, the characters, and how the whole thing moves — is `done`. What is worth
knowing about each of those is under *Traps* below rather than in a phase of its own.

**There is no neural model for this game and there will not be one** (Alex, 2026-09-04). The beam
search over `eval.rs`'s fifteen weights is the ai, the `ga puyo` subcommand trains nothing, and
nothing here is to be shaped around a `Genome`. This replaces the old phase 4 step 2; do not
reintroduce it.

---

## 1. The ai answers what is thrown at it — `done` 2026-09-04

Puyo Rusto's agent now reads `pending_nuisance()`, and it is still the only `game/ai/` in the
repository that reads a tray at all. **Two rules came out of it, and they are not equally worth
having** — the one the plan asked for is right and almost never applies, and the one it did not
ask for does all the work. Both are in `agent.rs` and `beam::ranking`, and `ga puyo duel` is what
told them apart.

### What the ai is asked, and when

The tray is read **at commit**, not at `begin` — and then **again, whenever it deepens while the
keys are still being pressed**. That second read is not a refinement, it is the feature. The
search finishes about a dozen frames after a pair spawns and a fielded opponent takes 300-500 ms
*per key* after that, so an attack essentially always arrives after the ai has already decided;
and classic Tsu offset drops the tray the moment this pair locks, so the pair in play when an
attack arrives is the only pair that can ever do anything about it. Deciding once and looking
away let **95%** of incoming through unseen (9 trays seen in 202 pairs). Keeping the search alive
in `Thinking::Decided` and re-ranking on a deeper tray costs one re-rank and a fresh route, and
takes it to **most of them** (25 in 450).

### The rule that works: fire before the rock lands, not after

`agent.rs`'s `incoming_rows` adds the part of the tray that is *certain* to land — full rows
first, so `min(pending, MAX_DROP) / COLUMNS` — to the spawn column's height, and a board that
would be over `PRESSED_HEIGHT` after it counts as `pressed`. A comfortable board with a rock
hanging over it fires now, because in Tsu there is no *after*.

It fires on **half the trays a row sees** (15 of 25 for one row, 14 of 30 for the other), and both
sides of a duel are better for it: over 24 seeds of `sharp` against `patient` at 400 ms a key,
pairs survived went 450 → 505, chains 18 → 28, nuisance sent 1,489 → 1,595, and the head to head
went 12-12 to 14-10.

### The rule the plan asked for, which is nearly dead code

`SearchConfig::answer_at` is the tray depth at which a row starts looking, and above it the ai
takes the **smallest** chain whose `fires` is at least `skill::nuisance(pending)`, ranked by the
board it leaves rather than by size. Everything about that is right, including the part that
looks wrong: **a partial answer is worth nothing**, because offset cancels against the tray and
then whatever is *still* waiting drops anyway — cover 102 of 132 and all 30 of the next drop lands
regardless. So a row that cannot cover it carries on building.

**It fired zero times in three thousand duelled pairs**, and the dial measures flat from `1` to
`u32::MAX`. The reason is specific and was measured rather than guessed — printing what was on
offer at every tray sighting:

* `Candidate::fires` is what the pair **in play** sets off *right now*, and it is **zero** at
  about six of every ten moments a tray is seen. The board has a chain; the trigger for it is not
  in the hand.
* The trays that actually arrive are big — 26, 31, 32, 35, 38, 39, 56, 62, 97, 126, 127, 132 — so
  covering one costs 1,800 to 9,200 points, where what is triggerable at that moment is 0 to
  1,000. The one sighting that could have covered its tray (38 pending, 5,440 available) fired
  through the `trigger` branch anyway.
* The rest are `pressed` already.

It is kept because it is correct and costs nothing at runtime, and because the numbers above are
worth having written down rather than rediscovered. **Do not read a low `answers` column as a
bug.**

### `ga puyo duel`, and the ladder it found

`ga puyo duel [seeds] [pair cap] [difficulty] [key delay ms] [a] [b]` — two rows on one seed
(so the same pairs: it is a paired comparison), each sending the other what its chains buy,
delivered only after both boards have stepped, the way the match screen routes it. Name both
brains for a head to head, neither for every row against every other. A brain can be written
`sharp@12` to move that row's `answer_at` without rebuilding, which is what a sweep is made of.
The key delay matters and defaults to zero: a fielded opponent presses keys at 300-500 ms and the
answering window is a different shape there.

**The ladder under fire roughly reverses at the top.** `swift` wins nine duels in ten; `sharp` —
what the marathon calls the best row, and what `hard` fields — loses more than it wins. A marathon
rewards patience and a fight rewards tempo: holding out for a chain worth forty-eight nuisance
takes a dozen pairs and a row throwing twelve every four buries you inside them. The ordering is
the same at 0 ms and at 400 ms a key, so it is not an artefact of one row thinking faster than
another. The table is in `SKILL_ORDER`'s doc comment.

**`SKILL_ORDER` was re-measured and deliberately not changed.** `ga puyo rank 12 600` reproduces
its old table to the decimal, which it must: a solo marathon takes no nuisance, so nothing in this
item can touch it. Making the difficulty ladder the *duel* ladder instead would make `swift` the
hardest opponent and change what all four dials mean — a gameplay decision, and Alex's.
**This is the open question item 3 inherits**, since item 3 is where the Puyo half of
`Difficulty::level` is set.

## 2. The defeat drain — `done` 2026-09-04

When a player is buried, **every puyo on the field pauses, then falls straight down and off the
bottom of the screen, column by column at different offsets** (Alex, 2026-09-04). That is Puyo's
own game over and it is what **all three themes** play, the particle theme included. It is
`GameOverStyle::Drain` in `engine/src/animate/game_over.rs`, beside `Screen` and `Curtain`, and
what it replaced was a `Curtain` with `curtain_cell: None` — a burial was a blank three-second
pause on two of the three themes.

**Measured off Alex's capture** of Kirby's Avalanche (`Screencast From 2026-09-04 13-10-09.mp4`,
4.1 s at ~30 fps, a stage theme this repo does not use — the mechanic is the game's, not the
theme's). Tracked by cross-correlating each column band of every frame against the first, which
gives that column's displacement to the pixel. The numbers are the constants at the top of
`game_over.rs`:

* **Each column falls as one rigid block.** Correlation stays above 0.9 at a single offset the
  whole way down, so the gaps in a column are carried with it: nothing re-settles, nothing
  compacts, and a hole in the middle of a column is still a hole as it leaves the screen. That is
  free in the renderer — the offset is keyed on the column and nothing else.
* **Every column has its own start, and the order is scattered rather than a sweep.** The six
  columns started at 0.00, 0.00, 0.03, 0.08, 0.29 and 0.35 s — in column order 2, 3, 1, 4, 5, 0.
  A spread of **at least 0.35 s**, and neighbours deliberately unalike. `Drain::delay` is
  `animate/nuisance.rs`'s golden-ratio hash of the column index rather than an RNG, so the same
  board drains the same way twice and nothing random reaches the render path. It leaves in the
  order 0, 5, 2, 4, 1, 3, which is **not** the capture's order and is not meant to be: what was
  measured is the spread and the scatter, and reproducing one console's exact permutation would
  cost a table for nothing.
* **It accelerates, and does not reach a terminal speed inside the board.** Fitting the four
  columns whose start is inside the capture gives 2226-2554 px/s² at a 76 px cell, so
  **≈30 cells/s²**. A column clears the 12 rows in ~0.9 s and, with the stagger, the whole field
  is gone **1.3 s** after the first column moves — which a test in `game_over.rs` pins, since it
  is the one number that says the drain still looks like the capture.
* **Puyos are clipped at the field's bottom edge** — they slide under the frame, cut off mid
  sprite, rather than being drawn over the panel below. The clip is set in `Theme::draw_board`
  around the whole sprite-sheet call rather than inside its cell loop: that loop returns on `?`
  in a dozen places and every one of them would have to put the clip back.
* **The hold before the first column moves is not in the capture**, which opens mid-fall. 0.3 s,
  chosen and not measured: short enough to read as a pause and not as a hang.
* **The winner's board is not draining** in this capture: it is already empty and is showing a
  blinking `YOU WIN` plaque with celebration confetti rising through it. Whether the winner's
  own field drains too is unevidenced — the defeated board drains and only it, until someone
  captures otherwise.

Three things the build found:

* **The drain runs on its own clock, and nothing was gating the stage on the loser anyway.**
  `is_complete` is the hold, the stagger, the fall and a 0.5 s beat — about 2.15 s, where every
  other style is a flat 6 s. It changes nothing downstream because
  `is_all_post_game_animation_complete` waits on the *winner* too and a victory is 10 s, so the
  loser's clock has never been what ends a stage. It also means a drain is over before
  `GAME_OVER_SCREEN_DELAY`, so it can never be dismissed by a held key — there is no card to skip.
* **Stack and garbage cells are the whole board at that moment.** `next_pair` tests the death
  square *before* it spawns, so a lost Puyo board never has an active pair on it, and the drain
  needs no arm for one.
* **`GameOverStyle` lost its `Eq`**, since the drain carries an acceleration. Nothing compared
  two of them.

`animation_shot` grew a **`scene` argument** for this: `drain` buries player one on the spot, so
the fall can be watched beside a board still in play. It is the only way to see this without
losing a real match, and it is how all three themes were checked.

## 3. Vs. integration — built 2026-09-07, one play test outstanding

The playlist now picks its games, the mode has stopped ranking, and all six crossings are
priced off a measurement. **What is not done is playing it.**

### The playlist picks its games

The top-level menu entry is **`vs. playlist`**, renamed from `dr. rustario vs. rustris`, and its
menu carries a tick per game between the playlist row and the difficulty dial — every one on to
start with, and **the last one cannot be turned off**. Puyo Rusto is in `PLAYLIST_ORDER`;
anyone who wants the old two-game compendium unticks it.

* The selection is a `GameSelection(PerGame<bool>)` on `VersusMode`, and what it hands out is a
  **`Dealt`** — `PLAYLIST_ORDER` filtered, in turn order. `PLAYLIST_ORDER` is the turn order and
  the default selection and **is no longer read by anything that deals a stage**: `PlaylistThemes`
  carries a `Dealt` and `slots`, `stage_count`, `first_game`, `fixed_stages` and `random_game` all
  narrow together. `GameKind::PLAYLIST_COUNT` is gone with it — what a playlist deals is a value,
  not a constant.
* **A checkbox is a `select_list` of `on`/`off`**, as planned, and unticking the last game is
  *refused* rather than prevented: the menu redraws its rows from the mode after every pick, so
  the row snaps back to `on` on its own.

**A playlist is as long as the *longest* theme list, not the shortest** (Alex, 2026-09-07), and a
game with fewer themes replays its own from the start. This is the one thing the plan had wrong.
Puyo Rusto has three themes where the other two have four, so the old rule — "only as long as the
shortest of the lists" — would have quietly stopped dealing Dr. Rustario's and Rustris's *particle*
themes the moment Puyo was ticked, and taken N64 and Rustris's SNES out of the retro marathon:
a theme sprint that is not every theme. Longest and shortest are the same number whenever the
lists are, which is every playlist there was before Puyo joined, so nothing moved seed for seed.
The fallback to every theme is now `covers_every_game` rather than `slots() > 0`, since under a
max a game with none of the family no longer empties the playlist.

### It does not rank, and `MatchSettings` carries an `Option`

As decided: `Mode::high_score_key` returns `Option<HighScoreKey>` and `all_high_score_keys` is
empty for `VersusMode`, which drops it out of the high score screen on its own.
`MatchSettings::high_score_key` and `Match`'s `high_scores` are `Option` too, so a match with no
key loads no table and `maybe_set_game_over` returns before it can name a `NewHighScore` — the
name entry screen is unreachable from a playlist. `Playlist::ranking` went with it, having no
caller left.

### The six prices, and how they were measured

**`ga cross` is the harness** (`launcher/src/cross.rs`, `cargo run --release -- ga cross`), and
the launcher is the only place it could live: pricing an attack between two games means seeing
all three. It plays each game's own ai alone — a fielded opponent, at that opponent's own key
delay — for five seeds of up to ten minutes, and keeps **every attack as the sender priced it**,
`strength` at home and `strength_for` abroad. That is the shipped table being measured rather
than a number worked out on paper, so re-running it after a change re-reads the change.

What it prints per crossing is a **share of home**: what a player faces from a foreign opponent
divided by what they face from an opponent of their own game. 1.00 is a foreign opponent pressing
exactly as hard as a home one.

| sender | receiver | what crosses | share |
|---|---|---|---|
| Dr. Rustario | Rustris | a row per pattern past the first, cap 4 | 0.24 |
| Dr. Rustario | Puyo Rusto | three nuisance for each of those rows | (see below) |
| Rustris | Dr. Rustario | 2 blocks for a tetris or T-spin double, 3 for a triple, 4 for a perfect clear | 0.79 |
| Rustris | Puyo Rusto | the same clears, at a row of nuisance a block | (see below) |
| Puyo Rusto | Dr. Rustario | a block per two rocks of nuisance, cap 4 | 0.49 |
| Puyo Rusto | Rustris | a row per two rocks of nuisance, cap 4 | 0.23 |

**The two crossings that already shipped are the calibration.** Dr. Rustario into Rustris reads
0.24 and Rustris into Dr. Rustario reads 0.79 — a three-fold spread between two prices that have
been played and are right, which is worth knowing before trusting any single number. The two new
crossings out of Puyo were aimed between them and landed at 0.49 and 0.23, either side of the
pair they sit beside.

**The table moved once the ai did**, which is the point of measuring the shipped code rather
than a model of it. Making the Dr. Rustario agent soft drop to its tuck waypoints took it from
2.36 attacks a minute to 2.98 — a quarter more, from nothing but placing pills faster — and
three of the six shares moved with it (0.19 → 0.24, 0.99 → 0.79, 0.61 → 0.49). Nothing needed
re-pricing; the spread got *tighter*. **Re-run `ga cross` after touching an agent**, not only
after touching a price.

**Four things it found that argument would not have.**

* **A Puyo player alone throws four boards of nuisance a minute** — 327 nuisance where Rustris
  manages thirteen rows and Dr. Rustario six blocks. It is a chain-building paradise: nothing
  arrives, so nothing offsets, and `sharp` holds out for monsters (the biggest single chain in
  fifty minutes was worth 449). Priced at parity a single chain would bury either of the others,
  which is why what ships is a *thirtieth* of parity and why the routine two-chains — most of what
  was counted — cross as **nothing at all**.
* **Puyo's own rate is therefore useless as a yardstick for attacks going the other way.** A share
  of 1.00 into Puyo would be a hundred nuisance a minute onto a 72 cell board. The crossings
  *into* Puyo are read as board filled per minute instead, against what the same sender delivers
  to the game it already crossed to: Dr. Rustario fills about a tenth of a Puyo board a minute
  and Rustris about a fifth, where Dr. Rustario fills a tenth of a Rustris well. **This is the
  asymmetry the plan predicted**, and it is bigger than it looks on paper.
* **"A four-chain is roughly the work of a tetris" is refuted by the rate.** A Puyo four-chain
  sends about thirty two nuisance, and a Rustris player lands a tetris-or-better two and a half
  times a minute — that pairing is a board of nuisance a minute and a Puyo player buried inside a
  stage. A tetris ships as **one row** of nuisance, a fifth of that.
* **Rustris's own ai is buried in every configuration measured** — at the mid difficulty dial it
  lasts two to three minutes, and at `normal` on four of five seeds it lasts twelve seconds. Its
  home rate is per minute *played*, which is the right denominator, but it means the 0.99 that
  Rustris-into-Dr.-Rustario reads is a ratio of two small numbers. Worth remembering before
  reading that column as precise.

The crossings are priced in each game's own `game/mod.rs` beside the ones that were already
there, and `every_crossing_between_two_games_is_priced` in `launcher/src/games.rs` is the guard:
the game crates are siblings and none of them can see the whole table, so that test is the only
place all six can be checked at once. **A forgotten pair is worth nothing and drops silently** —
nothing arriving looks exactly like nothing being sent.

### The Puyo half of the difficulty dial

**One speed step per dial step and nothing else** (Alex, 2026-09-07). The colour count and the
starting rows of nuisance stay at Puyo's own `rules::Difficulty::default()`, so the vs. dial
changes how fast a Puyo match plays and never what it deals. The fall curve has twelve steps, so
a dial of 10 is inside it — past `rules::MAX_START_LEVEL`, which is what Puyo's own menu offers
rather than a bound on the game.

**`SKILL_ORDER` stays the marathon's ladder** (Alex, 2026-09-07), which closes the question item 1
left open. `hard` goes on fielding `sharp` even though `sharp` loses more duels than it wins; the
ladder is right for the three single game modes, which is where most people meet it, and the duel
measurement stays written down in `SKILL_ORDER`'s doc comment.

### What is left

* **The play test**, which is Alex's: three games in a playlist, the ticks, and whether the six
  prices *feel* the way they measure. The attack ball has never been watched crossing between a
  Rustris board and a Dr. Rustario one either, and the same sitting closes that.
* **Margin time is still unbuilt**, and is the knob to reach for if matches drag. Sourced, not
  invented: from 96 s, target points go to 3/4 and halve every 16 s, at most 14 iterations or
  until they reach 1. It makes every chain send more as a match wears on, which is what an endless
  playlist needs and what nothing else here provides, and it lands *on top of* the speed ramp
  rather than instead of it. Nothing measured so far asks for it — every crossing is tuned *down*
  and a long playlist is exactly where that would show — so it is the first thing to try if the
  play test says matches go on too long.
* **`ga cross` measures a solo board**, not a fight. A duel between two different games would
  measure what each side *nets* after the other's pressure, which is the number a really careful
  tuning would want; `ga puyo duel` does that within one game and there is no cross-game
  equivalent. It was not needed to land the six prices in the band and it would be the next tool
  if they turn out wrong.

## 4. Audio levels — `done` 2026-09-04

The whole app is now levelled to one **house baseline**, which is written up in CLAUDE.md and
read back by `engine/art/audio_levels.py` — the meter to run after cutting *any* new audio. What
was found, since it is the sort of thing nobody would think to look for again:

* **Puyo Rusto was eight decibels louder than the rest of the compendium.** Its music sat at
  about −14 dBFS RMS where every theme of Rustris and Dr. Rustario sits at −22, and its effects
  with it. Nobody had heard it because Puyo takes no playlist turn yet, so no other game's tune
  ever follows one of these — the moment item 3 lands, it would have been the loud one. Fixed
  with one gain per theme (`GENESIS_GAIN`, `SNES_GAIN`, `PARTICLE_GAIN`, `MENU_GAIN` in
  `theme/data.rs`), applied to music and effects **together** so each theme's own mix survives.
* **`genesis`'s effects were levelled on peaks and are now levelled on RMS.** `retro_audio.py`
  matched each cut's peak to the particle theme's sound for the same slot; peak is one sample,
  and Mean Bean Machine's effects are far denser at the same peak (`lock` carries three times the
  RMS), so the set measured right and sounded hot — and carried eight decibels of spread the game
  does not have. `slot_gain` now matches RMS with the peak as a **cap**, which is a `--only sfx`
  re-run. It took the set from +3.4 dB against its own music to −0.7.
* **Rustris's particle theme was the other outlier**, five decibels *quiet* against its own tune
  (−6.9 where the house is −2.0). Lifted with `with_effects_at(176)`, which scales the decoded
  samples because the config's volume dial has no headroom above it; the bound is `stack-drop`'s
  peak, which lands at −3.2 dBFS.
* **Every theme now reads in band**: music −20.8 to −23.0 dBFS, effects −4.0 to +1.9 against it.
* **One thing was left alone**: `dr-rustario/nes`'s `fever-next-level-repeat` peaks at +1.0 dBFS
  in the file, because that is how it was mastered. Trimming the whole theme to fix one track's
  peak costs more than it buys, and the mixer sums music at half volume by default, so it does
  not clip in play.

Two rules came out of it and both are in CLAUDE.md: **never level a set slot by slot on peaks**,
and keep the two knobs apart — `with_gain` is a theme against the app, `with_effects_at` is a
theme's effects against its own music.

**The `snes` effects stay levelled a third way**: they came from a recording of the running game,
so what they hold *is* the mix — 14.4 dB of it — and the set moves as a set at one gain, bounded
by not letting its loudest sound pass the loudest the particle theme plays. Levelling that set
slot by slot flattened its mix and handed `move`, which fires on every frame of a held direction,
a +22 dB lift. `set_gain`'s doc comment writes it up.

**The seven inferred `genesis` slots are left alone** (Alex, 2026-09-04). The rip names only the
sounds whoever made it recognised, so `rotate`, `lock`/`settle`, `garbage`, `attack`, `pause` and
`hard-drop` are each a reading rather than a hearing, and RetroArch will not stay up long enough
on this machine to trigger them in game. They sound right; that is the end of it.

**Re-cutting churns the diff even where nothing changed** — libvorbis restamps each Ogg's serial —
so compare decoded PCM and `git restore` the files that did not actually move.

---

## Decisions that still bind

* **Faithful Tsu between two Puyo players** — the real chain power, colour and group bonus tables,
  target points, the nuisance queue and classic offset. Cross-game attacks are tuned *down* and are
  measured, not guessed.
* **`GameId(3)`, declared as `engine::game::ids::PUYO`.** Game ids live in the engine because a
  game pricing an attack has to name the game it crosses to, and the game crates are siblings.
* **The exact tables are Puyo Nexus's.** [puyo-nexus-rules.md](puyo-nexus-rules.md) is a local copy
  of every page carrying a rule — search it first, and search for the *mechanic* rather than the
  page you expect it on (the ghost-puyo row was got wrong first time because that rule is filed
  under *Gameplay Guides*). The live wiki is the authority and rejects automated fetches, so ask
  Alex to fetch a page rather than scripting it.
* **A `CellId` carries the colour and the link mask together**, so the board recomputes masks after
  every lock, pop and settle. The falling pair and every ghost draw unlinked; nuisance never links.
* **No hold; hard drop stays.** Tsu has neither. `hold()` is a no-op and a Puyo board shows no hold
  box; hard drop stays because the engine's input model, every pad mapping and the ghost piece are
  built on it.
* **The colour count is fixed for a whole match** and is *not* driven by `speed_index`. Stages
  advance per player while the colour stream is dealt from one shared seed to independent
  randomisers, so anything that changes what is *dealt* mid-match desynchronises two players who
  reach the change at different moments. The general rule for the games after this one:
  **`speed_index` may change how a game feels, never what it deals.**
* **Cross-game garbage arriving at Puyo joins the nuisance queue** — visible, offsettable, dropping
  when the chain finishes — rather than applying immediately the way the other two take a hit.
  Offset is the identity mechanic and it would be strange for it to work against one opponent and
  not another. It makes a tetris less frightening than its raw number suggests, which the pricing
  has to account for.
* **The speed ramp stays** (Alex, 2026-08-27). Tsu has no level that climbs with play, so
  `PUYOS_PER_STAGE = 30` and the twelve-step fall curve are an invented house rule — kept in single
  player and versus alike, because the whole compendium's mode structure is built on stages and a
  third game that opted out would cost the level sprint, the stage clear card and the speed band
  scenes their meaning here. Margin time lands *on top of* the ramp. **Nothing revisits this.**
* **Themes are `genesis`, `snes`, `particle`, oldest first**, which is the theme sprint's order and
  the retro playlist's. Every theme is named for its platform, as everywhere in this repo.
* **`pair.rs` is a sibling of `pill.rs`, not an extraction from it.** The two pieces rhyme but the
  kick tables, the quick turn and what happens to the halves all differ; a shared engine pair-piece
  would be all parameters and no substance.
* **No neural model, ever** (see *Status*).
* **The vs. playlist does not rank** (Alex, 2026-09-07). It has no high score table and never
  offers name entry; the three single game modes keep theirs. See item 3 for what that is instead
  of, and for the two problems it makes go away.
* **A playlist is as long as the longest theme list, and a short game wraps** (Alex, 2026-09-07),
  so that joining a game with fewer themes cannot take another game's themes off the playlist.
  See item 3.
* **The vs. difficulty dial is a speed step to Puyo Rusto and nothing else** (Alex, 2026-09-07):
  not the colour count, not the starting nuisance. `speed_index` may change how a game feels,
  never what it deals — and a vs. match must not hand one game a handicap the others do not have.
* **`SKILL_ORDER` stays the marathon's ladder** (Alex, 2026-09-07), even though the duel ladder
  roughly reverses at the top. See items 1 and 3.
* **The crossings are measured with `ga cross` and tuned down from parity**, never guessed, and
  the two directions across a Puyo border are priced separately because offset makes them
  different problems. See item 3.

## The art, and the rule about it

**No rip is in the repository and none will be.** They sit under their *verbatim* download names in
`puyo-rusto/art/` and `art/retro/`, each named by full path in the root `.gitignore` with a comment
saying which script reads it. Nobody renames a source file to something tidier — the name is the
provenance. Alex downloads them; the agent writes the cutter.

| script | what it cuts |
|---|---|
| `art/rip.py` | the particle theme's puyos (`check` writes an alignment board) |
| `art/rip_retro.py` | `genesis` and `snes` sheets, panels, fonts, vignettes, animation strips (`check` likewise) |
| `art/retro_audio.py`, `music.py`, `sfx.py` | music and effects |
| `art/mugshots.py`, `kirby.py` | the characters; both print the Rust table to paste back |
| `art/sprites.py` | the procedural puyos the rip replaced, kept as a description of what the sheet must contain |

Each script's doc comment carries its own archaeology — how the link variants are *found* in a
sheet that has no grid, how a loop point is measured rather than read, how the SNES layer switch
(`$212C` in a savestate) renders a background layer on its own, where the `snes` effects came from
and what had to be read out of the audio rather than heard. Re-run a script rather than editing its
output.

**The meter for anything this cuts is `engine/art/audio_levels.py`**, which lives in the engine
because the levels are the whole app's rather than this game's — run it after any audio change and
see item 4.

**Retro geometry was measured against the emulated game, not read off the rip**, and the rip was
wrong every time the two disagreed. The numbers live in each theme module beside a comment saying
what they were measured from. Keep doing it that way.

**RetroArch, if you drive it:** `--set video_vsync=false` or the session hangs a few seconds in
with no error; `--set savestate_file_compression=false` or a state is `#RZIP` rather than a plain
file. `video_driver=sdl2` does not start in this build. Both cores publish no memory map. Sessions
still die after a while on this machine — do not plan a long emulator drive.

---

## Traps

### The rules — `game/`

`mod.rs`, `board.rs`, `pair.rs`, `nuisance.rs`, `score.rs`, `random.rs`, `rules.rs`, `cell.rs`.
Every module's doc comment names the Puyo Nexus page it came from, and
`a_three_chain_scores_and_sends_what_the_published_table_says` is what makes "faithful" checkable
rather than asserted.

* **Top-out is the death square**, not a blocked spawn: the game is lost when a puyo comes to rest
  on the spawn point.
* **The ghost row does not pop.** `Board::grouping_color` reports a thirteenth-row cell as having
  no colour, so a group of four with one member up there does not pop *at all* — the other reading
  would fire the chain immediately and there would be no technique to speak of. Tsu's ceiling falls
  out of there being no fourteenth row, and a rotation whose pivot is in a ghost row is **refused
  outright** rather than kicked.
* **The quick turn is a swap, not a search.** The pair keeps the same two squares and the halves
  exchange places, which is why the rotation cannot fail.
* **Chain power is the multiplayer table, in one player as well as two.** One table is one
  behaviour to test. A solo marathon therefore scores lower than the arcade would have shown;
  swapping the single-player curve back in for one-player modes is small and contained.
* **The all clear pays out on the *next* chain**, not the one that earned it.
* **The event grammar is Dr. Rustario's combo grammar**: one `GameEvent::Clear` per chain *step*,
  `is_combo` false on the first and true after, a `Settle` between steps. That is what the particle
  field, the clear wave and the words are already listening for, so none of them has to learn what
  a chain is. `detail` carries `ClearDetail { chain, all_clear }`.
* **`clear_class` grades 0..3 and 3 is reserved for the biggest clear**, or the particle field's
  silhouette interrupt never fires.
* **`Difficulty` is the game's own five settings** (colours, starting rows of nuisance, a speed
  bonus on the hardest) — *not* the four ai difficulty names, which are a different thing wearing
  the same word.
* **Deliberate gaps, all sourced:** the soft drop bonus is out, because no page gives the points
  per cell and implementing it means guessing (`GameEvent::SoftDrop` is emitted if it is ever
  wanted); soft dropping onto a blocked cell locks immediately in Tsu and this uses one
  `LOCK_DELAY`; and **the nuisance scatter is a documented guess** — Puyo Nexus lists the
  distribution algorithm as an open question on its own reverse-engineering page, so full rows
  first and the remainder over distinct columns honours the sourced parts and guesses only the
  undocumented one.

### The particle theme

* **The skins.** The sheet carries several usable sets of the same puyos and `PuyoSkin::deal` hands
  each player a different one off the **match seed** — off the seed so a playlist swapping a board
  onto Puyo mid-match returns the puyos that player already had. A set earns its place only if four
  in a row read as **one mass**, which nothing shows a cell at a time: `rip.py check` draws a whole
  board per skin and is the only way to judge it.
* **The sheet is laid out against a texture limit.** One band per skin in a single column stood it
  past the 4096 `MAX_ATLAS_WIDTH` a GLES driver will allocate, so on a handheld the theme could not
  be built at all. `BANDS_ACROSS` in `rip.py` and `skin_block` in `theme/modern/mod.rs` are the
  only two places that know the layout.
* **There is no pre-built bank of alpha variants.** It was a whole copy of the atlas per fade step,
  roughly 106 MiB for one skin. The atlas is one texture in a `RefCell` with the fade applied at
  draw time.
* **The HUD is the score and nothing else** (Alex, 2026-08-27). A chain is a thing that *happens*,
  so it announces itself over the puyos through `engine::animate::popup` — one popup per clear on
  its own clock, drawn **last of all on the window** after the foreground particles, because drawn
  into the board texture it lands under the clear's own particle burst.
* **A popup is drawn in the colour of what popped**, and `BlockSpriteSheet` works that colour out
  by reading its own built atlas — averaging each cell's sprite weighted by saturation × brightness
  so an outline and the white of a puyo's eyes do not wash it out. The game cannot say (it knows a
  `CellId`, not what a theme paints it) and a theme cannot be asked to declare eighty of them.
  `PopupSpriteData` then lets a theme spell a caption out of its own art in *tokens*, falling back
  to the plain face for anything it cannot spell, whole rather than in part.
* **`ModernThemeOptions::visible_rows` counts the buffer rows in** — `ROWS` (13), not
  `VISIBLE_ROWS` (12). Passing 12 draws a playable row above the frame. Easy to get backwards.

### The retro themes

`genesis` (Dr. Robotnik's Mean Bean Machine) and `snes` (Kirby's Avalanche) — the two western
reskins of one Compile original, so both boards are exactly this game's board and both rips carry
the sixteen link variants already.

* **A third theme, `3ds` (Puyo Puyo Chronicle), was built and then cut** on 2026-08-28: modern art
  in a retro slot, and — the one that cost something — **its panel sized the board for every other
  theme**, since every theme of a game is drawn at the largest cell all of them can hold. `git log`
  has it. `SceneType::Cover` was written for it and is kept. If the slot is ever filled again it
  wants something 16-bit whose sheet carries the frames a bean needs to pop.
* **A retro theme's background needs a hole in it.** The board frame is drawn *under* the
  background, so a panel carrying its own well covers the board and every cell on it — a perfect
  empty field with the queue, tray and score all correct beside it, which is a very convincing way
  to look broken. `rip_retro.py` punches the hole and each theme has a test that the hole and the
  board agree.
* **`board_snips` are into the *padded* board texture** — add `top_padding` to their height or the
  bottom row is left outside the copy.
* **All skin slots key to the same art.** `data::cells` walks `PuyoSkin::all()` and a retro theme
  hands back the same points for every slot, paying only for duplicate keys. So on a retro theme
  both players draw the same puyos, because the original drew one set. That is not a bug.
* **The thirteenth row is open and has nothing behind it.** Closing it was tried first, at Alex's
  ask, and reversed on sight: the row above the field is *played in*, and hiding it reads as a bug
  the moment a board fills. Both panels are cut level with the top of their own field and the
  thirteenth row is `top_padding` above panel and board alike. The happy accident is that **a point
  in either padded background is a point on that console's screen**, which every coordinate in both
  themes now relies on.
* **The scenes are vignettes** (Alex's pick of five candidates) — a 96x54 png of the backdrop's own
  colour drawn through `SceneType::Cover`, smooth at 4k for a couple of kilobytes — **and the panel
  casts a shadow on them**, which is what lifts the panel off the scene. `PanelShadow` falls down
  and right only, takes a `margin` on all four sides, is composited rather than painted into the
  art (a margin round the art comes straight off the cell size), and does **not** move with the
  hard-drop ricochet, which offsets the board inside a panel that has not moved.
* **Panel size is the whole board's size.** Two levers, both in `genesis/mod.rs` and mirrored in
  `rip_retro.py`: `SIDE_TRIM` and `BOTTOM_PADDING`. Alex took `(8, 4)` and 8 rows. Past eleven rows
  the height binds and the cell comes down with it.
  `the_panel_art_stops_where_the_trim_says_it_does` reads the columns off the png and is the only
  thing that can hold art and constant together.
* **The music counts differ and that is fine.** `genesis` has four stage tunes and `snes` three;
  `GAME_MUSIC_TRACKS` is gone, because the count is each theme's own and a deal is never made
  across two tables.
* **Levels are a house metric and the meter is `engine/art/audio_levels.py`** — see item 4 and
  CLAUDE.md. Inside this game, `music_gain` lifts each console dump towards `src/theme/music/`'s
  level and is bound by **headroom** rather than level in both cases; `EFFECTS_TRIM` (71%, −3 dB)
  and each theme's own gain live in Rust and not in the files, because `sfx.py`'s rip is not on
  this machine, so the particle theme's set cannot be re-cut and every other theme is levelled
  against it as it is.

### The characters

`genesis` deals every player one of thirteen Mean Bean Machine faces; `snes` stands Kirby in the
arch. Both are dealt off the match seed the way `PuyoSkin` is, and both are reviewed *moving* —
`character_shot` and `kirby_shot` are the harnesses. The per-character reading lives in
`art/mugshots.py`, `art/kirby.py`, `genesis/mugshots.rs` and `snes/kirby.rs`.

* **The box holds the *player's own* face**, not the opponent's — the same move the two `NEXT`
  boxes already made.
* **Four states, read per player from that player's own board**: `idle`, `winning` (a chain, or a
  won match), `losing` (nuisance waiting, or the stack high), `defeat`. `winning` beats `losing` —
  a player who chains while buried is *answering* the nuisance. `defeat` and victory are terminal,
  and defeat is held for the rest of the match (see item 2).
* **A single pop does not count as `winning`** (Alex, 2026-08-28): `Clear` fires once per chain
  step, so a one-step clear would enter it several times a minute and sends nothing. Two or more.
* **Three rules stop it flip-flopping**, and all three are needed: a `MIN_DWELL` that holds a state
  whatever happens short of game over; hysteresis on the height trigger (`DANGER_ENTER` /
  `DANGER_LEAVE` with a real gap, or a stack sitting on one threshold strobes as each pair locks);
  and a `LINGER` that outlives the last `Clear`, refreshed rather than restarted by each step.
* **An overlay and a palette cycle are the same thing** — one `Layer`: a small sprite at an anchor
  in box coordinates on a clock of its own, whose variants merely happen to be recolours. So
  palette cycling stays out of the renderer. A layer's strip is per *row*, and a row with zero
  frames is a layer not drawn in that state at all; a cycle is cut as **only** the cycled pixels.
* **A cycled element is rarely only the cycled colour** — a light has a halo, and the halo cycles
  with it on the hardware and cannot here. So the portrait frame under a cycle layer has to sit at
  the *same* cycle phase as every other frame the sheet was drawn at. **A layer's strip looks
  correct on its own when it is wrong**: check a cycle in the composite, never in the cut art.
* **The sweat belongs to nobody.** One authored drop, one emitter, appended to every character,
  gated twice — the `losing` row says whether they are worried at all and `stack_danger` says how
  much (threshold 0.55, above `DANGER_ENTER`, because the sweat comes on *later* than the losing
  face). Its threshold and velocity are measured; its rate is eyeballed and nobody has minded.
* **The danger flash is deliberately not implemented** (Alex, 2026-08-29). Every character goes
  white in bursts near death, but it is the whole *sprite plane* brightening — character and puyos
  together, while the wall and the labels do not move — so it is a palette flash, not an animation.
  Recorded here so nobody adds it back thinking it was missed.
* **`FrameAnimationType::YoYo` is the wrong shape and nothing here uses it.** It runs `0..n` and
  back, repeating each end (`0 1 2 2 1 0`), where every ping-pong measured off the game holds each
  end once. A ping-pong is cut unrolled and played as a plain `Linear`. And **`LinearWithPause`
  gives its last frame a whole frame of its own *and then* the pause**, so a pass is
  `n/fps + pause` — and it **holds the last frame**, so a row is cut *action first, rest last*. The
  sheet's order is not the play order.
* **Emitters are drawn on the window**, not into the panel, since their particles leave the box.
  Speeds are declared in **box pixels an axis per 60 Hz frame**, the unit every capture was
  measured in, and converted in exactly one place.
* **Mirroring reaches three things**, not one: the portrait, a layer's anchor and a particle's box
  x. Nothing else in the compendium flips a sprite.
* **A face's texture is built when it is *dealt***, out of a `RefCell<HashMap<_, _>>` behind the
  `&self` draw, so most of the cast is never turned into a texture. One png per character rather
  than one sheet for the cast is what makes that worth anything. The deal lives on
  `PlayerAnimations`, not on the theme, because `draw_board` is `&self` on a `&'static` theme —
  the same constraint that put `PuyoSkin` on the `CellId`, arrived at from the other end.

### How it moves

Read off two captures of Mean Bean Machine, pixel by pixel. Five engine primitives and two retimed
ones; three of the five are *decoration* in the sense `popup.rs` establishes — the board carries on
underneath them, so they stay out of `blocks_tick()` and change no gameplay timing. Only
`animate/destroy.rs`'s pop blink and `animate/nuisance.rs`'s fall block a tick.

* **A landing is a new event, `GameEvent::Landed { cells }`, not a change to `Settle`.** `Settle`
  fires once for a whole board and only when something *moved*, so a pair landing flat produces
  none at all. An event is data on the wire, so it needs no `AnyGame` delegation and no pinning
  test — the decisive advantage over a `GameRender` method, and the pattern CLAUDE.md asks for.
* **Debris is measured in board cells and is unbounded** — a droplet leaves its cell and often the
  board, which is why it is drawn on the window rather than into the board texture. Whether a
  droplet thrown from an edge column strays onto the panel's stonework is one constant
  (`POP_DEBRIS.speed`) away from being fixed if anyone ever minds.
* **The attack ball belongs to no player**, so it lives on `ThemeContext` and is drawn unclipped. A
  flight is held in cells and player numbers, never pixels, and both ends resolve through whichever
  theme each player is on *at draw time* — so a theme change mid-flight moves the endpoints rather
  than leaving the ball flying to where a board used to be. Its colour is the **sending** player's,
  and it aims at `PendingLayout::origin`, because `draw_pending` slides every arriving icon out of
  exactly that point.
* **The tray has to be held back.** An attack is routed the moment the chain ends and the receiving
  game trays it there and then, so without `animate/tray.rs` the icons appear a third of a second
  before the ball carrying them lands. `match_screen` snapshots each tray's depth before the update
  loop and does not draw the new icons until the ball arrives.
* **A pop is a tell and then a strip** — the group flashes about three times over ~0.3 s **starting
  lit** (starting dark reads as a cell deleted rather than a warning), then a held surprised face,
  then the balls. The face is the long beat, not an even third, which is what `holding_first` is
  for.
* **`snes` plays the same strips, off its own sheet.** Kirby's Avalanche is the same Compile engine
  and the Blobs & Boulders rip carries every frame Mean Bean Machine's does, unlabelled and read
  off the art. Two things it does **not** have, and no theme invents art: the boulder gets **no
  bounce and no idle**, and there is **no attack ball** — that game draws none, so
  `draw_attack_ball` falls back to the popped blob with a white core.
* **An animation under `POP_DELAY` does not cost nothing.** `match_screen` skips `game.update`
  outright while an animation blocks the tick, so a strip and the delay **add**. `POP_DELAY` is
  90 ms and each theme carries its own beat: `genesis` ~770 ms a chain step (measured, and Alex's
  choice of "genesis plays slower" — settled, not a work item), `snes` ~550 ms and the particle
  theme 290 ms.
* **Nothing shakes.** Cross-correlated over 294 frames including a two-row drop: zero displacement,
  every frame. What reads as a rumble is every refugee bean bouncing at once. The rumble is opt-in
  and the particle theme is the only taker.
* **The per-column stagger in the nuisance fall is a golden-ratio hash of the column index**, not
  an RNG: neighbours get very different offsets so the row visibly breaks up rather than tilting,
  the same board falls the same way twice, and there is no randomness on the render path. It is
  written twice already — `nuisance.rs`'s stagger and `character.rs`'s `hashed`, which says so —
  and item 2 wants it a third time, so lift it into one place rather than copying it again.
* **The tray anchors at the well's right edge and fills leftwards** on `genesis`, which is the one
  thing on that theme the original does not do — it is drawn in `TOP_PADDING`, the band a pair
  spawns in, and the pair is drawn *over* it, so a left-anchored strip sat behind every spawn. The
  particle theme's tray went down the left of the board for the same reason. **An icon is drawn at
  three quarters of a cell**, on a pitch of the same number: the symbols are cut as whole cells and
  the art inside runs 12-16 px across, so drawing the cell at a half-cell pitch put a 12 px blob
  out at 6.
* Never in scope: the `y = 51` strips (harder squashes, dizzy faces) are uncut.

### The ai — `game/ai/`

`field.rs`, `quiet.rs`, `eval.rs`, `placement.rs`, `beam.rs`, `skill.rs`, `agent.rs`,
`input_sequence.rs`, `harness.rs`. There is no decomp to port, which is the one way this differs
from Dr. Rustario, so the shape came from the open literature — mostly
[ama](https://github.com/citrus610/ama) (MIT), whose evaluation is fifteen weights in one file
where `mayah`'s is about a hundred; plus takapt's beam-search idea and Ikeda et al.'s *Playing
PuyoPuyo*.

* **The quiescence search is the thing.** `quiet.rs` is what separates a bot that plays from one
  that tidies: a building player almost never fires anything, so a placement's own chain says
  nothing about nearly every placement on offer. What matters is the chain the field is *holding*.
* **The search is a state machine, not a function.** A pair takes a second to fall and nothing is
  waiting on the answer until it lands, so `Search::new` scores every root placement and stops, and
  each `step` expands more and hands the frame back. The strongest row costs 0.88 ms a frame
  instead of 10.6 ms in one lump — worth roughly a twelvefold budget at no cost in strength, which
  is why the ladder was **not** scaled down under `portmaster`. `width` is the dial if a device
  still cannot afford it, and `ga puyo rank` is compiled into the handheld build so measuring on
  the device is one command.
* **Two things follow from thinking slowly**, both handled in `agent.rs`: the pair goes on falling,
  so the keys are recomputed at the end from where it *is*; and if the chosen placement can no
  longer be reached the next one down is taken, which is why `beam::ranking` returns an order and
  not a winner. The pair may also come to rest before the search is done, so there is always an
  answer after `Search::new` and every step only sharpens it.
* **The ghost row is worth two features.** In `field.rs` it is the whole of the `NEIGHBOURS` table
  — the ghost row is nobody's neighbour, so the rule is stated once. In `eval.rs` it is the `ghost`
  weight, counting cells of that row walled off from the spawn column: a puyo resting up there is a
  *door closed*, because a pair moves sideways with one half in it.
* **Read colours through the mask, not around it.** A `CellId` is colour *and* link mask, so a
  feature comparing raw `CellId`s sees sixteen different reds and finds no chains at all.
  `Field::from_board` drops the mask on the way in.
* **The ladder is measured, not assumed.** `ga puyo rank [seeds] [pair cap] [difficulty]` plays
  every row over the same seeds and prints the `SKILL_ORDER` to paste back; `ga puyo play <seed>`
  plays one brain headless. The three rows sharing the `build` weights differ in *how long they
  hold a chain* rather than only in how hard they think, which is what came out of the first
  ranking run. The measure is a **solo marathon**, where no nuisance ever arrives, so it ranks what
  a row builds and not how it takes a hit — which is why item 1 needs a harness of its own.

### The launcher seams these all use

* **`ForeignPrices`** on `Attack` — a `[u32; 8]` keyed by `GameId`, `Copy`, **defaulting to zero**,
  with `with_foreign_for(receiver, price)` to author and `strength_for(receiver)` to read. An
  unpriced crossing is worth nothing and `Match::send_attack` drops it, rather than sending the
  wrong units silently. `GAMES` is 8: raise it rather than renumbering games.
* **`Game::pending_attacks() -> Vec<CellId>`** — the attack-queue strip. The game reports what is
  queued as its *own* `CellId`s and the theme draws them with `BlockSpriteSheet::draw_cell`, so the
  strip costs a theme no new art. A retro theme authors a `PendingLayout`; a particle theme sets
  `pending_max`.
* **`PerGame<T>`** and `GameKind`'s three lists: `ALL` (the order games are *numbered*, the key of
  every per-game collection), `RUNNING_ORDER` (the order they are *billed* on the pre-menu) and
  `PLAYLIST_ORDER` (the turn order, and the default selection once item 3 is in). Three because
  they are three different things; a test holds them to being the same games.
* **`AiBrain`** — `VersusAi::brains()` returns an ai player and one brain per game rather than a
  tuple that grows a dimension per game. A brain handed a board that is not its game does nothing.
  Each game's brain plays at *its own* declared key delay.
* **`modes.rs::game_mode(game)`** is the single place naming each game's standalone mode, so a game
  cannot be added to the shell and forgotten in the tests.
* **`MetricKind::Chain`** and **`words::CHAIN`** exist in the engine even though no HUD draws the
  counter: Puzzle Fighter and Bombliss both want the same one. Words are outlined ahead of time by
  `ParticleRender::build_captions`, so a word that was never outlined is silently dropped.

---

## Verification

* `cargo test --workspace`. `every_mode_offers_the_same_ai_opponents_and_demos` and
  `ai_difficulties_agree` in `launcher/src/modes.rs` gate the menu surface.
* `cargo run --example frame_shot -- 640 480 1 out/ puyo` — one frame on every theme, which is how
  theme geometry is checked without a display.
* `cargo run --example animation_shot -- 1920 1080 out/ genesis 50 80` — a scripted match stepped a
  frame at a time, one PNG every 50 ms, which is how anything that *moves* is checked. Its seventh
  argument is the scene: `drain` buries player one at the first frame, which is the only way to
  watch a game over without losing a match. Run it with `SDL_VIDEODRIVER=dummy
  SDL_RENDER_DRIVER=software`.
* `cargo run --example menu_shot -- 960 720 out/`; `field_preview sheet`; `character_shot` and
  `kirby_shot` for the cast.
* `ga puyo rank` for what a row builds; `ga puyo duel` for what it does under fire, which item 1
  has to write first; and the five-seed protocol for the attack prices.
* `python3 engine/art/audio_levels.py` after cutting any audio: it decodes every embedded ogg in
  the repository, applies the gains the Rust adds, and says what is out of the house band.
* Finally, play it. A 2-player match on each playlist, with the game checkboxes both ways.

## Working agreement

Work on this repository is **synchronous. One agent at a time. Never in parallel.**

* **This document is the shared memory.** Conversations do not carry over between agents; this file
  does. Read it before starting.
* **Status lives here**, updated in the same commit as the work it describes, so the document and
  the code never disagree.
* **One item at a time, in order.** If blocked, say so here with the reason and stop — do not route
  around it and do not start a later one instead. Surface it to Alex.
* **Record what a reader could not recover from the code**: decisions the plan did not anticipate,
  measured numbers with no home in a module, and anything that cost time. Not a diary — if the
  code, a test or a script's doc comment already says it, leave it there and say nothing.
* **Amend, do not append contradictions.** A document that argues with itself is worse than no
  document.
* **Stay inside this game.** While these items are open, nobody starts another game from
  [next-game-ideas.md](next-game-ideas.md) — see the status board there.
