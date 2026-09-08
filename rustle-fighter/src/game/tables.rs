//! The game's own data tables, transcribed from `SLUS_004.18`.
//!
//! Every table here was read out of the PlayStation executable at the address named above it,
//! not copied from a guide. They are in one module because they are *evidence*: the whole
//! roster's drop patterns are here including the four characters we do not field, because
//! cutting the table down would be editing the record. The rules doc's *Provenance* has the
//! recipe for reproducing the extraction in about a minute.

use crate::game::board::COLUMNS;

/// how many characters the drop pattern table holds - the whole arcade roster
pub const CHARACTERS: usize = 11;
pub const PATTERN_ROWS: usize = 12;
pub const PATTERN_COLUMNS: usize = 6;
/// how many permutations of the six columns single-gem delivery draws from
pub const ORDERINGS: usize = 8;
/// how many weighted gem distributions there are, and how long each is
pub const GEM_TABLES_COUNT: usize = 8;
pub const TABLE_ENTRIES: usize = 64;
/// the second half of a pair draws from only the **first half** of the table, so the two
/// halves of one pair have different distributions
pub const SECOND_HALF_ENTRIES: usize = 32;
/// how long each of the opening tables is - half a settled one, and drawn whole by both halves
pub const EARLY_TABLE_ENTRIES: usize = 32;

