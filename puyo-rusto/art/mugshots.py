#!/usr/bin/env python3
"""Read a Mean Bean Machine character off the Mugshots sheet and off captures of the game.

Inputs, not in the repo: `SHEET` in `retro/`, and clips `~/Videos/Screencasts/<character>-<row>.mp4`.

    python3 mugshots.py prep <character>     # frame diffs, fades and key analysis
    python3 mugshots.py strips <character>   # one png per row, plus diff overlays
    python3 mugshots.py cut [out_dir]        # one png per character, in src/theme/genesis/mugshots/

`cut` also prints the `CAST` table and sweat to paste over `src/theme/genesis/mugshots.rs`.  The
rest (`Clip`, `autofit`, `against_ncc`, `checkerboard`) is a library for reading clips in a REPL.
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

# the ripper's key, and the ripper's navy backdrop, which in the game is a hole onto the wall
KEY = (0, 108, 108)
NAVY = (0, 0, 96)

ROWS = ("idle", "winning", "losing", "defeat")

# a frame and its pitch on the sheet; 80x56 is exactly the genesis panel's mugshot hole
FRAME_W, FRAME_H = 80, 56
PITCH_X, PITCH_Y = 81, 57

# a character: its block's origin on the sheet and the frame count of each of its four rows
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

# (x0, y0, sx, sy): box origin in video pixels, then video pixels per box pixel.  A clip not
# listed here wants `autofit`; two clips of the same size need not share a framing.
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


# --- the sheet ---

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
        row: [
            a[
                oy + PITCH_Y * i : oy + PITCH_Y * i + FRAME_H,
                ox + PITCH_X * c : ox + PITCH_X * c + FRAME_W,
            ]
            for c in range(n)
        ]
        for i, (row, n) in enumerate(zip(ROWS, counts))
    }


def enclosed_key(frame):
    """navy pixels that do not touch the frame's border: wall through a gap, or a navy detail
    that a plain colour key would punch through."""
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
    """what changes between the frames of each row, whether the last is a fade, and how the
    character keys."""
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
    """where a loose sprite sits over a frame, by brute force; works when the row's frames
    already draw the sprite's rest state."""
    fh, fw = sprite.shape[:2]
    best = None
    for oy in range(0, 56 - fh + 1):
        for ox in range(0, 80 - fw + 1):
            d = np.abs(frame[oy : oy + fh, ox : ox + fw].astype(int) - sprite.astype(int)).mean()
            if best is None or d < best[0]:
                best = (round(float(d), 2), ox, oy)
    return best


# --- the captures ---


def extract(name, row):
    """a clip's frames and timestamps.  The captures are variable rate, so `-vsync 0` keeps
    every frame and periods are only good to about 20%."""
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
    """`fit` by zero-mean normalised correlation, which ignores brightness and contrast.
    Returns a correlation to maximise rather than a residual."""
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
    """a coarse isotropic sweep at quarter size, then a fine anisotropic `fit` near it.  Pass
    every frame of the row as `tmpls`, and `frame=-3` for a game over clip, which starts mid-match."""
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
    """a fit's (score, sx, sy, x0, y0) as the (x0, y0, sx, sy) that `Clip` and CALIBRATION take"""
    _, sx, sy, x0, y0 = fit_result
    return (x0, y0, sx, sy)


def checkerboard(clip, sheet_frame, path, squares=8, scale=8):
    """capture and sheet frame interleaved in squares, which shows a one pixel misalignment
    where a residual cannot.  Look at it before trusting a fit."""
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
        """per-pixel maximum deviation from the median frame, with the box outlined.  The game's
        beans cross the box, so only motion that recurs is the character's."""
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
        """the box's mean level per frame; bursts of 18-55% are the danger flash, not the character"""
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
    """label each video frame by the nearest candidate, absolutely; use this where a candidate
    is a halved frame.  `region` must hold only what the sheet's diff says changes."""
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
    """`against` by normalised correlation, which ignores the danger flash.  Never use it where
    one candidate is the halving of another: NCC cannot tell a fade from its source."""
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
    """one frame per step of `ramp`, a list of colour pairs that replace `replace`"""
    out = []
    for pair in ramp:
        f = frame.copy()
        for src, dst in zip(replace, pair):
            f[np.all(frame == np.array(src), axis=2)] = dst
        out.append(f)
    return out


