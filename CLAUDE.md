@README.md

## Layout

| crate | what it is |
|---|---|
| `engine/` | everything that is not game rules: SDL app shell, menus, high scores, config, input, rendering (sprite sheets, themes, fonts, particles, animations), audio mixer, the match session, and the shared AI core (`ai/`: neural network, genetic algorithm, key pacing) |
| `dr-rustario/` | Dr. Rustario's rules (bottle, pills, viruses), theme data, its neural AI and the deterministic one (`game/ai/n64/`) that actually plays |
| `rustris/` | Rustris's rules (board, SRS, scoring, garbage), theme data and its neural AI |
| `puyo-rusto/` | Puyo Rusto's rules (board, pairs, chains, the nuisance queue), its themes and its beam search ai |
| `launcher/` | the `dr-rustario-vs-rustris` binary: picks games and options and runs a match |

Each game's AI supplies the game-specific half - board features, placement search and the agent -
on top of `engine::ai`, which owns the network shapes, the genome, the genetic algorithm and its
`Fitness` seam. Both games use the same architecture - as many neurons wide as it has features,
two hidden layers deep - sized to their own feature count (Rustris `FeatureNetwork`: 20 features,
1281 weights; Dr. Rustario `BottleFeatureNetwork`: 29 features, 2640), declared by the
`feature_network!` macro in `engine/src/ai/neural.rs` because the genome conversions belong to
neither game. Models are embedded as raw weight arrays in each game's
`ai/models.rs`; Dr. Rustario's is `survival_trained()`, which is trained but not yet a good
enough player to field as a difficulty, so its only outing is as player 1 of the 2-player demo.
Every difficulty, the 1-player demo and `DrAiKind::default()` play `game/ai/n64/` instead - a
port of `aiset.c` from the N64 game's decompilation. That is a deterministic scorer: `field.rs` is the
bottle in the ai's own 17x8 `(st, co)` grid, `score.rs` measures the runs the two halves land
in, `chain.rs` asks whether the placement leaves a chain, `routes.rs` measures how much room the
pill has left, `params.rs` holds the weights and `mod.rs` picks the skill row and situation
column that select them.
`DrAiAgent` chooses between the two through `DrAiKind`; `ga dr play <seed> <level> <cap>
<every> <brain>` runs either headless, where a brain is `n64`, `n64:0`..`n64:5`, `neural` or
`linear`. The six skill rows are the one dial the original has, and `params.rs`'s `SKILL_ORDER`
ranks them worst to best (measured, not assumed - the rows are personalities, not a ladder):
that is what Dr. Rustario's four ai difficulties and its 2-player demo pick from, so a harder
setting is a better player as well as a faster one. A test in `rules.rs` pins the network to
that demo, since nothing else would catch a difficulty quietly being handed one.

Training the neural model is three stages, run in order by `ga dr auto` (see the readme's
*Training Dr. Rustario*): `ai/imitation.rs` teaches a network by gradient descent to rank
placements the way the n64 ai ranks them, since a genetic algorithm cannot select between
members that all score zero and from random weights nearly all of them do; then `ai/genetic.rs`
runs a survival phase (viruses destroyed before being buried) and an efficiency phase (bottles
finished on a pill budget) off that seed. The survival phase's finish line lives in the fitness
rather than after it (`ai/run.rs`'s `run_finished`, carried out through the aggregate
`GameResult`'s game over flag, which for a run of several seeds means *out of the run* rather
than *buried*): a candidate plays four seeds and is finished when one of them came out of
`TOP_TRAINING_LEVEL` and every other one reached `PROVEN_LEVEL`. Both numbers are measured
rather than picked. Stopping at bottle 20, as it used to, capped the fitness where a taught
model already stood, so 518 generations of a 641 generation run scored the exact maximum and
selection at the top was a random walk; and asking every seed to clear everything is a lottery a
whole night lost 516 times out of 516. The two probe seeds of the four are a cost dial:
a candidate averaging under `ABANDON_BELOW` viruses over them is not played out, and is scored
over all four regardless, so being cut can only cost it. `ga dr trial` is a short bounded run
that trains nothing, for checking a change has left something to climb, and `ga dr probe` (`ai/probe.rs`) is
the diagnostic the features were chosen with: it records what the n64 ai made of every placement
it was offered and measures how much of that opinion the features can express, cloning it onto
them and sending the clone out to play. The scored agent has no hold - what it learns from has
none, and a model taught to use one plays two orders of magnitude worse.

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
`features` draws one png per routine, `sheet` outlines every sprite of a theme and labels it
with what the bank made of it, one png per theme, and `<seconds>` snapshots a running field.

A clear can say something over the cells it just took: `GameRender::clear_popup` returns a
short caption, `animate/popup.rs` holds one per clear on its own clock - so a chain leaves a
trail of them climbing the board - and `Theme::popup_font` draws it outlined, rising and
shrinking away, **in the colour the theme paints the cells that went**. It goes on last of all,
on the window after the foreground particles (`ThemeContext::draw_popups`), because a caption
drawn with the board is underneath the very burst it is about.
That colour is not declared anywhere: `BlockSpriteSheet::cell_color` reads it off the built
atlas at theme build time, averaging a cell's sprite with pixels weighted by saturation times
brightness so an outline and a puyo's white eyes do not wash it out. Tinting means
`set_color_mod`, which is a mutation behind a `&self` draw, so the fill font sits in a
`RefCell` - which makes `Theme<'a>` invariant in `'a`, so anything holding themes has to own
them or leak them rather than shorten their lifetime. It is not `clear_word`, which spells one word across the
whole window in background particles for the once-a-match moments; this is small, local and
fires on every clear. It costs a theme no art and no opt-in, because whether there are popups
at all is the game's decision: Puyo Rusto counts its chain out from the first step, and the
other two games return `None`.

