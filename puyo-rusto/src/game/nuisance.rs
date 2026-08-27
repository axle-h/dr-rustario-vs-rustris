//! The nuisance queue: what is waiting to land on you, what your chain cancels, and how the
//! rest of it falls.
//!
//! This is the identity mechanic of the game. Under **classic Tsu offset** (Puyo Nexus,
//! [Offset rule](https://puyonexus.com/wiki/Offset_rule)) a chain first cancels whatever is
//! queued against you and only then sends anything on, and whatever still waits drops as soon
//! as your chain finishes - so you generally get exactly one chain to answer an attack. It is
//! what turns two people racing into two people fighting.

use crate::game::board::{Board, COLUMNS};
use crate::game::cell::{NuisanceIcon, PuyoCell};
use crate::game::random::Seed;
use crate::game::score::{NuisancePoints, ALL_CLEAR_NUISANCE};
use engine::game::CellId;
use rand::RngExt;
use rand_chacha::ChaChaRng;

/// a full row of the board
pub const ROW: u32 = COLUMNS;

/// The most nuisance that can fall at once: five rows.
///
/// Puyo Nexus, *Nuisance queue*: this is what the rock symbol stands for, and "the maximum
/// number of Garbage Puyos that can fall at once". Anything past it stays in the queue for the
/// next drop, which is what makes a big attack something you dig out of over several turns
/// rather than an instant burial.
pub const MAX_DROP: u32 = 30;

/// What a chain did to the queue.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outgoing {
    /// cancelled off what was waiting, and so never landed at all
    pub offset: u32,
    /// what was left over and goes to the opponent
    pub sent: u32,
}

impl Outgoing {
    pub fn total(&self) -> u32 {
        self.offset + self.sent
    }
}

/// One player's queue.
#[derive(Clone, Debug)]
pub struct Nuisance {
    pending: u32,
    points: NuisancePoints,
    /// an all clear has been earned and is owed against the next chain
    all_clear_owed: bool,
    /// Where the remainder of a drop scatters. Kept apart from anything that decides *what*
    /// is dealt: the pair pool is fixed when the match starts, so drawing from here can never
    /// put two players' colours out of step. It could not anyway - what lands on you depends
    /// on your opponent, and is yours alone.
    rng: ChaChaRng,
}

impl Nuisance {
    pub fn new(seed: Seed) -> Self {
        Self {
            pending: 0,
            points: NuisancePoints::default(),
            all_clear_owed: false,
            rng: seed.rng(),
        }
    }

    /// how much is waiting to fall
    pub fn pending(&self) -> u32 {
        self.pending
    }

    pub fn is_pending(&self) -> bool {
        self.pending > 0
    }

    /// an attack has landed in the tray. It waits there - visible, and answerable - rather
    /// than dropping straight in
    pub fn receive(&mut self, count: u32) {
        self.pending = self.pending.saturating_add(count);
    }

    /// an all clear was made: the next chain carries a whole rock extra
    pub fn earn_all_clear(&mut self) {
        self.all_clear_owed = true;
    }

    pub fn all_clear_owed(&self) -> bool {
        self.all_clear_owed
    }

    /// What a chain worth `score` does: cancel first, send the rest.
    ///
    /// The all clear bonus rides on the *next* chain to clear anything, so it is spent here
    /// rather than when the board was emptied - and it is spent whether or not the chain was
    /// otherwise worth a single puyo.
    pub fn resolve(&mut self, score: u32) -> Outgoing {
        if score == 0 {
            return Outgoing::default();
        }
        let mut nuisance = self.points.take(score);
        if self.all_clear_owed {
            nuisance += ALL_CLEAR_NUISANCE;
            self.all_clear_owed = false;
        }
        let offset = nuisance.min(self.pending);
        self.pending -= offset;
        Outgoing {
            offset,
            sent: nuisance - offset,
        }
    }

    /// how much of the queue falls now: everything waiting, up to the cap
    pub fn take_drop(&mut self) -> u32 {
        let dropping = self.pending.min(MAX_DROP);
        self.pending -= dropping;
        dropping
    }

    /// Which columns `count` puyos fall down, full rows first and the remainder scattered.
    ///
    /// Full rows before anything else is what makes a big attack land flat rather than in a
    /// tower; the remainder is spread over distinct columns so that the last few do not stack
    /// up in one place.
    pub fn drop_columns(&mut self, count: u32) -> Vec<i32> {
        let mut columns = vec![];
        for _ in 0..count / ROW {
            columns.extend(0..COLUMNS as i32);
        }
        let remainder = (count % ROW) as usize;
        if remainder > 0 {
            let mut choices: Vec<i32> = (0..COLUMNS as i32).collect();
            for i in (1..choices.len()).rev() {
                choices.swap(i, self.rng.random_range(0..=i));
            }
            choices.truncate(remainder);
            choices.sort();
            columns.extend(choices);
        }
        columns
    }

