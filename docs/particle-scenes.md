# Particle scenes: a plan for the modern themes' background

A design and staged implementation plan for turning the modern themes' particle background from
a single drifting emitter into a choreographed, colourful, game-reactive backdrop.

Written against `906410b`. Line numbers below were verified at that commit and will drift — treat
them as "look here", not as gospel, and re-grep for the symbol name if a line does not match.

---

## 1. Scope

**In scope.** The background particle field behind a match, for players on a **modern** theme
(the theme literally named `"modern"` in each game — `rustris/src/theme/modern/mod.rs:63`,
`dr-rustario/src/theme/modern/mod.rs:121`). That is the only theme whose scene is
`SceneType::Particles`; every retro theme uses `Solid`, `Checkerboard` or `Tile`.

**Selecting it to test:** the two games label it differently in their menus. Rustris calls it
**`modern`** (`rustris/src/game/rules.rs:22`); Dr. Rustario calls it **`particle`**
(`dr-rustario/src/game/rules.rs:27`). Both build the same `modern_theme`. Do not be caught out
looking for a "modern" option in the Dr. Rustario menu.

**Out of scope, deliberately.**

- **The menus.** The title/menu piece race (`prescribed_piece_race`) and the high-score/name-entry
  fireworks (`prescribed_fireworks`) keep the existing emitter model and are not touched.
- **Foreground per-player effects.** Line-clear sweeps, hard-drop debris, spawn bursts, perimeter
  sprays — everything driven from `SceneRender::emit_particles` into `fg_particles` — stay exactly
  as they are. They are already per-player and already work.
- **Retro themes.** Nothing here ever renders over a retro half of the screen.
- **Game rules.** Nothing in this plan may read or write game state in a way that affects play. The
  field observes; it never influences. No RNG shared with the games.

---

## 2. How it works today

### 2.1 The particle engine

`engine/src/particles/` is a **fire-and-forget emitter model**:

| file | what it holds |
|---|---|
| `mod.rs` | `Particles`: `Vec<ParticleGroup>` + `Vec<Box<dyn ParticleSource>>`, a `max_particles` budget, and the update loop |
| `particle.rs` | `Particle` (position, velocity, acceleration, alpha, colour, TTL, sprite, size, rotation, animation frame) and `ParticleGroup` (lifetime, `anchor_for`, `fade_in`, `fade_out`, `orbit`, particles) |
| `source.rs` | `ParticleSource` trait, `RandomParticleSource` (the builder-style workhorse), `AggregateParticleSource`, `ParticlePositionSource`, `ParticleModulation` |
| `prescribed.rs` | the concrete named effects: `prescribed_orbit`, `prescribed_fireworks`, `prescribed_piece_race`, `PrescribedParticles`, `PlayerTargetedParticles`, `RaceTheme` |
| `meta.rs` | `ParticleSprite`: 50 atlas sprites plus `Piece`/`Cell`/`Mascot` variants that index a theme's own sprite sheets |
| `render.rs` | `ParticleRender`: owns the atlas texture, the per-theme flattened sprite sheets and the `Particles`; draws every particle |
| `scale.rs` | `Scale`: converts window pixels ↔ **particle space**, a 0–1 box over the whole window |
| `geometry.rs`, `color.rs`, `quantity.rs` | `Vec2D`/`RectF`, `ParticleColor` (f64 RGB), `VariableQuantity`/`ProbabilityTable` |

The critical property: **a source emits groups and then has no further say.** A `ParticleGroup`
evolves on its own — velocity + acceleration, one optional gravity well (`orbit`), an optional
`anchor_for` freeze, fade in/out, a sine `pulse` on alpha — until each particle's TTL expires or it
leaves the 0–1 box heading outward (`Particle::is_escaped`). *Nothing can retarget a particle after
it is born.* Every idea in this document needs that to change.

Particle positions are in **particle space**: `Vec2D` in 0–1 across the whole window, converted at
draw time by `Scale::point_to_render_space`. Keep that convention.

### 2.2 The background today

