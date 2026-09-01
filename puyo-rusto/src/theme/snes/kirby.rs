//! The Kirby that stands in the arch at the foot of the centre column, and his fifteen routines.
//!
//! This is the `snes` theme's answer to [`super::super::genesis::mugshots`], and the two are
//! shaped differently on purpose. Mean Bean Machine draws the *opponent's* face in a fixed box
//! at a constant rate, which is a strip. Kirby's Avalanche stands a whole little character in
//! the arch: he walks about it, he flops on his face, he spins, he inflates and floats clean
//! out of the arch for a second and a half, and his frames are not one size - 14x14 standing,
//! 14x7 in the pancake he lands in, 6x14 when he bobs up tall. So this declares
//! [`engine::render::character::RoutineArt`] instead: poses addressed by rect, and routines
//! that name a pose, how long it is held and where it goes.
//!
//! **Every number below is measured**, off a 115 second capture of the emulated game, and
//! `puyo-rusto/art/kirby.py` is what measured them - it registers the capture against the
//! game's own screen, finds Kirby by colour, matches the crop against the rip at every offset,
//! and prints this file's tables. The one thing it cannot measure is which routine belongs on
//! which row: nothing in the capture ties one to anything happening on the board, and the game
//! gives no sign that anything does. So the rows are this game's reading, and within a row the
//! routine is dealt - the way [`crate::game::cell::PuyoSkin`] is, and a track, and a face.
//!
//! There is a cast of one, which the deal handles already: a theme with one character hands it
//! to everybody, and in a two player game the left one is drawn mirrored so each Kirby faces
//! the other player's board.

use engine::animate::character::{Routine, RoutineChoice, RoutineFrame, RoutineWay};
use engine::animate::frames::FrameAnimationType;
use engine::render::character::{CharacterData, CharacterSetData, RoutineArt};
use std::time::Duration;

mod sprites {
    pub const KIRBY: &[u8] = include_bytes!("kirby.png");
}

/// Every pose, as its rect in `kirby.png`. This script cuts the sheet and
/// prints the table, so the geometry is derived from the art rather than
/// typed out twice - the same bargain `mugshots.py` makes for the genesis cast.
///
/// The last few are **flipped copies**, for the routines the rip only drew one
/// way round. They are the one piece of derived art here.
const POSES: &[(i32, i32, u32, u32)] = &[
    (0, 0, 14, 14),     // 0
    (17, 0, 13, 14),    // 1
    (34, 0, 15, 13),    // 2
    (51, 0, 14, 7),     // 3
    (68, 0, 14, 14),    // 4
    (85, 0, 14, 13),    // 5
    (102, 0, 14, 13),   // 6
    (119, 0, 14, 14),   // 7
    (136, 0, 14, 16),   // 8
    (153, 0, 8, 15),    // 9
    (0, 18, 7, 16),     // 10
    (17, 18, 14, 16),   // 11
    (34, 18, 15, 15),   // 12
    (51, 18, 14, 15),   // 13
    (68, 18, 14, 13),   // 14
    (85, 18, 14, 15),   // 15
    (102, 18, 14, 13),  // 16
    (119, 18, 15, 12),  // 17
    (136, 18, 13, 11),  // 18
    (153, 18, 15, 13),  // 19
    (0, 36, 8, 16),     // 20
    (17, 36, 15, 13),   // 21
    (34, 36, 15, 15),   // 22
    (51, 36, 15, 15),   // 23
    (68, 36, 15, 14),   // 24
    (85, 36, 15, 14),   // 25
    (102, 36, 15, 7),   // 26
    (119, 36, 7, 14),   // 27
    (136, 36, 15, 15),  // 28
    (153, 36, 15, 15),  // 29
    (0, 54, 15, 14),    // 30
    (17, 54, 16, 12),   // 31
    (34, 54, 13, 13),   // 32
    (51, 54, 7, 15),    // 33
    (68, 54, 14, 15),   // 34
    (85, 54, 13, 13),   // 35
    (102, 54, 15, 14),  // 36
    (119, 54, 16, 14),  // 37
    (136, 54, 15, 12),  // 38
    (153, 54, 16, 15),  // 39
    (0, 72, 16, 15),    // 40
    (17, 72, 13, 15),   // 41
    (34, 72, 13, 11),   // 42
    (51, 72, 13, 13),   // 43
    (68, 72, 16, 15),   // 44
    (85, 72, 16, 12),   // 45
    (102, 72, 13, 15),  // 46
    (119, 72, 14, 15),  // 47
    (136, 72, 12, 15),  // 48
    (153, 72, 12, 14),  // 49
    (0, 90, 14, 14),    // 50
    (17, 90, 13, 15),   // 51
    (34, 90, 15, 12),   // 52
    (51, 90, 14, 14),   // 53
    (68, 90, 14, 14),   // 54
    (85, 90, 14, 16),   // 55
    (102, 90, 14, 15),  // 56
    (119, 90, 13, 15),  // 57
    (136, 90, 13, 16),  // 58
    (153, 90, 13, 13),  // 59
    (0, 108, 14, 11),   // 60
    (17, 108, 15, 17),  // 61
    (34, 108, 5, 16),   // 62
    (51, 108, 13, 15),  // 63
    (68, 108, 12, 14),  // 64
    (85, 108, 13, 13),  // 65
    (102, 108, 16, 15), // 66
    (119, 108, 14, 14), // 67
    (136, 108, 12, 14), // 68
    (153, 108, 13, 16), // 69
    (0, 126, 12, 12),   // 70
    (17, 126, 15, 13),  // 71
    (34, 126, 14, 14),  // 72
    (51, 126, 15, 14),  // 73
    (68, 126, 13, 11),  // 74
    (85, 126, 14, 15),  // 75
    (102, 126, 14, 15), // 76
    (119, 126, 14, 17), // 77
    (136, 126, 14, 12), // 78
    (153, 126, 14, 16), // 79
    (0, 144, 14, 14),   // 80
    (17, 144, 15, 16),  // 81
    (34, 144, 13, 17),  // 82
    (51, 144, 14, 16),  // 83
    (68, 144, 12, 15),  // 84
    (85, 144, 12, 14),  // 85
    (102, 144, 15, 13), // 86
    (119, 144, 15, 14), // 87
    (136, 144, 16, 17), // 88
    (153, 144, 14, 17), // 89
    (0, 162, 13, 14),   // 90
    (17, 162, 13, 16),  // 91
    (34, 162, 13, 16),  // 92
    (51, 162, 14, 14),  // 93
    (68, 162, 14, 14),  // 94
    (85, 162, 14, 16),  // 95
    (102, 162, 14, 15), // 96
    (119, 162, 14, 14), // 97
    (136, 162, 14, 14), // 98
    (153, 162, 12, 15), // 99
    (0, 180, 14, 16),   // 100
    (17, 180, 15, 14),  // 101
    (34, 180, 14, 14),  // 102
    (51, 180, 14, 7),   // 103
    (68, 180, 15, 13),  // 104
    (85, 180, 15, 13),  // 105
    (102, 180, 7, 16),  // 106
    (119, 180, 16, 14), // 107
    (136, 180, 8, 16),  // 108
    (153, 180, 15, 16), // 109
    (0, 198, 8, 15),    // 110
    (17, 198, 5, 16),   // 111
    (34, 198, 15, 12),  // 112
    (51, 198, 13, 13),  // 113
    (68, 198, 7, 15),   // 114
    (85, 198, 7, 14),   // 115
    (102, 198, 14, 14), // 116
    (119, 198, 15, 7),  // 117
    (136, 198, 14, 15), // 118
    (153, 198, 16, 17), // 119
    (0, 216, 14, 17),   // 120
    (17, 216, 16, 15),  // 121
    (34, 216, 14, 14),  // 122
    (51, 216, 13, 11),  // 123
    (68, 216, 13, 15),  // 124
    (85, 216, 12, 14),  // 125
    (102, 216, 13, 14), // 126
    (119, 216, 13, 16), // 127
    (136, 216, 12, 15), // 128
    (153, 216, 14, 15), // 129
    (0, 234, 12, 14),   // 130
    (17, 234, 13, 14),  // 131
    (34, 234, 13, 15),  // 132
    (51, 234, 13, 16),  // 133
    (68, 234, 15, 14),  // 134
    (85, 234, 13, 16),  // 135
    (102, 234, 12, 12), // 136
    (119, 234, 15, 12), // 137
    (136, 234, 13, 16), // 138
    (153, 234, 13, 13), // 139
    (0, 252, 13, 17),   // 140
    (17, 252, 15, 14),  // 141
    (34, 252, 14, 11),  // 142
    (51, 252, 13, 11),  // 143
    (68, 252, 15, 12),  // 144
    (85, 252, 15, 13),  // 145
    (102, 252, 14, 16), // 146
];

/// How long Kirby stands between routines, in 60 Hz ticks.
///
/// Measured over the twenty seven gaps the capture holds: the median is exactly
/// two seconds and they run from 0.93 to 2.30, so this is the shape of it and
/// not the spread.
const REST: Duration = Duration::from_millis(2000);

/// How much a dealing varies a routine's pace, either way.
const SPEED_SPREAD: f64 = 0.15;

/// Whether a routine he is out of position for may be walked into.
///
/// Always: the narrow windows would hardly ever be seen otherwise. Where it
/// happens the translation rides the routine's own first hop.
const APPROACH: bool = true;

/// The idle blink, twice
///
/// 5 frames over 0.50 seconds.
const BLINK: Routine = &[
    RoutineFrame {
        pose: Some(0),
        ticks: 10,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(7),
        ticks: 2,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(0),
        ticks: 8,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(7),
        ticks: 2,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(0),
        ticks: 8,
        at: (0, 0),
    },
];

/// Squints, yawns wide, settles back
///
/// 9 frames over 3.90 seconds.
const YAWN: Routine = &[
    RoutineFrame {
        pose: Some(0),
        ticks: 10,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(2),
        ticks: 20,
        at: (0, 1),
    },
    RoutineFrame {
        pose: Some(5),
        ticks: 22,
        at: (0, 1),
    },
    RoutineFrame {
        pose: Some(19),
        ticks: 18,
        at: (0, 1),
    },
    RoutineFrame {
        pose: Some(16),
        ticks: 24,
        at: (0, 1),
    },
    RoutineFrame {
        pose: Some(6),
        ticks: 40,
        at: (0, 1),
    },
    RoutineFrame {
        pose: Some(5),
        ticks: 60,
        at: (0, 1),
    },
    RoutineFrame {
        pose: Some(21),
        ticks: 26,
        at: (0, 1),
    },
    RoutineFrame {
        pose: Some(0),
        ticks: 14,
        at: (0, 0),
    },
];

