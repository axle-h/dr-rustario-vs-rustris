//! The contract between the engine and a falling block game's rules.
//!
//! A game is headless: it knows nothing about rendering or audio. It describes its board as
//! [`Cell`]s keyed by game-private [`CellId`]s, its queue and hold as [`PieceId`]s, and reports
//! what happened as [`GameEvent`]s. The engine's session, themes, animations and particles are
//! written against this module only.

pub mod geometry;
pub mod hold;
pub mod random;
pub mod timing;

use geometry::Point;
use std::time::Duration;

/// Identifies which game produced something, so a receiver can tell whether game-private
/// detail (e.g. the colours of Dr. Mario garbage) is meaningful to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GameId(pub u16);

/// Every game of the compendium, by id.
///
/// These live here rather than each in its own crate for two reasons: the ids have to be
/// unique across the whole binary, and pricing an attack means naming the game it is crossing
/// to (see [`ForeignPrices`]) - which a game crate could not do otherwise, since the games are
/// siblings and none of them depends on another. Each crate re-exports its own as `GAME_ID`.
///
/// Ids are small and dense because [`ForeignPrices`] keys on them directly; number a new game
/// from the end and raise [`ForeignPrices::GAMES`] if it runs out.
pub mod ids {
    use super::GameId;

    pub const DR_RUSTARIO: GameId = GameId(1);
    pub const RUSTRIS: GameId = GameId(2);
    pub const PUYO: GameId = GameId(3);
}

/// A game-private key for how a cell should be drawn, e.g. "red virus" or "left half of a
/// blue vitamin rotated east" or "T mino". The engine only compares them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellId(pub u16);

/// A game-private key for a whole piece as shown in the queue and hold box, e.g. a pill
/// shape or a tetromino shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PieceId(pub u16);

/// One board position as the engine sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cell {
    Empty,
    /// part of the piece the player is controlling
    Active(CellId),
    /// where the active piece would land
    Ghost(CellId),
    /// a locked block of any kind
    Stack(CellId),
    /// a block sent by an opponent, or otherwise not placed by the player
    Garbage(CellId),
}

impl Cell {
    pub fn id(&self) -> Option<CellId> {
        match self {
            Cell::Empty => None,
            Cell::Active(id) | Cell::Ghost(id) | Cell::Stack(id) | Cell::Garbage(id) => Some(*id),
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Cell::Empty)
    }
}

/// A cell together with where it is.
pub type PlacedCell = (Point, CellId);

/// What an attack is worth abroad: one price per receiving [`GameId`], in that game's own
/// units.
///
/// A single foreign number would be a silent bug the moment a third game arrives, because
/// every other game would be sent the same one and it would still compile. One price per
/// receiver makes each pair of games a deliberate decision instead. That is O(n²) to author,
/// which is the honest cost of this project's own principle that only the sender knows what a
/// clear took: a neutral "work unit" currency would be O(n) but would throw that away.
///
/// A pair nobody has priced is worth **nothing**, so a forgotten one drops the attack (see
/// `Match::send_attack`) rather than landing the wrong units on somebody.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ForeignPrices([u32; ForeignPrices::GAMES]);

impl ForeignPrices {
    /// how many games the compendium can price between. Ids are small and dense (Dr. Rustario
    /// is 1, Rustris 2), so a flat array keyed by id is the whole data structure; raise this
    /// when the compendium outgrows it.
    pub const GAMES: usize = 8;

    fn slot(receiver: GameId) -> Option<usize> {
        let index = receiver.0 as usize;
        (index < Self::GAMES).then_some(index)
    }

    fn price(&self, receiver: GameId) -> u32 {
        Self::slot(receiver).map_or(0, |slot| self.0[slot])
    }

    fn set(&mut self, receiver: GameId, price: u32) {
        // an id past the end is an authoring mistake rather than something a match can reach:
        // it fails every test run, and in a release build the attack is simply worth nothing
        // to that receiver, which is the same safe default as a pair nobody priced at all
        debug_assert!(
            Self::slot(receiver).is_some(),
            "game id {} is past ForeignPrices::GAMES ({}); raise it",
            receiver.0,
            Self::GAMES
        );
        if let Some(slot) = Self::slot(receiver) {
            self.0[slot] = price;
        }
    }
}

/// An attack sent from one player to another. `strength` is how big the attack is to a player
/// of the same game, in that game's own units - rows of garbage in Rustris, garbage blocks in
/// Dr. Rustario - and `foreign` how big it is to a player of each *other* game, in theirs.
/// `detail` is private to the sending game and only meaningful to a receiver of the same
/// `origin`.
///
/// The numbers differ because the games' units are not the same thing and neither are the
/// clears that earn them: only the sending game knows how much work the clear took, so only it
/// can say what that is worth to somebody playing another one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attack {
    pub origin: GameId,
    pub strength: u32,
    pub foreign: ForeignPrices,
    pub detail: u64,
}

