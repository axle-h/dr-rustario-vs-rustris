#!/usr/bin/env python3
"""Draws puyo-rusto/src/theme/modern/*.ogg.

The particle theme's sound effects are original the way its art was: a small chiptune
synthesiser, so they are generated rather than ripped. Everything lands as OGG Vorbis at
exactly 44,100 Hz, which is what the engine's decoder accepts and nothing else. The music a
match is played on is *not* here - `puyo-rusto/art/music.py` cuts that out of a rip.

    python3 puyo-rusto/art/audio.py       # needs ffmpeg with libvorbis

The chain sounds are the ones that matter: `pop-1` to `pop-4` are the same phrase a semitone
band higher each time, so a chain is *heard* climbing - which is the sound Puyo is known for
and what `clear_class` in `render.rs` grades a step for.
"""

import os
import subprocess
import tempfile
import wave

import numpy as np

RATE = 44100
OUT = os.path.normpath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src", "theme", "modern")
)

A4 = 440.0


def note(semitones_from_a4):
    return A4 * 2.0 ** (semitones_from_a4 / 12.0)


def silence(seconds):
    return np.zeros(int(seconds * RATE))


def envelope(n, attack, decay, sustain, release):
    """a plain ADSR over `n` samples, in seconds except `sustain`, which is a level"""
    a, d, r = (max(1, int(t * RATE)) for t in (attack, decay, release))
    s = max(0, n - a - d - r)
    return np.concatenate(
        [
            np.linspace(0, 1, a),
            np.linspace(1, sustain, d),
            np.full(s, sustain),
            np.linspace(sustain, 0, n - a - d - s),
        ]
    )[:n]


def wave_table(kind, phase):
    if kind == "square":
        return np.sign(np.sin(phase))
    if kind == "pulse":
        return np.where((phase / (2 * np.pi)) % 1.0 < 0.25, 1.0, -1.0)
    if kind == "triangle":
        return 2.0 / np.pi * np.arcsin(np.sin(phase))
    if kind == "saw":
        return 2.0 * ((phase / (2 * np.pi)) % 1.0) - 1.0
    return np.sin(phase)


def tone(freq, seconds, kind="square", gain=0.25, glide=None, adsr=(0.005, 0.05, 0.6, 0.08)):
    """one voice; `glide` is the frequency to slide to over the note"""
    n = max(1, int(seconds * RATE))
    t = np.arange(n) / RATE
    f = np.linspace(freq, glide, n) if glide else np.full(n, float(freq))
    phase = 2 * np.pi * np.cumsum(f) / RATE
    return wave_table(kind, phase) * envelope(n, *adsr) * gain


def noise(seconds, gain=0.2, adsr=(0.001, 0.02, 0.2, 0.05), lowpass=None):
    n = max(1, int(seconds * RATE))
    rng = np.random.default_rng(0xC0FFEE)
    sound = rng.uniform(-1, 1, n)
    if lowpass:
        # a one pole filter is enough to take the fizz off a hat or a thud
        alpha = np.clip(lowpass / (RATE / 2.0), 0.001, 1.0)
        filtered = np.zeros(n)
        acc = 0.0
        for i in range(n):
            acc += alpha * (sound[i] - acc)
            filtered[i] = acc
        sound = filtered * (1.0 / alpha) ** 0.5
    return sound * envelope(n, *adsr) * gain


def mix(*parts):
    n = max(len(p) for p in parts)
    out = np.zeros(n)
    for part in parts:
        out[: len(part)] += part
    return out


def at(track, offset_seconds, sound):
    """paint `sound` into `track` at an offset, growing the track if it has to"""
    start = int(offset_seconds * RATE)
    end = start + len(sound)
    if end > len(track):
        track = np.concatenate([track, np.zeros(end - len(track))])
    track[start:end] += sound
    return track


def normalise(sound, peak=0.86):
    high = np.max(np.abs(sound))
    return sound * (peak / high) if high > 0 else sound


def save(name, sound):
    """write one ogg, fading its last few ms so nothing clicks as it stops"""
    sound = normalise(sound)
    fade = min(len(sound), int(0.004 * RATE))
    sound[-fade:] *= np.linspace(1, 0, fade)
    pcm = (np.clip(sound, -1, 1) * 32767).astype("<i2")
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as tmp:
        path = tmp.name
    with wave.open(path, "wb") as f:
        f.setnchannels(1)
        f.setsampwidth(2)
        f.setframerate(RATE)
        f.writeframes(pcm.tobytes())
    target = os.path.join(OUT, name)
    subprocess.run(
        ["ffmpeg", "-y", "-loglevel", "error", "-i", path, "-ar", str(RATE), "-ac", "1",
         "-c:a", "libvorbis", "-q:a", "4", target],
        check=True,
    )
    os.unlink(path)
    print(f"{name} {len(sound) / RATE:.2f}s")


