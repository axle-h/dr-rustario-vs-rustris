#!/usr/bin/env python3
"""Cuts puyo-rusto's retro themes out of the rips in `art/retro/`.

`rip.py` is the particle theme's cutter and this is its sibling: same job, three more
sources. None of them are in the repository - they are spriters-resource downloads sitting
under their verbatim download names in `art/retro/`, gitignored beside the Puyo Puyo Tetris
sheet the particle theme is cut from. Re-run this rather than editing its output.

    python3 puyo-rusto/art/rip_retro.py                # cut every theme
    python3 puyo-rusto/art/rip_retro.py genesis        # ... or just one
    python3 puyo-rusto/art/rip_retro.py check          # write art/retro-alignment.png

The output layout is `rip.py`'s, so a retro theme's `mod.rs` addresses its sheet exactly the
way `theme/modern/mod.rs` addresses that one - one band of six rows, at the source block size
of whichever rip it came from:

    block (col, row) -> (PAD + PITCH * col, PAD + PITCH * row), BLOCK square
    rows 0-4  one colour each (red, green, blue, yellow, purple), column = link mask bits
    row 5     col 0 nuisance, cols 1-3 the tray's small, large and rock symbols

Where this differs from `rip.py` is how the sixteen link variants are *found*. That sheet
laid them out as a grid and the work was repairing necks that stopped short of their cell.
These sheets do not have a grid: PicsAndPixels arranged Mean Bean Machine's beans as
*groups* - a page of two-, three-, four- and five-bean shapes, each drawn as it appears in
play - so the sixteen variants are in there but scattered, and which variant a given bean is
depends on what it is standing next to.

So `link_grid` reads the arrangement rather than a table. Every group is a connected run of
non-background pixels whose bounding box is a whole number of cells; inside that box a cell
is occupied or it is not; and a bean's link mask is which of its four neighbours in that box
are occupied. One pass over the sheet yields every mask of every colour, with no coordinates
written down by hand and nothing to get wrong when the next sheet is laid out differently.
It also self-checks: a sheet that does not yield all sixteen of all five is a sheet this has
misread, and `link_grid` says so rather than writing a theme with holes in it.
"""

import os
import re
import sys
from collections import Counter, deque

import numpy as np
from PIL import Image, ImageFilter

Image.MAX_IMAGE_PIXELS = None

HERE = os.path.dirname(os.path.abspath(__file__))
RETRO = os.path.join(HERE, "retro")
SRC = os.path.normpath(os.path.join(HERE, "..", "src", "theme"))

# the output grid, which is `rip.py`'s: sixteen link variants along a row, a row per colour,
# then one more for the nuisance puyo and the tray's three symbols
PAD = 4
COLUMNS = 16
COLOR_ROWS = 5
SHEET_ROWS = COLOR_ROWS + 1

# this game's colour order, which is the order the rows come out in
COLORS = ["Red", "Green", "Blue", "Yellow", "Purple"]

# LinkMask's bits, from `game/cell.rs`: a puyo joined upwards has bit 1 set
UP, DOWN, LEFT, RIGHT = 1, 2, 4, 8


def source(name):
    path = os.path.join(RETRO, name)
    if not os.path.exists(path):
        raise SystemExit(
            f"{path} is not here.\n"
            "The rips are not carried in the repository - download the sheet from\n"
            "spriters-resource and drop it in art/retro/ under its own name."
        )
    return Image.open(path).convert("RGBA")


def rgb(image):
    return np.array(image.convert("RGB")).astype(int)


def background_of(pixels):
    """the sheet's transparent colour, which is simply the one it has most of.

    Every one of these rips keys transparency to a flat fill and no two of them agree on
    which - Mean Bean Machine's beans are on (0, 128, 128) and its fonts on (0, 108, 108),
    from the same ripper on the same day - so nothing here hard codes one.
    """
    counts = Counter(map(tuple, pixels.reshape(-1, 3)[::3]))
    return list(counts.most_common(1)[0][0])


def matches(pixels, color, tolerance=12):
    return np.abs(pixels - color).sum(2) < tolerance


def components(mask):
    """the 4-connected components of a boolean mask, as (x, y, w, h) boxes.

    Written out rather than imported: this repository's Python is numpy and PIL and nothing
    else, and `scipy.ndimage.label` is the only thing scipy would be here for.
    """
    height, width = mask.shape
    seen = np.zeros((height, width), bool)
    out = []
    for y0, x0 in zip(*np.nonzero(mask)):
        if seen[y0, x0]:
            continue
        queue = deque([(y0, x0)])
        seen[y0, x0] = True
        x_min = x_max = x0
        y_min = y_max = y0
        while queue:
            y, x = queue.popleft()
            x_min, x_max = min(x_min, x), max(x_max, x)
            y_min, y_max = min(y_min, y), max(y_max, y)
            for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                a, b = y + dy, x + dx
                if 0 <= a < height and 0 <= b < width and mask[a, b] and not seen[a, b]:
                    seen[a, b] = True
                    queue.append((a, b))
        out.append((x_min, y_min, x_max - x_min + 1, y_max - y_min + 1))
    return out


def hue_of(pixels, mask):
    """the median hue of a cell's own art, in degrees.

    The outline and the eyes are dropped first: every bean on every one of these sheets is
    drawn with a black rim and two white eyes, and averaging those in walks every colour
    towards the same grey.
    """
    px = pixels[mask]
    if len(px) == 0:
        return None
    high = px.max(1)
    low = px.min(1)
    px = px[(high > 60) & ((high - low) > 40)]
    if len(px) < 10:
        return None
    high = px.max(1).astype(float)
    low = px.min(1).astype(float)
    span = np.maximum(high - low, 1e-6)
    r, g, b = px[:, 0], px[:, 1], px[:, 2]
    hue = np.where(
        high == r,
        (g - b) / span % 6,
        np.where(high == g, (b - r) / span + 2, (r - g) / span + 4),
    )
    return float(np.median(hue * 60 % 360))


def classify(hue, bands):
    if hue is None:
        return None
    for name, low, high in bands:
        if low <= hue <= high:
            return name
    return None


