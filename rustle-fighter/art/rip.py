#!/usr/bin/env python3
"""Cut the arcade theme's board art out of the Super Puzzle Fighter II Turbo sprite rips.

The rips are **not** in the repository - they are Alex's drop at
``~/Downloads/Super Puzzle Fighter Art/``, 20 sheets from the Spriters Resource. This script
is the record of how the committed sheets were made; re-run it rather than hand-editing what
it writes.

    python3 rustle-fighter/art/rip.py            # cut everything
    python3 rustle-fighter/art/rip.py check      # ... and write a contact sheet beside it

Four gem sheets share one geometry and differ only in their colour and in how they key their
background: the red sheet is ``Gems.png`` and keys on green, the other three key on magenta.
That is the one thing the reader has to handle twice.

**The power gem is synthesised, and this is the finding that made it necessary.** The plan for
this game assumed the sheets carried per-cell power gem art that the corner codes could index.
They do not. What is on them is a *tiled body texture* in four shine frames plus a top edge
row with rounded corner notches: the arcade draws a power gem as a tiled fill with a border
composited over it at draw time, at whatever size the rectangle happens to be. Our renderer is
one snip per cell, so the nine masks a rectangle can actually produce - every cell of a
rectangle at least two on a side is (top|middle|bottom) x (left|middle|right) - are built here
out of the body texture, with the sheet's own corner notch and a bevel taken from the 2x2
power gem tile. The art is the arcade's; the *arrangement* into nine cells is ours.
"""

import os
import sys
from PIL import Image

RIPS = os.path.expanduser("~/Downloads/Super Puzzle Fighter Art")
PREFIX = "Arcade - Super Puzzle Fighter 2 Turbo - "
OUT = os.path.join(os.path.dirname(__file__), "..", "src", "theme", "arcade")

# the arcade's own cell, and the grid this script writes on
BLOCK = 16
# transparent air around every cell, so a scaled sprite cannot bleed into its neighbour. The
# same trick and the same number as `puyo-rusto/art/rip_retro.py`.
PAD = 4
PITCH = BLOCK + 2 * PAD

# --- where things are on a gem sheet, measured 2026-09-07 -------------------------------
# eight animation frames of the plain gem, then eight of the crash gem
PLAIN_ROW = (0, 0)
CRASH_ROW = (0, 16)
# the 2x2 power gem, four shine frames across; the first is the one that is cut
POWER_2X2 = (0, 32)
# the tiled power gem body: four columns of increasing shine, four rows of which the first
# carries the rounded top corner notches and the rest repeat
POWER_BODY = (128, 0)
# ten counter gem digits, four flash frames tall; the second frame is the lit one
COUNTER_DIGITS = (373, 499)

# --- the HUD sheet, measured 2026-09-07 --------------------------------------------------
HUD = "Miscellaneous - HUD.png"
# The playfield frame. Its interior is exactly six columns wide and thirteen rows tall, and
# its top row is hatched tabs over every column **except column 3** - which is the Drop Alley,
# open because that is where pieces enter. The art confirms `board::DROP_ALLEY` independently
# of the disassembly that gave us the number.
FRAME = (191, 16, 102, 211)
# the frame's interior within it: three pixels of wall each side, and the top row is the lip
FRAME_INSET = (3, 0)
# the NEXT label and the black box under it
NEXT_BOX = (301, 30, 36, 50)
# player one's score plate, the pink one; player two's purple twin sits beside it
SCORE_PLATE = (13, 14, 65, 38)
# Where its baked-in zeros sit inside it, which are painted out so a real score can go there.
# Measured off the plate at 8x: `SCORE` runs to y 12 and the seven zeros sit from y 14 to 26,
# spanning x 2 to 62.
PLATE_DIGITS = (2, 13, 61, 14)
# a pixel of the plate's own bed to fill them with, well clear of any digit
PLATE_BED = (2, 30)
# The bed colour itself, which the digit strip is also drawn on.
#
# The sheet sets the score face on a patch of the same pink the plate is made of. On the plate
# that is invisible; anywhere else - the speed step, printed on the brick wall - it is a pink
# box round the number, so it is keyed out of the font.
PLATE_BED_COLOR = (192, 96, 160)
# The score face: the sheet lays it out as two rows of five, and it is the face the score
# plate's own baked-in zeros are set in - eight pixels a digit, which is what fits the plate.
# The wide cyan strip along the bottom of the sheet is the block counter's and is twice as
# wide as this, so seven digits of it would run off the plate's end.
DIGITS = (138, 86, 8, 14)
DIGITS_PER_ROW = 5

