#!/usr/bin/env python3
"""Cuts puyo-rusto's retro themes out of the gitignored spriters-resource rips in `art/retro/`.
Re-run this rather than editing its output.

    python3 puyo-rusto/art/rip_retro.py            # cut every theme
    python3 puyo-rusto/art/rip_retro.py genesis    # ... or just one
    python3 puyo-rusto/art/rip_retro.py check      # write art/retro-alignment.png

Each `sprites.png` has `rip.py`'s layout, at the rip's own block size:

    block (col, row) -> (PAD + PITCH * col, PAD + PITCH * row), BLOCK square
    rows 0-4  red, green, blue, yellow, purple; column = link mask bits
    row 5     col 0 nuisance, cols 1-3 the tray's small, large and rock symbols
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

# the output grid: a column per link mask, a row per colour, then the nuisance row
PAD = 4
COLUMNS = 16
COLOR_ROWS = 5
SHEET_ROWS = COLOR_ROWS + 1

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
    """the sheet's transparent colour: its commonest, since each rip keys to a different fill"""
    counts = Counter(map(tuple, pixels.reshape(-1, 3)[::3]))
    return list(counts.most_common(1)[0][0])


def matches(pixels, color, tolerance=12):
    return np.abs(pixels - color).sum(2) < tolerance


def components(mask):
    """the 4-connected components of a boolean mask, as (x, y, w, h) boxes (numpy only, no scipy)"""
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
    """the median hue of a cell's art in degrees, ignoring the black rim and white eyes"""
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
    """every colour's sixteen link variants, as `{(colour, mask): (x, y)}` cell corners.

    A group is a connected run whose box is whole cells, and a bean's mask is which neighbours in
    that box are occupied; `backdrops` count as occupied but are not art."""
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
                # the first exemplar wins, so a variant comes from the smallest group showing it
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
    x, y = box
    cell = image.crop((x, y, x + block, y + block)).convert("RGBA")
    px = np.array(cell)
    rgb_only = px[:, :, :3].astype(int)
    for color in transparent:
        px[:, :, 3][np.abs(rgb_only - color).sum(2) < 12] = 0
    return Image.fromarray(px)


def assert_no_marker(tiles):
    """fail on a colour dominant in three or more colour rows: a backdrop nothing keyed out"""
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


# the scene: the backdrop colour, bright in the middle and falling to the corners, drawn small
VIGNETTE = (96, 54)
# the middle's brightness, and how much of it falls away by the corners
VIGNETTE_MIDDLE = 0.75
VIGNETTE_FALL = 0.62
# over one, so the middle stays open and the shoulder is short
VIGNETTE_CURVE = 1.3


def vignette(color):
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
    """ten digits packed edge to edge, since `numeric_sprites` splits the width in ten"""
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


# Dr. Robotnik's Mean Bean Machine (Sega Genesis)

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

# the flat fills behind the arranged groups: occupancy for `link_grid`, never art
GENESIS_BACKDROPS = ([34, 177, 76], [181, 255, 181], [255, 174, 201])

# where the hues of the five beans fall; red wraps, so it gets two bands
GENESIS_HUES = [
    ("Red", 330, 360),
    ("Red", 0, 15),
    ("Yellow", 20, 70),
    ("Green", 80, 160),
    ("Blue", 185, 255),
    ("Purple", 265, 320),
]

# the top strip, 19 pixels a frame per colour band; frame 1 is the bean at rest, 0 the pop flash
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

# the refugee bean, this game's nuisance puyo
GENESIS_REFUGEE = (627, 32)

# tray symbols for one, six and thirty: the game's blob and outlined bean, then the solid refugee
GENESIS_TRAY = [(665, 32), (627, 50), (646, 50)]

# the rock repainted in player one's score red, since white reads as a hole (authored, not ripped)
GENESIS_ROCK_INK = {(255, 255, 255): (224, 64, 96)}

# the pop, `(dx, y, size)` along each band: the wide-eyed last top-strip frame, then two balls
GENESIS_SURPRISED = (GENESIS_TOP_PITCH * 4, GENESIS_TOP_ROW, GENESIS_BLOCK)
GENESIS_BALL = (19, 157, 16)
GENESIS_SMALL_BALL = (19, 147, 8)
GENESIS_DROPLET = (29, 147, 8)

