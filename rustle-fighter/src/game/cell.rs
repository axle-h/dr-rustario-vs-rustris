//! What sits in a cell: the fields of the game's 16 bit cell word and its parallel power gem
//! array (the rules doc's *Cell encoding*), as enums rather than bits.

use engine::game::{CellId, GameId, PieceId};

pub const GAME_ID: GameId = engine::game::ids::RUSTLE_FIGHTER;

/// The four gem colours, numbered as the game's own tables number them. The drop pattern
/// table in [`crate::game::counter`] is written in these indices, so keep the discriminants.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    strum::EnumIter,
    strum::FromRepr,
)]
#[repr(u8)]
pub enum GemColor {
    #[default]
    Blue = 1,
    Yellow = 2,
    Green = 3,
    Red = 4,
}

impl GemColor {
    pub const N: usize = 4;

    pub const ALL: [GemColor; GemColor::N] = [
        GemColor::Blue,
        GemColor::Yellow,
        GemColor::Green,
        GemColor::Red,
    ];

    /// the colour the game's own tables call `index`, which is 1-4
    pub fn from_game_index(index: u8) -> Option<GemColor> {
        GemColor::from_repr(index)
    }

    /// 0-3, for indexing an array of one entry per colour
    pub fn index(self) -> usize {
        self as usize - 1
    }
}

/// Which power gem a cell belongs to, and whether it is one of its corners: the game's
/// parallel array, corner code in the low byte and gem id in the high.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PowerCell {
    pub id: PowerGemId,
    pub corner: Option<Corner>,
}

/// The id all the cells of one power gem share. Never zero: zero is "no power gem".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PowerGemId(pub u8);

/// The allocator behind the ids, `+0x264`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PowerGemIds(u8);

impl PowerGemIds {
    /// wraps 255 back to 1
    pub fn allocate(&mut self) -> PowerGemId {
        self.0 = if self.0 == 255 { 1 } else { self.0 + 1 };
        PowerGemId(self.0)
    }
}

/// A power gem's corners, with the codes the game stamps into the corner array. Formation
/// accepts a rectangle whose corners sum to 15 ([`crate::game::gems`]); which code sits in
/// which corner is not pinned by the disassembly, so this follows the scan order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Corner {
    BottomLeft = 1,
    TopLeft = 2,
    TopRight = 4,
    BottomRight = 8,
}

impl Corner {
    pub const ALL: [Corner; 4] = [
        Corner::BottomLeft,
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomRight,
    ];

    pub fn code(self) -> u32 {
        self as u32
    }
}

/// Which orthogonal neighbours share a cell's power gem, one bit each. Drawing information
/// only: the rules work off [`PowerCell`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct PowerMask(u8);

impl PowerMask {
    pub const UP: PowerMask = PowerMask(1);
    pub const DOWN: PowerMask = PowerMask(2);
    pub const LEFT: PowerMask = PowerMask(4);
    pub const RIGHT: PowerMask = PowerMask(8);
    /// joined to nothing: every gem that is not in a power gem, and the pair in play
    pub const NONE: PowerMask = PowerMask(0);

    pub const COUNT: usize = 16;

    /// The nine masks a rectangle at least two cells a side can produce, which a theme must
    /// carry art for.
    pub const REACHABLE: [PowerMask; 9] = [
        PowerMask(0b1010),
        PowerMask(0b1110),
        PowerMask(0b0110),
        PowerMask(0b1011),
        PowerMask(0b1111),
        PowerMask(0b0111),
        PowerMask(0b1001),
        PowerMask(0b1101),
        PowerMask(0b0101),
    ];

    pub fn from_bits(bits: u8) -> PowerMask {
        PowerMask(bits & 0b1111)
    }

    pub fn bits(self) -> u8 {
        self.0
    }

    pub fn with(self, other: PowerMask) -> PowerMask {
        PowerMask(self.0 | other.0)
    }

    pub fn has(self, other: PowerMask) -> bool {
        self.0 & other.0 != 0
    }
}

/// a counter gem's countdown from an undefended attack (`0x507`)
pub const COUNTER_COUNTDOWN: u8 = 5;
/// ... and from a defended one (`0x307`)
pub const COUNTER_COUNTDOWN_DEFENDED: u8 = 3;