# the brick wall the boards stand on, off the character background tile sheet
TILES = "Miscellaneous - Character-Specific Background Tiles.png"
BRICK = (8, 74, 64, 32)

SHEETS = {
    # name, background key
    "blue": ("Miscellaneous - Blue Gems.png", (255, 0, 255)),
    "yellow": ("Miscellaneous - Yellow Gems.png", (255, 0, 255)),
    "green": ("Miscellaneous - Green Gems.png", (255, 0, 255)),
    # the red sheet is the odd one out twice over: it is the unprefixed `Gems.png` and it
    # keys on green rather than magenta
    "red": ("Miscellaneous - Gems.png", (0, 255, 0)),
}

# the order colours are written in, which is `GemColor`'s own numbering: 1 blue, 2 yellow,
# 3 green, 4 red. `theme/arcade/mod.rs` indexes rows by it.
COLORS = ["blue", "yellow", "green", "red"]

# The nine masks a rectangle can produce, as (up, down, left, right) and the cell of a 3x3
# they are cut as. Bits are `PowerMask`'s: UP 1, DOWN 2, LEFT 4, RIGHT 8.
UP, DOWN, LEFT, RIGHT = 1, 2, 4, 8
POWER_MASKS = [
    (DOWN | RIGHT, 0, 0),
    (DOWN | LEFT | RIGHT, 1, 0),
    (DOWN | LEFT, 2, 0),
    (UP | DOWN | RIGHT, 0, 1),
    (UP | DOWN | LEFT | RIGHT, 1, 1),
    (UP | DOWN | LEFT, 2, 1),
    (UP | RIGHT, 0, 2),
    (UP | LEFT | RIGHT, 1, 2),
    (UP | LEFT, 2, 2),
]


def load(name, key):
    """One sheet, with its background key turned into real transparency.

    The key is matched with a little slack rather than exactly. The red sheet is a JPEG-era
    rip and its green has a hundred-odd pixels that are a shade off, which as an exact match
    leaves a fringe of green confetti round the art.
    """
    im = Image.open(os.path.join(RIPS, PREFIX + name)).convert("RGBA")
    px = im.load()
    for y in range(im.height):
        for x in range(im.width):
            r, g, b, _ = px[x, y]
            if key is None:
                key = (r, g, b)
            if max(abs(r - key[0]), abs(g - key[1]), abs(b - key[2])) <= 24:
                px[x, y] = (0, 0, 0, 0)
    return im


def cell(sheet, at, size=BLOCK):
    return sheet.crop((at[0], at[1], at[0] + size, at[1] + size))