def link_grid(image, block, bands, backdrops=()):
    """every colour's sixteen link variants, read off a sheet of arranged groups.

    `backdrops` are the flat colours the ripper painted behind the beans to show a group's
    extent - they are part of a cell's *occupancy* but not part of its art, so a cell is
    occupied if it holds either, and the art keyed out at the end is the sheet background and
    these together.

    Returns `{(colour, mask): (x, y)}` - the top left of a cell drawing that variant.
    """
    pixels = rgb(image)
    background = background_of(pixels)
    empty = matches(pixels, background)
    backdrop = np.zeros(empty.shape, bool)
    for color in backdrops:
        backdrop |= matches(pixels, color)
    art = ~(empty | backdrop)

    found = {}
    for x, y, width, height in components(~empty):
        if width % block or height % block or width < block or height < block:
            continue
        if not art[y : y + height, x : x + width].any():
            continue
        rows, cols = height // block, width // block
        occupied = np.zeros((rows, cols), bool)
        for r in range(rows):
            for c in range(cols):
                cell = ~empty[y + r * block : y + (r + 1) * block, x + c * block : x + (c + 1) * block]
                occupied[r, c] = cell.mean() > 0.5
        for r in range(rows):
            for c in range(cols):
                if not occupied[r, c]:
                    continue
                mask = 0
                if r > 0 and occupied[r - 1, c]:
                    mask |= UP
                if r < rows - 1 and occupied[r + 1, c]:
                    mask |= DOWN
                if c > 0 and occupied[r, c - 1]:
                    mask |= LEFT
                if c < cols - 1 and occupied[r, c + 1]:
                    mask |= RIGHT
                left, top = x + c * block, y + r * block
                color = classify(
                    hue_of(
                        pixels[top : top + block, left : left + block],
                        art[top : top + block, left : left + block],
                    ),
                    bands,
                )
                # the first exemplar of a variant wins, and the sheets are read top to
                # bottom, so a variant is taken from the smallest group that shows it
                if color is not None and (color, mask) not in found:
                    found[(color, mask)] = (left, top)
    missing = [
        (color, mask)
        for color in COLORS
        for mask in range(COLUMNS)
        if (color, mask) not in found
    ]
    if missing:
        raise SystemExit(f"the sheet is missing {len(missing)} link variants: {missing[:8]}")
    return found


def keyed(image, box, block, transparent):
    """one cell of a sheet, with every listed colour turned transparent"""
    x, y = box
    cell = image.crop((x, y, x + block, y + block)).convert("RGBA")
    px = np.array(cell)
    rgb_only = px[:, :, :3].astype(int)
    for color in transparent:
        px[:, :, 3][np.abs(rgb_only - color).sum(2) < 12] = 0
    return Image.fromarray(px)


def assert_no_marker(tiles):
    """catch a flat fill the ripper painted behind the beans that nothing keyed out.

    Mean Bean Machine's sheet has three of them - a green, a pale green and a pink - and
    nothing on it says so; the third was found by eye, in a cut sheet, after the first two
    had been listed. Puyos of different colours share only their black rim and their white
    eyes, so any *other* colour that is the dominant one in cells of three or more colour
    rows is not bean art: it is a marker, and this says which so it can be listed.
    """
    dominant = {}
    for (row, _), tile in tiles.items():
        if row >= COLOR_ROWS:
            continue
        px = np.array(tile)
        opaque = px[px[:, :, 3] > 0][:, :3]
        if len(opaque) == 0:
            continue
        for color, _ in Counter(map(tuple, opaque)).most_common(3):
            if max(color) - min(color) < 40 and (max(color) > 200 or max(color) < 60):
                continue  # the rim and the eyes, which every colour shares
            dominant.setdefault(color, set()).add(row)
    shared = {c: rows for c, rows in dominant.items() if len(rows) >= 3}
    if shared:
        raise SystemExit(
            "these colours appear in three or more colour rows, so they are the ripper's "
            "markers rather than art - list them as backdrops: "
            + ", ".join(str(list(c)) for c in shared)
        )


# The scene each retro theme stands on: a wash of the backdrop's own colour, lifted in the
# middle and falling away to the corners. It is written small and drawn through
# `SceneType::Cover`, which scales one picture over the whole window with linear filtering -
# so a hundred pixels across is smooth at 4k and costs a couple of kilobytes.
#
# Both of these themes tiled the game's own wall here and neither could any longer. Once the
# panel was cut level with the top of the field, a puyo spawning above it had that same hand
# scattered stone behind it, and neither plane read as being in front of the other. A flat
# colour fixed that and read as flat; this is the same colour with somewhere to fall to.
VIGNETTE = (96, 54)
# how bright the middle is and how much of that is left at the corners, as fractions of the
# backdrop's own colour
VIGNETTE_MIDDLE = 0.75
VIGNETTE_FALL = 0.62
# how sharply it falls: over one, so the middle stays open and the shoulder is short
VIGNETTE_CURVE = 1.3


def vignette(color):
    """the scene behind one theme's panels, in that theme's own colour"""
    width, height = VIGNETTE
    y, x = np.mgrid[0:height, 0:width]
    radius = np.hypot((x - width / 2) / (width / 2), (y - height / 2) / (height / 2))
    radius /= np.hypot(1, 1)
    wash = VIGNETTE_MIDDLE - VIGNETTE_FALL * radius**VIGNETTE_CURVE
    pixels = np.array(color, float) * wash[..., None]
    return Image.fromarray(pixels.clip(0, 255).round().astype(np.uint8))


def write_sheet(path, block, tiles):
    """the six row sheet, `tiles` keyed by (row, column)"""
    assert_no_marker(tiles)
    pitch = block + 2 * PAD
    out = Image.new("RGBA", (pitch * COLUMNS, pitch * SHEET_ROWS), (0, 0, 0, 0))
    for (row, col), tile in tiles.items():
        out.paste(tile, (PAD + pitch * col, PAD + pitch * row))
    os.makedirs(os.path.dirname(path), exist_ok=True)
    out.save(path)
    print(f"  {os.path.relpath(path, SRC)}  {out.width}x{out.height}")


def write_png(path, image):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    image.save(path)
    print(f"  {os.path.relpath(path, SRC)}  {image.width}x{image.height}")


def digits(image, band, first, pitch, width, transparent):
    """ten digits in a row at one pitch, packed edge to edge for `numeric_sprites`.

    That reader divides the sheet's width by ten and takes its whole height, so the output
    has to be exactly ten cells wide with no margin - which is not how any of these fonts is
    laid out on its own sheet.
    """
    top, bottom = band
    height = bottom - top
    out = Image.new("RGBA", (width * 10, height), (0, 0, 0, 0))
    px = rgb(image)
    for i in range(10):
        x = first + pitch * i
        cell = image.crop((x, top, x + width, bottom)).convert("RGBA")
        a = np.array(cell)
        source_rgb = px[top:bottom, x : x + width]
        for color in transparent:
            a[:, :, 3][np.abs(source_rgb - color).sum(2) < 12] = 0
        out.paste(Image.fromarray(a), (width * i, 0))
    return out