# --- the sweat ---


def drop_mask(a):
    """the blue of a sweat drop"""
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
    """drops a frame in the band of wall above the box, the only place a blue character's own
    art cannot count as sweat"""
    counts = []
    for f in clip.files:
        a = np.array(Image.open(f).convert("RGB")).astype(int)
        band = a[0 : max(clip.vy(0) - 3, 4), xslice[0] : xslice[1]]
        counts.append(len(blobs(drop_mask(band))))
    counts = np.array(counts)
    return counts.mean(), float((counts > 0).mean()), counts



# --- Cutting the theme art ---

# The frames of each row in play order, as sheet indices.  The half brightness fade that ends a
# defeat row is dropped, and each row is cut action first, rest last, because `LinearWithPause`
# holds the last frame of its strip and resumes from frame 0.
PLAY = {
    #            idle                winning             losing           defeat
    "frankly":  ([0, 1],            [0],                [0, 1],           [0]),
    # sheet frame 0 is frame 1 with the rim lights at another cycle step, so it is never played
    "arms":     ([2, 3, 2, 1],      [0, 1, 2, 1],       [0, 1],           [0]),
    "humpty":   ([1, 0],            [1, 2, 1, 0, 3, 0,
                                 1, 2, 1, 0, 3, 0], [0, 1, 2, 1],     [0]),
    "coconuts": ([1, 0, 1, 0],      [1, 0],             [0, 1, 2, 1],     [0]),
    "davy":     ([1, 0, 1, 0],      [0, 1],             [0, 1],           [0]),
    "skweel":   ([1, 2, 1, 0],      [0, 1, 2, 1],       [0, 1, 2, 3, 4, 5], [0]),
    "dynamight":([0, 1],            [0, 1, 2, 3, 4],    [0, 1],           [0]),
    "grounder": ([1, 2, 1, 0],      [1, 0, 1, 0],       [0, 1],           [0]),
    "spike":    ([1, 2, 1, 0],      [1, 0, 1, 0],       [1, 0, 1, 0],     [0]),
    "ffuzzy":   ([0, 1, 2, 1],      [0, 1, 2, 1],       [0, 1, 2, 1],     [0, 1, 2, 1]),
    "dragon":   ([1, 2, 1, 0],      [1, 0],             [1, 2, 1, 2, 1, 2, 0], [0]),
    "scratch":  ([1, 0],            [1, 0],             [1, 0],           [0]),
    "robotnik": ([1, 2, 1, 0],      [0, 1],             [0, 1],           [0]),
}

# How each row plays: (type, seconds for one whole pass, seconds an action frame is held).  The
# two are independent measurements off the captures; an `lwp` row's pause is the pass less the
# action.  Captures are variable rate, so all of it is +/-20%.
TIMING = {
    "frankly":  (("linear",   0.70, None), ("static", None, None),
                 ("linear",   0.37, None), ("static", None, None)),
    "arms":     (("lwp",    1.19, 0.12), ("linear",   0.56, None),
                 ("linear",   0.36, None), ("static", None, None)),
    "humpty":   (("lwp",    1.35, 0.33), ("lwp",    3.14, 0.10),
                 ("linear",   0.70, None), ("static", None, None)),
    "coconuts": (("lwp",    1.68, 0.16), ("lwp",    1.36, 0.53),
                 ("linear",   0.33, None), ("static", None, None)),
    "davy":     (("lwp",    2.48, 0.17), ("linear",   0.28, None),
                 ("linear",   0.31, None), ("static", None, None)),
    "skweel":   (("lwp",    1.27, 0.10), ("linear",   0.63, None),
                 ("linear", 0.66, None), ("static", None, None)),
    "dynamight":(("linear",   0.69, None), ("linear", 0.45, None),
                 ("linear",   0.25, None), ("static", None, None)),
    "grounder": (("lwp",    2.33, 0.10), ("lwp",    1.56, 0.19),
                 ("lwp",    1.62, 0.28), ("static", None, None)),
    "spike":    (("lwp",    2.40, 0.12), ("lwp",    2.34, 0.21),
                 ("lwp",    1.99, 0.16), ("static", None, None)),
    "ffuzzy":   (("linear",   0.54, None), ("linear",   0.67, None),
                 ("linear",   0.39, None), ("linear",   0.60, None)),
    "dragon":   (("lwp",    1.93, 0.13), ("lwp",    1.54, 0.35),
                 ("lwp",    1.60, 0.09), ("static", None, None)),
    "scratch":  (("lwp",    1.29, 0.40), ("lwp",    1.11, 0.22),
                 ("lwp",    1.10, 0.20), ("static", None, None)),
    "robotnik": (("lwp",    1.65, 0.09), ("linear",   0.29, None),
                 ("linear",   0.20, None), ("static", None, None)),
}

