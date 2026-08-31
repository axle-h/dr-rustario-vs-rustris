//! Dr. Mario 64's scoring weights, lifted from `ai_param_org` in `aiset.c`.
//!
//! The original picks a row by *skill* - `aiSelCom`, which a character's `ai_char_data` looks up
//! per situation - and a column by *situation*, which [`super::Situation`] works out from the
//! bottle. The pair names one row of 28 numbers, which `aiSetCharacter` fans out over a dozen
//! global tables. [`Params`] is that fanning out, done once per pill.

use crate::game::ai::n64::Situation;

pub const SKILLS: usize = 6;
const SITUATIONS: usize = 8;
const FIELDS: usize = 28;

/// The six rows worst to best, which is what a difficulty picks from, and which the strongest
/// of - `DEFAULT_SKILL` - is the teacher [`crate::game::ai::imitation`] gathers its corpus
/// from, the defender in the 2-player demo, and what `impossible` plays.
///
/// The rows are personalities rather than a ladder - the original picks one per character, not
/// per skill setting - so the order is measured. **It is measured in the same currency the
/// training fitness uses**: viruses destroyed inside a budget of
/// [`crate::game::ai::run::PILL_BUDGET`] pills, over twenty seeds at each of virus levels 0, 5,
/// 10, 15 and 20 (`ga dr play <seed> <level> 2500 100000 n64:<row>`), which is six hundred
/// whole games. That was not always so - it used to be bottles cleared less four per burial at
/// a 1200 pill cap - and the two disagree at the top, because the old rule charges four bottles
/// for a burial and row 4 buries itself more than twice as often as row 1 while clearing far
/// more than it. Which of those is the better player is exactly the question the pill budget
/// was introduced to answer, so it answers this one too.
///
/// | row | viruses | bottles | burials |
/// |--|--|--|--|
/// | 4 | 68834 | 1138 | 43 |
/// | 1 | 58404 |  990 | 18 |
/// | 2 | 56284 |  959 | 18 |
/// | 0 | 50750 |  920 | 46 |
/// | 5 | 36544 |  701 | 48 |
/// | 3 | 33829 |  656 | 33 |
pub const SKILL_ORDER: [u8; SKILLS] = [3, 5, 0, 2, 1, 4];

/// The row an ai plays when nothing picks one: the best of them.
pub const DEFAULT_SKILL: u8 = SKILL_ORDER[SKILLS - 1];