/// Turns his back and walks a few steps, then turns round again
///
/// 19 frames over 1.23 seconds.
const WALK: Routine = &[
    RoutineFrame {
        pose: Some(0),
        ticks: 6,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(54),
        ticks: 2,
        at: (2, 0),
    },
    RoutineFrame {
        pose: Some(54),
        ticks: 4,
        at: (3, 0),
    },
    RoutineFrame {
        pose: Some(54),
        ticks: 2,
        at: (6, 0),
    },
    RoutineFrame {
        pose: Some(54),
        ticks: 4,
        at: (8, 0),
    },
    RoutineFrame {
        pose: Some(54),
        ticks: 2,
        at: (10, 0),
    },
    RoutineFrame {
        pose: Some(83),
        ticks: 2,
        at: (13, -1),
    },
    RoutineFrame {
        pose: Some(75),
        ticks: 2,
        at: (13, -1),
    },
    RoutineFrame {
        pose: Some(75),
        ticks: 4,
        at: (15, -1),
    },
    RoutineFrame {
        pose: Some(72),
        ticks: 2,
        at: (17, 0),
    },
    RoutineFrame {
        pose: Some(72),
        ticks: 2,
        at: (18, 0),
    },
    RoutineFrame {
        pose: Some(72),
        ticks: 2,
        at: (19, 0),
    },
    RoutineFrame {
        pose: Some(67),
        ticks: 4,
        at: (20, 0),
    },
    RoutineFrame {
        pose: Some(67),
        ticks: 4,
        at: (21, 0),
    },
    RoutineFrame {
        pose: Some(84),
        ticks: 2,
        at: (22, -1),
    },
    RoutineFrame {
        pose: Some(55),
        ticks: 12,
        at: (22, -2),
    },
    RoutineFrame {
        pose: Some(55),
        ticks: 2,
        at: (21, -2),
    },
    RoutineFrame {
        pose: Some(73),
        ticks: 6,
        at: (20, 0),
    },
    RoutineFrame {
        pose: Some(0),
        ticks: 10,
        at: (20, 0),
    },
];

/// Flops flat on his face and stays down
///
/// 4 frames over 2.10 seconds.
const FLOP: Routine = &[
    RoutineFrame {
        pose: Some(0),
        ticks: 10,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(3),
        ticks: 6,
        at: (0, 7),
    },
    RoutineFrame {
        pose: Some(4),
        ticks: 92,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(0),
        ticks: 18,
        at: (0, 0),
    },
];

/// Lies down, then ricochets wall to wall across the arch, squashing thin against each, before shooting off the top and dropping back
///
/// 55 frames over 6.10 seconds, 0.20 of them off the top of the recording.
const RICOCHET: Routine = &[
    RoutineFrame {
        pose: Some(7),
        ticks: 2,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(0),
        ticks: 8,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(3),
        ticks: 40,
        at: (0, 7),
    },
    RoutineFrame {
        pose: Some(2),
        ticks: 2,
        at: (-8, 0),
    },
    RoutineFrame {
        pose: Some(21),
        ticks: 2,
        at: (-8, -1),
    },
    RoutineFrame {
        pose: Some(10),
        ticks: 2,
        at: (-4, -7),
    },
    RoutineFrame {
        pose: Some(10),
        ticks: 10,
        at: (-4, -8),
    },
    RoutineFrame {
        pose: Some(37),
        ticks: 4,
        at: (15, -9),
    },
    RoutineFrame {
        pose: Some(20),
        ticks: 10,
        at: (26, -14),
    },
    RoutineFrame {
        pose: Some(2),
        ticks: 4,
        at: (-1, -14),
    },
    RoutineFrame {
        pose: Some(10),
        ticks: 10,
        at: (-4, -19),
    },
    RoutineFrame {
        pose: Some(81),
        ticks: 2,
        at: (8, -21),
    },
    RoutineFrame {
        pose: Some(81),
        ticks: 2,
        at: (17, -23),
    },
    RoutineFrame {
        pose: Some(9),
        ticks: 10,
        at: (26, -25),
    },
    RoutineFrame {
        pose: Some(21),
        ticks: 4,
        at: (-1, -25),
    },
    RoutineFrame {
        pose: Some(10),
        ticks: 10,
        at: (-4, -31),
    },
    RoutineFrame {
        pose: Some(37),
        ticks: 4,
        at: (15, -32),
    },
    RoutineFrame {
        pose: Some(9),
        ticks: 8,
        at: (26, -37),
    },
    RoutineFrame {
        pose: Some(2),
        ticks: 2,
        at: (14, -37),
    },
    RoutineFrame {
        pose: Some(2),
        ticks: 4,
        at: (-1, -37),
    },
    RoutineFrame {
        pose: Some(62),
        ticks: 10,
        at: (-3, -44),
    },
    RoutineFrame {
        pose: Some(52),
        ticks: 2,
        at: (0, -44),
    },
    RoutineFrame {
        pose: Some(52),
        ticks: 2,
        at: (13, -46),
    },
    RoutineFrame {
        pose: Some(9),
        ticks: 12,
        at: (27, -49),
    },
    RoutineFrame {
        pose: Some(2),
        ticks: 2,
        at: (-2, -51),
    },
    RoutineFrame {
        pose: Some(9),
        ticks: 10,
        at: (-4, -55),
    },
    RoutineFrame {
        pose: Some(52),
        ticks: 2,
        at: (0, -55),
    },
    RoutineFrame {
        pose: Some(35),
        ticks: 2,
        at: (17, -55),
    },
    RoutineFrame {
        pose: Some(9),
        ticks: 12,
        at: (26, -62),
    },
    RoutineFrame {
        pose: None,
        ticks: 12,
        at: (26, -62),
    },
    RoutineFrame {
        pose: Some(52),
        ticks: 4,
        at: (16, -68),
    },
    RoutineFrame {
        pose: Some(52),
        ticks: 2,
        at: (21, -70),
    },
    RoutineFrame {
        pose: Some(33),
        ticks: 34,
        at: (26, -72),
    },
    RoutineFrame {
        pose: Some(33),
        ticks: 2,
        at: (26, -71),
    },
    RoutineFrame {
        pose: Some(33),
        ticks: 2,
        at: (26, -70),
    },
    RoutineFrame {
        pose: Some(33),
        ticks: 2,
        at: (27, -69),
    },
    RoutineFrame {
        pose: Some(9),
        ticks: 4,
        at: (27, -68),
    },
    RoutineFrame {
        pose: Some(9),
        ticks: 2,
        at: (27, -65),
    },
    RoutineFrame {
        pose: Some(9),
        ticks: 2,
        at: (27, -63),
    },
    RoutineFrame {
        pose: Some(9),
        ticks: 2,
        at: (27, -62),
    },
    RoutineFrame {
        pose: Some(20),
        ticks: 2,
        at: (27, -59),
    },
    RoutineFrame {
        pose: Some(9),
        ticks: 2,
        at: (26, -55),
    },
    RoutineFrame {
        pose: Some(20),
        ticks: 4,
        at: (26, -50),
    },
    RoutineFrame {
        pose: Some(20),
        ticks: 2,
        at: (27, -46),
    },
    RoutineFrame {
        pose: Some(62),
        ticks: 4,
        at: (28, -41),
    },
    RoutineFrame {
        pose: Some(9),
        ticks: 2,
        at: (27, -33),
    },
    RoutineFrame {
        pose: Some(10),
        ticks: 2,
        at: (27, -29),
    },
    RoutineFrame {
        pose: Some(10),
        ticks: 2,
        at: (27, -25),
    },
    RoutineFrame {
        pose: Some(10),
        ticks: 2,
        at: (27, -19),
    },
    RoutineFrame {
        pose: Some(27),
        ticks: 4,
        at: (27, -10),
    },
    RoutineFrame {
        pose: Some(27),
        ticks: 2,
        at: (27, -1),
    },
    RoutineFrame {
        pose: Some(27),
        ticks: 2,
        at: (27, 0),
    },
    RoutineFrame {
        pose: Some(3),
        ticks: 6,
        at: (23, 7),
    },
    RoutineFrame {
        pose: Some(4),
        ticks: 62,
        at: (23, 0),
    },
    RoutineFrame {
        pose: Some(37),
        ticks: 6,
        at: (22, 0),
    },
];

/// Turns to the left wall and climbs it hand over hand, out over the top, then drops back and lands flat
///
/// 37 frames over 3.90 seconds.
const CLIMB: Routine = &[
    RoutineFrame {
        pose: Some(0),
        ticks: 10,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(36),
        ticks: 8,
        at: (-1, -4),
    },
    RoutineFrame {
        pose: Some(14),
        ticks: 8,
        at: (0, -9),
    },
    RoutineFrame {
        pose: Some(24),
        ticks: 10,
        at: (0, -12),
    },
    RoutineFrame {
        pose: Some(14),
        ticks: 10,
        at: (0, -16),
    },
    RoutineFrame {
        pose: Some(11),
        ticks: 8,
        at: (0, -21),
    },
    RoutineFrame {
        pose: Some(14),
        ticks: 10,
        at: (0, -24),
    },
    RoutineFrame {
        pose: Some(24),
        ticks: 8,
        at: (0, -27),
    },
    RoutineFrame {
        pose: Some(14),
        ticks: 8,
        at: (0, -32),
    },
    RoutineFrame {
        pose: Some(36),
        ticks: 10,
        at: (-1, -35),
    },
    RoutineFrame {
        pose: Some(43),
        ticks: 10,
        at: (1, -42),
    },
    RoutineFrame {
        pose: Some(28),
        ticks: 10,
        at: (0, -46),
    },
    RoutineFrame {
        pose: Some(38),
        ticks: 8,
        at: (0, -48),
    },
    RoutineFrame {
        pose: Some(36),
        ticks: 2,
        at: (0, -53),
    },
    RoutineFrame {
        pose: Some(36),
        ticks: 8,
        at: (-1, -53),
    },
    RoutineFrame {
        pose: Some(49),
        ticks: 6,
        at: (1, -57),
    },
    RoutineFrame {
        pose: Some(61),
        ticks: 10,
        at: (0, -62),
    },
    RoutineFrame {
        pose: Some(38),
        ticks: 8,
        at: (0, -64),
    },
    RoutineFrame {
        pose: Some(52),
        ticks: 12,
        at: (0, -64),
    },
    RoutineFrame {
        pose: Some(45),
        ticks: 8,
        at: (0, -64),
    },
    RoutineFrame {
        pose: Some(46),
        ticks: 2,
        at: (0, -64),
    },
    RoutineFrame {
        pose: Some(45),
        ticks: 2,
        at: (0, -61),
    },
    RoutineFrame {
        pose: Some(40),
        ticks: 4,
        at: (0, -61),
    },
    RoutineFrame {
        pose: Some(66),
        ticks: 2,
        at: (0, -56),
    },
    RoutineFrame {
        pose: Some(78),
        ticks: 4,
        at: (0, -51),
    },
    RoutineFrame {
        pose: Some(78),
        ticks: 2,
        at: (0, -48),
    },
    RoutineFrame {
        pose: Some(40),
        ticks: 4,
        at: (0, -45),
    },
    RoutineFrame {
        pose: Some(13),
        ticks: 2,
        at: (0, -41),
    },
    RoutineFrame {
        pose: Some(66),
        ticks: 2,
        at: (0, -35),
    },
    RoutineFrame {
        pose: Some(22),
        ticks: 2,
        at: (0, -29),
    },
    RoutineFrame {
        pose: Some(22),
        ticks: 2,
        at: (0, -25),
    },
    RoutineFrame {
        pose: Some(22),
        ticks: 4,
        at: (0, -21),
    },
    RoutineFrame {
        pose: Some(22),
        ticks: 2,
        at: (0, -14),
    },
    RoutineFrame {
        pose: Some(22),
        ticks: 2,
        at: (0, -4),
    },
    RoutineFrame {
        pose: Some(35),
        ticks: 4,
        at: (0, 1),
    },
    RoutineFrame {
        pose: Some(3),
        ticks: 12,
        at: (0, 7),
    },
    RoutineFrame {
        pose: Some(37),
        ticks: 10,
        at: (-1, 0),
    },
];

