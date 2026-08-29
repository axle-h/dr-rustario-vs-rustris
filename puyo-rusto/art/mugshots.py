#!/usr/bin/env python3
"""Read a Dr. Robotnik's Mean Bean Machine character off the Mugshots sheet, and off a
screen capture of the emulated game.

This is the tooling behind phase 6 of docs/puyo-puyo-plan.md, which is where every number it
produced is written up.  It exists because the sheet carries the *frames* and nothing else: not
the timings, not where the loose sprites go, not which palette ramp runs on which row, and not
what any of it means.  Those come off short clips Alex records one character at a time.

Neither the sheet nor the clips are in the repository.  The sheet lives beside the other rips in
`puyo-rusto/art/retro/`; the clips live in `~/Videos/Screencasts/` as `<character>-<row>.mp4`.

    python3 mugshots.py prep <character>       # frame diffs, fades and key analysis
    python3 mugshots.py strips <character>     # one png per row, plus diff overlays

Everything past that is a library, because reading a character is a conversation with the clip
rather than a fixed pipeline.  The usual shape is:

    fr = cut('grounder')
    c  = Clip('grounder', 'idle', CALIBRATION['grounder']['idle'])
    against_ncc(c, fr['idle'], region=(19, 8, 46, 32))   # prints the pose timeline

For a character with no calibration yet, fit one first and convert it:

    fr  = cut('davy')
    cal = calibration(autofit('davy', 'idle', fr['idle'], coarse=(3.2, 4.6, 0.05)))
    c   = Clip('davy', 'idle', cal)
    checkerboard(c, fr['idle'][0], 'check.png')          # ALWAYS look at this before trusting it

`against` compares absolutely and `against_ncc` compares by normalised correlation.  Reach for
the second whenever the danger flash would otherwise wreck the labelling, and for the first
whenever one of the candidates is a *halved* frame -- a fade correlates perfectly with what it
fades, so NCC reports the two as the same picture.

Assembled from the working code that produced the ten characters written up in the plan; it was
not re-run after being assembled into one file.
"""

import glob
import os
import sys
from collections import deque

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
SHEET = os.path.join(
    HERE,
    "retro",
    "Sega Genesis - Dr. Robotnik's Mean Bean Machine - Miscellaneous - Mugshots.png",
)
CLIPS = os.path.expanduser("~/Videos/Screencasts")

# the ripper's key, and the navy every frame is drawn on.  The navy is the *ripper's* backdrop,
# not the Genesis's: in the game the box is a hole and the dungeon wall is behind the character.
KEY = (0, 108, 108)
NAVY = (0, 0, 96)

ROWS = ("idle", "winning", "losing", "defeat")

# every frame is 80x56 on an 81x57 pitch, four rows of frames per character, each block starting
# where its name label ends.  So a character is an origin and four frame counts.
CAST = {
    "frankly": ((327, 14), (2, 1, 2, 2)),
    "arms": ((1, 14), (4, 3, 2, 2)),
    "humpty": ((493, 14), (2, 4, 3, 2)),
    "coconuts": ((846, 14), (2, 2, 3, 2)),
    "davy": ((1, 257), (2, 2, 2, 2)),
    "skweel": ((202, 257), (3, 3, 6, 2)),
    "dynamight": ((690, 257), (2, 5, 2, 2)),
    "grounder": ((1, 500), (3, 2, 2, 2)),
    "spike": ((246, 500), (3, 2, 2, 2)),
    "ffuzzy": ((491, 500), (3, 3, 3, 4)),
    "dragon": ((835, 500), (3, 2, 3, 2)),
    "scratch": ((1, 743), (2, 2, 2, 1)),
    "robotnik": ((169, 743), (3, 2, 2, 1)),
}

# what the clips are called, where that differs from the key above
VIDEO_NAME = {
    "davy": "davy-sprocket",
    "ffuzzy": "sir-ffuzzy-logic",
    "dragon": "dragon-breath",
    "robotnik": "dr-robotnik",
}

