#!/usr/bin/env python3
"""Cuts a retro theme's music and sound effects out of that game's own rips.

    python3 puyo-rusto/art/retro_audio.py genesis    # needs ffmpeg with libvorbis
    python3 puyo-rusto/art/retro_audio.py snes

One subcommand per theme, the shape `rip_retro.py` already has - `genesis` is Dr. Robotnik's
Mean Bean Machine and `snes` is Kirby's Avalanche. The sources sit under
`puyo-rusto/art/retro/<theme>-music` and `<theme>-sfx` and are **not in the repository**, on
the same footing as everything else in `art/`: they are downloads under their own names and
re-running this means finding them again. A dump that ships both a lossless render and
somebody's transcode of it is taken at the lossless one - see `source`.

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

**Mean Bean Machine's four stage tunes each loop from their first bar**, so none of them is
cut into a pair - the theme is handed four tracks that simply repeat, and the mixer is told
there is no one-shot. Nothing picks between them, a match is dealt one.

The rip does carry a `Stages 1-4 Intro` beside each of them and it is tempting to read that as
the lead-in half of the pair. It is not: that is the music of the *stage announcement screen*,
which the game plays before a match rather than at the top of one, and using it puts nine
seconds of interstitial in front of every match. What the search finds at the head of a stage
tune itself is 0.25s - one `loop_start` block, which is the block size and not an intro.

## Kirby's Avalanche, which names no loop at all

The SNES dump is a set of SPCs rendered to wav, and an SPC carries a *play length* and a fade
in its ID666 tag but no loop point - not even the whole seconds Mean Bean Machine's rip writes
down. So there is nothing to seed `loop_length` with and `loop_period` sweeps every lag
instead. That works here because of how these particular renders are laid out: each is its
tune played through **exactly twice and cut off dead**, with the tag's fade never applied, so
one lag scores near 1 and nothing else comes close.

Its stage tunes then loop from their first bar rather than opening on a lead-in the way Mean
Bean Machine's do - the `Stage Intro` tracks in that dump are the pre-stage screen's own
looping music and lead into nothing. What `loop_start` finds at the head of each one is not a
musical intro but the quarter second or so in which the SNES's echo buffer fills: the render
is bit-identical between the two passes from that point on and audibly thinner before it. Cut
there, and the pair the mixer gets is the cold start the game itself opens on and a loop with
the echo already under it.

There are three of them where Mean Bean Machine has four, which is why nothing here says how
many tracks a theme must have.

## Levels, which are the whole game's and not one theme's

A console dump is quieter than the modern rip everything else here comes off - Kirby's
Avalanche by four decibels, Mean Bean Machine by three - and it is also *peakier*, having no
compression on it. So `music_gain` lifts each cut as far as `reference_music_level` or as far
as the headroom allows, whichever comes first, and for both of these it is the headroom: they
end up peaking at `MUSIC_CEILING` and still a couple of decibels under the particle theme.

One gain over the whole set, never a normalise per track: the levels within a dump are the
game's own mix and are meant to be uneven, which is `sfx.py`'s rule.

What is left of the gap is not a gain's to close, and it is not closed here. Take the mean peak
of a theme's effects over the RMS of its music and every theme of Rustris and Dr. Rustario
lands between +6 and +13 dB, with Rustris's Game Boy theme at +9; this game's retro themes were
at +15 before the lift above. The rest comes off the effects, in
`src/theme/data.rs`'s `EFFECTS_TRIM` - a number there rather than a gain in the files because
`sfx.py`'s rip is not on this machine and because the particle and SNES themes play the *same*
effect files, so there is no per-theme cut to bake it into.

## The sound effects, and the two places this differs from `sfx.py`

`sfx.py` says in as many words that it does not normalise, because the levels are the
original's mix and are meant to be uneven. This rip has no mix left to keep: **every one of its
sixty files peaks at the same sample value**, -0.5 dBFS, so a bean settling arrives as loud as
a stage fanfare. Levelling is therefore not editorialising here, it is undoing what the rip
did. Each effect is scaled to the peak of *the particle theme's own sound for the same slot*,
read off `src/theme/sfx/` at run time rather than written down here - that theme's rip came
with its levels intact, and matching it is what makes the two themes sit at one volume.

The silence the rip pads both ends with is trimmed off first, with a short fade over the tail,
exactly as `sfx.py` does it.

The second difference is that one effect is not cut from the rip at all: `hard-drop`, the only
slot in the theme standing in for an action Mean Bean Machine does not have, is *built* - out of
the game's own noise, because the rip has no whoosh anywhere in it to take. See [`whoosh`]. It
is levelled by the same rule as everything else and needs no exception to it.
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
# The game's four stage tunes in stage order. Nothing picks between them - a match is dealt one
# - so the order is only the order they were written in. Every one of them loops from its first
# bar, which is what the pass and the loop being the same second says and what the search then
# confirms; the rip's four `... Intro` tracks are the stage announcement screen's music and are
# not these tunes' lead-ins, so nothing here reaches for them.
GENESIS_MUSIC = {
    "stages-1-4": ("05", 31, 31),
    "stages-5-8": ("07", 27, 26),
    "stages-9-12": ("09", 31, 31),
    "stage-13": ("11", 47, 47),
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
# The rip names the ones whoever made it recognised and numbers the rest `sfx_N`, and a name is
# taken at its word only where the sound bears it out. Most do: the seven `chain` steps are a
# rising ladder - the peak of chain 1 is 1456 Hz, chain 2 2018, and so on up - which is the
# sound Puyo is known for, and `level_start` is the fanfare it says it is. `bad puyo` is this
# game's nuisance bean: the plural is the shower of them arriving, and the three singulars are
# the sizes of a send, the way Puyo Puyo Tetris's rip grades its own `oj_okuri` in four. **The
# one that is simply wrong is `move`**, and it was believed for a while - see the entry.
#
# `clear_class` in `puyo-rusto/src/render.rs` grades a chain step 0..3 - chain 1, 2, 3, then
# everything from 4 up - so it is the first four steps that are wanted.
GENESIS_SFX = {
    # `move` is the rip's name and not the game's job: it is a 30ms burst of bright noise,
    # nothing like the blip a bean pair slides sideways on. What that blip actually is is
    # `puyo_sine` - sixteen milliseconds of 742 Hz and nothing else, which is what a move has
    # to be when it fires on every frame of a held direction. `short_noise` is the knock the
    # pair turns on. Neither is named for what it does, so both are matched by ear against
    # the emulated game rather than taken off the filename.
    "move": "puyo_sine",
    "rotate": "short_noise",
    # a pair coming to rest, and the beans left standing dropping into the gap a group left
    "lock": "puyo_blob",
    "settle": "puyo_blob_2",
    # no `hard-drop`: it is the one slot nothing in this rip can fill - see [`whoosh`]
    "pop-1": "chain_1",
    "pop-2": "chain_2",
    "pop-3": "chain_3",
    "pop-4": "chain_4",
    "attack": "bad_puyo_1",
    "garbage": "bad_puyos",
    "speed-up": "level_start",
    "pause": "select",
}


# Kirby's Avalanche's three stage tunes, in the order the dump numbers them. A track is
# `(rip number, loop)` - and unlike the Genesis table that second number is **not read off
# anything**: an SPC names no loop point, so it is what `loop_period` found when this table was
# written, kept as the assertion rather than as the input. A re-render laid out differently
# fails here rather than shipping a broken loop, which is the whole of what it is for.
SNES_MUSIC = {
    "stage-1": ("104", 41.665),
    "stage-2": ("106", 48.306),
    "stage-3": ("108", 23.594),
}

# ... and the cues it plays once, each `(rip number, ...)` joined end to end. The dump splits
# the win music over two SPCs because the game changes song halfway through it, and the pair is
# one flourish: part 1 then part 2, with the render's padding trimmed from between them. There
# is a `Stage Clear` in the dump too and nothing here to play it: only single player has an
# interstitial, and the Genesis theme leaves that music unset as well.
SNES_JINGLES = {
    "victory": ("115a", "115b"),
    "game-over": ("113",),
}

# The hard drop, which is the one sound in this theme that is **made** rather than cut.
#
# Mean Bean Machine has no hard drop, and unlike every other borrowed slot it has nothing close
# to one either. Score all sixty of the rip's files for how much of each is noise rather than
# tone and for how far its brightness falls over its length: the best whoosh in the game is
# `sfx_12`, a bright hiss that collapses into a 200 Hz tone, which sat in this slot for a while
# and is not a whoosh. Everything noisier than it sweeps the wrong way - `sfx_20` is the
# noisiest thing the game owns and its brightness *climbs*, because it is an impact and not a
# fall. There is no whoosh in there to find; the game never needed one.
#
# So the material is borrowed and only the gesture is built. `sfx_20`'s body, past the impact
# that opens it, is a third of a second of the Genesis's noise channel and nothing else, and a
# whoosh is that noise under a cutoff sliding downward with an envelope that swells and drops.
# What comes out is the console's own timbre making a shape the console never made - which is
# what this slot is, a sound for an action the game does not have.
#
# The numbers were set against the particle theme's own `hard-drop`, the only other answer to
# this question in the repository: 0.32s long, and falling. This is 0.30s.

# where the noise comes from: a sound in the rip, and the seconds of it that are pure noise
HARD_DROP_NOISE = ("sfx_20", 0.07, 0.45)
# `(seconds, from Hz, to Hz, how late the fall happens)`. The cutoff slides geometrically from
# the second number to the third along `t ** late`, so above 1 it holds bright and drops at the
# end rather than gliding evenly, which is what reads as a fall rather than as a fade.
HARD_DROP_SWEEP = (0.30, 8000, 400, 1.8)
# `(where the swell peaks, how sharply it falls away after)`, as fractions of the length
HARD_DROP_ENVELOPE = (0.45, 1.4)


# what a lossless render of a dump is called, best first: a folder that carries both the wavs
# and somebody's OGG transcode of them should be cut from the wavs
LOSSLESS = (".flac", ".wav")


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


def loop_period(mono, ref=(1.0, 8.0), shortest=8.0):
    """That same lag with nothing to seed the search - an SPC dump names no loop point.

    Eight seconds of the first pass correlated against *every* lag rather than the window a
    hint points at, and normalised by the energy under each window, so a loud bar somewhere
    else in the tune cannot outscore the true one. A render that is the tune played twice and
    cut off dead has exactly one lag that scores anywhere near 1.

    The answer is only as sharp as `loop_length`'s is, which is to the sample: the peak halves
    a hundred samples either side of it. What this cannot do is tell a loop from twice a loop,
    which is what the caller's assertion is for.
    """
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
    # the energy under the window at each lag, as a running sum rather than a convolution:
    # the window is eight seconds long and there are a couple of million lags
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

    The longest run of [`loop_matches`] over `threshold`, with a dip of up to `gap` blocks
    inside it closed over rather than ending it, since one quiet bar that scores badly is not
    the end of a loop.

    The default is the Genesis rip's number: the two passes there are the same performance but
    not the same samples - the rip's own noise floor puts them at 0.9 to 0.98 rather than at 1
    - so 0.9 is just under how well that render matches itself anywhere. **That is what the
    threshold means, and it is not a constant of the job**: an emulator's own render is
    bit-identical between passes wherever the SNES's noise channel is quiet and can sit at 0.85
    where it is not, so a rip that scores differently has to say so - see `measured_split`.
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


def measured_split(directory, prefix, loop):
    """`(intro, repeat)` of a rip that carries no loop table - the SNES dump

    The same two searches [`split`] makes, with the period *found* rather than refined, and
    with the rip's own number checked against the answer rather than fed to it.

    [`loop_start`] is then asked for the run that is good *for this render*: three fifths of
    however well it matches itself once it is repeating, rather than the Genesis rip's flat
    0.9. Two of these three tunes are bit-identical between passes and the third never scores
    better than about 0.85 anywhere - vibrato that the SPC does not restart with the sequence -
    so at 0.9 that third one reads as four seconds of intro it does not have, and four seconds
    of tune would be heard once and never again.
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
    """the one file in `directory` whose name begins with the rip's number

    ... preferring a lossless render where the folder holds one, since a dump downloaded whole
    may carry both the wavs and an OGG transcode of them under the same names.
    """
    matches = [f for f in sorted(os.listdir(directory)) if f.startswith(prefix + " ")]
    lossless = [f for f in matches if os.path.splitext(f)[1].lower() in LOSSLESS]
    matches = lossless or matches
    if len(matches) != 1:
        raise SystemExit(f"{prefix}: {len(matches)} matches in {directory}")
    return os.path.join(directory, matches[0])


