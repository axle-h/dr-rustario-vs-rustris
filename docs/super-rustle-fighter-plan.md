# Super Rustle Fighter: what is left

The rules (`rustle-fighter/src/game/`) and the arcade theme (`src/theme/arcade/`) are done and
playable behind the `rustle-fighter` feature. The rules reference is
[super-puzzle-fighter-rules.md](super-puzzle-fighter-rules.md). Three pieces remain, in order.

## 1. The ai

Placement search, evaluation and agent under `rustle-fighter/src/game/ai/`, no neural model.
Puyo Rusto's beam search over a hand written evaluation is the nearest sibling. Power gems make
it different: the interesting move is often not to break, and nothing published covers when to.

Done when a difficulty ladder exists and is measured, the way `ga puyo rank` measures Puyo's.

Open question for a playtest first: the code pays a power gem nothing over the same gems loose
and linked. If power gems feel pointless in play, a scoring term is missing from the rules doc;
an ai built against that incentive is wasted.

## 2. The playlist turn and the crossings

Add the game to `GameKind::PLAYLIST_ORDER`. `every_crossing_between_two_games_is_priced` then asks
for its six prices (to and from each of the other three), which `ga cross` must measure. Re-run
the whole table, not only the new rows: the existing six will move.

Rustle Fighter's attack cancels against the opponent's before it lands and its damage grows with
round time, so a naive "what it throws a minute" misreads it. Read `launcher/src/cross.rs` first.

The gem distribution table and drop pattern stay fixed for a whole match: `speed_index` may
change how a game feels, never what it deals.

## 3. The fighter layer

The seven fighters and their animation states, driven by `engine/src/animate/character.rs`, and
the panel geometry measured against the arcade game rather than composed. The sheets are in
`~/Downloads/Super Puzzle Fighter Art/` (arcade rips; effects are PlayStation). Seven characters
have sheets: Chun-Li, Felicia (a gif), Hsien-Ko, Ken, Morrigan, Ryu, Sakura. Each sheet labels
its rows; the ones the game drives:

| row | drives |
|---|---|
| Idle | board fill 0 |
| Disadvantage 1 | board fill 1 (37-54 cells) |
| Disadvantage 2 | board fill 2 (above 54) |
| Advantage | the opponent is buried and you clear |
| Fight Start & Taunt | round start, and the once-a-round taunt |
| Special Move 1 / Super Combo | attacking; magnitude is `chain * chain` quartered, floor 1 |
| Minor / Major Damage | taking a small / large attack |
| Victory & Selected, Lose | round won and character select, round lost |

The arcade's CAUTION / WARNING / DANGER plates are cut but unused; they belong to this layer.

Done when `character_shot` renders all seven casts, and `frame_shot` and `animation_shot` render
a match that reads as the arcade game does.

## After this

The game has no particle theme, so no build of it carries only original art; the other three
games each have one.

The next games queued are Tetris Battle Gaiden (crystals fill a gauge that casts spells) and
Bombliss (bomb cells detonate on a clear). Both are alternate Rustris rulesets, a `GameConfig`
variant in `rustris/` reusing its board and tetrominoes, not new crates. Panel de Pon and
Magical Drop were rejected: neither has a falling piece, so neither fits `engine::game::Game`.
