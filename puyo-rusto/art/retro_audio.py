#!/usr/bin/env python3
"""Cuts a retro theme's music and sound effects out of that game's own rips.

    python3 puyo-rusto/art/retro_audio.py genesis    # needs ffmpeg with libvorbis

One subcommand per theme, the shape `rip_retro.py` already has - `genesis` is Dr. Robotnik's
Mean Bean Machine; `snes` joins it when its rips are found. The sources sit under
`puyo-rusto/art/retro/<theme>-music` and `<theme>-sfx` and are **not in the repository**, on
the same footing as everything else in `art/`: they are downloads under their own names and
re-running this means finding them again.

What the engine will take, and so what comes out of here:

* OGG Vorbis at exactly 44,100 Hz - `engine/src/audio/decode.rs` rejects anything else - at
  `-q:a 6`, which is what `music.py` and `sfx.py` already write.
* the mixer has no loop marker, so a track is *split*: `StructuredMusic::new(intro, repeat)`
  plays one file once and loops the other forever.

## The music, and why the split is measured rather than read

The rip renders a track the way this kind of rip always does - the intro, then the loop
**twice**, then a fade over the last ten seconds or so - and carries the loop point only as a
table of whole seconds, which is a third of a bar out at this tempo and would put an audible
stumble in every loop. So the split is found in the audio instead: the lag at which the render
repeats itself is the loop length, to the sample (`loop_length`), and the first block from
which it repeats is where the loop starts (`loop_start`). The table's seconds are kept as the
hint the search starts from and as the assertion it has to agree with, so a rip that is laid
out differently fails here rather than shipping a broken loop.

Landing *late* on the loop start is harmless and landing early is not - the render is periodic
from the true start onwards, so any later point loops just as seamlessly, while an earlier one
drags a bar of the intro into the loop. `loop_start` therefore reports the run it is sure of
and never reaches back before it.

**Mean Bean Machine writes each stage's lead-in as a track of its own**, which is exactly the
pair the mixer wants: `Stages 1-4 Intro` is the one-shot and `Stages 1-4` is what repeats. It
has four stage tunes, which is `theme::GAME_MUSIC_TRACKS` - nothing picks between them, a match
is dealt one, and every theme's table is the same length so that none of them is heard less
often than the others.

## The sound effects, and the one place this differs from `sfx.py`

`sfx.py` says in as many words that it does not normalise, because the levels are the
original's mix and are meant to be uneven. This rip has no mix left to keep: **every one of its
sixty files peaks at the same sample value**, -0.5 dBFS, so a bean settling arrives as loud as
a stage fanfare. Levelling is therefore not editorialising here, it is undoing what the rip
did. Each effect is scaled to the peak of *the particle theme's own sound for the same slot*,
read off `src/theme/sfx/` at run time rather than written down here - that theme's rip came
with its levels intact, and matching it is what makes the two themes sit at one volume.

The silence the rip pads both ends with is trimmed off first, with a short fade over the tail,
exactly as `sfx.py` does it.
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

# how quiet, against a sound's own peak, still counts as the padding the rip added, and how
# much of the head to keep and how long to fade the tail once it has been found - `sfx.py`'s
# numbers, since it is the same job on a different rip
SILENCE = 0.005
LEAD_IN = 0.004
FADE_OUT = 0.008

# Mean Bean Machine's music. A track is `(rip number, one pass, loop)` - the number is the
# filename prefix the rip gives it, and the two seconds are the rip's own loop table: how long
# one whole pass is and how much of that pass repeats, so the intro is the difference. They are
# whole seconds and are used as whole seconds: the loop is what `loop_length` starts its search
# from and the intro is what the answer is checked against, never what is cut.
#
# The game's four stage tunes in stage order, each with the lead-in the game writes as a track
# of its own. Nothing picks between them - a match is dealt one - so the order is only the
# order they were written in, but the *count* has to be `theme::GAME_MUSIC_TRACKS`.
GENESIS_MUSIC = {
    "stages-1-4": (("04", 9, 8), ("05", 31, 31)),
    "stages-5-8": (("06", 8, 4), ("07", 27, 26)),
    "stages-9-12": (("08", 9, 8), ("09", 31, 31)),
    "stage-13": (("10", 9, 9), ("11", 47, 47)),
}

# ... and the two it plays once. `Victory!` is the game's own three second flourish and has no
# loop at all; there is no track called game over, because what this game plays when you are
# buried is the music of the continue screen it puts you on, so one pass of that is what the
# theme gets.
GENESIS_JINGLES = {
    "victory": ("21", 3, None),
    "game-over": ("15", 13, 13),
}

# Which of the rip's wavs each of the theme's effects is.
#
# The rip names the ones whoever made it recognised and numbers the rest `sfx_N`, so the named
# ones are taken at their word: `move`, the seven `chain` steps (a rising ladder - the peak of
# chain 1 is 1456 Hz, chain 2 2018, and so on up - which is the sound Puyo is known for), and
# `level_start`. `bad puyo` is this game's nuisance bean: the plural is the shower of them
# arriving, and the three singulars are the sizes of a send, the way Puyo Puyo Tetris's rip
# grades its own `oj_okuri` in four.
#
# `clear_class` in `puyo-rusto/src/render.rs` grades a chain step 0..3 - chain 1, 2, 3, then
# everything from 4 up - so it is the first four steps that are wanted.
GENESIS_SFX = {
    "move": "move",
    "rotate": "puyo_sine",
    # a pair coming to rest, and the beans left standing dropping into the gap a group left
    "lock": "puyo_blob",
    "settle": "puyo_blob_2",
    # Mean Bean Machine has no hard drop, so there is no sound of one to take. This is the
    # closest thing the game owns: the short noise burst, which reads as the whoosh of the
    # fall, with `lock` landing on top of it the way it does after any other drop - which is
    # exactly what the particle theme does with Tetris's `hdrop`.
    "hard-drop": "short_noise",
    "pop-1": "chain_1",
    "pop-2": "chain_2",
    "pop-3": "chain_3",
    "pop-4": "chain_4",
    "attack": "bad_puyo_1",
    "garbage": "bad_puyos",
    "speed-up": "level_start",
    "pause": "select",
}


def decode(path, channels=2):
    """the whole file as interleaved i16 at the mixer's rate, as floats"""
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
    """The lag, to the sample, at which the render starts repeating itself.

    A plain cross correlation of fifteen seconds of the first pass against the window the hint
    points at. The minimum is sharp - a sample either side of it the difference doubles - so
    the whole seconds the rip's table carries are enough to start from.
    """
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


