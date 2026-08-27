#!/usr/bin/env python3
"""Draws puyo-rusto/src/theme/modern/sprites.png.

The particle theme's art is original, and a Puyo `CellId` carries a four bit mask of which
neighbours share its colour - so the sheet is keyed on `colour x 16` and there are eighty
coloured variants before nuisance and the tray icons. Nobody draws eighty sprites by hand.

Each puyo is the union of a body (a circle) and one neck per linked direction (a box running
from the centre out past the cell edge), as a signed distance field: `sdf < 0` is the puyo,
the band just inside it is the rim, and the necks run *past* the edge so no rim is drawn
across a join and two linked puyos meet flush.

    python3 puyo-rusto/art/sprites.py

Layout, which `theme/modern/mod.rs` mirrors in arithmetic:

    block (col, row) -> (PAD + PITCH * col, PAD + PITCH * row), BLOCK square
    rows 0-4  one colour each (red, green, blue, yellow, purple), column = link mask bits
    row 5     col 0 nuisance, cols 1-3 the tray's small, large and rock symbols

The mask bits are `LinkMask` in `game/cell.rs`: up 1, down 2, left 4, right 8.
"""

import os

import numpy as np
from PIL import Image

BLOCK = 64
PAD = 4
PITCH = BLOCK + 2 * PAD
COLUMNS = 16
ROWS = 6

# how far past the cell edge a neck runs, so a rim is never drawn across a join
OVERHANG = 8

BODY_RADIUS = 0.455
NECK_HALF_WIDTH = 0.33
# how much of a fillet the neck leaves where it flows out of the body
FILLET = 0.14
RIM = 3.0

OUT = os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "..", "src", "theme", "modern", "sprites.png"
)

# base, dark (rim), light (top of the gradient)
COLORS = [
    ((0xF0, 0x44, 0x2E), (0x7A, 0x14, 0x0C), (0xFF, 0x9C, 0x84)),  # red
    ((0x3C, 0xC6, 0x3C), (0x11, 0x60, 0x18), (0x9E, 0xF5, 0x88)),  # green
    ((0x2E, 0x6B, 0xF0), (0x0E, 0x2A, 0x78), (0x8C, 0xBD, 0xFF)),  # blue
    ((0xF0, 0xC4, 0x1E), (0x7C, 0x59, 0x00), (0xFF, 0xF0, 0x96)),  # yellow
    ((0xB8, 0x45, 0xE0), (0x53, 0x10, 0x6E), (0xE9, 0xA8, 0xF8)),  # purple
]
NUISANCE = ((0xB9, 0xBE, 0xC8), (0x4E, 0x54, 0x60), (0xEC, 0xEF, 0xF5))

# the canvas each sprite is drawn on: the cell plus the overhang the necks run into
SIZE = BLOCK + 2 * OVERHANG
GRID_Y, GRID_X = np.mgrid[0:SIZE, 0:SIZE]
GRID_X = GRID_X + 0.5
GRID_Y = GRID_Y + 0.5
CENTRE = SIZE / 2.0


def circle_sdf(cx, cy, r):
    return np.hypot(GRID_X - cx, GRID_Y - cy) - r


def box_sdf(x0, y0, x1, y1):
    """distance to an axis aligned box, negative inside"""
    dx = np.maximum(x0 - GRID_X, GRID_X - x1)
    dy = np.maximum(y0 - GRID_Y, GRID_Y - y1)
    outside = np.hypot(np.maximum(dx, 0), np.maximum(dy, 0))
    inside = np.minimum(np.maximum(dx, dy), 0)
    return outside + inside


def smooth_min(a, b, k):
    """union with a fillet, so a neck flows out of the body rather than notching into it"""
    h = np.clip(k - np.abs(a - b), 0.0, None) / k
    return np.minimum(a, b) - h * h * k * 0.25


def puyo_sdf(links):
    """the body, plus a neck out to the canvas edge for every linked direction"""
    half = NECK_HALF_WIDTH * BLOCK
    sdf = circle_sdf(CENTRE, CENTRE, BODY_RADIUS * BLOCK)
    necks = {
        1: (CENTRE - half, 0.0, CENTRE + half, CENTRE),  # up
        2: (CENTRE - half, CENTRE, CENTRE + half, SIZE),  # down
        4: (0.0, CENTRE - half, CENTRE, CENTRE + half),  # left
        8: (CENTRE, CENTRE - half, SIZE, CENTRE + half),  # right
    }
    for bit, box in necks.items():
        if links & bit:
            sdf = smooth_min(sdf, box_sdf(*box), FILLET * BLOCK)
    return sdf


def coverage(sdf):
    """antialiased 0..1 cover from a signed distance field"""
    return np.clip(0.5 - sdf, 0.0, 1.0)


def blend(dst, color, alpha):
    """paint `color` over the rgb planes of `dst` with `alpha` cover"""
    for channel in range(3):
        dst[:, :, channel] = dst[:, :, channel] * (1 - alpha) + color[channel] * alpha


