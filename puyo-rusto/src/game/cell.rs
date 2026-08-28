//! How Puyo Rusto's puyos are described to the engine.
//!
//! A [`CellId`] here carries a colour *and* a four bit mask of which orthogonal neighbours
//! share that colour, because puyos of a colour that touch are drawn joined - the signature
//! look of the game, and what tells a player at a glance what is linked to what. That is a
//! sprite concern rather than an engine one: the engine only compares `CellId`s, and a game
//! may recompute them whenever it likes.

use engine::game::random::Seed;
use engine::game::{CellId, GameId, PieceId};
use rand::seq::SliceRandom;

pub const GAME_ID: GameId = engine::game::ids::PUYO;

/// Which set of puyos a cell is drawn from.
///
/// The particle theme is cut from a rip carrying eleven usable sets of the same puyos (see
/// `puyo-rusto/art/rip.py`), and [`PuyoSkin::deal`] hands a different one to each player at
/// the start of every match, so a session is not two boards of the same puyos and no two
/// matches look alike either. The theme keys every one of them, so which a board gets is a
/// decision the *game* makes when it is built rather than one the theme makes when it is, and
/// a theme with only one set of art may key all eleven at the same sprites.
///
/// It rides in the [`CellId`] for the same reason [`LinkMask`] does - the engine's sheet is
/// keyed by cell id and nothing else, and a game may put whatever drawing information it
/// likes in one. Nothing in the rules ever reads it: [`PuyoCell`] itself carries no skin, so
/// two puyos of a colour are equal whoever is looking at them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct PuyoSkin(u8);

impl PuyoSkin {
    /// how many sets of puyos there are, which is how many a theme's sheet has to key. The
    /// sheet `puyo-rusto/art/rip.py` writes carries exactly this many, and a test in
    /// [`crate::theme::modern`] holds it to that
    pub const COUNT: usize = 11;

    /// the set a board falls back on when nobody dealt it one, which is every test's and the
    /// title screen's
    pub const FIRST: PuyoSkin = PuyoSkin(0);

    pub fn all() -> impl Iterator<Item = PuyoSkin> {
        (0..PuyoSkin::COUNT as u8).map(PuyoSkin)
    }

    /// One set each for `players`, all different, drawn from the match's own seed.
    ///
    /// From the seed rather than the thread's randomness so that a playlist swapping one board
    /// over mid-match deals that player the puyos they already had - and so that replaying a
    /// seed looks like it did. It reads nothing any player's game is reading from: a
    /// `GameRandom`'s pool is fixed when it is built, so this cannot put two players out of
    /// step.
    pub fn deal(seed: Seed, players: usize) -> Vec<PuyoSkin> {
        let mut all: Vec<PuyoSkin> = PuyoSkin::all().collect();
        all.shuffle(&mut seed.rng());
        all.into_iter().cycle().take(players).collect()
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// The colours a puyo can be. A match deals three, four or five of them - see
/// [`crate::game::rules::Difficulty`] - but the set they are drawn from is always these five.
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
pub enum PuyoColor {
    #[default]
    Red = 0,
    Green = 1,
    Blue = 2,
    Yellow = 3,
    Purple = 4,
}

impl PuyoColor {
    /// how many colours exist at all; a match uses a subset of this many
    pub const N: usize = 5;

    pub const ALL: [PuyoColor; PuyoColor::N] = [
        PuyoColor::Red,
        PuyoColor::Green,
        PuyoColor::Blue,
        PuyoColor::Yellow,
        PuyoColor::Purple,
    ];

    pub fn from_index(index: usize) -> PuyoColor {
        PuyoColor::from_repr((index % PuyoColor::N) as u8).unwrap_or_default()
    }
}

/// Which orthogonal neighbours share a puyo's colour, as one bit each.
///
/// The mask is *drawing* information, recomputed by [`crate::game::board::Board`] after every
/// lock, pop and settle. It never decides anything: connectivity for popping is worked out
/// from the colours themselves, so a stale mask would be an ugly board rather than a wrong
/// one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct LinkMask(u8);

impl LinkMask {
    pub const UP: LinkMask = LinkMask(1);
    pub const DOWN: LinkMask = LinkMask(2);
    pub const LEFT: LinkMask = LinkMask(4);
    pub const RIGHT: LinkMask = LinkMask(8);

    /// joined to nothing: what a falling pair, a ghost and every nuisance puyo draw as
    pub const NONE: LinkMask = LinkMask(0);

    /// how many distinct masks there are, which is how wide a theme's sprite grid must be
    pub const COUNT: usize = 16;

    pub fn bits(self) -> u8 {
        self.0
    }

    pub fn from_bits(bits: u8) -> Self {
        Self(bits & 0b1111)
    }

    pub fn with(self, other: LinkMask) -> Self {
        Self(self.0 | other.0)
    }

    pub fn has(self, other: LinkMask) -> bool {
        self.0 & other.0 != 0
    }

    /// how many neighbours this puyo is joined to
    pub fn links(self) -> u32 {
        self.0.count_ones()
    }
}

/// The game-specific meaning of a [`CellId`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PuyoCell {
    /// a coloured puyo, drawn joined to whichever neighbours match
    Puyo { color: PuyoColor, links: LinkMask },
    /// nuisance sent by an opponent. It never joins to anything, including other nuisance,
    /// and it is cleared only by a coloured group popping beside it
    Nuisance,
    /// one icon of the tray above the board, standing for [`NuisanceIcon`] puyos still to
    /// land. Never on the board itself - see [`crate::game::nuisance`]
    Tray(NuisanceIcon),
}

/// One symbol of the nuisance tray, and how many puyos it stands for.
///
/// The sizes are the game's own (Puyo Nexus, *Nuisance queue*): a small puyo is one, a large
/// one is a full row of six, and a rock is thirty - five rows, which is also the most that can
/// ever fall at once. A theme with no art of its own for these may draw all three as its plain
/// nuisance sprite; the tray still reads correctly, just without the shorthand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum NuisanceIcon {
    Small = 0,
    Large = 1,
    Rock = 2,
}

