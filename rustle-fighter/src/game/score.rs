//! Points, damage and defence, from the rules doc's *Score*, *Damage* and *Defence and offset*.
//! Seven accumulators sum to a base that is both the score and the sole input to [`damage`].

use crate::game::gems::Erased;

/// `+0x10c`, indexed by `min(chain - 1, 10)`
pub const CHAIN_BONUS: [u32; 11] = [0, 200, 400, 1000, 1600, 2200, 2800, 3400, 4000, 4000, 4000];

/// `+0x10e`: per distinct colour erased beyond the first. The off-by-one is a reading, pinned
/// by `twelve_loose_gems_and_a_crash_gem_score_fourteen_hundred_and_ten`.
pub const COLOR_BONUS: u32 = 200;

/// what one erased cell of each class is worth: `+0x114`, `+0x116` and `+0x118`
pub const PLAIN_POINTS: u32 = 100;
pub const CRASH_POINTS: u32 = 100;
pub const COUNTER_POINTS: u32 = 10;

/// `+0x29a`: a rainbow gem on the floor. Not an accumulator, so it deals no damage.
pub const TECH_BONUS: u32 = 10_000;

pub const ALL_CLEAR_POINTS: u32 = 600;
/// an All Clear's attack is cumulative: `+0x29c` grows by this and all of it is sent
pub const ALL_CLEAR_ATTACK: u32 = 6;

/// `DAT_8016E584`, the per-cell rate the reclaimed-garbage bonus pays at, indexed by `n - 1`
pub const RECLAIMED_STEPS: [u32; 51] = [
    100, 100, 100, 100, 100, 100, 100, 100, 150, 150, 150, 150, 150, 150, 150, 200, 200, 200, 200,
    200, 200, 200, 200, 200, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 300, 300, 300,
    300, 300, 300, 300, 300, 300, 300, 300, 300, 300, 350, 350, 350,
];

/// `0x8016E634`: the difficulty setting's contribution, as `(L + 10) / 100` of a tenth of the
/// scaled base; only the first three are reachable.
pub const LEVELS: [i32; 8] = [-2, 0, 3, 5, 5, 5, 5, 5];

/// the round timer stops here, `+0x1f0`
pub const MAX_ROUND_SECONDS: u32 = 539;

pub const MAX_PENDING: u32 = 250;

/// The difficulty setting, which scales every attack in the game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, strum::EnumIter)]
pub enum Level {
    Easy,
    #[default]
    Normal,
    Hard,
}

impl Level {
    fn factor(self) -> i32 {
        LEVELS[self as usize]
    }
}

/// The seven accumulators, `+0x10c` to `+0x118` in field order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Accumulators {
    pub chain: u32,
    pub color: u32,
    pub count: u32,
    pub reclaimed: u32,
    pub plain: u32,
    pub crash: u32,
    pub counter: u32,
}

impl Accumulators {
    /// What one erase step scored; the first pass of a break is chain 1.
    pub fn of(erased: &Erased, chain: u32) -> Accumulators {
        let count = |f: fn(&crate::game::cell::Gem) -> bool| erased.gems().filter(f).count() as u32;
        let plain = count(|gem| !gem.is_crash() && !gem.is_counter());
        let crash = count(|gem| gem.is_crash());
        let counter = count(|gem| gem.is_counter());
        let reclaimed = count(|gem| gem.is_reclaimed());
        let colors = erased.colors().len() as u32;

        Accumulators {
            chain: CHAIN_BONUS[(chain.max(1) as usize - 1).min(CHAIN_BONUS.len() - 1)],
            color: COLOR_BONUS * colors.saturating_sub(1),
            count: 10 * erased.count().saturating_sub(2),
            reclaimed: reclaimed_bonus(reclaimed),
            plain: PLAIN_POINTS * plain,
            crash: CRASH_POINTS * crash,
            counter: COUNTER_POINTS * counter,
        }
    }

    pub fn base(&self) -> u32 {
        self.chain
            + self.color
            + self.count
            + self.reclaimed
            + self.plain
            + self.crash
            + self.counter
    }
}

/// `+0x112`: `n` ripened counter gems, paid at a rate that climbs with `n`.
pub fn reclaimed_bonus(n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let step = RECLAIMED_STEPS[(n.min(RECLAIMED_STEPS.len() as u32) - 1) as usize];
    step * n
}

/// Tenths of the base added to damage, one per thirty seconds from 75. The `<=` is the only
/// reading under which the rules doc's count and closed form agree.
pub fn time_tier(round_seconds: u32) -> u32 {
    (0..12)
        .map(|i| 90 + 30 * i)
        .filter(|threshold| *threshold <= round_seconds.min(MAX_ROUND_SECONDS) + 15)
        .count() as u32
}

/// What a break of this `base` sends, in counter gems. `halved` is the diamond flag `+0x23f`
/// (an open reading); `handicap` is how many steps the opponent's handicap is above this
/// player's, each worth a tenth.
pub fn damage(base: u32, round_seconds: u32, level: Level, halved: bool, handicap: u32) -> u32 {
    let scaled = base + time_tier(round_seconds) * (base / 10);
    let mut n = (scaled / 10) * (10 + level.factor()) as u32 / 100;
    if halved {
        n /= 2;
    }
    n += (n / 10) * handicap;
    n
}