/// `0x8016E684`, 11 characters x 12 rows x 6 columns, one colour index per cell. The colour
/// numbering is [`crate::game::cell::GemColor`]'s: 1 blue, 2 yellow, 3 green, 4 red.
pub const DROP_PATTERNS: [[[u8; PATTERN_COLUMNS]; PATTERN_ROWS]; CHARACTERS] = [
    // 0 - Morrigan
    [
        [1, 2, 4, 4, 2, 1],
        [1, 2, 4, 4, 2, 1],
        [2, 1, 3, 3, 1, 2],
        [2, 1, 3, 3, 1, 2],
        [4, 3, 1, 1, 3, 4],
        [4, 3, 1, 1, 3, 4],
        [3, 4, 2, 2, 4, 3],
        [3, 4, 2, 2, 4, 3],
        [1, 2, 4, 4, 2, 1],
        [1, 2, 4, 4, 2, 1],
        [2, 1, 3, 3, 1, 2],
        [2, 1, 3, 3, 1, 2],
    ],
    // 1 - Chun-Li
    [
        [4, 4, 3, 3, 1, 1],
        [4, 4, 3, 3, 1, 1],
        [2, 2, 4, 4, 3, 3],
        [2, 2, 4, 4, 3, 3],
        [1, 1, 2, 2, 4, 4],
        [1, 1, 2, 2, 4, 4],
        [3, 3, 1, 1, 2, 2],
        [3, 3, 1, 1, 2, 2],
        [4, 4, 3, 3, 1, 1],
        [4, 4, 3, 3, 1, 1],
        [2, 2, 4, 4, 3, 3],
        [2, 2, 4, 4, 3, 3],
    ],
    // 2 - Ryu
    [
        [4, 3, 1, 2, 4, 3],
        [4, 3, 1, 2, 4, 3],
        [4, 3, 1, 2, 4, 3],
        [4, 3, 1, 2, 4, 3],
        [4, 3, 1, 2, 4, 3],
        [4, 3, 1, 2, 4, 3],
        [4, 3, 1, 2, 4, 3],
        [4, 3, 1, 2, 4, 3],
        [4, 3, 1, 2, 4, 3],
        [4, 3, 1, 2, 4, 3],
        [4, 3, 1, 2, 4, 3],
        [4, 3, 1, 2, 4, 3],
    ],
    // 3 - Ken
    [
        [4, 4, 4, 4, 4, 4],
        [3, 3, 3, 3, 3, 3],
        [1, 1, 1, 1, 1, 1],
        [2, 2, 2, 2, 2, 2],
        [4, 4, 4, 4, 4, 4],
        [3, 3, 3, 3, 3, 3],
        [1, 1, 1, 1, 1, 1],
        [2, 2, 2, 2, 2, 2],
        [4, 4, 4, 4, 4, 4],
        [3, 3, 3, 3, 3, 3],
        [1, 1, 1, 1, 1, 1],
        [2, 2, 2, 2, 2, 2],
    ],
    // 4 - Hsien-Ko
    [
        [2, 1, 1, 3, 3, 4],
        [1, 1, 3, 3, 4, 4],
        [1, 3, 3, 4, 4, 2],
        [3, 3, 4, 4, 2, 2],
        [3, 4, 4, 2, 2, 1],
        [4, 4, 2, 2, 1, 1],
        [4, 2, 2, 1, 1, 3],
        [2, 2, 1, 1, 3, 3],
        [2, 1, 1, 3, 3, 4],
        [1, 1, 3, 3, 4, 4],
        [1, 3, 3, 4, 4, 2],
        [3, 3, 4, 4, 2, 2],
    ],
    // 5 - Donovan
    [
        [4, 2, 4, 2, 4, 2],
        [4, 2, 4, 2, 4, 2],
        [3, 3, 3, 1, 1, 1],
        [3, 3, 3, 1, 1, 1],
        [4, 2, 4, 2, 4, 2],
        [4, 2, 4, 2, 4, 2],
        [3, 3, 3, 1, 1, 1],
        [3, 3, 3, 1, 1, 1],
        [4, 2, 4, 2, 4, 2],
        [4, 2, 4, 2, 4, 2],
        [3, 3, 3, 1, 1, 1],
        [3, 3, 3, 1, 1, 1],
    ],
    // 6 - Felicia
    [
        [3, 4, 4, 1, 1, 2],
        [3, 4, 4, 1, 1, 2],
        [3, 1, 1, 4, 4, 2],
        [3, 1, 1, 4, 4, 2],
        [3, 4, 4, 1, 1, 2],
        [3, 4, 4, 1, 1, 2],
        [3, 1, 1, 4, 4, 2],
        [3, 1, 1, 4, 4, 2],
        [3, 4, 4, 1, 1, 2],
        [3, 4, 4, 1, 1, 2],
        [3, 1, 1, 4, 4, 2],
        [3, 1, 1, 4, 4, 2],
    ],
    // 7 - Sakura
    [
        [3, 4, 4, 4, 4, 2],
        [3, 1, 1, 1, 1, 2],
        [3, 4, 4, 4, 4, 2],
        [3, 1, 1, 1, 1, 2],
        [3, 4, 4, 4, 4, 2],
        [3, 1, 1, 1, 1, 2],
        [3, 4, 4, 4, 4, 2],
        [3, 1, 1, 1, 1, 2],
        [3, 4, 4, 4, 4, 2],
        [3, 1, 1, 1, 1, 2],
        [3, 4, 4, 4, 4, 2],
        [3, 1, 1, 1, 1, 2],
    ],
    // 8 - Devilot
    [
        [4, 3, 1, 2, 4, 3],
        [2, 4, 3, 1, 2, 4],
        [1, 2, 4, 3, 1, 2],
        [3, 1, 2, 4, 3, 1],
        [4, 3, 1, 2, 4, 3],
        [2, 4, 3, 1, 2, 4],
        [1, 2, 4, 3, 1, 2],
        [3, 1, 2, 4, 3, 1],
        [4, 3, 1, 2, 4, 3],
        [2, 4, 3, 1, 2, 4],
        [1, 2, 4, 3, 1, 2],
        [3, 1, 2, 4, 3, 1],
    ],
    // 9 - Akuma
    [
        [3, 4, 2, 1, 3, 4],
        [1, 3, 4, 2, 1, 3],
        [2, 1, 3, 4, 2, 1],
        [4, 2, 1, 3, 4, 2],
        [3, 4, 2, 1, 3, 4],
        [1, 3, 4, 2, 1, 3],
        [2, 1, 3, 4, 2, 1],
        [4, 2, 1, 3, 4, 2],
        [3, 4, 2, 1, 3, 4],
        [1, 3, 4, 2, 1, 3],
        [2, 1, 3, 4, 2, 1],
        [4, 2, 1, 3, 4, 2],
    ],
    // 10 - Dan
    [
        [4, 4, 4, 4, 4, 4],
        [4, 4, 4, 4, 4, 4],
        [4, 4, 4, 4, 4, 4],
        [4, 4, 4, 4, 4, 4],
        [4, 4, 4, 4, 4, 4],
        [4, 4, 4, 4, 4, 4],
        [4, 4, 4, 4, 4, 4],
        [4, 4, 4, 4, 4, 4],
        [4, 4, 4, 4, 4, 4],
        [4, 4, 4, 4, 4, 4],
        [4, 4, 4, 4, 4, 4],
        [4, 4, 4, 4, 4, 4],
    ],
];

/// `DAT_8016E524`: eight permutations of the six columns, one picked per round with `rng & 7`.
/// The Drop Alley is **last in all eight**.
pub const COLUMN_ORDERINGS: [[usize; COLUMNS as usize]; ORDERINGS] = [
    [0, 1, 2, 5, 4, 3],
    [5, 4, 0, 1, 2, 3],
    [0, 5, 2, 4, 1, 3],
    [1, 2, 4, 5, 0, 3],
    [4, 1, 5, 0, 2, 3],
    [5, 0, 2, 1, 4, 3],
    [0, 1, 2, 4, 5, 3],
    [5, 4, 2, 1, 0, 3],
];