/// Flutters straight up out of the arch, feet kicking, and drops back
///
/// 57 frames over 3.23 seconds.
const FLOAT_UP: Routine = &[
    RoutineFrame {
        pose: Some(0),
        ticks: 8,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(1),
        ticks: 10,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(18),
        ticks: 6,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(59),
        ticks: 4,
        at: (0, -1),
    },
    RoutineFrame {
        pose: Some(59),
        ticks: 2,
        at: (0, -5),
    },
    RoutineFrame {
        pose: Some(86),
        ticks: 2,
        at: (-1, -9),
    },
    RoutineFrame {
        pose: Some(12),
        ticks: 2,
        at: (-1, -12),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 4,
        at: (-1, -12),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 2,
        at: (-1, -14),
    },
    RoutineFrame {
        pose: Some(8),
        ticks: 4,
        at: (-1, -17),
    },
    RoutineFrame {
        pose: Some(31),
        ticks: 4,
        at: (-2, -16),
    },
    RoutineFrame {
        pose: Some(31),
        ticks: 2,
        at: (-2, -15),
    },
    RoutineFrame {
        pose: Some(31),
        ticks: 4,
        at: (-2, -14),
    },
    RoutineFrame {
        pose: Some(12),
        ticks: 2,
        at: (-1, -14),
    },
    RoutineFrame {
        pose: Some(12),
        ticks: 2,
        at: (-1, -17),
    },
    RoutineFrame {
        pose: Some(12),
        ticks: 4,
        at: (-1, -20),
    },
    RoutineFrame {
        pose: Some(1),
        ticks: 2,
        at: (0, -23),
    },
    RoutineFrame {
        pose: Some(1),
        ticks: 2,
        at: (0, -24),
    },
    RoutineFrame {
        pose: Some(1),
        ticks: 2,
        at: (0, -25),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 2,
        at: (-1, -26),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 4,
        at: (-1, -27),
    },
    RoutineFrame {
        pose: Some(31),
        ticks: 2,
        at: (-2, -27),
    },
    RoutineFrame {
        pose: Some(31),
        ticks: 2,
        at: (-3, -27),
    },
    RoutineFrame {
        pose: Some(31),
        ticks: 2,
        at: (-3, -26),
    },
    RoutineFrame {
        pose: Some(31),
        ticks: 4,
        at: (-3, -24),
    },
    RoutineFrame {
        pose: Some(12),
        ticks: 2,
        at: (-1, -29),
    },
    RoutineFrame {
        pose: Some(12),
        ticks: 2,
        at: (-1, -31),
    },
    RoutineFrame {
        pose: Some(53),
        ticks: 4,
        at: (0, -32),
    },
    RoutineFrame {
        pose: Some(8),
        ticks: 2,
        at: (0, -36),
    },
    RoutineFrame {
        pose: Some(1),
        ticks: 2,
        at: (0, -37),
    },
    RoutineFrame {
        pose: Some(1),
        ticks: 8,
        at: (0, -38),
    },
    RoutineFrame {
        pose: Some(18),
        ticks: 4,
        at: (0, -38),
    },
    RoutineFrame {
        pose: Some(18),
        ticks: 4,
        at: (0, -36),
    },
    RoutineFrame {
        pose: Some(12),
        ticks: 2,
        at: (0, -38),
    },
    RoutineFrame {
        pose: Some(63),
        ticks: 4,
        at: (0, -44),
    },
    RoutineFrame {
        pose: Some(63),
        ticks: 2,
        at: (0, -46),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 4,
        at: (0, -47),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 2,
        at: (-1, -49),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 2,
        at: (-1, -50),
    },
    RoutineFrame {
        pose: Some(17),
        ticks: 12,
        at: (-1, -52),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 2,
        at: (-1, -50),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 2,
        at: (-1, -49),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 4,
        at: (0, -47),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 2,
        at: (-1, -44),
    },
    RoutineFrame {
        pose: Some(87),
        ticks: 2,
        at: (-1, -44),
    },
    RoutineFrame {
        pose: Some(8),
        ticks: 2,
        at: (0, -42),
    },
    RoutineFrame {
        pose: Some(8),
        ticks: 2,
        at: (0, -39),
    },
    RoutineFrame {
        pose: Some(8),
        ticks: 2,
        at: (0, -35),
    },
    RoutineFrame {
        pose: Some(8),
        ticks: 2,
        at: (0, -32),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 4,
        at: (-1, -27),
    },
    RoutineFrame {
        pose: Some(1),
        ticks: 2,
        at: (0, -22),
    },
    RoutineFrame {
        pose: Some(8),
        ticks: 2,
        at: (0, -19),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 2,
        at: (-1, -11),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 4,
        at: (-1, -7),
    },
    RoutineFrame {
        pose: Some(1),
        ticks: 2,
        at: (0, 1),
    },
    RoutineFrame {
        pose: Some(3),
        ticks: 8,
        at: (0, 7),
    },
    RoutineFrame {
        pose: Some(0),
        ticks: 10,
        at: (0, 0),
    },
];

/// Squashes right down and springs clean off the top of the arch
///
/// 21 frames over 1.90 seconds.
const SPRING: Routine = &[
    RoutineFrame {
        pose: Some(7),
        ticks: 2,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(37),
        ticks: 8,
        at: (-1, 0),
    },
    RoutineFrame {
        pose: Some(2),
        ticks: 12,
        at: (0, 1),
    },
    RoutineFrame {
        pose: Some(26),
        ticks: 16,
        at: (0, 7),
    },
    RoutineFrame {
        pose: Some(15),
        ticks: 4,
        at: (-1, -1),
    },
    RoutineFrame {
        pose: Some(15),
        ticks: 2,
        at: (-3, -28),
    },
    RoutineFrame {
        pose: Some(15),
        ticks: 2,
        at: (-5, -55),
    },
    RoutineFrame {
        pose: Some(15),
        ticks: 4,
        at: (-7, -69),
    },
    RoutineFrame {
        pose: Some(88),
        ticks: 2,
        at: (-11, -91),
    },
    RoutineFrame {
        pose: Some(88),
        ticks: 2,
        at: (-12, -99),
    },
    RoutineFrame {
        pose: Some(88),
        ticks: 4,
        at: (-14, -99),
    },
    RoutineFrame {
        pose: Some(77),
        ticks: 2,
        at: (-15, -95),
    },
    RoutineFrame {
        pose: Some(77),
        ticks: 2,
        at: (-17, -84),
    },
    RoutineFrame {
        pose: Some(77),
        ticks: 2,
        at: (-18, -74),
    },
    RoutineFrame {
        pose: Some(39),
        ticks: 2,
        at: (-20, -64),
    },
    RoutineFrame {
        pose: Some(39),
        ticks: 2,
        at: (-22, -41),
    },
    RoutineFrame {
        pose: Some(80),
        ticks: 2,
        at: (-23, -18),
    },
    RoutineFrame {
        pose: Some(74),
        ticks: 4,
        at: (-24, 1),
    },
    RoutineFrame {
        pose: Some(3),
        ticks: 8,
        at: (-25, 7),
    },
    RoutineFrame {
        pose: Some(2),
        ticks: 20,
        at: (-25, 1),
    },
    RoutineFrame {
        pose: Some(0),
        ticks: 12,
        at: (-25, 0),
    },
];