# The engine counts whole frame widths along a strip, so frames sit edge to edge.
ROW_PITCH = FRAME_H + 1

# --- The layers: drawn over a portrait, in the box, on a clock of their own ---

# A layer is either loose sprites or a palette cycle; a cycle is baked one frame per ramp step,
# so the renderer never cycles palettes.  A cycle frame holds only the cycled pixels, which is
# what lets it lie over a portrait that animates underneath.

# Arms' rim lights: `(224,224,0)` through eight shades, dark to bright
ARMS_RAMP = [
    [(96, 64, 0)], [(128, 64, 0)], [(160, 96, 32)], [(192, 128, 64)],
    [(224, 160, 96)], [(224, 192, 128)], [(224, 224, 160)], [(224, 224, 224)],
]
# Coconuts' coin, one ramp per row
COCONUTS_WIN = [
    [(64, 32, 0), (64, 32, 0)], [(128, 64, 0), (128, 64, 0)],
    [(160, 96, 32), (160, 96, 32)], [(160, 96, 32), (192, 128, 64)],
    [(192, 128, 64), (224, 160, 96)], [(224, 160, 96), (224, 192, 128)],
    [(224, 192, 128), (224, 224, 160)], [(224, 224, 160), (224, 224, 224)],
]
COCONUTS_LOSE = [
    [(224, 192, 128), (224, 224, 224)], [(160, 96, 64), (192, 128, 96)],
    [(64, 32, 0), (64, 32, 0)], [(224, 0, 0), (224, 0, 0)],
    [(128, 0, 0), (128, 0, 0)], [(32, 0, 0), (32, 0, 0)],
]
# Ffuzzy's eye yellow; its rest colour is ramp index 4, so the eyes pulse about it
FFUZZY_RAMP = [
    [(96, 96, 0), (64, 32, 0)], [(128, 128, 0), (96, 64, 0)], [(160, 160, 0), (128, 96, 0)],
    [(192, 192, 0), (160, 128, 0)], [(224, 224, 64), (192, 160, 32)],
]

# A ping-pong written out, holding each end once, where the engine's `YoYo` repeats them.
def pong(n):
    return list(range(n)) + list(range(n - 2, 0, -1))


