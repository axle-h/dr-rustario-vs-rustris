#!/usr/bin/env python3
"""Read Kirby's fifteen idle routines off Kirby's Avalanche, and cut the art for them.

`mugshots.py` is this script's sibling: same job, other game.  Where that one reads a *mugshot* -
a fixed box, one strip a state, a face and nothing else - this reads the little Kirby the game
stands in the arch at the foot of its centre column, who is a different animal.  He walks, he
spins, he flops on his face, he inflates and floats clean out of the arch for a second and a
half, and his frames are not one size: the idle pose is 14x14, the pancake he lands in is 14x7
and the tall thin one he bobs up into is 6x14.  So the art model here is a *routine* - a list of
(sprite, how long it is held, where it goes in the arch) - and not a strip at a constant rate.

Two sources, neither of them in the repository:

  * `retro/SNES - Kirby's Avalanche - Playable Characters - Kirby.png`, the spriters-resource
    rip, which carries the poses and nothing else - no order, no timing, no positions, and no
    grid either: it is packed edge to edge, so a sprite's own rect cannot be found by looking
    for the gap around it, because there is not one.
  * `~/Videos/Screencasts/Screencast From 2026-08-31 18-44-35.mp4`, a 115 second capture of the
    centre column of the emulated game, which carries all three of the things the rip does not.

    python3 kirby.py read      # re-derive SPRITES and ROUTINES from the capture
    python3 kirby.py cut       # write theme/snes/kirby.png and print the Rust to paste back

**How the tables below were got.**  The capture is registered against the game's own screen
first - `kirby-layer-03.png` is a render of the SNES background layers, so correlating the two
on their gradients gives the scale and the origin to a pixel: 4.74 and (104, 84), which is
`SCALE` and `ORIGIN` below.  Then, per frame, Kirby is found by **background subtraction**
against a plate of the empty column, and the crop is matched against the rip at *every* offset
and at every size within a pixel of what the registration implies - not against a grid of cells,
because there is no grid.  He is found on all 3465 frames of the capture, the search scores 0.93
mean normalised correlation, and the rect it settles on is the sprite's own bounding box.

Four things about the silhouette took a pass each and every one of them looked like something
else, so they are worth knowing before changing any of it:

  * the crop has to be the **whole recording** and not the arch - he is a sprite and the game
    draws him over every course of the column, and a silhouette the crop has cut matches a
    *sub-rect* of a sprite, so the cut sprite comes out cut too;
  * a **colour** key drops his black outline and his shaded side, which does the same thing
    more quietly - subtraction puts the standing pose at fourteen native pixels on 2691 frames
    where the colour key wandered between ten and fifteen;
  * the plate's exclusion box has to clear his **feet**, which hang four pixels below the pink
    his body is located by, or the plate takes them for woodwork and they stop differing from
    it;
  * and his feet are often a **separate blob** from his body, so the components are found on a
    dilated mask and intersected back, or the largest-blob rule keeps the body and drops the
    feet.

The `STAGE` sign is told from the woodwork by its own **variance** and not its colour, because
brown answers a "redder than blue" test exactly as a flame does.  `SPRITES` is those rects, and two of them are the same pose **only if
they agree to a pixel on every edge**. Every looser rule was tried and every one of them cuts a
sprite short: tightening a rect onto its own content reaches into the sprite next door, because
the rip is packed edge to edge; taking a group's union does the same; and grouping by overlap
merges a pose with a *sub-rect of itself* - (65,113) is a fourteen wide Kirby and also, on the
frames where the silhouette came out ten wide, the left ten of him - after which the group's
modal rect cuts the wide pose's right side off.

A pose is rare or it is common; it is not a misread because it is rare. The walk's own cycle is
eight sprites shown for two frames each in one routine, so a plain count threshold cut the
middle out of the walk. What tells a pose from a misread is how well it ever scored.

**What is measured and what is not.**  The capture is 30 fps against the game's 60, so a hold is
measured to two ticks and no better, and every number in `ROUTINES` is even for that reason.
Positions are the *foot line* and the *centre* of the found silhouette - the two things the
colour key gets right - because the mask clips a dark red foot at the edge and so the left and
right edges of the box wander a pixel either way.  A gap of four frames or fewer is Kirby
clipped by the arch's own edge and is walked across; a longer one is Kirby genuinely gone, which
happens in five of the fifteen and is a frame that draws nothing.

Which routine goes on which of the four `CharacterState` rows is **not** measured, and cannot be:
nothing in the capture ties a routine to anything happening on the board, and the game gives no
sign that anything does.  So the `state` column is this game's own reading - the placid ones idle,
the excited ones on a chain, the flat one when the stack is high - and within a row the routine
is dealt.
"""

import os
import sys

from collections import Counter

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
RETRO = os.path.join(HERE, "retro")
SHEET = os.path.join(RETRO, "SNES - Kirby's Avalanche - Playable Characters - Kirby.png")
LAYERS = os.path.join(RETRO, "kirby-layer-03.png")
CAPTURE = os.path.join(
    os.path.expanduser("~"), "Videos", "Screencasts",
    "Screencast From 2026-08-31 18-44-35.mp4")

# the rip's own background, which every cut sprite is keyed against
KEY = (64, 0, 192)

# the capture, registered against `kirby-layer-03.png` on gradients: one SNES pixel is this
# many capture pixels, and capture (0, 0) is this point on the SNES screen
SCALE = 4.74
ORIGIN = (104, 84)

# the arch opening, in the panel's own pixels - which are the SNES screen's, since `snes/mod.rs`
# cuts the panel from the screen itself.  Its bottom edge is the foot line every standing frame
# of the capture sits on, measured at y=199 on 2343 of 2775 frames.
BOX = (104, 157, 48, 42)

# **The whole recording**, and not the arch. Kirby is a sprite: the game draws him over every
# course of the centre column and right up past `STAGE`, seventy two pixels above the arch,
# which is twice the arch's own height. Three croppings were tried and every one of them lost
# him - the arch alone cut the climb off at the lintel, and a silhouette cut like that matches
# a *sub-rect* of a sprite, so the sprite came out cut too. At this crop he is found on all
# 3465 frames of the capture and 2700 of them measure exactly fourteen pixels across.
ARCH_CROP = (0, 0, 224, 600)

# A silhouette whose top is within this many capture pixels of the crop's is Kirby half
# *behind* the arch's own lintel, not a Kirby that shape. Those frames are kept out of the
# vocabulary and given the nearest whole pose: the theme clips the arch itself, so the whole
# pose should be drawn and hidden by it, and a routine that alternates a whole pose with a
# bottom sliver of one flashes.
LINTEL = 12

