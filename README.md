# Dr. Rustario vs. Rustris

A multi-themed compendium of falling block games, written in Rust on SDL2 for fun:

* **Dr. Rustario** - Dr. Mario (NES, SNES, N64 and particle themes)
* **Rustris** - guideline Tetris (Game Boy, NES, SNES and particle themes)
* **Puyo Rusto** - Puyo Puyo Tsu (Genesis, SNES and particle themes)
* **Super Rustle Fighter** - Super Puzzle Fighter II Turbo (arcade theme). Playable on its own,
  with no ai yet; only built with `--features rustle-fighter`
* **vs. playlist** - every player plays the same sequence over the games that have an ai

All resources are embedded in the binary. The first three games have an ai opponent; the write up
is at [ax-h.com/ai/machine-learning-from-scratch](https://ax-h.com/ai/machine-learning-from-scratch).

## Building

SDL2 is the only native dependency. A plain `cargo build --release` links it the platform's way:
`pkgconfig` (the default feature) on Linux and macOS, vcpkg on Windows. For a self-contained
binary elsewhere, use `--no-default-features --features vcpkg`; the two features are
alternatives, not additions.

| platform | how |
|--|--|
| Linux | `sudo dnf install SDL2-devel` or `sudo apt install libsdl2-dev pkg-config`, then `cargo build --release` |
| macOS | `brew install sdl2 pkg-config`, then `cargo build --release`; add `rustflags = ["-C", "link-args=-weak_framework CoreHaptics"]` under `[target.aarch64-apple-darwin]` in `~/.cargo/config.toml` |
| Windows | MSVC build tools and `git`; `cargo install cargo-vcpkg`, `cargo vcpkg build --manifest-path launcher/Cargo.toml`, `cargo build --release` |
| PortMaster (aarch64 handhelds) | `./build-portmaster.sh` -> `dist/dr-rustario-vs-rustris.zip`; `DEPLOY_HOST=root@device` copies it over |
| Browser (wasm) | `./build-browser.sh` -> `dist/browser/`, `./serve-browser.sh` serves it on :8080 |
| Android (pad handhelds) | `./build-android.sh` -> `dist/dr-rustario-vs-rustris.apk`; `DEPLOY=1` runs `adb install` |

The PortMaster, browser and Android builds run in Docker. PortMaster links the firmware's own
SDL2 and keeps config next to the binary; install by dropping the zip in
`PortMaster/autoinstall/`. `browser` and `portmaster` cannot be enabled together. The browser
build persists config in IndexedDB. Android has no touch controls, is landscape only, needs
Android 8.0, and maps the back button to Escape; keep `android/release.keystore`, since an
update must be signed with the key it was installed with. Neither browser nor Android builds
the `ga` training subcommand.

## Config

Config and high scores (`high_scores.yml`) are yaml, in:

* Windows: `$HOME\AppData\Roaming\dr-rustario-vs-rustris`
* macOS: `$HOME/Library/Application Support/dr-rustario-vs-rustris`
* Linux: `$XDG_CONFIG_HOME/dr-rustario-vs-rustris` or `$HOME/.config/dr-rustario-vs-rustris`
* Android: `Android/data/com.ax_h.drrustariovsrustris/files` on shared storage
* PortMaster: `/roms/ports/dr-rustario-vs-rustris/`

`video.mode` is `!Window { width, height }` (default 1280x720), `!FullScreen { width, height }`
or `!FullScreenDesktop`.

### Controls

Game controllers work through SDL's GameController API (`SDL_GAMECONTROLLERCONFIG` for
unrecognised pads); each pad takes the next free player slot.

| Button | Menu | Game |
|--|--|--|
| D-pad / left stick | Navigate | Move, soft drop (down), hard drop (up) |
| A | Select | Rotate clockwise |
| B | Back | Rotate anticlockwise |
| X / L1 / R1 | | Hold |
| Y | | Next theme |
| Start | Start | Pause |
| Select / Back | | Return to menu |

Keyboard controls are configured under `input` (`menu`, `player1`, `player2`, `pause`,
`next_theme`, `quit`); key names are in [engine/src/config.rs](engine/src/config.rs). Player 2
has no default keys.

## The vs. playlist

Its menu ticks which games are in it. A playlist deals its games in turn, theme slot by theme
slot, and never ranks, so it has no high score table.

Attacks cross between games at prices measured by `cargo run --release -- ga cross`, as a share
of what the receiving game's own opponents throw:

| sender | receiver | what crosses | share of a home opponent |
|--|--|--|--|
| Dr. Rustario | Rustris | a row per pattern past the first, up to 4 | 0.24 |
| Dr. Rustario | Puyo Rusto | three nuisance for each of those rows | an eighth of a board a minute |
| Rustris | Dr. Rustario | 2 blocks for a tetris or T-spin double, 3 for a triple, 4 for a perfect clear | 0.79 |
| Rustris | Puyo Rusto | the same clears, at a row of nuisance a block | a fifth of a board a minute |
| Puyo Rusto | Dr. Rustario | a block per two rocks of nuisance, up to 4 | 0.49 |
| Puyo Rusto | Rustris | a row per two rocks of nuisance, up to 4 | 0.23 |