/// What the smaller of two attacks cancels of the larger: half up to eleven, better above.
pub fn defence(n: u32) -> u32 {
    if n <= 11 {
        return n / 2;
    }
    let m = match n {
        12..=17 => 4,
        18..=23 => 5,
        24..=29 => 6,
        _ => 7,
    };
    (n / 8 + 1) * m
}

/// Offset, when an erase completes: a larger pool loses the smaller outright, a smaller one
/// reduces the larger by its [`defence`]. Returns both pools after, mine first.
pub fn exchange(mine: u32, theirs: u32) -> (u32, u32) {
    if theirs < mine {
        (mine - theirs, 0)
    } else {
        (0, theirs.saturating_sub(defence(mine)))
    }
}

/// the three plates on the HUD, read off the outgoing pool
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Warning {
    None,
    Caution,
    Warning,
    Danger,
}

impl Warning {
    pub fn of(pending: u32) -> Warning {
        match pending {
            0 => Warning::None,
            1..=10 => Warning::Caution,
            11..=30 => Warning::Warning,
            _ => Warning::Danger,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::board::tests::board;
    use crate::game::gems::marked;

    /// StrategyWiki's example: twelve gems and a crash gem score 1410
    #[test]
    fn twelve_loose_gems_and_a_crash_gem_score_fourteen_hundred_and_ten() {
        let erased = marked(&board(&["rrrrrg", "rrrrrg", "rrR..g"]));
        assert_eq!(erased.count(), 13);
        let scored = Accumulators::of(&erased, 1);
        assert_eq!(scored.plain, 1200);
        assert_eq!(scored.crash, 100);
        assert_eq!(scored.count, 110);
        assert_eq!(scored.chain, 0, "the first pass of a break is chain one");
        assert_eq!(scored.color, 0, "one colour is not a colour bonus");
        assert_eq!(scored.base(), 1410);
    }

    /// a four-gem crash sends four counter gems at Normal
    #[test]
    fn a_four_gem_crash_sends_four_counter_gems() {
        let erased = marked(&board(&["rr", "rR"]));
        let base = Accumulators::of(&erased, 1).base();
        assert_eq!(base, 420);
        assert_eq!(damage(base, 0, Level::Normal, false, 0), 4);
    }

    #[test]
    fn the_level_scales_every_attack() {
        assert_eq!(damage(420, 0, Level::Easy, false, 0), 3);
        assert_eq!(damage(420, 0, Level::Normal, false, 0), 4);
        assert_eq!(damage(420, 0, Level::Hard, false, 0), 5);
    }

    /// the time tier starts at 75 seconds and stops at twelve
    #[test]
    fn damage_climbs_with_the_round_clock() {
        assert_eq!(time_tier(0), 0);
        assert_eq!(time_tier(74), 0);
        assert_eq!(
            time_tier(75),
            1,
            "the first tier, and the closed form agrees"
        );
        assert_eq!(time_tier(105), 2);
        assert_eq!(
            time_tier(405),
            12,
            "+120%, and the closed form agrees here too"
        );
        assert_eq!(time_tier(MAX_ROUND_SECONDS), 12, "and it stops there");
        assert!(
            damage(1410, 400, Level::Normal, false, 0) > damage(1410, 0, Level::Normal, false, 0)
        );
    }

    #[test]
    fn the_chain_bonus_flattens_at_four_thousand() {
        assert_eq!(Accumulators::of(&Erased::default(), 1).chain, 0);
        assert_eq!(Accumulators::of(&Erased::default(), 4).chain, 1000);
        assert_eq!(Accumulators::of(&Erased::default(), 9).chain, 4000);
        assert_eq!(Accumulators::of(&Erased::default(), 40).chain, 4000);
    }

    /// defence matches the published table
    #[test]
    fn defence_is_better_than_two_for_one_above_eleven() {
        for (n, cancels) in [
            (8, 4),
            (12, 8),
            (16, 12),
            (18, 15),
            (24, 24),
            (30, 28),
            (40, 42),
            (60, 56),
        ] {
            assert_eq!(defence(n), cancels, "{n} gems");
        }
    }

    /// a defence of 24 cancels 24
    #[test]
    fn a_defended_break_of_twenty_four_cancels_a_full_attack() {
        assert_eq!(exchange(24, 24), (0, 0));
        assert_eq!(exchange(24, 30), (0, 6), "and part of a bigger one");
    }

    #[test]
    fn the_larger_attack_survives_undiminished_by_conversion() {
        assert_eq!(exchange(30, 10), (20, 0));
    }

    /// the reclaimed bonus climbs with `n` and caps at the last step
    #[test]
    fn ripened_counter_gems_pay_the_reclaimed_bonus() {
        assert_eq!(reclaimed_bonus(0), 0);
        assert_eq!(reclaimed_bonus(4), 400);
        assert_eq!(reclaimed_bonus(12), 12 * 150);
        assert_eq!(
            reclaimed_bonus(60),
            60 * 350,
            "and it is capped at the last step"
        );
    }

    #[test]
    fn the_warning_plates_match_the_pending_pool() {
        assert_eq!(Warning::of(0), Warning::None);
        assert_eq!(Warning::of(1), Warning::Caution);
        assert_eq!(Warning::of(11), Warning::Warning);
        assert_eq!(Warning::of(31), Warning::Danger);
    }
}
