#!/usr/bin/env python3
"""Cuts a retro theme's music and sound effects out of that game's own rips.

    python3 puyo-rusto/art/retro_audio.py genesis|snes [--only music|sfx]   # ffmpeg, libvorbis

`genesis` is Mean Bean Machine, `snes` Kirby's Avalanche. Sources are the unversioned
`art/retro/<theme>-music` and `<theme>-sfx`; output is `src/theme/<theme>/`, OGG at exactly
44,100 Hz. A looping track is split into `-intro` and `-repeat` at a loop found in the audio,
with each table's own numbers asserted against it so a differently laid out rip fails. Music
gets one gain per set; effects are levelled against the particle theme's sound for the same
slot. Run `engine/art/audio_levels.py` after any re-cut.
"""

import argparse
import os
import subprocess

import numpy as np

RATE = 44100
QUALITY = "6"

ART = os.path.dirname(os.path.abspath(__file__))
THEMES = os.path.normpath(os.path.join(ART, "..", "src", "theme"))
SOURCES = os.path.join(ART, "retro")

# padding threshold against a sound's own peak, head kept and tail fade, as `sfx.py` has them
SILENCE = 0.005
LEAD_IN = 0.004
FADE_OUT = 0.008

# how far under the loud channel the other one is before [`centre`] calls it silence
SILENT_CHANNEL = 0.01

# Mean Bean Machine's stage tunes, `(rip number, one pass, loop)` in the rip's own whole seconds:
# the loop seeds `loop_length` and `pass - loop` is the intro the answer is checked against. The
# rip's `07 Stages 5-8` is `Stages 1-4` at 7/6 speed and is left out. The `Stage Intro` tracks are
# the pre-stage screen's music, not lead-ins, and are left out too.
GENESIS_MUSIC = {
    "stages-1-4": ("05", 31, 31),
    "stages-9-12": ("09", 31, 31),
    "stage-13": ("11", 47, 47),
}

# the two it plays once; game over is one pass of the continue screen's music
GENESIS_JINGLES = {
    "victory": ("21", 3, None),
    "game-over": ("15", 13, 13),
}

# which of the rip's wavs each effect slot is; four pops, as `clear_class` grades 0..3
GENESIS_SFX = {
    # matched by ear: the rip's own `move` is a noise burst, not the pair's slide
    "move": "puyo_sine",
    "rotate": "short_noise",
    "lock": "puyo_blob",
    "settle": "puyo_blob_2",
    # no `hard-drop`: nothing in this rip fills it, see [`whoosh`]
    "pop-1": "chain_1",
    "pop-2": "chain_2",
    "pop-3": "chain_3",
    "pop-4": "chain_4",
    "attack": "bad_puyo_1",
    "garbage": "bad_puyos",
    "speed-up": "level_start",
    "pause": "select",
}


# Kirby's Avalanche's stage tunes, `(rip number, loop seconds)`. An SPC names no loop point, so
# the seconds are only asserted against what `loop_period` finds.
SNES_MUSIC = {
    "stage-1": ("104", 41.665),
    "stage-2": ("106", 48.306),
    "stage-3": ("108", 23.594),
}

# the cues it plays once, each `(rip number, ...)` trimmed and joined end to end
SNES_JINGLES = {
    "victory": ("115a", "115b"),
    "game-over": ("113",),
}

# Each effect slot's clip of the sound test capture, by number. The numbering is the split's
# (`snes-sfx/manifest.tsv`), so re-splitting the capture renumbers this table.
SNES_SFX = {
    "move": "24",
    "rotate": "04",
    "lock": "06",
    "hard-drop": "15",
    "pop-1": "07",
    "pop-2": "08",
    "pop-3": "09",
    "pop-4": "10",
    "attack": "43",
    "garbage": "20",  # hard right; 21 is the same sound hard left, never use both
    "speed-up": "26",
    "pause": "03",
}

