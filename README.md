# Dr. Rustario vs. Rustris

A multi-themed Tetris vs Dr.Mario clone. Written in SDL2 and Rust for fun:

* **Dr. Rustario** - a Dr. Mario clone (NES, SNES, N64 and particle themes)
* **Rustris** - Tetris with the guideline ruleset (Game Boy, NES, SNES and particle themes)
* **Dr. Rustario vs Rustris** - play a multi-player focussed playlist over both games.

## Building

Requires vcpkg to build on macos and Windows.

```bash
cargo install cargo-vcpkg
cargo vcpkg build
cargo build --release --no-default-features --features vcpkg
```

All resources are embedded into the binary, including both games' AI opponents, demo mode and
the `ga` training subcommand (`dr-rustario-vs-rustris ga [auto|survival|score|diagnose]`).
`ga play <seed> [line cap] [report every n lines] [survival|tetris]` plays a built-in model
headless on a fixed seed, reporting progress; it counts lines and banks the score itself as the
in-game counters are capped. `ga dr [auto|tune|diagnose]` trains Dr. Rustario instead, and
`ga dr play <seed> [virus level] [pill cap] [report every n pills]` plays its model headless.
A `ga dr auto` run has no generation limit: it stops when a candidate clears every bottle on its
training seeds and then does it again on five it has never played.

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
page (`web/index.html`) starts the game on a click - browsers only allow audio after a
user gesture - and mounts IndexedDB at `/data`, where config and high scores persist
across reloads. The `ga` training subcommand is not part of the browser build, but the
AI opponent and demo mode are. The wasm embeds all game assets, so serve it compressed.

## Config

Config and high scores are stored in yaml:

* Windows: `$HOME\AppData\Roaming\dr-rustario-vs-rustris`
* MacOS: `$HOME/Library/Application Support/dr-rustario-vs-rustris`
* Linux: `$XDG_CONFIG_HOME/dr-rustario-vs-rustris` or `$HOME/.config/dr-rustario-vs-rustris`

