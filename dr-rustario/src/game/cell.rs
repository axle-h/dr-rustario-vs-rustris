//! How Dr. Rustario's blocks, vitamins and viruses are described to the engine.

use crate::game::block::Block;
use crate::game::event::ColoredBlock;
use crate::game::geometry::Rotation;
use crate::game::pill::{Garbage, PillShape, VirusColor, Vitamin, VitaminOrdinal, Vitamins};
use engine::game::{Cell, CellId, GameId, PieceId, PlacedCell};

pub const GAME_ID: GameId = GameId(1);

/// The game-specific meaning of a [`CellId`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DrCell {
    Vitamin {
        color: VirusColor,
        rotation: Rotation,
        ordinal: VitaminOrdinal,
    },
    /// an orphaned vitamin or opponent garbage
    Garbage(VirusColor),
    Virus(VirusColor),
}

const KIND_VITAMIN: u16 = 0;
const KIND_GARBAGE: u16 = 1;
const KIND_VIRUS: u16 = 2;

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

impl DrCell {
    pub fn color(&self) -> VirusColor {
        match self {
            DrCell::Vitamin { color, .. } | DrCell::Garbage(color) | DrCell::Virus(color) => *color,
        }
    }

    pub fn is_virus(&self) -> bool {
        matches!(self, DrCell::Virus(_))
    }
}

impl From<DrCell> for CellId {
    fn from(cell: DrCell) -> Self {
        let (kind, color, rotation, ordinal) = match cell {
            DrCell::Vitamin {
                color,
                rotation,
                ordinal,
            } => (
                KIND_VITAMIN,
                color,
                rotation_index(rotation),
                ordinal as u16,
            ),
            DrCell::Garbage(color) => (KIND_GARBAGE, color, 0, 0),
            DrCell::Virus(color) => (KIND_VIRUS, color, 0, 0),
        };
        CellId(kind | (color as u16) << 2 | rotation << 4 | ordinal << 6)
    }
}

impl From<CellId> for DrCell {
    fn from(CellId(id): CellId) -> Self {
        let color = VirusColor::try_from(((id >> 2) & 0b11) as usize).unwrap_or_default();
        match id & 0b11 {
            KIND_VITAMIN => DrCell::Vitamin {
                color,
                rotation: rotation_of((id >> 4) & 0b11),
                ordinal: if (id >> 6) & 1 == 0 {
                    VitaminOrdinal::Left
                } else {
                    VitaminOrdinal::Right
                },
            },
            KIND_GARBAGE => DrCell::Garbage(color),
            _ => DrCell::Virus(color),
        }
    }
}

impl From<Block> for Cell {
    fn from(block: Block) -> Self {
        let vitamin = |color, rotation, ordinal| {
            CellId::from(DrCell::Vitamin {
                color,
                rotation,
                ordinal,
            })
        };
        match block {
            Block::Empty => Cell::Empty,
            Block::Vitamin(c, r, o) => Cell::Active(vitamin(c, r, o)),
            Block::Ghost(c, r, o) => Cell::Ghost(vitamin(c, r, o)),
            Block::Stack(c, r, o) => Cell::Stack(vitamin(c, r, o)),
            Block::Garbage(c) => Cell::Garbage(DrCell::Garbage(c).into()),
            Block::Virus(c) => Cell::Stack(DrCell::Virus(c).into()),
        }
    }
}

impl From<PillShape> for PieceId {
    fn from(shape: PillShape) -> Self {
        PieceId(
            PillShape::ALL
                .iter()
                .position(|s| *s == shape)
                .expect("every pill shape is in PillShape::ALL") as u16,
        )
    }
}

impl From<PieceId> for PillShape {
    fn from(PieceId(id): PieceId) -> Self {
        PillShape::ALL[id as usize % PillShape::ALL.len()]
    }
}

impl From<Vitamin> for PlacedCell {
    fn from(vitamin: Vitamin) -> Self {
        (
            vitamin.position(),
            DrCell::Vitamin {
                color: vitamin.color(),
                rotation: vitamin.rotation(),
                ordinal: vitamin.ordinal(),
            }
            .into(),
        )
    }
}

impl From<ColoredBlock> for PlacedCell {
    fn from(block: ColoredBlock) -> Self {
        let cell = if block.is_virus {
            DrCell::Virus(block.color)
        } else {
            DrCell::Garbage(block.color)
        };
        (block.position, cell.into())
    }
}

impl From<PlacedCell> for ColoredBlock {
    fn from((position, id): PlacedCell) -> Self {
        let cell = DrCell::from(id);
        ColoredBlock {
            position,
            color: cell.color(),
            is_virus: cell.is_virus(),
        }
    }
}

impl From<Garbage> for PlacedCell {
    fn from(garbage: Garbage) -> Self {
        (garbage.position, DrCell::Garbage(garbage.color).into())
    }
}

impl From<PlacedCell> for Garbage {
    fn from((position, id): PlacedCell) -> Self {
        Garbage::new(DrCell::from(id).color(), position)
    }
}

pub fn placed_vitamins(vitamins: Vitamins) -> Vec<PlacedCell> {
    vitamins.into_iter().map(PlacedCell::from).collect()
}

/// Pack up to 32 garbage colours into an [`engine::game::Attack::detail`].
pub fn encode_garbage(colors: &[VirusColor]) -> u64 {
    colors
        .iter()
        .take(32)
        .enumerate()
        .fold(0u64, |acc, (i, c)| acc | ((*c as u64 + 1) << (i * 2)))
}

pub fn decode_garbage(detail: u64) -> Vec<VirusColor> {
    (0..32)
        .map(|i| ((detail >> (i * 2)) & 0b11) as usize)
        .take_while(|v| *v != 0)
        .map(|v| VirusColor::try_from(v - 1).unwrap())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_ids_round_trip() {
        let mut cells = vec![];
        for color in [VirusColor::Yellow, VirusColor::Blue, VirusColor::Red] {
            cells.push(DrCell::Garbage(color));
            cells.push(DrCell::Virus(color));
            for rotation in [
                Rotation::North,
                Rotation::East,
                Rotation::South,
                Rotation::West,
            ] {
                for ordinal in [VitaminOrdinal::Left, VitaminOrdinal::Right] {
                    cells.push(DrCell::Vitamin {
                        color,
                        rotation,
                        ordinal,
                    });
                }
            }
        }
        for cell in cells {
            assert_eq!(DrCell::from(CellId::from(cell)), cell);
        }
    }

    #[test]
    fn piece_ids_round_trip() {
        for shape in PillShape::ALL {
            assert_eq!(PillShape::from(PieceId::from(shape)), shape);
        }
    }

    #[test]
    fn garbage_colors_round_trip() {
        let colors = vec![
            VirusColor::Red,
            VirusColor::Yellow,
            VirusColor::Yellow,
            VirusColor::Blue,
        ];
        assert_eq!(decode_garbage(encode_garbage(&colors)), colors);
        assert_eq!(decode_garbage(encode_garbage(&[])), vec![]);
    }
}