A theme with art for what its game says can offer it instead, as a `PopupSpriteData` on
`ModernThemeOptions`: a sheet, and the *tokens* it can spell with the rect of each. A token is
whatever the sheet drew as one piece, so Puyo Rusto's `2 chain` is spelt from a `2` and a
`chain` rather than from six letters, and `spell` in `render/font.rs` takes the longest token
that fits at each step. A caption the sheet cannot spell falls back to the face, whole - so a
sheet need only carry what its game actually says, and nothing is ever half drawn. The art is
drawn as it was cut and **not** tinted: modulating a gold glyph towards a blue puyo only makes
it a dark gold. Puyo Rusto's is `theme/modern/popup.png`, cut from the same rip by
`rip.py`'s `popup`: the ten digits on a fixed pitch and the word `Chain!` under them, every
cell the same height because each glyph was cut against its row's baseline rather than its own
bounding box - the round digits hang below the line and the word sits well above it - so the
whole caption is drawn at one y.

A theme may offer more than one track for a match, and which one it plays is **dealt, never
chosen**. `AudioTheme` keeps them in a list with the deal in a `Cell` (themes are built once
and lived on as `&'static`, so every play site holds a `&self`), and the one place a deal is
made is `ThemeContext::sync_music` - which is reached only when the theme the music belongs to
has changed, so a match keeps the track it opened on through a pause, a stage clear and a game
over, and is dealt another when the theme moves under it. A game with one tune is dealt it
every time, which is what asking for a deal means there. Puyo Rusto is the game this is for:
`theme::GAME_MUSIC_TRACKS` is how many tracks each of its themes offers, the particle theme's
being the four `art/music.py` cuts and `genesis`'s the four Mean Bean Machine stage tunes.
There was a `music` menu row that pinned one, backed by a `GameMusic` enum and the engine's
`MusicChoice`; it went, with all of its plumbing, once a second theme had a soundtrack of its
own and a track's *name* could only ever be right on one theme. Puyo Rusto is also the only
game with menu music of its own besides Rustris, through `Mode::menu_sounds`.

A game that can hold an attack rather than take it as it arrives reports what is still
waiting through `Game::pending_attacks`, as its own `CellId`s, and a theme that declares a
`PendingLayout` draws them as a strip from its own cell sprites - in the theme's own
background pixels, so the strip goes wherever that theme's furniture has room for it and the
ball that fills it is aimed there rather than at the board. A player can see what is hanging
over them. Dr. Rustario and Rustris both take a hit immediately and have none; Puyo
Rusto is the game this is for. An icon whose attack is still crossing the window is **not
drawn at all**: an attack is routed the moment the chain that earned it ends and the receiving
game trays it there and then, so `animate/tray.rs` holds the new ones back until the ball
carrying them lands and then slides them into their slots. `match_screen` snapshots every
tray's depth *before* the update loop, since by the time the routes are drained the icons are
already in it.

**Five things move that a board does not.** `animate/bounce.rs` is the squash a cell plays
where it lands, keyed by *point* rather than by cell - what bounces is a place on the board,
so a bounce whose cell has since moved is simply never looked up. It is fed by
`GameEvent::Landed { cells }`, which is **not** `Settle`: settle fires once for a whole board
and only when something moved, so a pair landing flat produces none and a half on a ledge
comes to rest a lock earlier than its partner. An event is data on the wire rather than a
trait method, so it needs no `AnyGame` arm and no pinning test - the reason it is an event.
`animate/debris.rs` is a fire-and-forget emitter measured in **board cells, not pixels**, and
unbounded: a droplet leaves its cell and often the board, which is why it is drawn on the
window after the foreground particles rather than into the board texture. It is fired from
`PlayerAnimations::update` off the cells that cross the burst frame of the destroy strip, so
the droplets **outlive the clear**, and `DebrisArt::Cell` always resolves so a burst needs no
art. `animate/attack_ball.rs` is the one thing that belongs to no player - every offset a
player owns is applied inside that player's own panel - so it lives on `ThemeContext` and is
drawn unclipped, holding its flight in cells and player numbers and resolving both ends
through whichever theme each player is on *at draw time*. It flies to the receiver's **tray**
rather than to the middle of their board: `Theme::pending_origin` is the middle of the strip,
which is the one point on it that means something - `draw_pending` slides *every* arriving
icon out of exactly there, so the ball bursts where its own icons then spread from, and a
test pins the two together. The shards it throws are `debris`, measured in board cells rather
than background pixels, so `Theme::attack_arrival_cell` is that same point in the other unit
rather than a second opinion about where a hit lands. A theme with no tray keeps the middle of
its own top row, which is where every one of them took a ball before any had a tray.
`animate/tray.rs` is above.
`ImpactAnimation::State::Rumble` is a shake, and it is **opt-in**: Mean Bean Machine does not
shake at all (measured - the wall between the boards cross-correlates to zero displacement
over a whole capture, nuisance drop included), and what reads as a rumble there is every
refugee bean bouncing at once. Only `puyo-rusto`'s particle theme takes it.

`DestroyStyle::Pop` carries a `blink`, the tell before the strip: the group flashes where it
stands - **starting lit**, drawn exactly as it sits on the board and joined to its neighbours
- and only then starts its strip. It also carries `holding_first`, because a pop is rarely
evenly paced: Mean Bean Machine's bean pulls its face and holds it for a quarter of a second
and then goes in a hurry, so the strip's first frame gets a slot of its own and the rest share
what is left. Asking for less than an even share changes nothing, which is what every theme
that asks for none is doing. Both **add** to the game's own `POP_DELAY`, because `match_screen`
skips `game.update` outright while an animation blocks the tick - which is the whole of why
`POP_DELAY` is 90 ms and a genesis chain step is ~820.

