# Dr. Rustario vs. Rustris

A multi-themed Tetris vs Dr.Mario clone. Written in SDL2 and Rust for fun:

* **Dr. Rustario** - a Dr. Mario clone (NES, SNES, N64 and particle themes)
* **Rustris** - Tetris with the guideline ruleset (Game Boy, NES, SNES and particle themes)
* **Puyo Rusto** - Puyo Puyo Tsu (Genesis, SNES and particle themes)
* **Dr. Rustario vs Rustris** - play a multi-player focussed playlist over both games.

## Building

SDL2 is the only native dependency, and there are two ways of finding it, configured with cargo features:

* `pkgconfig` (the default) - dynamically link SDL2 from the system, through `pkg-config`. This
  is the Linux and macOS route.
* `vcpkg` - build SDL2 from source with [vcpkg](https://vcpkg.io) and link it statically, so
  the binary carries it and needs no SDL2 beside it. This is the Windows route.

Whichever it is, pass `--no-default-features` with it: the two are alternatives rather than
additions, and with both on SDL2's build script probes both - which on Windows means a
`pkg-config` probe that panics the build before vcpkg is ever reached.

All resources are embedded into the binary.

### Windows

Needs the MSVC toolchain (Visual Studio Build Tools with the C++ workload) and `git` on the
path - vcpkg compiles SDL2 itself, so the first run takes a while. In a developer prompt:

```powershell
rustup default stable-x86_64-pc-windows-msvc
cargo install cargo-vcpkg
cargo vcpkg build --manifest-path launcher/Cargo.toml
cargo build --release --no-default-features --features vcpkg
```

The result statically linked to SDL2 at `target\release\dr-rustario-vs-rustris.exe`.

### Linux

```shell
# Fedora
sudo dnf install SDL2-devel

# Ubuntu/Debian
sudo apt install libsdl2-dev pkg-config

cargo build --release --no-default-features --features pkgconfig
```

`pkgconfig` is the default feature, so a plain `cargo build --release` does the same thing.

### macOS

```shell
brew install sdl2 pkg-config
cargo build --release --no-default-features --features pkgconfig
```

The linker will fail to link SDL2 haptics. You will need to add the following to `~/.cargo/config.toml`:

```toml
[target.aarch64-apple-darwin]
rustflags = ["-C", "link-args=-weak_framework CoreHaptics"]
```

vcpkg works here too, if a self-contained binary is wanted - the same three commands as
Windows, and vcpkg's one macOS triplet is a static one:

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

## The AI

All three games find every placement the piece in play can reach, score them, and hand the best
one to an agent that presses the keys. Rustris scores with a small neural network;
Dr. Rustario plays a port of Dr. Mario 64's own hand written scorer, and has a neural network
that is trained but not yet strong enough to field as a difficulty; Puyo Rusto searches several
pairs ahead with a beam search over a hand written evaluation, and has no neural model at all.
The network and the genetic algorithm that trains it are shared in `engine/src/ai`; each game
supplies its own features, placement search and agent. Only human players can enter the high
score table.

Full write up: [https://ax-h.com/ai/machine-learning-from-scratch](https://ax-h.com/ai/machine-learning-from-scratch)