def loop_start(mono, length, block=0.25, gap=8):
    """Where that repeat begins: everything before it is the intro.

    Per quarter second, how well the render matches itself a loop later. The two passes are the
    same performance but not the same samples - the rip's own noise floor puts them at 0.9 to
    0.98 rather than at 1 - so the run is taken at 0.9, and a dip of up to `gap` blocks inside
    it is closed over rather than ending it, since one quiet bar that scores badly is not the
    end of a loop.
    """
    size = int(block * RATE)
    count = (len(mono) - length) // size
    first = mono[:count * size].reshape(count, size)
    second = mono[length:length + count * size].reshape(count, size)
    scale = np.sqrt((first * first).sum(axis=1) * (second * second).sum(axis=1)) + 1e-12
    matches = (first * second).sum(axis=1) / scale > 0.9

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
    """`(intro, repeat)` of one rip: everything before the loop, and exactly one pass of it

    A track the rip gives no loop is all intro and no repeat, which is what the mixer plays
    through once.
    """
    prefix, whole, loop = track
    frames = decode(source(directory, prefix))
    if loop is None:
        return frames, None
    mono = frames.mean(axis=1)
    length = loop_length(mono, loop)
    start = loop_start(mono, length)
    # the loop length the search found is sample exact and the rip's table agrees with it to
    # the second; what is worth asserting is the *start*, since that is the half read out of
    # the audio alone, and the rip's own two numbers say what it should be
    if abs(start - (whole - loop) * RATE) > 1.5 * RATE:
        raise SystemExit(
            f"{prefix}: the loop starts at {start / RATE:.2f}s where the rip's {whole}s pass "
            f"of a {loop}s loop puts it at {whole - loop}s - is this rip laid out differently?"
        )
    return frames[:start], frames[start:start + length]


def source(directory, prefix):
    """the one file in `directory` whose name begins with the rip's number"""
    matches = [f for f in sorted(os.listdir(directory)) if f.startswith(prefix + " ")]
    if len(matches) != 1:
        raise SystemExit(f"{prefix}: {len(matches)} matches in {directory}")
    return os.path.join(directory, matches[0])


def trim(frames):
    """cut the rip's padding off both ends and fade what is left of the tail"""
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


def reference_peaks():
    """what the particle theme's own effect for each slot peaks at

    Read rather than written down: that theme is levelled the way its rip was mixed, and these
    are the numbers this one has to be put back to. A slot the particle theme has no sound for
    would be a slot the engine has no key for, so a miss is a mistake and says so.
    """
    folder = os.path.join(THEMES, "sfx")
    return {
        os.path.splitext(name)[0]: peak(decode(os.path.join(folder, name)))
        for name in sorted(os.listdir(folder))
        if name.endswith(".ogg")
    }


def genesis():
    """Dr. Robotnik's Mean Bean Machine"""
    music = os.path.join(SOURCES, "genesis-music")
    effects = os.path.join(SOURCES, "genesis-sfx")
    out = os.path.join(THEMES, "genesis")
    for folder in (music, effects):
        if not os.path.isdir(folder):
            raise SystemExit(f"no rip at {folder}")

    def write(name, frames):
        target = os.path.join(out, name + ".ogg")
        encode(frames, target)
        print(f"{os.path.relpath(target, THEMES):32} {len(frames) / RATE:6.2f}s "
              f"{os.path.getsize(target) // 1024:5} KiB")

    for slug, (lead, loop) in GENESIS_MUSIC.items():
        # the lead-in is a rip like any other, two passes and a fade, so one pass of it is the
        # whole of what the game plays before the stage tune takes over - and whatever the
        # stage tune has of its own before its loop goes on the end of it
        head, body = split(music, lead)
        intro, repeat = split(music, loop)
        write(f"{slug}-intro", np.concatenate([head, body, intro]))
        write(f"{slug}-repeat", repeat)

    for slug, track in GENESIS_JINGLES.items():
        intro, repeat = split(music, track)
        write(slug, intro if repeat is None else np.concatenate([intro, repeat]))

    levels = reference_peaks()
    for slug, sound in GENESIS_SFX.items():
        if slug not in levels:
            raise SystemExit(f"{slug}: the particle theme has no sound to level it against")
        frames = trim(decode(os.path.join(effects, sound + ".wav")))
        gain = levels[slug] / peak(frames)
        target = os.path.join(out, slug + ".ogg")
        encode(frames * gain, target)
        print(f"{os.path.relpath(target, THEMES):34} {len(frames) / RATE:6.2f}s "
              f"{os.path.getsize(target) // 1024:5} KiB  {sound}.wav at {20 * np.log10(gain):+.1f} dB")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("theme", choices=["genesis"], help="which theme's rips to cut")
    {"genesis": genesis}[parser.parse_args().theme]()


if __name__ == "__main__":
    main()
