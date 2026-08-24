# Dr. Rustario vs. Rustris

A multi-themed Tetris vs Dr.Mario clone. Written in SDL2 and Rust for fun:

* **Dr. Rustario** - a Dr. Mario clone (NES, SNES, N64 and modern themes)
* **Rustris** - Tetris with the guideline ruleset (Game Boy, NES, SNES and modern themes)
* **Dr. Rustario vs Rustris** - play a multi-player focussed playlist over both games.

## Building

Requires vcpkg to build on macos and Windows.

```bash
cargo install cargo-vcpkg
cargo vcpkg build
cargo build --release --no-default-features --features vcpkg
```

All resources are embedded into the binary, including the Rustris AI opponent, demo mode and
the `ga` training subcommand (`dr-rustario-vs-rustris ga [auto|survival|score|diagnose]`).

### macOS

The linker will fail to link SDL2 haptics. You will need to add the following to `~/.cargo/config.toml`:

```toml
[target.aarch64-apple-darwin]
rustflags = ["-C", "link-args=-weak_framework CoreHaptics"]
```

### Linux

```shell
# Fedora
sudo dnf install SDL2-devel

# Ubuntu/Debian
sudo apt install libsdl2-dev
```

Build with pkgconfig:

```shell
cargo build --release --no-default-features --features pkgconfig
```

### Retro handhelds (PortMaster)

The game is packaged as a [PortMaster](https://portmaster.games) port for aarch64 handhelds
(ROCKNIX, ArkOS, muOS, Knulli ...):

```shell
./build-portmaster.sh                          # -> dist/dr-rustario-vs-rustris.zip
DEPLOY_HOST=root@rocknix ./build-portmaster.sh # ... and copy it to the device
```

This cross-compiles in Docker (`Dockerfile.aarch64`: Ubuntu 20.04 / glibc 2.31, the PortMaster
baseline) with the `retro_handheld` feature, then zips the binary up with the launcher script
and metadata from [portmaster/](portmaster). SDL2 is not bundled: like other native PortMaster ports
the binary links the firmware's own `libSDL2-2.0.so.0`.

To install without PortMaster's catalogue drop the zip into the device's
`PortMaster/autoinstall/` folder (`/storage/roms/ports/PortMaster/autoinstall/` on ROCKNIX) and
open PortMaster, or unzip it straight into `/roms/ports/`. Config and high scores are then kept
in `/roms/ports/dr-rustario-vs-rustris/`.

The `retro_handheld` feature defaults to desktop fullscreen and no integer scaling, and stores
config next to the binary.

## Config

Config and high scores are stored in yaml:

* Windows: `$HOME\AppData\Roaming\dr-rustario-vs-rustris`
* MacOS: `$HOME/Library/Application Support/dr-rustario-vs-rustris`
* Linux: `$XDG_CONFIG_HOME/dr-rustario-vs-rustris` or `$HOME/.config/dr-rustario-vs-rustris`

High scores all live in one `high_scores.yml`, structured by game and then mode: one table
per mode of each game and per vs. playlist (start level, speed and difficulty share their
mode's table). Marathon tables rank the highest scores; sprint games (level, theme and
point sprints, and the vs. theme race, which is first to the end of the playlist) race a
single clock, shown in the bottom-left corner (single player) or bottom-centre
(multiplayer), and their tables rank the fastest times. The interleaved and back to back
vs. playlists are marathons: they cycle their playlist endlessly and rank scores. The clock
runs while anyone is playing: it stops while paused and when every player is held up at
once, so in single player a stage-clear card or theme change does not count against you,
while in a multiplayer race the clock keeps running.
Every table starts with default entries to beat: low scores, or long times for sprints; a
table that cannot be loaded falls back to the defaults, and new entries are saved along
with them. The high scores screen on the first menu pages through every table with
left/right. Per-mode files from earlier versions (`high_scores.dr-rustario.yml`,
`high_scores.rustris.yml`, ...) are ignored.

Most of it you can ignore except:

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

## Rustris AI

The **ai** option on a Rustris main menu selects who plays:

* `off` - human players.
* `vs challenging` / `vs difficult` / `vs impossible` - in a 2-player match the AI plays as player 2 (who must be on
  Rustris) and is speed limited by pressing one key every 250 ms / 80 ms / instantly (see `AiDifficulty` in
  `rustris/src/game/rules.rs`).
* `demo` - the first player's board is played by the AI at full speed; their controls are disabled.

Only human players can enter the high score table.

Full write up: [https://ax-h.com/ai/machine-learning-from-scratch](https://ax-h.com/ai/machine-learning-from-scratch)