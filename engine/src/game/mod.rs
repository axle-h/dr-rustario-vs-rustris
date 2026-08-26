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

/// An attack sent from one player to another. `strength` is how big the attack is to a player
/// of the same game, in that game's own units - rows of garbage in Rustris, garbage blocks in
/// Dr. Rustario - and `foreign` how big it is to a player of the other game, in theirs.
/// `detail` is private to the sending game and only meaningful to a receiver of the same
/// `origin`.
///
/// The two numbers differ because the two games' units are not the same thing and neither are
/// the clears that earn them: only the sending game knows how much work the clear took, so
/// only it can say what that is worth to somebody playing the other one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attack {
    pub origin: GameId,
    pub strength: u32,
    pub foreign: u32,
    pub detail: u64,
}

impl Attack {
    /// an attack worth the same to any receiver, whatever they are playing
    pub fn new(origin: GameId, strength: u32) -> Self {
        Self {
            origin,
            strength,
            foreign: strength,
            detail: 0,
        }
    }

    /// what this attack is worth to a player of another game, in that game's own units
    pub fn with_foreign(self, foreign: u32) -> Self {
        Self { foreign, ..self }
    }

    pub fn with_detail(self, detail: u64) -> Self {
        Self { detail, ..self }
    }

    /// how big the attack is to a player of `receiver`
    pub fn strength_for(&self, receiver: GameId) -> u32 {
        if receiver == self.origin {
            self.strength
        } else {
            self.foreign
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

    fn row(&self, y: u32) -> Vec<Cell> {
        (0..self.board_width())
            .map(|x| self.cell(Point::from_u32(x, y)))
            .collect()
    }
}
