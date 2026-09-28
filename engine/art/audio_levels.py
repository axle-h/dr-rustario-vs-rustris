#!/usr/bin/env python3
"""The whole app's audio meter: every theme's level against the house baseline.

    python3 engine/art/audio_levels.py            # the table, and what is out of band
    python3 engine/art/audio_levels.py --files    # every file, for chasing one sound

Decodes every embedded `.ogg`, applies the Rust's gains (`TRIMS`), and holds each theme's music
RMS to `MUSIC_TARGET`, its effects-minus-music to `BALANCE_TARGET`, and every peak under
`PEAK_CEILING`. Loudness is RMS, since peak is one sample. Needs `ffmpeg` and `numpy`;
`AUDIO_LEVELS_CACHE=<path>` caches the scan.
"""

import glob
import json
import os
import re
import subprocess
import sys

import numpy as np

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# ---------------------------------------------------------------- the baseline

# the house levels before the volume slider, taken from `rustris/gb`
MUSIC_TARGET = -22.0  # dBFS RMS
BALANCE_TARGET = -2.0  # dB, effects RMS relative to that theme's music RMS

# how far off either target a theme may sit before it is reported
MUSIC_TOLERANCE = 2.0
BALANCE_TOLERANCE = 4.0

# no file may peak above this, leaving room for Vorbis overshoot and the mixer's sum
PEAK_CEILING = -0.5  # dBFS

# each theme's `(music folder, effects folder)`, as embedded
THEMES = {
    "dr-rustario/nes": ("dr-rustario/src/theme/nes",) * 2,
    "dr-rustario/snes": ("dr-rustario/src/theme/snes",) * 2,
    "dr-rustario/n64": ("dr-rustario/src/theme/n64",) * 2,
    "dr-rustario/particle": ("dr-rustario/src/theme/modern",) * 2,
    "rustris/gb": ("rustris/src/theme/gb",) * 2,
    "rustris/nes": ("rustris/src/theme/nes",) * 2,
    "rustris/snes": ("rustris/src/theme/snes",) * 2,
    "rustris/particle": ("rustris/src/theme/modern",) * 2,
    "puyo/genesis": ("puyo-rusto/src/theme/genesis",) * 2,
    "puyo/snes": ("puyo-rusto/src/theme/snes",) * 2,
    "puyo/particle": ("puyo-rusto/src/theme/music", "puyo-rusto/src/theme/sfx"),
    "rustle-fighter/arcade": ("rustle-fighter/src/theme/arcade",) * 2,
    "menu/engine-modern": ("engine/src/menu/modern",) * 2,
    "menu/engine-retro": ("engine/src/menu/retro",) * 2,
    "menu/puyo": ("puyo-rusto/src/theme/menu",) * 2,
    "menu/rustris": ("rustris/src/theme/menu",) * 2,
}

# The Rust constants that hold each theme's whole-theme gain and effects-only trim, read out
# of the sources so this cannot disagree with the build.
TRIMS = [
    # theme, source, whole-theme gain, effects-only trim
    ("puyo/genesis", "puyo-rusto/src/theme/data.rs", "GENESIS_GAIN", "EFFECTS_TRIM"),
    ("puyo/snes", "puyo-rusto/src/theme/data.rs", "SNES_GAIN", "EFFECTS_TRIM"),
    ("puyo/particle", "puyo-rusto/src/theme/data.rs", "PARTICLE_GAIN", "EFFECTS_TRIM"),
    ("menu/puyo", "puyo-rusto/src/theme/data.rs", "MENU_GAIN", None),
    ("rustris/particle", "rustris/src/theme/data.rs", None, "PARTICLE_EFFECTS"),
    ("rustle-fighter/arcade", "rustle-fighter/src/theme/data.rs", "ARCADE_GAIN", None),
]


# In a theme folder a file is music if its name matches; in a menu folder everything but the
# two clicks is music, since `title` and `menu` are effect names in `rustris/nes`.
MUSIC_NAMES = re.compile(
    r"(^|-)music$|"
    r"-(intro|repeat)$|jingle$|^stages?-|^(korobeiniki|decisive|magical|tetro)"
)
MENU_EFFECTS = {"chime", "select"}


def is_music(folder, name):
    if "/menu" in folder:
        return name not in MENU_EFFECTS
    return bool(MUSIC_NAMES.search(name))


def decode(path):
    """the file as mono float samples at 44.1 kHz"""
    out = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", path, "-f", "f32le", "-ac", "1", "-ar", "44100", "-"],
        capture_output=True,
    )
    return np.frombuffer(out.stdout, dtype=np.float32).astype(np.float64)


def db(x):
    return 20 * np.log10(max(float(x), 1e-12))


def measure(path):
    """`(peak, rms, loudest 400 ms, seconds)` of one file, linear; themes average `rms`"""
    x = decode(path)
    if x.size == 0:
        return None
    peak = float(np.max(np.abs(x)))
    rms = float(np.sqrt(np.mean(x * x)))
    window = int(0.4 * 44100)
    if x.size > window:
        # a sliding mean of x^2 through one cumulative sum
        acc = np.concatenate([[0.0], np.cumsum(x * x)])
        best = float(np.max((acc[window:] - acc[:-window]) / window))
        momentary = float(np.sqrt(best))
    else:
        momentary = rms
    return peak, rms, momentary, x.size / 44100.0