def whoosh(directory):
    """The hard drop: the game's own noise under a falling cutoff. See [`HARD_DROP_NOISE`].

    A short-time Fourier transform with a Butterworth lowpass applied per frame and the corner
    moved between frames - a filter sweep written the plain way round, since the sound is a
    third of a second long and there is nothing to be gained from running a real filter over
    it. The noise is tiled up to whatever length is asked for; it is noise, so nothing shows.
    """
    source, head, tail = HARD_DROP_NOISE
    seconds, top, bottom, late = HARD_DROP_SWEEP
    noise = trim(decode(os.path.join(directory, source + ".wav"), 1))
    noise = noise[int(head * RATE):int(tail * RATE)]

    # A whole window of noise is analysed either side of the sound and then cut away. Without
    # it the first and last frames are the only ones covering their samples, the overlap-add
    # weight there is a fraction of what it is in the middle, and dividing by it puts a spike
    # six times the height of anything else at each end - which then sets the level for the
    # whole sound, since everything here is scaled by its peak.
    length, window, hop, order = int(seconds * RATE), 512, 128, 3
    span = length + 2 * window
    tiled = np.resize(noise, span + window)
    shaped, weight = np.zeros(span + window), np.zeros(span + window)
    taper = np.hanning(window)
    freqs = np.fft.rfftfreq(window, 1 / RATE)
    for at in range(0, span, hop):
        # the sweep is positioned against the sound itself, so it ignores the padding
        place = min(max((at - window) / length, 0.0), 1.0)
        corner = top * (bottom / top) ** (place ** late)
        spectrum = np.fft.rfft(tiled[at:at + window] * taper)
        shaped[at:at + window] += np.fft.irfft(
            spectrum / np.sqrt(1 + (freqs / corner) ** (2 * order)), window
        )
        weight[at:at + window] += taper * taper
    shaped = (shaped / np.maximum(weight, 1e-6))[window:window + length]

    # swell in, then fall away. A drop that opens at full height reads as a hit, and the whole
    # point of the sweep is that the landing is `lock`'s job rather than this sound's.
    peak_at, decay = HARD_DROP_ENVELOPE
    time = np.linspace(0, 1, length)
    swell = np.where(
        time < peak_at, (time / peak_at) ** 0.6, ((1 - time) / (1 - peak_at)) ** decay
    )
    return (shaped * swell).astype(np.float32)[:, None]


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


