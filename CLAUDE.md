@README.md

## Layout

| crate | what it is |
|---|---|
| `engine/` | everything that is not game rules: app shell, menus, high scores, config, input, rendering, audio, the match session, animation, particles, the shared ai core |
| `dr-rustario/`, `rustris/`, `puyo-rusto/` | each game's rules, themes and ai |
| `rustle-fighter/` | Super Rustle Fighter's rules and arcade theme; no ai yet. Behind the launcher's `rustle-fighter` feature, off by default: every `GameKind::RustleFighter` arm is `#[cfg]`'d |
| `launcher/` | the binary and the library Android loads (`android.rs`): `shell.rs` screens, `games.rs` `AnyGame`, `modes.rs` playlists, `cross.rs` `ga cross` |
| `android/` | the Gradle project; only builds through `build-android.sh` |

Game crates are siblings and never depend on each other; anything shared goes in `engine`.

## Commands

```shell
cargo build --release [--features rustle-fighter]
cargo test
./build-portmaster.sh | ./build-browser.sh | ./build-android.sh
cargo run --release -- ga ... # ai training and measurement, dispatched in launcher/src/lib.rs run()
```

Headless render harnesses, the way to see a change; with no display, prefix
`SDL_VIDEODRIVER=dummy SDL_RENDER_DRIVER=software`. Each example's doc comment has its usage:
`frame_shot`, `animation_shot`, `menu_shot`, `field_preview`, `character_shot`, `kirby_shot`,
`feature_shots`, `scale_report` (all `cargo run --example <name>`).

## Rules of the road

* `speed_index` may change how a game feels, never what it deals: playlist players are dealt
  from one shared seed and reach a stage change at different moments.
* Every match runs through `AnyGame` in `launcher/src/games.rs`. A defaulted `Game` method it does
  not delegate is silently never called; prefer a new `GameEvent` to a new trait method.
* Attacks cross games through `ForeignPrices` (`foreign_attack` in each `game/mod.rs`). An unpriced
  pair drops silently; `every_crossing_between_two_games_is_priced` catches it. Prices are
  measured by `ga cross`, never guessed.
* Every theme is built at startup in `Shell::new` and stays built; cell size is the largest all
  of a game's themes can hold, so panel dimensions are load-bearing.
* `engine/src/particles/source.rs` is fire-and-forget; `particles/field/` is a retained pool for a
  match, driven by `field/director.rs`, and never touches game state.

## Read before touching

| area | doc |
|---|---|
| any ai | [docs/ai.md](docs/ai.md) |
| art, audio, emulators | [docs/art-and-audio.md](docs/art-and-audio.md) |
| Super Rustle Fighter | [docs/super-rustle-fighter-plan.md](docs/super-rustle-fighter-plan.md), [docs/super-puzzle-fighter-rules.md](docs/super-puzzle-fighter-rules.md) |
| Puyo Rusto's rules | the Puyo Nexus page each `puyo-rusto/src/game/` module names |

## Keeping it tidy

* Working plans live in `docs/PLAN-*.md`, excluded from git; nothing cites them.
* A comment earns its place by carrying a constraint, and is then one or two sentences. No
  history, dates, measurements or run results in comments.
* A new invariant goes first as a comment on the code, then as one line in the area doc.
* README and this file stay slim. Terse wins.
