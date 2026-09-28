#!/usr/bin/env python3
"""Cut the arcade theme's board art from the rips in ~/Downloads/Super Puzzle Fighter Art/.

    python3 rustle-fighter/art/rip.py         # write src/theme/arcade/
    python3 rustle-fighter/art/rip.py check   # ... and gems-check.png, failing if a cell is off blue's

The nine power gem masks are synthesised: the arcade composites a power gem from a tiled body and
a border at draw time, so the sheets carry no per-cell art.
"""

import os
import sys
from PIL import Image, ImageDraw

RIPS = os.path.expanduser("~/Downloads/Super Puzzle Fighter Art")
PREFIX = "Arcade - Super Puzzle Fighter 2 Turbo - "
OUT = os.path.join(os.path.dirname(__file__), "..", "src", "theme", "arcade")

BLOCK = 16
# transparent air around every cell, so a scaled sprite cannot bleed into its neighbour
PAD = 4
PITCH = BLOCK + 2 * PAD

PLAIN_ROW = (0, 0)
CRASH_ROW = (0, 16)
POWER_2X2 = (0, 32)
# the tiled power gem body: four shine columns; the first row carries the top corner notches
POWER_BODY = (128, 0)
# the lit frame of the ten counter gem digits
COUNTER_DIGITS = (373, 499)

HUD = "Miscellaneous - HUD.png"
# the playfield frame: 6x13 inside, its top row open over the Drop Alley (`board::DROP_ALLEY`)
FRAME = (191, 16, 102, 211)
FRAME_INSET = (3, 0)
NEXT_BOX = (301, 30, 36, 50)
SCORE_PLATE = (13, 14, 65, 38)
PLATE_DIGITS = (2, 13, 61, 14)
PLATE_BED = (2, 30)
# the plate's pink, keyed out of the score face or the speed step shows a pink box
PLATE_BED_COLOR = (192, 96, 160)
# the score face, two rows of five at eight pixels a digit so seven fit the plate
DIGITS = (138, 86, 8, 14)
DIGITS_PER_ROW = 5

TILES = "Miscellaneous - Character-Specific Background Tiles.png"
BRICK = (8, 74, 64, 32)

# name, background key, and where the sheet's own grid starts. Every coordinate is measured off
# blue; the red sheet is a rip laid out one right and one up, corrected by its origin in `load`.
SHEETS = {
    "blue": ("Miscellaneous - Blue Gems.png", (255, 0, 255), (0, 0)),
    "yellow": ("Miscellaneous - Yellow Gems.png", (255, 0, 255), (0, 0)),
    "green": ("Miscellaneous - Green Gems.png", (255, 0, 255), (0, 0)),
    "red": ("Miscellaneous - Gems.png", (0, 255, 0), (1, -1)),
}

# `GemColor`'s numbering, which `theme/arcade/mod.rs` indexes rows by
COLORS = ["blue", "yellow", "green", "red"]

# (mask, column, row) of the 3x3 each mask is cut from; bits are `PowerMask`'s
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


def load(name, key, origin=(0, 0)):
    """One sheet keyed to transparency, slid back by `origin`; `None` keys on the top-left pixel."""
    im = Image.open(os.path.join(RIPS, PREFIX + name)).convert("RGBA")
    px = im.load()
    for y in range(im.height):
        for x in range(im.width):
            r, g, b, _ = px[x, y]
            if key is None:
                key = (r, g, b)
            if max(abs(r - key[0]), abs(g - key[1]), abs(b - key[2])) <= 24:
                px[x, y] = (0, 0, 0, 0)
    if origin == (0, 0):
        return im
    grid = Image.new("RGBA", im.size, (0, 0, 0, 0))
    grid.paste(im, (-origin[0], -origin[1]))
    return grid


def cell(sheet, at, size=BLOCK):
    return sheet.crop((at[0], at[1], at[0] + size, at[1] + size))


