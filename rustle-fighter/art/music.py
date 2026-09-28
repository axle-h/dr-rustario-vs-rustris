#!/usr/bin/env python3
"""Render the arcade's QSound VGZ music through ``~/projects/vgmplay-libvgm`` into loop pairs.

    cmake -S ~/projects/vgmplay-libvgm/libvgm -B <b> -DCMAKE_INSTALL_PREFIX=<p> && cmake --build <b> && cmake --install <b>
    cmake -S ~/projects/vgmplay-libvgm -B <v> -DCMAKE_PREFIX_PATH=<p> -DUSE_SDL2=OFF && cmake --build <v>
    VGMPLAY=<v>/vgmplay python3 rustle-fighter/art/music.py

The mixer has no loop marker, so each track is split into intro and repeat at the loop point its
VGM header carries. Run ``engine/art/audio_levels.py`` afterwards.
"""

import gzip
import os
import struct
import subprocess
import tempfile
import wave

RIPS = os.path.expanduser("~/Downloads/Super Puzzle Fighter Art/music")
OUT = os.path.join(os.path.dirname(__file__), "..", "src", "theme", "arcade")
VGMPLAY = os.environ.get("VGMPLAY", os.path.expanduser("~/projects/vgmplay-libvgm/build/vgmplay"))

TARGET_RMS_DB = -22.0
PEAK_CEILING_DB = -0.5

# the stage themes a match's music is dealt from, and the menu tune
TRACKS = {
    "stage-morrigan": "04 Stage Morrigan",
    "stage-hsien-ko": "05 Stage Hsien-Ko",
    "stage-felicia": "07 Stage Felicia",
    "stage-ryu": "11 Stage Ryu",
    "stage-ken": "12 Stage Ken",
    "stage-chun-li": "13 Stage Chun-Li",
    "stage-sakura": "14 Stage Sakura",
    "menu": "02 Character Select",
}
STINGS = {"victory": "08 Victory Theme", "game-over": "19 Game Over"}


def vgm_header(path):
    """A VGM's total and loop lengths in samples at 44100, vgmplay's rate; 0 loop if it has none."""
    with gzip.open(path, "rb") as f:
        head = f.read(0x40)
    if head[:4] != b"Vgm ":
        raise SystemExit(f"{path} is not a VGM")
    total = struct.unpack_from("<I", head, 0x18)[0]
    loop_offset = struct.unpack_from("<I", head, 0x1C)[0]
    loop_samples = struct.unpack_from("<I", head, 0x20)[0]
    return total, (loop_samples if loop_offset else 0)


def render(name, into):
    """One track to wav; vgmplay's exit code is unreliable, so success is the wav appearing."""
    if not os.path.exists(VGMPLAY):
        raise SystemExit(f"no vgmplay at {VGMPLAY}; see this script's doc comment")
    subprocess.run(
        [VGMPLAY, "-w", "-W", into, os.path.join(RIPS, name + ".vgz")],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    wav = os.path.join(into, name + ".wav")
    if not os.path.exists(wav):
        raise SystemExit(f"vgmplay wrote nothing for {name}")
    return wav


def read_wav(path):
    with wave.open(path) as w:
        return w.getframerate(), w.getnchannels(), w.readframes(w.getnframes())


def encode(samples, rate, channels, gain, path):
    subprocess.run(
        [
            "ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
            "-f", "s16le", "-ar", str(rate), "-ac", str(channels), "-i", "pipe:0",
            "-filter:a", f"volume={gain:.6f}",
            "-c:a", "libvorbis", "-qscale:a", "5", path,
        ],
        input=samples,
        check=True,
    )


def measure(samples):
    import array
    import math

    a = array.array("h")
    a.frombytes(samples)
    if not len(a):
        return -120.0, -120.0
    peak = max(abs(x) for x in a) / 32768
    rms = math.sqrt(sum(float(x) * x for x in a) / len(a)) / 32768
    to_db = lambda v: 20 * math.log10(max(v, 1e-9))
    return to_db(peak), to_db(rms)


def gain_for(samples):
    peak_db, rms_db = measure(samples)
    gain_db = TARGET_RMS_DB - rms_db
    if peak_db + gain_db > PEAK_CEILING_DB:
        gain_db = PEAK_CEILING_DB - peak_db
    return 10 ** (gain_db / 20), peak_db, rms_db


def loop_split(wav, total, loop):
    rate, channels, data = read_wav(wav)
    width = 2 * channels
    if not loop or loop >= total:
        return None, data
    intro_frames = total - loop
    cut = intro_frames * width
    # the render runs past the end of one loop, so the repeat is exactly `loop` frames of it
    return data[:cut], data[cut : cut + loop * width]


def main():
    os.makedirs(OUT, exist_ok=True)
    scratch = tempfile.mkdtemp(prefix="rustle-fighter-music-")
    for slug, name in {**TRACKS, **STINGS}.items():
        total, loop = vgm_header(os.path.join(RIPS, name + ".vgz"))
        wav = render(name, scratch)
        rate, channels, _ = read_wav(wav)
        intro, repeat = loop_split(wav, total, loop if slug in TRACKS else 0)
        gain, peak_db, rms_db = gain_for(repeat)
        for part, samples in (("intro", intro), ("repeat", repeat)):
            if samples is None:
                continue
            suffix = "" if part == "repeat" and intro is None else f"-{part}"
            path = os.path.normpath(os.path.join(OUT, f"{slug}{suffix}.ogg"))
            encode(samples, rate, channels, gain, path)
            print(f"{os.path.basename(path):28s} {len(samples) / (2 * channels) / rate:6.1f}s")
        print(f"  {name}: peak {peak_db:5.1f} rms {rms_db:5.1f} -> {TARGET_RMS_DB:.0f} dBFS")


if __name__ == "__main__":
    main()
