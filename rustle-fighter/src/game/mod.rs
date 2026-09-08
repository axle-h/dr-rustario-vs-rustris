//! Super Rustle Fighter II Turbo's rules, headless.
//!
//! One playfield is a [`Playfield`]: a board, the gems on it, the garbage waiting to land on
//! it, and the running totals a round keeps. Everything a break does happens in
//! [`Playfield::resolve`], and it is a **loop rather than a search** - mark, erase, score,
//! settle, look again - which is exactly how `FUN_801346A0` steps it, one pass per forty frame
//! erase animation.
//!
//! The point of a break is not the points. Every one of the seven accumulators feeds a single
//! `base`, the base goes on the score *and* is the sole input to the damage formula, and the
//! damage is counter gems on the opponent's board. Points and attacks are one number in this
//! game, which is why [`score`] is where both live.

pub mod board;
pub mod cell;
pub mod counter;
pub mod gems;
pub mod pair;
pub mod play;
pub mod random;
pub mod rules;
pub mod score;
pub mod tables;

use crate::game::board::Board;
use crate::game::cell::PowerGemIds;
use crate::game::counter::{CounterTray, Fighter};
use crate::game::gems::{Erased, Pressure};
use crate::game::pair::Pair;
use crate::game::score::{Accumulators, Level};

pub use crate::game::cell::GAME_ID;
pub use crate::game::play::Game;

/// One pass of the chain loop: what it took, and what that was worth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BreakStep {
    /// how deep the chain was on this pass; the first is 1
    pub chain: u32,
    pub erased: Erased,
    pub scored: Accumulators,
    /// counter gems this pass generated, before any offset
    pub attack: u32,
    /// points this pass scored, including the Tech Bonus and any All Clear
    pub points: u32,
    /// the board came up empty after this pass
    pub all_clear: bool,
}

/// What a whole break did, from the piece landing to the board coming to rest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Resolution {
    pub steps: Vec<BreakStep>,
    /// points scored, including the Tech Bonus and any All Clear
    pub points: u32,
    /// counter gems generated, before the opponent's pool is offset against them
    pub attack: u32,
    /// the deepest the chain got, `+0x266`
    pub best_chain: u32,
}

impl Resolution {
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// every cell the whole break took
    pub fn erased(&self) -> u32 {
        self.steps.iter().map(|step| step.erased.count()).sum()
    }
}

/// One player's board and the round totals that go with it.
#[derive(Clone, Debug)]
pub struct Playfield {
    pub board: Board,
    /// which fighter is playing here - it decides the pattern this player's *garbage* arrives
    /// in on the other board, not this one
    pub fighter: Fighter,
    /// the garbage on its way here, `+0x22a` read from the far side
    pub tray: CounterTray,
    pub score: u32,
    /// what this player has thrown and the opponent has not yet taken, `+0x22a`
    pub outgoing: u32,
    /// `+0x29c`, the running All Clear award: the first is worth six, the second twelve
    all_clear_award: u32,
    /// `+0x265` and `+0x266`
    pub gems_destroyed: u32,
    pub best_chain: u32,
    ids: PowerGemIds,
}

impl Playfield {
    pub fn new(fighter: Fighter, ordering: usize) -> Playfield {
        Playfield {
            board: Board::new(),
            fighter,
            tray: CounterTray::new(ordering),
            score: 0,
            outgoing: 0,
            all_clear_award: 0,
            gems_destroyed: 0,
            best_chain: 0,
            ids: PowerGemIds::default(),
        }
    }

    /// how buried this player is, which is the only thing the fighter sprite reads
    pub fn pressure(&self) -> Pressure {
        Pressure::of(&self.board)
    }

    /// Lay a pair down and resolve everything that follows from it.
    pub fn lock(&mut self, pair: &Pair, round_seconds: u32, level: Level) -> Resolution {
        pair.lock(&mut self.board);
        self.board.tick_countdowns();
        self.resolve(round_seconds, level)
    }

