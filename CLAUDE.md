@README.md

## Layout

| crate | what it is |
|---|---|
| `engine/` | everything that is not game rules: SDL app shell, menus, high scores, config, input, rendering (sprite sheets, themes, fonts, particles, animations), audio mixer, the match session, and the shared AI core (`ai/`: neural network, genetic algorithm, key pacing) |
| `dr-rustario/` | Dr. Rustario's rules (bottle, pills, viruses), theme data, its neural AI and the deterministic one (`game/ai/n64/`) that actually plays |
| `rustris/` | Rustris's rules (board, SRS, scoring, garbage), theme data and its neural AI |
| `launcher/` | the `dr-rustario-vs-rustris` binary: picks games and options and runs a match |

Each game's AI supplies the game-specific half - board features, placement search and the agent -
on top of `engine::ai`, which owns the network shapes, the genome, the genetic algorithm and its
`Fitness` seam. Both games use the same architecture - as many neurons wide as it has features,
two hidden layers deep - sized to their own feature count (Rustris `FeatureNetwork`: 20 features,
1281 weights; Dr. Rustario `BottleFeatureNetwork`: 22 features, 1541), declared by the
`feature_network!` macro in `engine/src/ai/neural.rs` because the genome conversions belong to
neither game. Models are embedded as raw weight arrays in each game's
`ai/models.rs`; Dr. Rustario's are random until a `ga dr auto` run replaces them, which is why
its opponent and demo play `game/ai/n64/` instead - a port of `aiset.c` from the N64 game's
decompilation. That is a deterministic scorer: `field.rs` is the bottle in the ai's own 17x8
`(st, co)` grid, `score.rs` measures the runs the two halves land in, `chain.rs` asks whether
the placement leaves a chain, `routes.rs` measures how much room the pill has left, `params.rs`
holds the weights and `mod.rs` picks the skill row and situation column that select them.
`DrAiAgent` chooses between the two through `DrAiKind`; `ga dr play <seed> <level> <cap>
<every> <brain>` runs either headless, where a brain is `n64`, `n64:0`..`n64:5`, `neural` or
`linear`. The six skill rows are the one dial the original has, and `params.rs`'s `SKILL_ORDER`
ranks them worst to best (measured, not assumed - the rows are personalities, not a ladder):
that is what Dr. Rustario's four ai difficulties and its 2-player demo pick from, so a harder
setting is a better player as well as a faster one.

Both games and the vs. mode offer the same ai modes - four difficulties and a 1- and 2-player
demo - off the title screen's `players` list. Each game names its own models per mode in its
`GameConfig::ai_players`; the vs. mode's `VersusAi` (`launcher/src/modes.rs`) just asks both
games for theirs, so an ai player there is a pair of brains and its controller dispatches on
`AnyGame::kind()`, resetting both agents as the playlist swaps the board for the other game's.

The particle engine has two models. `particles/source.rs` is the original fire-and-forget
emitter: a source emits a group and then has no further say, which is right for a burst and is
what every foreground effect and every menu uses. `particles/field/` is a retained pool
(`particles/pool.rs`) that owns its particles for the life of a match and steers them every
frame: it is the particle themes' background (built by `modern_theme`), and it is the only
thing that can retarget a particle that already exists. The field spans a *canvas* - the
union of the clips of the players on a particle scene - and its routines are authored in canvas-normalised coordinates,
so one written once fits the whole window or half of it. It observes the match through a
`SceneContext` built by the match screen and reacts to queued `FieldEvent`s; it never reads or
writes game state and shares no RNG with the games. What it is doing at any moment is a
`director.rs` state machine over a weighted playlist - a resting `Ambient` routine, then a
`Feature` gathered, held and shattered - and where a feature wants its particles is a
`formation.rs` `Formation`. The words it spells are the engine's own
(`reaction::words`, outlined ahead of time by the renderer): a game says *when* through
`GameRender::clear_word`, and the match screen offers the rest as `SceneContext::captions`,
since only it can name a game or its numbers. `cargo run --example field_preview` renders it headless:
`features` draws one png per routine, `sheet` outlines every sprite of every theme into a
labelled png, and `<seconds>` snapshots a running field.

A game implements `engine::game::Game` (a headless board of `Cell`s with game-private
`CellId`s, producing engine `GameEvent`s) and `engine::render::GameRender`; its themes are
data handed to the engine's `retro_theme` and `modern_theme` builders (a theme says which it
is through `Theme::family`, which is what lets the vs. mode's retro and particle playlists pick
their themes out). Attacks between players are a neutral strength plus game-private detail, so Dr. Rustario garbage keeps its colours
between two Dr. Rustario players and becomes random colours when it comes from Rustris.

A menu's items are built once, so a list whose options depend on another item - the mode
list, which loses the theme sprint as soon as a single theme is picked - is refreshed by
`MenuScreen::set_items` after every selection, from the launcher's `Shell`.
`cargo run --example menu_shot` draws both games' menus to pngs headless, walking the theme
and mode rows, which is how that is checked without a display.