# --------------------------------------------------------------------------------------
# Dr. Robotnik's Mean Bean Machine (Sega Genesis, 1993)
# --------------------------------------------------------------------------------------

GENESIS_BEANS = (
    "Sega Genesis - Dr. Robotnik's Mean Bean Machine - Miscellaneous - Beans.png"
)
GENESIS_BOARDS = (
    "Sega Genesis - Dr. Robotnik's Mean Bean Machine"
    " - Miscellaneous - Cutscene Backgrounds and Boards.png"
)
GENESIS_FONTS = (
    "Sega Genesis - Dr. Robotnik's Mean Bean Machine - Miscellaneous - Fonts.png"
)

GENESIS_BLOCK = 16

# the three flat fills PicsAndPixels painted behind the arranged groups: a green, a pale
# green and a pink. All three mean "a bean stands here" and none of them is bean art. The
# pink was found by `assert_no_marker` after the other two had been listed, which is what
# that check is for.
GENESIS_BACKDROPS = ([34, 177, 76], [181, 255, 181], [255, 174, 201])

# where the hues of Mean Bean Machine's five beans fall. Red wraps, so it gets two bands
GENESIS_HUES = [
    ("Red", 330, 360),
    ("Red", 0, 15),
    ("Yellow", 20, 70),
    ("Green", 80, 160),
    ("Blue", 185, 255),
    ("Purple", 265, 320),
]

# The top of the beans sheet is a strip of animation frames per colour on a 19 pixel pitch:
# five bands of them across, then the refugee bean. The first frame of each is the bean
# flashing white just before it pops, so the *second* is the one at rest - which is what an
# unlinked bean draws as, and the one variant the arrangements below cannot supply because
# nothing down there stands on its own.
GENESIS_TOP_ROW = 32
GENESIS_TOP_PITCH = 19
GENESIS_COLOR_BANDS = {
    "Red": 16,
    "Yellow": 137,
    "Green": 260,
    "Purple": 382,
    "Blue": 503,
}
GENESIS_IDLE_FRAME = 1

# the refugee bean, which is this game's nuisance puyo, and the two mono versions beside it.
# Mean Bean Machine has no tray to draw - an attack lands the moment it is sent - so the
# three tray symbols are the refugee at three weights rather than three sprites the game
# ever drew: the little one for a single, the bean itself for six, and the black and white
# one for a rock of thirty.
GENESIS_REFUGEE = (627, 32)
GENESIS_TRAY = [(665, 32), (627, 32), (627, 50)]

# The pop, which Mean Bean Machine plays in four beats: the bean sees it coming, curls into a
# ball, the ball shrinks, and what is left bursts into droplets. Every colour band carries all
# of it. The surprised face is the *last* frame of the strip at the top - the one with both
# eyes wide open - and the three balls sit in a block of their own below the arrangements,
# beside the halo and the wings the same bean wears as an angel. Each is `(dx, y, size)`: an
# offset along the colour's own band, a row on the sheet, and the square to cut.
GENESIS_SURPRISED = (GENESIS_TOP_PITCH * 4, GENESIS_TOP_ROW, GENESIS_BLOCK)
GENESIS_BALL = (19, 157, 16)
GENESIS_SMALL_BALL = (19, 147, 8)
GENESIS_DROPLET = (29, 147, 8)

# how far from the middle of the cell the four droplets have flown, one distance per frame.
# They are cut to their own ink first and placed by their centres, so the burst stays inside
# the cell it came out of and no droplet is drawn half off it.
GENESIS_DROPLET_SPREAD = (3, 5)

# the frames of one pop, which is what `DestroyStyle::Pop` counts. Every strip on the sheet is
# this many so a theme can name one number for all of them
GENESIS_POP_FRAMES = 5

# The refugee bean blinks where the beans wriggle: the sheet carries it shrunken and eyeless
# beside the one with its eyes open, which is the whole of the animation. It is the same art
# the tray draws a single nuisance with - see [`GENESIS_TRAY`] - and it is cut twice rather
# than shared, because the tray wants a still and this wants a frame.
GENESIS_REFUGEE_SMALL = (665, 32)
# ... and the white-outlined refugee, which is what it flashes to on its way out
GENESIS_REFUGEE_FLASH = (646, 50)

# the animation sheet's own grid. The frames of a strip have to be edge to edge - the engine
# addresses one by counting frame widths from its start - so only the rows are spaced out.
GENESIS_ANIM_ROW_GAP = 4

# The dungeon wall the panels stand against, read off the board art. Nothing is tiled with it
# any more - it is the colour [`vignette`] washes the scene in.
GENESIS_WALL = (0x42, 0x45, 0x00)

# the in-game board, which is the third of the four 320x224 boards on the boards sheet -
# measured against a live frame of the emulated game rather than picked by eye
GENESIS_BOARD_TILE = (1, 1180)
GENESIS_SCREEN = (320, 224)

# ... and the same board again, beside it: the *frame*. The sheet carries the screen as the
# two planes the Genesis drew it on, and the one on the left is only the back half - the
# dungeon wall and the wells sunk into it, with no border round anything. The stone that
# frames a well, the floor it stands on and the boxes the next beans and the mugshot sit in
# are all on the front plane, laid out over a flat key. Cutting the back plane alone is what
# left the beans looking like they stopped a row short of the bottom: the well was there,
# the floor under it was not, so the last row of beans had sixteen pixels of open well
# beneath it. Compositing the two is the whole fix, and it is what hands the panel its NEXT
# boxes as well.
GENESIS_FRAME_TILE = (323, 1180)
GENESIS_FRAME_KEY = (0, 64, 64)
# the left well within that board: six columns and twelve rows of 16, at (16, 16)
GENESIS_WELL = (16, 16, 96, 192)

# Mean Bean Machine lays a course of stone over the well mouth and the hole is cut through
# it: the spawning row is open to the scene. It was tried with the row drawn *behind* that
# stone first, and it looked right until a stack reached the top and a bean that mattered
# was simply not there.
# one player's panel is the well and the column of furniture beside it, cut where the second
# player's well begins
GENESIS_PANEL_WIDTH = 208