# frames in one pop, which `DestroyStyle::Pop` counts; the widest strip, so also the sheet's width
GENESIS_POP_FRAMES = 3

# the refugee bean shrunken and eyeless, its blink frame
GENESIS_REFUGEE_SMALL = (665, 32)
# the white-outlined refugee it flashes to on its way out
GENESIS_REFUGEE_FLASH = (646, 50)

# the landing squash, flat then tall, `(dx, y)` along each band; not the top strip's pop faces
GENESIS_SQUASH = (0, 70)
GENESIS_STRETCH = (19, 70)

# the refugee's squash, the same art as its blink; it has no stretch
GENESIS_REFUGEE_SQUASH = (665, 32)

# frames in a landing; a longer squash reads as a puyo drawn wrong
GENESIS_BOUNCE_FRAMES = 2

# one droplet centred in a cell, thrown as debris
GENESIS_DEBRIS_FRAMES = 1

# the attack balls on the teal backdrop in the sender's palette: P1 big, small, then P2's
GENESIS_ATTACK_BALLS = [(624, 98, 22, 20), (659, 101, 16, 16), (624, 134, 22, 20), (659, 137, 16, 16)]
GENESIS_BALL_CELL = 24

# only the rows are spaced: the engine finds a frame by counting frame widths from its strip's start
GENESIS_ANIM_ROW_GAP = 4

# the dungeon wall's colour, which [`vignette`] washes the scene in
GENESIS_WALL = (0x42, 0x45, 0x00)

# the in-game board, the third of the four 320x224 boards on the boards sheet
GENESIS_BOARD_TILE = (1, 1180)
GENESIS_SCREEN = (320, 224)

# the front plane beside it (borders, well floors, boxes) over a flat key, composited over the back
GENESIS_FRAME_TILE = (323, 1180)
GENESIS_FRAME_KEY = (0, 64, 64)
# the left well within that board: six columns and twelve rows of 16, at (16, 16)
GENESIS_WELL = (16, 16, 96, 192)

# the panel: the well and the furniture column, up to the second player's well
GENESIS_PANEL_WIDTH = 208

# the outer rock cleared from each side, `(left, right)`, keeping the png's width so the cell size
# holds; `SIDE_TRIM` in `genesis/mod.rs` must equal it, and a test there checks it
GENESIS_PANEL_TRIM = (8, 4)

# every face on the fonts sheet: 8 pixel glyphs on a 9 pixel pitch, the ten digits then A to T
GENESIS_GLYPH = (1, 9, 8)
GENESIS_ALPHABET = "0123456789ABCDEFGHIJKLMNOPQRST"

# the bold score face, green until [`GENESIS_SCORE_INK`] swaps it; the white one at 123 is not it
GENESIS_FACE_BOLD = (174, 190)

# the plain white face of `STAGE`, `1P` and the stage number, inked nine rows into a 16 row cell
GENESIS_FACE_PLAIN = (140, 156)

# green to player one's red, role for role, in the sheet's own colour scaling
GENESIS_SCORE_INK = {
    (0, 96, 0): (128, 0, 64),
    (0, 128, 0): (128, 0, 64),
    (64, 160, 0): (160, 32, 64),
    (64, 224, 0): (224, 64, 96),
    (160, 224, 160): (224, 128, 128),
}

# the flat behind each glyph cell, neither page colour nor ink
GENESIS_FONT_CELL = (0, 64, 64)

# the labels the game draws as text sprites, in screen coordinates like [`GENESIS_WELL`]
GENESIS_LABELS = (
    ("SCORE", GENESIS_FACE_BOLD, (128, 160)),
    ("STAGE", GENESIS_FACE_PLAIN, (128, 80)),
)

def keyed_box(image, box, transparent):
    x, y, w, h = box
    px = np.array(image.crop((x, y, x + w, y + h)).convert("RGBA"))
    flat = px[:, :, :3].astype(int)
    for color in transparent:
        px[:, :, 3][np.abs(flat - color).sum(2) < 12] = 0
    return Image.fromarray(px)