impl NuisanceIcon {
    /// largest first, which is the order [`Self::decompose`] wants them in
    pub const ALL: [NuisanceIcon; 3] =
        [NuisanceIcon::Rock, NuisanceIcon::Large, NuisanceIcon::Small];

    pub fn puyos(self) -> u32 {
        match self {
            NuisanceIcon::Small => 1,
            NuisanceIcon::Large => 6,
            NuisanceIcon::Rock => 30,
        }
    }

    /// `count` puyos as the icons that stand for them, biggest first
    pub fn decompose(count: u32) -> Vec<NuisanceIcon> {
        let mut left = count;
        let mut icons = vec![];
        for icon in NuisanceIcon::ALL {
            while left >= icon.puyos() {
                left -= icon.puyos();
                icons.push(icon);
            }
        }
        icons
    }

    fn from_index(index: u16) -> NuisanceIcon {
        match index {
            0 => NuisanceIcon::Small,
            1 => NuisanceIcon::Large,
            _ => NuisanceIcon::Rock,
        }
    }
}

const KIND_PUYO: u16 = 0;
const KIND_NUISANCE: u16 = 1;
const KIND_TRAY: u16 = 2;

impl PuyoCell {
    pub fn puyo(color: PuyoColor, links: LinkMask) -> Self {
        PuyoCell::Puyo { color, links }
    }

    /// a puyo joined to nothing, which is how a falling pair and a ghost are always drawn
    pub fn loose(color: PuyoColor) -> Self {
        PuyoCell::puyo(color, LinkMask::NONE)
    }

    /// the colour of a coloured puyo; nuisance and tray icons have none
    pub fn color(&self) -> Option<PuyoColor> {
        match self {
            PuyoCell::Puyo { color, .. } => Some(*color),
            _ => None,
        }
    }

    pub fn links(&self) -> LinkMask {
        match self {
            PuyoCell::Puyo { links, .. } => *links,
            _ => LinkMask::NONE,
        }
    }

    /// the same puyo joined to a different set of neighbours
    pub fn with_links(&self, links: LinkMask) -> Self {
        match self {
            PuyoCell::Puyo { color, .. } => PuyoCell::puyo(*color, links),
            other => *other,
        }
    }
}

// kind in bits 0-1, colour in 2-4, link mask in 5-8, skin in 9-12
impl PuyoCell {
    /// this cell as the engine's sheet keys it, drawn from `skin`'s sprites
    ///
    /// There is no `From<PuyoCell>` because there is no answer without a skin: a cell id that
    /// forgot which board it was for would draw player two's puyos out of player one's set.
    pub fn id(self, skin: PuyoSkin) -> CellId {
        let (kind, value, links) = match self {
            PuyoCell::Puyo { color, links } => (KIND_PUYO, color as u16, links.bits() as u16),
            PuyoCell::Nuisance => (KIND_NUISANCE, 0, 0),
            PuyoCell::Tray(icon) => (KIND_TRAY, icon as u16, 0),
        };
        CellId(kind | value << 2 | links << 5 | (skin.0 as u16) << 9)
    }
}

