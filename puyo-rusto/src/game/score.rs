//! Puyo Puyo Tsu's scoring, and the nuisance a score buys.
//!
//! Every table here is the game's own, read off Puyo Nexus rather than guessed at:
//! [Scoring](https://puyonexus.com/wiki/Scoring),
//! [List of attack powers](https://puyonexus.com/wiki/List_of_attack_powers),
//! [Tsu (rule)](https://puyonexus.com/wiki/Tsu_(rule)) and
//! [All clear](https://puyonexus.com/wiki/All_clear), read 2026-08-27. The unit tests at the
//! bottom check them against the worked chain scores that page publishes, which is what makes
//! "faithful" something the build can check rather than something this comment asserts.

/// Chain power by chain length, Puyo Puyo Tsu, **multiplayer**.
///
/// Tsu publishes two tables: this one, and a stiffer single player curve
/// (`4, 20, 24, 32, 48, 96, 160, ...`). This game uses the multiplayer one throughout, in one
/// player as well as two, because the attack economy is the reason the compendium took this
/// game on and one table is one behaviour to test. The consequence is that a solo marathon
/// score is lower than the arcade would have shown for the same chain.
///
/// Index 0 is a 1-chain. A chain longer than the table stays at the last entry.
pub const CHAIN_POWER: [u32; 24] = [
    0, 8, 16, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448, 480, 512, 544,
    576, 608, 640, 672,
];

/// Colour bonus by how many *different* colours the step cleared, classic scoring. Index 0 is
/// one colour. Tsu also defines six colours as 48, which this game cannot reach: it deals five.
pub const COLOR_BONUS: [u32; 5] = [0, 3, 6, 12, 24];

/// Group bonus by how many puyos were in the group, classic scoring. Index 0 is a group of
/// [`PUYOS_TO_POP`]; anything bigger than the table scores the last entry.
pub const GROUP_BONUS: [u32; 8] = [0, 2, 3, 4, 5, 6, 7, 10];

/// how many of a colour have to touch before they pop
pub const PUYOS_TO_POP: u32 = 4;

/// `(CP + CB + GB)` is held between these, so a 1-chain of one colour still scores something
pub const MIN_MULTIPLIER: u32 = 1;
pub const MAX_MULTIPLIER: u32 = 999;

/// Score per nuisance puyo under Tsu rules. Dividing a chain's score by this is what turns it
/// into an attack; the remainder is carried rather than thrown away.
pub const TARGET_POINTS: u32 = 70;

/// What an all clear is worth: thirty extra nuisance - a whole rock - on the *next* chain,
/// not on the one that emptied the board.
pub const ALL_CLEAR_NUISANCE: u32 = 30;

/// one group of one colour, popped in one step of a chain
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PoppedGroup {
    pub color: crate::game::cell::PuyoColor,
    pub size: u32,
}

/// the group bonus a group of `size` is worth
fn group_bonus(size: u32) -> u32 {
    let index = size.saturating_sub(PUYOS_TO_POP) as usize;
    GROUP_BONUS[index.min(GROUP_BONUS.len() - 1)]
}

/// the chain power of the `chain`th step of a chain, counting from 1
pub fn chain_power(chain: u32) -> u32 {
    let index = chain.max(1) as usize - 1;
    CHAIN_POWER[index.min(CHAIN_POWER.len() - 1)]
}

/// the colour bonus for having cleared `colors` different colours in one step
pub fn color_bonus(colors: u32) -> u32 {
    let index = colors.max(1) as usize - 1;
    COLOR_BONUS[index.min(COLOR_BONUS.len() - 1)]
}

/// What one step of a chain scores: `(10 * puyos) * clamp(CP + CB + GB, 1, 999)`.
///
/// `chain` counts from 1 for the first step. Nuisance puyos cleared alongside the groups are
/// *not* part of `groups` and do not score - only coloured puyos count towards `PC`.
pub fn step_score(chain: u32, groups: &[PoppedGroup]) -> u32 {
    if groups.is_empty() {
        return 0;
    }
    let puyos: u32 = groups.iter().map(|group| group.size).sum();
    let mut colors: Vec<crate::game::cell::PuyoColor> =
        groups.iter().map(|group| group.color).collect();
    colors.sort();
    colors.dedup();

    let bonus: u32 = chain_power(chain)
        + color_bonus(colors.len() as u32)
        + groups
            .iter()
            .map(|group| group_bonus(group.size))
            .sum::<u32>();
    10 * puyos * bonus.clamp(MIN_MULTIPLIER, MAX_MULTIPLIER)
}

/// The running remainder of the nuisance division, carried between placements.
///
/// Tsu divides a chain's score by the target points and keeps the fraction: a chain worth 1.70
/// nuisance sends one puyo now and hands 0.70 on to the next one. Kept in *points* rather than
/// as a float so that it is exact.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NuisancePoints {
    leftover: u32,
}