def power_cells(sheet):
    """The nine masks: the 2x2 tile's corners and edge middles round the body, cut as a 3x3."""
    body = cell(sheet, (POWER_BODY[0], POWER_BODY[1] + BLOCK))
    tile = sheet.crop(
        (POWER_2X2[0], POWER_2X2[1], POWER_2X2[0] + 2 * BLOCK, POWER_2X2[1] + 2 * BLOCK)
    )
    half = BLOCK // 2

    template = Image.new("RGBA", (3 * BLOCK, 3 * BLOCK), (0, 0, 0, 0))
    template.paste(body, (BLOCK, BLOCK))
    template.paste(tile.crop((half, 0, half + BLOCK, BLOCK)), (BLOCK, 0))
    template.paste(tile.crop((half, BLOCK, half + BLOCK, 2 * BLOCK)), (BLOCK, 2 * BLOCK))
    template.paste(tile.crop((0, half, BLOCK, half + BLOCK)), (0, BLOCK))
    template.paste(tile.crop((BLOCK, half, 2 * BLOCK, half + BLOCK)), (2 * BLOCK, BLOCK))
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
        name, key, origin = SHEETS[color]
        src = load(name, key, origin)
        cells = [cell(src, PLAIN_ROW), cell(src, CRASH_ROW)]
        cells += power_cells(src)
        cells += [
            cell(src, (COUNTER_DIGITS[0] + digit * BLOCK, COUNTER_DIGITS[1]))
            for digit in range(10)
        ]
        for column, art in enumerate(cells):
            sheet.paste(art, (column * PITCH + PAD, row * PITCH + PAD))
    return sheet


PANEL = (171, 211)
FRAME_AT = (0, 0)
NEXT_AT = (110, 4)
SCORE_AT = (106, 70)


def hud():
    return load(HUD, None)


def board_backdrop(sheet):
    """The board's interior, dark so the brick wall does not show through, under the frame's lip."""
    inner = (FRAME[0] + FRAME_INSET[0], FRAME[1] + FRAME_INSET[1])
    width, height = BLOCK * 6, BLOCK * 13
    out = Image.new("RGBA", (width, height), (12, 10, 24, 235))
    out.alpha_composite(sheet.crop((inner[0], inner[1], inner[0] + width, inner[1] + height)))
    return out


def panel(sheet):
    out = Image.new("RGBA", PANEL, (0, 0, 0, 0))
    out.alpha_composite(sheet.crop((FRAME[0], FRAME[1], FRAME[0] + FRAME[2], FRAME[1] + FRAME[3])), FRAME_AT)
    out.alpha_composite(sheet.crop((NEXT_BOX[0], NEXT_BOX[1], NEXT_BOX[0] + NEXT_BOX[2], NEXT_BOX[1] + NEXT_BOX[3])), NEXT_AT)
    plate = sheet.crop(
        (SCORE_PLATE[0], SCORE_PLATE[1], SCORE_PLATE[0] + SCORE_PLATE[2], SCORE_PLATE[1] + SCORE_PLATE[3])
    )
    out.alpha_composite(blank_plate_digits(plate), SCORE_AT)
    return out


def digits(sheet):
    """The ten score glyphs as one strip, which `FontRenderOptions::numeric_sprites` requires."""
    x, y, width, height = DIGITS
    out = Image.new("RGBA", (width * 10, height), (0, 0, 0, 0))
    for digit in range(10):
        row, column = divmod(digit, DIGITS_PER_ROW)
        glyph = sheet.crop(
            (x + column * width, y + row * height, x + (column + 1) * width, y + (row + 1) * height)
        )
        out.paste(glyph, (digit * width, 0))
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
    """Paint the baked-in zeros out with the bed at `PLATE_BED`, which must stay clear of them."""
    px = plate.load()
    bed = px[PLATE_BED[0], PLATE_BED[1]]
    for y in range(PLATE_DIGITS[1], PLATE_DIGITS[1] + PLATE_DIGITS[3]):
        for x in range(PLATE_DIGITS[0], PLATE_DIGITS[0] + PLATE_DIGITS[2]):
            px[x, y] = bed
    return plate


def scene():
    im = Image.open(os.path.join(RIPS, PREFIX + TILES)).convert("RGBA")
    return im.crop((BRICK[0], BRICK[1], BRICK[0] + BRICK[2], BRICK[1] + BRICK[3]))


