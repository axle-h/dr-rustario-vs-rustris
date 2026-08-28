#!/usr/bin/env python3
"""Cuts puyo-rusto/src/theme/{sfx,menu}/*.ogg out of a rip of Puyo Puyo Tetris 2's sound effects.

    python3 puyo-rusto/art/sfx.py [source directory]   # needs ffmpeg with libvorbis

The source is the sound effect rip as it is distributed - a directory of `<name>.acb (What
It Is)` folders of WAVs - and it is **not in the repository**: 33 MiB of source is not worth
carrying, the way neither the sprite sheet `rip.py` cuts nor the tracks `music.py` splits
are. It defaults to sitting next to this script.

This used to be `audio.py`, a chiptune synthesiser: the theme's art was procedural then too.
Both are the rip now, and a game's sound effects are the one place a synthesiser has nothing
to say - the sound of a chain climbing is the sound Puyo is *known* for, and an approximation
of it is just wrong.

Two things happen to a sound on the way in:

* `engine/src/audio/decode.rs` rejects anything that is not 44,100 Hz outright and the rip
  is a mixture of 44.1 and 48 kHz, so every one of them is resampled. Mono and stereo both
  decode, so whichever the rip made a sound is what it stays.
* the silence is trimmed off both ends, with a short fade over the last of it. The rip pads
  a sound out to a whole number of somebody's frames - a quarter of a second of nothing after
  a chain step that has to land again before the next one does - and the head it trims is
  latency on a key press.

What is *not* done to them is normalising. The levels are the original's mix and they are
meant to be uneven: a puyo settling is a fifth the height of a nuisance drop because it is
supposed to go unnoticed, and levelling them would put a landing thud on top of the win
fanfare.
"""

import os
import re
import subprocess
import sys

import numpy as np

RATE = 44100
QUALITY = "6"

OUT = os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src", "theme")
)
DEFAULT_SOURCE = os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    "Nintendo Switch - Puyo Puyo Tetris 2 - Sound Effects - Sound Effects",
)

# how quiet, against a sound's own peak, still counts as the silence the rip padded it with
SILENCE = 0.005
# how much of the head to keep once the trim has found where the sound starts, and how long
# to fade the tail, in seconds
LEAD_IN = 0.004
FADE_OUT = 0.008

# Which of the rip's sounds each of the theme's is, by the name it has in the rip rather than
# by its number, since the numbering is the archive's and not the game's. The bank the sound
# comes from is the first word of the folder, so `main` is `main.acb (General Sounds)`.
#
# `puy` is Puyo's own half of the game and `tet` is Tetris's; where this game borrows a
# mechanic from Tetris it borrows that half's sound with it, which is why the hard drop is a
# `tet`. Everything a Puyo player hears every second - move, rotate, land, and the seven
# steps of a chain - is Puyo's own.
SOUNDS = {
    "sfx/move.ogg": ("main", "se_puy07_move"),
    "sfx/rotate.ogg": ("main", "se_puy08_rotate"),
    # the pair coming to rest. `down` is the only landing Puyo's half of the rip has, and it
    # is this rather than the hard drop: a puyo pair falls of its own accord and the sound is
    # the arrival, not the descent
    "sfx/lock.ogg": ("main", "se_puy09_down"),
    # what the puyos left standing over a group do once it has gone. Deliberately the
    # quietest thing in the theme - it fires once per chain step, under a `ren` that is four
    # times its height, and a chain that clatters is a chain you cannot hear climbing
    "sfx/settle.ogg": ("main", "se_tet01_fall"),
    # Tsu has no hard drop; this game does, so it takes Tetris's whoosh and then `lock`
    # lands on top of it the way it does after any other fall
    "sfx/hard-drop.ogg": ("main", "se_tet04_hdrop"),
    # The chain. `ren` is 連 - the rip carries seven of them, one per step, each a fixed
    # interval over the last, and they are the reason to have ripped anything at all.
    # `clear_class` in `puyo-rusto/src/render.rs` grades a step 0..3 - chain 1, 2, 3, then
    # everything from 4 up - so it is the first four that are wanted and `ren4` is what the
    # original plays on the step this game stops counting at
    "sfx/pop-1.ogg": ("main", "se_puy00_ren1"),
    "sfx/pop-2.ogg": ("main", "se_puy01_ren2"),
    "sfx/pop-3.ogg": ("main", "se_puy02_ren3"),
    "sfx/pop-4.ogg": ("main", "se_puy03_ren4"),
    # 送り, sending: nuisance leaving for the other board. The rip grades it in four sizes
    # and the engine has one key for it, so it is the smallest of them - it goes off on top
    # of the `ren` that earned it and must not bury it
    "sfx/attack.ogg": ("main", "se_puy14_oj_okuri1"),
    # おじゃま, the nuisance itself, landing on this board
    "sfx/garbage.ogg": ("main", "se_puy12_ojama1"),
    "sfx/speed-up.ogg": ("main", "se_puy24_levelup"),
    "sfx/pause.ogg": ("se_sys", "se_sys04_pause"),
    "sfx/victory.ogg": ("main", "se_puy19_win"),
    "sfx/game-over.ogg": ("main", "se_puy20_lose"),
    # The menus'. Puyo Rusto already plays its own menu music where the other games play
    # the engine's, and these are the two clicks that go with it; they land beside that music
    # in `theme/menu/` rather than in the particle theme, because a menu is not a theme and
    # phase 3's retro themes walk the same one.
    "menu/chime.ogg": ("se_sys", "se_sys05_cursor"),
    "menu/select.ogg": ("se_sys", "se_sys02_decide"),
}