# Each character's layers in draw order.  `play` is the frame order per row (`[]` is not drawn)
# and `timing` is as `TIMING`.
LAYERS = {
    "arms": [
        dict(
            what="rim lights",
            cycle=[(224, 224, 0)], ramp=ARMS_RAMP, bbox=(0, 44, 80, 12),
            # a slow ping-pong at idle, a fast one-way chase once winning or losing
            play={"idle": pong(8)[::-1], "winning": [7, 6, 5, 4, 3, 2, 1, 0],
                  "losing": [7, 6, 5, 4, 3, 2, 1, 0], "defeat": []},
            timing={"idle": ("linear", 1.65, None), "winning": ("linear", 0.33, None),
                    "losing": ("linear", 0.32, None), "defeat": None},
        ),
    ],
    "coconuts": [
        dict(
            what="coin",
            cycle=[(224, 128, 0), (224, 192, 96)], ramp=None, bbox=(44, 0, 17, 12),
            # losing snaps back rather than ping-ponging, which is the effect
            ramps={"winning": COCONUTS_WIN, "losing": COCONUTS_LOSE},
            play={"idle": [], "winning": pong(8), "losing": [0, 1, 2, 3, 4, 5], "defeat": []},
            timing={"idle": None, "winning": ("linear", 0.601, None),
                    "losing": ("linear", 0.400, None), "defeat": None},
        ),
    ],
    "ffuzzy": [
        dict(
            what="eye yellow",
            cycle=[(192, 192, 0), (160, 128, 0)], ramp=FFUZZY_RAMP, bbox=(25, 16, 25, 15),
            # off on losing, where the blink below owns the same pixels
            play={"idle": pong(5), "winning": pong(5), "losing": [], "defeat": pong(5)},
            timing={"idle": ("linear", 1.2, None), "winning": ("linear", 0.518, None),
                    "losing": None, "defeat": ("linear", 1.2, None)},
        ),
        dict(
            what="eyes",
            loose=[(734, 614, 32, 24), (767, 614, 32, 24), (800, 614, 32, 24)],
            anchors=[(24, 8)],
            # wide, narrowed, shut; action first so the pause holds them open
            play={"idle": [], "winning": [], "losing": [1, 2, 1, 0], "defeat": []},
            timing={"idle": None, "winning": None,
                    "losing": ("lwp", 1.2, 0.067), "defeat": None},
        ),
    ],
    "humpty": [
        dict(
            what="arc",
            loose=[(655, 14, 24, 8), (655, 23, 24, 8)], blanks=1,
            # flashes at one of four slots between his antenna balls rather than travelling
            anchors=[(17, 5), (25, 5), (33, 6), (41, 6)], wander=0.55,
            play={"idle": [0, 1, 2], "winning": [], "losing": [], "defeat": []},
            timing={"idle": ("lwp", 0.55, 0.033), "winning": None,
                    "losing": None, "defeat": None},
        ),
        dict(
            what="wrung hands",
            loose=[(736, 128, 24, 16), (736, 145, 24, 16), (736, 162, 24, 16)],
            anchors=[(18, 24), (18, 27), (50, 32), (56, 34), (57, 27), (63, 31)], wander=0.14,
            play={"idle": [], "winning": [], "losing": [0, 1, 2], "defeat": []},
            timing={"idle": None, "winning": None,
                    "losing": ("linear", 0.42, None), "defeat": None},
        ),
    ],
}

# --- The emitters: thrown off a character, drawn on the window and not clipped to the box ---

# `speed` is box pixels an axis per 60 Hz frame.

DIAG = 0.7071
UP_L, UP_R, DOWN_L, DOWN_R = (-DIAG, -DIAG), (DIAG, -DIAG), (-DIAG, DIAG), (DIAG, DIAG)

EMITTERS = {
    "frankly": [
        dict(
            what="antenna sparks",
            loose=[(408, 71, 8, 16), (417, 71, 8, 16), (426, 71, 8, 16), (435, 71, 8, 16)],
            # the two antenna balls, each throwing three ways but never into his body
            sources=[((6.0, 15.0), [UP_L, UP_R, DOWN_L]),
                     ((77.0, 15.0), [UP_R, UP_L, DOWN_R])],
            speed=1.8, life=0.28, fade_last=0.35, fps=14,
            trigger={"winning": ("every", 0.70)},
        ),
    ],
    "humpty": [
        dict(
            what="antenna bolts",
            loose=[(817, 71, 8, 16), (826, 71, 8, 16), (835, 71, 8, 16)],
            # his antenna tips as drawn in winning frame 1
            sources=[((28.5, 7.5), [UP_L]), ((50.5, 7.5), [UP_R])],
            speed=2.2, life=0.10, fade_last=0.3, fps=30,
            # the play frames where the antennae are drawn in, twice a winning pass
            trigger={"winning": ("frames", (0, 6))},
        ),
    ],
}

# --- The sweat: on no sheet, so drawn here and cut into every character's png at one place ---