/// the slot a cell id was drawn for, which only the sheet that keyed it cares about
impl From<CellId> for PuyoSkin {
    fn from(CellId(id): CellId) -> Self {
        PuyoSkin(((id >> 9) & 0b1111) as u8)
    }
}

impl From<CellId> for PuyoCell {
    fn from(CellId(id): CellId) -> Self {
        let value = (id >> 2) & 0b111;
        match id & 0b11 {
            KIND_PUYO => PuyoCell::Puyo {
                color: PuyoColor::from_index(value as usize),
                links: LinkMask::from_bits(((id >> 5) & 0b1111) as u8),
            },
            KIND_NUISANCE => PuyoCell::Nuisance,
            _ => PuyoCell::Tray(NuisanceIcon::from_index(value)),
        }
    }
}

/// A pair as the queue and the mascot's hand show it: the two colours, pivot first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PuyoPiece {
    pub pivot: PuyoColor,
    pub child: PuyoColor,
}

impl PuyoPiece {
    pub fn new(pivot: PuyoColor, child: PuyoColor) -> Self {
        Self { pivot, child }
    }

    /// every pair that can be dealt, for a theme to key its previews on
    pub fn all() -> Vec<PuyoPiece> {
        PuyoColor::ALL
            .iter()
            .flat_map(|pivot| {
                PuyoColor::ALL
                    .iter()
                    .map(move |child| PuyoPiece::new(*pivot, *child))
            })
            .collect()
    }
}

// pivot in bits 0-2, child in 3-5, skin in 6-9
impl PuyoPiece {
    /// this pair as the engine's queue keys it, drawn from `skin`'s sprites
    ///
    /// The previews are composed from the cells rather than drawn again, so a pair carries
    /// the slot for the same reason a cell does - otherwise a player would watch the other
    /// player's puyos queue up over their own board.
    pub fn id(self, skin: PuyoSkin) -> PieceId {
        PieceId(self.pivot as u16 | (self.child as u16) << 3 | (skin.0 as u16) << 6)
    }
}

impl From<PieceId> for PuyoSkin {
    fn from(PieceId(id): PieceId) -> Self {
        PuyoSkin(((id >> 6) & 0b1111) as u8)
    }
}

