#!/usr/bin/env python3
"""Cut the arcade theme's effects from the PlayStation disc's WAVs, resampled to the mixer's 44100.

    python3 rustle-fighter/art/sfx.py

Slots were chosen by measurement, not by ear. `00019`-`00021` climb in pitch at one length, so
they are chain pops 1-3, with `00023`, the longest of that family, as pop 4; the four shortest,
flattest files are the clicks; the rest are placed by length and are provisional. Levels match
RMS with the peak only as a cap.
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

# within four decibels of the music's -22
TARGET_RMS_DB = -20.0
PEAK_CEILING_DB = -0.5

MIXER_RATE = 44100

SLOTS = {
    "move": "SE_COMN.EMI_00001",
    "rotate": "SE_COMN.EMI_00003",
    "settle": "SE_COMN.EMI_00022",
    "lock": "SE_COMN.EMI_00000",
    "hard-drop": "SE_COMN.EMI_00002",
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