def repaint(cell, table):
    px = np.array(cell)
    flat = px[:, :, :3].astype(int)
    for source_color, target in table.items():
        hit = np.abs(flat - source_color).sum(2) < 12
        px[:, :, 0][hit], px[:, :, 1][hit], px[:, :, 2][hit] = target
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
    px = np.array(repaint(out, recolour))
    # ... and no green may survive: add a missed shade to the table rather than widen the match
    left = np.array(Image.fromarray(px).convert("RGBA"))
    lit = (left[:, :, 3] > 0) & (left[:, :, 1].astype(int) > left[:, :, 0].astype(int))
    if lit.any():
        raise SystemExit(
            f"{lit.sum()} pixels are still green after the swap: "
            f"{sorted({tuple(c) for c in left[lit][:, :3]})}"
        )
    return Image.fromarray(px)


def genesis_cell(beans, box, size, transparent):
    """one beans sheet sprite, keyed and centred unscaled in a [`GENESIS_BLOCK`] cell"""
    tile = keyed(beans, box, size, transparent)
    if size == GENESIS_BLOCK:
        return tile
    cell = Image.new("RGBA", (GENESIS_BLOCK, GENESIS_BLOCK), (0, 0, 0, 0))
    offset = (GENESIS_BLOCK - size) // 2
    cell.paste(tile, (offset, offset))
    return cell


def genesis_animations(beans, transparent):
    """the pops, the refugee's pop and blink, the landing squashes and the droplets.

    Rows are in the order `theme/genesis/mod.rs` counts them, and nothing checks the two agree."""
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
            ]
        )
    # the refugee has no ball or droplets: it flashes white and shrinks away
    strips.append(
        [
            cut(GENESIS_REFUGEE, GENESIS_BLOCK),
            cut(GENESIS_REFUGEE_FLASH, GENESIS_BLOCK),
            cut(GENESIS_REFUGEE_SMALL, GENESIS_BLOCK),
        ]
    )
    strips.append(
        [
            cut(GENESIS_REFUGEE, GENESIS_BLOCK),
            cut(GENESIS_REFUGEE_SMALL, GENESIS_BLOCK),
            cut(GENESIS_REFUGEE, GENESIS_BLOCK),
        ]
    )
    for color in COLORS:
        band = GENESIS_COLOR_BANDS[color]
        sqx, sqy = GENESIS_SQUASH
        stx, sty = GENESIS_STRETCH
        strips.append(
            [
                cut((band + sqx, sqy), GENESIS_BLOCK),
                cut((band + stx, sty), GENESIS_BLOCK),
            ]
        )
    strips.append(
        [
            cut(GENESIS_REFUGEE_SQUASH, GENESIS_BLOCK),
            cut(GENESIS_REFUGEE, GENESIS_BLOCK),
        ]
    )
    # one droplet per colour, and the refugee's small bean in place of one
    dx, dy, dsize = GENESIS_DROPLET
    for color in COLORS:
        strips.append([cut((GENESIS_COLOR_BANDS[color] + dx, dy), dsize)])
    strips.append([cut(GENESIS_REFUGEE_SMALL, GENESIS_BLOCK)])
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


def genesis_attack_balls(beans):
    """the four attack balls, keyed on the teal backdrop, each centred in a [`GENESIS_BALL_CELL`]"""
    teal = beans.getpixel((2, 2))[:3]
    out = Image.new(
        "RGBA", (GENESIS_BALL_CELL * len(GENESIS_ATTACK_BALLS), GENESIS_BALL_CELL), (0, 0, 0, 0)
    )
    for i, (x, y, w, h) in enumerate(GENESIS_ATTACK_BALLS):
        tile = beans.crop((x, y, x + w, y + h)).convert("RGBA")
        px = np.array(tile)
        px[:, :, 3] = np.where(
            np.abs(px[:, :, :3].astype(int) - np.array(teal)).sum(axis=2) < 30, 0, 255
        )
        out.paste(
            Image.fromarray(px),
            (
                GENESIS_BALL_CELL * i + (GENESIS_BALL_CELL - w) // 2,
                (GENESIS_BALL_CELL - h) // 2,
            ),
        )
    return out