CHECK_OUT = os.path.normpath(os.path.join(os.path.dirname(__file__), "gems-check.png"))
CHECK_SCALE = 6
PLAIN, CRASH = 0, 1
POWER_COLUMNS = [2 + index for index in range(len(POWER_MASKS))]
DIGIT_COLUMNS = [2 + len(POWER_MASKS) + digit for digit in range(10)]


def sprite(sheet, color, column):
    """one cell off the written sheet, by the same arithmetic `theme/arcade/mod.rs` uses"""
    x, y = column * PITCH + PAD, COLORS.index(color) * PITCH + PAD
    return sheet.crop((x, y, x + BLOCK, y + BLOCK))


def boxes(sheet):
    """Every cell's bounding box; an opaque cell boxes the full square however it is cut."""
    out = {}
    for color in COLORS:
        for column in range(2 + len(POWER_MASKS) + 10):
            art = sprite(sheet, color, column)
            out[(color, column)] = art.getbbox()
    return out


def check(sheet):
    """Draw the gems on a board, and say which cells do not sit where blue's do."""
    bad = []
    found = boxes(sheet)
    for (color, column), box in found.items():
        want = found[(COLORS[0], column)]
        if box != want:
            bad.append(f"    {color} cell {column}: {box}, blue has {want}")
    print(f"{len(found)} cells, {len(bad)} out of place")
    for line in bad:
        print(line)

    def weave(column, height):
        return [
            [(COLORS[(x + y) % len(COLORS)], column) for x in range(12)]
            for y in range(height)
        ]

    # a 3x3 power gem per colour, so a seam between masks shows
    power = [
        [
            (color, POWER_COLUMNS[index])
            for color in COLORS
            for index, (_, _, my) in enumerate(POWER_MASKS)
            if my == gy
        ]
        for gy in range(3)
    ]

    sections = [
        ("plain gems", weave(PLAIN, 3)),
        ("crash gems", weave(CRASH, 2)),
        ("power gems, one 3x3 a colour", power),
        ("counter gems", [[(color, column) for column in DIGIT_COLUMNS] for color in COLORS]),
    ]

    header = 13
    width = max(len(row) for _, rows in sections for row in rows) * BLOCK
    height = sum(header + len(rows) * BLOCK for _, rows in sections)
    board = Image.new("RGBA", (width, height), (16, 16, 22, 255))
    label = ImageDraw.Draw(board)
    top = 0
    for title, rows in sections:
        label.text((2, top + 1), title, fill=(255, 235, 0, 255))
        top += header
        # a checker one square to the cell, so an off-centre gem shows an uneven border
        for y in range(len(rows)):
            for x in range(width // BLOCK):
                shade = 88 if (x + y) % 2 else 64
                at = (x * BLOCK, top + y * BLOCK)
                board.paste((shade, shade, shade + 12, 255), at + (at[0] + BLOCK, at[1] + BLOCK))
        for y, row in enumerate(rows):
            for x, (color, column) in enumerate(row):
                board.alpha_composite(sprite(sheet, color, column), (x * BLOCK, top + y * BLOCK))
        top += len(rows) * BLOCK

    board = board.resize((board.width * CHECK_SCALE, board.height * CHECK_SCALE), Image.NEAREST)
    board.convert("RGB").save(CHECK_OUT)
    print(f"{CHECK_OUT}  {board.width}x{board.height}  alignment board at {CHECK_SCALE}x")
    return not bad


def main():
    os.makedirs(OUT, exist_ok=True)
    sheet = gems()
    path = os.path.normpath(os.path.join(OUT, "gems.png"))
    sheet.save(path)
    print(f"{path}  {sheet.width}x{sheet.height}  {len(COLORS)} colours x {sheet.width // PITCH} cells")

    hud_sheet = hud()
    for name, art in (
        ("board.png", board_backdrop(hud_sheet)),
        ("background.png", panel(hud_sheet)),
        ("font.png", digits(hud_sheet)),
        ("scene.png", scene()),
    ):
        path = os.path.normpath(os.path.join(OUT, name))
        art.save(path)
        print(f"{path}  {art.width}x{art.height}")

    if "check" in sys.argv and not check(sheet):
        raise SystemExit("cells out of place")


if __name__ == "__main__":
    main()