# Mean Bean Machine has no hard drop sound, so [`whoosh`] builds one from this span of noise
HARD_DROP_NOISE = ("sfx_20", 0.07, 0.45)
# `(seconds, from Hz, to Hz, late)`: the cutoff slides along `t ** late`, dropping at the end
HARD_DROP_SWEEP = (0.30, 8000, 400, 1.8)
# `(where the swell peaks, how sharply it falls away after)`, as fractions of the length
HARD_DROP_ENVELOPE = (0.45, 1.4)


# lossless extensions, preferred over a transcode of the same track
LOSSLESS = (".flac", ".wav")


def decode(path, channels=2):
    """the whole file as float frames at the mixer's rate"""
    pcm = subprocess.run(
        ["ffmpeg", "-loglevel", "error", "-i", path, "-ar", str(RATE), "-ac", str(channels),
         "-f", "s16le", "-"],
        check=True,
        stdout=subprocess.PIPE,
    ).stdout
    return np.frombuffer(pcm, "<i2").reshape(-1, channels).astype(np.float32) / 32768.0


def encode(frames, target):
    pcm = (np.clip(frames, -1, 1) * 32767).astype("<i2").tobytes()
    subprocess.run(
        ["ffmpeg", "-y", "-loglevel", "error", "-f", "s16le", "-ar", str(RATE),
         "-ac", str(frames.shape[1]), "-i", "-", "-c:a", "libvorbis", "-q:a", QUALITY, target],
        check=True,
        input=pcm,
    )


def peak(frames):
    return float(np.abs(frames).max())


def loop_length(mono, hint):
    """The lag, to the sample, at which the render repeats, searched within 1.5s of `hint`."""
    coarse = int(round(hint * RATE))
    span = int(1.5 * RATE)
    start = RATE
    width = min(int(15 * RATE), len(mono) - coarse - span - start)
    if width <= 0:
        raise SystemExit(f"too short to carry two passes of a {hint}s loop")
    ref = mono[start:start + width]
    window = mono[start + coarse - span:start + coarse + span + width]
    size = 1 << int(np.ceil(np.log2(len(window) + width)))
    correlation = np.fft.irfft(
        np.fft.rfft(window, size) * np.conj(np.fft.rfft(ref, size)), size
    )[:2 * span + 1]
    return coarse - span + int(np.argmax(correlation))


def loop_period(mono, ref=(1.0, 8.0), shortest=8.0):
    """That same lag with no hint; it cannot tell a loop from twice a loop."""
    start, width = int(ref[0] * RATE), int(ref[1] * RATE)
    reference = mono[start:start + width].astype(np.float64)
    rest = mono[start:].astype(np.float64)
    span = len(rest) - width
    first = int(shortest * RATE)
    if span <= first:
        raise SystemExit(f"too short to carry two passes of a {shortest}s loop")
    size = 1 << int(np.ceil(np.log2(len(rest) + width)))
    correlation = np.fft.irfft(
        np.fft.rfft(rest, size) * np.conj(np.fft.rfft(reference, size)), size
    )[:span]
    energy = np.concatenate([[0.0], np.cumsum(rest * rest)])
    score = correlation / (
        np.sqrt((energy[width:width + span] - energy[:span]) * (reference * reference).sum())
        + 1e-12
    )
    return first + int(np.argmax(score[first:]))


def loop_matches(mono, length, block=0.25):
    """per quarter second of the render, how well it matches itself a loop later"""
    size = int(block * RATE)
    count = (len(mono) - length) // size
    first = mono[:count * size].reshape(count, size)
    second = mono[length:length + count * size].reshape(count, size)
    scale = np.sqrt((first * first).sum(axis=1) * (second * second).sum(axis=1)) + 1e-12
    return (first * second).sum(axis=1) / scale


