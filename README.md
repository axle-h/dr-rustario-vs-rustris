# Dr. Rustario vs. Rustris

A multi-themed Tetris vs Dr.Mario clone. Written in SDL2 and Rust for fun:

* **Dr. Rustario** - a Dr. Mario clone (NES, SNES, N64 and particle themes)
* **Rustris** - Tetris with the guideline ruleset (Game Boy, NES, SNES and particle themes)
* **Puyo Rusto** - Puyo Puyo Tsu (Genesis, SNES and particle themes)
* **Dr. Rustario vs Rustris** - play a multi-player focussed playlist over both games.

Puyo Rusto is the newest of them and is not finished: it is playable by one or two people on
all three of its themes, with its own high score tables and its own ai, but it does not take a
turn in the vs. playlists yet. The retro themes are the two 16-bit games Compile's original
became: **genesis** is Dr. Robotnik's Mean Bean Machine and **snes** is Kirby's Avalanche.
Each keeps its own game's furniture and puts the queue and the nuisance tray in the boxes
that game drew for them - Mean Bean Machine's two `NEXT` boxes and the box it keeps
Robotnik's mugshot in, Kirby's two name plates and the arch at the foot of its centre column.
Both panels stop level with the top of their own field, so the row a pair spawns in - which is
played in, and so is drawn - sits above the board against the backdrop with nothing to either
side, the way a piece does on the retro Rustris themes. On **genesis** a character stands in
the box the game keeps Robotnik's mugshot in and reacts to how you are doing - one of the
thirteen the game draws, dealt at the start of a match, and in a two player game each faces the
other player's board. They pull a face when you chain and another when you are nearly buried,
and they have their tics: Frankly's antennae throw sparks when he is winning, Humpty crackles an
arc between his and wrings his hands when he is not, Arms' ring of lights breathes while nothing
is happening and spins once it is, Coconuts' coin flares white and flushes red, and every one of
them sweats when your stack gets near the top. They stand on a wash of their own game's own colour rather than on
its tiled wall, and each casts a shadow on it, which is what leaves the board looking like a
board. Its particle theme
carries eleven sets
of puyos and deals a different one to each player at the start of every match, so a two player
game is never two boards of the same puyos and no two matches look alike; the title screen
sends all eleven past. It has a soundtrack the same way: one track over its menus, and four
more that a match is dealt one of - the track dealt at the start of a match plays to the end
of it, changing only if the theme does. The **genesis** theme has a soundtrack of its own:
Mean Bean Machine's four stage tunes, each with the lead-in the game writes as a separate
track, and its sound effects with them - while a refugee bean sitting alone in the well blinks
every couple of seconds. Every step of a chain counts itself out over the puyos it just took,
in the game's own face rather than the engine's.

It also *moves* the way the game does. A group about to go flashes where it stands, joined to
its neighbours and all, before it pulls a face, curls into a ball and bursts - and the droplets
it throws leave the cell they came from and are still in the air when the next step of the
chain starts. Every bean squashes where it lands, so the two halves of a pair jolt separately
and a slab of nuisance arrives one column at a time. Nuisance falls under gravity and raggedly,
each column a little behind the last, rather than dropping as a level wall. An attack crosses
the window as the game's own ball - a white core inside a rim in the sending player's colour,
red for player one and blue for player two - arcing over to burst on the other player's tray
and leave a refugee bean in it. The tray is on the wall over the board where Mean Bean Machine
puts it, stacked in from the right so the pair coming out of the spawn never covers it, and
the new icon slides into place out of the burst as the ball lands. Nothing shakes: the original does not, and what reads as a rumble there is every bean
bouncing at once. The particle theme, which owes nobody fidelity, shakes anyway.

Rustris follows the guideline: the Super Rotation System and its wall kicks, a seven bag
randomiser, hold, extended placement lock down, and guideline scoring. A T rotated into a slot
with three of the four corners of its bounding box filled is a T-spin - a mini when only one of
the corners in front of its stem is filled, unless it took the last of the wall kicks to get
there - and scores as one whether it clears a line or not. A difficult clear (a tetris or any
T-spin) straight after another is worth back to back, which a piece that clears nothing does not
break. Clears that chain build a combo, and emptying the board is a perfect clear. In a
two player game every one of those sends its own garbage.