    /// Drop `count` puyos onto a board, reporting where each landed.
    ///
    /// A column with no room takes nothing and the puyo is simply lost, which is what the
    /// real game does with a bottle that is already full to the brim.
    pub fn drop_onto(&mut self, board: &mut Board, count: u32) -> Vec<engine::game::PlacedCell> {
        let mut landed = vec![];
        for column in self.drop_columns(count) {
            if let Some(point) = board.drop_into(column, PuyoCell::Nuisance) {
                landed.push((point, CellId::from(PuyoCell::Nuisance)));
            }
        }
        if !landed.is_empty() {
            board.recompute_links();
        }
        landed
    }

    /// The tray above the board: what is waiting, as the symbols that stand for it.
    ///
    /// This is what [`engine::game::Game::pending_attacks`] reports, and what a theme's
    /// pending strip draws.
    pub fn tray(&self) -> Vec<CellId> {
        NuisanceIcon::decompose(self.pending)
            .into_iter()
            .map(|icon| CellId::from(PuyoCell::Tray(icon)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::board::tests::board;
    use crate::game::board::ROWS;
    use crate::game::score::TARGET_POINTS;

    fn nuisance() -> Nuisance {
        Nuisance::new(Seed::from_u64(42))
    }

    #[test]
    fn an_attack_waits_in_the_tray_rather_than_landing() {
        let mut queue = nuisance();
        assert!(!queue.is_pending());
        queue.receive(12);
        assert_eq!(queue.pending(), 12);
    }

    /// the whole point: a chain cancels what is waiting before it sends anything on
    #[test]
    fn a_chain_cancels_what_is_waiting_before_it_sends() {
        let mut queue = nuisance();
        queue.receive(10);
        // a chain worth exactly four nuisance
        let out = queue.resolve(4 * TARGET_POINTS);
        assert_eq!(out.offset, 4, "all four cancelled");
        assert_eq!(out.sent, 0, "and none of it crossed");
        assert_eq!(queue.pending(), 6, "six still waiting");
    }

    #[test]
    fn a_chain_bigger_than_the_queue_sends_the_excess() {
        let mut queue = nuisance();
        queue.receive(3);
        let out = queue.resolve(10 * TARGET_POINTS);
        assert_eq!(out.offset, 3);
        assert_eq!(out.sent, 7);
        assert_eq!(out.total(), 10);
        assert!(!queue.is_pending(), "the tray is clear");
    }

    #[test]
    fn a_chain_with_nothing_waiting_sends_all_of_it() {
        let mut queue = nuisance();
        let out = queue.resolve(5 * TARGET_POINTS);
        assert_eq!(out.offset, 0);
        assert_eq!(out.sent, 5);
    }

    /// the remainder carries between chains, so small chains eventually add up to a puyo
    #[test]
    fn the_remainder_carries_between_chains() {
        let mut queue = nuisance();
        assert_eq!(queue.resolve(40).sent, 0, "40 points is not yet a puyo");
        assert_eq!(queue.resolve(40).sent, 1, "80 points across two chains is");
    }

    /// clearing nothing is not a chain at all: it neither spends the carry nor the all clear
    #[test]
    fn a_placement_that_clears_nothing_resolves_to_nothing() {
        let mut queue = nuisance();
        queue.earn_all_clear();
        queue.receive(5);
        let out = queue.resolve(0);
        assert_eq!(out, Outgoing::default());
        assert_eq!(queue.pending(), 5, "and nothing was cancelled");
        assert!(queue.all_clear_owed(), "the all clear is still owed");
    }

    /// Tsu's all clear: thirty extra on the *next* chain, not on the one that emptied the
    /// board
    #[test]
    fn an_all_clear_rides_on_the_next_chain() {
        let mut queue = nuisance();
        queue.earn_all_clear();
        let out = queue.resolve(2 * TARGET_POINTS);
        assert_eq!(out.sent, 32, "two of its own plus a whole rock");
        assert!(!queue.all_clear_owed(), "and it is spent");
        assert_eq!(queue.resolve(2 * TARGET_POINTS).sent, 2, "only once");
    }

    /// ... and it is spent even by a chain too small to send anything of its own
    #[test]
    fn an_all_clear_rides_on_a_chain_worth_nothing_by_itself() {
        let mut queue = nuisance();
        queue.earn_all_clear();
        let out = queue.resolve(40);
        assert_eq!(out.sent, ALL_CLEAR_NUISANCE);
        assert!(!queue.all_clear_owed());
    }

    /// an all clear can be spent on cancelling, not only on attacking
    #[test]
    fn an_all_clear_can_be_spent_answering_an_attack() {
        let mut queue = nuisance();
        queue.earn_all_clear();
        queue.receive(30);
        let out = queue.resolve(40);
        assert_eq!(out.offset, ALL_CLEAR_NUISANCE);
        assert_eq!(out.sent, 0);
        assert!(!queue.is_pending());
    }

    #[test]
    fn no_more_than_five_rows_fall_at_once() {
        let mut queue = nuisance();
        queue.receive(100);
        assert_eq!(queue.take_drop(), MAX_DROP);
        assert_eq!(queue.pending(), 70, "the rest waits for the next turn");
        assert_eq!(queue.take_drop(), MAX_DROP);
        assert_eq!(queue.take_drop(), MAX_DROP);
        assert_eq!(queue.take_drop(), 10, "and then what is left");
        assert_eq!(queue.take_drop(), 0);
    }

    #[test]
    fn full_rows_fall_before_the_remainder_is_scattered() {
        let mut queue = nuisance();
        // two whole rows and four over
        let columns = queue.drop_columns(2 * ROW + 4);
        assert_eq!(columns.len() as u32, 2 * ROW + 4);
        let mut counts = [0; COLUMNS as usize];
        for column in columns.iter() {
            counts[*column as usize] += 1;
        }
        // every column took its two rows, and four of them took one more
        assert_eq!(counts.iter().filter(|c| **c == 2).count(), 2);
        assert_eq!(counts.iter().filter(|c| **c == 3).count(), 4);
    }

    /// the remainder goes to distinct columns rather than piling into one
    #[test]
    fn a_remainder_smaller_than_a_row_never_doubles_up() {
        let mut queue = nuisance();
        for _ in 0..50 {
            for count in 1..ROW {
                let columns = queue.drop_columns(count);
                assert_eq!(columns.len() as u32, count);
                let mut distinct = columns.clone();
                distinct.dedup();
                assert_eq!(
                    distinct.len(),
                    columns.len(),
                    "{count} scattered onto a column twice"
                );
            }
        }
    }

    #[test]
    fn an_exact_number_of_rows_needs_no_scattering() {
        let mut queue = nuisance();
        let columns = queue.drop_columns(ROW);
        assert_eq!(columns, (0..COLUMNS as i32).collect::<Vec<i32>>());
    }

    #[test]
    fn nuisance_lands_on_top_of_the_stack() {
        let mut queue = nuisance();
        let mut field = board(&["rrrrrr"]);
        let landed = queue.drop_onto(&mut field, ROW);
        assert_eq!(landed.len() as u32, ROW);
        for (point, _) in landed {
            assert_eq!(point.y, ROWS as i32 - 2, "the row above the stack");
        }
    }

    /// a board with no room takes what it can and the rest is lost
    #[test]
    fn nuisance_with_nowhere_to_go_is_lost() {
        let mut queue = nuisance();
        let mut field = Board::new();
        for column in 0..COLUMNS as i32 {
            for _ in 0..ROWS {
                field.drop_into(column, PuyoCell::Nuisance);
            }
        }
        assert!(queue.drop_onto(&mut field, ROW).is_empty());
    }

    #[test]
    fn the_tray_shows_what_is_waiting() {
        let mut queue = nuisance();
        assert!(queue.tray().is_empty());
        queue.receive(37);
        let tray: Vec<PuyoCell> = queue.tray().into_iter().map(PuyoCell::from).collect();
        assert_eq!(
            tray,
            vec![
                PuyoCell::Tray(NuisanceIcon::Rock),
                PuyoCell::Tray(NuisanceIcon::Large),
                PuyoCell::Tray(NuisanceIcon::Small),
            ]
        );
    }

    /// a worked exchange, end to end: they hit you, you chain back, the rest still lands
    #[test]
    fn one_chain_to_answer_an_attack_and_the_rest_still_lands() {
        let mut queue = nuisance();
        queue.receive(12);
        // a chain worth five: five cancelled, seven still waiting
        let out = queue.resolve(5 * TARGET_POINTS);
        assert_eq!(out.offset, 5);
        assert_eq!(out.sent, 0);
        assert_eq!(queue.pending(), 7);
        // ... and as the chain finishes, what is left drops
        assert_eq!(queue.take_drop(), 7);
        assert!(!queue.is_pending());
    }
}