def loop_start(mono, length, block=0.25, gap=8, threshold=0.9):
    """Where that repeat begins: everything before it is the intro.

    The longest run of [`loop_matches`] over `threshold`, closing dips of up to `gap` blocks.
    It never reaches back before that run: starting late still loops, starting early does not.
    """
    size = int(block * RATE)
    matches = loop_matches(mono, length, block) > threshold
    count = len(matches)

    closed, index = matches.copy(), 0
    while index < count:
        if not closed[index]:
            end = index
            while end < count and not closed[end]:
                end += 1
            if 0 < index and end < count and end - index <= gap:
                closed[index:end] = True
            index = end
        else:
            index += 1

    best, index = (0, 0), 0
    while index < count:
        if closed[index]:
            end = index
            while end < count and closed[end]:
                end += 1
            if end - index > best[0]:
                best = (end - index, index)
            index = end
        else:
            index += 1
    return best[1] * size


def split(directory, track):
    """`(intro, repeat)` of one rip; a track with no loop is all intro and `None`"""
    prefix, whole, loop = track
    frames = decode(source(directory, prefix))
    if loop is None:
        return frames, None
    mono = frames.mean(axis=1)
    length = loop_length(mono, loop)
    start = loop_start(mono, length)
    if abs(start - (whole - loop) * RATE) > 1.5 * RATE:
        raise SystemExit(
            f"{prefix}: the loop starts at {start / RATE:.2f}s where the rip's {whole}s pass "
            f"of a {loop}s loop puts it at {whole - loop}s - is this rip laid out differently?"
        )
    return frames[:start], frames[start:start + length]


def measured_split(directory, prefix, loop):
    """`(intro, repeat)` of a rip with no loop table, the period found and `loop` asserted.

    The threshold is relative to the render's own median match, which is under 0.9 for some.
    """
    frames = decode(source(directory, prefix))
    mono = frames.mean(axis=1)
    length = loop_period(mono)
    if abs(length - loop * RATE) > 0.1 * RATE:
        raise SystemExit(
            f"{prefix}: the render repeats every {length / RATE:.3f}s where this table says "
            f"{loop}s - is this a different render, or half a loop?"
        )
    steady = float(np.median(loop_matches(mono, length)))
    start = loop_start(mono, length, threshold=0.6 * steady)
    return frames[:start], frames[start:start + length]


def source(directory, prefix):
    """the one file in `directory` whose name begins with the rip's number, lossless first"""
    matches = [f for f in sorted(os.listdir(directory)) if f.startswith(prefix + " ")]
    lossless = [f for f in matches if os.path.splitext(f)[1].lower() in LOSSLESS]
    matches = lossless or matches
    if len(matches) != 1:
        raise SystemExit(f"{prefix}: {len(matches)} matches in {directory}")
    return os.path.join(directory, matches[0])


def clip(directory, number):
    """the one clip of the sound test capture named `clip-<number>_<second cut at>`"""
    matches = [f for f in sorted(os.listdir(directory)) if f.startswith(f"clip-{number}_")]
    if len(matches) != 1:
        raise SystemExit(f"clip-{number}: {len(matches)} matches in {directory}")
    return os.path.join(directory, matches[0])


def whoosh(directory):
    """The hard drop: the game's own noise under a per-frame Butterworth lowpass sweep."""
    source, head, tail = HARD_DROP_NOISE
    seconds, top, bottom, late = HARD_DROP_SWEEP
    noise = trim(decode(os.path.join(directory, source + ".wav"), 1))
    noise = noise[int(head * RATE):int(tail * RATE)]

    # a window of padding either side, or the overlap-add edges spike and set the level
    length, window, hop, order = int(seconds * RATE), 512, 128, 3
    span = length + 2 * window
    tiled = np.resize(noise, span + window)
    shaped, weight = np.zeros(span + window), np.zeros(span + window)
    taper = np.hanning(window)
    freqs = np.fft.rfftfreq(window, 1 / RATE)
    for at in range(0, span, hop):
        place = min(max((at - window) / length, 0.0), 1.0)
        corner = top * (bottom / top) ** (place ** late)
        spectrum = np.fft.rfft(tiled[at:at + window] * taper)
        shaped[at:at + window] += np.fft.irfft(
            spectrum / np.sqrt(1 + (freqs / corner) ** (2 * order)), window
        )
        weight[at:at + window] += taper * taper
    shaped = (shaped / np.maximum(weight, 1e-6))[window:window + length]

    # swell in, then fall away: opening at full height would read as the landing, which is `lock`
    peak_at, decay = HARD_DROP_ENVELOPE
    time = np.linspace(0, 1, length)
    swell = np.where(
        time < peak_at, (time / peak_at) ** 0.6, ((1 - time) / (1 - peak_at)) ** decay
    )
    return (shaped * swell).astype(np.float32)[:, None]