## Garbage between the two games

Against your own game an attack is what that game has always sent: rows of garbage between two
Rustris players, garbage blocks between two Dr. Rustario players. Across the two, though, the
units are not the same thing and neither is the work behind them - a Dr. Rustario bottle is
eight wide and its blocks only come out by matching four of a colour, so a row per row would
bury one in seconds - so an attack carries a second size for the other game, and the sender
prices it:

* **Rustris to Dr. Rustario**: only the clears worth working for cross at all. A tetris sends
  two garbage blocks - the size of the combo a Dr. Rustario player sends most often - a T-spin
  double two and a T-spin triple three, and a perfect clear four, whatever cleared it. Singles,
  doubles, triples, T-spin singles, minis, combos and back to back send their rows at home as
  they always did and nothing abroad.
* **Dr. Rustario to Rustris**: a combo sends a row per pattern past the first, up to four. Most
  combos are two patterns - one pill finishing two lines at once - which is nowhere near the
  work a Rustris player puts into a row, so it buys one; a real chain still hurts.

Which sizes those are was measured rather than guessed at: each game's own ai played five seeds
at full speed for fifty minutes of game time, counting what it sent.

| sending | to its own game | to the other game |
|--|--|--|
| Rustris, survival model | 32 rows/min | 0.2 blocks/min |
| Rustris, high scoring model | 144 rows/min | 55 blocks/min |
| Dr. Rustario, the N64 ai | 13 blocks/min | 7 rows/min |

A minute of the high scoring model is twenty seven tetrises, which is not a human rate; a human
tetris every fifteen seconds or so is four blocks a minute, next to the seven rows a minute a
Dr. Rustario player sends back. Row for row it used to be 144 garbage blocks a minute into an
eight wide bottle.

## Building

SDL2 is the only native dependency, and there are two ways of finding it - a feature apiece on
the `dr-rustario-vs-rustris` package in [launcher/](launcher), which is the workspace's default
member, so every command below is run from the repository root:

* `pkgconfig` (the default) - link the SDL2 the system already has, through `pkg-config`. This
  is the Linux and macOS route.
