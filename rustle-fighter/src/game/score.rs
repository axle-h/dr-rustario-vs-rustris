//! Points, damage and defence - which are one calculation, not three.
//!
//! Seven accumulators are summed into a **base**, the base is added to the score, and the base
//! is the *sole* input to the damage formula. That is why points and counter gems move
//! together in this game: there is no separate attack value anywhere. The rules doc's *Score*,
//! *Damage* and *Defence and offset* are where every number below is read from, and its
//! *Provenance* says which are cross-checked against a published figure.

use crate::game::gems::Erased;

/// `+0x10c`, indexed by `min(chain - 1, 10)`
pub const CHAIN_BONUS: [u32; 11] = [0, 200, 400, 1000, 1600, 2200, 2800, 3400, 4000, 4000, 4000];

/// `+0x10e`: this much per distinct colour erased beyond the first.
///
/// **[partial]** in the rules doc - the value is certain and the exact off-by-one is not, so a
/// single-colour break scoring nothing here is the reading, and it is the reading the
/// published 1410 point example agrees with.
pub const COLOR_BONUS: u32 = 200;

/// what one erased cell of each class is worth: `+0x114`, `+0x116` and `+0x118`
pub const PLAIN_POINTS: u32 = 100;
pub const CRASH_POINTS: u32 = 100;
/// a counter gem is worth **a tenth** of a normal gem, not the half the FAQs guess
pub const COUNTER_POINTS: u32 = 10;

/// `+0x29a`: a rainbow gem coming to rest on the floor.
///
/// It is not one of the seven accumulators, so it is worth points and **no damage at all** -
/// which is the thing about it the guides never say.
pub const TECH_BONUS: u32 = 10_000;

/// what one All Clear is worth in points ...
pub const ALL_CLEAR_POINTS: u32 = 600;
/// ... and in counter gems, cumulatively: `+0x29c` grows by this and the *whole of it* is
/// sent, so the first All Clear of a round sends 6, the second 12, the third 18
pub const ALL_CLEAR_ATTACK: u32 = 6;

/// `DAT_8016E584`, the per-cell rate the reclaimed-garbage bonus pays at, indexed by `n - 1`
pub const RECLAIMED_STEPS: [u32; 51] = [
    100, 100, 100, 100, 100, 100, 100, 100, 150, 150, 150, 150, 150, 150, 150, 200, 200, 200, 200,
    200, 200, 200, 200, 200, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 300, 300, 300,
    300, 300, 300, 300, 300, 300, 300, 300, 300, 300, 350, 350, 350,
];

/// `0x8016E634`: the difficulty setting's contribution, as `(L + 10) / 100` of a tenth of the
/// scaled base. Only the first three are reachable - the HUD has three plates.
pub const LEVELS: [i32; 8] = [-2, 0, 3, 5, 5, 5, 5, 5];

/// the round timer stops here, `+0x1f0`
pub const MAX_ROUND_SECONDS: u32 = 539;

/// the most damage that can be pending at once
pub const MAX_PENDING: u32 = 250;

/// The difficulty setting, which scales every attack in the game.
///
/// Worth x0.8 at its lowest and x1.3 at its highest against Normal, so **a measurement taken
/// without pinning it is not reproducible** - which is the trap the published damage figures
/// fall into. Normal is the setting that reproduces the FAQs' "four counter gems for a
/// four-gem crash".
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

/// The seven accumulators, each named for the field it is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Accumulators {
    /// `+0x10c`
    pub chain: u32,
    /// `+0x10e`
    pub color: u32,
    /// `+0x110`
    pub count: u32,
    /// `+0x112`
    pub reclaimed: u32,
    /// `+0x114`
    pub plain: u32,
    /// `+0x116`
    pub crash: u32,
    /// `+0x118`
    pub counter: u32,
}

impl Accumulators {
    /// What one erase step scored, at `chain` deep - the first pass of a break is chain 1.
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