**An attack ball is a sprite of its own, not a puyo**, wherever a theme has art for it
(`AttackBallData`): Mean Bean Machine draws a white core inside a coloured rim, wider than a
cell, in the **sending player's** palette - red for player one and blue for player two, the
same rule the score font follows - and in two sizes, the big one for an attack of a whole row
or more. The strip is player-major and big-first and it wraps, so one pair serves every player.
A theme that cut none falls back to the popped cell's own sprite with a white core over it,
which is what the particle theme does.

Such an attack is also drawn **arriving**: `GameRender::attack_fall` is a `NuisanceFall`,
`None` on a game whose garbage simply appears, and `animate/nuisance.rs` drops the cells in
from over the top of the board and holds the game's tick until the last one is down. The rules
have already put every cell where it comes to rest before the animation exists, so it is only
where each is *drawn* on the way there and a headless run never plays it at all. A column
falls as one piece keeping the spacing it lands in - so five rows read as a slab and nothing is
ever seen appearing in mid-board - with the *bottom* of every column starting one row above the
board, which means they all enter together and a column landing on an empty well arrives after
one landing on a full stack. The falling pass is clipped to `BoardGeometry::game_snip`, since a
theme's board texture may carry a stone lintel or a strip of sky above the top row and an
attack has to come in over the board's own edge rather than out of the furniture. It falls
under **gravity** with a per-column stagger, which is what the original does and what a
constant speed never looked like: the stagger is a golden-ratio hash of the column index rather
than an RNG, so neighbouring columns break the level row up rather than tilting it, the same
board falls the same way twice and there is no randomness on the render path. Every cell
reports itself as it lands, which is what makes each refugee bean bounce as it arrives.
`rules::NUISANCE_FALL` is the whole dial.

**A defaulted trait method is only heard if `AnyGame` names it.** Every match runs through the
launcher's wrapper, so a `Game` or `GameRender` method with a default that the wrapper does not
delegate silently answers the default and the game is never asked - and it costs nothing at
compile time. `pending_attacks` was one (the tray drew no icons at all), `attack_fall`
another (nuisance appeared rather than falling), `fall_progress` a third. All three are pinned
by tests in `launcher/src/games.rs`, since only a test can catch this. It is also why a new
seam is a `GameEvent` wherever it can be: an event is data on the wire and needs no arm.

**Nothing in `game/ai/` reads the tray.** `eval.rs`'s `nuisance` weight counts nuisance already
*on the board*; `pending_nuisance` reaches the search nowhere, so the ai never chains to answer
an attack and never hurries to cancel before a rock lands. It is why an ai duel looks so
one-sided - the first big chain decides it and the loser keeps calmly building while a full tray
empties onto it. The queue itself is right (measured: over a mirror duel both boards fire on the
same frame and both take the other's 71, and everything received is either down or still
waiting), and the lopsidedness is the ai and classic offset, not the routing.