# Every face on the fonts sheet is laid out the same way: eight pixels wide on a nine pixel
# pitch, thirty glyphs to a row - the ten digits and then A to T. So one reading serves all of
# them and a word is a lookup into the alphabet.
GENESIS_GLYPH = (1, 9, 8)
GENESIS_ALPHABET = "0123456789ABCDEFGHIJKLMNOPQRST"

# The face the game sets `NEXT`, `SCORE` and both players' scores in, four inks and sixteen
# rows. It is *green* on the sheet because green is the palette the labels take; a score is
# the same glyphs in the player's own, which for player one is [`GENESIS_SCORE_INK`]. Matched
# glyph for glyph against the game's own `00007536`, so this is the face and not a guess -
# the big white one at 123 the first pass used is `FINAL STAGE`'s and a different shape.
GENESIS_FACE_BOLD = (174, 190)

# ... and the smaller white one it sets `STAGE`, `1P` and `DR R` in, and the stage number with
# them. Its ink is nine rows inside a sixteen row cell, which is why the word is pasted at 80
# and reads at 84.
GENESIS_FACE_PLAIN = (140, 156)

# Green to red, by role. The sheet and a frame of the game are the same eight levels per
# channel at two different scalings, so the pairing was read off the game a digit at a time
# and written back in the sheet's own scaling.
GENESIS_SCORE_INK = {
    (0, 96, 0): (128, 0, 64),
    (0, 128, 0): (128, 0, 64),
    (64, 160, 0): (160, 32, 64),
    (64, 224, 0): (224, 64, 96),
    (160, 224, 160): (224, 128, 128),
}

# the flat the ripper laid behind each glyph cell, which is not the page colour and is not ink
GENESIS_FONT_CELL = (0, 64, 64)

# The words the frame plane does not carry. Mean Bean Machine prints `NEXT`, `1P`, `DR R`,
# `STAGE` and `SCORE` down the middle of the screen and *every one of them is a text sprite*,
# so the frame has the boxes and no words at all - and a bare number on stone says nothing.
# These two are the ones with a number under them. In **screen** coordinates, like
# [`GENESIS_WELL`], since the panel is cut sixteen rows down from there.
GENESIS_LABELS = (
    ("SCORE", GENESIS_FACE_BOLD, (128, 160)),
    ("STAGE", GENESIS_FACE_PLAIN, (128, 80)),
)

def keyed_box(image, box, transparent):
    """one rectangle of a sheet, with the flat colours in `transparent` knocked out"""
    x, y, w, h = box
    px = np.array(image.crop((x, y, x + w, y + h)).convert("RGBA"))
    flat = px[:, :, :3].astype(int)
    for color in transparent:
        px[:, :, 3][np.abs(flat - color).sum(2) < 12] = 0
    return Image.fromarray(px)


def genesis_glyphs(fonts, band, text, transparent, recolour=None):
    """one word out of one of the fonts sheet's faces, keyed and optionally palette swapped"""
    first, pitch, width = GENESIS_GLYPH
    top, bottom = band
    out = Image.new("RGBA", (width * len(text), bottom - top), (0, 0, 0, 0))
    for i, letter in enumerate(text):
        n = GENESIS_ALPHABET.index(letter)
        cell = keyed_box(
            fonts, (first + pitch * n, top, width, bottom - top), transparent
        )
        out.paste(cell, (width * i, 0))
    if not recolour:
        return out
    px = np.array(out)
    flat = px[:, :, :3].astype(int)
    for source, target in recolour.items():
        hit = np.abs(flat - source).sum(2) < 12
        px[:, :, 0][hit], px[:, :, 1][hit], px[:, :, 2][hit] = target
    # ... and nothing green may survive a swap out of a green face. The sheet has seven
    # shades in it, two of them a couple of dozen pixels across the whole row, and a shade
    # left off the table shows up as a lit edge on three of the ten digits and nowhere else -
    # which is exactly how it was found. Name the shade rather than widening the table.
    left = np.array(Image.fromarray(px).convert("RGBA"))
    lit = (left[:, :, 3] > 0) & (left[:, :, 1].astype(int) > left[:, :, 0].astype(int))
    if lit.any():
        raise SystemExit(
            f"{lit.sum()} pixels are still green after the swap: "
            f"{sorted({tuple(c) for c in left[lit][:, :3]})}"
        )
    return Image.fromarray(px)


def genesis_cell(beans, box, size, transparent):
    """one sprite of the beans sheet, keyed and centred in a whole [`GENESIS_BLOCK`] cell.

    The balls a popping bean shrinks through are drawn smaller than a cell and cut as such,
    so they are put back on the cell they came out of rather than scaled up to fill it.
    """
    tile = keyed(beans, box, size, transparent)
    if size == GENESIS_BLOCK:
        return tile
    cell = Image.new("RGBA", (GENESIS_BLOCK, GENESIS_BLOCK), (0, 0, 0, 0))
    offset = (GENESIS_BLOCK - size) // 2
    cell.paste(tile, (offset, offset))
    return cell


def genesis_burst(beans, band, spread, transparent):
    """the four droplets a bean bursts into, this far out from the middle of its cell"""
    dx, y, size = GENESIS_DROPLET
    droplet = keyed(beans, (band + dx, y), size, transparent)
    droplet = droplet.crop(droplet.getbbox())
    cell = Image.new("RGBA", (GENESIS_BLOCK, GENESIS_BLOCK), (0, 0, 0, 0))
    middle = GENESIS_BLOCK // 2
    for ox in (-spread, spread):
        for oy in (-spread, spread):
            cell.alpha_composite(
                droplet,
                (
                    middle + ox - droplet.width // 2,
                    middle + oy - droplet.height // 2,
                ),
            )
    return cell