# --------------------------------------------------------------------------------------
# The measured tables.  Re-derive them with `read`; both are printed in this form.

# Every pose the capture ever showed, as its own bounding box in the rip.  Kirby is not one
# size: 45 sprites from 6x14 to 18x11.
SPRITES = [
    ( 17, 129, 14, 14),   # 0
    ( 65,  17, 13, 14),   # 1
    (113,   2, 15, 13),   # 2
    ( 49, 120, 14,  7),   # 3
    ( 65, 113, 14, 14),   # 4
    ( 97,  98, 14, 13),   # 5
    (113, 114, 14, 13),   # 6
    ( 33, 129, 14, 14),   # 7
    ( 65,  16, 14, 16),   # 8
    ( 40,  49,  8, 15),   # 9
    (121,  16,  7, 16),   # 10
    ( 97,  64, 14, 16),   # 11
    (104,  16, 15, 15),   # 12
    ( 17,  65, 14, 15),   # 13
    (113,  65, 14, 13),   # 14
    ( 65,  65, 14, 15),   # 15
    ( 97, 114, 14, 13),   # 16
    ( 64,  16, 15, 12),   # 17
    ( 85,  17, 13, 11),   # 18
    (112,  98, 15, 13),   # 19
    (119,  17,  8, 16),   # 20
    ( 81,   2, 15, 13),   # 21
    ( 33,  65, 15, 15),   # 22
    ( 33,  16, 15, 15),   # 23
    ( 97,  81, 15, 14),   # 24
    ( 64,  17, 15, 14),   # 25
    ( 97,  83, 15,  7),   # 26
    (121,  17,  7, 14),   # 27
    ( 97,  79, 15, 15),   # 28
    ( 49,  16, 15, 15),   # 29
    (  1,  17, 15, 14),   # 30
    ( 82,  17, 16, 12),   # 31
    ( 81,  32, 13, 13),   # 32
    ( 49,  33,  7, 15),   # 33
    ( 17,  16, 14, 15),   # 34
    ( 49,   2, 13, 13),   # 35
    ( 96,  65, 15, 14),   # 36
    ( 16, 129, 16, 14),   # 37
    ( 79,  51, 15, 12),   # 38
    ( 64,  64, 16, 15),   # 39
    ( 17,  63, 16, 15),   # 40
    ( 17,  80, 13, 15),   # 41
    ( 65,  17, 13, 11),   # 42
    (114,  63, 13, 13),   # 43
    ( 48,  64, 16, 15),   # 44
    ( 16,  65, 16, 12),   # 45
    ( 66,  80, 13, 15),   # 46
    ( 49,  81, 14, 15),   # 47
    ( 50,  33, 12, 15),   # 48
    ( 98,  81, 12, 14),   # 49
    (  1,  81, 14, 14),   # 50
    (  1,  32, 13, 15),   # 51
    ( 48,   2, 15, 12),   # 52
    ( 81,  81, 14, 14),   # 53
    ( 33,   1, 14, 14),   # 54
    ( 81,  96, 14, 16),   # 55
    ( 33,  80, 14, 15),   # 56
    ( 82,  32, 13, 15),   # 57
    ( 98,  32, 13, 16),   # 58
    (105,  17, 13, 13),   # 59
    (  1,  33, 14, 11),   # 60
    ( 97,  79, 15, 17),   # 61
    (101,  78,  5, 16),   # 62
    (105,  15, 13, 15),   # 63
    ( 18,  33, 12, 14),   # 64
    (114,   2, 13, 13),   # 65
    ( 16,  65, 16, 15),   # 66
    ( 49,  97, 14, 14),   # 67
    (114,  33, 12, 14),   # 68
    ( 65,  32, 13, 16),   # 69
    ( 34,  32, 12, 12),   # 70
    (  0,  32, 15, 13),   # 71
    ( 33,  97, 14, 14),   # 72
    ( 81,  97, 15, 14),   # 73
    (114,   2, 13, 11),   # 74
    ( 16,  96, 14, 15),   # 75
    ( 33,  34, 14, 15),   # 76
    ( 65,  64, 14, 17),   # 77
    ( 17,  65, 14, 12),   # 78
    (  1,  16, 14, 16),   # 79
    ( 49,  65, 14, 14),   # 80
    ( 17, 128, 15, 16),   # 81
    ( 18,  32, 13, 17),   # 82
    (  1,  95, 14, 16),   # 83
    ( 67,  96, 12, 15),   # 84
    ( 83,  81, 12, 14),   # 85
    (104,  17, 15, 13),   # 86
    ( 64,  15, 15, 14),   # 87
    ( 64,  64, 16, 17),   # 88
    ( 17,  64, 14, 17),   # 89
    ( 97,  33, 13, 14),   # 90
    ( 34,  32, 13, 16),   # 91
    (113,  32, 13, 16),   # 92
]

# The rest between routines, in ticks: measured over the twenty seven gaps the capture holds,
# whose median is exactly two seconds. They run from 0.93 to 2.30 s, so the character stands
# for this long and then is dealt another - which is the shape of it, not the spread.
REST = 120

# Which routine is the blink - played between the others rather than as one of them - and which
# one a buried character's is derived from.
FILLER = "blink"
DEFEAT_FROM = "flop"

# Which routines are one move with a direction, and which of them the rip only drew one way
# round. A `kind` is what the bag deals; the *direction* is chosen from where Kirby is standing,
# which is the whole point - a Kirby against the right post should walk left, not slide left and
# walk back.
#
# `jump`, `spin` and `tumble` have both ways in the game's own art. The other four travel and
# were only ever recorded going one way, so their twin is **mirrored** - the poses flipped and
# the offsets reflected about the routine's own opening frame. That is derived and not measured,
# and it is the only art here that is.
KINDS = {
    "yawn": "yawn", "flop": "flop", "blink": "blink",
    "walk": "walk", "ricochet": "ricochet", "climb": "climb", "float-up": "float-up",
    "spring": "spring", "takeoff": "takeoff",
    "jump-right": "jump", "jump-left": "jump",
    "spin-right": "spin", "spin-left": "spin",
    "tumble-right": "tumble", "tumble-left": "tumble",
}
MIRROR = ["walk", "ricochet", "spring", "takeoff"]

# Whether a routine he is out of position for may be walked into at all.
#
# Nothing is refused for being out of position, or the narrow windows would hardly ever be
# seen: `ricochet` throws him wall to wall and so has three pixels of window in a forty eight
# pixel arch. When it happens the translation goes on the routine's own first hop - see the
# `glide` each way carries - so it is never a slide along the floor.
APPROACH = "true"