    /// the sum: what goes on the score, and the sole input to [`damage`]
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

/// `+0x112`, the **reclaimed-garbage bonus**: `n` cells that arrived as counter gems and
/// ripened, paid at a rate that itself climbs with `n`.
///
/// This is the whole of the guides' "wait for the counter gems to become normal gems before
/// you break them", and it is the one accumulator that was misread before the disassembly was
/// opened - it looks like a power gem premium and is not one.
pub fn reclaimed_bonus(n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let step = RECLAIMED_STEPS[(n.min(RECLAIMED_STEPS.len() as u32) - 1) as usize];
    step * n
}

/// **Damage grows with elapsed round time, not with board height.**
///
/// The first tier lands at 75 seconds and another every thirty after it, each worth a further
/// tenth of the base, to twelve - +120% - at 405. The guides' "how late in the game" is this;
/// their "gems destroyed high on the screen are worth more" is not in this calculation at all.
///
/// **The comparison is `<=`, and that is a reading rather than a transcription.** The rules
/// doc gives this two ways - as a count of the thresholds under `round_seconds + 15`, and as a
/// closed form - and the two agree at every second only if the count is inclusive. Under `<`
/// they disagree at exactly the two boundaries the doc quotes in prose, 75 and 405.
pub fn time_tier(round_seconds: u32) -> u32 {
    (0..12)
        .map(|i| 90 + 30 * i)
        .filter(|threshold| *threshold <= round_seconds.min(MAX_ROUND_SECONDS) + 15)
        .count() as u32
}

/// What a break of this `base` sends, in counter gems.
///
/// `halved` is the diamond flag `+0x23f` - Sirlin records the diamond as dealing half of what
/// the same break would deal without it, and `+0x23f` is very likely it, which is [open].
/// `handicap` is how many steps the *opponent's* handicap is above this player's; each one is
/// worth another tenth.
pub fn damage(base: u32, round_seconds: u32, level: Level, halved: bool, handicap: u32) -> u32 {
    let scaled = base + time_tier(round_seconds) * (base / 10);
    let mut n = (scaled / 10) * (10 + level.factor()) as u32 / 100;
    if halved {
        n /= 2;
    }
    n += (n / 10) * handicap;
    n
}

/// What the **smaller** of two attacks cancels of the larger.
///
/// The single most misreported rule in the game. "Two counter gems cancel one" is only true up
/// to eleven; above that defence gets progressively better, reaching one for one at 24 and
/// exceeding it beyond - a forty gem defensive break cancels forty-two.
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

/// **Offset.** The two players' pending pools resolve against each other the moment an erase
/// completes, and only one of them survives it.
///
/// The larger attacker subtracts the smaller straight off - no conversion. The *defender's*
/// pool is reduced by [`defence`] of what they threw, which is the conversion. Returns the two
/// pools after the exchange, mine first.
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

    /// **The published example, exactly.** StrategyWiki destroys twelve linked gems with a
    /// crash gem and reports 1410 points: `12x100 + 1x100 + 10x(13-2)`.
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

    /// **The published damage figure, exactly.** Crash three normal gems and four counter gems
    /// go over: base 420, and a tenth of it at the Normal level.
    #[test]
    fn a_four_gem_crash_sends_four_counter_gems() {
        let erased = marked(&board(&["rr", "rR"]));
        let base = Accumulators::of(&erased, 1).base();
        assert_eq!(base, 420);
        assert_eq!(damage(base, 0, Level::Normal, false, 0), 4);
    }

    /// the difficulty setting moves it, which is why a measurement that does not pin the level
    /// is not reproducible
    #[test]
    fn the_level_scales_every_attack() {
        assert_eq!(damage(420, 0, Level::Easy, false, 0), 3);
        assert_eq!(damage(420, 0, Level::Normal, false, 0), 4);
        assert_eq!(damage(420, 0, Level::Hard, false, 0), 5);
    }

    /// time, not height: nothing happens for the first ninety seconds and then a tenth of the
    /// base joins every thirty
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

    /// the chain bonus is the big one, and it stops climbing at nine
    #[test]
    fn the_chain_bonus_flattens_at_four_thousand() {
        assert_eq!(Accumulators::of(&Erased::default(), 1).chain, 0);
        assert_eq!(Accumulators::of(&Erased::default(), 4).chain, 1000);
        assert_eq!(Accumulators::of(&Erased::default(), 9).chain, 4000);
        assert_eq!(Accumulators::of(&Erased::default(), 40).chain, 4000);
    }

    /// **The published defence table, exactly.** "Two counter gems cancel one" is only true up
    /// to eleven.
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

    /// a twenty-four gem defended break cancels the lot and leaves nothing behind
    #[test]
    fn a_defended_break_of_twenty_four_cancels_a_full_attack() {
        assert_eq!(exchange(24, 24), (0, 0));
        assert_eq!(exchange(24, 30), (0, 6), "and part of a bigger one");
    }

    /// the larger attacker subtracts the smaller straight off - no conversion that way
    #[test]
    fn the_larger_attack_survives_undiminished_by_conversion() {
        assert_eq!(exchange(30, 10), (20, 0));
    }

    /// a break of gems that arrived as garbage pays extra, at a rate that itself climbs
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