def genesis_animations(beans, transparent):
    """the pop of each colour, the refugee bean's pop, and the refugee bean's blink.

    One strip per row, every frame a whole cell, laid out for
    `AnimationSpriteSheetData::non_exclusive_linear` - which counts frame widths from a
    strip's own start, so the frames of one strip are edge to edge and only the rows are
    spaced apart.
    """
    cut = lambda box, size: genesis_cell(beans, box, size, transparent)
    strips = []
    for color in COLORS:
        band = GENESIS_COLOR_BANDS[color]
        sx, sy, ssize = GENESIS_SURPRISED
        bx, by, bsize = GENESIS_BALL
        mx, my, msize = GENESIS_SMALL_BALL
        strips.append(
            [
                cut((band + sx, sy), ssize),
                cut((band + bx, by), bsize),
                cut((band + mx, my), msize),
                genesis_burst(beans, band, GENESIS_DROPLET_SPREAD[0], transparent),
                genesis_burst(beans, band, GENESIS_DROPLET_SPREAD[1], transparent),
            ]
        )
    blank = Image.new("RGBA", (GENESIS_BLOCK, GENESIS_BLOCK), (0, 0, 0, 0))
    # the refugee bean has no ball and no droplets of its own on the sheet - it flashes white
    # and shrinks away, which is the art it does have
    strips.append(
        [
            cut(GENESIS_REFUGEE, GENESIS_BLOCK),
            cut(GENESIS_REFUGEE_FLASH, GENESIS_BLOCK),
            cut(GENESIS_REFUGEE_SMALL, GENESIS_BLOCK),
            blank,
            blank,
        ]
    )
    # ... and its blink, which is the same shrunken bean between two of the still one
    strips.append(
        [
            cut(GENESIS_REFUGEE, GENESIS_BLOCK),
            cut(GENESIS_REFUGEE_SMALL, GENESIS_BLOCK),
            cut(GENESIS_REFUGEE, GENESIS_BLOCK),
        ]
    )
    pitch = GENESIS_BLOCK + GENESIS_ANIM_ROW_GAP
    out = Image.new(
        "RGBA",
        (GENESIS_BLOCK * GENESIS_POP_FRAMES, pitch * len(strips)),
        (0, 0, 0, 0),
    )
    for row, strip in enumerate(strips):
        for frame, cell in enumerate(strip):
            out.paste(cell, (GENESIS_BLOCK * frame, pitch * row))
    return out


def genesis_screen(boards, back_at, front_at):
    """One whole 320x224 board screen, the two planes the sheet keeps apart put together.

    The front plane is keyed on [`GENESIS_FRAME_KEY`] - a flat teal that means "the back
    plane shows here" - and covers the rest: every stone border, the well floors and the
    boxes down the middle. Composited against a live frame of the emulated game the result
    agrees to within the rip's own colour rounding; the back plane alone does not, and the
    sixteen rows under each well are where it differs.
    """
    bx, by = back_at
    fx, fy = front_at
    w, h = GENESIS_SCREEN
    back = np.array(boards.crop((bx, by, bx + w, by + h)).convert("RGBA"))
    front = np.array(boards.crop((fx, fy, fx + w, fy + h)).convert("RGBA"))
    drawn = np.abs(front[:, :, :3].astype(int) - GENESIS_FRAME_KEY).sum(2) > 12
    back[drawn] = front[drawn]
    return Image.fromarray(back)


def genesis():
    print("genesis (Dr. Robotnik's Mean Bean Machine)")
    out = os.path.join(SRC, "genesis")
    beans = source(GENESIS_BEANS)
    pixels = rgb(beans)
    background = background_of(pixels)
    transparent = [background, *GENESIS_BACKDROPS]

    grid = link_grid(beans, GENESIS_BLOCK, GENESIS_HUES, GENESIS_BACKDROPS)
    tiles = {}
    for row, color in enumerate(COLORS):
        for mask in range(COLUMNS):
            tiles[(row, mask)] = keyed(
                beans, grid[(color, mask)], GENESIS_BLOCK, transparent
            )
        # ... except the unlinked one, which comes off the animation strip at the top
        tiles[(row, 0)] = keyed(
            beans,
            (
                GENESIS_COLOR_BANDS[color] + GENESIS_TOP_PITCH * GENESIS_IDLE_FRAME,
                GENESIS_TOP_ROW,
            ),
            GENESIS_BLOCK,
            transparent,
        )
    tiles[(COLOR_ROWS, 0)] = keyed(beans, GENESIS_REFUGEE, GENESIS_BLOCK, transparent)
    for i, at in enumerate(GENESIS_TRAY):
        tiles[(COLOR_ROWS, 1 + i)] = keyed(beans, at, GENESIS_BLOCK, transparent)
    write_sheet(os.path.join(out, "sprites.png"), GENESIS_BLOCK, tiles)
    write_png(
        os.path.join(out, "animations.png"), genesis_animations(beans, transparent)
    )

    boards = source(GENESIS_BOARDS)
    bx, by = GENESIS_BOARD_TILE
    fx, fy = GENESIS_FRAME_TILE
    wx, wy, ww, wh = GENESIS_WELL
    screen = genesis_screen(boards, (bx, by), (fx, fy))
    # The panel is cut at the *well's* top edge and not the screen's, so it stops level with
    # the top of the field: the spawning row is drawn a cell above it, over the scene, with
    # nothing to either side of it - which is where the game's course of stone over the well
    # mouth goes, and is what a retro Rustris board does at its skyline. The theme puts the
    # cell back as `top_padding`, above the panel and the board alike, so **a point in the
    # padded background is a point on the Genesis screen**.
    panel = screen.crop((0, wy, GENESIS_PANEL_WIDTH, GENESIS_SCREEN[1]))
    # ... with a hole where the well is. The board is drawn *under* the background and the
    # cells with it, so a panel that carries its own well would cover the whole game - which
    # is exactly what it did the first time. Both of the other games' retro themes cut the
    # same hole; nothing says so anywhere but the art.
    hole = np.array(panel)
    hole[0:wh, wx : wx + ww, 3] = 0
    panel = Image.fromarray(hole)
    # ... and the one word of the middle column's furniture this rip can put back, added
    # after the fonts are read, further down
    write_png(
        os.path.join(out, "board.png"), screen.crop((wx, wy, wx + ww, wy + wh))
    )

    write_png(os.path.join(out, "scene.png"), vignette(GENESIS_WALL))

    fonts = source(GENESIS_FONTS)
    keys = [background_of(rgb(fonts)), GENESIS_FONT_CELL]
    for word, band, (lx, ly) in GENESIS_LABELS:
        panel.alpha_composite(genesis_glyphs(fonts, band, word, keys), (lx, ly - wy))
    write_png(os.path.join(out, "background.png"), panel)
    # the score is the bold face in the first player's red; the stage number is the plain one
    write_png(
        os.path.join(out, "font.png"),
        genesis_glyphs(fonts, GENESIS_FACE_BOLD, "0123456789", keys, GENESIS_SCORE_INK),
    )
    write_png(
        os.path.join(out, "font-small.png"),
        genesis_glyphs(fonts, GENESIS_FACE_PLAIN, "0123456789", keys),
    )


