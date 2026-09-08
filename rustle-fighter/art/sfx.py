#!/usr/bin/env python3
"""Cut the arcade theme's sound effects out of the PlayStation disc's own extractions.

    python3 rustle-fighter/art/sfx.py

The effects are the one part of this theme that is **not** arcade: Alex's drop carries thirteen
zips of plain WAVs pulled from the PlayStation disc's `data/*.emi` containers, and no arcade
effect rip at all. That is the right way round anyway - the effects are the half tied to
gameplay events, and the PlayStation port is the ruleset this game reproduces.

**How each slot was chosen, since nobody here can listen to them.** The set was measured rather
than auditioned: length, spectral centroid, spectral flatness, and the dominant pitch of a
window in the middle of each file. Two families fall straight out of those numbers:

* `00019`, `00020`, `00021` are three sounds of the same length (0.50-0.52s) and the same
  shape whose pitch **climbs** - 2068, 3108, 3275 Hz. That is a chain-step set, and it is what
  the four clear classes are built on, with `00023` (1.71s, and much the longest of the
  family) as the big break at the top.
* `00001`, `00003`, `00000`, `00022` are the four shortest files in the set (0.20-0.34s) and
  the only ones flat enough to be clicks rather than tones, which is what a move, a rotate, a
  lock and a settle are.

The rest are placed by length and are **provisional**: the fanfares are the three longest
files and go to victory, game over and the speed step. Anyone who can play them should check
this table first - it is the least certain thing in the theme.

**Resampled to 44100 Hz**, which the engine's mixer requires: the disc's own effects are
11025 and 22050 Hz and are rejected outright at their native rate.

Levelled to the house baseline: effects within about four decibels of the music's -22 dBFS
RMS, nothing over -0.5 dBFS peak. **Matched on RMS with the peak only as a cap**, never slot by
slot on peaks - two effects that both peak at -0.5 are ten decibels apart if one is a click and
the other a chord, which is the mistake `puyo-rusto/art/retro_audio.py` records making.
"""

import math
import os
import subprocess
import tempfile
import wave
import zipfile

RIPS = os.path.expanduser("~/Downloads/Super Puzzle Fighter Art/sfx")
ZIP = "PlayStation - Super Puzzle Fighter II Turbo - Miscellaneous - Sound Effects.zip"
OUT = os.path.join(os.path.dirname(__file__), "..", "src", "theme", "arcade")

# effects sit here, which is within four decibels of the music's -22
TARGET_RMS_DB = -20.0
PEAK_CEILING_DB = -0.5

# the rate the engine's mixer runs at; anything else is refused when a theme is built
MIXER_RATE = 44100

SLOTS = {
    # the four short clicks, in ascending length
    "move": "SE_COMN.EMI_00001",       # 0.20s, the shortest and the flattest
    "rotate": "SE_COMN.EMI_00003",     # 0.25s
    "settle": "SE_COMN.EMI_00022",     # 0.33s
    "lock": "SE_COMN.EMI_00000",       # 0.34s
    "hard-drop": "SE_COMN.EMI_00002",  # 0.47s
    # the chain, which climbs: 2068, 3108, 3275 Hz, and then the long one
    "pop-1": "SE_COMN.EMI_00019",
    "pop-2": "SE_COMN.EMI_00020",
    "pop-3": "SE_COMN.EMI_00021",
    "pop-4": "SE_COMN.EMI_00023",
    "attack": "SE_COMN.EMI_00007",
    "garbage": "SE_COMN.EMI_00004",
    "speed-up": "SE_COMN.EMI_00024",
    "pause": "SE_COMN.EMI_00027",
}


def measure(samples):
    import array

    a = array.array("h")
    a.frombytes(samples)
    if not len(a):
        return -120.0, -120.0
    peak = max(abs(x) for x in a) / 32768
    rms = math.sqrt(sum(float(x) * x for x in a) / len(a)) / 32768
    to_db = lambda v: 20 * math.log10(max(v, 1e-9))
    return to_db(peak), to_db(rms)


def main():
    os.makedirs(OUT, exist_ok=True)
    scratch = tempfile.mkdtemp(prefix="rustle-fighter-sfx-")
    with zipfile.ZipFile(os.path.join(RIPS, ZIP)) as z:
        z.extractall(scratch)
    folder = os.path.join(scratch, "Sound Effects")

    for slot, name in SLOTS.items():
        with wave.open(os.path.join(folder, name + ".wav")) as w:
            rate, channels = w.getframerate(), w.getnchannels()
            data = w.readframes(w.getnframes())
        peak_db, rms_db = measure(data)
        gain_db = TARGET_RMS_DB - rms_db
        if peak_db + gain_db > PEAK_CEILING_DB:
            gain_db = PEAK_CEILING_DB - peak_db
        path = os.path.normpath(os.path.join(OUT, slot + ".ogg"))
        subprocess.run(
            [
                "ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
                "-f", "s16le", "-ar", str(rate), "-ac", str(channels), "-i", "pipe:0",
                "-filter:a", f"volume={10 ** (gain_db / 20):.6f}",
                "-ar", str(MIXER_RATE),
                "-c:a", "libvorbis", "-qscale:a", "5", path,
            ],
            input=data,
            check=True,
        )
        print(
            f"{slot:10s} {name}  {rate:5d} -> {MIXER_RATE} Hz  "
            f"peak {peak_db:5.1f} rms {rms_db:5.1f}  {gain_db:+5.1f} dB"
        )


if __name__ == "__main__":
    main()
