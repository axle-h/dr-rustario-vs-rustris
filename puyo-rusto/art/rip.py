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

    python3 puyo-rusto/art/rip.py

The output layout is `sprites.py`'s, one skin under the next, so `theme/modern/mod.rs`
addresses either the same way:

    block (col, row) -> (PAD + PITCH * col, PAD + PITCH * row), BLOCK square
    skin s occupies rows SKIN_ROWS * s to SKIN_ROWS * s + 5
    rows 0-4  one colour each (red, green, blue, yellow, purple), column = link mask bits
    row 5     col 0 nuisance, cols 1-3 the tray's small, large and rock symbols
"""

import os

import numpy as np
from PIL import Image

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

# the fifteen whole skins, in the rip's own reading order. Skin 0 is the glossy Tsu one,
# which is what the theme showed when it was the only one cut out
SKINS = list(range(15))

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


def first_row(tile):
    rows = np.nonzero((tile[:, :, 3] > 0).any(axis=1))[0]
    return int(rows[0]) if len(rows) else SRC_BLOCK


def inset(source, skin):
    """how far below the top of its cell a skin's upward neck starts when nothing clipped it

    Most skins draw the neck right out to the cell edge, so this is zero and a neck that
    starts lower was cut short. Two of them (the blocky one and the one with antennae) sit
    four pixels in all round, and there the same four pixels are the art rather than a
    wound - repairing them would smear the antenna tips up the cell. Red is left out of
    the measurement because red is the row this is here to fix.
    """
    return min(first_row(cut(source, skin, row, sheet_index(UP))) for row in COLOR_ROWS[1:])


def repair(tile, links, top):
    """the two places the rip does not line up on its own grid

    * Every skin's top row is flush with its red puyos, so red is the one colour whose
      upward neck the crop took the top four pixels off. A neck is a prism, so its first
      surviving row is exactly what is missing and repeating that up to `top` - where the
      skin's other colours start their necks - restores it, and for the four colours that
      were not cropped there is nothing to repeat.
    * An upward neck starts one pixel *above* its own cell, landing in the bottom row of
      the puyo above: a line of the wrong colour under a puyo linked to nothing below. A
      puyo that *is* linked below fills that row itself, so only the rest are cleared.
    """
    if not links & DOWN:
        tile[-3:] = 0
    if links & UP:
        start = first_row(tile)
        if top < start < SRC_BLOCK:
            tile[top:start] = tile[start]
    return tile


def paste(sheet, tile, col, row):
    sheet.paste(Image.fromarray(tile, "RGBA"), (PAD + PITCH * col, PAD + PITCH * row))


def main():
    source = Image.open(SHEET).convert("RGBA")
    sheet = Image.new(
        "RGBA", (PITCH * COLUMNS, PITCH * SKIN_ROWS * len(SKINS)), (0, 0, 0, 0)
    )
    for index, skin in enumerate(SKINS):
        base = SKIN_ROWS * index
        top = inset(source, skin)
        for row, source_row in enumerate(COLOR_ROWS):
            for links in range(16):
                tile = cut(source, skin, source_row, sheet_index(links))
                paste(sheet, repair(tile, links, top), links, base + row)
        paste(sheet, cut(source, skin, *NUISANCE), 0, base + 5)
        for column, (row, col) in enumerate(TRAY):
            paste(sheet, cut(source, skin, row, col), 1 + column, base + 5)
    sheet.save(OUT)
    print(f"{OUT} {sheet.size[0]}x{sheet.size[1]} {len(SKINS)} skins")


if __name__ == "__main__":
    main()