# --------------------------------------------------------------------------------------
# Kirby's Avalanche (SNES, 1995)
# --------------------------------------------------------------------------------------

SNES_BLOBS = "SNES - Kirby's Avalanche - Miscellaneous - Blobs & Boulders.png"

SNES_BLOCK = 16

# the pale fill the ripper laid behind each block of sprites; the sheet background is the
# magenta `background_of` finds on its own
SNES_BACKDROPS = ([248, 128, 248],)

# Where each colour's "Placed Blob" grid starts. Unlike Mean Bean Machine's sheet this one is
# a *grid*: sixteen link variants in two rows of eight, in the game's own order, with a third
# row of loose frames under them that this ignores.
SNES_GRIDS = {
    "Blue": (8, 80),
    "Red": (192, 104),
    "Yellow": (192, 160),
    "Green": (8, 304),
    "Purple": (8, 360),
}

# The sheet's own link order, which is not this game's.
#
# Read off the grid rather than assumed: a neck runs to its cell edge (that is what makes two
# joined blobs meet flush), so which sides a variant is joined on is which edges its art
# touches in the middle - and the purple grid, whose blobs carry no shadow to confuse it,
# decodes to exactly this. Bit 1 of the sheet's index is up, 2 is right, 4 is down, 8 is left.
SNES_INDEX_BITS = ((1, UP), (2, RIGHT), (4, DOWN), (8, LEFT))
SNES_GRID_COLUMNS = 8

# the boulder, which is this game's nuisance puyo, and its dissolving frames. Kirby's
# Avalanche has no tray either, so the three symbols are the boulder at three weights: the
# most dissolved for a single, the next for six, and the whole rock for thirty.
SNES_BOULDER = (8, 152)



# --- and the rest of it, which no rip carries -------------------------------------------
#
# Kirby's Avalanche has one sheet of playfield art on spriters-resource and it is the blobs:
# no board, no background, no font. So those three come out of the game itself, and *not* by
# screenshotting the board - a frame has the blobs, Kirby, the opponent's portrait and both
# players' HUDs drawn into it.
#
# What they come out of instead is the SNES's own layer switch. `$212C` is the main screen
# designation - a bit per background layer and one for the sprites - and snes9x keeps it in
# `Memory.FillRAM`, which is a block of any savestate it writes. So: take a state mid-match,
# poke that one byte, load it back, and the emulator renders whichever layers you asked for
# and nothing else. Kirby's Avalanche plays in mode 2 with `$212C = 0x13`, which is BG1, BG2
# and the sprites; the two renders below are `0x03` (both backgrounds, no sprites) and `0x02`
# (BG2 alone). What that separates:
#
#   BG2   the forest, the grass border and the score line - the scenery
#   BG1   the wooden centre column, the flower border, and the blobs in play
#
# and the field itself is on neither: it is the backdrop colour, so the forest simply shows
# through it. That is why `board.png` is a crop of the BG2 render and not a frame of its own.
#
# Reproducing the two PNGs, if they are ever lost (`ra.py` is the retroarch skill's):
#
#   ra.py start --core snes9x --rom "Kirby's Avalanche (USA).sfc" \
#       --set video_vsync=false --set savestate_file_compression=false
#   ... drive it to a match, then ra.py state save 2
#   poke byte 0x212c of the state's FIL block to 0x03 (and again to 0x02), ra.py state load 2,
#   ra.py screenshot
#
# `video_vsync=false` is not optional: with it on RetroArch answers the command port for a few
# seconds and then stops replying to anything at all, while the process stays alive.
# the canopy the panels stand against, read off the forest layer - the colour [`vignette`]
# washes this theme's scene in
SNES_FOREST = (0x08, 0x28, 0x10)

SNES_LAYERS_BOTH = "kirby-layer-03.png"
SNES_LAYER_SCENERY = "kirby-layer-02.png"
SNES_STATE = "kirby-avalanche.state"

# the SNES screen, and the left player's field on it: six columns and twelve rows of 16.
# Measured off the game rather than the BG1 render, which is a pixel out: a blob's eyes sit
# three rows into its cell, and in a frame with the field full they land on 99, 115, 131 ...
# 195, so the bottom row is 192 and the top of the field is 208 - 192 = 16.
SNES_SCREEN = (256, 224)
SNES_FIELD = (8, 16, 96, 192)
# one player's panel is the field and the wooden column beside it, cut where the second
# player's field begins
SNES_PANEL_WIDTH = 152

# The game draws its own score along the bottom border, and this game draws its own over the
# top - so the number is painted out with a clean run of the grass beside it and only the
# `SC` label is kept. `PATCH` is what gets covered, `GRASS` is what covers it, and the last
# course of it is *clipped* to the patch: the grass is wider than what is left to cover, and
# laying a whole one down ran sixty pixels past the patch and over the wooden platform at the
# foot of the centre column - the one Kirby stands on - leaving a band of grass where the
# floor of the arch should be.
# The patch runs to the foot of the centre column, because the game right aligns its number
# there and two pixels of the last digit survived a patch that stopped short of it; the grass
# is taken from clear of the `SC`, because a course cut through the label repeats a sliver of
# it across the border.
SNES_SCORE_PATCH = (34, 200, 70, 24)
SNES_GRASS = (26, 200, 64, 24)

# The game's two name plates, at (104, 31), and the three wooden posts that used to be run up
# over them, are both gone from this script: the `NEXT` sign coming down nine rows (see
# [`SNES_NEXT_SIGN`]) lands its plank exactly over that run and covers it outright. The plates
# named one queue per player and this panel has both boxes to itself, so they had to go
# whichever way covered them.
#
# `FLOOR` lays a whole course of plank across the mouth of the arch, where the game stands
# Kirby and this one stands nothing: the ground under him is bare dark and reads as a gap in
# the column. It copies from the panel itself - the course between `STAGE` and the arch - so
# the wood is the game's own and no two courses are alike by accident.
SNES_FLOOR = (104, 192, 48, 16)
SNES_FLOOR_DONOR = (104, 120, 48, 16)

# Two blobs sit *outside* the field, in the little arch at the foot of the centre column where
# Kirby stands - so they survive punching the field out and have to be painted over. They are
# found rather than measured: a blob comes to rest on the arch's floor, which is black, so the
# patch is the bounding box of whatever is saturated in that band, filled with the colour the
# ring around it is mostly made of. The band is only the floor - the arch's own sky is blue and
# saturated too, and searching the whole arch paints that out with it.
SNES_ARCH = (104, 194, 48, 14)

