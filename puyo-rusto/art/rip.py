#!/usr/bin/env python3
"""Cuts puyo-rusto/src/theme/modern/sprites.png out of the Puyo Puyo Tetris rip.

The particle theme used to draw its own puyos (see `sprites.py`, kept beside this as the
procedural original); it now takes them from the sprite sheet in this directory, which is
the one thing in the repository that looks like the game it is a clone of.

The rip is sixteen skins laid out four by four, every one of them on the same 72 pixel grid,
and fifteen of them are whole skins: the last (row 3, column 3) is a grab bag of odds and
ends on no grid at all and is skipped. `SKINS` says which of the fifteen are cut - seven of
them - and each contributes the cells this game can draw; the theme deals one skin per
player, see `theme/modern/mod.rs`.

    rows 0-4    a colour each, sixteen link variants along the row
    (1, 18)     the nuisance puyo, which is also what the tray shows for six
    (1, 20)     the same puyo small, which is what the tray shows for one
    (11, 14)    a bigger nuisance again, which stands in for the rock at thirty - the rip
                carries the star, the moon and the crown but no rock of its own

The rip's link index is *not* this game's: it counts down 1, up 2, right 4, left 8, where
`LinkMask` in `game/cell.rs` counts up 1, down 2, left 4, right 8. `sheet_index` is that
swap, and it is one of the two reasons this is a script rather than a crop. The other is
`repair`.

    python3 puyo-rusto/art/rip.py            # cut both sheets
    python3 puyo-rusto/art/rip.py check      # ... and check every join in the puyos

It writes a second, much smaller sheet beside that one: `popup.png`, the caption a chain
step says over the puyos it just took. See `POPUP_ROWS`.

The output layout is `sprites.py`'s a band at a time, so `theme/modern/mod.rs` addresses
either the same way:

    block (col, row) -> (PAD + PITCH * col, PAD + PITCH * row), BLOCK square
    skin s occupies the band at `band(s)`, COLUMNS wide and SKIN_ROWS tall
    rows 0-4  one colour each (red, green, blue, yellow, purple), column = link mask bits
    row 5     col 0 nuisance, cols 1-3 the tray's small, large and rock symbols

The bands run BANDS_ACROSS at a time rather than all in one column, because the whole sheet
is loaded as a single texture: stacked, the fourteen skins cut before any were dropped stood
6720 pixels tall - past the 4096 a handheld's driver will allocate in a dimension, which is
the same ceiling `MAX_ATLAS_WIDTH` keeps the built atlas under. Seven of them would fit
either way; the layout is kept because it is the same pixels and one fewer thing to agree on.
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

# the rip's grid. A neck runs exactly to its cell's edge, so two linked puyos meet flush
# only if the cut is on these lines and nowhere near them
SRC_BLOCK = 72

# where the first skin's grid starts, and how far apart the skins are. The pitch is not a
# whole number of blocks: each skin was laid out on its own, and only its top left corner
# lines up with anything
SKIN_ORIGIN = (-2, 53)
SKIN_PITCH = (2048, 1024)
SKIN_COLUMNS = 4

# The skins this theme draws, in the rip's own reading order. Skin 0 is the glossy Tsu one,
# which is what the theme showed when it was the only one cut out.
#
# Fifteen of the sixteen are whole - the sixteenth (row 3, column 3) is a grab bag on no grid.
# Skin 7 is left out as well: its sixteen link variants are only eight, paired so that a puyo
# joined below draws exactly like one joined to nothing. It has no downward neck to cut, so
# nothing can make it meet the puyo underneath - see `check`, which is what found it.
#
# Skins 8, 10 and 11 are left out for how they *look* joined rather than for a missing neck.
# The point of a set is that four in a row read as one mass: 10 is a television with antennae
# whose necks are stubs, so a run of them stays a row of televisions with the antennae poking
# between; 8 is a stick figure that joins into an elongated humanoid; 11 is a small round face
# that merges but leaves a gappy mesh. `check` is how to see it - a whole board of one skin,
# which is the only way any of this reads.
#
# The last four are left out for what they *are* rather than for how they cut. Skin 5 is a
# pixel art face and 14 a bevelled gem brick: this game already has two retro themes cut from
# the consoles that drew them, and a retro set here is one of those without the console. Skin
# 13 is the Sonic cast (Knuckles, Shadow, Sonic, Tails, Amy), which belongs to Mean Bean
# Machine's side of the game and not to this one. And skin 9 spells its colours out as the
# letters R, G, B, Y, P, which is a legend rather than a puyo - four in a row read as a word.
#
# Skin 2 is a brick as well and is **kept**, which is the line being drawn by eye rather than
# by rule: it was cut with the other two and put back. What is left is seven sets that are all
# the same five colours and all drawn as pieces of one board.
SKINS = [0, 1, 2, 3, 4, 6, 12]

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
# how many skin bands lie side by side; see the layout note at the top
BANDS_ACROSS = 2

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

# the alpha a pixel needs before it is the puyo rather than the antialiasing around it. A
# neck is run out to its cell's edge by repeating its last line, and repeating a half
# transparent one only draws the seam a shade lighter than leaving it there
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
    """the neck one link added to `tile`, and which of its lines are wide enough to be one

    By *difference*, the linked tile against the same puyo unlinked - one skin wears
    antennae on the same line as its upward neck, and repeating those paints a band of
    antenna up the cell - and only within the margin that side's neck can be in. That
    second half is the part this used to get wrong: a tile joined on three sides carries
    three necks, and reading the outermost line of all of them at once hands one direction
    another's line. Skin 2's brick joined up, down and right was drawn with no downward neck
    at all, so running what it found - the *upward* one - down the cell flooded the tile.

    The mask is every pixel of the neck including the soft edge it stops at, so that edge is
    carried out with the rest of the line; which line to carry is read off the solid part.
    """
    solid = (tile[:, :, 3] >= SOLID) & ~(plain[:, :, 3] >= SOLID)
    counts = solid.sum(axis=1) if direction in (UP, DOWN) else solid.sum(axis=0)
    lines = [line for line in margin(box, direction) if counts[line] >= MIN_NECK]
    return (tile[:, :, 3] > 0) & ~(plain[:, :, 3] > 40), lines


def trim(tile, plain, box, links):
    """put every side this puyo is not joined on back to the unlinked puyo's

    The rip's cells are not quite its grid: an upward neck starts a pixel *above* its own
    cell and lands in the bottom row of the one above it on the sheet, which draws a line of
    the wrong colour under a puyo joined to nothing below. Restoring the margin rather than
    clearing it keeps the puyo's own soft edge, which the skin with antennae has out there.
    A corner goes back only when neither of the two sides it lies between is joined, since a
    neck of either may reach into it.
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
    """take a neck the rip drew on one variant and left off another

    A neck is the same shape whatever else the puyo is joined to, so one missing from a
    variant can be had from the variant that is joined that way and nothing else. Skin 2 is
    the one that needs it: its brick joined up, down and right was drawn without the
    downward neck the same brick joined up and down has.
    """
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
    """run the puyo's own edge out where the rip drew no neck at all

    The sheet has no room above its top row for an upward neck, so the first colour of every
    skin is cut without one - and unlike skin 2's missing downward neck there is no variant
    to borrow it from, since no red puyo on the sheet has one. The skin's other colours say
    where the neck goes and how wide it is; what it is made of is this puyo's own outermost
    line, which is the line a neck continues. `shape` is empty for a skin that draws no such
    neck in any colour, and that one is left with the gap it was drawn with.
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
    """run one line of a neck out to the edge of its cell

    A neck is a prism, so its last line is exactly what is missing between where the art
    stops and where the cell ends - and the rip has necks stopping anywhere from one to
    eight pixels short, because its skins were drawn on their own pitches and laid out on a
    common 72 pixel grid.
    """
    if direction == DOWN:
        tile[at + 1 :, mask[at]] = tile[at, mask[at]]
    elif direction == UP:
        tile[:at, mask[at]] = tile[at, mask[at]]
    elif direction == RIGHT:
        tile[mask[:, at], at + 1 :] = tile[mask[:, at], at][:, None]
    else:
        tile[mask[:, at], :at] = tile[mask[:, at], at][:, None]


def close(tile, box, links):
    """fill the notch where two necks meet

    A neck is drawn the width of the puyo and no wider, so the little square outside both of
    them - where a puyo is joined downward *and* to the right - is drawn by neither, and
    shows as a speck of nothing at the point four puyos meet. It is filled from the line
    beside it, which by now is a neck run out to the edge, and only where nothing is drawn
    already, so a skin that does draw its corners keeps what it drew.
    """
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
    """where each of a skin's four necks goes, taken from whichever colour draws it

    A skin's necks are one shape in five palettes, so a colour cut without one can still be
    told where it belongs by the colours that were not. Returns each neck's cross section as
    a line of the cell - which columns an upward one covers, which rows a leftward one does -
    and an empty one for a direction the skin has no neck in at all.
    """
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
    """make one cut tile meet the cells around it

    Every skin has necks that stop short of the cell they were drawn in: the sheet's own top
    row clips red's upward neck in all of them, and most skins were drawn on a pitch of their
    own and stop short at the bottom besides. Each side the puyo is joined on is run out to
    its edge - by the neck it has, the neck another variant has, or, failing both, its own
    outermost line - and each side it is not joined on is put back to the unlinked puyo's.
    """
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
            base_col, base_row = band(index)
            at = (
                PAD + PITCH * (base_col + links),
                PAD + PITCH * (base_row + color),
            )
            cell = sheet.crop((at[0], at[1], at[0] + BLOCK, at[1] + BLOCK))
            page.alpha_composite(cell, (x * BLOCK, top + header + y * BLOCK))
    page.convert("RGB").save(CHECK_OUT)
    print(f"{CHECK_OUT} {page.size[0]}x{page.size[1]} {len(SKINS)} skins, all 16 masks")


# ---------------------------------------------------------------- the chain caption

# Where the caption's face is on the rip: the digits and the word "Chain!", one size of one
# face, laid out as two rows with a "+" between the 9 and the word that this game has no use
# for. Each row is (top, bottom, left, right) of a window around it; `popup_row` finds the
# glyphs inside one by their alpha, so nothing here has to name a column.
POPUP_ROWS = [
    (4240, 4380, 2580, 3120),  # 0 1 2 3 4 5 6 7
    (4370, 4500, 2580, 2930),  # 8 9 + Chain!
]
# how many glyphs each row is expected to hold, and which of them are wanted: the second row
# is the two digits and then the word, with the "+" skipped
POPUP_DIGITS = [8, 2]
POPUP_WORD = (1, 3)  # (row, glyph): "Chain!"

# One glyph cell of the output. Everything is drawn against the row's own baseline rather
# than its own bounding box - the word sits above it and the round digits hang a little below
# it - so a run of cells drawn at one y keeps exactly the relationship the rip drew.
POPUP_CELL = (64, 100)  # the widest digit, and a line of the face
POPUP_WORD_CELL = (132, 100)
# where the baseline sits in a cell. The digits overhang it by two or three pixels, which is
# the overshoot every round glyph of every face has
POPUP_BASELINE = 96


def popup_row(source, window):
    """the glyphs of one row of the caption's face, in reading order

    A row of a rip is a row of a rip: the only thing that says where one glyph stops and the
    next starts is a column of nothing between them. Returns each glyph's own left and right
    and the row's baseline, which is as far down as any of them reach.
    """
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
    """Cut `popup.png`: the ten digits on one row and the word "Chain!" under them.

    A chain step says how far into the chain it is over the puyos it just took, and what it
    says it in is the game's own face rather than the engine's - which is the whole of why
    this is worth cutting. The two cell sizes are the only layout `theme/modern/mod.rs` has
    to know: digits on a fixed pitch, because a counter that jumps about as it climbs from 9
    to 10 reads worse than one that does not, and the word on its own wider one.
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
            # the four puyos joined one way each, which is where `repair` goes for a neck
            # the rip drew on one variant and left off another
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
