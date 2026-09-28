#!/usr/bin/env python3
"""Cuts puyo-rusto/src/theme/modern/sprites.png out of the Puyo Puyo Tetris rip.

    python3 puyo-rusto/art/rip.py            # cut sprites.png and popup.png
    python3 puyo-rusto/art/rip.py check      # ... then write alignment.png, every join drawn

The rip (`SHEET`, unversioned, beside this script) is sixteen skins on a 72 pixel grid. The
output layout, which `theme/modern/mod.rs` addresses:

    block (col, row) -> (PAD + PITCH * col, PAD + PITCH * row), BLOCK square
    skin s occupies the band at `band(s)`, COLUMNS wide and SKIN_ROWS tall
    rows 0-4  one colour each (red, green, blue, yellow, purple), column = link mask bits
    row 5     col 0 nuisance, cols 1-3 the tray's small, large and rock symbols
"""

import os
import sys

import numpy as np
from PIL import Image, ImageDraw

Image.MAX_IMAGE_PIXELS = None

HERE = os.path.dirname(os.path.abspath(__file__))
SHEET = os.path.join(
    HERE, "PC _ Computer - Puyo Puyo Tetris - Gameplay Elements - Puyo Puyo Elements.png"
)
OUT = os.path.normpath(os.path.join(HERE, "..", "src", "theme", "modern", "sprites.png"))
POPUP_OUT = os.path.normpath(
    os.path.join(HERE, "..", "src", "theme", "modern", "popup.png")
)

# the rip's grid; linked puyos meet flush only if the cut is exactly on it
SRC_BLOCK = 72

# where the first skin's grid starts, and the skins' pitch, not a whole number of blocks
SKIN_ORIGIN = (-2, 53)
SKIN_PITCH = (2048, 1024)
SKIN_COLUMNS = 4

# The skins this theme draws, by the rip's reading order. Skin 15 is on no grid, skin 7 has no
# downward neck, and the rest left out are chosen by eye on `check`'s board.
SKINS = [0, 1, 2, 3, 4, 6, 12]

COLOR_ROWS = [0, 1, 2, 3, 4]  # red, green, blue, yellow, purple

# (row, col) of the same grid
NUISANCE = (1, 18)
TRAY = [(1, 20), (1, 18), (11, 14)]  # small (1 puyo), large (6), a stand-in for the rock (30)

# the output sheet, `sprites.py`'s layout at the rip's block size, once per skin
BLOCK = SRC_BLOCK
PAD = 4
PITCH = BLOCK + 2 * PAD
COLUMNS = 16
SKIN_ROWS = 6
# bands side by side, keeping the one-texture sheet under a handheld's 4096 pixel limit
BANDS_ACROSS = 2

# LinkMask's bits: up 1, down 2, left 4, right 8
UP, DOWN, LEFT, RIGHT = 1, 2, 4, 8


def sheet_index(links):
    """the rip's column for a `LinkMask`: the rip counts down 1, up 2, right 4, left 8"""
    return (
        (1 if links & DOWN else 0)
        | (2 if links & UP else 0)
        | (4 if links & RIGHT else 0)
        | (8 if links & LEFT else 0)
    )


def origin(skin):
    """the top left of one skin's own 72 pixel grid"""
    row, column = divmod(skin, SKIN_COLUMNS)
    return (
        SKIN_ORIGIN[0] + SKIN_PITCH[0] * column,
        SKIN_ORIGIN[1] + SKIN_PITCH[1] * row,
    )


def cut(source, skin, row, col):
    x, y = origin(skin)
    x += SRC_BLOCK * col
    y += SRC_BLOCK * row
    return np.array(source.crop((x, y, x + SRC_BLOCK, y + SRC_BLOCK)))


# how many pixels of a line have to be neck before it counts as one
MIN_NECK = 8

# the alpha a pixel needs to be the puyo rather than its antialiasing
SOLID = 200


def body(plain):
    """the box the unlinked puyo fills, which is what every neck lies outside of"""
    solid = plain[:, :, 3] >= SOLID
    rows = np.nonzero(solid.sum(axis=1) >= MIN_NECK)[0]
    columns = np.nonzero(solid.sum(axis=0) >= MIN_NECK)[0]
    return rows[0], rows[-1], columns[0], columns[-1]


