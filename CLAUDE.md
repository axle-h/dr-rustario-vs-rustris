@README.md

## Layout

| crate | what it is |
|---|---|
| `engine/` | everything that is not game rules: SDL app shell, menus, high scores, config, input, rendering (sprite sheets, themes, fonts, particles, animations), audio mixer, the match session, and the shared AI core (`ai/`: neural network, genetic algorithm, key pacing) |
| `dr-rustario/` | Dr. Rustario's rules (bottle, pills, viruses), theme data, its neural AI and the deterministic one (`game/ai/n64/`) that actually plays |
| `rustris/` | Rustris's rules (board, SRS, scoring, garbage), theme data and its neural AI |
| `puyo-rusto/` | Puyo Rusto's rules (board, pairs, chains, the nuisance queue), its particle theme and the placeholder ai that stands in until one is trained |
| `launcher/` | the `dr-rustario-vs-rustris` binary: picks games and options and runs a match |

Each game's AI supplies the game-specific half - board features, placement search and the agent -
on top of `engine::ai`, which owns the network shapes, the genome, the genetic algorithm and its
`Fitness` seam. Both games use the same architecture - as many neurons wide as it has features,
two hidden layers deep - sized to their own feature count (Rustris `FeatureNetwork`: 20 features,
1281 weights; Dr. Rustario `BottleFeatureNetwork`: 29 features, 2640), declared by the
`feature_network!` macro in `engine/src/ai/neural.rs` because the genome conversions belong to
neither game. Models are embedded as raw weight arrays in each game's
`ai/models.rs`; Dr. Rustario's are random until a `ga dr auto` run replaces them, which is why
its opponent and demo play `game/ai/n64/` instead - a port of `aiset.c` from the N64 game's
decompilation. That is a deterministic scorer: `field.rs` is the bottle in the ai's own 17x8
`(st, co)` grid, `score.rs` measures the runs the two halves land in, `chain.rs` asks whether
the placement leaves a chain, `routes.rs` measures how much room the pill has left, `params.rs`
holds the weights and `mod.rs` picks the skill row and situation column that select them.
`DrAiAgent` chooses between the two through `DrAiKind`; `ga dr play <seed> <level> <cap>
<every> <brain>` runs either headless, where a brain is `n64`, `n64:0`..`n64:5`, `neural` or
`linear`. The six skill rows are the one dial the original has, and `params.rs`'s `SKILL_ORDER`
ranks them worst to best (measured, not assumed - the rows are personalities, not a ladder):
that is what Dr. Rustario's four ai difficulties and its 2-player demo pick from, so a harder
setting is a better player as well as a faster one.

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

A theme may offer more than one track for a match. `AudioTheme` keeps them in a list with the
pick in a `Cell` (themes are built once and lived on as `&'static`, so every play site holds a
`&self`), and the one place a pick is made is `ThemeContext::sync_music` - which is reached
only when the theme the music belongs to has changed, so a match keeps the track it opened on
through a pause, a stage clear and a game over, and is dealt another when the theme moves under
it. What a match asks for is `MatchSettings::music`: `MusicChoice::Random`, which every game
with a single tune passes and which means the same thing there, or `Track(i)` indexing the
tracks that theme was given. Puyo Rusto is the game this is for - `GameMusic` in its
`game/rules.rs` names its four in the order `theme::GAME_MUSIC` embeds them, and its `music`
menu row pins one - and it is also the only game with menu music of its own besides Rustris,
through `Mode::menu_sounds`.

A game that can hold an attack rather than take it as it arrives reports what is still
waiting through `Game::pending_attacks`, as its own `CellId`s, and a theme that declares a
`PendingLayout` draws them as a strip from its own cell sprites - so a player can see what is
hanging over them. Dr. Rustario and Rustris both take a hit immediately and have none; Puyo
Rusto is the game this is for.