SWEAT_BODY = (64, 128, 224)
SWEAT_CORE = (160, 224, 224)
SWEAT_BOX = 8
# a round 4x5 drop, hanging: `#` body, `o` core
SWEAT_ART = [
    " ## ",
    "#oo#",
    "#oo#",
    "#### ",
    " ## ",
]
SWEAT_SOURCES = [((10.0, 17.0), [UP_L, (-0.45, -0.89)]),
                 ((70.0, 17.0), [UP_R, (0.45, -0.89)])]
# above this much of the board filled, and at this many a second when it is full.  Above
# `DANGER_ENTER`, and only on the losing row, so a losing face over a low stack does not sweat.
SWEAT_ABOVE = 0.55
SWEAT_RATE = 10.0
SWEAT_SPEED = 1.2
SWEAT_LIFE = 0.60


def sweat_frame():
    """the shared drop, drawn rather than cut: no rip carries one."""
    a = np.zeros((SWEAT_BOX, SWEAT_BOX, 4), np.uint8)
    oy = (SWEAT_BOX - len(SWEAT_ART)) // 2
    ox = (SWEAT_BOX - max(len(r) for r in SWEAT_ART)) // 2
    for y, line in enumerate(SWEAT_ART):
        for x, ch in enumerate(line):
            if ch == "#":
                a[oy + y, ox + x] = (*SWEAT_BODY, 255)
            elif ch == "o":
                a[oy + y, ox + x] = (*SWEAT_CORE, 255)
    return a


def loose_frame(rect):
    """one loose sprite off the sheet, keyed on both the navy and the ripper's teal, which
    Humpty's wrung hands carry inside their rect as holes"""
    x, y, w, h = rect
    frame = sheet()[y : y + h, x : x + w].astype(int)
    rgb = frame.astype(np.uint8)
    hole = (np.abs(frame - np.array(KEY)).sum(-1) <= 6) | (
        np.abs(frame - np.array(NAVY)).sum(-1) <= 6
    )
    return np.dstack([rgb, np.where(hole, 0, 255).astype(np.uint8)])


def cycle_mask(name, row, cycled):
    """which pixels of a row cycle, as the union over the frames that row plays.  Per row,
    since Coconuts draws a different coin on each; unplayed frames can be at any cycle phase."""
    frames = [cut(name)[row][i] for i in set(PLAY[name][ROWS.index(row)])]
    union = None
    for frame in frames:
        mask = np.zeros(frame.shape[:2], bool)
        for source in cycled:
            mask |= np.abs(frame.astype(int) - np.array(source)).sum(-1) == 0
        if union is not None and mask.sum() != union.sum():
            # every played frame must cycle the same pixels, or the layer misses some lights
            raise SystemExit(
                "%s's %s row disagrees on which pixels cycle (%d px against %d): the ripper "
                "caught two of its frames at different points of the cycle, so one of them is "
                "the wrong frame to be playing" % (name, row, mask.sum(), union.sum())
            )
        union = mask if union is None else (union | mask)
    return union


def cycle_frame(name, row, cycled, replacement, bbox):
    """the cycled pixels of a row's own art, recoloured, and everything else transparent"""
    x, y, w, h = bbox
    union = cycle_mask(name, row, cycled)[y : y + h, x : x + w]
    out = np.zeros((h, w, 4), np.uint8)
    # both colours of a pair move together, so a step is a pulse and never a chase
    rest = cut(name)[row][PLAY[name][ROWS.index(row)][-1]].astype(int)
    for source, target in zip(cycled, replacement):
        mask = (np.abs(rest - np.array(source)).sum(-1) == 0)[y : y + h, x : x + w]
        out[mask] = (*target, 255)
    # pixels the rest frame caught mid-cycle take the pair's last colour
    stray = union & (out[..., 3] == 0)
    out[stray] = (*replacement[-1], 255)
    return out



def keyed(frame):
    """the frame as RGBA with the navy keyed out, including enclosed navy, which is always wall"""
    rgb = frame.astype(np.uint8)
    alpha = np.where(np.abs(frame.astype(int) - np.array(NAVY)).sum(-1) <= 6, 0, 255)
    return np.dstack([rgb, alpha.astype(np.uint8)])