def margin(box, direction):
    """the lines of the cell on one side of the puyo, which is where that neck can be"""
    top, bottom, left, right = box
    if direction == UP:
        return range(0, top)
    if direction == DOWN:
        return range(bottom + 1, SRC_BLOCK)
    if direction == LEFT:
        return range(0, left)
    return range(right + 1, SRC_BLOCK)


def side(box, direction):
    """that margin as the part of the cell it covers, corners excepted"""
    top, bottom, left, right = box
    lines = margin(box, direction)
    band = slice(lines.start, lines.stop)
    return (
        (band, slice(left, right + 1))
        if direction in (UP, DOWN)
        else (slice(top, bottom + 1), band)
    )


def corner(box, vertical, horizontal):
    """the square of the cell that is outside two of the puyo's sides at once"""
    rows, columns = margin(box, vertical), margin(box, horizontal)
    return slice(rows.start, rows.stop), slice(columns.start, columns.stop)


def neck(tile, plain, box, direction):
    """the neck one link added to `tile`, and its lines wide enough to be one

    Read against the unlinked puyo and only within this side's margin, so no side takes another's.
    """
    solid = (tile[:, :, 3] >= SOLID) & ~(plain[:, :, 3] >= SOLID)
    counts = solid.sum(axis=1) if direction in (UP, DOWN) else solid.sum(axis=0)
    lines = [line for line in margin(box, direction) if counts[line] >= MIN_NECK]
    return (tile[:, :, 3] > 0) & ~(plain[:, :, 3] > 40), lines


def trim(tile, plain, box, links):
    """put every unjoined side back to the unlinked puyo's, since rip necks bleed across cells

    A corner goes back only when neither side beside it is joined.
    """
    for direction in (UP, DOWN, LEFT, RIGHT):
        if not links & direction:
            region = side(box, direction)
            tile[region] = plain[region]
    for vertical in (UP, DOWN):
        for horizontal in (LEFT, RIGHT):
            if not links & (vertical | horizontal):
                region = corner(box, vertical, horizontal)
                tile[region] = plain[region]


def borrow(tile, donor, box, direction):
    """take a neck the rip left off one variant from the variant joined that way alone"""
    lines = margin(box, direction)
    if not lines:
        return
    band = slice(lines.start, lines.stop)
    here, theirs = (
        (tile[band], donor[band])
        if direction in (UP, DOWN)
        else (tile[:, band], donor[:, band])
    )
    take = (theirs[:, :, 3] > 0) & (here[:, :, 3] == 0)
    here[take] = theirs[take]


def graft(tile, box, direction, shape):
    """run the puyo's own edge out where the rip drew no neck, as wide as `shape`

    Red's upward neck is clipped off the sheet in every skin.
    """
    top, bottom, left, right = box
    if not shape.any():
        return
    if direction == UP:
        tile[:top, shape] = tile[top, shape]
    elif direction == DOWN:
        tile[bottom + 1 :, shape] = tile[bottom, shape]
    elif direction == LEFT:
        tile[shape, :left] = tile[shape, left][:, None]
    else:
        tile[shape, right + 1 :] = tile[shape, right][:, None]


def stretch(tile, mask, direction, at):
    """run one line of a neck out to the edge of its cell"""
    if direction == DOWN:
        tile[at + 1 :, mask[at]] = tile[at, mask[at]]
    elif direction == UP:
        tile[:at, mask[at]] = tile[at, mask[at]]
    elif direction == RIGHT:
        tile[mask[:, at], at + 1 :] = tile[mask[:, at], at][:, None]
    else:
        tile[mask[:, at], :at] = tile[mask[:, at], at][:, None]


def close(tile, box, links):
    """fill the corner two necks leave undrawn, only where nothing is drawn already"""
    _, _, left, right = box
    for vertical in (UP, DOWN):
        for horizontal, edge in ((LEFT, left), (RIGHT, right)):
            if not (links & vertical and links & horizontal):
                continue
            region = corner(box, vertical, horizontal)
            notch = tile[region]
            empty = notch[:, :, 3] == 0
            beside = np.broadcast_to(tile[region[0], edge][:, None], notch.shape)
            notch[empty] = beside[empty]


