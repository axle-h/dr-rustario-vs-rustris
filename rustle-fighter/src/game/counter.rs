//! Counter gems: the roster whose patterns they arrive in, the tray they wait in, and how
//! they land.
//!
//! Garbage in this game is not anonymous. Every gem that lands carries a colour, that colour
//! is read off the **attacker's own** table, and the HUD shows the opponent the pattern they
//! are about to receive - so a Ryu player is being handed six straight columns and knows it.
//! The table is `0x8016E684` in the PlayStation executable, transcribed here rather than
//! paraphrased; the rules doc's *Counter gem delivery* is the reading.

use crate::game::board::{Board, COLUMNS, ROWS};
use crate::game::cell::{Gem, GemColor, COUNTER_COUNTDOWN, COUNTER_COUNTDOWN_DEFENDED};
use engine::game::geometry::Point;

pub use crate::game::tables::{
    COLUMN_ORDERINGS, DROP_PATTERNS, ORDERINGS, PATTERN_COLUMNS, PATTERN_ROWS,
};

/// The playable roster: the seven we have arcade sprite sheets for.
///
/// The four that are cut - Donovan, Devilot, Akuma and Dan - keep their rows in
/// [`DROP_PATTERNS`], because the table is data read out of the game and cutting it down would
/// be editing the evidence. The index each name maps to was resolved without an emulator, by
/// reading the in-game "COUNTER GEM" panel off the arcade sheets and matching the 6x4 grid
/// against this table; see the rules doc's *Character index -> name*.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Default, strum::EnumIter, strum::IntoStaticStr,
)]
pub enum Fighter {
    Morrigan,
    ChunLi,
    #[default]
    Ryu,
    Ken,
    HsienKo,
    Felicia,
    Sakura,
}

impl Fighter {
    pub const ALL: [Fighter; 7] = [
        Fighter::Morrigan,
        Fighter::ChunLi,
        Fighter::Ryu,
        Fighter::Ken,
        Fighter::HsienKo,
        Fighter::Felicia,
        Fighter::Sakura,
    ];

    /// this fighter's index into the game's own tables, `+0x68`
    pub fn index(self) -> usize {
        match self {
            Fighter::Morrigan => 0,
            Fighter::ChunLi => 1,
            Fighter::Ryu => 2,
            Fighter::Ken => 3,
            Fighter::HsienKo => 4,
            Fighter::Felicia => 6,
            Fighter::Sakura => 7,
        }
    }

    /// the colour this fighter's garbage puts in `column` on the `row`th row it sends
    pub fn drop_color(self, row: usize, column: usize) -> GemColor {
        let index = DROP_PATTERNS[self.index()][row.min(PATTERN_ROWS - 1)][column];
        GemColor::from_game_index(index).expect("the pattern table holds colours 1-4")
    }

    pub fn name(self) -> &'static str {
        match self {
            Fighter::ChunLi => "Chun-Li",
            Fighter::HsienKo => "Hsien-Ko",
            other => other.into(),
        }
    }
}

/// A cell of garbage on its way down, with the countdown it will land carrying.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Delivery {
    pub column: i32,
    pub color: GemColor,
    pub countdown: u8,
}

/// Where a player's incoming garbage waits, and the cursor into the sender's pattern.
///
/// `pending` is the game's `+0x22a` read from the *other* side - it is generated as the
/// attacker's outgoing pool and lands here. `row` and `position` are `+0x1d2` and `+0x28c`:
/// the pattern row being filled and how far across it we are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CounterTray {
    pending: u32,
    /// whether what is pending arrived from an attack that was defended, which lands at a
    /// countdown of three rather than five
    defended: bool,
    row: usize,
    position: usize,
    ordering: usize,
}

impl CounterTray {
    /// `ordering` is `rng & 7`, picked once when the round starts: which of the eight
    /// permutations of the columns single gems are delivered in
    pub fn new(ordering: usize) -> CounterTray {
        CounterTray {
            ordering: ordering % ORDERINGS,
            ..CounterTray::default()
        }
    }

    pub fn pending(&self) -> u32 {
        self.pending
    }

    pub fn is_empty(&self) -> bool {
        self.pending == 0
    }

    /// take an attack, capped where the game caps it
    pub fn receive(&mut self, gems: u32, defended: bool) {
        self.pending = (self.pending + gems).min(crate::game::score::MAX_PENDING);
        self.defended = defended;
    }