    /// **One pass of the chain loop.** Settle, form what rectangles the board now allows,
    /// mark, erase, score - and say whether anything went.
    ///
    /// It is stepped rather than run to completion because the original steps it: `+0x21e` is
    /// a forty frame erase animation and the loop waits it out between passes, which is what
    /// makes a long chain something a player watches. [`Playfield::resolve`] is the same thing
    /// run out in one go, for anything that only wants the answer.
    ///
    /// `chain` is how many passes have already gone; the first pass is chain 1.
    pub fn resolve_step(
        &mut self,
        chain: u32,
        round_seconds: u32,
        level: Level,
    ) -> Option<BreakStep> {
        self.board.settle();
        gems::form_power_gems(&mut self.board, &mut self.ids);

        let erased = gems::marked(&self.board);
        if erased.is_empty() {
            return None;
        }
        let chain = chain + 1;
        gems::erase(&mut self.board, &erased);

        let scored = Accumulators::of(&erased, chain);
        let base = scored.base();
        let mut points = base;
        if erased.tech_bonus {
            // the Tech Bonus is not one of the seven accumulators, so it is worth points and
            // no damage at all
            points += score::TECH_BONUS;
        }
        let mut attack = score::damage(base, round_seconds, level, false, 0);

        // the census sees the board empty, and the All Clear award grows every time
        let all_clear = self.board.is_empty();
        if all_clear {
            self.all_clear_award += score::ALL_CLEAR_ATTACK;
            points += score::ALL_CLEAR_POINTS;
            attack += self.all_clear_award;
        }

        self.score += points;
        self.gems_destroyed += erased.count();
        self.best_chain = self.best_chain.max(chain);
        self.outgoing = (self.outgoing + attack).min(score::MAX_PENDING);
        Some(BreakStep {
            chain,
            erased,
            scored,
            attack,
            points,
            all_clear,
        })
    }

    /// The whole chain loop run out at once, for anything that wants the answer rather than
    /// the show - the tests, and the ai when it comes.
    pub fn resolve(&mut self, round_seconds: u32, level: Level) -> Resolution {
        let mut resolution = Resolution::default();
        while let Some(step) =
            self.resolve_step(resolution.steps.len() as u32, round_seconds, level)
        {
            resolution.points += step.points;
            resolution.attack += step.attack;
            resolution.best_chain = step.chain;
            resolution.steps.push(step);
        }
        resolution
    }

    /// **Offset.** Cancel what this player has just thrown against what is already falling
    /// towards them, and hand back what is left to send.
    ///
    /// Only one of the two pools survives: the larger attacker subtracts the smaller straight
    /// off, and the defender's tray is reduced by [`score::defence`] of what they threw - a
    /// conversion that is better than two for one above eleven gems and better than one for
    /// one above twenty-four. This is the local half of the same exchange [`offset`] performs
    /// between two boards.
    pub fn offset_outgoing(&mut self) -> u32 {
        let (sent, incoming) = score::exchange(self.outgoing, self.tray.pending());
        self.tray.cancel(self.tray.pending() - incoming);
        self.outgoing = 0;
        sent
    }

    /// Land whatever garbage is waiting, sent by `sender`, and settle it.
    ///
    /// Returns how many gems landed. The pattern is the *sender's*, which is why the HUD shows
    /// you the pattern you are about to be given.
    pub fn take_garbage(&mut self, sender: Fighter) -> u32 {
        let deliveries = self.tray.deliver(&self.board, sender);
        for delivery in &deliveries {
            counter::land(&mut self.board, *delivery);
        }
        self.board.settle();
        deliveries.len() as u32
    }
}