impl Attack {
    /// an attack worth `strength` at home and nothing at all abroad, until a price is put on
    /// each crossing it should make
    pub fn new(origin: GameId, strength: u32) -> Self {
        Self {
            origin,
            strength,
            foreign: ForeignPrices::default(),
            detail: 0,
        }
    }

    /// what this attack is worth to a player of `receiver`, in that game's own units
    pub fn with_foreign_for(mut self, receiver: GameId, price: u32) -> Self {
        self.foreign.set(receiver, price);
        self
    }

    pub fn with_detail(self, detail: u64) -> Self {
        Self { detail, ..self }
    }

    /// how big the attack is to a player of `receiver`
    pub fn strength_for(&self, receiver: GameId) -> u32 {
        if receiver == self.origin {
            self.strength
        } else {
            self.foreign.price(receiver)
        }
    }
}

/// Whether the current stage of a game is still being played.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageState {
    Playing,
    /// the stage goal was reached (a bottle cleared, ten lines made...)
    StageComplete,
    GameOver,
}

/// What happens at a stage boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageTransition {
    /// play stops on a "stage clear" card until the player dismisses it; the board resets
    Interstitial,
    /// play continues straight into the next stage
    Seamless,
}

/// A number a game wants shown on the HUD.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MetricKind {
    Score,
    Level,
    Lines,
    Viruses,
    /// the longest run of clears one placement set off. Named for the thing rather than for
    /// any one game's word for it, since this is a closed engine enum and more than one game
    /// wants the counter.
    Chain,
}

/// Something that happened inside a game during an update or in response to input. Events are
/// produced by the game that owns the board; the session knows which player that was.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameEvent {
    Move,
    Rotate,
    Hold,
    SoftDrop,
    /// the piece stepped down one row
    Fall,
    /// a new piece entered the board
    Spawn {
        piece: PieceId,
        cells: Vec<PlacedCell>,
        is_hold: bool,
    },
    /// the spawn animation (if any) finished and the piece is under player control
    Spawned,
    HardDrop {
        cells: Vec<PlacedCell>,
        dropped_rows: u32,
    },
    Lock {
        cells: Vec<PlacedCell>,
        /// locked by a hard or soft drop rather than by gravity
        dropped: bool,
    },
    /// cells were removed from the board. `count` is the game's own measure of how much was
    /// cleared (lines, patterns...) and `is_combo` whether it chained from a previous clear.
    /// `detail` is game-private, the way an [`Attack`]'s is: whatever grading the game's own
    /// renderer wants back out of it.
    Clear {
        cells: Vec<PlacedCell>,
        count: u32,
        is_combo: bool,
        detail: u64,
    },
    /// loose blocks fell after a clear
    Settle,
    /// Cells that just came to rest, at the points they came to rest on.
    ///
    /// Decoration: it holds nothing, makes no sound and costs a game that never sends it
    /// nothing at all. It is not [`GameEvent::Settle`], which fires once for a whole board
    /// and only when a settle *moved* something - a pair landing flat on the stack produces
    /// no settle, and a half resting on a ledge comes to rest a lock earlier than its
    /// partner. A theme that bounces a landing cell needs both of those, and needs to know
    /// *which* cells and *where*.
    Landed {
        cells: Vec<PlacedCell>,
    },
    AttackSent(Attack),
    AttackReceived {
        cells: Vec<PlacedCell>,
    },
    SpeedUp,
    StageComplete,
    GameOver,
    Victory,
    Paused,
    UnPaused,
    NextTheme,
}

/// The rules of a falling block game, simulated for one player.
pub trait Game {
    /// which game this is, for [`Attack::origin`]
    fn game_id(&self) -> GameId;

    fn update(&mut self, delta: Duration);
    fn left(&mut self);
    fn right(&mut self);
    fn rotate(&mut self, clockwise: bool);
    fn set_soft_drop(&mut self, soft_drop: bool);
    fn hard_drop(&mut self);
    fn hold(&mut self);

    /// take every event produced since the last drain, oldest first
    fn drain_events(&mut self) -> Vec<GameEvent>;

    fn board_width(&self) -> u32;
    /// every simulated row, including any hidden above the visible board
    fn board_height(&self) -> u32;
    /// rows shown to the player, counted from the bottom
    fn visible_height(&self) -> u32;
    fn cell(&self, point: Point) -> Cell;