def power_cells(sheet):
    """The nine masks, cut out of a 3x3 power gem built from the sheet's own art.

    Every pixel here is the arcade's; only the arrangement is ours. A 3x3 template is laid
    out at 48x48 and then cut into nine cells:

    * the **four corners** are the 2x2 power gem tile's own four quadrants, so a rounded
      corner is the corner the game draws;
    * the **four edges** are the middle band of that tile - the half of an edge that has no
      corner in it - which is what an edge cell between two corners looks like;
    * the **centre** is the tiled body texture, which is what the arcade fills a large gem
      with.

    That is exactly the nine a rectangle at least two on a side can produce, and no more: with
    two or more cells on each axis every cell is (top|middle|bottom) x (left|middle|right).
    """
    body = cell(sheet, (POWER_BODY[0], POWER_BODY[1] + BLOCK))
    tile = sheet.crop(
        (POWER_2X2[0], POWER_2X2[1], POWER_2X2[0] + 2 * BLOCK, POWER_2X2[1] + 2 * BLOCK)
    )
    half = BLOCK // 2

    template = Image.new("RGBA", (3 * BLOCK, 3 * BLOCK), (0, 0, 0, 0))
    # the middle, and the four edge bands, out of the body and the tile's cornerless middles
    template.paste(body, (BLOCK, BLOCK))
    template.paste(tile.crop((half, 0, half + BLOCK, BLOCK)), (BLOCK, 0))
    template.paste(tile.crop((half, BLOCK, half + BLOCK, 2 * BLOCK)), (BLOCK, 2 * BLOCK))
    template.paste(tile.crop((0, half, BLOCK, half + BLOCK)), (0, BLOCK))
    template.paste(tile.crop((BLOCK, half, 2 * BLOCK, half + BLOCK)), (2 * BLOCK, BLOCK))
    # ... and the four corners, which are the tile's own quadrants
    template.paste(tile.crop((0, 0, BLOCK, BLOCK)), (0, 0))
    template.paste(tile.crop((BLOCK, 0, 2 * BLOCK, BLOCK)), (2 * BLOCK, 0))
    template.paste(tile.crop((0, BLOCK, BLOCK, 2 * BLOCK)), (0, 2 * BLOCK))
    template.paste(tile.crop((BLOCK, BLOCK, 2 * BLOCK, 2 * BLOCK)), (2 * BLOCK, 2 * BLOCK))

    return [
        template.crop((gx * BLOCK, gy * BLOCK, (gx + 1) * BLOCK, (gy + 1) * BLOCK))
        for _, gx, gy in POWER_MASKS
    ]


def gems():
    """One sheet, a row per colour, on the [`PITCH`] grid `theme/arcade/mod.rs` reads."""
    columns = 2 + len(POWER_MASKS) + 10
    sheet = Image.new("RGBA", (columns * PITCH, len(COLORS) * PITCH), (0, 0, 0, 0))
    for row, color in enumerate(COLORS):
        name, key = SHEETS[color]
        src = load(name, key)
        cells = [cell(src, PLAIN_ROW), cell(src, CRASH_ROW)]
        cells += power_cells(src)
        cells += [
            cell(src, (COUNTER_DIGITS[0] + digit * BLOCK, COUNTER_DIGITS[1]))
            for digit in range(10)
        ]
        for column, art in enumerate(cells):
            sheet.paste(art, (column * PITCH + PAD, row * PITCH + PAD))
    return sheet


# --- the panel ---------------------------------------------------------------------------
# Everything below is arcade art; the *arrangement* is ours. The house rule is that retro
# geometry is measured against the emulated game, and that could not be done here - see the
# theme module, which says so where a reader will meet it.
PANEL = (171, 211)
FRAME_AT = (0, 0)
NEXT_AT = (110, 4)
SCORE_AT = (106, 70)


def hud():
    return load(HUD, None)


def board_backdrop(sheet):
    """The board's own interior: what the cells are drawn on top of.

    Dark, so the brick wall does not show through the playfield, with the frame's hatched top
    lip laid over its first row.
    """
    inner = (FRAME[0] + FRAME_INSET[0], FRAME[1] + FRAME_INSET[1])
    width, height = BLOCK * 6, BLOCK * 13
    out = Image.new("RGBA", (width, height), (12, 10, 24, 235))
    out.alpha_composite(sheet.crop((inner[0], inner[1], inner[0] + width, inner[1] + height)))
    return out


def panel(sheet):
    """The furniture round the board: the frame, the NEXT box and the score plate."""
    out = Image.new("RGBA", PANEL, (0, 0, 0, 0))
    out.alpha_composite(sheet.crop((FRAME[0], FRAME[1], FRAME[0] + FRAME[2], FRAME[1] + FRAME[3])), FRAME_AT)
    out.alpha_composite(sheet.crop((NEXT_BOX[0], NEXT_BOX[1], NEXT_BOX[0] + NEXT_BOX[2], NEXT_BOX[1] + NEXT_BOX[3])), NEXT_AT)
    plate = sheet.crop(
        (SCORE_PLATE[0], SCORE_PLATE[1], SCORE_PLATE[0] + SCORE_PLATE[2], SCORE_PLATE[1] + SCORE_PLATE[3])
    )
    out.alpha_composite(blank_plate_digits(plate), SCORE_AT)
    return out