def genesis_screen(boards, back_at, front_at):
    """one 320x224 screen: the back plane under the front plane keyed on [`GENESIS_FRAME_KEY`]"""
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
    tiles[(COLOR_ROWS, len(GENESIS_TRAY))] = repaint(
        tiles[(COLOR_ROWS, len(GENESIS_TRAY))], GENESIS_ROCK_INK
    )
    write_sheet(os.path.join(out, "sprites.png"), GENESIS_BLOCK, tiles)
    write_png(
        os.path.join(out, "animations.png"), genesis_animations(beans, transparent)
    )
    write_png(os.path.join(out, "attack.png"), genesis_attack_balls(beans))

    boards = source(GENESIS_BOARDS)
    bx, by = GENESIS_BOARD_TILE
    fx, fy = GENESIS_FRAME_TILE
    wx, wy, ww, wh = GENESIS_WELL
    screen = genesis_screen(boards, (bx, by), (fx, fy))
    # cut at the well's top so the spawning row is drawn over the scene; the theme adds that cell
    # back as `top_padding`, so the padded background matches the Genesis screen
    panel = screen.crop((0, wy, GENESIS_PANEL_WIDTH, GENESIS_SCREEN[1]))
    # ... with a hole where the well is, since the board is drawn under the background
    hole = np.array(panel)
    hole[0:wh, wx : wx + ww, 3] = 0
    # ... and the outer rock cleared, colour and all, so no filter finds an edge
    left, right = GENESIS_PANEL_TRIM
    hole[:, :left] = 0
    hole[:, GENESIS_PANEL_WIDTH - right :] = 0
    panel = Image.fromarray(hole)
    write_png(
        os.path.join(out, "board.png"), screen.crop((wx, wy, wx + ww, wy + wh))
    )

    write_png(os.path.join(out, "scene.png"), vignette(GENESIS_WALL))

    fonts = source(GENESIS_FONTS)
    keys = [background_of(rgb(fonts)), GENESIS_FONT_CELL]
    for word, band, (lx, ly) in GENESIS_LABELS:
        panel.alpha_composite(genesis_glyphs(fonts, band, word, keys), (lx, ly - wy))
    write_png(os.path.join(out, "background.png"), panel)
    write_png(
        os.path.join(out, "font.png"),
        genesis_glyphs(fonts, GENESIS_FACE_BOLD, "0123456789", keys, GENESIS_SCORE_INK),
    )
    write_png(
        os.path.join(out, "font-small.png"),
        genesis_glyphs(fonts, GENESIS_FACE_PLAIN, "0123456789", keys),
    )


# Kirby's Avalanche (SNES)

SNES_BLOBS = "SNES - Kirby's Avalanche - Miscellaneous - Blobs & Boulders.png"

SNES_BLOCK = 16

# the pale fill behind each block of sprites; the magenta background is found on its own
SNES_BACKDROPS = ([248, 128, 248],)

# each colour's "Placed Blob" grid: two rows of eight variants, then a row of loose frames
SNES_GRIDS = {
    "Blue": (8, 80),
    "Red": (192, 104),
    "Yellow": (192, 160),
    "Green": (8, 304),
    "Purple": (8, 360),
}

# the sheet's link order: index bit 1 is up, 2 right, 4 down, 8 left
SNES_INDEX_BITS = ((1, UP), (2, RIGHT), (4, DOWN), (8, LEFT))
SNES_GRID_COLUMNS = 8

# the boulder, this game's nuisance, with four dissolving frames; the tray uses three of them
SNES_BOULDER = (8, 152)
SNES_BOULDER_FRAMES = 4

# the frames the boulder pops through: the three after the whole rock the board already draws
SNES_BOULDER_POP = (1, 2, 3)

# the last frame, a scatter of dots, doubles as the boulder's debris
SNES_BOULDER_DEBRIS = 3

# the loose third row: wide eyes (pop), flat, tall (landing); not the in-play "Controlling Blob"
SNES_LOOSE_ROW = SNES_BLOCK * 2
SNES_SURPRISED = 0
SNES_SQUASH = SNES_BLOCK
SNES_STRETCH = SNES_BLOCK * 2

# each colour's "Dissolving Blob / Angel Trail / Win SFX" block: ball, smaller ball, spark
SNES_DISSOLVE = {
    "Blue": (8, 192),
    "Red": (192, 232),
    "Yellow": (216, 232),
    "Green": (8, 432),
    "Purple": (32, 432),
}
# `(dx, dy, w, h)` off the block's corner, the same for every colour, as `snes_animations` asserts
SNES_BALL = (0, 0, 8, 8)
SNES_TRAIL = (9, 2, 6, 5)
SNES_SPARK = (18, 3, 4, 3)