# what a character is called on screen, since the sheet is keyed on short names
DISPLAY = {
    "frankly": "Frankly", "arms": "Arms", "humpty": "Humpty", "coconuts": "Coconuts",
    "davy": "Davy Sprocket", "skweel": "Skweel", "dynamight": "Dynamight",
    "grounder": "Grounder", "spike": "Spike", "ffuzzy": "Sir Ffuzzy-Logik",
    "dragon": "Dragon Breath", "scratch": "Scratch", "robotnik": "Dr. Robotnik",
}

SWEAT_ORIGIN = (0, 4 * ROW_PITCH)


def anim_type(kind, n, seconds, dwell):
    """one `FrameAnimationType` from a frame count and a `TIMING` entry.  Never a `YoYo`, which
    repeats each end; a ping-pong is cut unrolled and played `Linear`."""
    if kind == "static":
        return "FrameAnimationType::Static"
    if kind == "lwp":
        # `LinearWithPause` shows the last frame for a whole frame before the pause, so a
        # pass is n/fps + pause
        fps = max(1, round(1.0 / dwell))
        pause = max(0.0, seconds - n / fps)
        return (
            "FrameAnimationType::LinearWithPause {"
            " fps: %d, pause_for: Duration::from_millis(%d), resume_from_frame: 0 }"
        ) % (fps, round(pause * 1000))
    return "FrameAnimationType::Linear { fps: %d }" % max(1, round(n / seconds))


def layer_frames(name, layer):
    """every frame of one layer, per row, as RGBA; empty where the row does not draw it"""
    out = {}
    for row in ROWS:
        play = layer["play"][row]
        if not play:
            out[row] = []
            continue
        if "loose" in layer:
            blank = np.zeros((layer["loose"][0][3], layer["loose"][0][2], 4), np.uint8)
            pool = [loose_frame(r) for r in layer["loose"]] + [blank] * layer.get("blanks", 0)
        else:
            ramp = layer.get("ramps", {}).get(row) or layer["ramp"]
            pool = [
                cycle_frame(name, row, layer["cycle"], step, layer["bbox"]) for step in ramp
            ]
        out[row] = [pool[i] for i in play]
    return out


def cut_character(name, out_dir):
    """write one png of a character: four portrait rows in play order, then the sweat, then
    every layer and emitter this one has - all keyed, all frames edge to edge."""
    frames = cut(name)
    plays = PLAY[name]
    layers = LAYERS.get(name, [])
    emitters = EMITTERS.get(name, [])

    blocks = []  # (origin, size, per-row strips) for the layers, and (origin, size, strip)
    layer_art = [layer_frames(name, l) for l in layers]
    y = SWEAT_ORIGIN[1] + SWEAT_BOX + 1
    layer_origins = []
    for art in layer_art:
        size = next((f.shape[1::-1] for row in ROWS for f in art[row]), (1, 1))
        layer_origins.append(((0, y), size))
        y += 4 * (size[1] + 1)
    emitter_origins = []
    for emitter in emitters:
        w, h = emitter["loose"][0][2], emitter["loose"][0][3]
        emitter_origins.append(((0, y), (w, h)))
        y += h + 1

    width = max([len(p) * FRAME_W for p in plays]
                + [len(art[row]) * size[0]
                   for art, (_, size) in zip(layer_art, layer_origins) for row in ROWS]
                + [len(e["loose"]) * size[0] for e, (_, size) in zip(emitters, emitter_origins)]
                + [SWEAT_BOX])
    out = np.zeros((y - 1, width, 4), np.uint8)

    for r, (row, play) in enumerate(zip(ROWS, plays)):
        for i, index in enumerate(play):
            out[r * ROW_PITCH : r * ROW_PITCH + FRAME_H,
                i * FRAME_W : (i + 1) * FRAME_W] = keyed(frames[row][index])
    ox, oy = SWEAT_ORIGIN
    out[oy : oy + SWEAT_BOX, ox : ox + SWEAT_BOX] = sweat_frame()
    for art, ((lx, ly), (w, h)) in zip(layer_art, layer_origins):
        for r, row in enumerate(ROWS):
            for i, f in enumerate(art[row]):
                out[ly + r * (h + 1) : ly + r * (h + 1) + h, lx + i * w : lx + (i + 1) * w] = f
    for emitter, ((ex, ey), (w, h)) in zip(emitters, emitter_origins):
        for i, rect in enumerate(emitter["loose"]):
            out[ey : ey + h, ex + i * w : ex + (i + 1) * w] = loose_frame(rect)

    path = os.path.join(out_dir, f"{name}.png")
    Image.fromarray(out).save(path)
    counts = [len(p) for p in plays]
    extras = sum(len(art[row]) for art in layer_art for row in ROWS)
    extras += sum(len(e["loose"]) for e in emitters) + 1
    return path, out.shape, counts, extras, layer_origins, emitter_origins