/// Runs on the spot and then flies up and out, coming back down a moment later
///
/// 64 frames over 3.10 seconds.
const TAKEOFF: Routine = &[
    RoutineFrame {
        pose: Some(7),
        ticks: 4,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(37),
        ticks: 6,
        at: (-1, 0),
    },
    RoutineFrame {
        pose: Some(51),
        ticks: 12,
        at: (0, -1),
    },
    RoutineFrame {
        pose: Some(68),
        ticks: 4,
        at: (1, 0),
    },
    RoutineFrame {
        pose: Some(90),
        ticks: 2,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(69),
        ticks: 2,
        at: (0, -1),
    },
    RoutineFrame {
        pose: Some(48),
        ticks: 4,
        at: (1, -1),
    },
    RoutineFrame {
        pose: Some(76),
        ticks: 2,
        at: (1, -1),
    },
    RoutineFrame {
        pose: Some(64),
        ticks: 2,
        at: (0, -1),
    },
    RoutineFrame {
        pose: Some(1),
        ticks: 2,
        at: (-1, -3),
    },
    RoutineFrame {
        pose: Some(90),
        ticks: 2,
        at: (-3, -7),
    },
    RoutineFrame {
        pose: Some(57),
        ticks: 4,
        at: (-3, -11),
    },
    RoutineFrame {
        pose: Some(69),
        ticks: 2,
        at: (-5, -17),
    },
    RoutineFrame {
        pose: Some(48),
        ticks: 2,
        at: (-5, -18),
    },
    RoutineFrame {
        pose: Some(91),
        ticks: 2,
        at: (-6, -22),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 4,
        at: (-10, -24),
    },
    RoutineFrame {
        pose: Some(92),
        ticks: 2,
        at: (-10, -31),
    },
    RoutineFrame {
        pose: Some(57),
        ticks: 4,
        at: (-11, -36),
    },
    RoutineFrame {
        pose: Some(69),
        ticks: 2,
        at: (-13, -40),
    },
    RoutineFrame {
        pose: Some(70),
        ticks: 4,
        at: (-14, -44),
    },
    RoutineFrame {
        pose: Some(38),
        ticks: 2,
        at: (-16, -46),
    },
    RoutineFrame {
        pose: Some(68),
        ticks: 2,
        at: (-17, -53),
    },
    RoutineFrame {
        pose: Some(58),
        ticks: 2,
        at: (-18, -56),
    },
    RoutineFrame {
        pose: Some(32),
        ticks: 2,
        at: (-19, -60),
    },
    RoutineFrame {
        pose: Some(69),
        ticks: 4,
        at: (-21, -64),
    },
    RoutineFrame {
        pose: Some(38),
        ticks: 2,
        at: (-22, -64),
    },
    RoutineFrame {
        pose: Some(82),
        ticks: 2,
        at: (-23, -74),
    },
    RoutineFrame {
        pose: Some(87),
        ticks: 4,
        at: (-25, -76),
    },
    RoutineFrame {
        pose: Some(87),
        ticks: 2,
        at: (-26, -81),
    },
    RoutineFrame {
        pose: Some(60),
        ticks: 2,
        at: (-27, -81),
    },
    RoutineFrame {
        pose: Some(42),
        ticks: 2,
        at: (-27, -84),
    },
    RoutineFrame {
        pose: Some(87),
        ticks: 2,
        at: (-28, -91),
    },
    RoutineFrame {
        pose: Some(17),
        ticks: 2,
        at: (-28, -92),
    },
    RoutineFrame {
        pose: Some(17),
        ticks: 4,
        at: (-28, -93),
    },
    RoutineFrame {
        pose: Some(17),
        ticks: 2,
        at: (-28, -96),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 2,
        at: (-28, -97),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 4,
        at: (-28, -98),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 2,
        at: (-28, -99),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 4,
        at: (-28, -100),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 4,
        at: (-28, -99),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 2,
        at: (-28, -98),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 4,
        at: (-28, -97),
    },
    RoutineFrame {
        pose: Some(17),
        ticks: 2,
        at: (-28, -96),
    },
    RoutineFrame {
        pose: Some(17),
        ticks: 2,
        at: (-28, -93),
    },
    RoutineFrame {
        pose: Some(71),
        ticks: 2,
        at: (-28, -92),
    },
    RoutineFrame {
        pose: Some(71),
        ticks: 2,
        at: (-28, -90),
    },
    RoutineFrame {
        pose: Some(17),
        ticks: 2,
        at: (-28, -86),
    },
    RoutineFrame {
        pose: Some(42),
        ticks: 2,
        at: (-27, -82),
    },
    RoutineFrame {
        pose: Some(60),
        ticks: 2,
        at: (-27, -79),
    },
    RoutineFrame {
        pose: Some(42),
        ticks: 4,
        at: (-27, -76),
    },
    RoutineFrame {
        pose: Some(8),
        ticks: 2,
        at: (-27, -72),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 2,
        at: (-27, -70),
    },
    RoutineFrame {
        pose: Some(60),
        ticks: 2,
        at: (-27, -60),
    },
    RoutineFrame {
        pose: Some(1),
        ticks: 2,
        at: (-27, -55),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 4,
        at: (-27, -53),
    },
    RoutineFrame {
        pose: Some(71),
        ticks: 2,
        at: (-27, -43),
    },
    RoutineFrame {
        pose: Some(42),
        ticks: 2,
        at: (-27, -35),
    },
    RoutineFrame {
        pose: Some(8),
        ticks: 2,
        at: (-27, -30),
    },
    RoutineFrame {
        pose: Some(25),
        ticks: 4,
        at: (-28, -23),
    },
    RoutineFrame {
        pose: Some(51),
        ticks: 2,
        at: (-27, -11),
    },
    RoutineFrame {
        pose: Some(8),
        ticks: 2,
        at: (-27, -5),
    },
    RoutineFrame {
        pose: Some(1),
        ticks: 4,
        at: (-27, 0),
    },
    RoutineFrame {
        pose: Some(3),
        ticks: 6,
        at: (-27, 7),
    },
    RoutineFrame {
        pose: Some(0),
        ticks: 6,
        at: (-27, 0),
    },
];

/// Squats and springs up to the right, landing flat
///
/// 18 frames over 1.23 seconds.
const JUMP_RIGHT: Routine = &[
    RoutineFrame {
        pose: Some(0),
        ticks: 10,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(3),
        ticks: 4,
        at: (0, 7),
    },
    RoutineFrame {
        pose: Some(35),
        ticks: 4,
        at: (0, 1),
    },
    RoutineFrame {
        pose: Some(89),
        ticks: 2,
        at: (1, -7),
    },
    RoutineFrame {
        pose: Some(89),
        ticks: 4,
        at: (4, -16),
    },
    RoutineFrame {
        pose: Some(66),
        ticks: 2,
        at: (4, -21),
    },
    RoutineFrame {
        pose: Some(66),
        ticks: 2,
        at: (5, -27),
    },
    RoutineFrame {
        pose: Some(66),
        ticks: 2,
        at: (6, -29),
    },
    RoutineFrame {
        pose: Some(66),
        ticks: 2,
        at: (6, -31),
    },
    RoutineFrame {
        pose: Some(66),
        ticks: 4,
        at: (8, -32),
    },
    RoutineFrame {
        pose: Some(66),
        ticks: 2,
        at: (9, -31),
    },
    RoutineFrame {
        pose: Some(66),
        ticks: 4,
        at: (11, -27),
    },
    RoutineFrame {
        pose: Some(66),
        ticks: 2,
        at: (13, -21),
    },
    RoutineFrame {
        pose: Some(22),
        ticks: 4,
        at: (14, -16),
    },
    RoutineFrame {
        pose: Some(22),
        ticks: 2,
        at: (15, -8),
    },
    RoutineFrame {
        pose: Some(35),
        ticks: 2,
        at: (17, 1),
    },
    RoutineFrame {
        pose: Some(3),
        ticks: 8,
        at: (17, 7),
    },
    RoutineFrame {
        pose: Some(0),
        ticks: 14,
        at: (17, 0),
    },
];

/// The same spring, to the left - the game draws both ways round
///
/// 19 frames over 1.43 seconds.
const JUMP_LEFT: Routine = &[
    RoutineFrame {
        pose: Some(0),
        ticks: 8,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(2),
        ticks: 14,
        at: (0, 1),
    },
    RoutineFrame {
        pose: Some(26),
        ticks: 2,
        at: (0, 1),
    },
    RoutineFrame {
        pose: Some(65),
        ticks: 4,
        at: (1, 1),
    },
    RoutineFrame {
        pose: Some(39),
        ticks: 2,
        at: (-2, -5),
    },
    RoutineFrame {
        pose: Some(39),
        ticks: 2,
        at: (-4, -14),
    },
    RoutineFrame {
        pose: Some(88),
        ticks: 2,
        at: (-5, -20),
    },
    RoutineFrame {
        pose: Some(88),
        ticks: 4,
        at: (-6, -25),
    },
    RoutineFrame {
        pose: Some(15),
        ticks: 2,
        at: (-6, -29),
    },
    RoutineFrame {
        pose: Some(15),
        ticks: 4,
        at: (-8, -32),
    },
    RoutineFrame {
        pose: Some(15),
        ticks: 2,
        at: (-9, -32),
    },
    RoutineFrame {
        pose: Some(77),
        ticks: 4,
        at: (-11, -32),
    },
    RoutineFrame {
        pose: Some(39),
        ticks: 2,
        at: (-13, -25),
    },
    RoutineFrame {
        pose: Some(44),
        ticks: 2,
        at: (-15, -18),
    },
    RoutineFrame {
        pose: Some(44),
        ticks: 4,
        at: (-16, -14),
    },
    RoutineFrame {
        pose: Some(80),
        ticks: 2,
        at: (-16, -4),
    },
    RoutineFrame {
        pose: Some(65),
        ticks: 2,
        at: (-16, 1),
    },
    RoutineFrame {
        pose: Some(26),
        ticks: 8,
        at: (-17, 7),
    },
    RoutineFrame {
        pose: Some(2),
        ticks: 16,
        at: (-17, 1),
    },
];

/// Spins on the spot, twice round, travelling right
///
/// 18 frames over 1.07 seconds.
const SPIN_RIGHT: Routine = &[
    RoutineFrame {
        pose: Some(0),
        ticks: 10,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(79),
        ticks: 2,
        at: (1, -2),
    },
    RoutineFrame {
        pose: Some(79),
        ticks: 2,
        at: (2, -2),
    },
    RoutineFrame {
        pose: Some(34),
        ticks: 2,
        at: (3, -1),
    },
    RoutineFrame {
        pose: Some(34),
        ticks: 4,
        at: (4, -1),
    },
    RoutineFrame {
        pose: Some(23),
        ticks: 2,
        at: (6, -1),
    },
    RoutineFrame {
        pose: Some(23),
        ticks: 4,
        at: (8, -1),
    },
    RoutineFrame {
        pose: Some(29),
        ticks: 2,
        at: (9, -1),
    },
    RoutineFrame {
        pose: Some(29),
        ticks: 2,
        at: (10, -1),
    },
    RoutineFrame {
        pose: Some(30),
        ticks: 4,
        at: (11, 0),
    },
    RoutineFrame {
        pose: Some(30),
        ticks: 2,
        at: (13, 0),
    },
    RoutineFrame {
        pose: Some(34),
        ticks: 2,
        at: (14, -1),
    },
    RoutineFrame {
        pose: Some(34),
        ticks: 2,
        at: (15, -1),
    },
    RoutineFrame {
        pose: Some(23),
        ticks: 4,
        at: (16, -1),
    },
    RoutineFrame {
        pose: Some(23),
        ticks: 2,
        at: (18, -1),
    },
    RoutineFrame {
        pose: Some(29),
        ticks: 6,
        at: (20, -1),
    },
    RoutineFrame {
        pose: Some(0),
        ticks: 2,
        at: (20, 0),
    },
    RoutineFrame {
        pose: Some(37),
        ticks: 10,
        at: (20, 0),
    },
];