# box origin in video pixels, then video pixels per box pixel across and down.  Measured, so a
# re-run need not refit; a clip not listed here wants `autofit`.  Note that two clips of the same
# pixel size are not necessarily the same framing.
CALIBRATION = {
    "coconuts": {r: (135, 70, 4.225, 4.53) for r in ("idle", "winning", "losing")},
    "skweel": {
        "idle": (86, 61, 3.79, 3.77),
        "winning": (53, 56, 3.79, 3.81),
        "losing": (53, 56, 3.79, 3.81),
    },
    "dynamight": {
        "idle": (99, 68, 3.79, 3.81),
        "winning": (99, 68, 3.79, 3.81),
        "losing": (91, 65, 3.79, 3.81),
        "game-over": (91, 65, 3.79, 3.81),
    },
    "grounder": {
        r: (99, 68, 3.79, 3.81)
        for r in ("idle", "winning", "losing", "game-over")
    },
    "spike": {
        r: (97, 61, 3.64, 3.62)
        for r in ("idle", "winning", "losing-and-game-over")
    },
    "ffuzzy": {
        "idle": (92, 86, 3.76, 3.84),
        "winning": (92, 86, 3.76, 3.84),
        "losing": (91, 86, 3.79, 3.84),
        "game-over": (91, 87, 3.80, 3.80),
    },
    "dragon": {
        r: (77, 59, 3.71, 3.80)
        for r in ("idle", "winning", "losing", "game-over")
    },
}


# ---------------------------------------------------------------------------------------
# the sheet
# ---------------------------------------------------------------------------------------

_sheet = None


def sheet():
    global _sheet
    if _sheet is None:
        _sheet = np.array(Image.open(SHEET).convert("RGB")).astype(int)
    return _sheet


def cut(name):
    """one character as {row: [frames]}, each frame an 80x56 int array"""
    (ox, oy), counts = CAST[name]
    a = sheet()
    return {
        row: [a[oy + 57 * i : oy + 57 * i + 56, ox + 81 * c : ox + 81 * c + 80] for c in range(n)]
        for i, (row, n) in enumerate(zip(ROWS, counts))
    }


def enclosed_key(frame):
    """key pixels that do not touch the frame's border - wall showing through a gap in the
    character, or a dark navy *detail* that a plain colour key would punch a hole through.  Only
    Sir Ffuzzy-Logik (8 and 2 px) and Dr. Robotnik (48-60 px) have enough to want an eyeball."""
    m = np.all(frame == np.array(NAVY), axis=2)
    h, w = m.shape
    seen = np.zeros_like(m)
    q = deque()
    for y in range(h):
        for x in range(w):
            if (y in (0, h - 1) or x in (0, w - 1)) and m[y, x] and not seen[y, x]:
                seen[y, x] = True
                q.append((y, x))
    while q:
        y, x = q.popleft()
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            ny, nx = y + dy, x + dx
            if 0 <= ny < h and 0 <= nx < w and m[ny, nx] and not seen[ny, nx]:
                seen[ny, nx] = True
                q.append((ny, nx))
    return m & ~seen