def trim(frames):
    """cut the padding off both ends and fade the tail"""
    level = np.abs(frames).max(axis=1)
    loudest = level.max()
    if loudest <= 0:
        return frames
    loud = np.nonzero(level > loudest * SILENCE)[0]
    first = max(0, loud[0] - int(LEAD_IN * RATE))
    last = min(len(frames), loud[-1] + 1 + int(FADE_OUT * RATE))
    cut = frames[first:last].copy()
    fade = min(len(cut), int(FADE_OUT * RATE))
    cut[-fade:] *= np.linspace(1, 0, fade)[:, None]
    return cut


def centre(frames):
    """copy the loud channel over a silent one, since the mixer pans effects itself"""
    levels = np.abs(frames).max(axis=0)
    if levels.min() > SILENT_CHANNEL * levels.max():
        return frames
    return np.repeat(frames[:, [int(np.argmax(levels))]], frames.shape[1], axis=1)


def rms(frames):
    return float(np.sqrt((frames.astype(np.float64) ** 2).mean()))


def reference_music_level():
    """the RMS of the particle theme's music loops in `src/theme/music/`"""
    folder = os.path.join(THEMES, "music")
    loops = [name for name in sorted(os.listdir(folder)) if name.endswith("-repeat.ogg")]
    return float(np.mean([rms(decode(os.path.join(folder, name))) for name in loops]))


# the loudest a cut's music may peak, leaving room for Vorbis overshoot and the mixer's sum
MUSIC_CEILING = 0.95


def music_gain(loops, everything):
    """One gain for a set's music: up to [`reference_music_level`] or [`MUSIC_CEILING`]."""
    wanted = reference_music_level() / float(np.mean([rms(frames) for frames in loops]))
    room = MUSIC_CEILING / max(peak(frames) for frames in everything)
    return (wanted, "the level") if wanted <= room else (room, "the headroom")


def write_music(out, cuts, loops):
    gain, bound = music_gain(loops, cuts.values())
    print(f"the whole set at {20 * np.log10(gain):+.1f} dB, held by {bound}, peaking at "
          f"{max(peak(frames) for frames in cuts.values()) * gain:.2f}")
    for name, frames in cuts.items():
        target = os.path.join(out, name + ".ogg")
        encode(frames * gain, target)
        print(f"{os.path.relpath(target, THEMES):32} {len(frames) / RATE:6.2f}s "
              f"{os.path.getsize(target) // 1024:5} KiB")


def reference_levels():
    """`(peak, rms)` of the particle theme's own effect for each slot"""
    folder = os.path.join(THEMES, "sfx")
    levels = {}
    for name in sorted(os.listdir(folder)):
        if name.endswith(".ogg"):
            frames = decode(os.path.join(folder, name))
            levels[os.path.splitext(name)[0]] = (peak(frames), rms(frames))
    return levels


def reference(levels, slug):
    if slug not in levels:
        raise SystemExit(f"{slug}: the particle theme has no sound to level it against")
    return levels[slug]


def slot_gain(levels, slug, cut):
    """The gain that puts one cut where the particle theme's sound for the same slot sits.

    It matches RMS, with the reference's peak only as a cap: peak is one sample and says nothing
    about loudness, so levelling a dense set on peaks makes it sound hot.
    """
    reference_peak, reference_rms = reference(levels, slug)
    wanted = reference_rms / rms(cut)
    room = reference_peak / peak(cut)
    return (wanted, "its level") if wanted <= room else (room, "its peak")