/// Original name: `ai_param_org`
#[rustfmt::skip]
const AI_PARAM: [[[i32; FIELDS]; SITUATIONS]; SKILLS] = [
    [
        [0, 0, -50, 0, 100, 100, 10, -170, -190, -230, -250, 0, 0, 0, 0, 0, 0, 1000, -400, 0, 0, 0, 194, 490, 490, 0, 0, 0],
        [5, 10, 30, 0, 100, 100, 10, -70, -90, -80, -100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 14, 40, 45, 14, 40, 45],
        [131, 2030, 30, 0, 400, 400, 10, -170, -190, -230, -250, 0, 0, 0, 0, 0, 3000, 0, 0, 0, 0, 0, 14, 40, 45, 0, 0, 0],
        [131, 2030, 30, 0, 400, 400, 10, -370, -390, -430, -450, 0, 0, 0, 0, 0, 3000, 0, 0, 0, 0, 1, 14, 40, 45, 0, 0, 0],
        [431, 2030, 30, 0, 1000, 1000, 10, -70, -90, -80, -100, -360, -420, -540, -480, -500, 3000, 0, 0, 0, 0, 1, 0, 0, 0, 54, 140, 145],
        [5, 10, 30, 0, 0, 500, 10, -70, -90, -80, -100, 0, -140, -180, -160, -200, 0, 0, 0, 50, 50, 0, 0, 0, 0, 114, 240, 245],
        [0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 31, 50, 0, 400, 400, 3, -170, -190, -230, -250, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 14, 40, 40, 0, 0, 0],
    ],
    [
        [0, 0, -50, 0, 100, 100, 10, -170, -190, -230, -250, 0, 0, 0, 0, 0, 0, 1000, -400, 0, 0, 0, 194, 490, 490, 0, 0, 0],
        [5, 10, 30, 0, 100, 100, 10, -70, -90, -80, -100, 0, 0, 0, 0, 0, 0, 500, 0, 0, 0, 0, 14, 40, 45, 14, 40, 45],
        [131, 2030, 30, 0, 400, 400, 10, -170, -190, -230, -250, 0, 0, 0, 0, 0, 3000, 200, -200, 0, 0, 0, 64, 140, 145, 0, 0, 0],
        [131, 2030, 30, 0, 400, 400, 10, -370, -390, -530, -550, 0, 0, 0, 0, 0, 3000, 200, 0, 0, 0, 1, 64, 140, 145, 0, 0, 0],
        [431, 2030, 30, 0, 1000, 1000, 10, -70, -90, -80, -100, -360, -420, -540, -480, -600, 3000, 200, 0, 0, 0, 1, 0, 0, 0, 64, 140, 145],
        [5, 10, 30, 0, 0, 500, 10, -70, -90, -80, -100, 0, -140, -180, -160, -200, 0, 200, 0, 50, 50, 0, 0, 0, 0, 114, 240, 245],
        [31, 50, 30, 0, 400, 400, 10, -370, -390, -430, -450, 0, 0, 0, 0, 0, 0, 3300, 0, 330, 330, 0, 194, 490, 490, 0, 0, 0],
        [31, 50, 30, 0, 400, 400, 10, -370, -390, -430, -450, 0, 0, 0, 0, 0, 0, 300, -200, 0, 0, 0, 194, 490, 490, 0, 0, 0],
    ],
    [
        [0, 0, -50, 0, 100, 100, 10, -170, -190, -230, -250, 0, 0, 0, 0, 0, 0, 1000, -400, 0, 0, 0, 194, 490, 490, 0, 0, 0],
        [5, 10, 30, 0, 100, 100, 10, -70, -90, -80, -100, 0, 0, 0, 0, 0, 0, 500, 0, 0, 0, 0, 14, 40, 45, 14, 40, 45],
        [131, 2030, 30, 0, 400, 400, 10, -170, -190, -230, -250, 0, 0, 0, 0, 0, 3000, 500, -500, 0, 0, 0, 114, 240, 245, 0, 0, 0],
        [131, 2030, 30, 0, 400, 400, 10, -370, -390, -430, -450, 0, 0, 0, 0, 0, 3000, 500, 0, 0, 0, 1, 114, 240, 245, 0, 0, 0],
        [431, 2030, 30, 0, 1000, 1000, 10, -70, -90, -80, -100, -360, -420, -540, -480, -600, 3000, 200, 0, 0, 0, 1, 0, 0, 0, 64, 140, 145],
        [5, 10, 30, 0, 0, 500, 10, -70, -90, -80, -100, 0, -140, -180, -160, -200, 0, 500, 0, 50, 50, 0, 0, 0, 0, 114, 240, 245],
        [31, 50, 30, 0, 400, 400, 10, -170, -190, -230, -250, 0, 0, 0, 0, 0, 0, 4400, 0, 440, 440, 0, 194, 490, 490, 0, 0, 0],
        [31, 50, 30, 0, 400, 400, 30, -170, -190, -230, -250, 0, 0, 0, 0, 0, 0, 1400, -800, 0, 0, 0, 194, 490, 490, 0, 0, 0],
    ],
    [
        [0, 0, -50, 0, 100, 100, 10, -170, -190, -230, -250, 0, 0, 0, 0, 0, 0, 1000, -400, 0, 0, 0, 194, 490, 490, 0, 0, 0],
        [5, 10, 30, 0, 100, 100, 10, -70, -90, -80, -100, 0, 0, 0, 0, 0, 0, 500, 0, 0, 0, 0, 14, 40, 45, 14, 40, 45],
        [131, 2030, 30, 0, 400, 400, 10, -170, -190, -230, -250, 0, 0, 0, 0, 0, 3000, 500, -500, 0, 0, 0, 114, 240, 245, 0, 0, 0],
        [131, 2030, 30, 0, 400, 400, 10, -370, -390, -430, -450, 0, 0, 0, 0, 0, 3000, 500, 0, 0, 0, 1, 114, 240, 245, 0, 0, 0],
        [431, 2030, 30, 0, 1000, 1000, 10, -70, -90, -80, -100, -360, -420, -540, -480, -600, 3000, 200, 0, 0, 0, 1, 0, 0, 0, 64, 140, 145],
        [5, 10, 30, 0, 0, 500, 10, -70, -90, -80, -100, 0, -140, -180, -160, -200, 0, 500, 0, 50, 50, 0, 0, 0, 0, 114, 240, 245],
        [0, 0, -50, 0, 100, 100, 10, -170, -190, -230, -250, 0, 0, 0, 0, 0, 0, 4400, 0, 440, 440, 0, 194, 490, 490, 0, 0, 0],
        [0, 0, -50, 0, 100, 100, 100, -170, -190, -230, -250, 0, 0, 0, 0, 0, 0, 1400, -800, 0, 0, 0, 194, 490, 990, 0, 0, 0],
    ],
    [
        [0, 0, -50, 0, 100, 100, 10, -170, -190, -230, -250, 0, 0, 0, 0, 0, 0, 1000, -400, 0, 0, 0, 0, 0, 0, 194, 490, 490],
        [5, 10, 30, 0, 100, 100, 10, -70, -90, -80, -100, 0, 0, 0, 0, 0, 0, 500, 0, 0, 0, 0, 14, 40, 45, 14, 40, 45],
        [131, 2030, 30, 0, 400, 400, 10, -170, -190, -230, -250, 0, 0, 0, 0, 0, 3000, 500, -500, 0, 0, 0, 114, 240, 245, 0, 0, 0],
        [131, 2030, 30, 0, 400, 400, 10, -370, -390, -430, -450, 0, 0, 0, 0, 0, 3000, 500, 0, 0, 0, 1, 114, 240, 245, 0, 0, 0],
        [431, 2030, 30, 0, 1000, 1000, 10, -70, -90, -80, -100, -360, -420, -540, -480, -600, 3000, 200, 0, 0, 0, 1, 0, 0, 0, 64, 140, 145],
        [5, 10, 30, 0, 0, 500, 10, -70, -90, -80, -100, 0, -140, -180, -160, -200, 0, 500, 0, 50, 50, 0, 0, 0, 0, 114, 240, 245],
        [31, 50, 30, 0, 400, 400, 10, -170, -190, -230, -250, 0, 0, 0, 0, 0, 0, 4400, 0, 440, 440, 0, 194, 490, 490, 0, 0, 0],
        [31, 50, 30, 0, 400, 800, 30, -170, -190, -230, -250, 0, -170, -190, -230, -250, 0, 1400, 0, 0, 0, 0, 0, 0, 0, 194, 490, 490],
    ],
    [
        [0, 0, -50, 1000, 100, 100, 10, -170, -190, -230, -250, 0, 0, 0, 0, 0, 0, 0, -400, 0, 0, 0, 194, 490, 490, 194, 490, 490],
        [5, 10, 30, 500, 100, 100, 10, -70, -90, -80, -100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 14, 40, 45, 14, 40, 45],
        [131, 2030, 30, 500, 400, 400, 10, -170, -190, -230, -250, 0, 0, 0, 0, 0, 3000, 0, -500, 0, 0, 0, 114, 240, 245, 0, 0, 0],
        [131, 2030, 30, 500, 400, 400, 10, -370, -390, -430, -450, 0, 0, 0, 0, 0, 3000, 0, 0, 0, 0, 1, 114, 240, 245, 0, 0, 0],
        [431, 2030, 30, 200, 1000, 1000, 10, -70, -90, -80, -100, -360, -420, -540, -480, -600, 3000, 0, 0, 0, 0, 1, 0, 0, 0, 64, 140, 145],
        [5, 10, 30, 500, 0, 500, 10, -70, -90, -80, -100, 0, -140, -180, -160, -200, 0, 0, 0, 50, 50, 0, 0, 0, 0, 114, 240, 245],
        [0, 0, -50, 4400, 100, 100, 10, -170, -190, -230, -250, 0, 0, 0, 0, 0, 0, 0, 0, 440, 440, 0, 194, 490, 490, 0, 0, 0],
        [0, 0, -50, 1400, 100, 100, 100, -170, -190, -230, -250, 0, -170, -190, -230, -250, 0, 700, -800, 0, 0, 0, 194, 490, 990, 194, 490, 990],
    ],
];