# frames in one pop, which `DestroyStyle::Pop` counts; the widest strip, so also the sheet's width
SNES_POP_FRAMES = 3
# the squash and the stretch; a longer landing reads as a blob drawn wrong
SNES_BOUNCE_FRAMES = 2
SNES_DEBRIS_FRAMES = 1

# only the rows are spaced: the engine finds a frame by counting frame widths from its strip's start
SNES_ANIM_ROW_GAP = 4


# The board, panel and font come from the game: snes9x renders of a mid-match savestate with the
# main screen byte `$212C` poked in its FIL block, 0x03 for BG1 and BG2 and 0x02 for BG2 (the
# scenery) alone. The field is the backdrop colour, so `board.png` is a crop of BG2; the font is
# read from the uncompressed state's VRAM. With the retroarch skill's `ra.py`:
#
#   ra.py start --core snes9x --rom "Kirby's Avalanche (USA).sfc" \
#       --set video_vsync=false --set savestate_file_compression=false
#   drive to a match, ra.py state save 2, poke 0x212c, ra.py state load 2, ra.py screenshot

# the forest canopy's colour, which [`vignette`] washes the scene in
SNES_FOREST = (0x08, 0x28, 0x10)

SNES_LAYERS_BOTH = "kirby-layer-03.png"
SNES_LAYER_SCENERY = "kirby-layer-02.png"
SNES_STATE = "kirby-avalanche.state"

# the SNES screen, and the left field: 6x12 cells of 16, read off the game (BG1 is a pixel out)
SNES_SCREEN = (256, 224)
# ... stopping a row short: the last row is the console's flat border, a stripe under every match
SNES_SCREEN_BOTTOM = SNES_SCREEN[1] - 1
SNES_FIELD = (8, 16, 96, 192)
# the panel: the field and the wooden column, up to the second player's field
SNES_PANEL_WIDTH = 152

# the game's score, grassed over to the column's foot; the last course stops short of the platform
SNES_SCORE_PATCH = (34, 200, 70, 24)
SNES_GRASS = (26, 200, 64, 24)

# a course of plank laid across the empty arch, copied from the panel's own course below `STAGE`
SNES_FLOOR = (104, 192, 48, 16)
SNES_FLOOR_DONOR = (104, 120, 48, 16)

# the blobs in the arch outside the field; only its black floor, since the sky is saturated too
SNES_ARCH = (104, 194, 48, 14)

# the `NEXT` sign and plank, moved down to the field's top so the panel's cut does not halve it
SNES_NEXT_SIGN = (104, 7, 48, 24)

# the stage number's recess, filled flat since this game shows no level
SNES_STAGE_NUMBER = (120, 103, 16, 16)

# the `NEXT` boxes and the arch mouth, not cut but measured here for `snes/mod.rs`
SNES_NEXT_BOXES = ((104, 38, 24, 41), (128, 38, 24, 41))
SNES_ARCH_MOUTH = (104, 186, 48, 14)

# The score face in VRAM tiles: each digit is two, the top at `SNES_FONT_TILE + n` and the bottom
# `SNES_FONT_ROW` further on. The eight row face at 896 is the menu font and too short beside `SC`.
SNES_FONT_TILE = 769
SNES_FONT_ROW = 16

# ink index to colour, in the left player's palette (the right one draws in white)
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
    """paint over whatever is saturated in `region` with the commonest colour around it"""
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
    """flood one flat region with its commonest colour, covering a digit drawn on it"""
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

    # worked on the whole screen, in the game's coordinates, and cut to the panel at the end
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
    sx, sy, sw, sh = SNES_NEXT_SIGN
    panel.paste(panel.crop((sx, sy, sx + sw, sy + sh)), (sx, gy))

    # ... and the hole the board draws through, which is the field
    holed = np.array(panel)
    holed[gy : gy + gh, gx : gx + gw, 3] = 0
    # ... cut at the field's top edge, as genesis is
    write_png(
        os.path.join(out, "background.png"),
        Image.fromarray(holed).crop((0, gy, SNES_PANEL_WIDTH, SNES_SCREEN_BOTTOM)),
    )

    # the field is the backdrop showing through, so its art is the scenery layer
    write_png(
        os.path.join(out, "board.png"),
        scenery.crop((gx, gy, gx + gw, gy + gh)).convert("RGBA"),
    )
    write_png(
        os.path.join(out, "font.png"),
        snes_font(snes_vram(os.path.join(RETRO, SNES_STATE))),
    )
    write_png(os.path.join(out, "scene.png"), vignette(SNES_FOREST))