def _rust_rows(name):
    parts = []
    for r, row in enumerate(ROWS):
        n = len(PLAY[name][r])
        parts.append("            (%d, %s)," % (n, anim_type(*TIMING[name][r][:1], n, *TIMING[name][r][1:])))
    return chr(10).join(parts)


def _rust_layers(name, layer_origins):
    layers = LAYERS.get(name, [])
    if not layers:
        return "            layers: &[],"
    lines = ["            layers: &["]
    for layer, ((lx, ly), (w, h)) in zip(layers, layer_origins):
        lines.append("                // %s" % layer["what"])
        lines.append("                LayerData {")
        lines.append("                    origin: (%d, %d)," % (lx, ly))
        lines.append("                    size: (%d, %d)," % (w, h))
        lines.append("                    row_pitch: %d," % (h + 1))
        lines.append("                    states: [")
        for row in ROWS:
            n = len(layer["play"][row])
            timing = layer["timing"][row]
            t = anim_type(*timing[:1], n, *timing[1:]) if timing else "FrameAnimationType::Static"
            lines.append("                        (%d, %s)," % (n, t))
        lines.append("                    ],")
        anchors = layer.get("anchors", [(layer["bbox"][0], layer["bbox"][1])] if "bbox" in layer else [(0, 0)])
        lines.append("                    anchors: &[%s]," % ", ".join("(%d, %d)" % a for a in anchors))
        wander = layer.get("wander")
        lines.append("                    wander: %s," % (
            "Some(Duration::from_millis(%d))" % round(wander * 1000) if wander else "None"))
        lines.append("                },")
    lines.append("            ],")
    return chr(10).join(lines)


def _rust_component(v):
    # clippy denies 0.7071 as an approximate constant, so print the Rust module's `DIAG`
    if abs(v) == DIAG:
        return "-DIAG" if v < 0 else "DIAG"
    return "%.4f" % v


def _rust_sources(sources):
    out = []
    for at, dirs in sources:
        out.append(
            "                        EmitterSource { at: (%.1f, %.1f), directions: &[%s] },"
            % (at[0], at[1], ", ".join(
                "(%s, %s)" % (_rust_component(x), _rust_component(y)) for x, y in dirs))
        )
    return chr(10).join(out)


def _rust_emitters(name, emitter_origins):
    emitters = EMITTERS.get(name, [])
    if not emitters:
        return "            emitters: &[],"
    lines = ["            emitters: &["]
    for emitter, ((ex, ey), (w, h)) in zip(emitters, emitter_origins):
        trigger = emitter["trigger"]
        triggers = []
        for row in ROWS:
            t = trigger.get(row)
            if t is None:
                triggers.append("None")
            elif t[0] == "every":
                triggers.append("Some(EmitterTrigger::Every(Duration::from_millis(%d)))"
                                % round(t[1] * 1000))
            else:
                triggers.append("Some(EmitterTrigger::OnFrame(&[%s]))"
                                % ", ".join(str(f) for f in t[1]))
        lines.append("                // %s" % emitter["what"])
        lines.append("                EmitterData {")
        lines.append("                    origin: (%d, %d)," % (ex, ey))
        lines.append("                    size: (%d, %d)," % (w, h))
        lines.append("                    frames: %d," % len(emitter["loose"]))
        lines.append("                    fps: %d," % emitter["fps"])
        lines.append("                    triggers: [")
        for t in triggers:
            lines.append("                        %s," % t)
        lines.append("                    ],")
        lines.append("                    sources: &[")
        lines.append(_rust_sources(emitter["sources"]))
        lines.append("                    ],")
        lines.append("                    speed: %.1f," % emitter["speed"])
        lines.append("                    life: Duration::from_millis(%d)," % round(emitter["life"] * 1000))
        lines.append("                    fade_last: %.2f," % emitter["fade_last"])
        lines.append("                },")
    lines.append("            ],")
    return chr(10).join(lines)