/// how much of the priority survives when the bottle is lopsided, by column. Original name:
/// `WallRate`, indexed by which side of the bottle is stacked up
pub const WALL_RATE: [[i32; 8]; 4] = [
    [10, 10, 10, 10, 10, 10, 10, 10],
    [64, 64, 32, 16, 8, 4, 2, 1],
    [1, 2, 4, 8, 16, 32, 64, 64],
    [64, 64, 16, 4, 4, 16, 64, 64],
];

/// what it costs to leave a block this high up, by column. Original names: `bad_point` and
/// `bad_point2`, the second being what it costs to do it with both halves at once
pub const BAD_POINT: [i32; 8] = [-90, -270, -360, -900, -900, -360, -270, -90];
pub const BAD_POINT2: [i32; 8] = [-90, -270, -360, -9000, -9000, -360, -270, -90];

/// The tables `aiSetCharacter` fills in for the pill about to be played.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// what each entry of a `hei`/`wid` row is worth once a line has been made. Original name:
    /// `pri_point`
    pub pri_point: [i32; 9],
    /// what clearing this many lines at once is worth. Original name: `EraseLinP`
    pub erase_lin_p: [i32; 9],
    pub hei_erase_lin_rate: f32,
    pub wid_erase_lin_rate: f32,
    /// what a run of this length that has not gone yet is worth, vertically and horizontally.
    /// Original names: `HeiLinesAllp` and `WidLinesAllp`
    pub hei_lines_allp: [i32; 9],
    pub wid_lines_allp: [i32; 9],
    /// what it costs to strand a half where nothing can join it. Original names: `AloneCapP`
    /// and `AloneCapWP`
    pub alone_cap_p: [i32; 6],
    pub alone_cap_wp: [i32; 6],
    /// what it costs to strand both halves at once, scaled by how high they are. Original
    /// name: `LPriP`
    pub l_pri_p: i32,
    /// what covering a virus is worth, or costs. Original name: `OnVirusP`
    pub on_virus_p: i32,
    /// what a placement that sets up a chain is worth, and what one that only sets up a chain
    /// for a colour that is not coming is worth. Original names: `RensaP` and `RensaMP`
    pub rensa_p: i32,
    pub rensa_mp: i32,
    /// whether the lopsided-bottle multiplier applies at all
    pub wall: bool,
}