# The arch is forty eight pixels across, which is what a routine's whole drawn path has to stay
# inside. Every one of the fifteen was captured being played from an origin that keeps it there
# - the game itself only plays a routine where it fits - so nothing is ever shifted to make one
# fit, and a routine simply is not dealt where it would not.
ARCH_WIDTH = 48

# How much a dealing varies a routine's pace, as a fraction either way. Not measured - the game
# plays each of these at one pace - and here because the same spin twice in a minute at exactly
# the same speed reads as a loop rather than as a character.
SPEED_SPREAD = 0.15

# One entry a routine: its name, the `CharacterState` row it is dealt on, what it is, and the
# frames.  A frame is (pose, how many 60 Hz ticks it is held, where its top left goes).
#
# A pose of `None` is Kirby off the top of the **recording** and nothing else - he is a sprite
# and the game draws him over every course of the centre column, right up past `STAGE`. Six
# capture frames of `ricochet` are the whole of it; every other routine is now tracked from
# its first frame to its last.
#
# The place is **relative to the frame the routine opens on**, and not to the box, because the
# order routines are dealt in is not the order the capture happened to play them: a routine
# that walks Kirby nineteen pixels to the right has to start wherever the last one left him.
# Every routine opens on a standing pose and every one lands back on the floor, so only the
# horizontal displacement is carried between them - and `takeoff` reaches seventy two pixels
# above the arch, which is twice the arch's own height.
#
# **Three of these are pairs**, and the pairing is the game's own: `jump`, `spin` and `tumble`
# each have a left and a right, drawn as separate sprites rather than as one flipped - the two
# tumbles' sprites are 0.91 to 0.98 mirrors of each other and not copies. They are one move
# with a direction, and which one gets dealt is decided by where Kirby is standing: against the
# right post, only the leftward one fits. The spins are the same *sprites* in the opposite
# order, which is how a spin reverses.
ROUTINES = [
    ("blink", "idle",
     "the idle blink, twice",
     [
         (   0,  10, (   0,   0)),
         (   7,   2, (   0,   0)),
         (   0,   8, (   0,   0)),
         (   7,   2, (   0,   0)),
         (   0,   8, (   0,   0)),
     ]),
    ("yawn", "idle",
     "squints, yawns wide, settles back",
     [
         (   0,  10, (   0,   0)),
         (   2,  20, (   0,   1)),
         (   5,  22, (   0,   1)),
         (  19,  18, (   0,   1)),
         (  16,  24, (   0,   1)),
         (   6,  40, (   0,   1)),
         (   5,  60, (   0,   1)),
         (  21,  26, (   0,   1)),
         (   0,  14, (   0,   0)),
     ]),
    ("walk", "idle",
     "turns his back and walks a few steps, then turns round again",
     [
         (   0,   6, (   0,   0)),
         (  54,   2, (   2,   0)),
         (  54,   4, (   3,   0)),
         (  54,   2, (   6,   0)),
         (  54,   4, (   8,   0)),
         (  54,   2, (  10,   0)),
         (  83,   2, (  13,  -1)),
         (  75,   2, (  13,  -1)),
         (  75,   4, (  15,  -1)),
         (  72,   2, (  17,   0)),
         (  72,   2, (  18,   0)),
         (  72,   2, (  19,   0)),
         (  67,   4, (  20,   0)),
         (  67,   4, (  21,   0)),
         (  84,   2, (  22,  -1)),
         (  55,  12, (  22,  -2)),
         (  55,   2, (  21,  -2)),
         (  73,   6, (  20,   0)),
         (   0,  10, (  20,   0)),
     ]),
    ("flop", "losing",
     "flops flat on his face and stays down",
     [
         (   0,  10, (   0,   0)),
         (   3,   6, (   0,   7)),
         (   4,  92, (   0,   0)),
         (   0,  18, (   0,   0)),
     ]),
    ("ricochet", "idle",
     "lies down, then ricochets wall to wall across the arch, squashing thin against each, before shooting off the top and dropping back",
     [
         (   7,   2, (   0,   0)),
         (   0,   8, (   0,   0)),
         (   3,  40, (   0,   7)),
         (   2,   2, (  -8,   0)),
         (  21,   2, (  -8,  -1)),
         (  10,   2, (  -4,  -7)),
         (  10,  10, (  -4,  -8)),
         (  37,   4, (  15,  -9)),
         (  20,  10, (  26, -14)),
         (   2,   4, (  -1, -14)),
         (  10,  10, (  -4, -19)),
         (  81,   2, (   8, -21)),
         (  81,   2, (  17, -23)),
         (   9,  10, (  26, -25)),
         (  21,   4, (  -1, -25)),
         (  10,  10, (  -4, -31)),
         (  37,   4, (  15, -32)),
         (   9,   8, (  26, -37)),
         (   2,   2, (  14, -37)),
         (   2,   4, (  -1, -37)),
         (  62,  10, (  -3, -44)),
         (  52,   2, (   0, -44)),
         (  52,   2, (  13, -46)),
         (   9,  12, (  27, -49)),
         (   2,   2, (  -2, -51)),
         (   9,  10, (  -4, -55)),
         (  52,   2, (   0, -55)),
         (  35,   2, (  17, -55)),
         (   9,  12, (  26, -62)),
         (None,  12, (  26, -62)),
         (  52,   4, (  16, -68)),
         (  52,   2, (  21, -70)),
         (  33,  34, (  26, -72)),
         (  33,   2, (  26, -71)),
         (  33,   2, (  26, -70)),
         (  33,   2, (  27, -69)),
         (   9,   4, (  27, -68)),
         (   9,   2, (  27, -65)),
         (   9,   2, (  27, -63)),
         (   9,   2, (  27, -62)),
         (  20,   2, (  27, -59)),
         (   9,   2, (  26, -55)),
         (  20,   4, (  26, -50)),
         (  20,   2, (  27, -46)),
         (  62,   4, (  28, -41)),
         (   9,   2, (  27, -33)),
         (  10,   2, (  27, -29)),
         (  10,   2, (  27, -25)),
         (  10,   2, (  27, -19)),
         (  27,   4, (  27, -10)),
         (  27,   2, (  27,  -1)),
         (  27,   2, (  27,   0)),
         (   3,   6, (  23,   7)),
         (   4,  62, (  23,   0)),
         (  37,   6, (  22,   0)),
     ]),
    ("climb", "winning",
     "turns to the left wall and climbs it hand over hand, out over the top, then drops back and lands flat",
     [
         (   0,  10, (   0,   0)),
         (  36,   8, (  -1,  -4)),
         (  14,   8, (   0,  -9)),
         (  24,  10, (   0, -12)),
         (  14,  10, (   0, -16)),
         (  11,   8, (   0, -21)),
         (  14,  10, (   0, -24)),
         (  24,   8, (   0, -27)),
         (  14,   8, (   0, -32)),
         (  36,  10, (  -1, -35)),
         (  43,  10, (   1, -42)),
         (  28,  10, (   0, -46)),
         (  38,   8, (   0, -48)),
         (  36,   2, (   0, -53)),
         (  36,   8, (  -1, -53)),
         (  49,   6, (   1, -57)),
         (  61,  10, (   0, -62)),
         (  38,   8, (   0, -64)),
         (  52,  12, (   0, -64)),
         (  45,   8, (   0, -64)),
         (  46,   2, (   0, -64)),
         (  45,   2, (   0, -61)),
         (  40,   4, (   0, -61)),
         (  66,   2, (   0, -56)),
         (  78,   4, (   0, -51)),
         (  78,   2, (   0, -48)),
         (  40,   4, (   0, -45)),
         (  13,   2, (   0, -41)),
         (  66,   2, (   0, -35)),
         (  22,   2, (   0, -29)),
         (  22,   2, (   0, -25)),
         (  22,   4, (   0, -21)),
         (  22,   2, (   0, -14)),
         (  22,   2, (   0,  -4)),
         (  35,   4, (   0,   1)),
         (   3,  12, (   0,   7)),
         (  37,  10, (  -1,   0)),
     ]),
    ("float-up", "winning",
     "flutters straight up out of the arch, feet kicking, and drops back",
     [
         (   0,   8, (   0,   0)),
         (   1,  10, (   0,   0)),
         (  18,   6, (   0,   0)),
         (  59,   4, (   0,  -1)),
         (  59,   2, (   0,  -5)),
         (  86,   2, (  -1,  -9)),
         (  12,   2, (  -1, -12)),
         (  25,   4, (  -1, -12)),
         (  25,   2, (  -1, -14)),
         (   8,   4, (  -1, -17)),
         (  31,   4, (  -2, -16)),
         (  31,   2, (  -2, -15)),
         (  31,   4, (  -2, -14)),
         (  12,   2, (  -1, -14)),
         (  12,   2, (  -1, -17)),
         (  12,   4, (  -1, -20)),
         (   1,   2, (   0, -23)),
         (   1,   2, (   0, -24)),
         (   1,   2, (   0, -25)),
         (  25,   2, (  -1, -26)),
         (  25,   4, (  -1, -27)),
         (  31,   2, (  -2, -27)),
         (  31,   2, (  -3, -27)),
         (  31,   2, (  -3, -26)),
         (  31,   4, (  -3, -24)),
         (  12,   2, (  -1, -29)),
         (  12,   2, (  -1, -31)),
         (  53,   4, (   0, -32)),
         (   8,   2, (   0, -36)),
         (   1,   2, (   0, -37)),
         (   1,   8, (   0, -38)),
         (  18,   4, (   0, -38)),
         (  18,   4, (   0, -36)),
         (  12,   2, (   0, -38)),
         (  63,   4, (   0, -44)),
         (  63,   2, (   0, -46)),
         (  25,   4, (   0, -47)),
         (  25,   2, (  -1, -49)),
         (  25,   2, (  -1, -50)),
         (  17,  12, (  -1, -52)),
         (  25,   2, (  -1, -50)),
         (  25,   2, (  -1, -49)),
         (  25,   4, (   0, -47)),
         (  25,   2, (  -1, -44)),
         (  87,   2, (  -1, -44)),
         (   8,   2, (   0, -42)),
         (   8,   2, (   0, -39)),
         (   8,   2, (   0, -35)),
         (   8,   2, (   0, -32)),
         (  25,   4, (  -1, -27)),
         (   1,   2, (   0, -22)),
         (   8,   2, (   0, -19)),
         (  25,   2, (  -1, -11)),
         (  25,   4, (  -1,  -7)),
         (   1,   2, (   0,   1)),
         (   3,   8, (   0,   7)),
         (   0,  10, (   0,   0)),
     ]),
    ("spring", "winning",
     "squashes right down and springs clean off the top of the arch",
     [
         (   7,   2, (   0,   0)),
         (  37,   8, (  -1,   0)),
         (   2,  12, (   0,   1)),
         (  26,  16, (   0,   7)),
         (  15,   4, (  -1,  -1)),
         (  15,   2, (  -3, -28)),
         (  15,   2, (  -5, -55)),
         (  15,   4, (  -7, -69)),
         (  88,   2, ( -11, -91)),
         (  88,   2, ( -12, -99)),
         (  88,   4, ( -14, -99)),
         (  77,   2, ( -15, -95)),
         (  77,   2, ( -17, -84)),
         (  77,   2, ( -18, -74)),
         (  39,   2, ( -20, -64)),
         (  39,   2, ( -22, -41)),
         (  80,   2, ( -23, -18)),
         (  74,   4, ( -24,   1)),
         (   3,   8, ( -25,   7)),
         (   2,  20, ( -25,   1)),
         (   0,  12, ( -25,   0)),
     ]),
    ("takeoff", "winning",
     "runs on the spot and then flies up and out, coming back down a moment later",
     [
         (   7,   4, (   0,   0)),
         (  37,   6, (  -1,   0)),
         (  51,  12, (   0,  -1)),
         (  68,   4, (   1,   0)),
         (  90,   2, (   0,   0)),
         (  69,   2, (   0,  -1)),
         (  48,   4, (   1,  -1)),
         (  76,   2, (   1,  -1)),
         (  64,   2, (   0,  -1)),
         (   1,   2, (  -1,  -3)),
         (  90,   2, (  -3,  -7)),
         (  57,   4, (  -3, -11)),
         (  69,   2, (  -5, -17)),
         (  48,   2, (  -5, -18)),
         (  91,   2, (  -6, -22)),
         (  25,   4, ( -10, -24)),
         (  92,   2, ( -10, -31)),
         (  57,   4, ( -11, -36)),
         (  69,   2, ( -13, -40)),
         (  70,   4, ( -14, -44)),
         (  38,   2, ( -16, -46)),
         (  68,   2, ( -17, -53)),
         (  58,   2, ( -18, -56)),
         (  32,   2, ( -19, -60)),
         (  69,   4, ( -21, -64)),
         (  38,   2, ( -22, -64)),
         (  82,   2, ( -23, -74)),
         (  87,   4, ( -25, -76)),
         (  87,   2, ( -26, -81)),
         (  60,   2, ( -27, -81)),
         (  42,   2, ( -27, -84)),
         (  87,   2, ( -28, -91)),
         (  17,   2, ( -28, -92)),
         (  17,   4, ( -28, -93)),
         (  17,   2, ( -28, -96)),
         (  25,   2, ( -28, -97)),
         (  25,   4, ( -28, -98)),
         (  25,   2, ( -28, -99)),
         (  25,   4, ( -28,-100)),
         (  25,   4, ( -28, -99)),
         (  25,   2, ( -28, -98)),
         (  25,   4, ( -28, -97)),
         (  17,   2, ( -28, -96)),
         (  17,   2, ( -28, -93)),
         (  71,   2, ( -28, -92)),
         (  71,   2, ( -28, -90)),
         (  17,   2, ( -28, -86)),
         (  42,   2, ( -27, -82)),
         (  60,   2, ( -27, -79)),
         (  42,   4, ( -27, -76)),
         (   8,   2, ( -27, -72)),
         (  25,   2, ( -27, -70)),
         (  60,   2, ( -27, -60)),
         (   1,   2, ( -27, -55)),
         (  25,   4, ( -27, -53)),
         (  71,   2, ( -27, -43)),
         (  42,   2, ( -27, -35)),
         (   8,   2, ( -27, -30)),
         (  25,   4, ( -28, -23)),
         (  51,   2, ( -27, -11)),
         (   8,   2, ( -27,  -5)),
         (   1,   4, ( -27,   0)),
         (   3,   6, ( -27,   7)),
         (   0,   6, ( -27,   0)),
     ]),
    ("jump-right", "winning",
     "squats and springs up to the right, landing flat",
     [
         (   0,  10, (   0,   0)),
         (   3,   4, (   0,   7)),
         (  35,   4, (   0,   1)),
         (  89,   2, (   1,  -7)),
         (  89,   4, (   4, -16)),
         (  66,   2, (   4, -21)),
         (  66,   2, (   5, -27)),
         (  66,   2, (   6, -29)),
         (  66,   2, (   6, -31)),
         (  66,   4, (   8, -32)),
         (  66,   2, (   9, -31)),
         (  66,   4, (  11, -27)),
         (  66,   2, (  13, -21)),
         (  22,   4, (  14, -16)),
         (  22,   2, (  15,  -8)),
         (  35,   2, (  17,   1)),
         (   3,   8, (  17,   7)),
         (   0,  14, (  17,   0)),
     ]),
    ("jump-left", "winning",
     "the same spring, to the left - the game draws both ways round",
     [
         (   0,   8, (   0,   0)),
         (   2,  14, (   0,   1)),
         (  26,   2, (   0,   1)),
         (  65,   4, (   1,   1)),
         (  39,   2, (  -2,  -5)),
         (  39,   2, (  -4, -14)),
         (  88,   2, (  -5, -20)),
         (  88,   4, (  -6, -25)),
         (  15,   2, (  -6, -29)),
         (  15,   4, (  -8, -32)),
         (  15,   2, (  -9, -32)),
         (  77,   4, ( -11, -32)),
         (  39,   2, ( -13, -25)),
         (  44,   2, ( -15, -18)),
         (  44,   4, ( -16, -14)),
         (  80,   2, ( -16,  -4)),
         (  65,   2, ( -16,   1)),
         (  26,   8, ( -17,   7)),
         (   2,  16, ( -17,   1)),
     ]),
    ("spin-right", "winning",
     "spins on the spot, twice round, travelling right",
     [
         (   0,  10, (   0,   0)),
         (  79,   2, (   1,  -2)),
         (  79,   2, (   2,  -2)),
         (  34,   2, (   3,  -1)),
         (  34,   4, (   4,  -1)),
         (  23,   2, (   6,  -1)),
         (  23,   4, (   8,  -1)),
         (  29,   2, (   9,  -1)),
         (  29,   2, (  10,  -1)),
         (  30,   4, (  11,   0)),
         (  30,   2, (  13,   0)),
         (  34,   2, (  14,  -1)),
         (  34,   2, (  15,  -1)),
         (  23,   4, (  16,  -1)),
         (  23,   2, (  18,  -1)),
         (  29,   6, (  20,  -1)),
         (   0,   2, (  20,   0)),
         (  37,  10, (  20,   0)),
     ]),
    ("spin-left", "winning",
     "the same spin the other way",
     [
         (   7,   2, (   0,   0)),
         (   0,   8, (   0,   0)),
         (  29,   4, (  -1,  -1)),
         (  29,   2, (  -2,  -1)),
         (  23,   2, (  -3,  -1)),
         (  23,   4, (  -5,  -1)),
         (  34,   2, (  -6,  -1)),
         (  34,   2, (  -7,  -1)),
         (  30,   4, (  -9,   0)),
         (  30,   2, ( -10,   0)),
         (  29,   2, ( -11,  -1)),
         (  23,   4, ( -13,  -1)),
         (  23,   2, ( -14,  -1)),
         (  34,   2, ( -16,  -1)),
         (  34,   2, ( -17,  -1)),
         (  34,   4, ( -18,  -1)),
         (  30,   2, ( -19,   0)),
         (  30,   2, ( -21,   0)),
         (   0,  10, ( -21,   0)),
     ]),
    ("tumble-right", "winning",
     "turns his back and tumbles along to the right, three times round",
     [
         (   0,  10, (   0,   0)),
         (  50,   4, (   1,   0)),
         (  41,   4, (   2,  -1)),
         (  41,   4, (   3,  -1)),
         (  56,   4, (   4,  -1)),
         (  50,   4, (   5,   0)),
         (  50,   2, (   6,   0)),
         (  41,   4, (   6,  -1)),
         (  41,   2, (   7,  -1)),
         (  56,   4, (   8,  -1)),
         (  56,   2, (   9,  -1)),
         (  50,   2, (   9,   0)),
         (  50,   4, (  10,   0)),
         (  41,   4, (  11,  -1)),
         (  41,   4, (  12,  -1)),
         (  56,   4, (  13,  -1)),
         (   0,  12, (  14,   0)),
     ]),
    ("tumble-left", "winning",
     "the same tumble the other way",
     [
         (   0,  10, (   0,   0)),
         (  53,   2, (  -1,   0)),
         (  53,   4, (  -2,   0)),
         (  46,   6, (  -2,  -1)),
         (  47,   4, (  -4,  -1)),
         (  53,   2, (  -5,   0)),
         (  53,   6, (  -6,   0)),
         (  46,   4, (  -6,  -1)),
         (  47,   2, (  -8,  -1)),
         (  47,   6, (  -9,  -1)),
         (  85,   2, (  -9,   0)),
         (  53,   2, ( -10,   0)),
         (  46,   8, ( -11,  -1)),
         (  47,   2, ( -13,  -1)),
         (  47,   4, ( -14,  -1)),
         (   2,  22, ( -14,   1)),
         (   0,   8, ( -14,   0)),
     ]),
]