/// One gem. The four variants are the game's cell classes, minus the transient markers it uses
/// while a break is resolving.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Gem {
    /// An ordinary coloured gem: classes 1-4.
    Plain {
        color: GemColor,
        /// the game's `0x80` bit and its entry in the parallel array
        power: Option<PowerCell>,
        /// arrived as a counter gem that ran out, which the reclaimed-garbage accumulator
        /// `+0x112` pays for
        reclaimed: bool,
    },
    /// A crash gem: classes 9-12, the plain colour with `0x8` set.
    Crash(GemColor),
    /// Garbage: class 7, countdown in bits 8-11 and colour in bits 12-15.
    Counter { color: GemColor, countdown: u8 },
    /// The rainbow gem, class 5.
    Rainbow,
}

impl Gem {
    pub fn plain(color: GemColor) -> Gem {
        Gem::Plain {
            color,
            power: None,
            reclaimed: false,
        }
    }

    pub fn counter(color: GemColor, countdown: u8) -> Gem {
        Gem::Counter { color, countdown }
    }

    /// The colour a break spreads through this cell by, the game's `cell & 7`; a counter gem
    /// and the rainbow have none.
    pub fn break_color(&self) -> Option<GemColor> {
        match self {
            Gem::Plain { color, .. } | Gem::Crash(color) => Some(*color),
            Gem::Counter { .. } | Gem::Rainbow => None,
        }
    }

    /// the colour this gem is drawn in; a counter gem shows the colour it will become
    pub fn color(&self) -> Option<GemColor> {
        match self {
            Gem::Plain { color, .. } | Gem::Crash(color) | Gem::Counter { color, .. } => {
                Some(*color)
            }
            Gem::Rainbow => None,
        }
    }

    pub fn is_crash(&self) -> bool {
        matches!(self, Gem::Crash(_))
    }

    pub fn is_counter(&self) -> bool {
        matches!(self, Gem::Counter { .. })
    }

    pub fn is_reclaimed(&self) -> bool {
        matches!(
            self,
            Gem::Plain {
                reclaimed: true,
                ..
            }
        )
    }

    pub fn power(&self) -> Option<PowerCell> {
        match self {
            Gem::Plain { power, .. } => *power,
            _ => None,
        }
    }

    /// Only plain gems can join a power gem; the formation scan rejects every other class.
    pub fn can_join_power_gem(&self) -> bool {
        matches!(self, Gem::Plain { .. })
    }

    pub fn with_power(self, power: Option<PowerCell>) -> Gem {
        match self {
            Gem::Plain {
                color, reclaimed, ..
            } => Gem::Plain {
                color,
                power,
                reclaimed,
            },
            other => other,
        }
    }

    /// One tick of the countdown, per piece the receiver drops; at zero a counter gem becomes a
    /// reclaimed plain gem.
    pub fn tick_countdown(self) -> Gem {
        match self {
            Gem::Counter { color, countdown } if countdown <= 1 => Gem::Plain {
                color,
                power: None,
                reclaimed: true,
            },
            Gem::Counter { color, countdown } => Gem::Counter {
                color,
                countdown: countdown - 1,
            },
            other => other,
        }
    }
}

/// Half of the pair in play, as the queue shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Half {
    Plain(GemColor),
    Crash(GemColor),
    Rainbow,
}

impl Half {
    pub fn gem(self) -> Gem {
        match self {
            Half::Plain(color) => Gem::plain(color),
            Half::Crash(color) => Gem::Crash(color),
            Half::Rainbow => Gem::Rainbow,
        }
    }

    pub fn demoted(self) -> Half {
        match self {
            Half::Crash(color) => Half::Plain(color),
            other => other,
        }
    }
}

/// A whole pair, as the two NEXT boxes show it: pivot first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GemPair {
    pub pivot: Half,
    pub child: Half,
}

impl GemPair {
    pub fn new(pivot: Half, child: Half) -> GemPair {
        GemPair { pivot, child }
    }

    /// every pair that can be dealt, for a theme to key its previews on; only the child can be
    /// a rainbow
    pub fn all() -> Vec<GemPair> {
        let halves = |rainbow: bool| {
            GemColor::ALL
                .into_iter()
                .flat_map(|color| [Half::Plain(color), Half::Crash(color)])
                .chain(rainbow.then_some(Half::Rainbow))
                .collect::<Vec<_>>()
        };
        halves(false)
            .into_iter()
            .flat_map(|pivot| {
                halves(true)
                    .into_iter()
                    .map(move |child| GemPair::new(pivot, child))
            })
            .collect()
    }
}