/// `0x8016D81C`: **eight** 64 entry weighted draws, values 1-4 for a plain gem of that colour
/// and 9-12 for its crash gem. Six favour two colours (12 entries) over the other two (7) -
/// one for each of the six pairs of four colours - and the last two are flat at nine each.
/// The biased six hold 26 crash gems in 64 and 11 in their first 32, the flat two 28 and 12,
/// so crash gems are 40.6% of the first half's draw and 34.4% of the second's whichever table
/// is in play.
///
/// These are what the game settles into. For the first [`crate::game::random::EARLY_PAIRS`]
/// pairs of a round it deals from [`EARLY_GEM_TABLES`] instead, which hold no crash gems at
/// all.
pub const GEM_TABLES: [[u8; TABLE_ENTRIES]; GEM_TABLES_COUNT] = [
    [
        1, 1, 1, 1, 1, 2, 3, 4, 1, 2, 3, 4, 1, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4,
        9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11,
        12, 2, 2, 2, 2, 9, 2, 11, 12,
    ],
    [
        1, 2, 3, 4, 1, 2, 3, 4, 2, 2, 2, 2, 9, 2, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9,
        10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11,
        12, 3, 3, 3, 3, 9, 10, 3, 12,
    ],
    [
        1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 9, 10, 3, 12, 3, 3, 3, 3, 9, 10, 11, 12, 1, 2, 3, 4, 9,
        10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11,
        12, 4, 4, 4, 4, 9, 10, 11, 4,
    ],
    [
        1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 9, 10, 11, 4, 1, 2, 3, 4, 9, 10, 11, 12, 4, 4, 4, 4, 9,
        10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11,
        12, 1, 1, 1, 1, 1, 10, 11, 12,
    ],
    [
        1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 9, 10, 3, 12, 1, 2, 3, 4, 9, 10, 11, 12, 3, 3, 3, 3, 9,
        10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11,
        12, 1, 1, 1, 1, 1, 10, 11, 12,
    ],
    [
        1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 9, 10, 11, 4, 1, 2, 3, 4, 9, 10, 11, 12, 4, 4, 4, 4, 9,
        10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11,
        12, 2, 2, 2, 2, 2, 10, 11, 12,
    ],
    [
        1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4,
        9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11,
        12, 1, 2, 3, 4, 9, 10, 11, 12,
    ],
    [
        1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4,
        9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11,
        12, 1, 2, 3, 4, 9, 10, 11, 12,
    ],
];

/// `0x8016DA1C`: the **opening** tables, eight 32 entry draws that hold **no crash gems at
/// all** - the same six colour biases and two flat tables as [`GEM_TABLES`], `+0x292` picking
/// between them the same way. While a round is inside its first
/// [`crate::game::random::EARLY_PAIRS`] pairs the pivot is drawn from here, so it cannot be a
/// crash gem and only the child can carry one.
pub const EARLY_GEM_TABLES: [[u8; EARLY_TABLE_ENTRIES]; GEM_TABLES_COUNT] = [
    [
        1, 1, 1, 1, 1, 1, 3, 4, 2, 2, 2, 2, 2, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2,
        3, 4,
    ],
    [
        1, 1, 1, 1, 1, 1, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 3, 3, 3, 3, 3, 2, 3, 4, 1, 2, 3, 4, 1, 2,
        3, 4,
    ],
    [
        1, 1, 1, 1, 1, 1, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 4, 4, 4, 4, 4, 2,
        3, 4,
    ],
    [
        1, 2, 3, 4, 1, 2, 3, 4, 2, 2, 2, 2, 2, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 4, 4, 4, 4, 4, 2,
        3, 4,
    ],
    [
        1, 2, 3, 4, 1, 2, 3, 4, 2, 2, 2, 2, 2, 2, 3, 4, 3, 3, 3, 3, 3, 2, 3, 4, 1, 2, 3, 4, 1, 2,
        3, 4,
    ],
    [
        1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 3, 3, 3, 3, 3, 2, 3, 4, 4, 4, 4, 4, 4, 2,
        3, 4,
    ],
    [
        1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2,
        3, 4,
    ],
    [
        1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2,
        3, 4,
    ],
];

/// `0x8016DB1C`: the child's draw for those opening pairs, and the **only** source of a crash
/// gem in them. One table for all eight biases - the opening leans the pivot's colours, never
/// the crash gem's - and twelve of its thirty two entries are crash gems.
pub const EARLY_CHILD_TABLE: [u8; EARLY_TABLE_ENTRIES] = [
    9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 9, 10, 11, 12, 1, 2, 3, 4, 4, 1, 2, 3, 4,
    1, 2, 3,
];

/// `0x8016DB3C`: the pair number each rainbow gem arrives on. Every 25th pair to 600, then
/// four more at 800-950, and then the table reads 9999 - which is to say never again.
pub const RAINBOW_SCHEDULE: [u32; 28] = [
    25, 50, 75, 100, 125, 150, 175, 200, 225, 250, 275, 300, 325, 350, 375, 400, 425, 450, 475,
    500, 525, 550, 575, 600, 800, 850, 900, 950,
];