def snes_cell(blobs, box, transparent):
    """one blobs sheet sprite, keyed and centred unscaled in a [`SNES_BLOCK`] cell"""
    _, _, w, h = box
    tile = keyed_box(blobs, box, transparent)
    if (w, h) == (SNES_BLOCK, SNES_BLOCK):
        return tile
    cell = Image.new("RGBA", (SNES_BLOCK, SNES_BLOCK), (0, 0, 0, 0))
    cell.paste(tile, ((SNES_BLOCK - w) // 2, (SNES_BLOCK - h) // 2))
    return cell


def snes_dissolve_boxes(strip):
    x, y = strip
    return [
        (x + dx, y + dy, w, h) for dx, dy, w, h in (SNES_BALL, SNES_TRAIL, SNES_SPARK)
    ]


def snes_assert_dissolve_blocks(blobs, transparent):
    """fail unless each dissolve block is three islands of art at exactly the offsets above,
    since a mistyped offset otherwise cuts a sliver of the next sprite"""
    pixels = rgb(blobs)
    drawn = np.ones(pixels.shape[:2], bool)
    for color in transparent:
        drawn &= ~matches(pixels, color)
    for color, strip in SNES_DISSOLVE.items():
        for box in snes_dissolve_boxes(strip):
            x, y, w, h = box
            inside = drawn[y : y + h, x : x + w]
            # the art touches all four edges of its bounding box
            if not (
                inside.any(1).all() and inside.any(0).all()
            ):
                raise SystemExit(
                    f"{color}'s sprite at {box} does not reach the edges of the box it is "
                    "cut with: the offsets in SNES_BALL / SNES_TRAIL / SNES_SPARK are wrong"
                )
            # ... and nothing is drawn in the ring around it, so the cut is the whole sprite
            ring = drawn[y - 1 : y + h + 1, x - 1 : x + w + 1].copy()
            ring[1 : 1 + h, 1 : 1 + w] = False
            if ring.any():
                raise SystemExit(
                    f"{color}'s sprite at {box} runs past the box it is cut with"
                )


def snes_animations(blobs, transparent):
    """the pops, the boulder's pop, the landing squashes and every burst.

    Rows are in the order `theme/snes/mod.rs` counts them, and nothing checks the two agree."""
    snes_assert_dissolve_blocks(blobs, transparent)
    cut = lambda box: snes_cell(blobs, box, transparent)
    loose = lambda color, dx: (
        SNES_GRIDS[color][0] + dx,
        SNES_GRIDS[color][1] + SNES_LOOSE_ROW,
        SNES_BLOCK,
        SNES_BLOCK,
    )
    boulder = lambda frame: (
        SNES_BOULDER[0] + SNES_BLOCK * frame,
        SNES_BOULDER[1],
        SNES_BLOCK,
        SNES_BLOCK,
    )
    strips = []
    for color in COLORS:
        ball, trail, _ = snes_dissolve_boxes(SNES_DISSOLVE[color])
        strips.append([cut(loose(color, SNES_SURPRISED)), cut(ball), cut(trail)])
    strips.append([cut(boulder(frame)) for frame in SNES_BOULDER_POP])
    for color in COLORS:
        strips.append([cut(loose(color, SNES_SQUASH)), cut(loose(color, SNES_STRETCH))])
    for color in COLORS:
        strips.append([cut(snes_dissolve_boxes(SNES_DISSOLVE[color])[2])])
    strips.append([cut(boulder(SNES_BOULDER_DEBRIS))])

    pitch = SNES_BLOCK + SNES_ANIM_ROW_GAP
    out = Image.new(
        "RGBA", (SNES_BLOCK * SNES_POP_FRAMES, pitch * len(strips)), (0, 0, 0, 0)
    )
    for row, strip in enumerate(strips):
        for frame, cell in enumerate(strip):
            out.paste(cell, (SNES_BLOCK * frame, pitch * row))
    return out


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
    write_png(
        os.path.join(out, "animations.png"), snes_animations(blobs, transparent)
    )
    snes_art(out)



THEMES = {"genesis": genesis, "snes": snes}

def check():
    """draw every theme's cells as a board using all sixteen masks, to show the seams"""
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