/// The same spin the other way
///
/// 19 frames over 1.03 seconds.
const SPIN_LEFT: Routine = &[
    RoutineFrame {
        pose: Some(7),
        ticks: 2,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(0),
        ticks: 8,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(29),
        ticks: 4,
        at: (-1, -1),
    },
    RoutineFrame {
        pose: Some(29),
        ticks: 2,
        at: (-2, -1),
    },
    RoutineFrame {
        pose: Some(23),
        ticks: 2,
        at: (-3, -1),
    },
    RoutineFrame {
        pose: Some(23),
        ticks: 4,
        at: (-5, -1),
    },
    RoutineFrame {
        pose: Some(34),
        ticks: 2,
        at: (-6, -1),
    },
    RoutineFrame {
        pose: Some(34),
        ticks: 2,
        at: (-7, -1),
    },
    RoutineFrame {
        pose: Some(30),
        ticks: 4,
        at: (-9, 0),
    },
    RoutineFrame {
        pose: Some(30),
        ticks: 2,
        at: (-10, 0),
    },
    RoutineFrame {
        pose: Some(29),
        ticks: 2,
        at: (-11, -1),
    },
    RoutineFrame {
        pose: Some(23),
        ticks: 4,
        at: (-13, -1),
    },
    RoutineFrame {
        pose: Some(23),
        ticks: 2,
        at: (-14, -1),
    },
    RoutineFrame {
        pose: Some(34),
        ticks: 2,
        at: (-16, -1),
    },
    RoutineFrame {
        pose: Some(34),
        ticks: 2,
        at: (-17, -1),
    },
    RoutineFrame {
        pose: Some(34),
        ticks: 4,
        at: (-18, -1),
    },
    RoutineFrame {
        pose: Some(30),
        ticks: 2,
        at: (-19, 0),
    },
    RoutineFrame {
        pose: Some(30),
        ticks: 2,
        at: (-21, 0),
    },
    RoutineFrame {
        pose: Some(0),
        ticks: 10,
        at: (-21, 0),
    },
];

/// Turns his back and tumbles along to the right, three times round
///
/// 17 frames over 1.23 seconds.
const TUMBLE_RIGHT: Routine = &[
    RoutineFrame {
        pose: Some(0),
        ticks: 10,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(50),
        ticks: 4,
        at: (1, 0),
    },
    RoutineFrame {
        pose: Some(41),
        ticks: 4,
        at: (2, -1),
    },
    RoutineFrame {
        pose: Some(41),
        ticks: 4,
        at: (3, -1),
    },
    RoutineFrame {
        pose: Some(56),
        ticks: 4,
        at: (4, -1),
    },
    RoutineFrame {
        pose: Some(50),
        ticks: 4,
        at: (5, 0),
    },
    RoutineFrame {
        pose: Some(50),
        ticks: 2,
        at: (6, 0),
    },
    RoutineFrame {
        pose: Some(41),
        ticks: 4,
        at: (6, -1),
    },
    RoutineFrame {
        pose: Some(41),
        ticks: 2,
        at: (7, -1),
    },
    RoutineFrame {
        pose: Some(56),
        ticks: 4,
        at: (8, -1),
    },
    RoutineFrame {
        pose: Some(56),
        ticks: 2,
        at: (9, -1),
    },
    RoutineFrame {
        pose: Some(50),
        ticks: 2,
        at: (9, 0),
    },
    RoutineFrame {
        pose: Some(50),
        ticks: 4,
        at: (10, 0),
    },
    RoutineFrame {
        pose: Some(41),
        ticks: 4,
        at: (11, -1),
    },
    RoutineFrame {
        pose: Some(41),
        ticks: 4,
        at: (12, -1),
    },
    RoutineFrame {
        pose: Some(56),
        ticks: 4,
        at: (13, -1),
    },
    RoutineFrame {
        pose: Some(0),
        ticks: 12,
        at: (14, 0),
    },
];

/// The same tumble the other way
///
/// 17 frames over 1.57 seconds.
const TUMBLE_LEFT: Routine = &[
    RoutineFrame {
        pose: Some(0),
        ticks: 10,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(53),
        ticks: 2,
        at: (-1, 0),
    },
    RoutineFrame {
        pose: Some(53),
        ticks: 4,
        at: (-2, 0),
    },
    RoutineFrame {
        pose: Some(46),
        ticks: 6,
        at: (-2, -1),
    },
    RoutineFrame {
        pose: Some(47),
        ticks: 4,
        at: (-4, -1),
    },
    RoutineFrame {
        pose: Some(53),
        ticks: 2,
        at: (-5, 0),
    },
    RoutineFrame {
        pose: Some(53),
        ticks: 6,
        at: (-6, 0),
    },
    RoutineFrame {
        pose: Some(46),
        ticks: 4,
        at: (-6, -1),
    },
    RoutineFrame {
        pose: Some(47),
        ticks: 2,
        at: (-8, -1),
    },
    RoutineFrame {
        pose: Some(47),
        ticks: 6,
        at: (-9, -1),
    },
    RoutineFrame {
        pose: Some(85),
        ticks: 2,
        at: (-9, 0),
    },
    RoutineFrame {
        pose: Some(53),
        ticks: 2,
        at: (-10, 0),
    },
    RoutineFrame {
        pose: Some(46),
        ticks: 8,
        at: (-11, -1),
    },
    RoutineFrame {
        pose: Some(47),
        ticks: 2,
        at: (-13, -1),
    },
    RoutineFrame {
        pose: Some(47),
        ticks: 4,
        at: (-14, -1),
    },
    RoutineFrame {
        pose: Some(2),
        ticks: 22,
        at: (-14, 1),
    },
    RoutineFrame {
        pose: Some(0),
        ticks: 8,
        at: (-14, 0),
    },
];

/// Turns his back and walks a few steps, then turns round again - the same, mirrored
///
/// 19 frames over 1.23 seconds.
const WALK_MIRRORED: Routine = &[
    RoutineFrame {
        pose: Some(93),
        ticks: 6,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(94),
        ticks: 2,
        at: (-2, 0),
    },
    RoutineFrame {
        pose: Some(94),
        ticks: 4,
        at: (-3, 0),
    },
    RoutineFrame {
        pose: Some(94),
        ticks: 2,
        at: (-6, 0),
    },
    RoutineFrame {
        pose: Some(94),
        ticks: 4,
        at: (-8, 0),
    },
    RoutineFrame {
        pose: Some(94),
        ticks: 2,
        at: (-10, 0),
    },
    RoutineFrame {
        pose: Some(95),
        ticks: 2,
        at: (-13, -1),
    },
    RoutineFrame {
        pose: Some(96),
        ticks: 2,
        at: (-13, -1),
    },
    RoutineFrame {
        pose: Some(96),
        ticks: 4,
        at: (-15, -1),
    },
    RoutineFrame {
        pose: Some(97),
        ticks: 2,
        at: (-17, 0),
    },
    RoutineFrame {
        pose: Some(97),
        ticks: 2,
        at: (-18, 0),
    },
    RoutineFrame {
        pose: Some(97),
        ticks: 2,
        at: (-19, 0),
    },
    RoutineFrame {
        pose: Some(98),
        ticks: 4,
        at: (-20, 0),
    },
    RoutineFrame {
        pose: Some(98),
        ticks: 4,
        at: (-21, 0),
    },
    RoutineFrame {
        pose: Some(99),
        ticks: 2,
        at: (-20, -1),
    },
    RoutineFrame {
        pose: Some(100),
        ticks: 12,
        at: (-22, -2),
    },
    RoutineFrame {
        pose: Some(100),
        ticks: 2,
        at: (-21, -2),
    },
    RoutineFrame {
        pose: Some(101),
        ticks: 6,
        at: (-21, 0),
    },
    RoutineFrame {
        pose: Some(93),
        ticks: 10,
        at: (-20, 0),
    },
];

/// Lies down, then ricochets wall to wall across the arch, squashing thin against each, before shooting off the top and dropping back - the same, mirrored
///
/// 55 frames over 6.10 seconds, 0.20 of them off the top of the recording.
const RICOCHET_MIRRORED: Routine = &[
    RoutineFrame {
        pose: Some(102),
        ticks: 2,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(93),
        ticks: 8,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(103),
        ticks: 40,
        at: (0, 7),
    },
    RoutineFrame {
        pose: Some(104),
        ticks: 2,
        at: (7, 0),
    },
    RoutineFrame {
        pose: Some(105),
        ticks: 2,
        at: (7, -1),
    },
    RoutineFrame {
        pose: Some(106),
        ticks: 2,
        at: (11, -7),
    },
    RoutineFrame {
        pose: Some(106),
        ticks: 10,
        at: (11, -8),
    },
    RoutineFrame {
        pose: Some(107),
        ticks: 4,
        at: (-17, -9),
    },
    RoutineFrame {
        pose: Some(108),
        ticks: 10,
        at: (-20, -14),
    },
    RoutineFrame {
        pose: Some(104),
        ticks: 4,
        at: (0, -14),
    },
    RoutineFrame {
        pose: Some(106),
        ticks: 10,
        at: (11, -19),
    },
    RoutineFrame {
        pose: Some(109),
        ticks: 2,
        at: (-9, -21),
    },
    RoutineFrame {
        pose: Some(109),
        ticks: 2,
        at: (-18, -23),
    },
    RoutineFrame {
        pose: Some(110),
        ticks: 10,
        at: (-20, -25),
    },
    RoutineFrame {
        pose: Some(105),
        ticks: 4,
        at: (0, -25),
    },
    RoutineFrame {
        pose: Some(106),
        ticks: 10,
        at: (11, -31),
    },
    RoutineFrame {
        pose: Some(107),
        ticks: 4,
        at: (-17, -32),
    },
    RoutineFrame {
        pose: Some(110),
        ticks: 8,
        at: (-20, -37),
    },
    RoutineFrame {
        pose: Some(104),
        ticks: 2,
        at: (-15, -37),
    },
    RoutineFrame {
        pose: Some(104),
        ticks: 4,
        at: (0, -37),
    },
    RoutineFrame {
        pose: Some(111),
        ticks: 10,
        at: (12, -44),
    },
    RoutineFrame {
        pose: Some(112),
        ticks: 2,
        at: (-1, -44),
    },
    RoutineFrame {
        pose: Some(112),
        ticks: 2,
        at: (-14, -46),
    },
    RoutineFrame {
        pose: Some(110),
        ticks: 12,
        at: (-21, -49),
    },
    RoutineFrame {
        pose: Some(104),
        ticks: 2,
        at: (1, -51),
    },
    RoutineFrame {
        pose: Some(110),
        ticks: 10,
        at: (10, -55),
    },
    RoutineFrame {
        pose: Some(112),
        ticks: 2,
        at: (-1, -55),
    },
    RoutineFrame {
        pose: Some(113),
        ticks: 2,
        at: (-16, -55),
    },
    RoutineFrame {
        pose: Some(110),
        ticks: 12,
        at: (-20, -62),
    },
    RoutineFrame {
        pose: None,
        ticks: 12,
        at: (-26, -62),
    },
    RoutineFrame {
        pose: Some(112),
        ticks: 4,
        at: (-17, -68),
    },
    RoutineFrame {
        pose: Some(112),
        ticks: 2,
        at: (-22, -70),
    },
    RoutineFrame {
        pose: Some(114),
        ticks: 34,
        at: (-19, -72),
    },
    RoutineFrame {
        pose: Some(114),
        ticks: 2,
        at: (-19, -71),
    },
    RoutineFrame {
        pose: Some(114),
        ticks: 2,
        at: (-19, -70),
    },
    RoutineFrame {
        pose: Some(114),
        ticks: 2,
        at: (-20, -69),
    },
    RoutineFrame {
        pose: Some(110),
        ticks: 4,
        at: (-21, -68),
    },
    RoutineFrame {
        pose: Some(110),
        ticks: 2,
        at: (-21, -65),
    },
    RoutineFrame {
        pose: Some(110),
        ticks: 2,
        at: (-21, -63),
    },
    RoutineFrame {
        pose: Some(110),
        ticks: 2,
        at: (-21, -62),
    },
    RoutineFrame {
        pose: Some(108),
        ticks: 2,
        at: (-21, -59),
    },
    RoutineFrame {
        pose: Some(110),
        ticks: 2,
        at: (-20, -55),
    },
    RoutineFrame {
        pose: Some(108),
        ticks: 4,
        at: (-20, -50),
    },
    RoutineFrame {
        pose: Some(108),
        ticks: 2,
        at: (-21, -46),
    },
    RoutineFrame {
        pose: Some(111),
        ticks: 4,
        at: (-19, -41),
    },
    RoutineFrame {
        pose: Some(110),
        ticks: 2,
        at: (-21, -33),
    },
    RoutineFrame {
        pose: Some(106),
        ticks: 2,
        at: (-20, -29),
    },
    RoutineFrame {
        pose: Some(106),
        ticks: 2,
        at: (-20, -25),
    },
    RoutineFrame {
        pose: Some(106),
        ticks: 2,
        at: (-20, -19),
    },
    RoutineFrame {
        pose: Some(115),
        ticks: 4,
        at: (-20, -10),
    },
    RoutineFrame {
        pose: Some(115),
        ticks: 2,
        at: (-20, -1),
    },
    RoutineFrame {
        pose: Some(115),
        ticks: 2,
        at: (-20, 0),
    },
    RoutineFrame {
        pose: Some(103),
        ticks: 6,
        at: (-23, 7),
    },
    RoutineFrame {
        pose: Some(116),
        ticks: 62,
        at: (-23, 0),
    },
    RoutineFrame {
        pose: Some(107),
        ticks: 6,
        at: (-24, 0),
    },
];

