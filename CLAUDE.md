@README.md

## Layout

| crate | what it is |
|---|---|
| `engine/` | everything that is not game rules: SDL app shell, menus, high scores, config, input, rendering (sprite sheets, themes, fonts, particles, animations), audio mixer, the match session, and the shared AI core (`ai/`: neural network, genetic algorithm, key pacing) |
| `dr-rustario/` | Dr. Rustario's rules (bottle, pills, viruses), theme data and its neural AI |
| `rustris/` | Rustris's rules (board, SRS, scoring, garbage), theme data and its neural AI |
| `launcher/` | the `dr-rustario-vs-rustris` binary: picks games and options and runs a match |

Each game's AI supplies the game-specific half - board features, placement search and the agent -
on top of `engine::ai`, which owns the network shapes, the genome, the genetic algorithm and its
`Fitness` seam. Both games use the same architecture - as many neurons wide as it has features,
two hidden layers deep - sized to their own feature count (Rustris `FeatureNetwork`: 20 features,
1281 weights; Dr. Rustario `BottleFeatureNetwork`: 22 features, 1541), declared by the
`feature_network!` macro in `engine/src/ai/neural.rs` because the genome conversions belong to
neither game. Models are embedded as raw weight arrays in each game's
`ai/models.rs`; Dr. Rustario's are random until a `ga dr auto` run replaces them.

A game implements `engine::game::Game` (a headless board of `Cell`s with game-private
`CellId`s, producing engine `GameEvent`s) and `engine::render::GameRender`; its themes are
data handed to the engine's `retro_theme` and `modern_theme` builders. Attacks between players
are a neutral strength plus game-private detail, so Dr. Rustario garbage keeps its colours
between two Dr. Rustario players and becomes random colours when it comes from Rustris.