# Dr. Rustario vs. Rustris

A multi-themed Tetris vs Dr.Mario clone. Written in SDL2 and Rust for fun:

* **Dr. Rustario** - a Dr. Mario clone (NES, SNES, N64 and particle themes)
* **Rustris** - Tetris with the guideline ruleset (Game Boy, NES, SNES and particle themes)
* **Puyo Rusto** - Puyo Puyo Tsu (Genesis, SNES and particle themes)
* **Super Rustle Fighter** - Super Puzzle Fighter II Turbo (arcade theme). Playable on its
  own; it has no ai yet, so it takes no turn in the vs. playlist.
* **Dr. Rustario vs Rustris** - play a multi-player focussed playlist over the games that
  have an ai.

## Building

SDL2 is the only native dependency, and there are two ways of finding it, configured with cargo features:

* `pkgconfig` - dynamically link SDL2 from the system, through `pkg-config`. This is the
  Linux and macOS route.
* `vcpkg` - build SDL2 from source with [vcpkg](https://vcpkg.io) and link it statically, so
  the binary carries it and needs no SDL2 beside it. This is the Windows route.

**Each platform's own route is what a plain `cargo build --release` does**, so neither
normally wants naming: `pkgconfig` is the default feature and is simply absent on the targets
that have no pkg-config, where vcpkg is wired in instead. The one build that wants a flag is a
self-contained macOS (or Linux) binary, which is `vcpkg` - and it wants `--no-default-features`
with it, since the two are alternatives rather than additions and SDL2's build script probes
both when it is given both.

All resources are embedded into the binary.

### Windows

Needs the MSVC toolchain (Visual Studio Build Tools with the C++ workload) and `git` on the
path - vcpkg compiles SDL2 itself, so the first run takes a while. In a developer prompt:

```powershell
rustup default stable-x86_64-pc-windows-msvc
cargo install cargo-vcpkg
cargo vcpkg build --manifest-path launcher/Cargo.toml
cargo build --release
```

The result statically linked to SDL2 at `target\release\dr-rustario-vs-rustris.exe`.

### Linux

```shell
# Fedora
sudo dnf install SDL2-devel

# Ubuntu/Debian
sudo apt install libsdl2-dev pkg-config

cargo build --release
```

### macOS

```shell
brew install sdl2 pkg-config
cargo build --release
```

The linker will fail to link SDL2 haptics. You will need to add the following to `~/.cargo/config.toml`:

```toml
[target.aarch64-apple-darwin]
rustflags = ["-C", "link-args=-weak_framework CoreHaptics"]
```

vcpkg works here too, if a self-contained binary is wanted - the same three commands as
Windows, and vcpkg's one macOS triplet is a static one. This is the one build that names a
feature, since it has to turn the platform's own default off:

```shell
cargo install cargo-vcpkg
cargo vcpkg build --manifest-path launcher/Cargo.toml
cargo build --release --no-default-features --features vcpkg
```

### Retro handhelds (PortMaster)

The game is packaged as a [PortMaster](https://portmaster.games) port for aarch64 handhelds
(ROCKNIX, ArkOS, muOS, Knulli ...):

```shell
./build-portmaster.sh                          # -> dist/dr-rustario-vs-rustris.zip
DEPLOY_HOST=root@rocknix ./build-portmaster.sh # ... and copy it to the device
```

This cross-compiles in Docker (`Dockerfile.aarch64`: Ubuntu 20.04 / glibc 2.31, the PortMaster
baseline) with the `portmaster` feature, then zips the binary up with the launcher script
and metadata from [portmaster/](portmaster). SDL2 is not bundled: like other native PortMaster ports
the binary links the firmware's own `libSDL2-2.0.so.0`.

To install without PortMaster's catalogue drop the zip into the device's
`PortMaster/autoinstall/` folder (`/storage/roms/ports/PortMaster/autoinstall/` on ROCKNIX) and
open PortMaster, or unzip it straight into `/roms/ports/`. Config and high scores are then kept
in `/roms/ports/dr-rustario-vs-rustris/`.

The `portmaster` feature defaults to desktop fullscreen and stores config next to the binary.

### Browser (wasm)

The game runs in the browser via [Emscripten](https://emscripten.org)
(`wasm32-unknown-emscripten`), behind the `browser` feature. `browser` and `portmaster`
are mutually exclusive: enabling both fails the build.

```shell
./build-browser.sh          # -> dist/browser/ (index.html + js + wasm)
./serve-browser.sh          # serve it on http://localhost:8080 (PORT=... to change)
```

This builds in Docker (`Dockerfile.browser`: emsdk pinned to ABI-match the Rust
toolchain's prebuilt std) with the emscripten link flags from `.cargo/config.toml`. The
page (`web/index.html`) starts the game on a click and mounts IndexedDB at `/data`,
where config and high scores persist across reloads.
The `ga` training subcommand is not part of the browser build, but the
AI opponent and demo mode are. The wasm embeds all game assets, so serve it compressed.

## Config

Config and high scores are stored in yaml:

* Windows: `$HOME\AppData\Roaming\dr-rustario-vs-rustris`
* MacOS: `$HOME/Library/Application Support/dr-rustario-vs-rustris`
* Linux: `$XDG_CONFIG_HOME/dr-rustario-vs-rustris` or `$HOME/.config/dr-rustario-vs-rustris`

High scores all live in one `high_scores.yml`.

You can ignore most of the config except:

### Video Mode

* `Window` (default) - note if your screen is not at least 720p then the game may not even load on first attempt.
    ```yaml
    video:
      mode: !Window
        width: 1280
        height: 720
    ```
* `FullScreen` - native fullscreen (recommended), note the game should scale to any weird resolution but was designed for 1080p & 4k.
    ```yaml
    video:
      mode: !FullScreen
        width: 1920
        height: 1080
    ```  
* `FullScreenDesktop` - fullscreen in windowed mode
    ```yaml
    video:
      mode: !FullScreenDesktop
    ```

### Controls

Game controllers are supported out of the box through SDL's GameController API (set
`SDL_GAMECONTROLLERCONFIG` for unrecognised pads). The pad layout is fixed; each pad takes
the next free player slot:

| Button | Menu | Game |
|--|--|--|
| D-pad / left stick | Navigate | Move, soft drop (down), hard drop (up) |
| A | Select | Rotate clockwise |
| B | Back | Rotate anticlockwise |
| X / L1 / R1 | | Hold |
| Y | | Next theme |
| Start | Start | Pause |
| Select / Back | | Return to menu |

Keyboard controls are configurable:

```yaml
input:
  menu:
    up: Up
    down: Down
    left: Left
    right: Right
    select: X
    start: Return
  player1:
    move_left: Left
    move_right: Right
    soft_drop: Down
    hard_drop: Up
    rotate_clockwise: X
    rotate_anticlockwise: Z
    hold: LShift
  player2: ~
  pause: F1
  next_theme: F2
  quit: Escape
```

All key names are defined in [engine/src/config.rs](engine/src/config.rs).

There are no default player 2 controls.

## The vs. playlist

`vs. playlist` runs one playlist over the three games, every player playing the same sequence.
Its menu picks **which** games are in it - a tick per game, all three on to start with, and
the last one cannot be turned off - so the old two-game compendium is two ticks away. A
playlist deals its ticked games in turn, theme slot by theme slot; a game with fewer themes
than the longest list replays its own from the start rather than shortening the playlist.

**The playlist does not rank.** Nine playlists times seven subsets of three games is more
variations of the game than any high score table could usefully hold, so there is none and a
playlist never offers name entry. The three single game modes keep their tables.

### What an attack is worth in another game

Only the sender knows what a clear took, so only it can say what that is worth to somebody
playing a different game - which makes six directed prices between three games. They are
**measured, not guessed**: `cargo run --release -- ga cross` plays each game's own ai alone
for fifty minutes of game time and counts what it throws, then every crossing is read as a
share of what a player of the receiving game faces from an opponent of *their* game. 1.00 is
a foreign opponent pressing exactly as hard as a home one, and nothing is over it.

| sender | receiver | what crosses | share of a home opponent |
|--|--|--|--|
| Dr. Rustario | Rustris | a row per pattern past the first, up to 4 | 0.24 |
| Dr. Rustario | Puyo Rusto | three nuisance for each of those rows | an eighth of a board a minute |
| Rustris | Dr. Rustario | 2 blocks for a tetris or T-spin double, 3 for a triple, 4 for a perfect clear | 0.79 |
| Rustris | Puyo Rusto | the same clears, at a row of nuisance a block | a fifth of a board a minute |
| Puyo Rusto | Dr. Rustario | a block per two rocks of nuisance, up to 4 | 0.49 |
| Puyo Rusto | Rustris | a row per two rocks of nuisance, up to 4 | 0.23 |

The two directions are not symmetric and are not meant to be. Garbage arriving at a Puyo board
joins the nuisance tray, where offset can cancel it and the ai will chain back at it, so a
crossing *into* Puyo is read against how much board it fills rather than against Puyo's own
output - a Puyo player alone throws 327 nuisance a minute, four boards' worth, because nothing
is arriving to offset. What leaves Puyo lands on a player with no offset at all, so it is
tuned a long way down: the routine two-chains a Puyo player throws constantly cross as
nothing, and only a chain worth digging out of is felt elsewhere.

## The AI

All three games find every placement the piece in play can reach, score them, and hand the best
one to an agent that presses the keys. Rustris scores with a small neural network;
Dr. Rustario plays a port of Dr. Mario 64's own hand written scorer; Puyo Rusto searches several
pairs ahead with a beam search over a hand written evaluation, and has no neural model at all.

Dr. Rustario also has a **trained neural network, and nothing fields it**. It is the stronger
player on the numbers - over twenty seeds at the training budget it destroyed 20,016 viruses
and finished 422 bottles against the port's 18,093 and 405, winning seventeen of the twenty -
and it is not good to watch, which is the question that decides what a difficulty plays. It
wins by grinding where the port plays legibly. Every difficulty and both demos are rows of the
port's own six; the network stays reachable through `ga dr`.
The network and the genetic algorithm that trains it are shared in `engine/src/ai`; each game
supplies its own features, placement search and agent. Only human players can enter the high
score table.

Full write up: [https://ax-h.com/ai/machine-learning-from-scratch](https://ax-h.com/ai/machine-learning-from-scratch)