# Kirby's `NEXT` sign is nailed *above* the top edge of the field, and the panel is cut at that
# edge so that the spawning row has nothing behind it - which cut the sign in half. So the sign
# comes down.
#
# What moves is the whole assembly and not just the letters: the sign is letters *on* a plank,
# 7 to 31 on the screen, and moving any less of it than that leaves a plank sawn through
# halfway - which is what the first attempt did. Nine rows down puts the letters' own top edge
# level with the top of the field they label, and the plank lands where the game's two name
# plates were. Those are painted out on this panel anyway (they name one queue per player and
# this panel has both boxes to itself), so the plank covers the hole they leave and the boxes
# hang off it exactly as they hung off it before.
SNES_NEXT_SIGN = (104, 7, 48, 24)

# The recess the game prints its stage number in. This game shows no level at all, so a number
# sitting there would be wrong on every stage but the first. The recess is a flat sixteen
# square of one colour with the digit drawn on it, so it is *filled* rather than patched -
# `snes_paint_out` would take the black around the box for the colour to cover the digit with
# and leave a black hole under `STAGE`, which is exactly what it did.
SNES_STAGE_NUMBER = (120, 103, 16, 16)

# The two boxes under `NEXT`, one per name plate, and the arch at the foot of the column.
# Nothing is cut from these - they are here because they are what `snes/mod.rs` addresses,
# and measuring them once here keeps the theme's numbers and the art they were read off in
# the same file. The plates are the two runs of white at row 31; the arch is the run of sky
# blue below the last of the slats.
SNES_NEXT_BOXES = ((104, 38, 24, 41), (128, 38, 24, 41))
SNES_ARCH_MOUTH = (104, 186, 48, 14)

# the ten digits in the game's own face, as tiles of its VRAM. Three indices: nothing, the
# dark outline and the white fill, which is what they are drawn as on screen.
# The game's own numeric face: **sixteen rows and not eight**. Each digit is two tiles, the
# top at `SNES_FONT_TILE + n` and the bottom `SNES_FONT_ROW` further on, because the font is
# laid out sixteen glyphs to a VRAM row. There is an eight row face at tile 896 as well - the
# small one the game sets its menus in - and cutting the score in it drew numbers a little
# over half the height of the `SC` they sit beside, which is what gave this away.
#
# Which tiles those are was not guessed. The game prints one digit on the layer render (the
# `0` of its own score) and one in the `STAGE` recess, so both were masked off the render and
# matched against a decode of all 2048 tiles: the score's `0` is 769 over 785 exactly, the
# stage's `1` is 770 over 786, and no other reading of the sheet produces a pair that agrees
# to the pixel.
SNES_FONT_TILE = 769
SNES_FONT_ROW = 16

# ... and the four inks it uses, paired index to colour off those same two digits. The
# palette is the *player's*: the left panel draws its numbers in the red its `SC` is drawn in
# and the right one in white. This panel is the left one.
SNES_FONT_INK = {
    1: (0x00, 0x00, 0x00, 0xff),
    5: (0xE7, 0x51, 0x63, 0xff),
    6: (0xEF, 0x86, 0x84, 0xff),
    15: (0xFF, 0xFF, 0xFF, 0xff),
}


def snes_vram(path):
    """the VRAM block of an uncompressed RetroArch savestate"""
    data = open(path, "rb").read()
    match = re.search(rb"VRA:([0-9]{6}):", data)
    if not match:
        raise SystemExit(f"{path} has no VRA block - is it an uncompressed state?")
    size = int(match.group(1))
    return data[match.end() : match.end() + size]


def snes_font(vram):
    """ten digits, each two 8x8 tiles stacked, decoded from 4bpp planar and inked"""
    out = Image.new("RGBA", (8 * 10, 16), (0, 0, 0, 0))
    px = out.load()
    for digit in range(10):
        for half in range(2):
            base = (SNES_FONT_TILE + digit + half * SNES_FONT_ROW) * 32
            for y in range(8):
                planes = (
                    vram[base + y * 2],
                    vram[base + y * 2 + 1],
                    vram[base + 16 + y * 2],
                    vram[base + 16 + y * 2 + 1],
                )
                for x in range(8):
                    bit = 7 - x
                    index = sum(((p >> bit) & 1) << i for i, p in enumerate(planes))
                    if index in SNES_FONT_INK:
                        px[digit * 8 + x, half * 8 + y] = SNES_FONT_INK[index]
    return out


def snes_paint_out(panel, region):
    """paint over whatever is saturated in `region`, with the colour around it.

    Used twice: for the blobs standing in the arch at the foot of the centre column, which
    survive punching the field out because they are not in the field; and for the stage number.
    """
    ax, ay, aw, ah = region
    px = np.array(panel)
    region = px[ay : ay + ah, ax : ax + aw, :3].astype(int)
    high = region.max(2)
    saturated = (high - region.min(2)) > 60
    if not saturated.any():
        return
    ys, xs = np.nonzero(saturated)
    top, bottom = ys.min(), ys.max() + 1
    left, right = xs.min(), xs.max() + 1
    ring = []
    for y in range(max(top - 2, 0), min(bottom + 2, ah)):
        for x in range(max(left - 2, 0), min(right + 2, aw)):
            if not (top <= y < bottom and left <= x < right):
                ring.append(tuple(region[y, x]))
    fill = Counter(ring).most_common(1)[0][0] if ring else (0, 0, 0)
    px[ay + top : ay + bottom, ax + left : ax + right, :3] = fill
    px[ay + top : ay + bottom, ax + left : ax + right, 3] = 255
    panel.paste(Image.fromarray(px), (0, 0))


def snes_fill_flat(panel, region):
    """flood one flat region of the panel with the colour it is mostly made of.

    For a recess the game draws a number into: the box is one colour and the digit is a
    handful of pixels on it, so the colour to cover the digit with is the region's own
    commonest, and there is no bounding box to find.
    """
    x, y, w, h = region
    px = np.array(panel)
    fill = Counter(map(tuple, px[y : y + h, x : x + w, :3].reshape(-1, 3))).most_common(1)
    px[y : y + h, x : x + w, :3] = fill[0][0]
    px[y : y + h, x : x + w, 3] = 255
    panel.paste(Image.fromarray(px), (0, 0))