def set_gain(levels, cuts):
    """One gain for a whole capture's effects, keeping the game's own mix within the set."""
    wanted = np.mean([reference(levels, slug)[0] for slug in cuts]) / np.mean(
        [peak(frames) for frames in cuts.values()]
    )
    room = max(levels[slug][0] for slug in cuts) / max(peak(frames) for frames in cuts.values())
    return (wanted, "the level") if wanted <= room else (room, "the loudest sound it may match")


def write_effect(out, slug, frames, gain, provenance):
    target = os.path.join(out, slug + ".ogg")
    encode(frames * gain, target)
    print(f"{os.path.relpath(target, THEMES):34} {len(frames) / RATE:6.2f}s "
          f"{os.path.getsize(target) // 1024:5} KiB  {provenance} at {20 * np.log10(gain):+.1f} dB")


def genesis(only):
    music = os.path.join(SOURCES, "genesis-music")
    effects = os.path.join(SOURCES, "genesis-sfx")
    out = os.path.join(THEMES, "genesis")
    for folder in (music, effects):
        if not os.path.isdir(folder):
            raise SystemExit(f"no rip at {folder}")

    if only != "sfx":
        cuts = {}
        for slug, track in GENESIS_MUSIC.items():
            # only the loop is written: the intro `split` finds here is one search block, not music
            _, repeat = split(music, track)
            cuts[f"{slug}-repeat"] = repeat
        for slug, track in GENESIS_JINGLES.items():
            intro, repeat = split(music, track)
            cuts[slug] = intro if repeat is None else np.concatenate([intro, repeat])
        write_music(out, cuts, [f for n, f in cuts.items() if n.endswith("-repeat")])

    if only == "music":
        return
    levels = reference_levels()
    for slug, sound in GENESIS_SFX.items():
        cut = trim(decode(os.path.join(effects, sound + ".wav")))
        gain, bound = slot_gain(levels, slug, cut)
        write_effect(out, slug, cut, gain, f"{sound}.wav, held by {bound}")
    built = whoosh(effects)
    gain, bound = slot_gain(levels, "hard-drop", built)
    write_effect(out, "hard-drop", built, gain,
                 f"built from {HARD_DROP_NOISE[0]}.wav, held by {bound}")


def snes(only):
    """Kirby's Avalanche: music from SPC renders, effects from a sound test capture"""
    music = os.path.join(SOURCES, "snes-music")
    effects = os.path.join(SOURCES, "snes-sfx")
    out = os.path.join(THEMES, "snes")
    for folder in (music, effects):
        if not os.path.isdir(folder):
            raise SystemExit(f"no rip at {folder}")

    if only != "sfx":
        cuts, loops = {}, []
        for slug, (prefix, loop) in SNES_MUSIC.items():
            intro, repeat = measured_split(music, prefix, loop)
            cuts[f"{slug}-intro"], cuts[f"{slug}-repeat"] = intro, repeat
            loops.append(repeat)
        for slug, parts in SNES_JINGLES.items():
            cuts[slug] = np.concatenate([trim(decode(source(music, prefix))) for prefix in parts])
        write_music(out, cuts, loops)

    if only == "music":
        return
    levels = reference_levels()
    cuts = {slug: trim(centre(decode(clip(effects, number))))
            for slug, number in SNES_SFX.items()}
    gain, bound = set_gain(levels, cuts)
    print(f"the whole set at {20 * np.log10(gain):+.1f} dB, held by {bound}, peaking at "
          f"{max(peak(frames) for frames in cuts.values()) * gain:.2f}")
    for slug, frames in cuts.items():
        write_effect(out, slug, frames, gain, f"clip-{SNES_SFX[slug]}")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("theme", choices=["genesis", "snes"], help="which theme's rips to cut")
    parser.add_argument("--only", choices=["music", "sfx"],
                        help="cut half of it, leaving the other half's files as they are")
    arguments = parser.parse_args()
    {"genesis": genesis, "snes": snes}[arguments.theme](arguments.only)


if __name__ == "__main__":
    main()