# ------------------------------------------------------------------ sound effects

def sfx_move():
    return tone(note(4), 0.05, "pulse", 0.3, adsr=(0.001, 0.01, 0.3, 0.03))


def sfx_rotate():
    return tone(note(7), 0.08, "square", 0.28, glide=note(14), adsr=(0.001, 0.02, 0.4, 0.04))


def sfx_lock():
    return mix(
        tone(note(-17), 0.13, "triangle", 0.5, glide=note(-24), adsr=(0.001, 0.04, 0.3, 0.08)),
        noise(0.07, 0.14, (0.001, 0.02, 0.15, 0.05), lowpass=1400),
    )


def sfx_settle():
    return mix(
        tone(note(-12), 0.10, "triangle", 0.32, glide=note(-19), adsr=(0.001, 0.03, 0.2, 0.06)),
        noise(0.05, 0.08, (0.001, 0.015, 0.1, 0.03), lowpass=2200),
    )


def sfx_hard_drop():
    return mix(
        tone(note(12), 0.16, "saw", 0.18, glide=note(-14), adsr=(0.001, 0.05, 0.3, 0.09)),
        at(silence(0.22), 0.12, sfx_lock()),
    )


def sfx_pop(step):
    """the chain sound: the same three note flourish, a whole tone higher per step"""
    root = 2 * step
    track = silence(0.5)
    for i, degree in enumerate((0, 4, 7)):
        track = at(
            track,
            0.045 * i,
            tone(note(root + degree + 7), 0.30, "pulse", 0.24, adsr=(0.002, 0.06, 0.45, 0.20)),
        )
    track = at(track, 0.0, noise(0.09, 0.10, (0.001, 0.03, 0.1, 0.05), lowpass=3600))
    return track


def sfx_attack_sent():
    return tone(note(-5), 0.30, "square", 0.24, glide=note(19), adsr=(0.004, 0.08, 0.6, 0.15))


def sfx_garbage():
    track = silence(0.45)
    for i in range(4):
        track = at(
            track,
            0.06 * i,
            tone(note(-9 - 3 * i), 0.22, "triangle", 0.26, adsr=(0.002, 0.05, 0.4, 0.12)),
        )
    return at(track, 0.0, noise(0.30, 0.14, (0.01, 0.10, 0.3, 0.18), lowpass=900))


def sfx_speed_up():
    track = silence(0.6)
    for i, degree in enumerate((0, 4, 7, 12)):
        track = at(track, 0.075 * i, tone(note(degree), 0.28, "pulse", 0.22))
    return track


def sfx_pause():
    return mix(
        tone(note(7), 0.11, "square", 0.22),
        at(silence(0.3), 0.11, tone(note(2), 0.16, "square", 0.20)),
    )


# ------------------------------------------------------------------ music

# The music a match is played on is not here: it is a Puyo Puyo Tetris rip, cut into an intro
# and a loop by `puyo-rusto/art/music.py`. What is left is the two stings, which are the
# theme's own.


def music_victory():
    track = silence(2.6)
    for i, degree in enumerate((0, 4, 7, 12, 7, 12, 16)):
        track = at(track, 0.11 * i, tone(note(degree), 0.5, "pulse", 0.20))
    for i, degree in enumerate((-12, -5, 0)):
        track = at(track, 0.11 * i, tone(note(degree), 1.6, "triangle", 0.26))
    return track


def music_game_over():
    track = silence(3.0)
    for i, degree in enumerate((0, -2, -5, -9)):
        track = at(
            track,
            0.34 * i,
            tone(note(degree), 0.9, "square", 0.18, adsr=(0.01, 0.25, 0.4, 0.35)),
        )
        track = at(
            track,
            0.34 * i,
            tone(note(degree - 24), 0.9, "triangle", 0.26, adsr=(0.01, 0.25, 0.4, 0.35)),
        )
    return track


def main():
    save("move.ogg", sfx_move())
    save("rotate.ogg", sfx_rotate())
    save("lock.ogg", sfx_lock())
    save("settle.ogg", sfx_settle())
    save("hard-drop.ogg", sfx_hard_drop())
    for step in range(4):
        save(f"pop-{step + 1}.ogg", sfx_pop(step))
    save("attack.ogg", sfx_attack_sent())
    save("garbage.ogg", sfx_garbage())
    save("speed-up.ogg", sfx_speed_up())
    save("pause.ogg", sfx_pause())
    save("victory.ogg", music_victory())
    save("game-over.ogg", music_game_over())


if __name__ == "__main__":
    main()