impl From<GemPair> for PieceId {
    fn from(pair: GemPair) -> Self {
        let half = |half: Half| match half {
            Half::Plain(color) => color as u16,
            Half::Crash(color) => 8 + color as u16,
            Half::Rainbow => 5,
        };
        PieceId(half(pair.pivot) | half(pair.child) << 5)
    }
}

/// a [`CellId`]'s two kind bits
const KIND_PLAIN: u16 = 0;
const KIND_CRASH: u16 = 1;
const KIND_COUNTER: u16 = 2;
const KIND_RAINBOW: u16 = 3;

/// the countdown a counter gem is drawn with, capped at what the sheet has digits for
pub const MAX_DRAWN_COUNTDOWN: u8 = 9;

// cell id: kind in bits 0-1, colour in 2-3, then a plain gem's mask or a counter's digit
impl Gem {
    /// This gem as the engine's sheet keys it, joined to `mask`, which only a plain gem reads.
    pub fn id(self, mask: PowerMask) -> CellId {
        let (kind, color, extra) = match self {
            Gem::Plain { color, .. } => (KIND_PLAIN, color as u16, mask.bits() as u16),
            Gem::Crash(color) => (KIND_CRASH, color as u16, 0),
            Gem::Counter { color, countdown } => (
                KIND_COUNTER,
                color as u16,
                countdown.min(MAX_DRAWN_COUNTDOWN) as u16,
            ),
            Gem::Rainbow => (KIND_RAINBOW, 0, 0),
        };
        CellId(kind | (color.saturating_sub(1) & 0b11) << 2 | extra << 4)
    }

    /// the loose sprite, as a falling pair and a preview draw
    pub fn loose_id(self) -> CellId {
        self.id(PowerMask::NONE)
    }
}

/// A cell id read back, for a theme keying its sheet. The inverse of [`Gem::id`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GemSprite {
    Plain { color: GemColor, mask: PowerMask },
    Crash(GemColor),
    Counter { color: GemColor, countdown: u8 },
    Rainbow,
}

impl GemSprite {
    /// Every sprite a theme's sheet must hold.
    pub fn all() -> Vec<GemSprite> {
        let mut all = vec![GemSprite::Rainbow];
        for color in GemColor::ALL {
            all.push(GemSprite::Plain {
                color,
                mask: PowerMask::NONE,
            });
            all.push(GemSprite::Crash(color));
            for mask in PowerMask::REACHABLE {
                all.push(GemSprite::Plain { color, mask });
            }
            for countdown in 0..=MAX_DRAWN_COUNTDOWN {
                all.push(GemSprite::Counter { color, countdown });
            }
        }
        all
    }

    pub fn id(self) -> CellId {
        match self {
            GemSprite::Plain { color, mask } => Gem::plain(color).id(mask),
            GemSprite::Crash(color) => Gem::Crash(color).loose_id(),
            GemSprite::Counter { color, countdown } => Gem::counter(color, countdown).loose_id(),
            GemSprite::Rainbow => Gem::Rainbow.loose_id(),
        }
    }
}