def banks(source):
    """the rip's folders by the bank they hold: `main.acb (General Sounds)` -> `main`"""
    found = {}
    for entry in sorted(os.listdir(source)):
        if os.path.isdir(os.path.join(source, entry)):
            found[entry.split(".", 1)[0]] = os.path.join(source, entry)
    return found


def find(directory, sound):
    """the one file in `directory` the game calls `sound`

    The rip numbers its files in the order the archive held them and that number is not the
    game's, so it is stripped and the name behind it is what is matched. A couple of files
    carry two names separated by a comma - one slot the game fills twice over, and `se_tet01_fall`
    is one of them - so either name finds the file.
    """
    matches = [
        f
        for f in sorted(os.listdir(directory))
        if sound in re.sub(r"^\d+_", "", os.path.splitext(f)[0]).split(", ")
    ]
    if len(matches) != 1:
        raise SystemExit(f"{sound}: {len(matches)} matches in {directory}")
    return os.path.join(directory, matches[0])


def probe_channels(path):
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "a:0", "-show_entries",
         "stream=channels", "-of", "csv=p=0", path],
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    ).stdout.strip()
    return int(out)


def decode(path, channels):
    """the whole file as interleaved i16 at the mixer's rate"""
    pcm = subprocess.run(
        ["ffmpeg", "-loglevel", "error", "-i", path, "-ar", str(RATE), "-ac", str(channels),
         "-f", "s16le", "-"],
        check=True,
        stdout=subprocess.PIPE,
    ).stdout
    return np.frombuffer(pcm, "<i2").reshape(-1, channels).astype(np.float32) / 32768.0


def trim(frames):
    """cut the rip's padding off both ends and fade what is left of the tail"""
    level = np.abs(frames).max(axis=1)
    peak = level.max()
    if peak <= 0:
        return frames
    loud = np.nonzero(level > peak * SILENCE)[0]
    start = max(0, loud[0] - int(LEAD_IN * RATE))
    end = min(len(frames), loud[-1] + 1 + int(FADE_OUT * RATE))
    cut = frames[start:end].copy()
    fade = min(len(cut), int(FADE_OUT * RATE))
    cut[-fade:] *= np.linspace(1, 0, fade)[:, None]
    return cut


def encode(frames, target):
    pcm = (np.clip(frames, -1, 1) * 32767).astype("<i2").tobytes()
    subprocess.run(
        ["ffmpeg", "-y", "-loglevel", "error", "-f", "s16le", "-ar", str(RATE),
         "-ac", str(frames.shape[1]), "-i", "-", "-c:a", "libvorbis", "-q:a", QUALITY, target],
        check=True,
        input=pcm,
    )


def main():
    source = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_SOURCE
    if not os.path.isdir(source):
        raise SystemExit(f"no sound effect rip at {source}")
    found = banks(source)
    for name, (bank, sound) in SOUNDS.items():
        if bank not in found:
            raise SystemExit(f"{name}: the rip has no {bank} bank in {source}")
        path = find(found[bank], sound)
        channels = probe_channels(path)
        frames = trim(decode(path, channels))
        target = os.path.join(OUT, name)
        encode(frames, target)
        print(
            f"{name:22} {len(frames) / RATE:5.2f}s "
            f"{'stereo' if channels == 2 else 'mono  '} {os.path.basename(path)}"
        )


if __name__ == "__main__":
    main()