/// Squashes right down and springs clean off the top of the arch - the same, mirrored
///
/// 21 frames over 1.90 seconds.
const SPRING_MIRRORED: Routine = &[
    RoutineFrame {
        pose: Some(102),
        ticks: 2,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(107),
        ticks: 8,
        at: (-1, 0),
    },
    RoutineFrame {
        pose: Some(104),
        ticks: 12,
        at: (-1, 1),
    },
    RoutineFrame {
        pose: Some(117),
        ticks: 16,
        at: (-1, 7),
    },
    RoutineFrame {
        pose: Some(118),
        ticks: 4,
        at: (1, -1),
    },
    RoutineFrame {
        pose: Some(118),
        ticks: 2,
        at: (3, -28),
    },
    RoutineFrame {
        pose: Some(118),
        ticks: 2,
        at: (5, -55),
    },
    RoutineFrame {
        pose: Some(118),
        ticks: 4,
        at: (7, -69),
    },
    RoutineFrame {
        pose: Some(119),
        ticks: 2,
        at: (9, -91),
    },
    RoutineFrame {
        pose: Some(119),
        ticks: 2,
        at: (10, -99),
    },
    RoutineFrame {
        pose: Some(119),
        ticks: 4,
        at: (12, -99),
    },
    RoutineFrame {
        pose: Some(120),
        ticks: 2,
        at: (15, -95),
    },
    RoutineFrame {
        pose: Some(120),
        ticks: 2,
        at: (17, -84),
    },
    RoutineFrame {
        pose: Some(120),
        ticks: 2,
        at: (18, -74),
    },
    RoutineFrame {
        pose: Some(121),
        ticks: 2,
        at: (18, -64),
    },
    RoutineFrame {
        pose: Some(121),
        ticks: 2,
        at: (20, -41),
    },
    RoutineFrame {
        pose: Some(122),
        ticks: 2,
        at: (23, -18),
    },
    RoutineFrame {
        pose: Some(123),
        ticks: 4,
        at: (25, 1),
    },
    RoutineFrame {
        pose: Some(103),
        ticks: 8,
        at: (25, 7),
    },
    RoutineFrame {
        pose: Some(104),
        ticks: 20,
        at: (24, 1),
    },
    RoutineFrame {
        pose: Some(93),
        ticks: 12,
        at: (25, 0),
    },
];

/// Runs on the spot and then flies up and out, coming back down a moment later - the same, mirrored
///
/// 64 frames over 3.10 seconds.
const TAKEOFF_MIRRORED: Routine = &[
    RoutineFrame {
        pose: Some(102),
        ticks: 4,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(107),
        ticks: 6,
        at: (-1, 0),
    },
    RoutineFrame {
        pose: Some(124),
        ticks: 12,
        at: (1, -1),
    },
    RoutineFrame {
        pose: Some(125),
        ticks: 4,
        at: (1, 0),
    },
    RoutineFrame {
        pose: Some(126),
        ticks: 2,
        at: (1, 0),
    },
    RoutineFrame {
        pose: Some(127),
        ticks: 2,
        at: (1, -1),
    },
    RoutineFrame {
        pose: Some(128),
        ticks: 4,
        at: (1, -1),
    },
    RoutineFrame {
        pose: Some(129),
        ticks: 2,
        at: (-1, -1),
    },
    RoutineFrame {
        pose: Some(130),
        ticks: 2,
        at: (2, -1),
    },
    RoutineFrame {
        pose: Some(131),
        ticks: 2,
        at: (2, -3),
    },
    RoutineFrame {
        pose: Some(126),
        ticks: 2,
        at: (4, -7),
    },
    RoutineFrame {
        pose: Some(132),
        ticks: 4,
        at: (4, -11),
    },
    RoutineFrame {
        pose: Some(127),
        ticks: 2,
        at: (6, -17),
    },
    RoutineFrame {
        pose: Some(128),
        ticks: 2,
        at: (7, -18),
    },
    RoutineFrame {
        pose: Some(133),
        ticks: 2,
        at: (7, -22),
    },
    RoutineFrame {
        pose: Some(134),
        ticks: 4,
        at: (9, -24),
    },
    RoutineFrame {
        pose: Some(135),
        ticks: 2,
        at: (11, -31),
    },
    RoutineFrame {
        pose: Some(132),
        ticks: 4,
        at: (12, -36),
    },
    RoutineFrame {
        pose: Some(127),
        ticks: 2,
        at: (14, -40),
    },
    RoutineFrame {
        pose: Some(136),
        ticks: 4,
        at: (16, -44),
    },
    RoutineFrame {
        pose: Some(137),
        ticks: 2,
        at: (15, -46),
    },
    RoutineFrame {
        pose: Some(125),
        ticks: 2,
        at: (19, -53),
    },
    RoutineFrame {
        pose: Some(138),
        ticks: 2,
        at: (19, -56),
    },
    RoutineFrame {
        pose: Some(139),
        ticks: 2,
        at: (20, -60),
    },
    RoutineFrame {
        pose: Some(127),
        ticks: 4,
        at: (22, -64),
    },
    RoutineFrame {
        pose: Some(137),
        ticks: 2,
        at: (21, -64),
    },
    RoutineFrame {
        pose: Some(140),
        ticks: 2,
        at: (24, -74),
    },
    RoutineFrame {
        pose: Some(141),
        ticks: 4,
        at: (24, -76),
    },
    RoutineFrame {
        pose: Some(141),
        ticks: 2,
        at: (25, -81),
    },
    RoutineFrame {
        pose: Some(142),
        ticks: 2,
        at: (27, -81),
    },
    RoutineFrame {
        pose: Some(143),
        ticks: 2,
        at: (28, -84),
    },
    RoutineFrame {
        pose: Some(141),
        ticks: 2,
        at: (27, -91),
    },
    RoutineFrame {
        pose: Some(144),
        ticks: 2,
        at: (27, -92),
    },
    RoutineFrame {
        pose: Some(144),
        ticks: 4,
        at: (27, -93),
    },
    RoutineFrame {
        pose: Some(144),
        ticks: 2,
        at: (27, -96),
    },
    RoutineFrame {
        pose: Some(134),
        ticks: 2,
        at: (27, -97),
    },
    RoutineFrame {
        pose: Some(134),
        ticks: 4,
        at: (27, -98),
    },
    RoutineFrame {
        pose: Some(134),
        ticks: 2,
        at: (27, -99),
    },
    RoutineFrame {
        pose: Some(134),
        ticks: 4,
        at: (27, -100),
    },
    RoutineFrame {
        pose: Some(134),
        ticks: 4,
        at: (27, -99),
    },
    RoutineFrame {
        pose: Some(134),
        ticks: 2,
        at: (27, -98),
    },
    RoutineFrame {
        pose: Some(134),
        ticks: 4,
        at: (27, -97),
    },
    RoutineFrame {
        pose: Some(144),
        ticks: 2,
        at: (27, -96),
    },
    RoutineFrame {
        pose: Some(144),
        ticks: 2,
        at: (27, -93),
    },
    RoutineFrame {
        pose: Some(145),
        ticks: 2,
        at: (27, -92),
    },
    RoutineFrame {
        pose: Some(145),
        ticks: 2,
        at: (27, -90),
    },
    RoutineFrame {
        pose: Some(144),
        ticks: 2,
        at: (27, -86),
    },
    RoutineFrame {
        pose: Some(143),
        ticks: 2,
        at: (28, -82),
    },
    RoutineFrame {
        pose: Some(142),
        ticks: 2,
        at: (27, -79),
    },
    RoutineFrame {
        pose: Some(143),
        ticks: 4,
        at: (28, -76),
    },
    RoutineFrame {
        pose: Some(146),
        ticks: 2,
        at: (27, -72),
    },
    RoutineFrame {
        pose: Some(134),
        ticks: 2,
        at: (26, -70),
    },
    RoutineFrame {
        pose: Some(142),
        ticks: 2,
        at: (27, -60),
    },
    RoutineFrame {
        pose: Some(131),
        ticks: 2,
        at: (28, -55),
    },
    RoutineFrame {
        pose: Some(134),
        ticks: 4,
        at: (26, -53),
    },
    RoutineFrame {
        pose: Some(145),
        ticks: 2,
        at: (26, -43),
    },
    RoutineFrame {
        pose: Some(143),
        ticks: 2,
        at: (28, -35),
    },
    RoutineFrame {
        pose: Some(146),
        ticks: 2,
        at: (27, -30),
    },
    RoutineFrame {
        pose: Some(134),
        ticks: 4,
        at: (27, -23),
    },
    RoutineFrame {
        pose: Some(124),
        ticks: 2,
        at: (28, -11),
    },
    RoutineFrame {
        pose: Some(146),
        ticks: 2,
        at: (27, -5),
    },
    RoutineFrame {
        pose: Some(131),
        ticks: 4,
        at: (28, 0),
    },
    RoutineFrame {
        pose: Some(103),
        ticks: 6,
        at: (27, 7),
    },
    RoutineFrame {
        pose: Some(93),
        ticks: 6,
        at: (27, 0),
    },
];