def rms(frames):
    return float(np.sqrt((frames.astype(np.float64) ** 2).mean()))


def reference_music_level():
    """what this game's own music sits at, as RMS

    Read off `src/theme/music/` rather than written down, the way [`reference_peaks`] reads the
    effects, and for the same reason: those tracks and every theme's effects are cut from one
    Puyo Puyo Tetris rip, so that is the level the effects were balanced against and the level
    a console dump has to be put back to. A theme can be switched with a keypress in the middle
    of a match as well, so a set left where its own dump left it is heard as the music going
    quiet rather than as the theme changing.
    """
    folder = os.path.join(THEMES, "music")
    loops = [name for name in sorted(os.listdir(folder)) if name.endswith("-repeat.ogg")]
    return float(np.mean([rms(decode(os.path.join(folder, name))) for name in loops]))


# The loudest a cut's own music is allowed to peak. Vorbis hands back a sample or two above the
# one it was given, and the mixer sums music and effects before anything clips, so the top of
# the scale is not this script's to spend.
MUSIC_CEILING = 0.95


def music_gain(loops, everything):
    """One gain for a whole cut's music: up to [`reference_music_level`], or as far as the
    headroom allows, whichever comes first.

    One gain over the set and never a normalise per track - the levels within a dump are the
    game's own mix and are meant to be uneven, which is `sfx.py`'s rule. `loops` is what the
    level is read off, since a repeat is what a match is actually played on; the peak is taken
    over `everything`, intros and jingles included, because they are written at this gain too.

    It is usually the ceiling that binds. A console dump has some three decibels more crest
    than the modern master it is being matched to - the rip is compressed and an emulator's
    render is not - so the last part of that gap cannot be had with a gain, and closing it is
    `data.rs`'s `EFFECTS_TRIM`, not a limiter here.
    """
    wanted = reference_music_level() / float(np.mean([rms(frames) for frames in loops]))
    room = MUSIC_CEILING / max(peak(frames) for frames in everything)
    return (wanted, "the level") if wanted <= room else (room, "the headroom")