def snes_art(out):
    both = source(SNES_LAYERS_BOTH)
    scenery = source(SNES_LAYER_SCENERY)
    gx, gy, gw, gh = SNES_FIELD

    # painted on the whole screen, so every rect below is the game's own coordinate, and cut
    # to the panel at the end
    panel = both.crop((0, 0, SNES_PANEL_WIDTH, SNES_SCREEN[1])).convert("RGBA")
    grass = panel.crop(
        (
            SNES_GRASS[0],
            SNES_GRASS[1],
            SNES_GRASS[0] + SNES_GRASS[2],
            SNES_GRASS[1] + SNES_GRASS[3],
        )
    )
    px, py, pw, ph = SNES_SCORE_PATCH
    for x in range(px, px + pw, grass.width):
        course = min(grass.width, px + pw - x)
        panel.paste(grass.crop((0, 0, course, grass.height)), (x, py))
    snes_paint_out(panel, SNES_ARCH)
    snes_fill_flat(panel, SNES_STAGE_NUMBER)
    dx, dy, dw, dh = SNES_FLOOR_DONOR
    panel.paste(panel.crop((dx, dy, dx + dw, dy + dh)), (SNES_FLOOR[0], SNES_FLOOR[1]))
    # ... the `NEXT` sign brought down onto its own plank, so the cut below does not halve it
    sx, sy, sw, sh = SNES_NEXT_SIGN
    panel.paste(panel.crop((sx, sy, sx + sw, sy + sh)), (sx, gy))

    # ... and the hole the board draws through, which is the field
    holed = np.array(panel)
    holed[gy : gy + gh, gx : gx + gw, 3] = 0
    # ... and then the panel is cut at the *field's* top edge, so it stops level with it and
    # the spawning row is drawn a cell above the panel, over the scene, with nothing to either
    # side. The hedge the game lays across the top of the screen goes with that cut. The theme
    # puts the cell back as `top_padding`, above the panel and the board alike, so a point in
    # the padded background is a point on the SNES screen.
    write_png(
        os.path.join(out, "background.png"),
        Image.fromarray(holed).crop((0, gy, SNES_PANEL_WIDTH, SNES_SCREEN[1])),
    )

    # the field is the backdrop showing through, so its art is the scenery behind it: the
    # game's own twelve rows, stopping where the game stopped them
    write_png(
        os.path.join(out, "board.png"),
        scenery.crop((gx, gy, gx + gw, gy + gh)).convert("RGBA"),
    )
    write_png(
        os.path.join(out, "font.png"),
        snes_font(snes_vram(os.path.join(RETRO, SNES_STATE))),
    )
    write_png(os.path.join(out, "scene.png"), vignette(SNES_FOREST))


def snes_index(mask):
    """the sheet's column for one of this game's link masks"""
    return sum(bit for bit, link in SNES_INDEX_BITS if mask & link)


def snes():
    print("snes (Kirby's Avalanche)")
    out = os.path.join(SRC, "snes")
    blobs = source(SNES_BLOBS)
    transparent = [background_of(rgb(blobs)), *SNES_BACKDROPS]

    tiles = {}
    for row, color in enumerate(COLORS):
        gx, gy = SNES_GRIDS[color]
        for mask in range(COLUMNS):
            index = snes_index(mask)
            x = gx + SNES_BLOCK * (index % SNES_GRID_COLUMNS)
            y = gy + SNES_BLOCK * (index // SNES_GRID_COLUMNS)
            tiles[(row, mask)] = keyed(blobs, (x, y), SNES_BLOCK, transparent)
    bx, by = SNES_BOULDER
    tiles[(COLOR_ROWS, 0)] = keyed(blobs, (bx, by), SNES_BLOCK, transparent)
    for i, frame in enumerate([2, 1, 0]):
        tiles[(COLOR_ROWS, 1 + i)] = keyed(
            blobs, (bx + SNES_BLOCK * frame, by), SNES_BLOCK, transparent
        )
    write_sheet(os.path.join(out, "sprites.png"), SNES_BLOCK, tiles)
    snes_art(out)


# --------------------------------------------------------------------------------------

THEMES = {"genesis": genesis, "snes": snes}

def check():
    """draw every theme's cells as a board that uses all sixteen masks.

    A seam is a hairline and the only way to see one is to put two puyos side by side - so
    this is `rip.py check`'s twin, and it is the thing to look at after touching any of the
    cutting above.
    """
    board = [
        "1111.2",
        "1..1.2",
        "1.333.",
        "..3.3.",
        "44.3.5",
        "4.555.",
    ]
    scale = 3
    panels = []
    for name in THEMES:
        path = os.path.join(SRC, name, "sprites.png")
        if not os.path.exists(path):
            continue
        sheet = Image.open(path).convert("RGBA")
        block = sheet.width // COLUMNS - 2 * PAD
        pitch = block + 2 * PAD
        rows, cols = len(board), len(board[0])
        panel = Image.new("RGBA", (cols * block, rows * block), (0, 0, 0, 255))
        for r, line in enumerate(board):
            for c, ch in enumerate(line):
                if ch == ".":
                    continue
                color = int(ch) - 1
                mask = 0
                for dr, dc, bit in ((-1, 0, UP), (1, 0, DOWN), (0, -1, LEFT), (0, 1, RIGHT)):
                    a, b = r + dr, c + dc
                    if 0 <= a < rows and 0 <= b < cols and board[a][b] == ch:
                        mask |= bit
                cell = sheet.crop(
                    (
                        PAD + pitch * mask,
                        PAD + pitch * color,
                        PAD + pitch * mask + block,
                        PAD + pitch * color + block,
                    )
                )
                panel.paste(cell, (c * block, r * block), cell)
        panels.append(panel.resize((panel.width * scale, panel.height * scale), Image.NEAREST))
    if not panels:
        raise SystemExit("nothing cut yet")
    width = sum(p.width for p in panels) + 8 * (len(panels) - 1)
    out = Image.new("RGBA", (width, max(p.height for p in panels)), (0, 0, 0, 255))
    x = 0
    for p in panels:
        out.paste(p, (x, 0))
        x += p.width + 8
    path = os.path.join(HERE, "retro-alignment.png")
    out.save(path)
    print(f"wrote {path}")


def main():
    args = sys.argv[1:]
    if args == ["check"]:
        check()
        return
    names = args or list(THEMES)
    for name in names:
        if name not in THEMES:
            raise SystemExit(f"unknown theme {name}; try {', '.join(THEMES)} or check")
        THEMES[name]()


if __name__ == "__main__":
    main()
