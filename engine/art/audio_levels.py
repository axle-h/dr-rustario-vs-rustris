#!/usr/bin/env python3
"""What every theme in the compendium actually plays at, and what it owes the house.

This is the whole app's audio meter.  It decodes every embedded `.ogg` in the repository -
music and effects, all three games, the engine's own menus - and prints each theme's level
against the **house baseline**, so a new theme, a re-cut set of effects or a fresh rip can be
held to the same loudness as everything already here.

**Loudness is RMS, not peak.**  Peak is the single loudest sample and says nothing about how
loud a sound is: two effects that both peak at -0.5 dBFS are ten decibels apart if one of them
is a click and the other a chord.  Every level here is therefore RMS, in dBFS, with the peak
carried alongside only as the headroom the file has left.  A set that is levelled on peaks -
which is what `puyo-rusto/art/retro_audio.py` did to Mean Bean Machine's effects - comes out
*sounding* hot while every meter says it is fine.

Three numbers describe a theme:

* `music` - the RMS of its music, energy-averaged over every track it may play.  This is the
  bed the whole theme sits on and it is what the baseline pins.
* `effects` - the RMS of its effect set, energy-averaged.
* `balance` - `effects - music`, in dB.  It is the number that says whether a theme's effects
  sit *in* its music or on top of it, and it is a property of the theme rather than of the app,
  so the house holds it to a band rather than to a value.

The trims the app applies on top of these (`with_gain`, `with_effects_at`) are read out of the
Rust and reported too, so what this prints is what a player hears rather than what is in the
file.

Usage:

    python3 engine/art/audio_levels.py            # the table, and what is out of band
    python3 engine/art/audio_levels.py --files    # every file, for chasing one sound

Needs `ffmpeg` on the path and `numpy`; it reads the repository and writes nothing.
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

# The house levels, measured pre-slider: what a theme's music and effects are levelled to.
#
# Set from `rustris/gb`, which is Alex's reference for a balance that reads right, and rounded.
# Every other theme of Rustris and Dr. Rustario was already within a couple of decibels of it;
# Puyo Rusto's own assets are mastered some eight decibels hotter than the rest of the app and
# are trimmed back to it in `puyo-rusto/src/theme/data.rs`.
MUSIC_TARGET = -22.0  # dBFS RMS
BALANCE_TARGET = -2.0  # dB, effects RMS relative to that theme's music RMS

# How far out of true a theme may sit before it is worth doing something about.  A decibel is
# not audible on its own here; three is.
MUSIC_TOLERANCE = 2.0
BALANCE_TOLERANCE = 4.0

# No file may peak above this: Vorbis hands back a sample or two above the encoder's own
# ceiling, and a file already at 0 dBFS clips as soon as it is mixed with anything.
PEAK_CEILING = -0.5  # dBFS

# A theme's music and its effects, by the folder they are embedded from.  A theme that shares
# another's folder (`puyo-rusto`'s particle theme plays `theme/sfx` over `theme/music`) names
# both.
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
    "menu/engine-modern": ("engine/src/menu/modern",) * 2,
    "menu/engine-retro": ("engine/src/menu/retro",) * 2,
    "menu/puyo": ("puyo-rusto/src/theme/menu",) * 2,
    "menu/rustris": ("rustris/src/theme/menu",) * 2,
}

# What the Rust does to each theme on top of the files: the gain it plays the whole theme at,
# and the trim it plays that theme's effects at on top of it, each named by the constant that
# holds it.  Read out of the sources rather than written down here, so this table cannot say one
# thing while the build does another.
TRIMS = [
    # theme, source, whole-theme gain, effects-only trim
    ("puyo/genesis", "puyo-rusto/src/theme/data.rs", "GENESIS_GAIN", "EFFECTS_TRIM"),
    ("puyo/snes", "puyo-rusto/src/theme/data.rs", "SNES_GAIN", "EFFECTS_TRIM"),
    ("puyo/particle", "puyo-rusto/src/theme/data.rs", "PARTICLE_GAIN", "EFFECTS_TRIM"),
    ("menu/puyo", "puyo-rusto/src/theme/data.rs", "MENU_GAIN", None),
    ("rustris/particle", "rustris/src/theme/data.rs", None, "PARTICLE_EFFECTS"),
]


# A file in a theme folder is music if its name says so; everything else there is an effect.
#
# A menu folder is the other way round - its two clicks are named and everything else it holds
# is a track - because a menu's tunes are called `title` and `menu`, which in `rustris/nes` are
# the names of two blips. The same word is an effect in one folder and a track in the next, so
# the folder decides and not the name.
MUSIC_NAMES = re.compile(
    r"(^|-)(music|music_crit|music-critical|music-menu)$|"
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
    """`(peak, rms, loudest 400 ms, seconds)` of one file, all linear

    The 400 ms window is what a momentary loudness meter reads and is the honest number for a
    sound with a long tail: a settle that rings for two seconds is not quiet because most of
    it is decay.  Whole-file RMS is reported as `rms` and is what the theme tables average,
    since an effect that rings *is* quieter over its length and a set of them mixes that way.
    """
    x = decode(path)
    if x.size == 0:
        return None
    peak = float(np.max(np.abs(x)))
    rms = float(np.sqrt(np.mean(x * x)))
    window = int(0.4 * 44100)
    if x.size > window:
        # a sliding mean of x^2 through one cumulative sum, rather than a convolution: the
        # music tracks are minutes long and the naive form takes hours over the whole repo
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
    """the gains the Rust applies, as `{theme: (music percent, effects percent)}`

    A theme not named in [`TRIMS`] plays its files as they are, at 100/100.
    """

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
