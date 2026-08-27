#!/usr/bin/env python3
"""Cuts puyo-rusto/src/theme/modern/sprites.png out of the Puyo Puyo Tetris rip.

The particle theme used to draw its own puyos (see `sprites.py`, kept beside this as the
procedural original); it now takes them from the sprite sheet in this directory, which is
the one thing in the repository that looks like the game it is a clone of.

The rip is sixteen skins laid out four by four, every one of them on the same 72 pixel grid,
and fifteen of them are whole skins: the last (row 3, column 3) is a grab bag of odds and
ends on no grid at all and is skipped. Each of the fifteen contributes the cells this game
can draw, and the theme deals one skin per player - see `theme/modern/mod.rs`.

    rows 0-4    a colour each, sixteen link variants along the row
    (1, 18)     the nuisance puyo, which is also what the tray shows for six
    (1, 20)     the same puyo small, which is what the tray shows for one
    (11, 14)    a bigger nuisance again, which stands in for the rock at thirty - the rip
                carries the star, the moon and the crown but no rock of its own

The rip's link index is *not* this game's: it counts down 1, up 2, right 4, left 8, where
`LinkMask` in `game/cell.rs` counts up 1, down 2, left 4, right 8. `sheet_index` is that
swap, and it is one of the two reasons this is a script rather than a crop. The other is
`repair`.

    python3 puyo-rusto/art/rip.py            # cut the sheet
    python3 puyo-rusto/art/rip.py check      # ... and check every join in it

The output layout is `sprites.py`'s, one skin under the next, so `theme/modern/mod.rs`
addresses either the same way:

    block (col, row) -> (PAD + PITCH * col, PAD + PITCH * row), BLOCK square
    skin s occupies rows SKIN_ROWS * s to SKIN_ROWS * s + 5
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

# the rip's grid. A neck runs exactly to its cell's edge, so two linked puyos meet flush
# only if the cut is on these lines and nowhere near them
SRC_BLOCK = 72

# where the first skin's grid starts, and how far apart the skins are. The pitch is not a
# whole number of blocks: each skin was laid out on its own, and only its top left corner
# lines up with anything
SKIN_ORIGIN = (-2, 53)
SKIN_PITCH = (2048, 1024)
SKIN_COLUMNS = 4

# The whole skins, in the rip's own reading order. Skin 0 is the glossy Tsu one, which is what
# the theme showed when it was the only one cut out.
#
# Fifteen of the sixteen are whole - the sixteenth (row 3, column 3) is a grab bag on no grid.
# Skin 7 is left out as well: its sixteen link variants are only eight, paired so that a puyo
# joined below draws exactly like one joined to nothing. It has no downward neck to cut, so
# nothing can make it meet the puyo underneath - see `check`, which is what found it.
SKINS = [0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14]

# the sheet's own eighty coloured cells are in the order this game numbers its colours
COLOR_ROWS = [0, 1, 2, 3, 4]  # red, green, blue, yellow, purple

# where the rest of it is, as (row, col) of the same grid
NUISANCE = (1, 18)
TRAY = [(1, 20), (1, 18), (11, 14)]  # small (1 puyo), large (6), "rock" (30)

# the output sheet, which is `sprites.py`'s layout at the rip's block size, once per skin
BLOCK = SRC_BLOCK
PAD = 4
PITCH = BLOCK + 2 * PAD
COLUMNS = 16
SKIN_ROWS = 6

# LinkMask's bits: up 1, down 2, left 4, right 8
UP, DOWN, LEFT, RIGHT = 1, 2, 4, 8


def sheet_index(links):
    """the rip's column for one of this game's link masks"""
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


# how many pixels of a line have to be neck before it counts as one, rather than a stray
# pixel of shading that happens to differ from the unlinked puyo
MIN_NECK = 8


def neck(tile, plain):
    """which pixels of `tile` are the necks its links added, against the same puyo unlinked"""
    return (tile[:, :, 3] > 40) & ~(plain[:, :, 3] > 40)


def stretch(tile, mask, direction):
    """Run a neck out to the edge of its cell.

    A neck is a prism, so its last line is exactly what is missing between where the art
    stops and where the cell ends - and the rip has necks stopping anywhere from one to
    eight pixels short, because its skins were drawn on their own pitches and laid out on a
    common 72 pixel grid. Only the line's *neck* pixels are carried out: a skin whose puyo
    wears antennae has them on the same line, and repeating those would draw a band of them
    up the cell.
    """
    lines = mask.any(axis=1) if direction in (UP, DOWN) else mask.any(axis=0)
    wide = np.nonzero(
        (mask.sum(axis=1) if direction in (UP, DOWN) else mask.sum(axis=0)) >= MIN_NECK
    )[0]
    if not lines.any() or not len(wide):
        return
    if direction == DOWN and wide[-1] < SRC_BLOCK - 1:
        at = wide[-1]
        tile[at + 1 :, mask[at]] = tile[at, mask[at]]
    elif direction == UP and wide[0] > 0:
        at = wide[0]
        tile[: at, mask[at]] = tile[at, mask[at]]
    elif direction == RIGHT and wide[-1] < SRC_BLOCK - 1:
        at = wide[-1]
        tile[mask[:, at], at + 1 :] = tile[mask[:, at], at][:, None]
    elif direction == LEFT and wide[0] > 0:
        at = wide[0]
        tile[mask[:, at], : at] = tile[mask[:, at], at][:, None]


def repair(tile, plain, links):
    """the two places the rip does not line up on its own grid

    * A neck that stops short of its cell draws a seam between two puyos that are supposed
      to be one shape. Every skin has some: the sheet's own top row clips red's upward neck
      in all of them, and most skins were drawn on a pitch of their own and stop short at
      the bottom besides. `stretch` runs each one out to the edge it belongs at.
    * An upward neck starts one pixel *above* its own cell, landing in the bottom row of
      the puyo above: a line of the wrong colour under a puyo linked to nothing below. A
      puyo that *is* linked below fills that row itself, so only the rest are cleared.
    """
    if not links & DOWN:
        tile[-3:] = 0
    mask = neck(tile, plain)
    for direction in (UP, DOWN, LEFT, RIGHT):
        if links & direction:
            stretch(tile, mask, direction)
    return tile


def paste(sheet, tile, col, row):
    sheet.paste(Image.fromarray(tile, "RGBA"), (PAD + PITCH * col, PAD + PITCH * row))


# ---------------------------------------------------------------- checking the cut

CHECK_OUT = os.path.join(HERE, "alignment.png")

# A board that uses all sixteen masks, so every join a game can ask for is on the page: a
# solid block (corners, edges and interior), a plus (four arms joined one way each), a row of
# three and a column of three (the two ends and a middle), and a puyo joined to nothing.
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
    """Draw every skin's joins, so a neck that does not meet its neighbour is visible.

    This is the only way to see the thing `repair` exists to fix. A seam is a hairline and
    reading it off the sheet a cell at a time will not show it: two puyos have to be put
    side by side. Reads the sheet that was written rather than the rip, so what it checks
    is what the game will draw.
    """
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
            # the block and the plus in one colour, the rows and the lone puyo in another,
            # since a seam shows differently on a dark colour than a light one
            color = 0 if y >= 5 else 1
            at = (PAD + PITCH * links, PAD + PITCH * (SKIN_ROWS * index + color))
            cell = sheet.crop((at[0], at[1], at[0] + BLOCK, at[1] + BLOCK))
            page.alpha_composite(cell, (x * BLOCK, top + header + y * BLOCK))
    page.convert("RGB").save(CHECK_OUT)
    print(f"{CHECK_OUT} {page.size[0]}x{page.size[1]} {len(SKINS)} skins, all 16 masks")


def main():
    source = Image.open(SHEET).convert("RGBA")
    sheet = Image.new(
        "RGBA", (PITCH * COLUMNS, PITCH * SKIN_ROWS * len(SKINS)), (0, 0, 0, 0)
    )
    for index, skin in enumerate(SKINS):
        base = SKIN_ROWS * index
        for row, source_row in enumerate(COLOR_ROWS):
            plain = cut(source, skin, source_row, sheet_index(0))
            for links in range(16):
                tile = cut(source, skin, source_row, sheet_index(links))
                paste(sheet, repair(tile, plain, links), links, base + row)
        paste(sheet, cut(source, skin, *NUISANCE), 0, base + 5)
        for column, (row, col) in enumerate(TRAY):
            paste(sheet, cut(source, skin, row, col), 1 + column, base + 5)
    sheet.save(OUT)
    print(f"{OUT} {sheet.size[0]}x{sheet.size[1]} {len(SKINS)} skins")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "check":
        check()
    else:
        main()