def write_music(out, cuts, loops):
    """every track of one cut, at the one gain [`music_gain`] allows the set"""
    gain, bound = music_gain(loops, cuts.values())
    print(f"the whole set at {20 * np.log10(gain):+.1f} dB, held by {bound}, peaking at "
          f"{max(peak(frames) for frames in cuts.values()) * gain:.2f}")
    for name, frames in cuts.items():
        target = os.path.join(out, name + ".ogg")
        encode(frames * gain, target)
        print(f"{os.path.relpath(target, THEMES):32} {len(frames) / RATE:6.2f}s "
              f"{os.path.getsize(target) // 1024:5} KiB")


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


def write_effect(out, slug, levels, frames, provenance):
    """one effect, put back to the peak the particle theme's own sound for that slot has"""
    if slug not in levels:
        raise SystemExit(f"{slug}: the particle theme has no sound to level it against")
    gain = levels[slug] / peak(frames)
    target = os.path.join(out, slug + ".ogg")
    encode(frames * gain, target)
    print(f"{os.path.relpath(target, THEMES):34} {len(frames) / RATE:6.2f}s "
          f"{os.path.getsize(target) // 1024:5} KiB  {provenance} at {20 * np.log10(gain):+.1f} dB")


def genesis(only):
    """Dr. Robotnik's Mean Bean Machine"""
    music = os.path.join(SOURCES, "genesis-music")
    effects = os.path.join(SOURCES, "genesis-sfx")
    out = os.path.join(THEMES, "genesis")
    for folder in (music, effects):
        if not os.path.isdir(folder):
            raise SystemExit(f"no rip at {folder}")

    if only != "sfx":
        cuts = {}
        for slug, track in GENESIS_MUSIC.items():
            # only the loop is written: what `split` hands back as the intro of one of these is
            # a single `loop_start` block, the resolution of the search rather than any music,
            # and the theme says `None` for the one-shot rather than carry a quarter second of
            # the first bar and play it twice
            _, repeat = split(music, track)
            cuts[f"{slug}-repeat"] = repeat
        for slug, track in GENESIS_JINGLES.items():
            intro, repeat = split(music, track)
            cuts[slug] = intro if repeat is None else np.concatenate([intro, repeat])
        write_music(out, cuts, [f for n, f in cuts.items() if n.endswith("-repeat")])

    if only == "music":
        return
    levels = reference_peaks()
    for slug, sound in GENESIS_SFX.items():
        cut = trim(decode(os.path.join(effects, sound + ".wav")))
        write_effect(out, slug, levels, cut, f"{sound}.wav")
    write_effect(out, "hard-drop", levels, whoosh(effects), f"built from {HARD_DROP_NOISE[0]}.wav")