impl Params {
    pub fn of(skill: u8, situation: Situation) -> Self {
        let row = &AI_PARAM[(skill as usize).min(SKILLS - 1)][situation as usize];

        let mut erase_lin_p = [0; 9];
        erase_lin_p[1] = row[2];
        erase_lin_p[2] = (row[2] + row[3]) >> 1;
        for value in erase_lin_p.iter_mut().skip(3) {
            *value = row[3];
        }

        let mut hei_lines_allp = [0; 9];
        hei_lines_allp[2] = row[22];
        hei_lines_allp[3] = row[23];
        for value in hei_lines_allp.iter_mut().skip(4) {
            *value = row[24];
        }

        let mut wid_lines_allp = [0; 9];
        wid_lines_allp[2] = row[25];
        wid_lines_allp[3] = row[26];
        for value in wid_lines_allp.iter_mut().skip(4) {
            *value = row[27];
        }

        Self {
            // [2] is never given a value by `aiSetCharacter`, so it keeps the one `pri_point`
            // is declared with; the rest of the untouched entries are zero
            pri_point: [0, row[1], 9, 0, row[0], 0, 0, row[19], row[20]],
            erase_lin_p,
            hei_erase_lin_rate: row[4] as f32 * 0.01,
            wid_erase_lin_rate: row[5] as f32 * 0.01,
            hei_lines_allp,
            wid_lines_allp,
            // [1] keeps the value `AloneCapP` is declared with
            alone_cap_p: [0, -60, row[7], row[8], row[9], row[10]],
            alone_cap_wp: [0, row[11], row[12], row[13], row[14], row[15]],
            l_pri_p: row[6],
            on_virus_p: row[16],
            rensa_p: row[17],
            rensa_mp: row[18],
            wall: row[21] != 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_order_ranks_every_row_once_and_the_default_is_the_best_of_them() {
        let mut rows = SKILL_ORDER;
        rows.sort();
        assert_eq!(rows, [0, 1, 2, 3, 4, 5], "a row is missing or ranked twice");
        // worst to best, so the last is the strongest - which is the row every one-row decision
        // in the crate takes: the ai a difficulty tops out at, the demo's defender, and the
        // teacher `imitation::lessons` gathers its corpus from
        assert_eq!(DEFAULT_SKILL, SKILL_ORDER[SKILLS - 1]);
    }
}