    /// cancel `gems` of what is pending, which is what offset does before it ever lands
    pub fn cancel(&mut self, gems: u32) {
        self.pending = self.pending.saturating_sub(gems);
    }

    fn countdown(&self) -> u8 {
        if self.defended {
            COUNTER_COUNTDOWN_DEFENDED
        } else {
            COUNTER_COUNTDOWN
        }
    }

    /// **What lands this time**, from `sender`'s pattern.
    ///
    /// Six or more pending puts a **full row of six** down at once and takes six off the pool.
    /// Fewer puts them down **one per column**, in the round's own column ordering - in which
    /// the Drop Alley is last in all eight, which is the code's version of the guides' rule
    /// that the alley only fills once every other column has taken a gem.
    ///
    /// A column with no room is skipped and the gem is **discarded**, not held over.
    pub fn deliver(&mut self, board: &Board, sender: Fighter) -> Vec<Delivery> {
        if self.pending == 0 {
            return vec![];
        }
        let countdown = self.countdown();
        let mut landed = vec![];
        if self.pending >= COLUMNS {
            self.pending -= COLUMNS;
            for column in 0..COLUMNS as i32 {
                if has_room(board, column) {
                    landed.push(Delivery {
                        column,
                        color: sender.drop_color(self.row, column as usize),
                        countdown,
                    });
                }
            }
            self.advance_row();
        } else {
            let ordering = COLUMN_ORDERINGS[self.ordering];
            for _ in 0..self.pending {
                let column = ordering[self.position] as i32;
                if has_room(board, column) {
                    landed.push(Delivery {
                        column,
                        color: sender.drop_color(self.row, column as usize),
                        countdown,
                    });
                }
                self.position += 1;
                if self.position == PATTERN_COLUMNS {
                    self.advance_row();
                }
            }
            self.pending = 0;
        }
        landed
    }

    fn advance_row(&mut self) {
        self.position = 0;
        self.row = (self.row + 1) % PATTERN_ROWS;
    }
}

/// is there anywhere in this column for a gem to land?
fn has_room(board: &Board, column: i32) -> bool {
    (0..ROWS as i32).any(|y| board.is_free(Point::new(column, y)))
}