def neck_shapes(source, skin):
    """each of a skin's four necks as a cross section of the cell, from any colour that draws it"""
    shapes = {}
    for direction in (UP, DOWN, LEFT, RIGHT):
        best = np.zeros(SRC_BLOCK, dtype=bool)
        for source_row in COLOR_ROWS:
            plain = cut(source, skin, source_row, sheet_index(0))
            tile = cut(source, skin, source_row, sheet_index(direction))
            box = body(plain)
            mask, lines = neck(tile, plain, box, direction)
            if not lines:
                continue
            at = lines[-1] if direction in (DOWN, RIGHT) else lines[0]
            across = mask[at] if direction in (UP, DOWN) else mask[:, at]
            if across.sum() > best.sum():
                best = across
        shapes[direction] = best
    return shapes


def repair(tile, plain, links, donors, shapes):
    """run each joined side out to the cell edge, by its own neck, a donor's, or a graft"""
    box = body(plain)
    trim(tile, plain, box, links)
    for direction in (UP, DOWN, LEFT, RIGHT):
        if not links & direction:
            continue
        mask, lines = neck(tile, plain, box, direction)
        if not lines:
            borrow(tile, donors[direction], box, direction)
            mask, lines = neck(tile, plain, box, direction)
        if not lines:
            graft(tile, box, direction, shapes[direction])
            continue
        far = direction in (DOWN, RIGHT)
        at = lines[-1] if far else lines[0]
        if at != (SRC_BLOCK - 1 if far else 0):
            stretch(tile, mask, direction, at)
    close(tile, box, links)
    return tile