    /// How far the piece in play is through the row it is falling into, 0.0 at the top of the
    /// cell and approaching 1.0 at the bottom, so a renderer can slide it between cells rather
    /// than stepping it whole ones.
    ///
    /// Defaults to 0.0, which is a piece drawn on the grid and is what Rustris and Dr. Rustario
    /// want: both step a piece a cell at a time and their soft drops are slow enough that the
    /// step reads as a fall. Puyo Rusto overrides it because its soft drop is two frames a row -
    /// the rate the original hardcodes - and at that rate a whole-cell step is a strobe rather
    /// than a fall, the original having slid the pair eight pixels a frame inside its cell.
    fn fall_progress(&self) -> f64 {
        0.0
    }

    /// upcoming pieces, soonest first
    fn queue(&self) -> Vec<PieceId>;
    fn held(&self) -> Option<PieceId>;

    fn metric(&self, kind: MetricKind) -> Option<u32>;
    fn score(&self) -> u32;
    fn set_score(&mut self, score: u32);
    /// a game-neutral difficulty index carried between stages of a playlist
    fn speed_index(&self) -> u32;
    fn set_speed_index(&mut self, index: u32);

    fn stage_state(&self) -> StageState;
    fn stage_transition(&self) -> StageTransition;
    /// how many stages this game has completed
    fn completed_stages(&self) -> u32;
    /// continue counting from a previous game's stages (a playlist)
    fn set_completed_stages(&mut self, stages: u32);
    /// start the next stage after `StageComplete`, keeping score and speed
    fn next_stage(&mut self) -> Result<(), String>;

    fn receive_attack(&mut self, attack: Attack);

    /// Attacks already sent that have not landed yet, soonest first, as the cells this game
    /// would draw them with - so a player can see what is hanging over them and decide
    /// whether to answer it or take it.
    ///
    /// Only a game that can hold an attack has any: a game that takes a hit the moment it
    /// arrives leaves this empty and its themes draw no strip at all. What each icon is worth
    /// is the game's business, so one may stand for a single block or for a whole row.
    fn pending_attacks(&self) -> Vec<CellId> {
        vec![]
    }

    fn row(&self, y: u32) -> Vec<Cell> {
        (0..self.board_width())
            .map(|x| self.cell(Point::from_u32(x, y)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the trap a third game sets: one foreign number would send a Rustris tetris to
    /// Dr. Rustario and to Puyo alike, in whichever units happened to be meant, and compile
    #[test]
    fn an_attack_is_priced_for_each_game_separately() {
        let third = GameId(3);
        let attack = Attack::new(ids::RUSTRIS, 4)
            .with_foreign_for(ids::DR_RUSTARIO, 2)
            .with_foreign_for(third, 7);
        assert_eq!(attack.strength_for(ids::RUSTRIS), 4, "at home");
        assert_eq!(attack.strength_for(ids::DR_RUSTARIO), 2);
        assert_eq!(attack.strength_for(third), 7);
    }

    /// a pair nobody has priced is worth nothing, so the attack is dropped rather than
    /// landing a number that means something else where it lands
    #[test]
    fn an_unpriced_game_is_never_hit() {
        let attack = Attack::new(ids::RUSTRIS, 4).with_foreign_for(ids::DR_RUSTARIO, 2);
        assert_eq!(attack.strength_for(GameId(3)), 0);
        assert_eq!(
            Attack::new(ids::RUSTRIS, 4).strength_for(ids::DR_RUSTARIO),
            0
        );
    }

    /// ... including one numbered past the table. Authoring a price for it trips the debug
    /// assertion, so the mistake cannot ship; reading one back is what a match would do, and
    /// that is worth nothing rather than worth whatever happens to be in slot zero
    #[test]
    fn a_game_past_the_table_is_worth_nothing_rather_than_something_wrong() {
        let far = GameId(ForeignPrices::GAMES as u16);
        let attack = Attack::new(ids::RUSTRIS, 4).with_foreign_for(ids::DR_RUSTARIO, 2);
        assert_eq!(attack.strength_for(far), 0);
    }

    #[test]
    fn every_game_has_its_own_id() {
        let all = [ids::DR_RUSTARIO, ids::RUSTRIS, ids::PUYO];
        for (i, id) in all.iter().enumerate() {
            assert!(!all[..i].contains(id), "{id:?} is used twice");
            assert!(
                (id.0 as usize) < ForeignPrices::GAMES,
                "{id:?} is past ForeignPrices::GAMES"
            );
        }
    }
}
