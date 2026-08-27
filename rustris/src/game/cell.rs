//! How Rustris's minos and garbage are described to the engine.

use crate::game::block::BlockState;
use crate::game::geometry::{Point, Rotation};
use crate::game::tetromino::{Minos, TetrominoShape};
use engine::game::{Cell, CellId, GameId, PieceId, PlacedCell};

pub const GAME_ID: GameId = engine::game::ids::RUSTRIS;

/// The game-specific meaning of a [`CellId`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mino {
    Tetromino {
        shape: TetrominoShape,
        rotation: Rotation,
        /// which of the four minos of the shape this is
        mino: u8,
    },
    Garbage,
}

const KIND_GARBAGE: u16 = 0x100;

fn rotation_index(rotation: Rotation) -> u16 {
    match rotation {
        Rotation::North => 0,
        Rotation::East => 1,
        Rotation::South => 2,
        Rotation::West => 3,
    }
}

fn rotation_of(index: u16) -> Rotation {
    match index {
        0 => Rotation::North,
        1 => Rotation::East,
        2 => Rotation::South,
        _ => Rotation::West,
    }
}

impl Mino {
    pub fn id(shape: TetrominoShape, rotation: Rotation, mino: u32) -> CellId {
        Mino::Tetromino {
            shape,
            rotation,
            mino: mino as u8,
        }
        .into()
    }

    pub fn garbage() -> CellId {
        Mino::Garbage.into()
    }

    /// every mino id of every shape in every rotation
    pub fn all() -> Vec<(TetrominoShape, Rotation, u8)> {
        let mut all = vec![];
        for shape in TetrominoShape::ALL {
            for rotation in [
                Rotation::North,
                Rotation::East,
                Rotation::South,
                Rotation::West,
            ] {
                for mino in 0..4u8 {
                    all.push((shape, rotation, mino));
                }
            }
        }
        all
    }
}

impl From<Mino> for CellId {
    fn from(cell: Mino) -> Self {
        match cell {
            Mino::Tetromino {
                shape,
                rotation,
                mino,
            } => CellId((shape.id() as u16) | rotation_index(rotation) << 3 | (mino as u16) << 5),
            Mino::Garbage => CellId(KIND_GARBAGE),
        }
    }
}

impl From<CellId> for Mino {
    fn from(CellId(id): CellId) -> Self {
        if id & KIND_GARBAGE != 0 {
            return Mino::Garbage;
        }
        Mino::Tetromino {
            shape: TetrominoShape::ALL[(id & 0b111) as usize % TetrominoShape::ALL.len()],
            rotation: rotation_of((id >> 3) & 0b11),
            mino: ((id >> 5) & 0b11) as u8,
        }
    }
}

impl From<BlockState> for Cell {
    fn from(block: BlockState) -> Self {
        match block {
            BlockState::Empty => Cell::Empty,
            BlockState::Tetromino(s, r, m) => Cell::Active(Mino::id(s, r, m)),
            BlockState::Ghost(s, r, m) => Cell::Ghost(Mino::id(s, r, m)),
            BlockState::Stack(s, r, m) => Cell::Stack(Mino::id(s, r, m)),
            BlockState::Garbage => Cell::Garbage(Mino::garbage()),
        }
    }
}

impl From<TetrominoShape> for PieceId {
    fn from(shape: TetrominoShape) -> Self {
        PieceId(shape.id() as u16)
    }
}

impl From<PieceId> for TetrominoShape {
    fn from(PieceId(id): PieceId) -> Self {
        TetrominoShape::ALL[id as usize % TetrominoShape::ALL.len()]
    }
}

/// the cells of a piece's minos at their board positions
pub fn placed_minos(shape: TetrominoShape, rotation: Rotation, minos: Minos) -> Vec<PlacedCell> {
    minos
        .into_iter()
        .enumerate()
        .map(|(i, p)| (p, Mino::id(shape, rotation, i as u32)))
        .collect()
}

/// a row of garbage with a hole
pub fn garbage_row(y: u32, width: u32, hole: u32) -> Vec<PlacedCell> {
    (0..width)
        .filter(|x| *x != hole)
        .map(|x| (Point::from_u32(x, y), Mino::garbage()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_ids_round_trip() {
        for (shape, rotation, mino) in Mino::all() {
            let cell = Mino::Tetromino {
                shape,
                rotation,
                mino,
            };
            assert_eq!(Mino::from(CellId::from(cell)), cell);
        }
        assert_eq!(Mino::from(Mino::garbage()), Mino::Garbage);
    }

    #[test]
    fn piece_ids_round_trip() {
        for shape in TetrominoShape::ALL {
            assert_eq!(TetrominoShape::from(PieceId::from(shape)), shape);
        }
    }
}