def band(index):
    """where skin `index`'s band starts, as a (column, row) of the sheet's own grid"""
    return COLUMNS * (index % BANDS_ACROSS), SKIN_ROWS * (index // BANDS_ACROSS)


def paste(sheet, tile, col, row):
    sheet.paste(Image.fromarray(tile, "RGBA"), (PAD + PITCH * col, PAD + PITCH * row))


CHECK_OUT = os.path.join(HERE, "alignment.png")

# a board that uses all sixteen link masks
CHECK_BOARD = [
    "..####...#...",
    "..####..###..",
    "..####...#...",
    "..####.......",
    ".............",
    ".#...###.....",
    ".#......#....",
    ".#...........",
]


def check_masks(rows):
    """the link mask of every puyo of `rows`, worked out the way `board.rs` works them out"""
    height, width = len(rows), len(rows[0])

    def filled(x, y):
        return 0 <= x < width and 0 <= y < height and rows[y][x] == "#"

    masks = {}
    for y in range(height):
        for x in range(width):
            if not filled(x, y):
                continue
            links = 0
            if filled(x, y - 1):
                links |= UP
            if filled(x, y + 1):
                links |= DOWN
            if filled(x - 1, y):
                links |= LEFT
            if filled(x + 1, y):
                links |= RIGHT
            masks[(x, y)] = links
    return masks


def check():
    """Draw every skin's joins from the written sheet, so a seam between neighbours shows."""
    sheet = Image.open(OUT).convert("RGBA")
    masks = check_masks(CHECK_BOARD)
    missing = set(range(16)) - set(masks.values())
    if missing:
        raise SystemExit(f"CHECK_BOARD never uses masks {sorted(missing)}")
    width = len(CHECK_BOARD[0]) * BLOCK
    height = len(CHECK_BOARD) * BLOCK
    header = 26
    page = Image.new("RGBA", (width, (height + header) * len(SKINS)), (16, 16, 22, 255))
    label = ImageDraw.Draw(page)
    for index, skin in enumerate(SKINS):
        top = (height + header) * index
        label.text((6, top + 6), f"skin {index} (rip {skin})", fill=(255, 235, 0, 255))
        for (x, y), links in masks.items():
            # two colours, since a seam shows differently on dark and light
            color = 0 if y >= 5 else 1
            base_col, base_row = band(index)
            at = (
                PAD + PITCH * (base_col + links),
                PAD + PITCH * (base_row + color),
            )
            cell = sheet.crop((at[0], at[1], at[0] + BLOCK, at[1] + BLOCK))
            page.alpha_composite(cell, (x * BLOCK, top + header + y * BLOCK))
    page.convert("RGB").save(CHECK_OUT)
    print(f"{CHECK_OUT} {page.size[0]}x{page.size[1]} {len(SKINS)} skins, all 16 masks")


# Windows `(top, bottom, left, right)` around the two rows of the chain caption's face on the
# rip; `popup_row` finds the glyphs inside by alpha.
POPUP_ROWS = [
    (4240, 4380, 2580, 3120),  # 0 1 2 3 4 5 6 7
    (4370, 4500, 2580, 2930),  # 8 9 + Chain!
]
POPUP_DIGITS = [8, 2]
POPUP_WORD = (1, 3)  # (row, glyph): "Chain!"

# one glyph cell of the output; glyphs sit on the row's baseline, not their own bounding box
POPUP_CELL = (64, 100)  # the widest digit, and a line of the face
POPUP_WORD_CELL = (132, 100)
# where the baseline sits in a cell, leaving room for round digits' overshoot
POPUP_BASELINE = 96


def popup_row(source, window):
    """each glyph's `(left, right)` in one row, split at empty columns, and the row's baseline"""
    top, bottom, left, right = window
    alpha = np.array(source.crop((left, top, right, bottom)))[:, :, 3] > 20
    columns = alpha.any(axis=0)
    glyphs, start = [], None
    for x in range(len(columns) + 1):
        if x < len(columns) and columns[x]:
            start = x if start is None else start
        elif start is not None:
            glyphs.append((left + start, left + x - 1))
            start = None
    return glyphs, top + int(np.nonzero(alpha.any(axis=1))[0].max())


def popup_glyph(source, glyph, baseline, cell):
    """one glyph, centred in its cell and sitting on the row's baseline"""
    left, right = glyph
    width, height = cell
    top = baseline - POPUP_BASELINE + 1
    art = source.crop((left, top, right + 1, top + height))
    tile = Image.new("RGBA", cell, (0, 0, 0, 0))
    tile.paste(art, ((width - art.width) // 2, 0))
    return tile


def popup(source):
    """Cut `popup.png`: the ten digits on a fixed pitch, and the word "Chain!" under them.

    The two cell sizes are the only layout `theme/modern/mod.rs` has to know.
    """
    rows = [popup_row(source, window) for window in POPUP_ROWS]
    for (glyphs, _), expected in zip(rows, POPUP_DIGITS):
        if len(glyphs) < expected:
            raise SystemExit(f"the caption's face has moved: {len(glyphs)} glyphs, not {expected}")

    digits = []
    for (glyphs, baseline), count in zip(rows, POPUP_DIGITS):
        for glyph in glyphs[:count]:
            digits.append(popup_glyph(source, glyph, baseline, POPUP_CELL))
    word_glyphs, word_baseline = rows[POPUP_WORD[0]]
    word = popup_glyph(source, word_glyphs[POPUP_WORD[1]], word_baseline, POPUP_WORD_CELL)

    pitch = POPUP_CELL[1] + 2 * PAD
    sheet = Image.new(
        "RGBA",
        ((POPUP_CELL[0] + 2 * PAD) * len(digits), pitch * 2),
        (0, 0, 0, 0),
    )
    for index, digit in enumerate(digits):
        sheet.paste(digit, (PAD + (POPUP_CELL[0] + 2 * PAD) * index, PAD))
    sheet.paste(word, (PAD, PAD + pitch))
    sheet.save(POPUP_OUT)
    print(f"{POPUP_OUT} {sheet.size[0]}x{sheet.size[1]} {len(digits)} digits and a word")



def main():
    source = Image.open(SHEET).convert("RGBA")
    down = -(-len(SKINS) // BANDS_ACROSS)  # rows of bands, rounded up
    sheet = Image.new(
        "RGBA",
        (PITCH * COLUMNS * BANDS_ACROSS, PITCH * SKIN_ROWS * down),
        (0, 0, 0, 0),
    )
    for index, skin in enumerate(SKINS):
        base_col, base_row = band(index)
        shapes = neck_shapes(source, skin)
        for row, source_row in enumerate(COLOR_ROWS):
            plain = cut(source, skin, source_row, sheet_index(0))
            # the four puyos joined one way each, for `repair` to borrow a neck from
            donors = {
                direction: cut(source, skin, source_row, sheet_index(direction))
                for direction in (UP, DOWN, LEFT, RIGHT)
            }
            for links in range(16):
                tile = cut(source, skin, source_row, sheet_index(links))
                tile = repair(tile, plain, links, donors, shapes)
                paste(sheet, tile, base_col + links, base_row + row)
        paste(sheet, cut(source, skin, *NUISANCE), base_col, base_row + 5)
        for column, (row, col) in enumerate(TRAY):
            paste(sheet, cut(source, skin, row, col), base_col + 1 + column, base_row + 5)
    sheet.save(OUT)
    print(f"{OUT} {sheet.size[0]}x{sheet.size[1]} {len(SKINS)} skins")
    popup(source)


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "check":
        check()
    else:
        main()