High scores all live in one `high_scores.yml`, structured by game and then mode: one table
per mode of each game and per vs. playlist (start level, speed and difficulty share their
mode's table). Each game offers the same four modes whether one or two are playing: a
marathon and the level, theme and point sprints. The theme sprint runs one level per theme,
so it is only offered while **themes** is `all` - pick a single theme and it drops off the
list. Marathon tables rank the highest scores; sprint games (level, theme and
point sprints, and the vs. races: the theme race, which is first to the end of the
playlist, and the 3, 5 and 10 level random sprints, which deal a random game and theme
each level) race a single clock, shown in the bottom-left corner (single player) or
bottom-centre (multiplayer), and their tables rank the fastest times. The interleaved,
back to back, retro, particle and random marathon vs. playlists are marathons: the first
two cycle their playlist endlessly, the retro playlist alternates the games through their
retro themes only and the particle playlist alternates them on their particle themes, the
random marathon deals random games and themes forever, and all rank scores. Every vs.
playlist shares one difficulty dial, 0 (default) to 10: it sets
Dr. Rustario's virus level and fall speed and Rustris's starting level together, from no
viruses at low speed on level 0 up to virus level 10 at high speed on level 10. In a
random playlist every player faces the same random sequence, dealt fresh each match. The clock
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

### Particle density

The particle themes' background is a particle field: a retained pool of particles spanning
every player on a particle theme, resting between features and then gathering into one before
shattering back. It rests as a slow orbit, a drifting flow, a vortex, or a constellation of
particles with lines drawn between near neighbours; the features it gathers into are a piece
silhouette (a tetromino, a pill, a virus or either game's mascot, outlined from the sprites
of the themes in play), a word spelt out, a lattice, a waveform,
a set of concentric rings drifting about, a spiral, a Lissajous figure, weather blowing
through, and the boards themselves pulling the field around. It reacts to the match: clears
send a wave through it, a big clear calls up a silhouette, an attack flies across it as a
comet, and a stack near the top drains it toward red. It spells `TETRIS` on a four line
clear, `COMBO` on a Dr. Rustario combo and `GAME OVER` when someone is buried; left to
itself it spells the game in play, the level, or `VS`. It keeps out of the players' way,
thinning and dimming where it would cross a playfield. Draw calls are what it costs, so how
much of it there is is configurable (config file only, there is no video options menu):

```yaml
video:
  particle_density: Auto   # Auto | Low | Medium | High | Ultra
```

`Auto` is `High` on a desktop and `Low` on a handheld. It sizes the pool, the number of links
drawn between neighbouring particles, how finely a silhouette is sampled and how many
concurrent effects there may be. At `Low` the links are single pixel lines rather than
textured quads.

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

Both games are played by the same machinery: features are extracted from the board, fed to a
small neural network that scores every placement the piece in play can reach, and the best one
is handed to an agent that presses the keys. The network and the genetic algorithm that trains
it are shared in `engine/src/ai`; each game supplies its own features, placement search and
agent. Only human players can enter the high score table.

Full write up: [https://ax-h.com/ai/machine-learning-from-scratch](https://ax-h.com/ai/machine-learning-from-scratch)

### Rustris

The **ai** option on a Rustris main menu selects who plays:

* `off` - human players.
* `vs easy` / `vs normal` / `vs hard` / `vs impossible` - in a 2-player match the AI plays as player 2 (who must
  be on Rustris) and is speed limited by pressing one key every 500 ms / 400 ms / 300 ms / instantly (see
  `AiDifficulty` in `rustris/src/game/rules.rs`). `easy` and `normal` play the survival-trained model, which
  keeps its stack low and rarely attacks; `hard` and `impossible` play the high scoring model, which chases
  4-line Tetris clears.
* `1-player ai demo` - the first player's board is played by the AI at full speed; their controls are disabled.
* `2-player ai demo` - the AI plays both boards at full speed: the survival model as player 1 against the
  high scoring model as player 2.

Trained by `ga [auto|survival|score]`, which optimises for survival and then for 4-line clears.

### Dr. Rustario

The **ai** option on a Dr. Rustario main menu offers the same choices, minus the 2-player demo:
there is one Dr. Rustario model, so it would only play itself.

* `off` - human players.
* `vs easy` / `vs normal` / `vs hard` / `vs impossible` - in a 2-player match the AI plays as player 2
  (who must be on Dr. Rustario), speed limited to one key every 500 ms / 400 ms / 300 ms / instantly
  (see `AiDifficulty` in `dr-rustario/src/game/rules.rs`). Every difficulty plays the same model and
  differs only in how fast it is allowed to press keys.
* `1-player ai demo` - the first player's bottle is played by the AI at full speed; their controls are
  disabled.

The model reads ten things about the bottle - viruses, the work still needed to clear every
virus, runs of two and three that would take a virus with them, the same two runs where no virus
is involved, blocks buried under other colours counted separately for viruses and for everything
else, the tallest column and the holes under the stack - as both a change and a total, plus what
the placement itself wasted and cleared: twenty two inputs in all, feeding two hidden layers of
twenty two, the same architecture the Rustris model trained well at. Two of them carry most of the
weight. The **work** count asks, for every virus, how many matching blocks a line of four through
it still needs, and counts a virus nothing can reach any more as worse than any reachable one; it
is what points the agent at the viruses instead of at a tidy heap in the corner. The **wasted
halves** count charges for a half that does not join a run of its own colour with a virus in it,
which is what makes dropping a double blue while a blue virus is still in the bottle a bad move
rather than a neutral one. Tucking sideways under an overhang is not searched, since the agent
has no single step soft drop to execute it with.

`ga dr auto` trains it in a single stage with a single measure: candidates play the game itself,
starting on the first bottle, clearing it, moving on to the next, and are scored on the viruses
they took out before they were buried. There is no pill budget and nothing rewards speed - a
model may take as long as it likes over a bottle - so this trains purely for survival. Each
genome plays three whole games. A run has no generation limit; it ends when a candidate clears
every bottle up to level 20 on all three of its training seeds and then proves it on five seeds
it has never played, and carries on training from that candidate if it cannot. `ga dr tune` runs
the same thing seeded from the built in model instead of from scratch.

The weights currently embedded are **random, not trained**: the Dr. Rustario opponent and demo
will not play well until a `ga dr auto` run replaces them.