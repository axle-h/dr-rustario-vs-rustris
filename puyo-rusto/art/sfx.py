#!/usr/bin/env python3
"""Cuts puyo-rusto/src/theme/{sfx,menu}/*.ogg out of a rip of Puyo Puyo Tetris 2's sound effects.

    python3 puyo-rusto/art/sfx.py [source directory]   # needs ffmpeg with libvorbis

The source is the rip as distributed, a directory of `<name>.acb (What It Is)` folders of WAVs,
and is not in the repository; it defaults to sitting next to this script.

Every sound is resampled to 44,100 Hz, the only rate `engine/src/audio/decode.rs` accepts, and
its padding is trimmed off both ends with a short fade. Nothing is normalised: the levels are
the original's mix and are meant to be uneven.
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

# how quiet, against a sound's own peak, still counts as the rip's padding
SILENCE = 0.005
# seconds of head kept before the sound, and of fade over the tail
LEAD_IN = 0.004
FADE_OUT = 0.008

# `(bank, name)` of each slot's sound, by name since the rip's numbering is the archive's. The
# bank is the first word of the folder, so `main` is `main.acb (General Sounds)`.
SOUNDS = {
    "sfx/move.ogg": ("main", "se_puy07_move"),
    "sfx/rotate.ogg": ("main", "se_puy08_rotate"),
    "sfx/lock.ogg": ("main", "se_puy09_down"),
    # the quietest sound in the theme, since it fires under every chain step
    "sfx/settle.ogg": ("main", "se_tet01_fall"),
    # Tsu has no hard drop, so this is Tetris's, with `lock` landing on top of it
    "sfx/hard-drop.ogg": ("main", "se_tet04_hdrop"),
    # only four chain steps, since `clear_class` in `puyo-rusto/src/render.rs` grades them 0..3
    "sfx/pop-1.ogg": ("main", "se_puy00_ren1"),
    "sfx/pop-2.ogg": ("main", "se_puy01_ren2"),
    "sfx/pop-3.ogg": ("main", "se_puy02_ren3"),
    "sfx/pop-4.ogg": ("main", "se_puy03_ren4"),
    # the smallest send size, so it does not bury the chain step that earned it
    "sfx/attack.ogg": ("main", "se_puy14_oj_okuri1"),
    "sfx/garbage.ogg": ("main", "se_puy12_ojama1"),
    "sfx/speed-up.ogg": ("main", "se_puy24_levelup"),
    "sfx/pause.ogg": ("se_sys", "se_sys04_pause"),
    "sfx/victory.ogg": ("main", "se_puy19_win"),
    "sfx/game-over.ogg": ("main", "se_puy20_lose"),
    # the menu's clicks, in `theme/menu/` since every theme walks the same menu
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
    """the one file in `directory` named `sound`, past its number prefix or after a comma"""
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