impl From<PieceId> for PuyoPiece {
    fn from(PieceId(id): PieceId) -> Self {
        PuyoPiece::new(
            PuyoColor::from_index((id & 0b111) as usize),
            PuyoColor::from_index(((id >> 3) & 0b111) as usize),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    /// every cell the game can draw has to survive the round trip, since the board stores
    /// `CellId`s and reads them back to recompute the masks
    #[test]
    fn every_cell_survives_the_round_trip() {
        let mut cells = vec![PuyoCell::Nuisance];
        cells.extend(NuisanceIcon::ALL.map(PuyoCell::Tray));
        for color in PuyoColor::iter() {
            for bits in 0..LinkMask::COUNT as u8 {
                cells.push(PuyoCell::puyo(color, LinkMask::from_bits(bits)));
            }
        }
        for cell in cells {
            for skin in PuyoSkin::all() {
                let id = cell.id(skin);
                assert_eq!(PuyoCell::from(id), cell, "{cell:?}");
                assert_eq!(PuyoSkin::from(id), skin, "{cell:?}");
            }
        }
    }

    /// ... and no two of them collide, or the board would draw one thing as another - and
    /// that has to hold across the skins as well, since both players' sets share one sheet
    #[test]
    fn no_two_cells_share_an_id() {
        let mut seen = std::collections::HashSet::new();
        for skin in PuyoSkin::all() {
            for color in PuyoColor::iter() {
                for bits in 0..LinkMask::COUNT as u8 {
                    assert!(seen.insert(PuyoCell::puyo(color, LinkMask::from_bits(bits)).id(skin)));
                }
            }
            assert!(seen.insert(PuyoCell::Nuisance.id(skin)));
            for icon in NuisanceIcon::ALL {
                assert!(seen.insert(PuyoCell::Tray(icon).id(skin)));
            }
        }
        // five colours by sixteen masks, plus nuisance and the three tray icons, per skin
        assert_eq!(
            seen.len(),
            PuyoSkin::COUNT * (PuyoColor::N * LinkMask::COUNT + 4)
        );
    }

    #[test]
    fn a_pair_survives_the_round_trip() {
        let mut seen = std::collections::HashSet::new();
        for skin in PuyoSkin::all() {
            for piece in PuyoPiece::all() {
                let id = piece.id(skin);
                assert_eq!(PuyoPiece::from(id), piece);
                assert_eq!(PuyoSkin::from(id), skin);
                assert!(seen.insert(id));
            }
        }
        assert_eq!(PuyoPiece::all().len(), PuyoColor::N * PuyoColor::N);
        assert_eq!(seen.len(), PuyoSkin::COUNT * PuyoColor::N * PuyoColor::N);
    }

    /// the whole point: two players are never dealt the same puyos, and two matches rarely
    /// the same pair
    #[test]
    fn a_match_deals_every_player_a_different_set() {
        let mut seen = std::collections::HashSet::new();
        for seed in 0..200u64 {
            let dealt = PuyoSkin::deal(Seed::from_u64(seed), 2);
            assert_eq!(dealt.len(), 2);
            assert_ne!(dealt[0], dealt[1], "seed {seed}");
            assert!(dealt.iter().all(|s| s.index() < PuyoSkin::COUNT));
            seen.insert(dealt);
        }
        // the sets two at a time is `COUNT * (COUNT - 1)` ordered pairs, and two hundred
        // seeds drawn with replacement land on about five sixths of however many that is -
        // so half of them is a bound a shuffle that is shuffling clears comfortably at any
        // skin count, and one that is not lands on a handful
        let pairs = PuyoSkin::COUNT * (PuyoSkin::COUNT - 1);
        assert!(seen.len() > pairs / 2, "{} distinct deals", seen.len());
    }

    /// ... and one seed always deals the same, which is what lets a playlist hand a player
    /// back the puyos they were already playing with
    #[test]
    fn one_seed_deals_the_same_set_every_time() {
        let seed = Seed::from_u64(7);
        assert_eq!(PuyoSkin::deal(seed, 2), PuyoSkin::deal(seed, 2));
        // ... and asking for one player is asking for the first of the same deal
        assert_eq!(PuyoSkin::deal(seed, 1)[0], PuyoSkin::deal(seed, 2)[0]);
    }

    /// more players than sets is not a thing this game can do, but wrapping beats panicking
    #[test]
    fn more_players_than_sets_wraps() {
        let dealt = PuyoSkin::deal(Seed::from_u64(3), PuyoSkin::COUNT + 2);
        assert_eq!(dealt.len(), PuyoSkin::COUNT + 2);
        assert_eq!(dealt[0], dealt[PuyoSkin::COUNT]);
    }

    #[test]
    fn nuisance_icons_stand_for_a_puyo_a_row_and_five_rows() {
        assert_eq!(NuisanceIcon::Small.puyos(), 1);
        assert_eq!(NuisanceIcon::Large.puyos(), 6);
        assert_eq!(NuisanceIcon::Rock.puyos(), 30);
    }

    #[test]
    fn a_tray_shows_the_biggest_icons_that_fit() {
        use NuisanceIcon::*;
        assert_eq!(NuisanceIcon::decompose(0), vec![]);
        assert_eq!(NuisanceIcon::decompose(1), vec![Small]);
        assert_eq!(NuisanceIcon::decompose(6), vec![Large]);
        assert_eq!(NuisanceIcon::decompose(7), vec![Large, Small]);
        assert_eq!(NuisanceIcon::decompose(30), vec![Rock]);
        assert_eq!(NuisanceIcon::decompose(37), vec![Rock, Large, Small]);
        // and whatever it shows adds back up to what is waiting
        for count in 0..200 {
            let total: u32 = NuisanceIcon::decompose(count)
                .iter()
                .map(|i| i.puyos())
                .sum();
            assert_eq!(total, count, "{count} puyos");
        }
    }

    #[test]
    fn a_falling_pair_and_nuisance_are_joined_to_nothing() {
        assert_eq!(PuyoCell::loose(PuyoColor::Red).links(), LinkMask::NONE);
        assert_eq!(PuyoCell::Nuisance.links(), LinkMask::NONE);
        // ... and nuisance stays unjoined however it is asked
        assert_eq!(
            PuyoCell::Nuisance.with_links(LinkMask::UP.with(LinkMask::DOWN)),
            PuyoCell::Nuisance
        );
    }

    #[test]
    fn a_mask_counts_its_links() {
        assert_eq!(LinkMask::NONE.links(), 0);
        assert_eq!(LinkMask::UP.with(LinkMask::RIGHT).links(), 2);
        assert!(LinkMask::UP.with(LinkMask::RIGHT).has(LinkMask::UP));
        assert!(!LinkMask::UP.with(LinkMask::RIGHT).has(LinkMask::DOWN));
    }
}
