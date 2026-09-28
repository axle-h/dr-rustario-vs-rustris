#!/usr/bin/env python3
"""Cuts puyo-rusto/src/theme/{menu,music}/*.ogg out of a set of converted rips.

    python3 puyo-rusto/art/music.py [source directory]   # needs ffmpeg with libvorbis

The source, not in the repository, is a directory of OGGs and a `loops.json` of loop points.
Every track is resampled to 44,100 Hz, the only rate `engine/src/audio/decode.rs` accepts, and
split on raw PCM at its loop point into `<slug>-intro.ogg` and `<slug>-repeat.ogg`, since the
mixer has no loop marker.
"""

import json
import os
import subprocess
import sys

RATE = 44100
CHANNELS = 2
BYTES_PER_FRAME = 2 * CHANNELS
QUALITY = "6"

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src", "theme"))
DEFAULT_SOURCE = os.path.expanduser("~/Downloads/pp/ogg")

# each `loops.json` track's `(directory, slug)`
TRACKS = {
    "Puyo Puyo Tetris - It's Main Menu!": ("menu", "menu"),
    "Puyo Puyo Tetris - Korobeiniki (2014)": ("music", "korobeiniki"),
    "Puyo Puyo Tetris - Dimension Stage ~ Decisive Battle (2014)": ("music", "decisive-battle"),
    "Puyo Puyo Tetris - Faceoff in Puyopuyo World! - Magical Confrontation! (2014)": (
        "music",
        "magical-confrontation",
    ),
    "Puyo Puyo Tetris - PuyoTetroMix (2014)": ("music", "tetro-mix"),
}


def decode(path):
    """the whole file as interleaved stereo i16 at the mixer's rate"""
    return subprocess.run(
        ["ffmpeg", "-loglevel", "error", "-i", path, "-ar", str(RATE), "-ac", str(CHANNELS),
         "-f", "s16le", "-"],
        check=True,
        stdout=subprocess.PIPE,
    ).stdout


def encode(pcm, target):
    subprocess.run(
        ["ffmpeg", "-y", "-loglevel", "error", "-f", "s16le", "-ar", str(RATE),
         "-ac", str(CHANNELS), "-i", "-", "-c:a", "libvorbis", "-q:a", QUALITY, target],
        check=True,
        input=pcm,
    )


def rescale(samples, source_rate):
    """a frame index in the source's own rate, in the mixer's"""
    return round(samples * RATE / source_rate)


def main():
    source = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_SOURCE
    with open(os.path.join(source, "loops.json")) as f:
        loops = json.load(f)

    named = {track["id"]: track for track in loops["tracks"]}
    missing = set(TRACKS) - set(named)
    if missing:
        raise SystemExit("loops.json has no " + ", ".join(sorted(missing)))

    for track_id, (folder, slug) in TRACKS.items():
        track = named[track_id]
        out = os.path.join(ROOT, folder)
        os.makedirs(out, exist_ok=True)

        pcm = decode(os.path.join(source, track["file"]))
        frames = len(pcm) // BYTES_PER_FRAME
        start = rescale(track["loop_start_samples"], track["sample_rate"])
        end = min(rescale(track["loop_end_samples"], track["sample_rate"]), frames)
        if not 0 < start < end:
            raise SystemExit(f"{track_id}: loop {start}..{end} of {frames} frames")

        for name, first, last in (("intro", 0, start), ("repeat", start, end)):
            target = os.path.join(out, f"{slug}-{name}.ogg")
            encode(pcm[first * BYTES_PER_FRAME:last * BYTES_PER_FRAME], target)
            print(f"{os.path.relpath(target, os.path.dirname(ROOT))}"
                  f" {(last - first) / RATE:.1f}s {os.path.getsize(target) // 1024} KiB")


if __name__ == "__main__":
    main()