/// put a delivery on the board, in the top free cell of its column - it settles from there
pub fn land(board: &mut Board, delivery: Delivery) {
    let top = (0..ROWS as i32).find(|y| board.is_free(Point::new(delivery.column, *y)));
    if let Some(y) = top {
        board.set(
            Point::new(delivery.column, y),
            Some(Gem::counter(delivery.color, delivery.countdown)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::board::tests::board;
    use crate::game::cell::GemColor::*;

    /// **Ryu is six straight columns**, the pattern the guides call the easiest to counter -
    /// and the row that decoded the colour numbering, read off his own in-game panel.
    #[test]
    fn ryus_pattern_is_six_straight_columns() {
        for row in 0..PATTERN_ROWS {
            let colors: Vec<GemColor> = (0..PATTERN_COLUMNS)
                .map(|column| Fighter::Ryu.drop_color(row, column))
                .collect();
            assert_eq!(colors, vec![Red, Green, Blue, Yellow, Red, Green]);
        }
    }

    /// Chun-Li is six groups of 2x2, which is what her published description says
    #[test]
    fn chun_lis_pattern_is_two_by_two_blocks() {
        for row in [0, 1] {
            let colors: Vec<GemColor> = (0..PATTERN_COLUMNS)
                .map(|column| Fighter::ChunLi.drop_color(row, column))
                .collect();
            assert_eq!(colors, vec![Red, Red, Green, Green, Blue, Blue]);
        }
    }

    /// Ken's rows alternate colours, and each is solid
    #[test]
    fn kens_rows_are_each_one_colour() {
        for row in 0..PATTERN_ROWS {
            let first = Fighter::Ken.drop_color(row, 0);
            assert!((0..PATTERN_COLUMNS).all(|c| Fighter::Ken.drop_color(row, c) == first));
        }
        assert_ne!(Fighter::Ken.drop_color(0, 0), Fighter::Ken.drop_color(1, 0));
    }

    /// Felicia's index came from the FAQ's `gbbrry` string rather than from a panel - hers is
    /// the one sheet that carries none
    #[test]
    fn felicias_second_row_is_the_string_that_identified_her() {
        let colors: Vec<GemColor> = (0..PATTERN_COLUMNS)
            .map(|column| Fighter::Felicia.drop_color(2, column))
            .collect();
        assert_eq!(colors, vec![Green, Blue, Blue, Red, Red, Yellow]);
    }

    /// **The Drop Alley is last in all eight orderings.** That is the whole of the guides'
    /// rule that it only fills once every other column has taken a gem.
    #[test]
    fn the_drop_alley_is_last_in_every_column_ordering() {
        for ordering in COLUMN_ORDERINGS {
            assert_eq!(
                ordering[PATTERN_COLUMNS - 1] as i32,
                crate::game::board::DROP_ALLEY
            );
            let mut sorted = ordering;
            sorted.sort_unstable();
            assert_eq!(sorted, [0, 1, 2, 3, 4, 5], "and each is a permutation");
        }
    }

    /// six or more pending lands a full row at once
    #[test]
    fn six_pending_gems_land_as_a_row() {
        let mut tray = CounterTray::new(0);
        tray.receive(7, false);
        let landed = tray.deliver(&Board::new(), Fighter::Ryu);
        assert_eq!(landed.len(), 6);
        assert_eq!(
            tray.pending(),
            1,
            "and the seventh waits for the next piece"
        );
        assert_eq!(landed[0].countdown, COUNTER_COUNTDOWN);
    }

    /// fewer than six land one per column, in the round's ordering, alley last
    #[test]
    fn a_partial_row_lands_one_per_column_with_the_alley_last() {
        let mut tray = CounterTray::new(0);
        tray.receive(5, false);
        let landed = tray.deliver(&Board::new(), Fighter::Ryu);
        assert_eq!(
            landed.iter().map(|d| d.column).collect::<Vec<_>>(),
            vec![0, 1, 2, 5, 4]
        );
        assert!(tray.is_empty());
    }

    /// an attack that was defended arrives on a shorter fuse
    #[test]
    fn a_defended_attack_lands_at_a_countdown_of_three() {
        let mut tray = CounterTray::new(0);
        tray.receive(6, true);
        let landed = tray.deliver(&Board::new(), Fighter::Ryu);
        assert!(landed
            .iter()
            .all(|d| d.countdown == COUNTER_COUNTDOWN_DEFENDED));
    }

    /// a full column has nowhere to put a gem, and the gem is discarded rather than held
    #[test]
    fn a_gem_with_nowhere_to_go_is_discarded() {
        let mut full = Board::new();
        for y in 0..ROWS as i32 {
            full.set(Point::new(0, y), Some(Gem::plain(Red)));
        }
        let mut tray = CounterTray::new(0);
        tray.receive(6, false);
        let landed = tray.deliver(&full, Fighter::Ryu);
        assert_eq!(landed.len(), 5);
        assert!(tray.is_empty(), "and the pool still paid for it");
    }

    /// the pattern row advances as the garbage stacks up, so a second row is the sender's
    /// second row
    #[test]
    fn the_pattern_row_advances_with_each_row_delivered() {
        let mut tray = CounterTray::new(0);
        tray.receive(12, false);
        let first = tray.deliver(&Board::new(), Fighter::Ken);
        let second = tray.deliver(&Board::new(), Fighter::Ken);
        assert_ne!(first[0].color, second[0].color, "Ken's rows alternate");
    }

    #[test]
    fn a_delivery_lands_in_the_top_free_cell_of_its_column() {
        let mut b = board(&["rr"]);
        land(
            &mut b,
            Delivery {
                column: 0,
                color: Blue,
                countdown: 5,
            },
        );
        let top = Board::points()
            .find(|p| b.get(*p).is_some_and(|gem| gem.is_counter()))
            .expect("it landed");
        assert_eq!(top, Point::new(0, 0));
    }

    /// the roster is the seven we have sheets for, and their indices are the ones the panels
    /// pinned
    #[test]
    fn the_roster_indexes_into_the_games_own_table() {
        assert_eq!(
            Fighter::ALL.map(|f| f.index()),
            [0, 1, 2, 3, 4, 6, 7],
            "5 is Donovan and 8-10 are Devilot, Akuma and Dan, all cut"
        );
        assert_eq!(Fighter::HsienKo.name(), "Hsien-Ko");
    }
}