# --------------------------------------------------------------------------------------


def frames_of(path, size):
    """the capture, decoded whole and cropped to the arch - about 400 MB, which is fine"""
    import subprocess
    width, height = size
    pipe = subprocess.Popen(
        ["ffmpeg", "-v", "error", "-i", path, "-f", "rawvideo", "-pix_fmt", "rgb24", "-"],
        stdout=subprocess.PIPE)
    x, y, w, h = ARCH_CROP
    out = []
    while True:
        raw = pipe.stdout.read(width * height * 3)
        if len(raw) < width * height * 3:
            break
        frame = np.frombuffer(raw, np.uint8).reshape(height, width, 3)
        out.append(frame[y:y + h, x:x + w].copy())
    return np.stack(out)


def largest_blob(mask):
    """the biggest connected run of the mask, 8-connected.

    The largest and not the whole mask, because the stage the capture was taken on has other
    things in the arch - a firefly, a shooting star - and a bounding box over both walks
    Kirby's measured place about.
    """
    height, width = mask.shape
    runs = []
    for y in range(height):
        row = mask[y]
        x = 0
        while x < width:
            if row[x]:
                start = x
                while x < width and row[x]:
                    x += 1
                runs.append((y, start, x - 1))
            else:
                x += 1
    if not runs:
        return None
    parent = list(range(len(runs)))

    def find(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i

    by_row = {}
    for index, (y, _, _) in enumerate(runs):
        by_row.setdefault(y, []).append(index)
    for y in sorted(by_row):
        for above in by_row.get(y - 1, []):
            _, ax0, ax1 = runs[above]
            for here in by_row[y]:
                _, bx0, bx1 = runs[here]
                if bx0 <= ax1 + 1 and ax0 <= bx1 + 1:
                    i, j = find(above), find(here)
                    if i != j:
                        parent[j] = i
    weight = {}
    for index, (_, x0, x1) in enumerate(runs):
        root = find(index)
        weight[root] = weight.get(root, 0) + (x1 - x0 + 1)
    best = max(weight, key=weight.get)
    out = np.zeros_like(mask)
    for index, (y, x0, x1) in enumerate(runs):
        if find(index) == best:
            out[y, x0:x1 + 1] = True
    return out


def plate_of(arch):
    """A picture of the arch with nobody in it: the median of the frames Kirby is out of.

    Bootstrapped by colour, which is good enough to say *whether* he is there and not good
    enough to say how wide he is.
    """
    present = []
    for frame in arch:
        pixels = frame.astype(int)
        mask = (pixels[:, :, 0] > 110) & (pixels[:, :, 0] - pixels[:, :, 1] > 40)
        present.append(denoise(mask).sum() >= 120)
    absent, run = [], []
    for index, there in enumerate(present):
        if not there:
            run.append(index)
        else:
            if len(run) >= 12:
                absent += run[3:-3]      # not the frames he is entering or leaving on
            run = []
    if len(run) >= 12:
        absent += run[3:-3]
    return np.median(arch[absent], axis=0).astype(np.uint8), len(absent)


def denoise(mask, neighbours=6):
    padded = np.pad(mask.astype(np.uint8), 1)
    count = sum(padded[1 + dy:1 + dy + mask.shape[0], 1 + dx:1 + dx + mask.shape[1]]
                for dy in (-1, 0, 1) for dx in (-1, 0, 1))
    return mask & (count >= neighbours)


def silhouettes(arch, plate):
    """Kirby per frame, by **background subtraction** and not by colour.

    A colour key - pink, and redder than it is green - drops his black outline and his shaded
    side, so a box over it is a pixel or three narrow. Subtraction against a plate of the empty
    arch keeps the whole of him, and the standing pose then measures fourteen native pixels on
    2512 frames of 3005 instead of wandering between ten and fifteen.
    """
    plate = plate.astype(np.int16)
    masks, boxes = [], []
    for frame in arch:
        diff = np.abs(frame.astype(np.int16) - plate).max(axis=-1)
        mask = denoise(diff > 80)
        blob = largest_blob(mask) if mask.sum() >= 80 else None
        if blob is None or blob.sum() < 80:
            masks.append(mask)
            boxes.append(None)
            continue
        ys, xs = np.where(blob)
        masks.append(blob)
        boxes.append((int(xs.min()), int(ys.min()), int(xs.max()), int(ys.max())))
    return np.stack(masks), boxes


def read():
    """Re-derive `SPRITES` from the capture and print what the free search settled on.

    The search is the expensive half - every frame against the rip at every offset and every
    size within a pixel of what the registration implies - and it is what the 0.94 in this
    file's docstring is. It prints the vocabulary; turning that into `ROUTINES` is the
    segmenting and the run-length encoding, which is written up in the docstring and is not
    worth the code to re-run, since the capture is one file and does not change.
    """
    from numpy.lib.stride_tricks import sliding_window_view as window

    sheet = Image.open(SHEET).convert("RGB")
    pixels = np.asarray(sheet, np.int16)
    solid = ~np.all(pixels == np.array(KEY), axis=-1)
    luma = np.where(solid, np.asarray(sheet.convert("L"), np.float32), 0.0)
    # The sprite region. The rip's last band of poses starts on row 129, so this cannot stop
    # at 130 - a fourteen row window over the row every standing frame lives in would hang four
    # rows past the edge and be thrown out, which is how a re-derivation once landed on a
    # different standing pose two thirds of the way up the sheet. What is off limits is the
    # tSR credits, which are the right hand side of the last band and nothing else.
    allowed = np.zeros_like(solid)
    allowed[0:130, :] = True
    allowed[130:176, 0:48] = True

    print("decoding the capture ...")
    arch = frames_of(CAPTURE, (224, 618))
    print("  %d frames" % len(arch))
    plate, clean = plate_of(arch)
    print("  %d of them with nobody in the arch, which is the plate" % clean)
    masks, boxes = silhouettes(arch, plate)
    print("  Kirby found on %d of them" % sum(1 for b in boxes if b))
    lintel = sum(1 for b in boxes if b and b[1] <= LINTEL)
    print("  %d of those clipped by the lintel, and so not a pose" % lintel)

    cache = {}

    def candidates(w, h):
        if (w, h) not in cache:
            flat = window(luma, (h, w)).reshape(176 - h + 1, 128 - w + 1, h * w)
            lit = window(solid, (h, w)).reshape(176 - h + 1, 128 - w + 1, h * w)
            ok = window(allowed, (h, w)).reshape(176 - h + 1, 128 - w + 1, h * w).all(axis=-1)
            grid = lit.reshape(lit.shape[0], lit.shape[1], h, w)
            # the window has to be the sprite's own box: content on all four edges
            tight = (grid[:, :, 0, :].any(-1) & grid[:, :, -1, :].any(-1)
                     & grid[:, :, :, 0].any(-1) & grid[:, :, :, -1].any(-1))
            centred = flat - flat.mean(-1, keepdims=True)
            cache[(w, h)] = (centred, np.linalg.norm(centred, axis=-1),
                             ok & tight, lit.mean(-1))
        return cache[(w, h)]

    found = Counter()
    scores = []
    for index, box in enumerate(boxes):
        if box is None or box[1] <= LINTEL:
            continue
        x0, y0, x1, y1 = box
        colour = arch[index][y0:y1 + 1, x0:x1 + 1].astype(np.float32)
        lit = masks[index][y0:y1 + 1, x0:x1 + 1]
        grey = (0.299 * colour[:, :, 0] + 0.587 * colour[:, :, 1]
                + 0.114 * colour[:, :, 2]) * lit
        grey = Image.fromarray(grey)
        alpha = Image.fromarray((lit * 255).astype(np.uint8))
        best = None
        for w in range(max(3, round((x1 - x0 + 1) / SCALE) - 1), round((x1 - x0 + 1) / SCALE) + 2):
            for h in range(max(3, round((y1 - y0 + 1) / SCALE) - 1),
                           round((y1 - y0 + 1) / SCALE) + 2):
                if w > 128 or h > 176:
                    continue
                centred, norm, ok, density = candidates(w, h)
                v = np.asarray(grey.resize((w, h), Image.BILINEAR), np.float32).ravel()
                a = (np.asarray(alpha.resize((w, h), Image.BILINEAR), np.float32) / 255.).mean()
                v = v - v.mean()
                scale = np.linalg.norm(v)
                if scale == 0:
                    continue
                c = (centred @ v) / (np.maximum(norm, 1e-6) * scale)
                c = np.where(ok & (np.abs(density - a) <= 0.18), c, -9.0)
                where = int(np.argmax(c))
                y, x = divmod(where, c.shape[1])
                if best is None or c.ravel()[where] > best[0]:
                    best = (float(c.ravel()[where]), x, y, w, h)
        if best is None or best[0] < -1.0:
            continue
        scores.append(best[0])
        found[best[1:]] += 1

    print("  %d matched, mean correlation %.3f" % (len(scores), sum(scores) / len(scores)))
    print()
    print("# the rects the free search settled on, most used first. Two of these are the same")
    print("# pose only if they agree to a pixel on every edge - grouping by overlap instead")
    print("# merges a pose with a sub-rect of itself and then cuts the wide one short.")
    print("# Compare with SPRITES above.")
    for rect, n in found.most_common(60):
        print("    (%3d, %3d, %2d, %2d),   # x%d" % (rect + (n,)))


def mirrored_routines():
    """The one-directional routines, reflected: `(name, kind, desc, frames)` per twin.

    A frame's place is relative to the routine's opening frame, so the reflection has to keep
    frame 0 at zero: a pose `w` wide drawn at `dx` reflects to `w0 - dx - w`, where `w0` is the
    opening pose's width. The pose itself is flipped, and the flipped ones are appended to
    `SPRITES` - which is why this returns the extra poses as well.
    """
    extra = []          # indices into SPRITES to flip, in the order they are appended
    out = []
    for name, kind, desc, frames in [(n, KINDS[n], d, f) for n, _, d, f in ROUTINES]:
        if name not in MIRROR:
            continue
        first = frames[0][0]
        w0 = SPRITES[first][2]
        flipped = []
        for pose, ticks, (dx, dy) in frames:
            if pose is None:
                flipped.append((None, ticks, (-dx, dy)))
                continue
            if pose not in extra:
                extra.append(pose)
            w = SPRITES[pose][2]
            flipped.append((len(SPRITES) + extra.index(pose), ticks, (w0 - dx - w, dy)))
        out.append((name + "-mirrored", kind, desc + " - the same, mirrored", flipped))
    return out, extra


def cut(out_dir):
    """Write `kirby.png`: every sprite of `SPRITES` and the flipped ones the mirrored routines
    need, keyed, on a readable grid.

    A grid and not a strip, because this art is addressed by *rect* rather than by counting
    frame widths from a strip's start - the frames are not one size, so counting cannot work -
    and a grid is the layout a person can check against the rip.
    """
    sheet = Image.open(SHEET).convert("RGBA")
    pixels = np.array(sheet)
    key = np.all(pixels[:, :, :3] == np.array(KEY), axis=-1)
    pixels[key] = (0, 0, 0, 0)
    sheet = Image.fromarray(pixels)

    _, extra = mirrored_routines()
    tiles = [(sheet.crop((x, y, x + w, y + h)), w, h) for x, y, w, h in SPRITES]
    for pose in extra:
        x, y, w, h = SPRITES[pose]
        tiles.append((sheet.crop((x, y, x + w, y + h)).transpose(Image.FLIP_LEFT_RIGHT), w, h))

    pitch = (max(w for _, w, _ in tiles) + 1, max(h for _, _, h in tiles) + 1)
    across = 10
    down = (len(tiles) + across - 1) // across
    page = Image.new("RGBA", (across * pitch[0], down * pitch[1]), (0, 0, 0, 0))
    rects = []
    for index, (tile, w, h) in enumerate(tiles):
        at = ((index % across) * pitch[0], (index // across) * pitch[1])
        page.paste(tile, at)
        rects.append((at[0], at[1], w, h))
    path = os.path.join(out_dir, "kirby.png")
    page.save(path)
    return path, page.size, rects


def rust(rects):
    """The tables to paste over `puyo-rusto/src/theme/snes/kirby.rs`."""
    twins, _ = mirrored_routines()
    every = [(n, KINDS[n], d, f) for n, _, d, f in ROUTINES] + twins

    def ident(name):
        return name.upper().replace("-", "_")

    def window(frames):
        left = min(dx for _, _, (dx, _) in frames)
        right = max(dx + rects[pose][2] for pose, _, (dx, _) in frames if pose is not None)
        return max(0, -left), ARCH_WIDTH - right

    def glide(frames):
        """When the character may be translated into position: the first stretch he is off the
        ground for, so a routine that starts by hopping is walked into **in the air**.

        Sliding along the floor to get somewhere is the one thing that reads as a glitch, and
        `ricochet` opens by lying down for two thirds of a second, which is exactly where a
        translation from tick zero would land.
        """
        tick = 0
        for pose, ticks, (_, dy) in frames:
            if pose is not None and dy < -2:
                return tick, ticks
            tick += ticks
        # never leaves the ground: walk it in over the opening frame, which is what walking is
        return 0, frames[0][1]

    reach = {}
    for name, _, _, frames in every:
        reach[name] = max(0, -min(dy for _, _, (_, dy) in frames))
    top = max(reach.values()) or 1

    out = []
    out.append("/// Every pose, as its rect in `kirby.png`. This script cuts the sheet and")
    out.append("/// prints the table, so the geometry is derived from the art rather than")
    out.append("/// typed out twice - the same bargain `mugshots.py` makes for the genesis cast.")
    out.append("///")
    out.append("/// The last few are **flipped copies**, for the routines the rip only drew one")
    out.append("/// way round. They are the one piece of derived art here.")
    out.append("const POSES: &[(i32, i32, u32, u32)] = &[")
    for index, (x, y, w, h) in enumerate(rects):
        out.append("    (%3d, %3d, %2d, %2d), // %d" % (x, y, w, h, index))
    out.append("];")
    out.append("")
    out.append("/// How long Kirby stands between routines, in 60 Hz ticks.")
    out.append("///")
    out.append("/// Measured over the twenty seven gaps the capture holds: the median is exactly")
    out.append("/// two seconds and they run from 0.93 to 2.30, so this is the shape of it and")
    out.append("/// not the spread.")
    out.append("const REST: Duration = Duration::from_millis(%d);" % (REST * 1000 // 60))
    out.append("")
    out.append("/// How much a dealing varies a routine's pace, either way.")
    out.append("const SPEED_SPREAD: f64 = %s;" % SPEED_SPREAD)
    out.append("")
    out.append("/// Whether a routine he is out of position for may be walked into.")
    out.append("///")
    out.append("/// Always: the narrow windows would hardly ever be seen otherwise. Where it")
    out.append("/// happens the translation rides the routine's own first hop.")
    out.append("const APPROACH: bool = %s;" % APPROACH)
    out.append("")

    for name, _, desc, frames in every:
        held = sum(t for _, t, _ in frames)
        gone = sum(t for pose, t, _ in frames if pose is None)
        out.append("/// %s%s" % (desc[0].upper(), desc[1:]))
        out.append("///")
        note = ", %.2f of them off the top of the recording" % (gone / 60.0) if gone else ""
        out.append("/// %d frames over %.2f seconds%s." % (len(frames), held / 60.0, note))
        out.append("const %s: Routine = &[" % ident(name))
        for pose, ticks, (dx, dy) in frames:
            at = "None" if pose is None else "Some(%d)" % pose
            out.append("    RoutineFrame { pose: %-8s ticks: %3d, at: (%3d, %3d) }," % (
                at + ",", ticks, dx, dy))
        out.append("];")
        out.append("")

    # ... and the derived one: the flop with its own recovery cut off and the last pose held
    flop = dict((n, f) for n, _, _, f in every)[DEFEAT_FROM]
    collapse = list(flop[:-1])
    collapse[-1] = (collapse[-1][0], 60 * 60, collapse[-1][2])
    out.append("/// The flop, with its own recovery cut off and the last pose held: a buried")
    out.append("/// player's Kirby stays down rather than picking himself up every two seconds.")
    out.append("///")
    out.append("/// The one routine here that is **derived** and not measured - every frame is")
    out.append("/// the capture's, but the game never showed a Kirby who had lost.")
    out.append("const COLLAPSE: Routine = &[")
    for pose, ticks, (dx, dy) in collapse:
        at = "None" if pose is None else "Some(%d)" % pose
        out.append("    RoutineFrame { pose: %-8s ticks: %3d, at: (%3d, %3d) }," % (
            at + ",", ticks, dx, dy))
    out.append("];")
    out.append("")

    out.append("/// Everything he may be dealt: one entry a **kind**, each with the ways round")
    out.append("/// it can be played.")
    out.append("///")
    out.append("/// The bag deals a *kind* and the **direction is chosen from where he is**, so")
    out.append("/// a Kirby against the right post walks left rather than sliding left to walk")
    out.append("/// back. `jump`, `spin` and `tumble` have both ways in the game's own art; the")
    out.append("/// four that travel and were only recorded one way have a mirrored twin.")
    out.append("///")
    out.append("/// The intensity is **vertical reach**, scaled so the biggest is one: a blink")
    out.append("/// and a spin are nothing much, a jump is something, and climbing clean out of")
    out.append("/// the arch is the most he does. It decides the order a bag comes out in.")
    out.append("const CHOICES: &[RoutineChoice] = &[")
    kinds = []
    for name, kind, _, _ in every:
        if name == FILLER:
            continue
        if kind not in kinds:
            kinds.append(kind)
    for kind in kinds:
        members = [(n, f) for n, k, _, f in every if k == kind and n != FILLER]
        out.append("    RoutineChoice {")
        out.append("        intensity: %.2f," % (max(reach[n] for n, _ in members) / top))
        out.append("        ways: &[")
        for n, frames in members:
            low, high = window(frames)
            assert low <= high, "%s fits nowhere in a %d wide arch" % (n, ARCH_WIDTH)
            at, span = glide(frames)
            out.append("            RoutineWay { frames: %s, origins: (%d, %d), glide: (%d, %d) }," % (
                ident(n), low, high, at, span))
        out.append("        ],")
        out.append("    },")
    out.append("];")
    out.append("")
    blink = dict((n, f) for n, _, _, f in every)[FILLER]
    low, high = window(blink)
    out.append("/// Played *between* routines and not as one of them, half the time.")
    out.append("///")
    out.append("/// The blink is a tic. Dealt like a routine it is a fraction of what he does")
    out.append("/// and reads as a loop; between them it reads as a character standing there.")
    out.append("const FILLER: RoutineWay = RoutineWay {")
    out.append("    frames: %s," % ident(FILLER))
    out.append("    origins: (%d, %d)," % (low, high))
    out.append("    glide: (0, 0),")
    out.append("};")
    return "\n".join(out)


if __name__ == "__main__":
    command = sys.argv[1] if len(sys.argv) > 1 else ""
    if command == "cut":
        out_dir = sys.argv[2] if len(sys.argv) > 2 else os.path.join(
            HERE, "..", "src", "theme", "snes")
        path, size, rects = cut(out_dir)
        print("%s  %dx%d  %d poses" % (os.path.relpath(path), size[0], size[1], len(rects)))
        print()
        print("// paste over the tables in puyo-rusto/src/theme/snes/kirby.rs")
        print(rust(rects))
        sys.exit(0)
    if command == "read":
        read()
        sys.exit(0)
    print(__doc__)
    sys.exit(2)
