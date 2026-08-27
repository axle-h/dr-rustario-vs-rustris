//! The dials: how many colours a match deals, how fast puyos fall, and how long a stage is.

use std::time::Duration;

/// How hard the match is, in the game's own terms.
///
/// Puyo Nexus, [Tsu (rule)](https://puyonexus.com/wiki/Tsu_(rule)): the difficulty setting is
/// what decides "how many colors the player will receive and how many Garbage Puyos they will
/// start out with on their field". Five colours is a much harder game than three, because a
/// chain needs its colours to come back round.
///
/// The colour count is fixed for a whole match and is **never** driven by
/// [`crate::game::Game::speed_index`]. Stages advance per player while the pair pool is dealt
/// from one shared seed, so a colour count that changed part way through would deal the player
/// who got there first a colour the other is not drawing yet - and from then on the two would
/// be playing different games. `speed_index` may change how a game feels, never what it deals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Difficulty {
    VeryEasy,
    Easy,
    #[default]
    Normal,
    Hard,
    VeryHard,
}

impl Difficulty {
    pub const ALL: [Difficulty; 5] = [
        Difficulty::VeryEasy,
        Difficulty::Easy,
        Difficulty::Normal,
        Difficulty::Hard,
        Difficulty::VeryHard,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Difficulty::VeryEasy => "very easy",
            Difficulty::Easy => "easy",
            Difficulty::Normal => "normal",
            Difficulty::Hard => "hard",
            Difficulty::VeryHard => "very hard",
        }
    }

    pub fn from_name(name: &str) -> Option<Difficulty> {
        Difficulty::ALL.into_iter().find(|d| d.name() == name)
    }

    /// how many of the five colours this match deals
    pub fn colors(&self) -> usize {
        match self {
            Difficulty::VeryEasy | Difficulty::Easy => 3,
            Difficulty::Normal => 4,
            Difficulty::Hard | Difficulty::VeryHard => 5,
        }
    }

    /// rows of nuisance already on the board when the match starts
    pub fn starting_nuisance_rows(&self) -> u32 {
        match self {
            Difficulty::Easy | Difficulty::VeryHard => 2,
            _ => 0,
        }
    }

    /// the very hard setting also drops puyos a little faster from the off
    pub fn speed_bonus(&self) -> u32 {
        match self {
            Difficulty::VeryHard => 2,
            _ => 0,
        }
    }
}

/// How many puyos have to be cleared to finish a stage.
///
/// Puyo has no natural level, so a stage is a speed step: play carries on seamlessly and the
/// pairs simply start falling faster. Roughly seven or eight groups' worth, which is a
/// comparable stretch of play to Rustris's ten lines.
pub const PUYOS_PER_STAGE: u32 = 30;

/// the fastest a pair falls of its own accord
pub const MIN_FALL_DELAY: Duration = Duration::from_millis(90);

/// How long a pair takes to fall one row at each speed step.
///
/// Past the end of the table it stays at [`MIN_FALL_DELAY`]; a stage beyond that changes
/// nothing but the number on the HUD, which is the same shape Rustris's own curve has.
pub const FALL_DELAY_MS: [u64; 12] = [800, 700, 600, 520, 450, 380, 320, 260, 210, 170, 130, 100];

/// how much faster a pair falls while the player holds soft drop
pub const SOFT_DROP_FACTOR: u32 = 12;

/// how long a resting pair may still be nudged about before it locks
pub const LOCK_DELAY: Duration = Duration::from_millis(400);

/// the pause on a popped group before it disappears, so a chain can be watched
pub const POP_DELAY: Duration = Duration::from_millis(280);

/// the pause while loose puyos fall, after a lock or between chain steps
pub const SETTLE_DELAY: Duration = Duration::from_millis(120);

/// the pause while the queue empties onto the board
pub const NUISANCE_DELAY: Duration = Duration::from_millis(300);

/// the pause before a new pair appears
pub const SPAWN_DELAY: Duration = Duration::from_millis(120);

/// how long one row of gravity takes at this speed step
pub fn fall_delay(speed_index: u32) -> Duration {
    FALL_DELAY_MS
        .get(speed_index as usize)
        .map(|ms| Duration::from_millis(*ms))
        .unwrap_or(MIN_FALL_DELAY)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the sourced difficulty table: three colours at the bottom, five at the top, with two
    /// rows of nuisance on the two settings that start you buried
    #[test]
    fn difficulty_sets_the_colour_count() {
        assert_eq!(Difficulty::VeryEasy.colors(), 3);
        assert_eq!(Difficulty::Easy.colors(), 3);
        assert_eq!(Difficulty::Normal.colors(), 4);
        assert_eq!(Difficulty::Hard.colors(), 5);
        assert_eq!(Difficulty::VeryHard.colors(), 5);
        assert_eq!(Difficulty::default(), Difficulty::Normal);
    }

    #[test]
    fn two_of_the_settings_start_you_buried() {
        assert_eq!(Difficulty::Easy.starting_nuisance_rows(), 2);
        assert_eq!(Difficulty::VeryHard.starting_nuisance_rows(), 2);
        for difficulty in [Difficulty::VeryEasy, Difficulty::Normal, Difficulty::Hard] {
            assert_eq!(difficulty.starting_nuisance_rows(), 0);
        }
    }

    #[test]
    fn the_hardest_setting_also_drops_faster() {
        assert_eq!(Difficulty::VeryHard.speed_bonus(), 2);
        assert_eq!(Difficulty::Hard.speed_bonus(), 0);
    }

    #[test]
    fn every_difficulty_is_named_and_found_by_name() {
        for difficulty in Difficulty::ALL {
            assert_eq!(Difficulty::from_name(difficulty.name()), Some(difficulty));
        }
        assert_eq!(Difficulty::from_name("impossible"), None);
    }

    /// the colour count only ever goes up with difficulty, never down
    #[test]
    fn harder_never_means_fewer_colours() {
        for pair in Difficulty::ALL.windows(2) {
            assert!(pair[0].colors() <= pair[1].colors());
        }
    }

    #[test]
    fn pairs_fall_faster_at_every_step_and_then_level_off() {
        for step in 1..FALL_DELAY_MS.len() as u32 {
            assert!(
                fall_delay(step) < fall_delay(step - 1),
                "step {step} is not faster than the one before"
            );
        }
        assert_eq!(fall_delay(FALL_DELAY_MS.len() as u32), MIN_FALL_DELAY);
        assert_eq!(fall_delay(9999), MIN_FALL_DELAY);
    }
}