Puyo Rusto is being built in phases against [docs/puyo-puyo-plan.md](docs/puyo-puyo-plan.md),
which is the shared memory for that work - read it before touching the crate. It is on the
pre-menu and playable by humans on all four of its themes - `genesis` (Dr. Robotnik's Mean Bean
Machine), `snes` (Kirby's Avalanche), `3ds` (Puyo Puyo Chronicle) and `particle`, in that order,
which is oldest hardware first the way the other two games order theirs. The three retro themes
are cut by `puyo-rusto/art/rip_retro.py`, whose sources are not in the repository: it reads each
sheet's own link order off the art rather than being told it, and for `snes` - whose rips carry
no board, background or font - it renders the SNES's background layers on their own by poking
`$212C` in a savestate, which is how that theme's panel exists at all.

Where a retro panel's furniture goes was **measured against the emulated game**, not read off
the rip, and the two disagree. Mean Bean Machine's sheet keeps the screen as the two planes the
Genesis drew it on: the left one is the dungeon wall with the wells sunk into it, and every
stone border, every well *floor* and the boxes down the middle are on the right one, over a flat
key. Cutting the left plane alone gave the panel no floor, so the last row of beans had sixteen
pixels of open well under it and looked like it had stopped a row short; `genesis_screen`
composites the two, which is the fix and is also what hands the panel its `NEXT` boxes. Those
boxes are holes in the frame plane, so their rects are exact and `genesis/mod.rs` names them
rather than guessing: a pair per `NEXT` box (the game's own and its opponent's - a panel here
belongs to one player, so the queue runs through both) and the nuisance tray in the mugshot box,
which is the one piece of furniture this game has no use for. Kirby's Avalanche wanted the same
treatment and a pixel besides - a blob's eyes sit three rows into its cell and in a full field
they land on 99, 115 ... 195, so the field starts at 16 and not the 15 the layer render read.
Its two name plates are painted out - they label one queue per player and this panel has both
boxes to itself - so its queue goes in the gaps between the three wooden posts that frame them,
and its tray across the mouth of the arch at the foot of the centre column, which is the only
clear run as wide as six icons at half a cell. Its numbers are drawn in the red the game draws
that player's in: the ten font tiles use an outline index and a fill index and nothing else,
and which colours those take is the palette's, which is the player's - the left panel is red
and the right one white.

**The level is a HUD row on every Puyo theme**, which it never was: `MAX_LEVEL` was there to
size the digits and nothing drew it. All three source games print it and two of them call it a
*stage*, and it is the same number the menu offers as the `level` to start on - so it goes in
the recess under `STAGE` on `snes`, where Mean Bean Machine prints its own on `genesis`, at the
far end of the score strip on `3ds`, and as a labelled row on the particle theme. Placing it is
what `theme::data::hud` is for: a retro theme used to map one `MetricSnips` over `HUD_MAX`,
which is right for one row and draws two on top of each other. `genesis` gets the word `STAGE`
back with it, cut whole off the end of the big face's own row (`0123456789FINALSTAGE`) - that
column's five words are all text sprites and the frame plane carries none of them, so the panel
had boxes and no words, and a bare digit on stone says nothing. It is the only one of the five
that face can spell.

Chronicle is the odd one out again: it stands both fields on one painted scene rather than
giving each player a panel, so this theme does too. Its `background.png` is transparent - it is
written out at all only because the engine sizes a player's side of the window by it - and the
backdrop is `SceneType::Cover`, which is one picture scaled until it covers the window and
centred, drawn with linear filtering where every other scene here is a tile that has to keep its
hard edges. A 3DS top screen is 400x240 and has to hold up at 1080p and beyond, which it gets to
in two steps: three times in the rip, with Lanczos and a light unsharp, and the rest at draw
time with `Cover`'s own filter. Three rather than five because the art is painted - all soft
gradients, no fine detail, so the interpolation has nothing to lose - and 1200x720 is 685 KiB in
the binary and 3.4 MiB of texture against 1.5 MiB and 8.6 MiB at full size.

Its ai is phase 4 and its
place in the vs. playlists phase 5. Two things follow from that. Its four ai
difficulties are all backed by `PuyoAiKind::Placeholder`, which drops the pair in a column
picked at random - the menu offers what every other game offers because the launcher's tests
hold every game to that list, and phase 4 puts a real brain behind it. And a versus playlist
deals the games in `GameKind::PLAYLIST_ORDER`, which is a *third* list beside `ALL` (how games
are numbered) and `RUNNING_ORDER` (how they are billed): a game is on the pre-menu as soon as
it can be played and takes a playlist turn only once it has the themes and the ai to hold up
its end of one.

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