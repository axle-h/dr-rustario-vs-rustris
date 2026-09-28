# Art and audio

The rips these are cut from are not in the repository. Re-run the script rather than hand-editing
its output; each script's docstring says how.

* `puyo-rusto/art/rip.py` (particle puyos; `check` writes an alignment board), `rip_retro.py`
  (Genesis and SNES sheets, panels, vignettes), `retro_audio.py`, `music.py`, `sfx.py`
* `puyo-rusto/art/mugshots.py`, `kirby.py`: characters; both print the Rust table to paste back.
  `sprites.py` describes what the ripped sheet must contain
* `rustle-fighter/art/rip.py` (gems, frame, panel; `check` fails if a colour's cells are off
  blue's; the nine power gem masks are synthesised), `music.py` (arcade VGZ through
  `~/projects/vgmplay-libvgm`), `sfx.py` (the PlayStation effects, and how each slot was chosen)
* `dr-rustario/art/build_doc.py`: the feature-reference page
* `engine/art/audio_levels.py`: the whole app's audio meter; run it after cutting any audio

Retro theme geometry is measured against the emulated games, not read off the rips; each theme
module says what from.

## Levels

Music at -22 dBFS RMS, effects within about 4 dB of it, nothing peaking over -0.5 dBFS.
`AudioTheme::with_gain` levels a whole theme and keeps its mix; `with_effects_at` corrects a
theme's effects against its own music. Match slots on RMS with peak only as a cap, never on peak.

## Emulators

RetroArch needs `--set video_vsync=false` (or it hangs) and
`--set savestate_file_compression=false`; `video_driver=sdl2` does not start, neither Puyo core
publishes a memory map, and sessions die after a while. Puyo Nexus rejects automated fetches:
ask Alex to fetch a page.