def rust_sweat():
    """the cast-wide emitter, whose art every character's png carries at the same place"""
    return chr(10).join([
        "    sweat: Some(EmitterData {",
        "        origin: (%d, %d)," % SWEAT_ORIGIN,
        "        size: (%d, %d)," % (SWEAT_BOX, SWEAT_BOX),
        "        frames: 1,",
        "        fps: 0,",
        "        triggers: [",
        "            None,",
        "            None,",
        "            Some(SWEAT_TRIGGER),",
        "            None,",
        "        ],",
        "        sources: &[",
        _rust_sources(SWEAT_SOURCES),
        "        ],",
        "        speed: %.1f," % SWEAT_SPEED,
        "        life: Duration::from_millis(%d)," % round(SWEAT_LIFE * 1000),
        "        fade_last: 0.40,",
        "    }),",
    ])


def rust_trigger():
    return ("const SWEAT_TRIGGER: EmitterTrigger = EmitterTrigger::Danger { above: %.2f, "
            "per_second: %.1f };") % (SWEAT_ABOVE, SWEAT_RATE)


def rust_table(geometry):
    """the cast table to paste into puyo-rusto/src/theme/genesis/mugshots.rs"""
    lines = []
    for name in CAST:
        layer_origins, emitter_origins = geometry[name]
        lines.append(chr(10).join([
            "        // %s" % name,
            "        CharacterData {",
            '            name: "%s",' % DISPLAY[name],
            "            file: sprites::%s," % name.upper(),
            "            states: [",
            _rust_rows(name),
            "            ],",
            "            // a mugshot is a strip in a fixed box, which is the other art model",
            "            // beside `RoutineArt` - see engine::render::character",
            "            routines: None,",
            _rust_layers(name, layer_origins),
            _rust_emitters(name, emitter_origins),
            "        },",
        ]))
    return chr(10).join(lines)


def cut_all(out_dir):
    os.makedirs(out_dir, exist_ok=True)
    total = 0
    extra = 0
    geometry = {}
    for name in CAST:
        path, shape, counts, extras, layer_origins, emitter_origins = cut_character(name, out_dir)
        geometry[name] = (layer_origins, emitter_origins)
        total += sum(counts)
        extra += extras
        print("  %-10s %-46s %dx%d  rows %s  extras %d" % (
            name, os.path.relpath(path), shape[1], shape[0], counts, extras))
    print()
    print("%d portrait frames over %d characters (127 on the sheet, less the 11 fades)"
          % (total, len(CAST)))
    print("%d layer, emitter and sweat frames beside them" % extra)
    return geometry


if __name__ == "__main__":
    if len(sys.argv) < 2 or sys.argv[1] not in ("prep", "strips", "cut"):
        print(__doc__)
        sys.exit(2)
    command = sys.argv[1]
    if command == "cut":
        out = sys.argv[2] if len(sys.argv) > 2 else os.path.join(
            HERE, "..", "src", "theme", "genesis", "mugshots")
        geometry = cut_all(out)
        print()
        print("// paste over CAST in puyo-rusto/src/theme/genesis/mugshots.rs")
        print(rust_trigger())
        print(rust_table(geometry))
        print()
        print("// ... and over the sweat of CharacterSetData")
        print(rust_sweat())
        sys.exit(0)
    if len(sys.argv) < 3 or sys.argv[2] not in CAST:
        print("characters:", ", ".join(sorted(CAST)))
        sys.exit(2)
    who = sys.argv[2]
    if command == "prep":
        prep(who)
    else:
        strips(who)