def shade(sdf, base, dark, light):
    """a lit body with a rim: a vertical gradient, darkened towards the outline"""
    rgba = np.zeros((SIZE, SIZE, 4), dtype=np.float64)
    inside = coverage(sdf)

    ramp = np.clip((GRID_Y - CENTRE * 0.35) / (SIZE * 0.8), 0.0, 1.0)[:, :, None]
    body = np.array(light) * (1 - ramp) + np.array(base) * ramp
    # ... and a little darker again as it curves away at the edge
    curve = np.clip((sdf + RIM * 2.5) / (RIM * 2.5), 0.0, 1.0)[:, :, None]
    body = body * (1 - 0.28 * curve) + np.array(dark) * (0.28 * curve)

    for channel in range(3):
        rgba[:, :, channel] = body[:, :, channel]
    rim = np.clip((sdf + RIM) / RIM, 0.0, 1.0) * inside
    blend(rgba, dark, rim)
    rgba[:, :, 3] = inside * 255.0
    return rgba


def highlight(rgba, sdf):
    """the specular blob every puyo carries on its upper left"""
    spot = circle_sdf(CENTRE - 0.19 * BLOCK, CENTRE - 0.20 * BLOCK, 0.115 * BLOCK)
    alpha = coverage(spot) * coverage(sdf) * 0.75
    blend(rgba, (255, 255, 255), alpha)


def eyes(rgba, sdf, pupil_scale=1.0, spread=0.165, lift=0.03):
    """two whites with a pupil in each, which is what makes it a puyo and not a bead"""
    for side in (-1, 1):
        ex = CENTRE + side * spread * BLOCK
        ey = CENTRE - lift * BLOCK
        white = np.hypot((GRID_X - ex) / (0.135 * BLOCK), (GRID_Y - ey) / (0.165 * BLOCK)) - 1.0
        # the sdf of a squashed circle is only approximate; scale it back to pixels
        white = white * (0.135 * BLOCK)
        blend(rgba, (255, 255, 255), coverage(white) * coverage(sdf))
        pupil = circle_sdf(ex + side * 0.012 * BLOCK, ey + 0.012 * BLOCK, 0.062 * BLOCK * pupil_scale)
        blend(rgba, (26, 20, 30), coverage(pupil) * coverage(sdf))


def scowl(rgba, sdf):
    """nuisance is not pleased to be here"""
    mouth = box_sdf(
        CENTRE - 0.16 * BLOCK, CENTRE + 0.13 * BLOCK, CENTRE + 0.16 * BLOCK, CENTRE + 0.175 * BLOCK
    )
    blend(rgba, (60, 52, 66), coverage(mouth) * coverage(sdf))


def puyo_sprite(links, palette):
    sdf = puyo_sdf(links)
    rgba = shade(sdf, *palette)
    highlight(rgba, sdf)
    eyes(rgba, sdf)
    return rgba


def nuisance_sprite(scale=1.0):
    sdf = circle_sdf(CENTRE, CENTRE, BODY_RADIUS * BLOCK * scale)
    rgba = shade(sdf, *NUISANCE)
    highlight(rgba, sdf)
    eyes(rgba, sdf, pupil_scale=0.8, spread=0.15 * scale, lift=0.05 * scale)
    scowl(rgba, sdf)
    return rgba


def rock_sprite():
    """thirty puyos at once, which the tray shows as a rock rather than five rows of icons"""
    r = BODY_RADIUS * BLOCK
    facets = [
        circle_sdf(CENTRE - 0.10 * BLOCK, CENTRE - 0.08 * BLOCK, r * 0.62),
        circle_sdf(CENTRE + 0.14 * BLOCK, CENTRE - 0.02 * BLOCK, r * 0.55),
        circle_sdf(CENTRE - 0.02 * BLOCK, CENTRE + 0.15 * BLOCK, r * 0.60),
    ]
    sdf = facets[0]
    for facet in facets[1:]:
        sdf = smooth_min(sdf, facet, 0.08 * BLOCK)
    rgba = shade(sdf, (0x8B, 0x8F, 0x9B), (0x33, 0x37, 0x42), (0xC8, 0xCD, 0xD8))
    highlight(rgba, sdf)
    return rgba


def paste(sheet, rgba, col, row):
    """the cell out of the middle of the overhung canvas, into its slot on the sheet"""
    cell = rgba[OVERHANG : OVERHANG + BLOCK, OVERHANG : OVERHANG + BLOCK]
    image = Image.fromarray(np.clip(cell, 0, 255).astype(np.uint8), "RGBA")
    sheet.paste(image, (PAD + PITCH * col, PAD + PITCH * row))


def main():
    sheet = Image.new("RGBA", (PITCH * COLUMNS, PITCH * ROWS), (0, 0, 0, 0))
    for row, palette in enumerate(COLORS):
        for links in range(16):
            paste(sheet, puyo_sprite(links, palette), links, row)
    paste(sheet, nuisance_sprite(), 0, 5)
    paste(sheet, nuisance_sprite(0.62), 1, 5)
    paste(sheet, nuisance_sprite(), 2, 5)
    paste(sheet, rock_sprite(), 3, 5)
    sheet.save(os.path.normpath(OUT))
    print(f"{os.path.normpath(OUT)} {sheet.size[0]}x{sheet.size[1]}")


if __name__ == "__main__":
    main()
