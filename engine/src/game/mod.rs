//! The contract between the engine and a headless falling block game: a board of [`Cell`]s
//! keyed by game-private [`CellId`]s, reporting what happened as [`GameEvent`]s.

pub mod geometry;
pub mod hold;
pub mod pair;
pub mod random;
pub mod timing;

use geometry::Point;
use std::time::Duration;

/// Which game produced something, so a receiver knows whether game-private detail means
/// anything to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GameId(pub u16);

/// Every game's id, kept together so they stay unique and so a game can price an attack
/// into a sibling it cannot depend on. They index [`ForeignPrices`] directly, so a new game
/// takes the next number and an unpriced crossing into it is worth nothing and drops silently.
pub mod ids {
    use super::GameId;

    pub const DR_RUSTARIO: GameId = GameId(1);
    pub const RUSTRIS: GameId = GameId(2);
    pub const PUYO: GameId = GameId(3);
    pub const RUSTLE_FIGHTER: GameId = GameId(4);
}

/// A game-private key for how a cell is drawn; the engine only compares them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellId(pub u16);

/// A game-private key for a whole piece as shown in the queue and hold box.
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
/// units. An unpriced pair is worth nothing, so the attack drops silently rather than landing
/// the wrong units.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ForeignPrices([u32; ForeignPrices::GAMES]);

impl ForeignPrices {
    /// one past the highest [`GameId`] that can be priced
    pub const GAMES: usize = 8;

    fn slot(receiver: GameId) -> Option<usize> {
        let index = receiver.0 as usize;
        (index < Self::GAMES).then_some(index)
    }

    fn price(&self, receiver: GameId) -> u32 {
        Self::slot(receiver).map_or(0, |slot| self.0[slot])
    }

    fn set(&mut self, receiver: GameId, price: u32) {
        // a release build leaves an id past the end unpriced, which is worth nothing
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

/// An attack: `strength` in the sending game's own units, `foreign` in each other game's.
/// `detail` is only meaningful to a receiver of the same `origin`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attack {
    pub origin: GameId,
    pub strength: u32,
    pub foreign: ForeignPrices,
    pub detail: u64,
}

impl Attack {
    /// an attack worth `strength` at home and nothing abroad until priced
    pub fn new(origin: GameId, strength: u32) -> Self {
        Self {
            origin,
            strength,
            foreign: ForeignPrices::default(),
            detail: 0,
        }
    }

    pub fn with_foreign_for(mut self, receiver: GameId, price: u32) -> Self {
        self.foreign.set(receiver, price);
        self
    }

    pub fn with_detail(self, detail: u64) -> Self {
        Self { detail, ..self }
    }

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
    /// the longest run of clears one placement set off
    Chain,
}

/// Something that happened inside a game. Prefer a new event to a new defaulted [`Game`]
/// method: an event needs no delegating arm in the launcher's `AnyGame`.
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
    /// cells were removed; `count` is in the game's own measure and `detail` is game-private
    Clear {
        cells: Vec<PlacedCell>,
        count: u32,
        is_combo: bool,
        detail: u64,
    },
    /// loose blocks fell after a clear
    Settle,
    /// Cells that just came to rest, where they rest; decoration only. Unlike
    /// [`GameEvent::Settle`] it fires per cell, including cells that landed without a settle.
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
///
/// The launcher's `AnyGame` must delegate every method, and a defaulted one it misses is
/// silently never called; prefer a new [`GameEvent`].
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

    /// How far the piece in play is through the row it is falling into, from 0.0 towards 1.0,
    /// so a renderer can slide it between cells; 0.0 draws it on the grid.
    fn fall_progress(&self) -> f64 {
        0.0
    }

    /// upcoming pieces, soonest first
    fn queue(&self) -> Vec<PieceId>;
    fn held(&self) -> Option<PieceId>;

    fn metric(&self, kind: MetricKind) -> Option<u32>;
    fn score(&self) -> u32;
    fn set_score(&mut self, score: u32);
    /// A game-neutral difficulty index carried between stages of a playlist. It may change how
    /// a game feels, never what it deals: players reach a change at different moments while
    /// dealt from one shared seed.
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

    /// Attacks received that have not landed yet, soonest first, as the cells to draw them
    /// with. Empty for a game that takes a hit the moment it arrives, which draws no strip.
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

    /// each receiving game reads its own price off one attack
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

    /// an unpriced receiver reads zero
    #[test]
    fn an_unpriced_game_is_never_hit() {
        let attack = Attack::new(ids::RUSTRIS, 4).with_foreign_for(ids::DR_RUSTARIO, 2);
        assert_eq!(attack.strength_for(GameId(3)), 0);
        assert_eq!(
            Attack::new(ids::RUSTRIS, 4).strength_for(ids::DR_RUSTARIO),
            0
        );
    }

    /// a receiver numbered past the table reads zero
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