/// The flop, with its own recovery cut off and the last pose held: a buried
/// player's Kirby stays down rather than picking himself up every two seconds.
///
/// The one routine here that is **derived** and not measured - every frame is
/// the capture's, but the game never showed a Kirby who had lost.
const COLLAPSE: Routine = &[
    RoutineFrame {
        pose: Some(0),
        ticks: 10,
        at: (0, 0),
    },
    RoutineFrame {
        pose: Some(3),
        ticks: 6,
        at: (0, 7),
    },
    RoutineFrame {
        pose: Some(4),
        ticks: 3600,
        at: (0, 0),
    },
];

/// Everything he may be dealt: one entry a **kind**, each with the ways round
/// it can be played.
///
/// The bag deals a *kind* and the **direction is chosen from where he is**, so
/// a Kirby against the right post walks left rather than sliding left to walk
/// back. `jump`, `spin` and `tumble` have both ways in the game's own art; the
/// four that travel and were only recorded one way have a mirrored twin.
///
/// The intensity is **vertical reach**, scaled so the biggest is one: a blink
/// and a spin are nothing much, a jump is something, and climbing clean out of
/// the arch is the most he does. It decides the order a bag comes out in.
const CHOICES: &[RoutineChoice] = &[
    RoutineChoice {
        intensity: 0.00,
        ways: &[RoutineWay {
            frames: YAWN,
            origins: (0, 33),
            glide: (0, 10),
        }],
    },
    RoutineChoice {
        intensity: 0.02,
        ways: &[
            RoutineWay {
                frames: WALK,
                origins: (0, 12),
                glide: (0, 6),
            },
            RoutineWay {
                frames: WALK_MIRRORED,
                origins: (22, 34),
                glide: (0, 6),
            },
        ],
    },
    RoutineChoice {
        intensity: 0.00,
        ways: &[RoutineWay {
            frames: FLOP,
            origins: (0, 34),
            glide: (0, 10),
        }],
    },
    RoutineChoice {
        intensity: 0.72,
        ways: &[
            RoutineWay {
                frames: RICOCHET,
                origins: (8, 10),
                glide: (54, 2),
            },
            RoutineWay {
                frames: RICOCHET_MIRRORED,
                origins: (26, 26),
                glide: (54, 2),
            },
        ],
    },
    RoutineChoice {
        intensity: 0.64,
        ways: &[RoutineWay {
            frames: CLIMB,
            origins: (1, 32),
            glide: (10, 8),
        }],
    },
    RoutineChoice {
        intensity: 0.52,
        ways: &[RoutineWay {
            frames: FLOAT_UP,
            origins: (3, 33),
            glide: (28, 2),
        }],
    },
    RoutineChoice {
        intensity: 0.99,
        ways: &[
            RoutineWay {
                frames: SPRING,
                origins: (25, 33),
                glide: (42, 2),
            },
            RoutineWay {
                frames: SPRING_MIRRORED,
                origins: (1, 9),
                glide: (42, 2),
            },
        ],
    },
    RoutineChoice {
        intensity: 1.00,
        ways: &[
            RoutineWay {
                frames: TAKEOFF,
                origins: (28, 33),
                glide: (38, 2),
            },
            RoutineWay {
                frames: TAKEOFF_MIRRORED,
                origins: (1, 6),
                glide: (38, 2),
            },
        ],
    },
    RoutineChoice {
        intensity: 0.32,
        ways: &[
            RoutineWay {
                frames: JUMP_RIGHT,
                origins: (0, 17),
                glide: (18, 2),
            },
            RoutineWay {
                frames: JUMP_LEFT,
                origins: (17, 33),
                glide: (28, 2),
            },
        ],
    },
    RoutineChoice {
        intensity: 0.02,
        ways: &[
            RoutineWay {
                frames: SPIN_RIGHT,
                origins: (0, 12),
                glide: (0, 10),
            },
            RoutineWay {
                frames: SPIN_LEFT,
                origins: (21, 34),
                glide: (0, 2),
            },
        ],
    },
    RoutineChoice {
        intensity: 0.01,
        ways: &[
            RoutineWay {
                frames: TUMBLE_RIGHT,
                origins: (0, 20),
                glide: (0, 10),
            },
            RoutineWay {
                frames: TUMBLE_LEFT,
                origins: (14, 34),
                glide: (0, 10),
            },
        ],
    },
];

/// Played *between* routines and not as one of them, half the time.
///
/// The blink is a tic. Dealt like a routine it is a fraction of what he does
/// and reads as a loop; between them it reads as a character standing there.
const FILLER: RoutineWay = RoutineWay {
    frames: BLINK,
    origins: (0, 34),
    glide: (0, 0),
};

/// Where a routine's opening frame goes in the arch, and how far either way he may end up.
///
/// Every routine opens on Kirby standing, so `HOME.1` is the same for all fifteen: the box's
/// own floor less the fourteen rows a standing pose is.
///
/// `WANDER` is a backstop and nothing more, now that a routine carries the window it may be
/// started from: the windows already keep him inside the arch, and this only says what a home
/// is clamped to if one of them is ever wrong.
const HOME: (i32, i32) = (17, BOX.3 as i32 - 14);
const WANDER: (i32, i32) = (0, BOX.2 as i32 - 14);

/// The cast, which is one: Kirby's Avalanche stands the *player's own* character here, not an
/// opponent, so there is nobody else to be dealt.
///
/// (The game's opponents do have a box of their own - the mugshot at the top right, whose
/// whole cast is in the `Battle Faces` rip - and that one is a strip in a fixed box, so it is
/// the other art model and not this one.)
pub const CAST: &[CharacterData] = &[CharacterData {
    name: "Kirby",
    file: sprites::KIRBY,
    // unread while `routines` is set, and there is nothing truthful to put here
    states: [(1, FrameAnimationType::Static); 4],
    routines: Some(RoutineArt {
        poses: POSES,
        choices: CHOICES,
        filler: Some(FILLER),
        defeat: COLLAPSE,
        approach: APPROACH,
        rest: REST,
        home: HOME,
        wander: WANDER,
        speed_spread: SPEED_SPREAD,
    }),
    layers: &[],
    emitters: &[],
}];

/// The arch opening, in the panel's own pixels - which are the SNES screen's, since the panel
/// is cut from the screen itself.
///
/// The top is the lintel: everything is drawn into the panel texture, so a box any taller
/// would paint over the stone rather than be hidden by it, which is exactly what should happen
/// to a Kirby who has jumped out of the arch.
///
/// The **bottom is the plank** and not the game's own floor. Measured, Kirby's feet are on
/// y=199 - 2343 of 2775 frames of the capture put them there - which is seven rows down inside
/// the course `rip_retro.py`'s `SNES_FLOOR` lays across the arch mouth, where the game leaves
/// bare dark. That course is also the only run in this column as wide as the nuisance tray
/// needs, so it stays and Kirby stands on it: his feet at 192, the tray's boulders in the wood
/// below them, and nothing overlapping. What it costs is seven rows of headroom - he leaves
/// the top of the arch a little sooner than the game's own does, in the five routines that
/// take him out of it at all.
pub const BOX: (i32, i32, u32, u32) = (104, 157, 48, 35);