impl From<CellId> for GemSprite {
    fn from(CellId(id): CellId) -> Self {
        let color = GemColor::ALL[((id >> 2) & 0b11) as usize];
        let extra = ((id >> 4) & 0b1111) as u8;
        match id & 0b11 {
            KIND_PLAIN => GemSprite::Plain {
                color,
                mask: PowerMask::from_bits(extra),
            },
            KIND_CRASH => GemSprite::Crash(color),
            KIND_COUNTER => GemSprite::Counter {
                color,
                countdown: extra,
            },
            _ => GemSprite::Rainbow,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the colours are numbered 1-4 as the drop pattern table is written
    #[test]
    fn the_colours_are_numbered_the_way_the_games_own_tables_number_them() {
        assert_eq!(GemColor::Blue as u8, 1);
        assert_eq!(GemColor::Yellow as u8, 2);
        assert_eq!(GemColor::Green as u8, 3);
        assert_eq!(GemColor::Red as u8, 4);
        for color in GemColor::ALL {
            assert_eq!(GemColor::from_game_index(color as u8), Some(color));
        }
        assert_eq!(GemColor::from_game_index(0), None, "0 is the empty cell");
        assert_eq!(GemColor::from_game_index(7), None, "7 is a counter gem");
    }

    /// a crash gem breaks by the same colour as a plain gem
    #[test]
    fn a_crash_gem_breaks_by_the_same_colour_a_plain_gem_does() {
        assert_eq!(
            Gem::Crash(GemColor::Red).break_color(),
            Gem::plain(GemColor::Red).break_color()
        );
    }

    /// counter gems and the rainbow have no break colour
    #[test]
    fn neither_a_counter_gem_nor_the_rainbow_has_a_break_colour() {
        assert_eq!(Gem::counter(GemColor::Red, 5).break_color(), None);
        assert_eq!(Gem::Rainbow.break_color(), None);
        assert_eq!(
            Gem::counter(GemColor::Red, 5).color(),
            Some(GemColor::Red),
            "though it does show the colour it will become"
        );
    }

    #[test]
    fn a_counter_gem_ripens_into_a_gem_that_remembers_it_was_garbage() {
        let mut gem = Gem::counter(GemColor::Green, COUNTER_COUNTDOWN);
        for expected in (1..COUNTER_COUNTDOWN).rev() {
            gem = gem.tick_countdown();
            assert_eq!(gem, Gem::counter(GemColor::Green, expected));
        }
        gem = gem.tick_countdown();
        assert_eq!(
            gem,
            Gem::Plain {
                color: GemColor::Green,
                power: None,
                reclaimed: true
            }
        );
        assert!(gem.is_reclaimed(), "and the score pass pays extra for it");
        assert_eq!(gem.tick_countdown(), gem, "an ordinary gem does not tick");
    }

    /// the allocator wraps 255 to 1, since 0 means no power gem
    #[test]
    fn power_gem_ids_wrap_past_zero() {
        let mut ids = PowerGemIds::default();
        assert_eq!(ids.allocate(), PowerGemId(1));
        let mut ids = PowerGemIds(254);
        assert_eq!(ids.allocate(), PowerGemId(255));
        assert_eq!(ids.allocate(), PowerGemId(1));
    }

    /// nine of the sixteen masks can occur, including a 2x2's four
    #[test]
    fn only_nine_masks_can_ever_be_drawn() {
        let reachable: std::collections::HashSet<u8> =
            PowerMask::REACHABLE.iter().map(|m| m.bits()).collect();
        assert_eq!(reachable.len(), 9);
        for mask in PowerMask::REACHABLE {
            assert!(mask.bits().count_ones() >= 2, "{mask:?}");
        }
        for corner in [
            PowerMask::DOWN.with(PowerMask::RIGHT),
            PowerMask::DOWN.with(PowerMask::LEFT),
            PowerMask::UP.with(PowerMask::RIGHT),
            PowerMask::UP.with(PowerMask::LEFT),
        ] {
            assert!(reachable.contains(&corner.bits()), "{corner:?}");
        }
    }

    /// the four corner codes sum to fifteen
    #[test]
    fn the_four_corner_codes_sum_to_fifteen() {
        assert_eq!(Corner::ALL.iter().map(|c| c.code()).sum::<u32>(), 15);
    }

    /// every sprite round-trips through a distinct cell id
    #[test]
    fn every_sprite_round_trips_through_its_cell_id() {
        let all = GemSprite::all();
        let ids: std::collections::HashSet<CellId> = all.iter().map(|s| s.id()).collect();
        assert_eq!(ids.len(), all.len(), "and no two share one");
        for sprite in all {
            assert_eq!(GemSprite::from(sprite.id()), sprite);
        }
    }

    /// the power mask changes a plain gem's cell id and nothing else's
    #[test]
    fn the_power_mask_reaches_the_cell_id_and_nothing_else() {
        let gem = Gem::plain(GemColor::Red);
        assert_ne!(gem.id(PowerMask::UP), gem.loose_id());
        assert_eq!(gem.id(PowerMask::NONE), gem.loose_id());
        assert_eq!(
            Gem::Crash(GemColor::Red).id(PowerMask::UP),
            Gem::Crash(GemColor::Red).loose_id(),
            "and nothing but a plain gem reads it at all"
        );
    }

    #[test]
    fn every_pair_that_can_be_dealt_has_its_own_piece_id() {
        let all = GemPair::all();
        let ids: std::collections::HashSet<PieceId> =
            all.iter().map(|pair| PieceId::from(*pair)).collect();
        assert_eq!(ids.len(), all.len());
        assert_eq!(
            all.len(),
            8 * 9,
            "eight pivots, and a child may be the rainbow"
        );
    }
}