def scan(cache_path=None):
    """every ogg in the repository, measured, keyed by folder then by file stem"""
    if cache_path and os.path.exists(cache_path):
        return json.load(open(cache_path))
    rows = {}
    for path in sorted(glob.glob(os.path.join(REPO, "**", "*.ogg"), recursive=True)):
        rel = os.path.relpath(path, REPO)
        if rel.startswith("target/") or "/art/" in rel or "/audio/test/" in rel:
            continue
        stats = measure(path)
        if stats is None:
            continue
        rows.setdefault(os.path.dirname(rel), {})[os.path.basename(rel)[:-4]] = stats
    if cache_path:
        json.dump(rows, open(cache_path, "w"))
    return rows


def energy_mean(values):
    return float(np.sqrt(np.mean([v * v for v in values]))) if values else 0.0


def rust_trims():
    """the gains the Rust applies, as `{theme: (music percent, effects percent)}`"""

    def constant(source, name):
        text = open(os.path.join(REPO, source)).read()
        found = re.search(rf"const {name}: i32 = (\d+)", text)
        if not found:
            raise SystemExit(f"{source}: no {name} to read")
        return int(found.group(1))

    trims = {}
    for theme, source, gain, effects in TRIMS:
        music = constant(source, gain) if gain else 100
        trims[theme] = (music, music * (constant(source, effects) if effects else 100) // 100)
    return trims


def theme_levels(rows, trims):
    out = {}
    for theme, (music_dir, sfx_dir) in THEMES.items():
        music_files = rows.get(music_dir, {})
        sfx_files = rows.get(sfx_dir, {})
        music = [v[1] for k, v in music_files.items() if is_music(music_dir, k)]
        sfx = [v[1] for k, v in sfx_files.items() if not is_music(sfx_dir, k)]
        music_trim, sfx_trim = trims.get(theme, (100, 100))
        peaks = [v[0] * music_trim / 100 for k, v in music_files.items() if is_music(music_dir, k)]
        peaks += [v[0] * sfx_trim / 100 for k, v in sfx_files.items() if not is_music(sfx_dir, k)]
        out[theme] = {
            "music": db(energy_mean(music)) + db(music_trim / 100) if music else None,
            "effects": db(energy_mean(sfx)) + db(sfx_trim / 100) if sfx else None,
            "peak": db(max(peaks)) if peaks else None,
            "trim": (music_trim, sfx_trim),
            "n": len(music) + len(sfx),
        }
    return out


def report(rows, show_files):
    trims = rust_trims()
    levels = theme_levels(rows, trims)
    print(
        f"house: music {MUSIC_TARGET:+.1f} dBFS RMS (+-{MUSIC_TOLERANCE:.0f}), "
        f"balance {BALANCE_TARGET:+.1f} dB (+-{BALANCE_TOLERANCE:.0f}), "
        f"no file over {PEAK_CEILING:+.1f} dBFS\n"
    )
    print(f"{'theme':22} {'n':>3} {'trim':>9} {'music':>7} {'effects':>8} {'balance':>8} {'peak':>7}")
    problems = []
    for theme, v in levels.items():
        trim = f"{v['trim'][0]}/{v['trim'][1]}%"
        music = f"{v['music']:7.1f}" if v["music"] is not None else "      -"
        effects = f"{v['effects']:8.1f}" if v["effects"] is not None else "       -"
        if v["music"] is not None and v["effects"] is not None:
            balance = v["effects"] - v["music"]
            balance_s = f"{balance:8.1f}"
            if abs(balance - BALANCE_TARGET) > BALANCE_TOLERANCE:
                problems.append(
                    f"{theme}: effects sit {balance:+.1f} dB against its music, "
                    f"where the house is {BALANCE_TARGET:+.1f}"
                )
        else:
            balance_s = "       -"
        if v["music"] is not None and abs(v["music"] - MUSIC_TARGET) > MUSIC_TOLERANCE:
            problems.append(
                f"{theme}: music at {v['music']:+.1f} dBFS, {v['music'] - MUSIC_TARGET:+.1f} "
                f"off the house baseline"
            )
        if v["peak"] is not None and v["peak"] > PEAK_CEILING:
            problems.append(f"{theme}: a file peaks at {v['peak']:+.1f} dBFS")
        print(f"{theme:22} {v['n']:3d} {trim:>9} {music} {effects} {balance_s} {v['peak']:7.1f}")

    print()
    if problems:
        print("out of band:")
        for line in problems:
            print(f"  * {line}")
    else:
        print("every theme is in band.")

    if show_files:
        for folder in sorted(rows):
            print(f"\n== {folder}")
            for name, (peak, rms, momentary, seconds) in sorted(rows[folder].items()):
                kind = "music" if is_music(folder, name) else "sfx  "
                print(
                    f"   {kind} {name:28} peak {db(peak):6.1f}  rms {db(rms):6.1f}  "
                    f"400ms {db(momentary):6.1f}  {seconds:6.2f}s"
                )


if __name__ == "__main__":
    cache = os.environ.get("AUDIO_LEVELS_CACHE")
    report(scan(cache), "--files" in sys.argv)