impl NuisancePoints {
    /// how many nuisance puyos `score` buys, carrying the remainder into the next call
    pub fn take(&mut self, score: u32) -> u32 {
        let total = score + self.leftover;
        self.leftover = total % TARGET_POINTS;
        total / TARGET_POINTS
    }

    /// the carried remainder, in points
    pub fn leftover(&self) -> u32 {
        self.leftover
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::cell::PuyoColor;

    fn group(color: PuyoColor, size: u32) -> PoppedGroup {
        PoppedGroup { color, size }
    }

    /// a plain group of four, one colour: 10 * 4 * clamp(0 + 0 + 0, 1, 999) = 40
    #[test]
    fn a_single_group_of_four_scores_forty() {
        assert_eq!(step_score(1, &[group(PuyoColor::Red, 4)]), 40);
    }

    /// Puyo Nexus's *List of Chain Scores*: the published per-step points for a chain made
    /// entirely of four-puyo links. This is the table that makes "faithful" checkable.
    ///
    /// Those figures are quoted for the single player attack powers, so they are recomputed
    /// here from the multiplayer table this game uses - the arithmetic is the thing under
    /// test, and the shape (40, then chain power taking over) has to match.
    #[test]
    fn each_step_of_a_four_link_chain_scores_its_chain_power() {
        for chain in 1..=24 {
            let expected = 10 * 4 * chain_power(chain).clamp(MIN_MULTIPLIER, MAX_MULTIPLIER);
            assert_eq!(
                step_score(chain, &[group(PuyoColor::Red, 4)]),
                expected,
                "chain {chain}"
            );
        }
        // the first step has no chain power at all, so the clamp is what scores it
        assert_eq!(step_score(1, &[group(PuyoColor::Red, 4)]), 40);
        // ... and the second is the first with a chain power of 8
        assert_eq!(step_score(2, &[group(PuyoColor::Red, 4)]), 320);
        assert_eq!(step_score(3, &[group(PuyoColor::Red, 4)]), 640);
        assert_eq!(step_score(4, &[group(PuyoColor::Red, 4)]), 1280);
        assert_eq!(step_score(5, &[group(PuyoColor::Red, 4)]), 2560);
    }

    /// two colours popping together add a colour bonus of 3
    #[test]
    fn clearing_two_colours_at_once_pays_a_colour_bonus() {
        let groups = [group(PuyoColor::Red, 4), group(PuyoColor::Blue, 4)];
        // 10 * 8 * clamp(0 + 3 + 0 + 0) = 240
        assert_eq!(step_score(1, &groups), 240);
        assert_eq!(color_bonus(1), 0);
        assert_eq!(color_bonus(2), 3);
        assert_eq!(color_bonus(3), 6);
        assert_eq!(color_bonus(4), 12);
        assert_eq!(color_bonus(5), 24);
    }

    /// ... and two groups of the *same* colour do not
    #[test]
    fn two_groups_of_one_colour_pay_no_colour_bonus() {
        let groups = [group(PuyoColor::Red, 4), group(PuyoColor::Red, 4)];
        // 10 * 8 * clamp(0 + 0 + 0 + 0) = 80
        assert_eq!(step_score(1, &groups), 80);
    }

    #[test]
    fn a_bigger_group_pays_a_group_bonus() {
        // 10 * 5 * clamp(0 + 0 + 2) = 100
        assert_eq!(step_score(1, &[group(PuyoColor::Red, 5)]), 100);
        // 10 * 11 * clamp(0 + 0 + 10) = 1100, and the table stops there
        assert_eq!(step_score(1, &[group(PuyoColor::Red, 11)]), 1100);
        assert_eq!(
            step_score(1, &[group(PuyoColor::Red, 12)]),
            10 * 12 * 10,
            "the group bonus caps at eleven puyos"
        );
        assert_eq!(group_bonus(4), 0);
        assert_eq!(group_bonus(10), 7);
    }

    /// The floor is what does the work here. A 1-chain of one colour has a chain power of 0,
    /// no colour bonus and no group bonus, so without the floor it would score nothing at all;
    /// clamping to 1 is what makes it worth 40.
    ///
    /// The ceiling, on the other hand, cannot be reached in this game. The biggest step a
    /// 6x13 board can hold is five colours in groups of eleven - 672 + 24 + 50 = 746 - and
    /// the multiplayer chain power tops out at 672, so `MAX_MULTIPLIER` is carried for
    /// fidelity to the formula rather than because a match will ever meet it.
    #[test]
    fn the_multiplier_floors_at_one_so_a_first_link_still_scores() {
        assert_eq!(chain_power(1) + color_bonus(1) + group_bonus(4), 0);
        assert_eq!(step_score(1, &[group(PuyoColor::Red, 4)]), 40);

        let biggest: Vec<PoppedGroup> = PuyoColor::ALL
            .iter()
            .map(|color| group(*color, 11))
            .collect();
        let multiplier = chain_power(24)
            + color_bonus(5)
            + biggest.iter().map(|g| group_bonus(g.size)).sum::<u32>();
        assert_eq!(multiplier, 746);
        assert!(
            multiplier < MAX_MULTIPLIER,
            "the ceiling stays out of reach"
        );
        assert_eq!(step_score(24, &biggest), 10 * 55 * multiplier);
    }

    #[test]
    fn the_chain_power_table_runs_out_at_its_last_entry() {
        assert_eq!(chain_power(1), 0);
        assert_eq!(chain_power(24), 672);
        assert_eq!(chain_power(99), 672, "past the table it stays at the end");
    }

    /// nothing cleared is worth nothing, rather than the clamp paying out
    #[test]
    fn clearing_nothing_scores_nothing() {
        assert_eq!(step_score(1, &[]), 0);
    }

    /// Puyo Nexus's worked example: 1.70 nuisance sends one puyo and carries 0.70
    #[test]
    fn the_nuisance_remainder_is_carried_not_discarded() {
        let mut points = NuisancePoints::default();
        assert_eq!(points.take(119), 1, "119 / 70 is 1.70");
        assert_eq!(points.leftover(), 49, "0.70 of a puyo, in points");
        // the carry then tops the next chain up over the line
        assert_eq!(points.take(21), 1, "21 + 49 is exactly 70");
        assert_eq!(points.leftover(), 0);
    }

    #[test]
    fn a_chain_worth_less_than_a_puyo_sends_nothing_yet() {
        let mut points = NuisancePoints::default();
        assert_eq!(points.take(40), 0, "a single group of four sends nothing");
        assert_eq!(points.leftover(), 40);
        // ... but two of them do
        assert_eq!(points.take(40), 1);
        assert_eq!(points.leftover(), 10);
    }

    /// a two chain of four-links is 360 points, which is five nuisance and a carry
    #[test]
    fn a_two_chain_sends_five_nuisance() {
        let mut points = NuisancePoints::default();
        let score = step_score(1, &[group(PuyoColor::Red, 4)])
            + step_score(2, &[group(PuyoColor::Blue, 4)]);
        assert_eq!(score, 360);
        assert_eq!(points.take(score), 5);
        assert_eq!(points.leftover(), 10);
    }

    /// Puyo Nexus's *List of Chain Scores*, every row of it.
    ///
    /// The published table runs to a nineteen chain - which is also the longest chain the game
    /// can be made to produce - and gives the running total in points and the nuisance it
    /// buys, for a chain made entirely of four-puyo links. Reproducing all nineteen from the
    /// tables in this module is the strongest check there is that the chain power curve, the
    /// clamp, the target points and the carry are all the game's own and not something that
    /// merely agrees with it for the first few links.
    ///
    /// (The page says it assumes the single player attack powers. It does not: its own
    /// figures - 40, 320, 640, 1280, 2560 - are `10 * 4 *` this module's multiplayer curve,
    /// and the single player one would open at 160. The numbers are what is being followed
    /// here, not the caption.)
    #[test]
    fn the_whole_published_table_of_chain_scores_comes_back_out() {
        const PUBLISHED: [(u32, u32, u32); 19] = [
            // chain length, total points, total nuisance
            (1, 40, 0),
            (2, 360, 5),
            (3, 1000, 14),
            (4, 2280, 32),
            (5, 4840, 69),
            (6, 8680, 124),
            (7, 13800, 197),
            (8, 20200, 288),
            (9, 27880, 398),
            (10, 36840, 526),
            (11, 47080, 672),
            (12, 58600, 837),
            (13, 71400, 1020),
            (14, 85480, 1221),
            (15, 100840, 1440),
            (16, 117480, 1678),
            (17, 135400, 1934),
            (18, 154600, 2208),
            (19, 175080, 2501),
        ];
        for (length, expected_score, expected_nuisance) in PUBLISHED {
            let score: u32 = (1..=length)
                .map(|chain| step_score(chain, &[group(PuyoColor::Red, 4)]))
                .sum();
            assert_eq!(score, expected_score, "{length} chain, points");
            let mut points = NuisancePoints::default();
            assert_eq!(
                points.take(score),
                expected_nuisance,
                "{length} chain, nuisance"
            );
        }
    }

    /// the group bonus is per group and the colour bonus is per colour, so a step with two
    /// unequal groups of different colours pays one of each
    #[test]
    fn group_bonuses_add_up_over_the_groups_of_a_step() {
        let groups = [group(PuyoColor::Red, 5), group(PuyoColor::Blue, 6)];
        // 10 * 11 * clamp(0 + 3 + (2 + 3)) = 880
        assert_eq!(step_score(1, &groups), 880);
        // ... and the same two sizes in one colour lose only the colour bonus
        let one_colour = [group(PuyoColor::Red, 5), group(PuyoColor::Red, 6)];
        assert_eq!(step_score(1, &one_colour), 10 * 11 * 5);
    }
}