def snes(only):
    """Kirby's Avalanche

    Music only: the SNES dump is a set of SPCs and carries no sound effects at all, so this
    theme keeps playing the game's own until a rip of them turns up. `--only sfx` has nothing
    to do here and says so.
    """
    music = os.path.join(SOURCES, "snes-music")
    out = os.path.join(THEMES, "snes")
    if not os.path.isdir(music):
        raise SystemExit(f"no rip at {music}")
    if only == "sfx":
        raise SystemExit("the SNES dump is SPCs and carries no effects; there is nothing to cut")

    cuts, loops = {}, []
    for slug, (prefix, loop) in SNES_MUSIC.items():
        intro, repeat = measured_split(music, prefix, loop)
        cuts[f"{slug}-intro"], cuts[f"{slug}-repeat"] = intro, repeat
        loops.append(repeat)
    for slug, parts in SNES_JINGLES.items():
        cuts[slug] = np.concatenate([trim(decode(source(music, prefix))) for prefix in parts])
    write_music(out, cuts, loops)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("theme", choices=["genesis", "snes"], help="which theme's rips to cut")
    parser.add_argument("--only", choices=["music", "sfx"],
                        help="cut half of it, leaving the other half's files as they are")
    arguments = parser.parse_args()
    {"genesis": genesis, "snes": snes}[arguments.theme](arguments.only)


if __name__ == "__main__":
    main()