def digits(sheet):
    """The ten score glyphs, dealt out of the sheet's two rows of five into one strip.

    `FontRenderOptions::numeric_sprites` wants a strip ten glyphs wide and reads the glyph
    width off the file, so the two rows have to become one.
    """
    x, y, width, height = DIGITS
    out = Image.new("RGBA", (width * 10, height), (0, 0, 0, 0))
    for digit in range(10):
        row, column = divmod(digit, DIGITS_PER_ROW)
        glyph = sheet.crop(
            (x + column * width, y + row * height, x + (column + 1) * width, y + (row + 1) * height)
        )
        out.paste(glyph, (digit * width, 0))
    # the plate's own pink comes with the face and has to go - see `PLATE_BED_COLOR`
    px = out.load()
    for gy in range(out.height):
        for gx in range(out.width):
            r, g, b, _ = px[gx, gy]
            if max(
                abs(r - PLATE_BED_COLOR[0]),
                abs(g - PLATE_BED_COLOR[1]),
                abs(b - PLATE_BED_COLOR[2]),
            ) <= 24:
                px[gx, gy] = (0, 0, 0, 0)
    return out


def blank_plate_digits(plate):
    """Take the zeros the score plate is drawn with off it, so the real score can go there.

    The plate art has `0000000` baked in. Painted over rather than left underneath: a drawn
    glyph does not cover the whole cell, so a baked zero shows through the gaps of every digit
    that is not one and the number comes out unreadable. The fill is the plate's own bed,
    sampled at [`PLATE_BED`] - which is a point well clear of the digits, because sampling
    beside them picks up the yellow of a zero and paints the plate a solid yellow bar.
    """
    px = plate.load()
    bed = px[PLATE_BED[0], PLATE_BED[1]]
    for y in range(PLATE_DIGITS[1], PLATE_DIGITS[1] + PLATE_DIGITS[3]):
        for x in range(PLATE_DIGITS[0], PLATE_DIGITS[0] + PLATE_DIGITS[2]):
            px[x, y] = bed
    return plate


def scene():
    """the brick wall the panels stand on, as one tile"""
    im = Image.open(os.path.join(RIPS, PREFIX + TILES)).convert("RGBA")
    return im.crop((BRICK[0], BRICK[1], BRICK[0] + BRICK[2], BRICK[1] + BRICK[3]))


def main():
    os.makedirs(OUT, exist_ok=True)
    sheet = gems()
    path = os.path.normpath(os.path.join(OUT, "gems.png"))
    sheet.save(path)
    print(f"{path}  {sheet.width}x{sheet.height}  {len(COLORS)} colours x {sheet.width // PITCH} cells")

    sheet = hud()
    for name, art in (
        ("board.png", board_backdrop(sheet)),
        ("background.png", panel(sheet)),
        ("font.png", digits(sheet)),
        ("scene.png", scene()),
    ):
        path = os.path.normpath(os.path.join(OUT, name))
        art.save(path)
        print(f"{path}  {art.width}x{art.height}")

    if "check" in sys.argv:
        # the same sheet at 4x on a mid grey, which is the only way to see whether a cut
        # landed on the art or a pixel beside it
        big = Image.new("RGBA", sheet.size, (96, 96, 110, 255))
        big.alpha_composite(sheet)
        big = big.resize((sheet.width * 4, sheet.height * 4), Image.NEAREST)
        check = os.path.normpath(os.path.join(OUT, "..", "..", "..", "art", "gems-check.png"))
        big.save(check)
        print(f"{check}  contact sheet at 4x")


if __name__ == "__main__":
    main()