pub fn cast() -> CharacterSetData {
    CharacterSetData {
        characters: CAST,
        // the routine path addresses poses by rect and never reads these two; they are the
        // strip path's, and a cast of routines has no strip
        frame_size: (BOX.2, BOX.3),
        row_pitch: BOX.3,
        // Mean Bean Machine's sweat is Mean Bean Machine's: six of its characters throw
        // identical drops and this game's Kirby throws nothing at all
        sweat: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every routine there is, taken from the tables rather than listed again: the pool's own
    /// ways, the filler and the derived defeat one.
    fn every_routine() -> Vec<Routine> {
        let mut all: Vec<Routine> = CHOICES
            .iter()
            .flat_map(|c| c.ways)
            .map(|w| w.frames)
            .collect();
        all.push(FILLER.frames);
        all.push(COLLAPSE);
        all
    }

    /// A pose index is written by `kirby.py` and read by the engine with no bounds check on
    /// the way, so a table that has drifted from the sheet draws the wrong Kirby rather than
    /// failing - which is the sort of thing only a test catches.
    #[test]
    fn every_frame_names_a_pose_that_exists() {
        for routine in every_routine() {
            assert!(!routine.is_empty(), "a routine has no frames");
            for (index, frame) in routine.iter().enumerate() {
                if let Some(pose) = frame.pose {
                    assert!(
                        pose < POSES.len(),
                        "frame {index} names pose {pose} of {}",
                        POSES.len()
                    );
                }
                assert!(frame.ticks > 0, "frame {index} is held for no time");
            }
        }
    }

    /// Every pose is cut, so every pose should be played: one nobody names is a sprite the
    /// cutter put in the sheet for nothing.
    #[test]
    fn every_pose_is_played() {
        let mut seen = vec![false; POSES.len()];
        for routine in every_routine() {
            for frame in routine {
                if let Some(pose) = frame.pose {
                    seen[pose] = true;
                }
            }
        }
        let idle: Vec<usize> = seen
            .iter()
            .enumerate()
            .filter(|(_, on)| !**on)
            .map(|(i, _)| i)
            .collect();
        assert!(idle.is_empty(), "poses nothing plays: {idle:?}");
    }

    /// Every routine opens on the character standing, which is what makes a frame's place
    /// relative to its first one mean anything - and what lets `HOME.1` be one number for all
    /// fifteen rather than one per routine.
    #[test]
    fn every_routine_opens_standing_at_its_own_origin() {
        for routine in every_routine() {
            let first = routine[0];
            assert_eq!(
                first.at,
                (0, 0),
                "a routine does not open at its own origin"
            );
            let pose = first.pose.expect("a routine opens on a pose");
            assert_eq!(
                POSES[pose].3, 14,
                "a routine opens on pose {pose}, which is not standing"
            );
        }
    }

    /// The standing pose sits on the box's floor, and a routine may not push the character
    /// out of the far side of the arch.
    #[test]
    fn the_box_holds_him() {
        assert_eq!(
            HOME.1 + 14,
            BOX.3 as i32,
            "a standing pose is not on the floor"
        );
        // and no way may put a pose off the arch from anywhere its own window allows, which
        // is the whole of what a window means
        for choice in CHOICES {
            for way in choice.ways {
                for home in [way.origins.0, way.origins.1] {
                    for frame in way.frames {
                        let Some(pose) = frame.pose else { continue };
                        let left = home + frame.at.0;
                        let right = left + POSES[pose].2 as i32;
                        assert!(
                            left >= 0 && right <= BOX.2 as i32,
                            "a way from {home} draws at {left}..{right}, off a {} wide arch",
                            BOX.2
                        );
                    }
                }
            }
        }
    }

    /// Nothing is rare: the bag is drained strictly, so every routine plays once before any
    /// plays twice. What the intensity decides is the *order* within a cycle, not whether a
    /// routine may come out at all.
    #[test]
    fn the_bag_plays_everything_once_a_cycle() {
        use engine::animate::character::CharacterAnimation;
        let mut c = CharacterAnimation::new();
        c.deal(CAST[0].meta_for_test(), 0, false);
        let mut dealt = vec![];
        let mut seen = 0u64;
        let total = 60 * 60 * 12;
        for tick in 0..total {
            let through = tick as f64 / total as f64;
            c.danger(1.0 - (through * 2.0 - 1.0).abs(), false);
            if c.deals() != seen {
                seen = c.deals();
                if let Some(i) = c.routine() {
                    dealt.push(i);
                }
            }
            c.update(std::time::Duration::from_nanos(16_666_667));
        }
        assert!(
            dealt.len() >= CHOICES.len() * 3,
            "only {} dealings",
            dealt.len()
        );
        for cycle in dealt.chunks(CHOICES.len()) {
            if cycle.len() < CHOICES.len() {
                continue;
            }
            let mut seen = cycle.to_vec();
            seen.sort_unstable();
            seen.dedup();
            assert_eq!(
                seen.len(),
                CHOICES.len(),
                "a cycle repeated instead of dealing everything: {cycle:?}"
            );
        }
    }

    /// ... and the pool spans the whole range, or the intensity is not driving anything.
    #[test]
    fn the_pool_runs_from_still_to_the_top_of_the_arch() {
        let art = CAST[0].routines.expect("Kirby plays routines");
        let low = art.choices.iter().map(|c| c.intensity).fold(1.0, f64::min);
        let high = art.choices.iter().map(|c| c.intensity).fold(0.0, f64::max);
        assert!(low <= 0.05, "nothing in the pool stays on the floor");
        assert!(high >= 0.95, "nothing in the pool leaves the arch");
    }

    /// The blink is the filler and is **not** in the pool: dealt like a routine it is a
    /// fourteenth of what he does and reads as a loop.
    #[test]
    fn the_blink_is_filler_and_not_a_choice() {
        let art = CAST[0].routines.expect("Kirby plays routines");
        assert!(art.filler.is_some());
        assert!(
            !art.choices
                .iter()
                .any(|c| c.ways.iter().any(|w| w.frames == BLINK)),
            "the blink is in the pool as well as being the filler"
        );
    }

    /// **The game only ever plays a routine where the whole of it fits**, which is why nothing
    /// here is shifted to make one fit: every one of the fifteen was captured being started
    /// from an origin inside its own window. Shifting was what put a jump between the end of
    /// one routine and the start of the next.
    #[test]
    fn the_recording_started_every_routine_inside_its_own_window() {
        // where the capture had him standing when it played each one, by the routine it named
        const CAUGHT_AT: &[(&str, i32)] = &[
            ("yawn", 3),
            ("walk", 2),
            ("flop", 9),
            ("ricochet", 9),
            ("climb", 6),
            ("float-up", 30),
            ("spring", 32),
            ("takeoff", 32),
            ("jump-right", 6),
            ("jump-left", 23),
            ("spin-right", 9),
            ("spin-left", 30),
            ("tumble-right", 12),
            ("tumble-left", 23),
        ];
        let named: &[(&str, Routine)] = &[
            ("yawn", YAWN),
            ("walk", WALK),
            ("flop", FLOP),
            ("ricochet", RICOCHET),
            ("climb", CLIMB),
            ("float-up", FLOAT_UP),
            ("spring", SPRING),
            ("takeoff", TAKEOFF),
            ("jump-right", JUMP_RIGHT),
            ("jump-left", JUMP_LEFT),
            ("spin-right", SPIN_RIGHT),
            ("spin-left", SPIN_LEFT),
            ("tumble-right", TUMBLE_RIGHT),
            ("tumble-left", TUMBLE_LEFT),
        ];
        for (name, at) in CAUGHT_AT {
            let frames = named
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, f)| *f)
                .unwrap_or_else(|| panic!("{name} is not a routine"));
            let way = CHOICES
                .iter()
                .flat_map(|c| c.ways)
                .find(|w| w.frames == frames)
                .unwrap_or_else(|| panic!("{name} is in no kind"));
            assert!(
                *at >= way.origins.0 && *at <= way.origins.1,
                "{name} was played from {at}, outside its window {:?}",
                way.origins
            );
        }
    }

    /// From anywhere he can stand there is always something to deal, or he stops.
    #[test]
    fn there_is_always_something_to_play_from_anywhere() {
        for home in WANDER.0..=WANDER.1 {
            let any = CHOICES.iter().any(|c| {
                c.ways
                    .iter()
                    .any(|w| home >= w.origins.0 && home <= w.origins.1)
            });
            assert!(any, "nothing can be played from {home}");
        }
    }

    /// Three minutes of a quiet board: he should use most of the arch and most of the pool.
    ///
    /// This is the regression test for a character who *stood still*. A routine's displacement
    /// used to be read off whatever was playing when the rest ran out - and the blink replaces
    /// the run, so half the time a walk that carried him twenty pixels put him straight back
    /// where it started. He covered the middle of the arch and nothing else.
    #[test]
    fn a_quiet_board_still_moves_him_about_and_deals_widely() {
        use engine::animate::character::{CharacterAnimation, CharacterFrame};
        let mut c = CharacterAnimation::new();
        c.deal(CAST[0].meta_for_test(), 0, false);
        c.danger(0.15, false);
        let mut counts = vec![0usize; CHOICES.len()];
        let mut seen = 0u64;
        let (mut low, mut high) = (i32::MAX, i32::MIN);
        let mut last = usize::MAX;
        for _ in 0..(60 * 180) {
            if c.deals() != seen {
                seen = c.deals();
                if let Some(i) = c.routine() {
                    assert_ne!(i, last, "the same routine twice running");
                    counts[i] += 1;
                    last = i;
                }
            }
            if let CharacterFrame::Placed(_, at) = c.drawing() {
                low = low.min(at.0);
                high = high.max(at.0);
            }
            c.update(std::time::Duration::from_nanos(16_666_667));
        }
        let span = high - low;
        assert!(
            span >= (WANDER.1 - WANDER.0) * 3 / 4,
            "he only used {span} of the arch's {}",
            WANDER.1 - WANDER.0
        );
        // and **everything** should turn up, not merely the quiet ones: the bag is drained
        // strictly, so three minutes is enough for at least one of every routine there is
        for (i, n) in counts.iter().enumerate() {
            assert!(*n >= 1, "routine {i} was never dealt in three minutes");
        }
    }

    /// **Nothing moves him between routines.** Ninety seconds of the intensity sweep the
    /// `loop` capture uses, watching every tick for a horizontal step bigger than any routine
    /// takes on its own.
    ///
    /// This is the regression test for two separate ways of losing his position, both of which
    /// looked like a character teleporting. A routine used to be *shifted* to make it fit,
    /// which jumped him from where the last one left him to wherever the next needed to start;
    /// and the drawn place was read off `home` rather than the run's own origin, so the blink
    /// stood where the routine before it had *started* instead of where it finished.
    ///
    /// `ricochet` is exempt because its hops are the animation: it throws him wall to wall
    /// across the arch, twenty seven pixels at a time, and the recording does the same.
    #[test]
    fn nothing_moves_him_between_routines() {
        use engine::animate::character::{CharacterAnimation, CharacterFrame};
        // whatever hops on its own is exempt: `ricochet` throws him wall to wall and the
        // recording does the same. Found rather than named, so a routine that grows a hop
        // later does not fail this for the wrong reason.
        let hops: Vec<usize> = CHOICES
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.ways.iter().any(|way| {
                    way.frames
                        .windows(2)
                        .any(|w| (w[1].at.0 - w[0].at.0).abs() > 4)
                })
            })
            .map(|(i, _)| i)
            .collect();
        assert_eq!(
            hops.len(),
            1,
            "expected one hopping routine, found {hops:?}"
        );
        let mut c = CharacterAnimation::new();
        c.deal(CAST[0].meta_for_test(), 0, false);
        let mut last: Option<i32> = None;
        let mut was_ricochet = false;
        let total = 60 * 90;
        for tick in 0..total {
            let through = tick as f64 / total as f64;
            c.danger(1.0 - (through * 2.0 - 1.0).abs(), false);
            if tick % (60 * 15) == 60 * 7 {
                c.chained();
            }
            let now_ricochet = c.routine().is_some_and(|i| hops.contains(&i));
            if let CharacterFrame::Placed(_, at) = c.drawing() {
                if let (Some(before), false, false) = (last, now_ricochet, was_ricochet) {
                    assert!(
                        (at.0 - before).abs() <= 4,
                        "tick {tick}: he moved {} pixels in one tick, {before} to {}",
                        (at.0 - before).abs(),
                        at.0
                    );
                }
                last = Some(at.0);
            }
            was_ricochet = now_ricochet;
            c.update(std::time::Duration::from_nanos(16_666_667));
        }
    }
}