* `vcpkg` - build SDL2 from source with [vcpkg](https://vcpkg.io) and link it statically, so
  the binary carries it and needs no SDL2 beside it. This is the Windows route.

Whichever it is, pass `--no-default-features` with it: the two are alternatives rather than
additions, and with both on SDL2's build script probes both - which on Windows means a
`pkg-config` probe that panics the build before vcpkg is ever reached.

All resources are embedded into the binary, including every game's AI opponent, demo mode and
the `ga` training subcommand (`dr-rustario-vs-rustris ga [auto|survival|score|diagnose]`).
`ga play <seed> [line cap] [report every n lines] [survival|tetris]` plays a built-in model
headless on a fixed seed, reporting progress; it counts lines and banks the score itself as the
in-game counters are capped. `ga dr ...` trains and inspects Dr. Rustario instead - see
[Training Dr. Rustario](#training-dr-rustario) for the whole of it - and
`ga dr play <seed> [virus level] [pill cap] [report every n pills] [brain]` plays it headless,
where the brain is `n64` (the default), `n64:0` to `n64:5` to pick one of the N64 ai's own rows
of weights, `neural` for the trained network or `linear` for the hand written baseline.
`ga puyo ...` plays and ranks Puyo Rusto, which trains nothing: `ga puyo play <seed>
[difficulty] [pair cap] [report every n pairs] [brain]` plays one of its six skill rows
headless, and `ga puyo rank [seeds] [pair cap] [difficulty]` plays every row over the same
seeds and prints the ranking the four difficulties are picked out of.

### Windows

Needs the MSVC toolchain (Visual Studio Build Tools with the C++ workload) and `git` on the
path - vcpkg compiles SDL2 itself, so the first run takes a while. In a developer prompt:

```powershell
rustup default stable-x86_64-pc-windows-msvc
cargo install cargo-vcpkg
cargo vcpkg build --manifest-path launcher/Cargo.toml
cargo build --release --no-default-features --features vcpkg
```

`--manifest-path` is not optional: cargo-vcpkg runs against a *package* and the manifest at the
root of the repository is a virtual one (a `[workspace]` with no `[package]` of its own), which
is what `cannot run on a virtual manifest` means when it is left off. The vcpkg metadata it
reads therefore lives in [launcher/Cargo.toml](launcher/Cargo.toml) rather than in the
workspace manifest, since cargo-vcpkg reads `package.metadata` and never `workspace.metadata`.

Nothing needs to be set in the environment: the tree lands in `target\vcpkg` and the SDL2 build
script walks up from its own output directory to find it, taking the `x64-windows-static-md`
triplet the manifest declares. Both commands do have to see the same `CARGO_TARGET_DIR`, since
that is the tree it walks up to. The result is
`target\release\dr-rustario-vs-rustris.exe`, with SDL2 inside it.

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
list. It is also what a match anyone is playing opens on, which is why it heads the sprints:
a stage on each theme in turn is the mode that shows a game off. An ai demo opens on the
marathon instead, being a thing to watch rather than a race, and a single theme leaves a
lone player the marathon and two players the 1 level sprint. Marathon tables rank the
highest scores; sprint games (level, theme and point sprints, and the vs. races: the
theme sprint, which is first to the end of the playlist, and the 3, 5 and 10 level random
sprints, which deal a random game and theme each level) race a single clock, shown in the
bottom-left corner (single player) or bottom-centre (multiplayer), and their tables rank
the fastest times. The interleaved,
back to back, retro, particle and random marathon vs. playlists are marathons: the first
two cycle their playlist endlessly, the interleaved playlist carrying each game on through
its own themes as its turn comes round again, the retro playlist alternates the games
through their retro themes only and the particle playlist alternates them on their
particle themes, the random marathon deals random games and themes forever, and all rank
scores. Every vs.
playlist shares one difficulty dial, 0 (default) to 10: it sets
Dr. Rustario's virus level and fall speed and Rustris's starting level together, from no
viruses at low speed on level 0 up to virus level 10 at high speed on level 10. In a
random playlist every player faces the same random sequence, dealt fresh each match. Every
player is also dealt the same game: one seed per match decides Dr. Rustario's bottles and
pills and Rustris's pieces for everyone, so however far apart the playlist has moved two
players, and whatever garbage they have been sent, they play the same bottles and the same
bag. The clock runs while anyone is playing: it stops while paused and when every player is
held up at once, so in single player a stage-clear card or theme change does not count
against you, while in a multiplayer race the clock keeps running.
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
clear, `T-SPIN` on a Rustris T-spin, `PERFECT` on an emptied Rustris board, `COMBO` on a
Dr. Rustario combo and `GAME OVER` when someone is buried; left to
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

All three games find every placement the piece in play can reach, score them, and hand the best
one to an agent that presses the keys. Rustris scores with a small neural network;
Dr. Rustario plays a port of Dr. Mario 64's own hand written scorer, and has a neural network
that is trained but not yet strong enough to field as a difficulty; Puyo Rusto searches several
pairs ahead with a beam search over a hand written evaluation, and has no neural model at all.
The network and the genetic algorithm that trains it are shared in `engine/src/ai`; each game
supplies its own features, placement search and agent. Only human players can enter the high
score table.

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

The **ai** option on a Dr. Rustario main menu offers the same choices:

* `off` - human players.
* `vs easy` / `vs normal` / `vs hard` / `vs impossible` - in a 2-player match the AI plays as player 2
  (who must be on Dr. Rustario), speed limited to one key every 500 ms / 400 ms / 300 ms / instantly
  (see `AiDifficulty` in `dr-rustario/src/game/rules.rs`). Every difficulty plays Dr. Mario 64's own
  scorer, but on a different one of its six rows of weights - the one dial the original ai has - so a
  harder setting is a better player and not merely a faster one: `easy` and `normal` play the two rows
  that hardly get past the first few bottles, `hard` the runner up and `impossible` the best of them.
* `1-player ai demo` - the first player's bottle is played by the AI at full speed; their controls are
  disabled. It plays the same scorer, on the row `DrAiKind::default()` hands out.
* `2-player ai demo` - the AI plays both bottles at full speed: the trained neural network as player 1
  against the best of the N64 ai's rows of weights as player 2. That demo is the only place the
  network plays.

#### Dr. Mario 64's own ai

What every difficulty and the 1-player demo play - and what defends player 2 in the 2-player
demo - is a port of `aiset.c` from the Nintendo 64 game's decompilation, in
`dr-rustario/src/game/ai/n64`: a deterministic scorer with no learning in it anywhere. For
every place the pill can come to rest it drops the two halves into a copy of the bottle,
measures the run of colour each one lands in, takes away whatever clears, measures what is
left, asks whether the bottle it leaves behind would chain if the next pill were any of the
three colours, and adds the answers up with a table of weights. The highest total wins.

A run is measured twice over - once as the cells actually touching, which is what clears, and
once as the cells within reach counting the gaps a later pill could still fill, which is what a
line could become - and it is the second that makes it build towards clears rather than only
taking the ones in front of it. The weights are not fixed: the original picks a *skill* row and
a *situation* column, and the situation is read off the bottle at the start of every pill -
whether there is room left to move, whether the end is in sight, whether a column of one colour
is building in the middle - which is what makes it play differently with a bottle full of
viruses than with two left in the corner. The six skill rows are personalities rather than a
ladder - the original picks one per character, not per skill setting - so which of them is the
better player was measured rather than assumed: each row played twenty seeds at virus levels 5,
10, 15 and 20, ranked on bottles cleared less four per burial. That ranking (`SKILL_ORDER` in
`dr-rustario/src/game/ai/n64/params.rs`) is what the four difficulties pick from, and what the
2-player demo takes the best of them from.

Left out of the port: the sixteen characters and the moods that nudge their weights about, the
deliberate mistakes, and the frame level key pacing, which the engine's own key pacer already
does. The candidates come from Dr. Rustario's placement search rather than the original's, so
this game's wall kicks are honoured; only the scoring is the N64's.

#### The neural model

What the model reads was not guessed at: `ga dr probe` plays the deterministic ai above,
records what it made of *every* placement it was offered, and measures how much of that opinion
a set of features can reproduce - as a linear fit, and then by cloning the ai onto them with
gradient descent and sending the clone out to play whole games. The features are what came out
of that.

Twenty nine inputs, feeding two hidden layers of twenty nine, the same architecture the Rustris
model trained well at. Twenty five of them are *comparative* and are centred on the mean over
the placements of the pill in play, since a scorer only ever has to separate those from each
other: how the bottle moved (viruses, the work still needed to clear every virus, blocks buried
under other colours counted apart for viruses and everything else, the tallest column, the holes
under the stack, and runs one and two short of a match counted separately along a row and down a
column), and what the placement itself did (what it cleared, the run each half landed in and how
long that run could still become, halves left one and two short with room to finish, halves left
where no line can ever join them, what it did to a virus underneath it, and how many ways the
next pill could clear something in the bottle it leaves behind). The last four are *context* and
are deliberately not centred - the viruses, work, height and holes of the bottle before the pill
- because the N64 ai runs what amounts to two opposite policies, one while it is digging a full
bottle out and another once the end is in sight, and a network with no idea which it is in can
only learn the average of the two.

Three of those carry most of the weight. The **work** count asks, for every virus, how many
matching blocks a line of four through it still needs, and counts a virus nothing can reach any
more as worse than any reachable one; it is what points the agent at the viruses instead of at a
tidy heap in the corner. **Room** always means room a pill can actually get a half into, so a run
of three with its only gap under an overhang is junk rather than a threat. And a **stranded**
half - one in a run that can never reach four - is the single thing the N64 ai weighs most
heavily: taking it out of the original changes its mind on 44% of pills. Tucking sideways under
an overhang is not searched, since the agent has no single step soft drop to execute it with.

The agent does not reach for the pill in hold, and that is deliberate. It learns to rank
placements from the deterministic ai, which has no hold and scores a *placement* against the
bottle rather than one pill against another - so asking it which of two pills to play gives an
answer that sounds reasonable and is not. Taught with hold on offer a model plays 25 viruses
over five games and buries itself in every one; taught without it, the same model plays 3143 and
finishes 80 bottles. Hold is worth having back, but only behind something that can judge it.

The weights currently embedded are a **survival trained** model, in `models::survival_trained()`:
4635 viruses, 101 bottles and 18615 pills over five games, buried in three of them. It is not
good enough to field as a difficulty yet - watching it play is what settled that - so the four
difficulties and the 1-player demo all play the deterministic ai above, and the network's one
outing is as player 1 of the 2-player demo, against the best row of weights the N64 ai has. To
watch it on a seed of your own, run
`ga dr play <seed> <level> <pill cap> <report every> neural`.

#### Training Dr. Rustario

`ga dr auto` runs the whole thing: three stages, in order, each starting from what the last left
behind. It takes hours, prints as it goes, and ends by printing the weights to paste into the
binary.

| stage | what it optimises | how it ends |
|--|--|--|
| **1. imitation** | ranking placements the way the deterministic ai ranks them | when the lessons have been learned; no game is played |
| **2. survival** | viruses destroyed before being buried, from the first bottle up | when a candidate finishes the run: one of its four seeds out of bottle 30, and every other one of them at least as far as bottle 20 |
| **3. efficiency** | bottles finished within a budget of 3000 pills | after 150 generations - there is always a faster model, so this one is bounded by count |

**Stage one** exists because a genetic algorithm can only select between members it can tell
apart, and from random weights it cannot: 20 of 24 random genomes clear no virus at all, so the
fitness meant to rank them is zero for most of the population. So the network is taught first,
by gradient descent, to reproduce what the deterministic ai thought of every placement it was
offered over ten thousand pills.

It keeps teaching until one of them plays 1500 viruses over five whole games, because how well a
clone reproduces the ai and how well it *plays* are not the same measure: four clones of one
corpus agreed with it on 53% to 56% of pills and played anywhere between 291 and 2526 viruses.
Only the initial weights are drawn again - the corpus is deterministic, and re-gathering it
would only cost time. About half of all clones clear the bar, so it rarely takes more than two
or three; if 25 of them cannot, it says so and goes on with the best. A taught model clears
around 2500 viruses over five whole games where a random one clears none, and stage two's first
generation opens at 800 viruses instead of 2.

**Stage two** is the one that used to be all of training. It plays the game and counts viruses,
with no pill budget and no reward for speed, so it trains purely for staying alive. Seeded from
a taught model it mutates gently (3-8% of genes, by 0.05) rather than widely (10-20%, by 0.1):
there is a great deal to preserve, and at the wide rates the median member of a taught
population scores a twentieth of the model it came from.

What ends it is the **finish line**, and the finish line is inside the fitness rather than
bolted on after it, so what training selects for and what stops training are the same thing. A
candidate plays four seeds, and it has finished the run when one of them came out of the last
bottle and every other one of them got at least as far as bottle 20. Both halves of that are
there for a reason. Asking every seed to clear the whole game is a lottery rather than a test -
a model good enough to top its generation cleared a fresh seed about one time in five, so five
of five is a one in three thousand event, and a whole night of training lost it 516 times out of
516. And asking only for an average would take a model that is lucky once over one that is
reliable four times.

The last bottle is 30 rather than 20 because stopping at 20 put a ceiling on the measure that
the best member reached in its *first* generation and then sat on: over a 641 generation run the
best member scored the exact maximum in 518 of them. Above a ceiling the fitness cannot tell its
candidates apart, so selection at the top was a random walk for nine hours - the best genome
differed from the previous generation's in 619 of 640 - and the median crept up 80 viruses while
the leader learned nothing at all. The bottles past 20 are not more of the same, either: level
19 and up confine their viruses to the top three rows, and level 24 up carries the game's
maximum of 99. There is a long way to climb up there. The deterministic ai the whole thing
learns from dies around bottle 19 or 20 itself, and got as far as 25 once in six seeds.

**Stage three** asks a model that has stopped dying to stop dawdling. Same game, but the clock
is a pill budget and the score is bottles finished, so taking the clear in front of it beats
tidying, and finishing a bottle in three hundred pills beats nine hundred. The budget is three
thousand, which at the pills a bottle a good model takes reaches somewhere around bottle twenty
of the thirty one stage two asks for; it was half that while the run stopped at bottle twenty. Survival is not
thrown away by it - a model that buries itself finishes no more bottles - but it is checked
afterwards all the same, and if the sweep is lost the run says so and prints the stage two model
as the one to embed.

Every stage can also be run on its own:

```shell
ga dr auto                                  # all three, which is a training run
ga dr pretrain [pills] [threshold]          # stage one alone, printing the weights it learned
ga dr survive                               # stage two alone, from random weights
ga dr tune                                  # stages two and three, from the embedded model
ga dr trial [population] [generations] [stage]   # a short bounded run that trains nothing
ga dr diagnose                              # play the embedded model on five unseen seeds
ga dr probe [seeds] [level] [pills]         # what the deterministic ai is paying for, and how
                                            # much of it the features can express
```

`ga dr trial` is the one to reach for after changing the features, the fitness or the teaching:
a small population over a handful of generations, reporting every one, which is enough to see
whether the algorithm has been left anything it can climb. Its `stage` is `scratch` (the
default: stage two from random weights), `taught` (stage two from a quick imitation seed) or
`efficiency` (stage three from one). It runs to its generation count and stops; nothing about
it finishes a run.

**Getting the result into the binary.** Every stage that finishes prints its weights as the body
of `survival_trained`, ready to paste over the one in
[dr-rustario/src/game/ai/models.rs](dr-rustario/src/game/ai/models.rs):

```
// the model to embed: paste this over the body of models::survival_trained
    DrNeuralNetwork::new(&[
        0.051050, -0.697642, ...
    ])
```

Nothing is written to disk except a `generation-record-<timestamp>.csv` in the working
directory, one per phase, holding each generation's statistics and its best genome as the same
weights.

**What a generation costs** is `POPULATION` times `SEEDS_PER_GAME` whole games, and a good
model's game runs to thousands of pills - so those two are the dials to reach for if training is
too slow. Two seeds measured luck rather than skill: taking the best of 250 candidates over 2
seeds is an extreme of 250 noisy samples, so *somebody* cleared both every generation whatever
the population was worth. Four is dearer, which is what the **probe seeds** are for - a
candidate plays two, and only one averaging at least `ABANDON_BELOW` viruses over them is played
out on the other two. A candidate that is cut is still averaged over all four seeds it was
given rather than the two it played, so being cut can only ever cost it and can never lift it
above a candidate that went the distance. At 250 candidates over 4 seeds a generation takes
around a minute on a desktop.

**The other knobs**, all constants at the top of
[dr-rustario/src/game/ai/genetic.rs](dr-rustario/src/game/ai/genetic.rs) and
[imitation.rs](dr-rustario/src/game/ai/imitation.rs): `LESSON_PILLS` (how many pills stage one
learns from), `TAUGHT_ENOUGH` and `PRETRAIN_ATTEMPTS` (how well a taught network has to play
before stage two starts from it, and how many tries it gets), `PILL_BUDGET` and
`EFFICIENCY_GENERATIONS` (stage three), and, in [run.rs](dr-rustario/src/game/ai/run.rs),
`TOP_TRAINING_LEVEL` and `PROVEN_LEVEL` (the two halves of the finish line) with `PROBE_SEEDS`
and `ABANDON_BELOW` (when a candidate is cut short).

### Puyo Rusto

The **ai** option on a Puyo Rusto main menu offers the same choices, and every difficulty is a
different player rather than the same one hurried along:

* `off` - human players.
* `vs easy` / `vs normal` / `vs hard` / `vs impossible` - in a 2-player match the AI plays as
  player 2, speed limited to one key every 500 ms / 400 ms / 300 ms / instantly, on one of six
  rows of settings that get progressively better at the game.
* `1-player ai demo` - the first player's board is played by the best row at full speed.
* `2-player ai demo` - both boards at full speed: the second best row against the best.

#### How it plays

There is no decompilation to port here, which is the one way this differs from Dr. Rustario.
Puyo Puyo's own cpu opponents have never been reverse engineered into anything readable - Mean
Bean Machine's lives in an unlabelled 68000 disassembly, and Puyo VS's own is two hundred
lines that take the biggest chain in front of them and otherwise place at random. So this one
is built out of the open literature, and mostly out of
[ama](https://github.com/citrus610/ama), the strongest open Puyo Puyo Tsu ai.

For every placement of the pair in play it drops the two halves, runs the chain, and scores
the board that is left; then it plays the next pair onto the best boards, and the one after
that, keeping only the best handful at each step. What it can see is what a player can see -
the pair in play and the two behind it - and past those it carries on down *invented*
continuations, since guessing which colours come next costs nothing when the question being
asked is whether there is still room to build.

The board is scored on fifteen numbers, and the one that matters is not on the board at all.
A player who is building a chain almost never fires anything, so scoring what a placement
clears says nothing about nearly every placement on offer. Instead it asks what chain the
board is *holding*: for every column the pair could still be carried over, and every colour
already down there, it drops puyos one at a time until a group of four forms, runs the chain
out, and takes the best answer. How long that chain runs, how many puyos it would take to set
off, how much room is left to stretch it further, and how much of the board survives it are
four of the fifteen. The rest keep the field in a state where such an answer keeps existing:
its shape, its wells and towers, its pairs and threes, nuisance sitting on it, and how much
room is left over the spawn column.

The hidden thirteenth row is handled twice over, because it is two different things. Nothing
up there pops or counts towards a group, so it is nobody's neighbour when the chain loop looks
for one. And a puyo resting up there is a *door closed* - a pair moves sideways with one half
in that row, so everything past it is out of reach however empty the column below looks, which
is both a penalty and the bound on where the search will look at all.

The six rows differ in how far they see, how many boards they hold in mind, and - the part that
changes the game rather than the strength - how big a chain they will hold out for before
firing it. Which of them is the better player was measured rather than assumed: every row
played the same twelve seeds for six hundred pairs and was ranked on the score it banked.

| row | score/pair | best chain | nuisance sent |
|--|--|--|--|
| greedy - takes every clear it sees |  48.4 |  4 |  5,126 |
| tidy - holds out for a small chain | 191.1 |  7 | 19,706 |
| swift - plays for a fast second chain | 284.4 |  8 | 29,550 |
| builder - builds properly | 433.0 | 10 | 44,567 |
| patient - and waits for a whole rock | 571.9 | 12 | 58,822 |
| sharp - and for one that decides a match | 761.8 | 12 | 78,351 |

`easy` and `normal` play the two weakest, `hard` the runner up and `impossible` the best. There
is no neural model: Puyo Rusto is the one game here whose scorer was never asked to teach one.

It thinks a piece at a time. The hardest row takes about ten milliseconds to decide where a
pair goes, which is most of a frame on a desktop and rather more than one on a handheld - but
nothing is waiting for the answer until the pair lands, and a pair takes a second to fall. So
the search is stepped once a frame and the answer taken when it is ready, which is the same
search at under a millisecond a frame rather than ten in one. Two things follow: the pair has
fallen by the time it has an answer, so the keys are worked out again from where it is; and if
the board is too full for the pair to fall far, the search is cut short and plays the best it
had got to, which is why every placement is scored before the first step.

### Dr. Rustario vs. Rustris

The vs. mode offers the same ai modes, and every one of them is the games' own. A playlist
deals both games, so an ai player is a *pair* of brains - a Dr. Rustario one and a Rustris one -
and each is whatever that game would field for the mode chosen; the agent handed a board plays
whichever game the playlist has just dealt it, and forgets what it had queued for the board that
has gone.

* `off` - human players.
* `vs easy` / `vs normal` / `vs hard` / `vs impossible` - the ai plays as player 2, at each game's
  own difficulty of that name: the same models and the same key rates it would play at in that
  game on its own.
* `1-player ai demo` - one board, played by the ai at full speed through the whole playlist.
* `2-player ai demo` - both boards at full speed, each game fielding the two models it puts against
  each other in its own 2-player demo: Rustris's survival model against its high scoring one, and
  Dr. Rustario's trained network against the best of the N64 ai's rows of weights.

The vs. **difficulty** dial is a separate thing that applies to everyone: it is how hard the games
are, not how well the ai plays them.