The whole modern background is **one source**: `prescribed_orbit`
(`engine/src/particles/prescribed.rs:277`). It splits the window into quadrants and gives each an
`orbit_source` (`:289`) emitting 10 particles per second, ~10 s lifetimes, all pulled toward a
single gravity point at screen centre with a `1/r²` approximation, 80% plain circles / 10% hollow
circles / 10% stars, with a slow alpha pulse.

Steady-state population ≈ **4 sources × 10/s × 10 s = 400 particles**. `MAX_BACKGROUND_PARTICLES`
is 100 000 (`engine/src/app/mod.rs:32`), so the budget is not the constraint; draw calls are.

### 2.3 Wiring

- `launcher/src/shell.rs:123-129` builds two `ParticleRender`s that live for the whole process:
  `fg_particles` (no theme sprites) and `bg_particles` (**all** themes' sprites).
- `MatchScreen::new` clears both and adds the orbit source: `match_screen.rs:118`.
- `MatchScreen::update` ticks them only when unpaused (`:568`), and ticks `bg_particles` only if
  some player is on a particle scene (`:571`, gated by `ThemeContext::render_scene_particles`).
- Draw order inside the frame buffer (`match_screen.rs:~625-645`):
  1. `themes.draw_scene(...)` — per-player scene backgrounds; a particle scene draws nothing
  2. `themes.draw_scene_particles(c, bg_particles)` — **the background field**
  3. `themes.draw_players(...)` — boards, HUD, mascots
  4. the sprint timer
  5. `fg_particles.draw(c)` — event bursts, on top of the boards
  6. the paused overlay
- `ThemeContext::draw_scene_particles` (`engine/src/render/context.rs:609`) loops players, and for
  each one whose theme is a particle scene sets `canvas.set_clip_rect(player_clip)` and draws the
  **entire** field. With two modern players that draws every particle twice, once per clip.

### 2.4 Geometry facts that matter

- `MAX_PLAYERS = 2` (`launcher/src/shell.rs:22`).
- `Scale::player_clip` (`engine/src/scale.rs:82`) makes clips **vertical slices that tile the
  window**: player *i* owns `[w·i/n, w·(i+1)/n) × full height`.
- Therefore **the union of any set of player clips is one contiguous rect**: the whole window, the
  left half, or the right half. This is what makes the canvas model in §5.2 simple. *If MAX_PLAYERS
  ever exceeds 2, revisit it* — a non-particle player between two particle players would break
  contiguity and the field would need multiple clips again.
- `ThemeContext::player_board_snip(player)` (`context.rs:303`) gives the on-screen rect of **any**
  player's playfield, including one we never draw over. That is what makes off-canvas attack
  endpoints possible.
- `ThemeContext::player_renders_scene_particles(player)` (`:735`) and `render_scene_particles()`
  (`:740`) already answer "is this player on a particle scene".

### 2.5 Events available

`GameEvent` (`engine/src/game/mod.rs:115`): `Move`, `Rotate`, `Hold`, `SoftDrop`, `Fall`, `Spawn`,
`Spawned`, `HardDrop { cells, dropped_rows }`, `Lock { cells, dropped }`,
`Clear { cells, count, is_combo }`, `Settle`, `AttackSent(Attack)`, `AttackReceived { cells }`,
`SpeedUp`, `StageComplete`, `GameOver`, `Victory`, `Paused`, `UnPaused`, `NextTheme`.

`Attack { origin: GameId, strength: u32, detail: u64 }` (`:65`). `count` on `Clear` is the game's
own measure — lines for Rustris, cleared patterns for Dr. Rustario — and
`GameRender::clear_class(&event)` (`engine/src/render/mod.rs:39`, overridden per game in
`rustris/src/render.rs:9` and `dr-rustario/src/render.rs:10`) grades it for audio; reuse that
grading rather than inventing another.

Events reach the renderer in `match_screen.rs:~371-395`, already tagged with the player that
caused them.

**The attack's victim is not currently visible to the renderer.** `Session::send_attack`
(`engine/src/session.rs:387`) picks a random other player internally and only `AttackSent(Attack)`
surfaces. Stage 6 fixes this.

---

## 3. Decisions

These were settled in discussion with Alex. They are decisions, not suggestions — if one turns out
to be wrong in practice, raise it rather than quietly doing something else.

1. **One shared field, not per-player fields.** A single retained particle pool spanning both
   players, so full-screen effects are possible. "Per-player particles" means the *foreground*
   effects, which already exist and are not being changed.
2. **Nothing renders over a retro half.** The field is clipped to the players on a particle scene.
3. **When only part of the screen is visible, effects generalise to the visible area.** In a
   2-player match with one modern and one retro player, routines lay out inside the modern
   player's half rather than over the whole window with half of it clipped away. This is the
   *canvas* concept in §5.2.
4. **Sprites are drawn as edges only** — the outline of a silhouette, never a filled shape.
5. **The sprite set is the union of the games being played.** 1 player: that game. 2 players, both
   Rustris: Rustris shapes only. Both Dr. Rustario: Dr. Rustario shapes only. One of each: a
   mixture. Driven by which *games* are playing, not by which themes.
6. **Attacks are events queued into the same field.** No separate attack layer — there is no
   ownership or layering reason for one.
7. **Every player's events feed the field, regardless of their theme.** A retro player's line clear
   still ripples through the visible modern half; that reads correctly as "something happened over
   there". This means event routing needs no theme check at all.
8. **Roughly 30% of the pool stays ambient** while a feature routine runs, so the field never looks
   empty or fully hijacked.
9. **Colour should be dynamic and pleasing, not a fixed palette.** Both modern themes are currently
   `particle_color: Color::WHITE`, so this is a real change in look, and it is wanted.
10. **Lines are textured quads, not a rasteriser.** No SDL_gfx, no new dependency.
11. **Density is configurable** through `VideoConfig`.
12. **The wasm build is a demo** and may assume full density. **PortMaster handhelds are the
    binding performance constraint.**

---

## 4. Constraints and gotchas

### 4.1 Draw calls are the limit

Every particle is `set_color_mod` + `set_alpha_mod` + `copy` on a shared texture
(`particles/render.rs`). The per-particle colour mod **defeats SDL's renderer batching** — SDL only
batches copies that share texture *and* colour/alpha mod. This is already true today at ~400
particles drawn twice.

Mitigations, in order of value:
- Edges-only silhouettes (a ~400 px tall tetromino outline at 5 px spacing is roughly **160
  points**, not thousands — this is the single biggest reason the plan is affordable).
- One clip and one draw pass instead of one per player (§5.2 gives this for free).
- Sort particles by colour before drawing, or quantise colour to a small palette, so runs of
  identical colour mod batch.
- The density dial (§5.7).

Budget for orientation: today ≈ 400 particles drawn twice = 800 copies. A full-density field of
~600 drawn once is fewer, with much more happening.

### 4.2 Platform

- **PortMaster / aarch64 handhelds** link the *firmware's* `libSDL2-2.0.so.0`, not a bundled one.
  A symbol the firmware's SDL lacks is an **undefined symbol at load time — the binary will not
  start**, it does not degrade gracefully. So: **do not call `Canvas::render_geometry`**
  (`SDL_RenderGeometry`, SDL ≥ 2.0.18) even though sdl2 0.38 exposes it. `SDL_RenderDrawLine`,
  `SDL_RenderCopyEx` and `SDL_RenderFillRect` are all SDL 2.0.0 and safe.
- **wasm/emscripten.** `read_pixels` on a *render-target texture* (via `with_texture_canvas`) is
  fine and is already done for block masks. Only reading the **backbuffer** is undefined under
  WebGL — never do that. The emscripten stack is set to 8 MB (`-sSTACK_SIZE=8MB`); avoid large
  stack arrays anyway, prefer heap.
- Build to check your work: `cargo build --release`
  on Linux. Browser: `./build-browser.sh` then `./serve-browser.sh`.

### 4.3 Everything can change mid-match

- **Theme switching** (F2, or the `ThemeMode::All` playlists) changes a player's theme, and with it
  whether they render particles at all — so **the canvas can change size mid-match**.
- **Playlist stage switches** change which *game* a player is playing, so the sprite set changes.
- Both must be handled without a visible glitch. See §5.2.

### 4.4 Determinism and fairness

Nothing here touches game state, so there is no replay or high-score risk. Keep it that way: the
field must never consume from a game's RNG, and must never be able to stall the game loop.

---

## 5. Architecture

### 5.1 The field

Add a retained-pool concept alongside the existing emitter model rather than bending emitters to do
something they cannot:

```rust
/// a self-owned, long-lived set of particles that updates itself in place
pub trait ParticlePool {
    fn update(&mut self, delta: Duration, ctx: &SceneContext);
    fn particles(&self) -> &[Particle];
    fn clip(&self) -> Option<Rect>;
}
```

`Particles` gains `pools: Vec<Box<dyn ParticlePool>>` and `Particles::particles()` chains them in
after the group particles. The existing source/group machinery is untouched, so the menus keep
working unchanged.

The field is **a fixed-size pool of particles that never die**. This matters for more than
allocation: `ParticleGroup::update_life` currently does a `Vec::remove` per dead particle per
frame, which is O(n) each — fine for short-lived bursts, wrong for a permanent field. Particles
that leave the canvas wrap or are re-seeded inside it; they are never removed.

### 5.2 The canvas

The field does not work in "the window". It works in a **canvas**: the union of the clips of the
players whose theme is a particle scene.

```rust
pub struct SceneContext {
    /// union of the clips of players on a particle scene, in particle space
    pub canvas: RectF,
    pub players: Vec<PlayerRegion>,
    /// the union of the games being played, for the sprite set and palette
    pub games: Vec<GameId>,
}

pub struct PlayerRegion {
    pub player: u32,
    pub clip: RectF,      // this player's slice of the window
    pub board: RectF,     // their playfield, from ThemeContext::player_board_snip
    pub theme: usize,     // index into ParticleRender's theme sprites, see §5.5
    pub palette: Palette,
    pub in_canvas: bool,  // false for a player on a retro theme
}
```

Because clips are vertical slices and `MAX_PLAYERS == 2` (§2.4), `canvas` is always **one
contiguous rect**: whole window, left half, or right half. So the field draws **once**, with one
clip, replacing the per-player loop in `ThemeContext::draw_scene_particles` (`context.rs:609`).

**Routines are authored in canvas-normalised coordinates** — 0–1 across the canvas, mapped into
particle space on the way out. A routine written once then fits whether it owns the whole window or
one half, with no special cases. This is decision 3 handled structurally.

**On canvas change** (a theme switch flips a player between modern and retro, or the window
resizes): remap every particle's position proportionally from the old canvas to the new one, and
drop any in-flight feature routine back to ambient. Do not try to keep a half-finished sprite morph
across a resize.

### 5.3 Motion: fields and targets

Two additions to `particle.rs`:

1. Generalise `ParticleGroup::orbit: Option<Vec2D>` into a `Field` enum with today's gravity well
   as one variant, so nothing regresses:
   ```rust
   pub enum Field {
       Orbit(Vec2D),                       // today's `orbit`, unchanged maths
       Vortex { centre: Vec2D, strength: f64 },
       Flow { .. },                        // sum-of-sines / cheap curl noise
       Repel { centre: Vec2D, strength: f64 },
   }
   ```
2. Per-particle `target: Option<Vec2D>` with a spring/ease toward it. This is the primitive that
   every formation routine is built on.

### 5.4 The director

The field owns a director: a small state machine over

```
Ambient(routine, remaining) → Feature(routine, phase) → Ambient(next) → …
```

with a weighted playlist that never repeats a feature twice running, and a reaction queue for game
events. During a `Feature`, ~30% of the pool stays on the ambient routine (decision 8).

**Ambient routines** (the resting state, ~8–15 s each):

| routine | what it does |
|---|---|
| Orbit | today's gravity-well drift, ported in so the current look is never lost |
| Flow field | sum-of-sines / cheap curl noise advection; organic, no allocation |
| Galaxy / vortex | angular velocity ∝ 1/r around a wandering centre; arms form and shear |
| Constellation | slow drift with links drawn between near neighbours (§5.6) |

**Feature routines** (a few seconds, then back to ambient):

| routine | what it does |
|---|---|
| **Sprite edge morph** | the headline: gather into a silhouette outline, hold and drift, shatter (§5.5) |
| Lattice snap | snap to a rectilinear grid, breathe, then shear and collapse — structure with no masks at all |
| Oscilloscope | a sine/waveform ribbon across the canvas, amplitude driven by game intensity |
| Text morph | same machinery as the sprite morph, mask from `FontRender` output ("TETRIS", "LEVEL 8") |
| Weather | directional rain/snow/embers, speed tied to fall speed |
| Board haloes | rings orbiting each board, tightening and reddening as a stack climbs |
| Gravity wells | boards attract or repel the field; danger makes them pulse |

### 5.5 Sprite edge morphs

**Phases:** `gather` (~1.2 s ease into targets) → `hold` (~2–3 s, slowly drifting, tumbling and
breathing — the shape should move, not sit still) → `shatter` (outward shockwave) → ambient.

**Where the silhouettes come from.** `BlockMask` (`engine/src/render/block_mask.rs`) already turns
opaque pixels into a lattice of points — that is how masked line-clear particles work. Add:

```rust
impl BlockMask {
    /// lattice points where the mask is set and at least one 4-neighbour is not
    pub fn edges(&self, offset: Point, spacing: u32) -> Vec<Point>;
}
```

Sources, all of which are already `create_texture_target_blended` render targets and so can be
`read_pixels`'d exactly as cells are today:

- **piece previews** — `PreviewSpriteSheet`, built at `sprite_sheet.rs:379`; tetrominoes and pills.
  No masks exist for these yet; add them.
- **animated idle cells** — `AnimationSpriteSheet::block_mask` (`render/animation.rs:310`) already exists; viruses and pill
  halves.
- **mascots** — `MascotSprites` (`sprite_sheet.rs:231`), same mechanism.

Build edge lattices **lazily on first use and cache them**, rather than eagerly at theme build:
themes are all constructed at startup and an eager pass would add a `read_pixels` per sprite per
theme to load time, which matters most on wasm.

**Because we render only the silhouette, the source theme's art style is irrelevant** — an NES
tetromino edge and a modern one are the same outline. So a retro-themed player still contributes
their game's shapes without dragging their art into the modern field. This is what makes decision 5
(union of *games*) implementable.

**Finding the sprites for a theme.** `Theme::race_theme(index, pieces, scale)`
(`engine/src/render/mod.rs:211`) already answers "what sprites does this theme offer" — pieces,
animated cells, mascot — and is what the menu piece race consumes. Reuse the pattern, built from
the *currently playing* themes rather than all of them. `rustris::theme::race_themes` and
`dr_rustario::theme::race_themes` show how each game builds its piece list.

**Theme indices.** `ParticleSprite::{Piece,Cell,Mascot}` carry `theme: usize`, indexing
`ParticleRender::theme_sprites`, which is built from `themes.all` (`launcher/src/shell.rs:129`).
`ThemeContext` is built from the *same* slice (`match_screen.rs:74-80`), so the index spaces match.
But `ThemeContext::current` is private with no accessor — **add `pub fn current_theme_index(&self,
player: u32) -> usize`** so the field can name the sprites of the theme a player is actually on.

### 5.6 Constellation links

Draw each link as a **rotated textured quad**: a soft streak texture generated procedurally at
startup (say 64×8, a horizontal gradient falling off at both ends), drawn with `copy_ex` scaled to
the segment length and rotated to its angle. No new rasteriser, no new dependency, no
`render_geometry` (§4.2), it takes colour mod like everything else, and it looks better than a
hairline: soft, thick, glowing, consistent with the existing sprites.

`Canvas::draw_line` is the low-density fallback: one call, 1 physical pixel, aliased and nearly
invisible at 4k, but very cheap.

**Neighbour search:** a uniform grid bucketed at the link radius; each particle scans its 3×3
buckets. Cap at ~3 links per particle and a hard total per frame from the density setting. O(n) —
do not write a quadtree.

### 5.7 Colour

Both modern themes set `particle_color: Color::WHITE`
(`rustris/src/theme/modern/mod.rs:117`, `dr-rustario/src/theme/modern/mod.rs:200`), reaching the
scene as `SceneType::Particles { base_color, .. }` and the theme as
`Theme::particle_color() -> Option<Color>` (`render/mod.rs:185`, `None` for retro themes). Give the
modern themes real palettes — Dr. Rustario: red / blue / golden `#E1BE00`; Rustris: the seven
tetromino colours — and keep `base_color` as the fallback so the foreground effects are unaffected.

The colour layer is **independent of the motion layer**, so routines × palettes multiply variety.
Colour is dynamic (decision 9):

- **Wandering hue.** The palette is seeded from the playing games, then hue does a slow constrained
  random walk around it — always related to the theme, never static.
- **Per-board radiation.** Each board radiates its own palette into the field with distance
  weighting, so in 2 players the middle is a contested gradient that visibly shifts as one player
  pressures the other.
- **Event pushes.** A clear nudges the palette and it eases back; a Tetris flips it hard.
- **Danger drain.** Saturation bleeds toward red as a stack nears the top.
- **Temperature.** Optionally hue by speed, or by distance-to-target during a morph — hot flying
  in, cool once settled.

`ParticleColor` (`particles/color.rs`) is plain f64 RGB; add HSV conversion helpers there.

### 5.8 Reactions

Every player's events feed the field (decision 7), with no theme check.

| trigger | effect |
|---|---|
| `Clear` | horizontal shockwave from the cleared row's y; local particles shoved outward; colour flash. Scaled by `count` / `clear_class` |
| `Clear` with `is_combo`, or back-to-back | escalating: each successive clear adds a brighter ring and raises field energy for a few seconds |
| a **Tetris / 4-virus clear** | the big one: interrupt the routine, blast the field apart, ring across the canvas, re-form into that game's hero silhouette |
| `SpeedUp` | whole field accelerates, palette shifts, a wave sweeps outward |
| danger (stack near the top) | agitation, red drain, faster halo pulse. **Read per frame from the game, not from an event** — there is no danger `GameEvent` |
| `AttackSent` | the comet, §5.9 |
| `AttackReceived` | the victim's local field greyed and pressed downward |
| `Victory` / `StageComplete` | fanfare over the winner's half; the loser's half desaturates and falls |
| `Paused` | the field already stops updating (`match_screen.rs:568`); consider desaturating |

### 5.9 Attacks

One implementation, with endpoints resolved against the canvas:

| situation | effect |
|---|---|
| both boards in the canvas | full comet, attacker's board → victim's board, arrival burst and shove |
| attacker in canvas, victim not | the comet leaves the board and exits the canvas edge on the victim's side |
| victim in canvas, attacker not | the comet enters from the canvas edge and bursts on the board |
| neither | there is no field at all; nothing to do |

`player_board_snip` knows where every board is, including one that is never drawn over (§2.4), so
"the victim's side" is always answerable. Trail colour is the attacker's palette; thickness and
particle count scale with `Attack::strength`.

**Plumbing required.** `Session::send_attack` (`engine/src/session.rs:387`) chooses the victim
internally. Add a drained queue of routes — `(from: u32, to: u32, strength: u32)` — that
`MatchScreen::update` reads each frame alongside the events. Keep it a queue drained per frame, not
a callback, to match how events already flow.

---

## 6. Bugs to fix first

Both are in `engine/src/render/block_mask.rs` and both block the sprite morphs.

1. **`lattice` indexes with the wrong dimension.** Line 46 reads
   `self.mask[y as usize * self.height as usize + x as usize]`; the mask is row-major so the stride
   is **`width`**, not `height`. Invisible today because every cell sprite is square. Fatal for
   piece previews and mascots, which are not.
2. **`lattice` drops the last partial row and column.** Line 42 loops `0..self.height / spacing`,
   so a mask whose height is not a multiple of `spacing` loses its bottom edge — exactly the edge a
   silhouette outline needs.

Fix these in Stage 0 with a unit test on a deliberately non-square mask.

---

## 7. Staged plan

Each stage should build and run on its own. Do not start a stage before the previous one is
working.

### Stage 0 — foundations (no visible change)

- Fix both `BlockMask` bugs (§6), with a non-square unit test.
- Generalise `ParticleGroup::orbit` into the `Field` enum, today's gravity well as one variant.
- Add per-particle `target` + spring easing.
- Add the `ParticlePool` trait; chain pools into `Particles::particles()`.

**Acceptance:** the game looks pixel-identical to before. This stage is pure groundwork.

### Stage 1 — the field

- `SceneContext` / `PlayerRegion`; canvas computation and proportional remapping on change.
- The field type and its director (ambient ↔ feature state machine, weighted playlist, no immediate
  repeats).
- Port `prescribed_orbit` in as the first ambient routine; add flow-field drift and vortex.
- Move clipping out of `ThemeContext::draw_scene_particles` (`context.rs:609`) into the field so it
  draws **once** with the canvas clip.
- `VideoConfig::particle_density` (§5.7 of the config, see below) — it lands here because it sizes
  the pool.

**Config.** Add to `VideoConfig` (`engine/src/config.rs:133`), which is **config-file only** — there
is no video options menu to hang it on, and `integer_scale` right above it is the pattern to copy,
including its `#[cfg(feature = "portmaster")]` default override. Something like:

```yaml
video:
  particle_density: Auto   # Auto | Low | Medium | High | Ultra
```

`Auto` = High normally, Medium or Low under the `portmaster` feature. It scales pool size, link
budget, edge-lattice spacing and concurrent effect count.

**Acceptance:** the background belongs to the visible area — in a 2-player match with one modern
and one retro player, the field fills the modern half rather than being half-clipped. Switching
themes mid-match with F2 rescales it cleanly.

### Stage 2 — colour

- HSV helpers on `ParticleColor`; palettes on both modern themes.
- The wandering-hue driver, per-board radiation, the contested middle.
- Event pushes and the danger drain.

**Acceptance:** biggest visible win per line of code, and it lands before any expensive machinery.

### Stage 3 — constellations

- Procedural streak texture; the rotated-quad link renderer; `draw_line` fallback at low density.
- Bucketed neighbour search, link caps wired to density.

**Acceptance:** links appear and disappear smoothly, framerate holds at full density on desktop.

### Stage 4 — sprite edge morphs

- `BlockMask::edges`; lazily built and cached edge lattices for previews, idle cells and mascots.
- `ThemeContext::current_theme_index` accessor.
- The gather → hold-and-drift → shatter phase machine, target assignment, random placement and
  scale within the canvas.
- The playing-games sprite set (decision 5).

**Acceptance:** a Rustris match never shows a pill outline; a Dr. Rustario match never shows a
tetromino; a mixed vs. match shows both.

### Stage 5 — local reactions

- Clears scaled by `count`/`clear_class`, combo escalation, `SpeedUp`, per-frame danger.
- The Tetris / 4-virus hijack that blows the field apart and re-forms a hero silhouette.

### Stage 6 — attacks

- The attack-route queue out of `Session::send_attack`.
- The comet with canvas-relative endpoint resolution, arrival burst and victim shove.

**Acceptance:** works in all four situations in §5.9, including the half-visible cases.

### Stage 7 — long tail

Lattice snap, oscilloscope, weather tied to fall speed, board haloes, gravity wells, text morphs.
Each is small once Stages 0–4 exist; this is where variety gets cheap.

---

## 8. Testing

- **Unit-testable:** the `BlockMask` fixes, canvas union and remapping maths, the neighbour grid,
  HSV round-trips, endpoint resolution for all four attack situations. Prefer these — the
  particle modules already carry `#[cfg(test)]` blocks.
- **Manual matrix**, per stage: 1 player modern; 2 players both modern; 2 players modern + retro
  (both orderings, since the canvas is the left or right half); a vs. playlist that switches games
  mid-match; F2 theme cycling mid-match; pause and resume.
- **Performance:** check the release build on a handheld or with density forced to Low before
  declaring a stage done. Draw-call count is the number to watch, not particle count.

---

## 9. Deferred / open

- **Menus** keep the emitter model (decision, §1). If the field ever proves nicer there too, that
  is a separate piece of work.
- **More than 2 players** would break the contiguous-canvas assumption (§2.4). Not a concern now.
- **Sorting or quantising particle colour** to restore SDL batching is listed as a mitigation in
  §4.1 but is not scheduled. Do it if profiling says to, not before.
- **A video options menu** to expose density in-game — currently config-file only, matching every
  other video setting.