Puyo Rusto is being built in phases against [docs/puyo-puyo-plan.md](docs/puyo-puyo-plan.md),
which is the shared memory for that work - read it before touching the crate. It is on the
pre-menu and playable by humans on all three of its themes - `genesis` (Dr. Robotnik's Mean
Bean Machine), `snes` (Kirby's Avalanche) and `particle`, in that order, which is oldest
hardware first the way the other two games order theirs. There was a fourth, `3ds` (Puyo Puyo
Chronicle); it was dropped on 2026-08-28 - it is modern art in a retro slot, its animations
would have wanted sprites nobody has cut, and its panel was the tallest of the four, which
capped the cell size of every theme the game has. The two retro themes are cut by `puyo-rusto/art/rip_retro.py`, whose sources are not in the repository: it reads each
sheet's own link order off the art rather than being told it, and for `snes` - whose rips carry
no board, background or font - it renders the SNES's background layers on their own by poking
`$212C` in a savestate, which is how that theme's panel exists at all.

**A retro theme's audio is `puyo-rusto/art/retro_audio.py`**, one subcommand per theme against
rips that are not in the repository either, and `genesis` is the one that has been done. Two
things in it are worth knowing. The music rip renders every track as intro + loop *twice* +
fade and carries the loop point only as a table of whole seconds, which is a third of a bar out
and would put a stumble in every loop - so the split is **measured**: cross correlation finds
the loop length to the sample, a normalised match profile finds where it starts, and the rip's
own two numbers are the assertion rather than the input. Landing late on the loop start is
harmless and landing early is not, since the render is periodic from the true start onwards, so
the search never reaches back before the run it is sure of. And Mean Bean Machine writes each
stage's lead-in as a track of its own, which is exactly the pair `StructuredMusic::new` takes.
The other thing is the **levels**: `sfx.py` does not normalise because the particle theme's rip
came with its mix intact, and this one did not - all sixty of its files peak at the same sample
value - so each effect is scaled to the peak of the particle theme's sound for the same slot,
read off `src/theme/sfx/` at run time. Mean Bean Machine has no hard drop, so `hard-drop.ogg`
is the nearest noise the game owns, the way the particle theme borrows Tetris's.

Where a retro panel's furniture goes was **measured against the emulated game**, not read off
the rip, and the two disagree. Mean Bean Machine's sheet keeps the screen as the two planes the
Genesis drew it on: the left one is the dungeon wall with the wells sunk into it, and every
stone border, every well *floor* and the boxes down the middle are on the right one, over a flat
key. Cutting the left plane alone gave the panel no floor, so the last row of beans had sixteen
pixels of open well under it and looked like it had stopped a row short; `genesis_screen`
composites the two, which is the fix and is also what hands the panel its `NEXT` boxes. Those
boxes are holes in the frame plane, so their rects are exact and `genesis/mod.rs` names them
rather than guessing: a pair per `NEXT` box (the game's own and its opponent's - a panel here
belongs to one player, so the queue runs through both) and the mugshot box, which holds the
character. Kirby's Avalanche wanted the same
treatment and a pixel besides - a blob's eyes sit three rows into its cell and in a full field
they land on 99, 115 ... 195, so the field starts at 16 and not the 15 the layer render read.
Its two name plates are painted out - they label one queue per player and this panel has both
boxes to itself - so its queue goes in the gaps between the three wooden posts that frame them.
Painting anything out of that column leaves a hole, and a flat dark band in a wooden column
reads as one, so the column's own woodwork closes them: the posts are run up to meet the plank
the `NEXT` sign is nailed to, and a whole course of plank is laid across the mouth of the arch
where the game stands Kirby and this one stands nothing. The tray stands on that course - the
only run in the column as wide as a tray needs - four boulders at three quarters of a cell,
which fills its forty eight pixels exactly.

**Its numbers are the game's own sixteen row face and not the eight row one**, which is two
tiles stacked - the top at VRAM tile 769 and the bottom sixteen further on, since the font is
laid out sixteen glyphs to a row. Tile 896 is a *different* face, the small one the game sets
its menus in, and cutting the score in it drew numbers a little over half the height of the
`SC` they sit beside. Neither the tiles nor the inks were guessed: the layer render carries two
of the game's own digits - the `0` of its score and the `1` in the `STAGE` recess - so both
were masked off it and matched against a decode of all 2048 tiles, which puts the `0` at 769
over 785 and the `1` at 770 over 786 to the pixel, and pairs each index with the colour it
came out as. The palette is the *player's*: the left panel draws in the red its `SC` is drawn
in, the right one in white.

**The level is a HUD row on every Puyo theme**, which it never was: `MAX_LEVEL` was there to
size the digits and nothing drew it. Both source games print it and call it a
*stage*, and it is the same number the menu offers as the `level` to start on - so it goes in
the recess under `STAGE` on `snes`, where Mean Bean Machine prints its own on `genesis`, and as
a labelled row on the particle theme. Placing it is
what `theme::data::hud` is for: a retro theme used to map one `MetricSnips` over `HUD_MAX`,
which is right for one row and draws two on top of each other. `genesis` gets the words
`SCORE` and `STAGE` back with it, and the two faces the game sets them in. That column's five
words are all text sprites and the frame plane carries none of them, so the panel had boxes and
no words at all; the fonts sheet has the lot, and reading it once serves all of it, since every
face on it is eight wide on a nine pixel pitch with thirty glyphs to a row - the digits and
then A to T - so a word is a lookup into that alphabet. The game uses two: a bold sixteen row
face for `NEXT`, `SCORE` and *both players' scores*, and a smaller plain white one for `STAGE`,
`1P`, `DR R` and the stage number. The bold one is **green** on the sheet because green is the
palette the labels take - a score is the same glyphs in the player's own, and matching the
sheet's green `0` against the game's own `00007536` both proves the face and pairs each ink
with the red it comes out as. The swap asserts nothing green survives it: that face has seven
shades, two of them a couple of dozen pixels across the whole row, and a shade left off the
table lights an edge on three digits and nowhere else.

**A Puyo board is thirteen rows and the thirteenth has nothing behind it.** The row above the
field is played in - a puyo resting up there is still in the game - so it is drawn; but both
panels are cut *level with the top of their own field* and each board's art stops there too,
the way a retro Rustris board's frame stops at its skyline and the spawning piece is drawn on
the backdrop above it. The row is `top_padding`, above the panel and the board alike, so it is
a cell of bare scene with the panel below it and nothing to either side. Mean Bean Machine's
course of stone over the well mouth and Kirby's hedge across the top of the screen both go with
that cut, and on both themes **a point in the padded background is a point on that console's
screen**. Two other arrangements were built and both were wrong: the row drawn *behind* that
furniture (a `covered_top` option, since removed), which hid a puyo that mattered as soon as a
stack reached the top; and the board's own art grown a course higher so the row sat inside it,
which read as a taller well rather than as room above the board.

**Their scenes are vignettes and were tiles.** Once the row was open, a puyo spawning above the
board had the same hand-scattered stone behind it that the panel below it is made of, and
neither plane read as being in front of the other. So each theme now stands on a wash of its
own backdrop's colour - `rip_retro.py`'s `vignette`, a 96x54 png lifted in the middle and
falling away to the corners, drawn through `SceneType::Cover`, which scales one picture over
the window with linear filtering and so is smooth at 4k for a couple of kilobytes. The tiles
are gone. A flat `Solid` and a dimmed tile were both tried; flat read as flat, and the dimmed
tile was still the panel's own stone.

**And the panel casts a shadow on it**, which is what lifts it off: `PanelShadow`, declared
once for both themes in `theme::data::panel_shadow`. It falls **down and to the right only** -
a light over the panel's top left shoulder - because it grows from its own top left corner
rather than spreading round the panel: a ring centred on the panel puts a band of shadow along
its top edge, which is where a spawning pair is the only thing standing on the scene. It is
cast from the panel and so does *not* move with the hard drop's ricochet, which jolts the
board inside a panel that stays where it is - as every retro theme here has always done. It is **not** painted into the panel art,
which is where it would naturally go - a margin round the art comes straight off the board,
since every theme of a game is drawn at the largest cell all of them can hold and in a two
player game the panels are sized by the width they have, so eight pixels of margin costs about
a twentieth of the board. Drawn at composite time (in `ThemeContext::draw_players`, before the
board, which is the one moment both the board and the panel are still to come) it costs the
layout nothing and may fall outside the player's own area, which is what a shadow should do.
Its `skip_top` is the theme's `top_padding`: that band is transparent and is where a pair
spawns, so it casts nothing - a shadow cast from the whole padded box puts a dark rectangle
behind the one row that is meant to have nothing behind it.

Bringing the panels down to the field's top cost Kirby its `NEXT` sign, which the game nails
*above* that edge - so `rip_retro.py` moves it down nine rows. What moves is the whole
assembly, letters *and* the plank they are nailed to (rows 7 to 31 of the screen): moving any
less of it leaves a plank sawn through halfway, which is what the first attempt did. The plank
lands exactly over the game's two name plates - which named one queue per player and had to go
whichever way - so it covers them outright, and the paint-out and the woodwork that used to
fill that hole have both gone from the script with them.

Panel height is what sizes the board, since every theme of a game is drawn at the largest cell
*all* of them can hold, so the tallest panel decides it for the rest. Both retro panels are
208 source rows and the spawning cell over them makes 224, which is their console's own screen
height - that took Puyo's cell from 66 pixels to 73, a board 949 pixels tall on a 1080p screen
where it was 858. What holds it at 73 rather than the 76 the two panels would allow is the
particle theme, which is built at whatever they can reach and is then the tallest of the three
itself.

**`genesis` pops the way Mean Bean Machine does.** The group first *flashes* where it stands,
about three times over 300 ms, drawn exactly as it sits on the board and joined to its
neighbours (`DestroyStyle::Pop`'s `blink`), which is what makes a chain readable - a step says
which group is going before it goes. Then the three frame strip in
`theme/genesis/animations.png`: the bean sees it coming (the surprised face, the last frame of
the strip at the top of the rip), curls into a ball, and the ball shrinks until there is
nothing of it. What it bursts into is **not on the strip at all** - it is thrown as
`animate/debris.rs` pieces on the strip's last frame, so the droplets leave the cell, the
board and the panel, and are still in the air while the next chain step blinks. The strip used
to draw four droplets *inside* the cell at two spreads, which is as far as a sprite can throw
anything. Every colour band on the beans sheet carries the balls - under the arrangements,
beside the halo and wings the same bean wears as an angel - and one droplet of its own, and
`rip_retro.py`'s `genesis_animations` lays them out one strip per row.

**A bean also squashes where it lands**, over the two frames each colour band carries on a row
of its own: a **flat** bean and a **tall** one, drawn nowhere else and used for nothing else,
which is squash and stretch. They are *not* the middle frames of the top strip, which look
like a squash and are the faces a bean pulls on its way out. Neither frame carries a neck,
which is the game's own art: a bean is briefly unlinked from its neighbours where it lands and
joins them as it settles. The refugee bean has a flat of its own - the same art its blink shuts
its eyes with - and no tall, so it settles straight back.

**Its tray is on the wall above the board**, where the game draws it - the band is
`TOP_PADDING` and a point in the padded background is a point on the Genesis screen, so
placing it there is only the numbers. It is anchored at the well's **right** edge and fills
leftwards, which is the one thing here the original does not do: that band is also the row a
pair spawns in and the pair is drawn *over* the tray, so left-anchored the front of the strip
sat behind every pair that came out of the spawn. Filling the other way puts the heaviest
icons - a tray is decomposed biggest first - furthest from it, and what is left over is the
three columns of the well the spawn column is not. An icon is drawn at **three quarters of a
cell** on a pitch of the same number, so four of them fill those 48 pixels exactly and
nothing overlaps; a test pins the whole strip clear of the spawn, since the numbers read as a
tidy row either way round. Three quarters and not the half the game draws, because the three
symbols are cut as whole cells like every other sprite and the art inside one runs 12 to 16
pixels across - at the pitch a 12 pixel blob came out at 6 and the black bean's white outline
and eyes mushed into a smudge. The same correction is the whole of the `snes` tray's, on its
plank. The mugshot box, where the tray used to be under a comment saying the Genesis never
drew one, holds the character.

Its rock of thirty is **painted rather than borrowed**: it is the solid white refugee, the
frame a bean flashes to on its way out, swapped to the red player one's score is printed in
by `rip_retro.py`'s `GENESIS_ROCK_INK`. White in a tray reads as a hole in the wall, the game
draws a red symbol here and no rip carries one - so it is authored art, the way the sweat in
`mugshots.py` is.

**A character stands in the mugshot box and answers how that player's match is going.**
`engine/animate/character.rs` is the state machine - four states (`Idle`, `Winning`, `Losing`,
`Defeat`) with a minimum dwell, a linger past the last clear and two danger thresholds with a
gap between them, so a stack sitting on one does not strobe - and `engine/render/character.rs`
is the art. It is game-neutral and nothing in it knows what game is being played; `genesis` is
the only theme with a cast so far. It is **built lazily**: thirteen faces and at most two are
ever on screen, so `CharacterSet` keeps the `include_bytes!` and turns one into a texture only
when it is *dealt*, out of a `RefCell<HashMap>` behind the `&self` draw - which is the only
lazily built theme art here, and one png per character rather than one sheet for the cast is
what makes it worth anything. Who a player gets is **dealt off the match seed**, the way
`PuyoSkin::deal` is, and the two players of a two player game never get the same face; the one
on the left is drawn **mirrored**, so each faces the other player's board. Nothing else in the
compendium flips a sprite.

Over the portrait go **layers**, and an overlay and a palette cycle turned out to be the same
thing: a small sprite, at an anchor in box coordinates, on a clock of its own, whose variants
merely happen to be recolours. So the cutter bakes each ramp step as a frame and palette
cycling stays out of the renderer. A layer has **a strip per state, like the portrait**, and a
state with zero frames is a layer that is not drawn there at all - which is Coconuts' coin on
idle and Sir Ffuzzy-Logik's eyes off every row but his losing one; and a cycle is cut as
**only the cycled pixels**, which is what lets it lie over a portrait that is animating
underneath. A layer may also *flicker between anchors* rather than travel, which is what
Humpty's arc and his wrung hands do, and it never flashes twice in the same slot.

Off it go **emitters**, which leave the box and are **not clipped to it** - on the Genesis a
spark crosses the stone of the centre column and goes on over the playfield. They are drawn on
the window by `ThemeContext::draw_character_particles`, a near-copy of `draw_debris` anchored
on the panel rather than on the board. Three triggers: `Every` (Frankly's sparks, a burst of
six on a clock), `OnFrame` (Humpty's bolts, as his antennae are drawn in - a *slice* of frames,
since his winning row runs the gesture twice) and `Danger` (the **sweat**). A source carries
its own directions, because Frankly's two antenna balls each omit the diagonal that would go
into his body. Speeds are in box pixels an axis per 60 Hz frame, which is the unit every
capture was measured in.

**The sweat is nobody's art.** Six characters throw identical drops and no sheet in any rip
carries one, so it is authored in the cutter and written into *every* character's png at the
same place. It is gated **twice over**: it runs on the **losing row only**, and there on a dial
above 0.55 of the board filled - higher than `DANGER_ENTER`, because the drops come on *later*
than the face does. Measured: Grounder holds the losing face for a whole capture with a low
stack and sweats nothing at all. A drop already in the air finishes its flight rather than
vanishing when the state changes.
The **danger flash** - the whole sprite plane going white near the top - is measured, written
up and deliberately not implemented.

`puyo-rusto/art/mugshots.py` cuts all thirteen and **prints the Rust table to paste back**, so
every strip's geometry is derived from the art rather than typed twice; `cargo run --example
character_shot` draws the whole cast through all four states to a contact sheet. Every number
in `theme/genesis/mugshots.rs` is measured off screen captures of the emulated game, written up
per character in the plan.

Its three symbols are the classic **1 / 6 / 30** (`NuisanceIcon`, decomposed biggest first),
and the first two are measured off the game: the little eyeless blob for a single and the
black bean with the white outline for a row of six. The rock of thirty is a **placeholder** -
the game draws a red symbol for it and the beans rip carries none, so it borrows the
white-outlined bean; `GENESIS_TRAY` in `rip_retro.py` is the one line to change. A strip's frames are edge to edge because the engine
addresses one by counting frame widths from the strip's own start; only the rows are spaced.
The refugee bean has no ball of its own, so it flashes white and shrinks away instead, and -
alone on the board - it *blinks* where it sits, through a three frame idle strip on a
`LinearWithPause` that holds its eyes-open frame for two seconds between blinks. Nuisance is
a `Cell::Garbage` and used to be drawn by the still-sprite path, so the blink also meant
routing garbage through `draw_stack_cell`; with no idle strip that is the same draw it always
was, which is every other theme in the repository.

Its place in the vs. playlists is phase 5, and a versus playlist deals the games in
`GameKind::PLAYLIST_ORDER`, which is a *third* list beside `ALL` (how games are numbered) and
`RUNNING_ORDER` (how they are billed): a game is on the pre-menu as soon as it can be played
and takes a playlist turn only once it has the themes and the ai to hold up its end of one.

Puyo Rusto's ai (phase 4, step 1) is the one of the three that had **no original to port**.
Puyo Puyo's own cpu opponents have never been decompiled into anything readable - Mean Bean
Machine's is in an unlabelled 68000 disassembly and Puyo VS's `Puyolib/AI.cpp` takes the
biggest chain in front of it and otherwise places at random - so `game/ai/` is built out of
the open literature, and mostly out of [ama](https://github.com/citrus610/ama) (MIT), whose
whole evaluation is fifteen weights in one file. `field.rs` is the board the search thinks on:
one byte a cell, no link masks, no allocation in a chain, and **always settled**, which is why
its chain loop pops before it settles where `board.rs`'s settles first. `quiet.rs` is the
piece that makes it a Puyo player rather than a tidy one - a *quiescence search* that asks
what chain the field is holding by dropping key puyos into every reachable column, in every
colour on the board, until a group of four forms, and running the chain out. `eval.rs` is the
fifteen weights, `placement.rs` has two move generators (the root one replays real `Pair`
moves so the wall kicks and the keys to press come free; the search one names two columns and
nothing else), `beam.rs` is the search - it walks the visible queue once and then forks down
several *invented* continuations, which is takapt's idea by way of ama's six fixed ones - and
`skill.rs` is the six rows. Which of those is the better player is **measured, not assumed**:
`ga puyo rank` plays every row over the same seeds and prints the `SKILL_ORDER` to paste back,
exactly as `SKILL_ORDER` works for Dr. Rustario, and the four difficulties pick out of it.
There is **no neural model and nothing provisioned for one** - adding it means adding a
`PuyoAiKind` variant beside `Scorer`, the way `DrAiKind` carries both.

**The search is a state machine, stepped once a frame, and it is always interruptible.**
`Search::new` plays every placement of the pair in play and stops; each `step` plays the next
pair onto eight more of the boards it is holding and hands the frame back. That is not a
performance trick, it is what makes the same search affordable on a handheld: the agent has
the pair's whole fall time - a second, sixty frames - so the hardest row costs 0.88 ms a frame
rather than 10.6 ms in one, with no board of the search given up, which is why the ladder is
**not** also scaled down under the `portmaster` feature. Two things follow and both live in
`agent.rs`: the pair goes on *falling* while it thinks, so the keys are re-derived at the end
from where the pair is (`root_moves` costs no evaluations) and the next placement down the
order is taken if the best is now out of reach - which is why `beam::ranking` returns an order
rather than a winner; and the pair may come to *rest* first on a board too full to fall
through, so the root layer is done in `Search::new` and there is always an answer.
`SearchConfig::steps` is what a measured think time is divided by to get the cost of a frame,
and `ga puyo rank` prints it. What made the evaluation itself cheap enough is in the plan: a
compile time neighbour table, cutting the beam's root layer like any other layer, not
re-walking the visible queue once per continuation, and popping before settling.

Puyo Rusto's particle theme is puyos, music and sound effects all cut out of rips - three
sources and three scripts, none of the sources carried here. `puyo-rusto/art/rip.py` writes `src/theme/modern/sprites.png` out of a sheet that
is **not in the repository** - it is 12 MiB and gitignored, so re-running the script means
finding the rip again - and the rip is sixteen skins on one 72 pixel grid, fifteen of them
whole (the sixteenth is a grab bag on no grid) and eleven of those cut, a band of six rows per
skin and `BANDS_ACROSS` bands side by side. The theme keys **all eleven**; which two a match
shows is `PuyoSkin::deal`'s answer at the start of it, so the two boards of a two player game
are never the same puyos and no two matches look alike. Four of the fifteen are dropped, for
two different reasons. One has no downward neck at all: its sixteen link variants are only
eight, paired so that a puyo joined below draws exactly like one joined to nothing, so nothing
can make it meet the puyo underneath. The other three cut and join perfectly well and were
dropped for how they *look* joined, which is the whole point of a set - four in a row have to
read as one mass. A television with antennae has necks so short that a run of them stays a row
of televisions with the antennae poking between; a stick figure joins into an elongated
humanoid; a small round face merges into a gappy mesh. `rip.py check` is the only way to see
any of that, since it takes a whole board of one skin to show it.

The bands run side by side rather than all in one column because the sheet is loaded **whole,
as a single texture**, and fourteen bands stacked stood it 6720 pixels tall - past the 4096
`MAX_ATLAS_WIDTH` a GLES driver will allocate in a dimension, which is the same ceiling the
built atlas is already kept under. That is not a slow theme on a handheld, it is a theme that
cannot be built there at all. Two across is 2560x2880 and the same pixels either way. The
layout lives in exactly two places that have to agree - `band()` in `rip.py` and `skin_block`
in `theme/modern/mod.rs` - and a test holds the sheet to both its shape and that ceiling.

It is a script rather than a crop because the rip numbers a puyo's links differently (down 1,
up 2, right 4, left 8, against `LinkMask`'s up 1, down 2, left 4, right 8) and because almost
none of its skins reach their own cell edges. Every skin was drawn on a pitch of its own and
laid out on the common 72 pixel grid, so necks stop anywhere from one to eight pixels short
and every join draws a seam - which is what `repair` is for. It finds a neck by *difference*,
the linked tile against the same puyo unlinked, and runs the outermost line of that difference
out to the cell edge: a neck is a prism, so its last line is exactly what is missing. Locating
it by difference rather than by the tile's own outermost pixels is the part that matters - one
skin wears antennae on the same line as its upward neck, and repeating those paints a band of
antenna up the cell - and so is looking for it only in the *margin* that side's neck can be
in, since a tile joined on three sides carries three necks and reading the outermost line of
all of them at once hands one direction another's line. Three things stand behind the
stretch, in order: `borrow` takes a neck the rip drew on one variant and left off another
(skin 2's brick joined up, down and right has no downward neck, and running its upward one
down the cell used to flood the whole tile); `graft` runs the puyo's own outermost line out
where no variant has the neck at all, which is every skin's first colour, since the sheet has
no room above its top row to draw an upward one in; and `close` fills the notch outside two
necks at once, the speck of nothing where four puyos meet. A side the puyo is *not* joined on
is put back to the unlinked puyo's, which is how the bleed from the next cell along comes off.
`python3 puyo-rusto/art/rip.py check` writes `art/alignment.png`
(gitignored): every skin drawn as a board that uses all sixteen masks, since a seam is a
hairline and the only way to see one is to put two puyos side by side. The music is a
rip too: `puyo-rusto/art/music.py` cuts `src/theme/menu/` and `src/theme/music/` out of a
directory of converted tracks and a `loops.json` of their loop points that is **also not in
the repository** (`~/Downloads/pp/ogg` by default). It resamples, because the mixer takes
44,100 Hz and nothing else, and it *splits* each track at its loop point, because the mixer
has no loop marker - `StructuredMusic::new(intro, repeating)` plays one file once and loops
the other forever - cutting the raw pcm rather than seeking with ffmpeg so the seam lands on
the sample the loop point names. The sound effects are a third rip: `puyo-rusto/art/sfx.py`
cuts the particle theme's fifteen out of a dump of Puyo Puyo Tetris 2's, and the menu's two
clicks with them, from a directory that is **not in the repository** either (33 MiB, and it
sits next to the script by default). Its `SOUNDS` table is the whole of it - one line per
sound naming the bank and the name the game gave it - and everything else in the script is
resampling to the mixer's 44,100 Hz and trimming the padding the rip leaves on both ends. It
does **not** normalise: the levels are the original's mix and are meant to be uneven, a puyo
settling a fifth the height of a nuisance landing because it is supposed to go unnoticed.
Re-run them rather than editing their output. `art/sprites.py` is the procedural art the rip
replaced - eighty puyos as signed distance fields - kept because it owes the rip nothing and
because it is the only description in the repository of what the sheet has to contain; it now
writes `art/procedural-sprites.png`, which is gitignored, and not the theme's sheet. It is faithful
Puyo Puyo Tsu, and every table in it (chain power, colour and group bonus, 70 target points,
the 30 nuisance all clear, the pair pool) is sourced from Puyo Nexus rather than guessed, with
the page named in each module's doc comment.
[docs/puyo-nexus-rules.md](docs/puyo-nexus-rules.md) is a local copy of every page of that wiki
that carries a rule - search it before implementing one, and search for the mechanic rather
than the page you expect it on, because the wiki files several rules outside `Category:Rules`. `board.rs` owns the chain loop, which reports
itself in the same grammar `bottle.rs` uses for a combo - one `Clear` per chain step with a
`Settle` between - so the particle field reacts to a chain without knowing what one is.
`nuisance.rs` is the part that makes the game what it is: an attack waits in a visible tray,
a chain cancels it before sending anything on, and whatever is left drops as the chain ends.
A Puyo `CellId` carries its colour, a four bit mask of which neighbours match and a
`PuyoSkin` - because puyos of a colour that touch are drawn joined, and because each player's
board is drawn from its own set of puyos. `board.rs` recomputes the masks after every lock,
pop and settle. The skin is dealt by the *game*, not chosen by the theme: `PuyoSkin::deal`
takes the match seed and hands every player a different one of the eleven, `Game::new` is
handed theirs, and every `CellId` and `PieceId` it reports carries it. Off the seed rather
than the thread's randomness so a playlist swapping one board onto Puyo mid-match hands that
player the puyos they already had; and `PuyoCell` itself has no skin on it, so nothing in the
rules can tell two players' puyos apart and `board_of` in `launcher/src/modes.rs` reads the
skin back off before comparing two players' boards. The theme therefore keys `PuyoSkin::COUNT`
sets of all eighty four cells and of all twenty five previews - nine hundred and twenty four
- and two things follow. `BlockSpriteSheet` wraps its atlas onto another row past
`MAX_ATLAS_WIDTH`, and its preview sheet onto shelves the same way, rather than laying
everything in one line that no driver would allocate. And the pre-built bank of alpha
variants had to go: it was sixty three whole copies of the atlas, one per fade step, so a
`&self` draw could pick one without a `&mut` - about 106 MiB for a *single* skin, and most of
a gigabyte for the fourteen there were then. The atlas now sits in a `RefCell` and a fade is
`set_alpha_mod` at draw time, which is the same trick the popup font's tint already used, and
puts the whole set at around 20 MiB. Whether it is the race or a match asking, they share the one sheet:
`race_themes` offers a pair per colour of every skin, so the title screen is the whole rip
going past before a match picks two out of it. The hidden thirteenth row is not merely invisible: a *ghost puyo* there
cannot pop and does not count towards the four a group needs (`Board::is_ghost`), so a chain
with a foot in it is held back until that puyo drops into view.

Every theme of every game is built at startup, in `Shell::new`, and stays built: the title
screen's sprite race draws from all of them at once and so does the particle field's silhouette
bank, so there is no theme that could be deferred without breaking those. What that costs is a
long wait on a slow device, so it happens behind `engine::app::loading`, a progress bar drawn in
flat rectangles - it runs before any theme, font or sheet exists to draw with. Each game offers
`all_themes_with_progress` beside `all_themes`, taking an `engine::render::ThemeProgress` that is
called as each theme lands; `all_themes` is the same thing with a callback that does nothing, so
the examples and tests are untouched. **`App::new` presents a frame the moment the window
exists**, before any of it: a Wayland toplevel is not mapped until the client commits a buffer,
so until something is presented the window does not exist for the compositor - which is what a
PortMaster session's `swaymsg [app_id=...] fullscreen enable` helper was failing to find while
the game loaded.

Building them all is also what a 1 GiB handheld cannot afford, and the whole of that bill is
Dr. Rustario's particle Dr.: 591 frames of 478 pixels square over four sheets, the largest
7170x7648 and so 209 MiB once it is a texture. `BlockSpriteSheet::new` loads a sheet whole and
*then* scales it down to the six and a half blocks he is drawn at, so it is held twice over
while that happens - measured with `SDL_RENDER_DRIVER=software`, which puts textures in the
rss the way a unified memory device does, that is an 800 MB startup peak against a 430 MB
resting one, and it is the peak the oom killer takes. It is the source art and not the window
that costs it: 762 MB at 320x240 against 870 MB at 1080p. Three of the four sheets are also
past the 4096 pixels a Mali G31 will allocate in a dimension. So `dr-rustario/build.rs` halves
them into `OUT_DIR` for the `portmaster` and `browser` builds - 800 MB down to 490 MB - and
`theme/modern/mod.rs` has two arms of `include_bytes!` to pick which it gets. A desktop is what
4k is drawn from and keeps the art as it was drawn. The halving is a plain 2x2 average rather
than a filter with any reach, because a sheet is a grid of frames whose size the theme works
out by division and two of the four draw right to a frame's edge: a 2x2 block reads only within
itself, so the frames stay where the theme looks for them and nothing bleeds from one into the
next, and the grid stays declared once, in the theme. The colours are weighted by their alpha
first - the sheets are palette pngs on a flat green matte, transparent by index, so every
transparent pixel still carries (71, 112, 76) and averaging it in would wash green into every
edge. `image` is an optional build-dependency, so a desktop build compiles neither it nor the
halving.

A game implements `engine::game::Game` (a headless board of `Cell`s with game-private
`CellId`s, producing engine `GameEvent`s) and `engine::render::GameRender`; its themes are
data handed to the engine's `retro_theme` and `modern_theme` builders (a theme says which it
is through `Theme::family`, which is what lets the vs. mode's retro and particle playlists pick
their themes out). An attack between players carries a `strength` in the sending game's own
units, a game-private detail, and a `ForeignPrices` table - one price per receiving `GameId`,
in that game's own units - since a Rustris row and a Dr. Rustario block are not the same thing
and neither are the clears that earn them. Only the sender knows what the clear took, so only
it can price the crossing (`foreign_attack` in each game's `game/mod.rs`, one arm per
receiver); the session hands the receiver `Attack::strength_for` its own game id and drops an
attack worth nothing over there. A pair nobody priced is worth nothing, so a forgotten
crossing drops rather than landing the wrong units. Every game's id lives in
`engine::game::ids` and not in the game's own crate, because pricing an attack means naming
the game it crosses to and the game crates are siblings that do not depend on each other. So
Dr. Rustario garbage keeps its colours between two Dr. Rustario players and becomes random
colours when it comes from Rustris.

A menu's items are built once, so a list whose options depend on another item - the mode
list, which loses the theme sprint as soon as a single theme is picked - is refreshed by
`MenuScreen::set_items` after every selection, from the launcher's `Shell`.
`cargo run --example menu_shot` draws both games' menus to pngs headless, walking the theme
and mode rows, which is how that is checked without a display.