def prep(name):
    """what changes between the frames of each row, whether the last one is a fade, and how the
    character keys.  Do this before opening a video: it is what tells you where to look."""
    fr = cut(name)
    print("=====", name, CAST[name])
    for row, fs in fr.items():
        print("--", row, len(fs))
        for i in range(len(fs) - 1):
            d = np.abs(fs[i + 1] - fs[i]).sum(axis=2) > 0
            ys, xs = np.nonzero(d)
            halving = ((fs[i] // 2) == fs[i + 1]).all(axis=2).mean()
            print(
                "   %d->%d %5d px rows %2d-%2d cols %2d-%2d   halving %.3f"
                % (i, i + 1, d.sum(), ys.min(), ys.max(), xs.min(), xs.max(), halving)
            )
        for i, f in enumerate(fs):
            teal = int(np.all(f == np.array(KEY), axis=2).sum())
            e = int(enclosed_key(f).sum())
            m = ~np.all(f == np.array(NAVY), axis=2)
            ys, xs = np.nonzero(m)
            print(
                "   frame %d teal %d enclosed-navy %d art rows %d-%d cols %d-%d"
                % (i, teal, e, ys.min(), ys.max(), xs.min(), xs.max())
            )
    return fr


def strips(name, out=".", scale=6):
    """one png per row, and one per row showing what differs between consecutive frames"""
    fr = cut(name)
    for row, fs in fr.items():
        img = Image.new("RGB", (80 * len(fs) * scale, 56 * scale), (255, 0, 255))
        for i, f in enumerate(fs):
            img.paste(
                Image.fromarray(f.astype(np.uint8)).resize(
                    (80 * scale, 56 * scale), Image.NEAREST
                ),
                (i * 80 * scale, 0),
            )
        img.save(os.path.join(out, f"{name}_{row}.png"))
        if len(fs) < 2:
            continue
        img = Image.new("RGB", (80 * (len(fs) - 1) * scale, 56 * scale), (255, 0, 255))
        for i in range(len(fs) - 1):
            d = np.abs(fs[i + 1] - fs[i]).sum(axis=2) > 0
            a = (fs[i] * 0.3).astype(np.uint8)
            a[d] = [255, 0, 255]
            img.paste(
                Image.fromarray(a).resize((80 * scale, 56 * scale), Image.NEAREST),
                (i * 80 * scale, 0),
            )
        img.save(os.path.join(out, f"{name}_{row}_diff.png"))


def find_overlay(frame, sprite):
    """where a loose sprite sits over a frame, by brute force.  Works whenever the overlay's
    *rest* state is what the row's frames already draw - Sir Ffuzzy-Logik's open eyes land at
    box (24, 8) on all three of his losing frames - which saves reading it off a capture."""
    fh, fw = sprite.shape[:2]
    best = None
    for oy in range(0, 56 - fh + 1):
        for ox in range(0, 80 - fw + 1):
            d = np.abs(frame[oy : oy + fh, ox : ox + fw].astype(int) - sprite.astype(int)).mean()
            if best is None or d < best[0]:
                best = (round(float(d), 2), ox, oy)
    return best


# ---------------------------------------------------------------------------------------
# the captures
# ---------------------------------------------------------------------------------------


def extract(name, row):
    """frames and their timestamps, separately.  `-vsync 0` matters: these are variable frame
    rate captures at roughly 20-30 fps against the Genesis's 60, and a timestamp is when the
    frame was captured rather than when it was drawn.  Expect +-20% on any period."""
    video = VIDEO_NAME.get(name, name)
    d = f"{video}_{row}"
    if not os.path.isdir(d):
        os.makedirs(d)
        clip = os.path.join(CLIPS, f"{video}-{row}.mp4")
        os.system(f'ffmpeg -v error -i "{clip}" -vsync 0 {d}/f%03d.png')
        os.system(
            "ffprobe -v error -select_streams v -show_entries frame=pts_time "
            f'-of csv=p=0 "{clip}" > {d}/pts.txt'
        )
    return (
        sorted(glob.glob(f"{d}/f*.png")),
        [float(line) for line in open(f"{d}/pts.txt") if line.strip()],
    )


def fit(vidfile, tmpl, sxr, syr, xr, yr, step=2):
    """absolute-difference fit of one sheet frame into one video frame, over the frame's own
    non-key pixels.  Returns (residual, sx, sy, x0, y0)."""
    m = ~np.all(tmpl == np.array(NAVY), axis=2)
    t = tmpl.astype(float)
    im = Image.open(vidfile).convert("RGB")
    W, H = im.size
    best = None
    for sx in sxr:
        w = int(round(80 * sx))
        for sy in syr:
            h = int(round(56 * sy))
            for y0 in range(yr[0], yr[1], step):
                if y0 + h > H:
                    continue
                for x0 in range(xr[0], xr[1], step):
                    if x0 + w > W:
                        continue
                    reg = np.array(
                        im.crop((x0, y0, x0 + w, y0 + h)).resize((80, 56), Image.BILINEAR)
                    ).astype(float)
                    d = np.abs(reg - t)[m].mean()
                    if best is None or d < best[0]:
                        best = (round(float(d), 3), round(float(sx), 3), round(float(sy), 3), x0, y0)
    return best


def ncc_fit(vidfile, tmpl, sxr, syr, xr, yr, step=1):
    """the same, by zero-mean normalised correlation, which is immune to the brightness and
    contrast difference between an upscaled capture and the sheet.  Returns a correlation to be
    *maximised*, where `fit` returns a residual to be minimised."""
    m = ~np.all(tmpl == np.array(NAVY), axis=2)
    t = tmpl.astype(float).mean(axis=2)[m]
    t = (t - t.mean()) / (t.std() + 1e-9)
    im = Image.open(vidfile).convert("RGB")
    W, H = im.size
    best = None
    for sx in sxr:
        w = int(round(80 * sx))
        for sy in syr:
            h = int(round(56 * sy))
            for y0 in range(yr[0], yr[1], step):
                if y0 + h > H:
                    continue
                for x0 in range(xr[0], xr[1], step):
                    if x0 + w > W:
                        continue
                    a = np.array(
                        im.crop((x0, y0, x0 + w, y0 + h)).resize((80, 56), Image.BILINEAR)
                    ).astype(float).mean(axis=2)[m]
                    a = (a - a.mean()) / (a.std() + 1e-9)
                    c = float((a * t).mean())
                    if best is None or c > best[0]:
                        best = (round(c, 4), round(float(sx), 3), round(float(sy), 3), x0, y0)
    return best


def autofit(name, row, tmpls, coarse=(3.2, 6.4, 0.1), frame=0):
    """coarse isotropic sweep on a quarter-size copy, then a fine anisotropic refine at full
    resolution near the answer.  A full two-axis search at full resolution is minutes a clip;
    this is seconds and gave the same answer every time it was checked.

    Pass every frame of the row as `tmpls` - the clip's first frame is not necessarily the row's
    first pose, and on a game over clip it is usually still mid-match, so pass `frame=-3`."""
    files, _ = extract(name, row)
    im = Image.open(files[frame]).convert("RGB")
    k = 4
    small = im.resize((im.size[0] // k, im.size[1] // k), Image.BILINEAR)
    best = None
    for t in tmpls:
        m = ~np.all(t == np.array(NAVY), axis=2)
        tf = t.astype(float)
        for s in np.arange(*coarse):
            w = int(round(80 * s / k))
            h = int(round(56 * s / k))
            if w < 8 or h < 6 or w > small.size[0] or h > small.size[1]:
                continue
            for y0 in range(0, small.size[1] - h + 1):
                for x0 in range(0, small.size[0] - w + 1):
                    reg = np.array(
                        small.crop((x0, y0, x0 + w, y0 + h)).resize((80, 56), Image.BILINEAR)
                    ).astype(float)
                    d = np.abs(reg - tf)[m].mean()
                    if best is None or d < best[0]:
                        best = (round(float(d), 3), float(s), x0 * k, y0 * k)
    _, s, x0, y0 = best
    out = None
    for t in tmpls:
        r = fit(
            files[frame],
            t,
            np.arange(s - 0.15, s + 0.16, 0.02),
            np.arange(s - 0.15, s + 0.16, 0.02),
            (x0 - 2 * k, x0 + 2 * k + 1),
            (y0 - 2 * k, y0 + 2 * k + 1),
            1,
        )
        if out is None or r[0] < out[0]:
            out = r
    return out


def calibration(fit_result):
    """`fit`, `ncc_fit` and `autofit` all return (score, sx, sy, x0, y0); `Clip` and the
    CALIBRATION table take (x0, y0, sx, sy). This is the conversion, and forgetting it is the
    obvious way to waste an afternoon."""
    _, sx, sy, x0, y0 = fit_result
    return (x0, y0, sx, sy)


def checkerboard(clip, sheet_frame, path, squares=8, scale=8):
    """the only honest way to check an alignment.  Two pictures side by side do not show a
    one-pixel error and a high residual is not a bad fit - Spike's best alignment sits at 25
    where Grounder's sits at 13, purely because he is a saturated orange."""
    a = clip.crop(clip.files[0], (0, 0, 80, 56))
    t = sheet_frame.astype(float)
    out = np.zeros((56, 80, 3))
    for y in range(56):
        for x in range(80):
            out[y, x] = a[y, x] if ((x // squares + y // squares) % 2 == 0) else t[y, x]
    Image.fromarray(out.astype(np.uint8)).resize((80 * scale, 56 * scale), Image.NEAREST).save(path)


class Clip:
    def __init__(self, name, row, cal):
        self.name, self.row = name, row
        self.x0, self.y0, self.sx, self.sy = cal
        self.files, self.pts = extract(name, row)

    def vx(self, x):
        return int(round(self.x0 + x * self.sx))

    def vy(self, y):
        return int(round(self.y0 + y * self.sy))

    def crop(self, f, region):
        x0, y0, x1, y1 = region
        return np.array(
            Image.open(f)
            .convert("RGB")
            .crop((self.vx(x0), self.vy(y0), self.vx(x1), self.vy(y1)))
            .resize((x1 - x0, y1 - y0), Image.BILINEAR)
        ).astype(float)

    def motion(self, path, tmin=None, tmax=None, gain=3):
        """per-pixel maximum deviation from the median frame, with the box outlined.  Draw this
        before measuring anything: it shows at a glance what moves and whether any of it leaves
        the box.  Everything lit outside the box is the two playfields, which the framing catches
        at both edges - and the game's own beans do cross the box, so a one-off streak is the
        game and only something that recurs is the character."""
        keep = [
            f
            for f, p in zip(self.files, self.pts)
            if (tmin is None or p >= tmin) and (tmax is None or p <= tmax)
        ]
        a = np.stack([np.array(Image.open(f).convert("RGB")).astype(np.int16) for f in keep])
        dev = np.abs(a - np.median(a, axis=0)).mean(axis=3).max(axis=0)
        img = np.dstack([np.clip(dev * gain, 0, 255).astype(np.uint8)] * 3)
        bx0, by0, bx1, by1 = self.vx(0), self.vy(0), self.vx(80), self.vy(56)
        img[by0, bx0:bx1] = [255, 0, 0]
        img[by1 - 1, bx0:bx1] = [255, 0, 0]
        img[by0:by1, bx0] = [255, 0, 0]
        img[by0:by1, bx1 - 1] = [255, 0, 0]
        Image.fromarray(img).save(path)
        outside = np.ones(dev.shape, bool)
        outside[by0:by1, bx0:bx1] = False
        return dev, (dev[outside].max(), int((dev[outside] > 25).sum()))

    def box_brightness(self):
        """the box's mean level per frame.  Flat means nothing is happening; a rise of 18-55% in
        bursts is the danger flash, which is the sprite plane and not the character."""
        return np.array(
            [
                np.array(Image.open(f).convert("RGB")).astype(float)[
                    self.vy(0) : self.vy(56), self.vx(0) : self.vx(80)
                ].mean()
                for f in self.files
            ]
        )


def _timeline(clip, out, labels, header):
    names = labels or [str(i) for i in range(64)]
    print(header)
    prev, start = None, 0
    for i, (p, lab, _) in enumerate(out):
        if lab != prev:
            if prev is not None:
                print(
                    "   %-12s t=%.3f dwell %.3f s (%.1f fr)"
                    % (names[prev], out[start][0], p - out[start][0], (p - out[start][0]) * 60)
                )
            prev, start = lab, i
    print(
        "   %-12s t=%.3f dwell %.3f s (%.1f fr)"
        % (names[prev], out[start][0], out[-1][0] - out[start][0], (out[-1][0] - out[start][0]) * 60)
    )
    print("   seq:", "".join(str(o[1]) for o in out))


def against(clip, cands, region=(0, 0, 80, 56), labels=None, quiet=False, tmin=None, tmax=None):
    """label each video frame by the nearest candidate, absolutely.  Use this whenever a halved
    frame is one of the candidates.

    Choosing the region is the whole game.  It must contain what the sheet's diff says changes
    and exclude everything else, or every flicker becomes a state."""
    x0, y0, x1, y1 = region
    t = [c[y0:y1, x0:x1].astype(float) for c in cands]
    out = []
    for f, p in zip(clip.files, clip.pts):
        if (tmin is not None and p < tmin) or (tmax is not None and p > tmax):
            continue
        a = clip.crop(f, region)
        e = [np.abs(a - s).mean() for s in t]
        out.append((p, int(np.argmin(e)), min(e)))
    if not quiet:
        _timeline(
            clip,
            out,
            labels,
            "%s/%s %s  residual mean %.1f max %.1f"
            % (clip.name, clip.row, region, np.mean([o[2] for o in out]), max(o[2] for o in out)),
        )
    return out


def against_ncc(clip, cands, region=(0, 0, 80, 56), labels=None, quiet=False, tmin=None, tmax=None):
    """the same, by normalised correlation, which ignores the danger flash entirely - it is a
    gain change.  Never use it where one candidate is the halving of another."""
    x0, y0, x1, y1 = region
    t = []
    for c in cands:
        v = c[y0:y1, x0:x1].astype(float).mean(axis=2).ravel()
        t.append((v - v.mean()) / (v.std() + 1e-9))
    out = []
    for f, p in zip(clip.files, clip.pts):
        if (tmin is not None and p < tmin) or (tmax is not None and p > tmax):
            continue
        a = clip.crop(f, region).mean(axis=2).ravel()
        a = (a - a.mean()) / (a.std() + 1e-9)
        sc = [float((a * s).mean()) for s in t]
        out.append((p, int(np.argmax(sc)), max(sc)))
    if not quiet:
        _timeline(
            clip,
            out,
            labels,
            "%s/%s %s  ncc mean %.3f min %.3f"
            % (clip.name, clip.row, region, np.mean([o[2] for o in out]), min(o[2] for o in out)),
        )
    return out


def palette_swapped(frame, replace, ramp):
    """one frame per step of a palette ramp, for matching a cycle against.  `replace` is the
    pair of colours the ripper says are replaced and `ramp` is the list of pairs it says they
    are replaced with."""
    out = []
    for pair in ramp:
        f = frame.copy()
        for src, dst in zip(replace, pair):
            f[np.all(frame == np.array(src), axis=2)] = dst
        out.append(f)
    return out


# ---------------------------------------------------------------------------------------
# the sweat, which is shared across the whole cast and graded by how bad the board is
# ---------------------------------------------------------------------------------------


def drop_mask(a):
    """the blue of a sweat drop.  A drop is a round blue blob about 3-4 screen pixels across
    with a lighter core."""
    r, g, b = a[:, :, 0], a[:, :, 1], a[:, :, 2]
    return (b > 130) & (b - r > 55) & (b - g > 25) & (g > r)


def blobs(m, lo=6, hi=90):
    """connected components of a mask, kept only if they are drop-sized and roughly round"""
    seen = np.zeros_like(m)
    out = []
    ys, xs = np.nonzero(m)
    for y, x in zip(ys, xs):
        if seen[y, x]:
            continue
        q = deque([(y, x)])
        seen[y, x] = True
        pts = []
        while q:
            cy, cx = q.popleft()
            pts.append((cy, cx))
            for dy in (-1, 0, 1):
                for dx in (-1, 0, 1):
                    ny, nx = cy + dy, cx + dx
                    if (
                        0 <= ny < m.shape[0]
                        and 0 <= nx < m.shape[1]
                        and m[ny, nx]
                        and not seen[ny, nx]
                    ):
                        seen[ny, nx] = True
                        q.append((ny, nx))
        if lo <= len(pts) <= hi:
            a = np.array(pts)
            h = a[:, 0].max() - a[:, 0].min() + 1
            w = a[:, 1].max() - a[:, 1].min() + 1
            if 0.5 < w / h < 2.0 and w <= 24 and h <= 24:
                out.append((float(a[:, 1].mean()), float(a[:, 0].mean()), len(pts)))
    return out


def sweat_rate(clip, xslice):
    """drops a frame in the band of wall *above* the box, which no character's art reaches.

    Count there and nowhere else: over the whole frame a blue-ish character counts as his own
    sweat, which gave Grounder twenty drops a frame, all of them his face.  In the band he gives
    0.00 for the same clip, and the visual check agrees."""
    counts = []
    for f in clip.files:
        a = np.array(Image.open(f).convert("RGB")).astype(int)
        band = a[0 : max(clip.vy(0) - 3, 4), xslice[0] : xslice[1]]
        counts.append(len(blobs(drop_mask(band))))
    counts = np.array(counts)
    return counts.mean(), float((counts > 0).mean()), counts


if __name__ == "__main__":
    if len(sys.argv) < 3 or sys.argv[1] not in ("prep", "strips"):
        print(__doc__)
        sys.exit(2)
    command, who = sys.argv[1], sys.argv[2]
    if who not in CAST:
        print("characters:", ", ".join(sorted(CAST)))
        sys.exit(2)
    if command == "prep":
        prep(who)
    else:
        strips(who)
