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
# one player's panel is the well and the column of furniture beside it, cut where the second
# player's well begins
GENESIS_PANEL_WIDTH = 208

# the big white face the game sets FINAL STAGE in: ten digits, 8x16, on a nine pixel pitch
GENESIS_DIGITS = (123, 139, 1, 9, 8)

# The scene behind the boards is the dungeon wall, tiled. There is no seamless tile in the
# board art - the stone is hand scattered and repeats on nothing - so this is a course of it
# taken out of the left border, which is the one strip that is stone and nothing else. It
# tiles with a visible horizontal band and that is fine: courses of stone are what a wall has.
GENESIS_TILE = (0, 64, 16, 32)


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

    boards = source(GENESIS_BOARDS)
    bx, by = GENESIS_BOARD_TILE
    fx, fy = GENESIS_FRAME_TILE
    wx, wy, ww, wh = GENESIS_WELL
    screen = genesis_screen(boards, (bx, by), (fx, fy))
    # the panel is cut at the well's top edge, not the screen's: the thirteenth row floats
    # above the frame in `top_padding`, and the theme's own transparent rows are cheaper and
    # exact where a strip of the game's stone border would only be nearly right
    panel = screen.crop((0, wy, GENESIS_PANEL_WIDTH, GENESIS_SCREEN[1]))
    # ... with a hole where the well is. The board frame is drawn *under* the background and
    # the cells with it, so a panel that carries its own well would cover the whole game -
    # which is exactly what it did the first time. Both of the other games' retro themes cut
    # the same hole; nothing says so anywhere but the art.
    hole = np.array(panel)
    hole[0:wh, wx : wx + ww, 3] = 0
    panel = Image.fromarray(hole)
    write_png(os.path.join(out, "background.png"), panel)
    well = screen.crop((wx, wy, wx + ww, wy + wh))
    write_png(os.path.join(out, "board.png"), well)

    tx, ty, tw, th = GENESIS_TILE
    tile = screen.crop((tx, ty, tx + tw, ty + th))
    write_png(os.path.join(out, "background-tile.png"), tile)

    fonts = source(GENESIS_FONTS)
    top, bottom, first, pitch, width = GENESIS_DIGITS
    font_bg = background_of(rgb(fonts))
    write_png(
        os.path.join(out, "font.png"),
        digits(fonts, (top, bottom), first, pitch, width, [font_bg, [0, 0, 0]]),
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
# `SC` label is kept. `PATCH` is what gets covered, `GRASS` is what covers it.
SNES_SCORE_PATCH = (34, 200, 68, 24)
SNES_GRASS = (22, 200, 64, 24)

# Two blobs sit *outside* the field, in the little arch at the foot of the centre column where
# Kirby stands - so they survive punching the field out and have to be painted over. They are
# found rather than measured: a blob comes to rest on the arch's floor, which is black, so the
# patch is the bounding box of whatever is saturated in that band, filled with the colour the
# ring around it is mostly made of. The band is only the floor - the arch's own sky is blue and
# saturated too, and searching the whole arch paints that out with it.
SNES_ARCH = (104, 194, 48, 14)

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

# the canopy, tiled behind the boards. The forest layer is the only one with a stretch of it
# that is all leaves.
SNES_TILE = (16, 32, 32, 32)

# the ten digits in the game's own face, as tiles of its VRAM. Three indices: nothing, the
# dark outline and the white fill, which is what they are drawn as on screen.
SNES_FONT_TILE = 896
SNES_FONT_INK = {1: (0x20, 0x18, 0x30, 0xff), 15: (0xff, 0xff, 0xff, 0xff)}


def snes_vram(path):
    """the VRAM block of an uncompressed RetroArch savestate"""
    data = open(path, "rb").read()
    match = re.search(rb"VRA:([0-9]{6}):", data)
    if not match:
        raise SystemExit(f"{path} has no VRA block - is it an uncompressed state?")
    size = int(match.group(1))
    return data[match.end() : match.end() + size]


def snes_font(vram):
    """ten 8x8 tiles, decoded from 4bpp planar and inked"""
    out = Image.new("RGBA", (8 * 10, 8), (0, 0, 0, 0))
    px = out.load()
    for digit in range(10):
        base = (SNES_FONT_TILE + digit) * 32
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
                    px[digit * 8 + x, y] = SNES_FONT_INK[index]
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
    fx, fy, fw, fh = SNES_FIELD

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
        panel.paste(grass, (x, py))
    snes_paint_out(panel, SNES_ARCH)
    snes_fill_flat(panel, SNES_STAGE_NUMBER)
    # ... and the hole the board draws through, which is the field
    holed = np.array(panel)
    holed[fy : fy + fh, fx : fx + fw, 3] = 0
    write_png(os.path.join(out, "background.png"), Image.fromarray(holed))

    # the field is the backdrop showing through, so its art is the scenery behind it
    write_png(
        os.path.join(out, "board.png"),
        scenery.crop((fx, fy, fx + fw, fy + fh)).convert("RGBA"),
    )
    tx, ty, tw, th = SNES_TILE
    write_png(
        os.path.join(out, "background-tile.png"),
        scenery.crop((tx, ty, tx + tw, ty + th)).convert("RGBA"),
    )
    write_png(
        os.path.join(out, "font.png"),
        snes_font(snes_vram(os.path.join(RETRO, SNES_STATE))),
    )


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
# Puyo Puyo Chronicle (3DS, 2016)
# --------------------------------------------------------------------------------------

THREE_DS_SKIN = "3DS - Puyo Puyo Chronicle (JPN) - Miscellaneous - Puyo Skin (Gummy).png"

# The sheet is a contact sheet rather than a spritesheet: every sprite sits on its own black
# tile with a one pixel line of the page between tiles, and the tiles are *not square* - 18
# across and 17 down. So the cut is 18 by 17 and the bottom row of art is repeated once to
# fill an 18 by 18 cell, which is what closes a downward neck against the puyo below. It is
# the same argument as `rip.py`'s `repair`, at one pixel instead of eight.
THREE_DS_CELL = (18, 17)
THREE_DS_PITCH = (19, 18)
THREE_DS_BLOCK = 18

# the page, and the black each sprite is drawn on
THREE_DS_BACKDROPS = ([0, 0, 0],)

# The sheet's own link order, read off the grid the same way Kirby's was and confirmed by all
# five colours decoding to the identical permutation - which is what says it was read and not
# guessed. Index is `4 * row + column` of a colour's four by four block.
THREE_DS_ORDER = [2, 8, 12, 4, 3, 10, 14, 6, 1, 11, 15, 7, 0, 9, 13, 5]

# one colour's block is four rows; the five run down the left of the sheet in this game's own
# colour order
THREE_DS_BLOCK_ROWS = 4
THREE_DS_GRID_COLUMNS = 4

# the page the sheet is laid out on, which is what shows between one tile and the next
THREE_DS_PAGE = (231, 231, 202)

# The nuisance puyo, and the tray's three: the small grey one for a single, the white one for
# six and the iron block for a rock of thirty. These are **ordinals along the top row**, not
# columns - see [`three_ds_row_tiles`], which is what finds them.
THREE_DS_NUISANCE = 13
THREE_DS_TRAY = [17, 15, 18]


def three_ds_row_tiles(image):
    """the left edge of every tile along the sheet's top row, in order.

    The sheet is not all on one pitch. The puyo blocks down the left are on nineteen and so
    is most of the top row, but a gap partway along it shifts everything after by a pixel and
    then by three - so cutting the nuisance and the tray by column number takes three pixels
    of the tile next door and loses three of their own, which is exactly what the tray looked
    like. The tiles are found instead, by the page showing between them.
    """
    band = np.array(image.convert("RGB"))[: THREE_DS_CELL[1]].astype(int)
    page = (np.abs(band - THREE_DS_PAGE).sum(2) < 12).all(axis=0)
    tiles, x = [], 0
    while x < len(page):
        if page[x]:
            x += 1
            continue
        tiles.append(x)
        while x < len(page) and not page[x]:
            x += 1
    return tiles


def three_ds_cut(image, col, row, transparent):
    """one cell, 18 by 17, squared off to 18 by 18 by repeating its last row of art"""
    return three_ds_cut_at(
        image, THREE_DS_PITCH[0] * col, THREE_DS_PITCH[1] * row, transparent
    )


def three_ds_cut_at(image, x, y, transparent):
    """... from an explicit corner, for the tiles that are not on the pitch"""
    w, h = THREE_DS_CELL
    cell = image.crop((x, y, x + w, y + h)).convert("RGBA")
    px = np.array(cell)
    flat = px[:, :, :3].astype(int)
    for color in transparent:
        px[:, :, 3][np.abs(flat - color).sum(2) < 12] = 0
    squared = np.zeros((THREE_DS_BLOCK, THREE_DS_BLOCK, 4), np.uint8)
    squared[:h] = px
    squared[h:] = px[h - 1]
    return Image.fromarray(squared)


THREE_DS_OBJECTS = "3DS - Puyo Puyo Chronicle (JPN) - Miscellaneous - Field Objects.png"
THREE_DS_BACKGROUNDS = "3DS - Puyo Puyo Chronicle (JPN) - Miscellaneous - In-Game Backgrounds.png"

# The four field frames, in the order the sheet draws them. They are `board_snips`, one per
# speed band, so the field changes colour as the pairs come down faster - which is a thing
# the rip offers and the game itself does not, and is the only liberty this theme takes.
THREE_DS_FRAMES = [(10, 11), (134, 11), (259, 11), (385, 11)]
THREE_DS_FRAME_SIZE = (112, 192)
# the frame's border, and so where its interior starts
THREE_DS_FRAME_INSET = 6
# the field's own fill, a dark blue print of puyos
THREE_DS_FILL = (593, 379, 100, 180)

# The frame is drawn at the rip's own scale and the puyos at theirs, and the two do not agree:
# the frame's interior is 100 by 180 where twelve rows of an eighteen pixel puyo is 216. So the
# frame is scaled up until its interior is exactly twelve rows tall - uniformly, because a
# rounded corner stretched on one axis reads as a mistake - and the six columns sit centred in
# the width that leaves.
THREE_DS_ROWS, THREE_DS_COLUMNS = 12, 6

# the in-game backgrounds, twenty two of them on a four by six grid of 3DS top screens
THREE_DS_BG_SIZE = (400, 240)
THREE_DS_BG_PICK = (0, 0)

# What the background is written at. Chronicle paints one scene behind both fields and this
# theme does the same - `SceneType::Cover` scales it over the whole window - so a 3DS top
# screen has to hold up at 1080p and beyond, which is around five times the size it was
# drawn. It gets there in two steps: three times here, with Lanczos and a light unsharp, and
# the rest at draw time with the linear filter `Cover` turns on. Three rather than five
# because the last stretch is the cheap one to give away: the art is painted, all soft
# gradients and no fine detail, so the interpolation has nothing to lose - and 1200x720 is
# 685 KiB in the binary and 3.4 MiB of texture against 1.5 MiB and 8.6 MiB at full size.
THREE_DS_BG_SCALE = 3
THREE_DS_BG_SHARPEN = (2, 60, 2)

# One player's panel: the framed field, a column beside it for the queue and the nuisance
# tray, and a strip under it for the score - which goes there because seven digits of the
# game's own face are wider than the column beside the board, and because it is where
# Chronicle puts it.
#
# It carries no art of its own. Chronicle stands both fields on one scene rather than giving
# each player a panel, so the scene is the whole of the backdrop and the panel is the hole
# the field draws through and nothing else - which is why this is written out transparent
# rather than as a slice of the same picture: a slice would sit on top of the scene it was
# cut from, at a different scale, and read as a rectangle.
THREE_DS_PANEL = (240, 272)
THREE_DS_FRAME_AT = (7, 11)

# the game's own digits, which the field objects sheet carries: 0-6 along one row and 7-9 on
# the next, every one a different width and all of them sitting on their row's baseline
THREE_DS_DIGITS = [
    (583, 10, 15, 24), (603, 11, 11, 22), (619, 10, 14, 23), (637, 10, 15, 24),
    (655, 10, 15, 24), (674, 11, 14, 23), (691, 10, 15, 24),
    (583, 43, 16, 23), (601, 42, 15, 24), (618, 43, 16, 23),
]
THREE_DS_DIGIT_CELL = (18, 26)


def three_ds_font(objects):
    """the ten digits packed into equal cells, bottom aligned on their own baseline"""
    w, h = THREE_DS_DIGIT_CELL
    out = Image.new("RGBA", (w * 10, h), (0, 0, 0, 0))
    for i, (x, y, dw, dh) in enumerate(THREE_DS_DIGITS):
        glyph = objects.crop((x, y, x + dw, y + dh))
        out.paste(glyph, (w * i + (w - dw) // 2, h - 1 - dh), glyph)
    return out


def three_ds_field(objects):
    """the four field frames with the game's own fill inside them, and where the cells go.

    The frame goes in `board.png` rather than in the panel, because `board_snips` indexes the
    board file and nothing else - so putting it there is what lets the field change colour as
    the pairs come down faster. Returns the four frames, the frame's size, and the top left of
    the six by twelve grid inside it.
    """
    interior_h = THREE_DS_ROWS * THREE_DS_BLOCK
    scale = interior_h / (THREE_DS_FRAME_SIZE[1] - 2 * THREE_DS_FRAME_INSET)
    size = (
        round(THREE_DS_FRAME_SIZE[0] * scale),
        round(THREE_DS_FRAME_SIZE[1] * scale),
    )
    inset = round(THREE_DS_FRAME_INSET * scale)
    well_w = THREE_DS_COLUMNS * THREE_DS_BLOCK
    well_x = inset + (size[0] - 2 * inset - well_w) // 2
    fx, fy, fw, fh = THREE_DS_FILL
    fill = objects.crop((fx, fy, fx + fw, fy + fh)).resize(
        (well_w, interior_h), Image.LANCZOS
    )
    frames = []
    for x, y in THREE_DS_FRAMES:
        frame = objects.crop(
            (x, y, x + THREE_DS_FRAME_SIZE[0], y + THREE_DS_FRAME_SIZE[1])
        ).resize(size, Image.LANCZOS)
        one = Image.new("RGBA", size, (0, 0, 0, 0))
        one.alpha_composite(fill, (well_x, inset))
        one.alpha_composite(frame)
        frames.append(one)
    return frames, size, (well_x, inset)


def three_ds_panel():
    """one player's panel, which is nothing at all - see [`THREE_DS_PANEL`]

    It is still written, because the engine sizes a player's side of the window by the
    panel's own size and a retro theme has to hand it one.
    """
    return Image.new("RGBA", THREE_DS_PANEL, (0, 0, 0, 0))


def three_ds_scene(backgrounds):
    """the scene behind both fields: one of the game's own backgrounds, blown up"""
    bw, bh = THREE_DS_BG_SIZE
    col, row = THREE_DS_BG_PICK
    scene = backgrounds.crop((col * bw, row * bh, (col + 1) * bw, (row + 1) * bh))
    scene = scene.resize(
        (bw * THREE_DS_BG_SCALE, bh * THREE_DS_BG_SCALE), Image.LANCZOS
    ).convert("RGB")
    radius, percent, threshold = THREE_DS_BG_SHARPEN
    return scene.filter(ImageFilter.UnsharpMask(radius, percent, threshold))


def three_ds():
    print("3ds (Puyo Puyo Chronicle)")
    out = os.path.join(SRC, "three_ds")
    sheet = source(THREE_DS_SKIN)
    transparent = [background_of(rgb(sheet)), *THREE_DS_BACKDROPS]

    index_of = {mask: index for index, mask in enumerate(THREE_DS_ORDER)}
    tiles = {}
    for row, _ in enumerate(COLORS):
        for mask in range(COLUMNS):
            index = index_of[mask]
            col = index % THREE_DS_GRID_COLUMNS
            sheet_row = THREE_DS_BLOCK_ROWS * row + index // THREE_DS_GRID_COLUMNS
            tiles[(row, mask)] = three_ds_cut(sheet, col, sheet_row, transparent)
    top = three_ds_row_tiles(sheet)
    tiles[(COLOR_ROWS, 0)] = three_ds_cut_at(sheet, top[THREE_DS_NUISANCE], 0, transparent)
    for i, tile in enumerate(THREE_DS_TRAY):
        tiles[(COLOR_ROWS, 1 + i)] = three_ds_cut_at(sheet, top[tile], 0, transparent)
    write_sheet(os.path.join(out, "sprites.png"), THREE_DS_BLOCK, tiles)

    objects = source(THREE_DS_OBJECTS)
    backgrounds = source(THREE_DS_BACKGROUNDS)
    frames, frame_size, cells_at = three_ds_field(objects)
    strip = Image.new("RGBA", (frame_size[0] * len(frames), frame_size[1]), (0, 0, 0, 0))
    for i, frame in enumerate(frames):
        strip.paste(frame, (frame_size[0] * i, 0))
    write_png(os.path.join(out, "board.png"), strip)
    write_png(os.path.join(out, "background.png"), three_ds_panel())
    write_png(os.path.join(out, "scene.png"), three_ds_scene(backgrounds))
    write_png(os.path.join(out, "font.png"), three_ds_font(objects))
    print(
        f"    frame {frame_size}, cells at {cells_at}, panel {THREE_DS_PANEL},"
        f" frame drawn at {THREE_DS_FRAME_AT}"
    )


# --------------------------------------------------------------------------------------

THEMES = {"genesis": genesis, "snes": snes, "3ds": three_ds}

# what each theme's directory is called under `src/theme/`. Only Chronicle's differs from its
# own name, and it has to: `3ds` cannot begin a Rust identifier, so the module - and the
# directory holding it - is `three_ds` while the menu row stays `3ds`.
THEME_DIRS = {"genesis": "genesis", "snes": "snes", "3ds": "three_ds"}


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
        path = os.path.join(SRC, THEME_DIRS[name], "sprites.png")
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