/// **Offset.** Resolve two players' pending attacks against each other, the moment an erase
/// completes.
///
/// Only one pool survives it: the larger attacker subtracts the smaller straight off, and the
/// defender's pool is reduced by [`score::defence`] of what they threw - a conversion that is
/// better than two for one above eleven gems, and better than one for one above twenty-four.
pub fn offset(a: &mut Playfield, b: &mut Playfield) {
    let (mine, theirs) = score::exchange(a.outgoing, b.outgoing);
    a.outgoing = mine;
    b.outgoing = theirs;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::board::tests::board;
    use crate::game::cell::{Gem, GemColor};
    use engine::game::geometry::Point;

    fn playfield(rows: &[&str]) -> Playfield {
        Playfield {
            board: board(rows),
            ..Playfield::new(Fighter::Ryu, 0)
        }
    }

    /// **A four-gem crash sends four.** The published figure, end to end.
    #[test]
    fn a_four_gem_crash_sends_four_counter_gems() {
        // the greens are there only so the board does not come up empty, which would add the
        // All Clear award on top of the break
        let mut player = playfield(&["rrg", "rRg"]);
        let resolution = player.resolve(0, Level::Normal);
        assert_eq!(resolution.steps.len(), 1);
        assert_eq!(resolution.steps[0].scored.base(), 420);
        assert_eq!(resolution.attack, 4);
        assert_eq!(player.outgoing, 4);
    }

    /// **Twelve loose gems score 1410.** Also the published figure, end to end.
    #[test]
    fn twelve_loose_gems_and_a_crash_gem_score_fourteen_hundred_and_ten() {
        let mut player = playfield(&["rrrrrg", "rrrrrg", "rrR..g"]);
        let resolution = player.resolve(0, Level::Normal);
        assert_eq!(resolution.erased(), 13, "twelve gems and the crash gem");
        assert_eq!(resolution.points, 1410);
    }

    /// **The first All Clear sends six**, the second twelve, the third eighteen - the award
    /// accumulates across the round, which the FAQs guessed was a flat twelve.
    #[test]
    fn the_all_clear_award_grows_through_a_round() {
        let mut player = playfield(&["rr", "rR"]);
        let first = player.resolve(0, Level::Normal);
        assert!(first.steps[0].all_clear);
        assert_eq!(first.attack, 4 + 6, "the break, and six for the All Clear");

        player.board = board(&["rr", "rR"]);
        let second = player.resolve(0, Level::Normal);
        assert_eq!(second.attack, 4 + 12);
        player.board = board(&["rr", "rR"]);
        assert_eq!(player.resolve(0, Level::Normal).attack, 4 + 18);
    }

    /// **A twenty-four gem defended break cancels a full attack.** Defence is one for one at
    /// twenty-four and better beyond it.
    #[test]
    fn a_defended_break_of_twenty_four_cancels_the_lot() {
        let mut attacker = Playfield::new(Fighter::Ryu, 0);
        let mut defender = Playfield::new(Fighter::Ken, 0);
        attacker.outgoing = 24;
        defender.outgoing = 24;
        offset(&mut attacker, &mut defender);
        assert_eq!((attacker.outgoing, defender.outgoing), (0, 0));
    }

    /// a break that leaves gems hanging drops them, and what lands may break in turn - which
    /// is the chain
    #[test]
    fn what_falls_into_place_breaks_in_turn() {
        // the green crash gem is stranded a column away from the greens until the reds under
        // it go, and then it falls in beside them
        let mut player = playfield(&["..G.", "rrrR", "gg.."]);
        let resolution = player.resolve(0, Level::Normal);
        assert_eq!(resolution.steps.len(), 2, "two passes of the loop");
        assert_eq!(resolution.steps[0].chain, 1);
        assert_eq!(resolution.steps[1].chain, 2);
        assert_eq!(resolution.best_chain, 2);
        assert_eq!(
            resolution.steps[1].scored.chain,
            score::CHAIN_BONUS[1],
            "and the second pass is paid the chain bonus the first was not"
        );
    }

    /// counter gems ripen as the receiver drops pieces, and a ripened one pays the
    /// reclaimed-garbage bonus when it finally goes
    #[test]
    fn ripened_garbage_is_worth_more_than_the_gems_beside_it() {
        let mut player = playfield(&["4r", "rR"]);
        for _ in 0..cell::COUNTER_COUNTDOWN {
            player.board.tick_countdowns();
        }
        let floor = board::ROWS as i32 - 1;
        assert_eq!(
            player.board.get(Point::new(0, floor - 1)),
            Some(Gem::Plain {
                color: GemColor::Red,
                power: None,
                reclaimed: true
            }),
            "it turned into an ordinary red"
        );
        let resolution = player.resolve(0, Level::Normal);
        assert!(resolution.steps[0].scored.reclaimed > 0);
    }

    /// a piece landing ticks every counter gem on the board down one
    #[test]
    fn locking_a_piece_ripens_the_garbage() {
        let mut player = playfield(&["1....."]);
        let pair = Pair::new(
            board::SPAWN,
            cell::GemPair::new(
                cell::Half::Plain(GemColor::Green),
                cell::Half::Plain(GemColor::Blue),
            ),
        );
        player.lock(&pair, 0, Level::Normal);
        let floor = board::ROWS as i32 - 1;
        assert_eq!(
            player.board.get(Point::new(0, floor)),
            Some(Gem::counter(GemColor::Blue, cell::COUNTER_COUNTDOWN - 1))
        );
    }

    /// garbage arrives in the *sender's* pattern, which is why the HUD shows it to you
    #[test]
    fn garbage_arrives_in_the_senders_pattern() {
        let mut player = Playfield::new(Fighter::ChunLi, 0);
        player.tray.receive(6, false);
        assert_eq!(player.take_garbage(Fighter::Ryu), 6);
        let floor = board::ROWS as i32 - 1;
        let row: Vec<Option<GemColor>> = (0..board::COLUMNS as i32)
            .map(|x| {
                player
                    .board
                    .get(Point::new(x, floor))
                    .and_then(|g| g.color())
            })
            .collect();
        assert_eq!(
            row,
            (0..6)
                .map(|c| Some(Fighter::Ryu.drop_color(0, c)))
                .collect::<Vec<_>>()
        );
    }
}
