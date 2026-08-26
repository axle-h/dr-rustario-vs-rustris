use super::geometry::{Point, Pose, Rotation};
#[allow(unused_imports)]
use bitflags::{bitflags, Flags};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum TetrominoShape {
    /// XXXX
    #[default]
    I,
    /// XX
    /// XX
    O,
    ///  X
    /// XXX
    T,
    ///  XX
    /// XX
    S,
    /// XX
    ///  XX
    Z,
    /// X
    /// XXX
    J,
    ///   X
    /// XXX
    L,
}

impl TetrominoShape {
    pub const ALL: [TetrominoShape; 7] = [
        TetrominoShape::I,
        TetrominoShape::O,
        TetrominoShape::T,
        TetrominoShape::S,
        TetrominoShape::Z,
        TetrominoShape::J,
        TetrominoShape::L,
    ];

    pub fn id(&self) -> u8 {
        match self {
            TetrominoShape::I => 0,
            TetrominoShape::O => 1,
            TetrominoShape::T => 2,
            TetrominoShape::S => 3,
            TetrominoShape::Z => 4,
            TetrominoShape::J => 5,
            TetrominoShape::L => 6,
        }
    }

    pub fn meta(&self) -> &TetrominoMeta {
        match self {
            TetrominoShape::I => &I,
            TetrominoShape::O => &O,
            TetrominoShape::T => &T,
            TetrominoShape::S => &S,
            TetrominoShape::Z => &Z,
            TetrominoShape::J => &J,
            TetrominoShape::L => &L,
        }
    }

    fn offsets(&self, rotation: &Rotation) -> TetrominoOffsets {
        match self {
            TetrominoShape::I => match rotation {
                Rotation::North => TETROMINO_OFFSETS_I_NORTH,
                Rotation::East => TETROMINO_OFFSETS_I_EAST,
                Rotation::South => TETROMINO_OFFSETS_I_SOUTH,
                Rotation::West => TETROMINO_OFFSETS_I_WEST,
            },
            TetrominoShape::O => match rotation {
                Rotation::North => TETROMINO_OFFSETS_O_NORTH,
                Rotation::East => TETROMINO_OFFSETS_O_EAST,
                Rotation::South => TETROMINO_OFFSETS_O_SOUTH,
                Rotation::West => TETROMINO_OFFSETS_O_WEST,
            },
            _ => match rotation {
                Rotation::North => TETROMINO_OFFSETS_NORTH,
                Rotation::East => TETROMINO_OFFSETS_EAST,
                Rotation::South => TETROMINO_OFFSETS_SOUTH,
                Rotation::West => TETROMINO_OFFSETS_WEST,
            },
        }
    }
}

#[derive(Copy, Clone)]
struct Offset(i32, i32);

type TetrominoOffsets = [Offset; 5];

/// https://tetris.wiki/Super_Rotation_System
const TETROMINO_OFFSETS_NORTH: TetrominoOffsets = [
    Offset(0, 0),
    Offset(0, 0),
    Offset(0, 0),
    Offset(0, 0),
    Offset(0, 0),
];
const TETROMINO_OFFSETS_EAST: TetrominoOffsets = [
    Offset(0, 0),
    Offset(1, 0),
    Offset(1, -1),
    Offset(0, 2),
    Offset(1, 2),
];
const TETROMINO_OFFSETS_SOUTH: TetrominoOffsets = [
    Offset(0, 0),
    Offset(0, 0),
    Offset(0, 0),
    Offset(0, 0),
    Offset(0, 0),
];
const TETROMINO_OFFSETS_WEST: TetrominoOffsets = [
    Offset(0, 0),
    Offset(-1, 0),
    Offset(-1, -1),
    Offset(0, 2),
    Offset(-1, 2),
];

const TETROMINO_OFFSETS_I_NORTH: TetrominoOffsets = [
    Offset(0, 0),
    Offset(-1, 0),
    Offset(2, 0),
    Offset(-1, 0),
    Offset(2, 0),
];
const TETROMINO_OFFSETS_I_EAST: TetrominoOffsets = [
    Offset(-1, 0),
    Offset(0, 0),
    Offset(0, 0),
    Offset(0, 1),
    Offset(0, -2),
];
const TETROMINO_OFFSETS_I_SOUTH: TetrominoOffsets = [
    Offset(-1, 1),
    Offset(1, 1),
    Offset(-2, 1),
    Offset(1, 0),
    Offset(-2, 0),
];
const TETROMINO_OFFSETS_I_WEST: TetrominoOffsets = [
    Offset(0, 1),
    Offset(0, 1),
    Offset(0, 1),
    Offset(0, -1),
    Offset(0, 2),
];

const TETROMINO_OFFSETS_O_NORTH: TetrominoOffsets = [Offset(0, 0); 5];
const TETROMINO_OFFSETS_O_EAST: TetrominoOffsets = [Offset(0, -1); 5];
const TETROMINO_OFFSETS_O_SOUTH: TetrominoOffsets = [Offset(-1, -1); 5];
const TETROMINO_OFFSETS_O_WEST: TetrominoOffsets = [Offset(-1, 0); 5];

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Perimeter: u8 {
        const Top = 0b00000001;
        const Right = 0b00000010;
        const Bottom = 0b00000100;
        const Left = 0b00001000;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Corner: u8 {
        const None = 0;
        const TopLeft = 0b00000001;
        const TopRight = 0b00000010;
        const BottomRight = 0b00000100;
        const BottomLeft = 0b00001000;
    }
}

pub type Minos = [Point; 4];
pub type MinoPerimeter = [Perimeter; 4];
pub type MinoCorners = [Corner; 4];

pub fn minos_of(p0: (i32, i32), p1: (i32, i32), p2: (i32, i32), p3: (i32, i32)) -> Minos {
    [p0, p1, p2, p3].map(|(x, y)| Point::new(x, y))
}

const fn perimeter(minos: [u8; 4]) -> MinoPerimeter {
    // error[E0658]: `for` is not allowed in a `const fn`
    let mut result = [Perimeter::Top; 4];
    result[0] = Perimeter::from_bits_truncate(minos[0]);
    result[1] = Perimeter::from_bits_truncate(minos[1]);
    result[2] = Perimeter::from_bits_truncate(minos[2]);
    result[3] = Perimeter::from_bits_truncate(minos[3]);
    result
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TetrominoMeta {
    shape: TetrominoShape,
    spawn_point: Point,
    minos: Minos,
    perimeter: MinoPerimeter,
    outside_corners: MinoCorners,
    bounding_box: u32,
}

impl TetrominoMeta {
    pub fn wall_kicks(&self, from_rotation: Rotation, to_rotation: Rotation) -> Vec<Point> {
        let offsets_from = self.shape.offsets(&from_rotation);
        let offsets_to = self.shape.offsets(&to_rotation);

        let result = offsets_from
            .iter()
            .zip(offsets_to.iter())
            .map(move |(a, b)| Point::new(a.0 - b.0, a.1 - b.1))
            .collect::<Vec<Point>>();

        if self.shape == TetrominoShape::O {
            vec![result[0]]
        } else {
            result
        }
    }

    pub fn rotated_minos(&self, rotation: Rotation) -> Minos {
        if rotation == Rotation::North {
            return self.minos;
        }

        let center = (self.bounding_box as i32 - 1) / 2;
        let to_origin = -center;

        let (rotations, clockwise) = match rotation {
            Rotation::North => (0, true),
            Rotation::East => (1, true),
            Rotation::South => (2, true),
            Rotation::West => (1, false),
        };

        self.minos
            .iter()
            .map(|p| {
                let mut result = p.translate(to_origin, to_origin);
                for _ in 0..rotations {
                    result = result.rotate(clockwise);
                }
                result.translate(center, center)
            })
            .collect::<Vec<Point>>()
            .try_into()
            .unwrap()
    }

    pub fn normal_minos(&self) -> Minos {
        let normal_offset = Point::new(
            self.minos.iter().map(|p| p.x).min().unwrap(),
            self.minos.iter().map(|p| p.y).min().unwrap(),
        );
        self.minos.map(|p| p - normal_offset)
    }

    pub fn perimeter(&self) -> MinoPerimeter {
        self.perimeter
    }

    pub fn outside_corners(&self) -> MinoCorners {
        self.outside_corners
    }

    pub fn spawn_point(&self) -> Point {
        self.spawn_point
    }
}

const I: TetrominoMeta = TetrominoMeta {
    shape: TetrominoShape::I,
    spawn_point: Point::new(2, 18),
    minos: [
        Point::new(1, 2),
        Point::new(2, 2),
        Point::new(3, 2),
        Point::new(4, 2),
    ],
    perimeter: perimeter([
        Perimeter::Top.bits() | Perimeter::Bottom.bits() | Perimeter::Left.bits(),
        Perimeter::Top.bits() | Perimeter::Bottom.bits(),
        Perimeter::Top.bits() | Perimeter::Bottom.bits(),
        Perimeter::Top.bits() | Perimeter::Right.bits() | Perimeter::Bottom.bits(),
    ]),
    outside_corners: [Corner::None, Corner::None, Corner::None, Corner::None],
    bounding_box: 5,
};

const J: TetrominoMeta = TetrominoMeta {
    shape: TetrominoShape::J,
    spawn_point: Point::new(3, 19),
    minos: [
        Point::new(0, 1),
        Point::new(1, 1),
        Point::new(2, 1),
        Point::new(0, 2),
    ],
    perimeter: perimeter([
        Perimeter::Bottom.bits() | Perimeter::Left.bits(),
        Perimeter::Top.bits() | Perimeter::Bottom.bits(),
        Perimeter::Top.bits() | Perimeter::Right.bits() | Perimeter::Bottom.bits(),
        Perimeter::Top.bits() | Perimeter::Right.bits() | Perimeter::Left.bits(),
    ]),
    outside_corners: [Corner::TopRight, Corner::None, Corner::None, Corner::None],
    bounding_box: 3,
};

const L: TetrominoMeta = TetrominoMeta {
    shape: TetrominoShape::L,
    spawn_point: Point::new(3, 19),
    minos: [
        Point::new(0, 1),
        Point::new(1, 1),
        Point::new(2, 1),
        Point::new(2, 2),
    ],
    perimeter: perimeter([
        Perimeter::Top.bits() | Perimeter::Bottom.bits() | Perimeter::Left.bits(),
        Perimeter::Top.bits() | Perimeter::Bottom.bits(),
        Perimeter::Right.bits() | Perimeter::Bottom.bits(),
        Perimeter::Top.bits() | Perimeter::Right.bits() | Perimeter::Left.bits(),
    ]),
    outside_corners: [Corner::None, Corner::None, Corner::TopLeft, Corner::None],
    bounding_box: 3,
};

const O: TetrominoMeta = TetrominoMeta {
    shape: TetrominoShape::O,
    spawn_point: Point::new(3, 19),
    minos: [
        Point::new(1, 1),
        Point::new(2, 1),
        Point::new(1, 2),
        Point::new(2, 2),
    ],
    perimeter: perimeter([
        Perimeter::Bottom.bits() | Perimeter::Left.bits(),
        Perimeter::Right.bits() | Perimeter::Bottom.bits(),
        Perimeter::Top.bits() | Perimeter::Left.bits(),
        Perimeter::Top.bits() | Perimeter::Right.bits(),
    ]),
    outside_corners: [Corner::None, Corner::None, Corner::None, Corner::None],
    bounding_box: 3,
};

const S: TetrominoMeta = TetrominoMeta {
    shape: TetrominoShape::S,
    spawn_point: Point::new(3, 19),
    minos: [
        Point::new(0, 1),
        Point::new(1, 1),
        Point::new(1, 2),
        Point::new(2, 2),
    ],
    perimeter: perimeter([
        Perimeter::Top.bits() | Perimeter::Bottom.bits() | Perimeter::Left.bits(),
        Perimeter::Right.bits() | Perimeter::Bottom.bits(),
        Perimeter::Top.bits() | Perimeter::Left.bits(),
        Perimeter::Top.bits() | Perimeter::Right.bits() | Perimeter::Bottom.bits(),
    ]),
    outside_corners: [
        Corner::None,
        Corner::TopLeft,
        Corner::BottomRight,
        Corner::None,
    ],
    bounding_box: 3,
};

const T: TetrominoMeta = TetrominoMeta {
    shape: TetrominoShape::T,
    spawn_point: Point::new(3, 19),
    minos: [
        Point::new(0, 1),
        Point::new(1, 1),
        Point::new(2, 1),
        Point::new(1, 2),
    ],
    perimeter: perimeter([
        Perimeter::Top.bits() | Perimeter::Bottom.bits() | Perimeter::Left.bits(),
        Perimeter::Bottom.bits(),
        Perimeter::Top.bits() | Perimeter::Right.bits() | Perimeter::Bottom.bits(),
        Perimeter::Top.bits() | Perimeter::Right.bits() | Perimeter::Left.bits(),
    ]),
    outside_corners: [
        Corner::None,
        Corner::from_bits_truncate(Corner::TopLeft.bits() | Corner::TopRight.bits()),
        Corner::None,
        Corner::None,
    ],
    bounding_box: 3,
};

const Z: TetrominoMeta = TetrominoMeta {
    shape: TetrominoShape::Z,
    spawn_point: Point::new(3, 19),
    minos: [
        Point::new(1, 1),
        Point::new(2, 1),
        Point::new(0, 2),
        Point::new(1, 2),
    ],
    perimeter: perimeter([
        Perimeter::Bottom.bits() | Perimeter::Left.bits(),
        Perimeter::Top.bits() | Perimeter::Right.bits() | Perimeter::Bottom.bits(),
        Perimeter::Top.bits() | Perimeter::Bottom.bits() | Perimeter::Left.bits(),
        Perimeter::Top.bits() | Perimeter::Right.bits(),
    ]),
    outside_corners: [
        Corner::TopRight,
        Corner::None,
        Corner::None,
        Corner::BottomLeft,
    ],
    bounding_box: 3,
};

/// How the piece last got where it is. A spin is only a spin if the piece was rotated into
/// place, so this is what the guideline's three corner rule is gated on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum LastMove {
    #[default]
    Spawn,
    /// moved sideways, or fell
    Shift,
    /// rotated, and whether it took the last of the SRS kicks to fit. That kick is the one
    /// that wrenches a T into a slot it could not otherwise reach, so it always earns a full
    /// spin rather than a mini
    Rotation { last_kick: bool },
}

/// the index of the last SRS kick offset, the one that only a wrenched rotation reaches
const LAST_WALL_KICK: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tetromino {
    shape: TetrominoShape,
    pose: Pose,
    lock_placements: u32,
    y_min: i32,
    last_move: LastMove,
}

impl Tetromino {
    pub fn new(shape: TetrominoShape) -> Self {
        let position = shape.meta().spawn_point;
        Self {
            shape,
            pose: Pose::from_position(position),
            lock_placements: 0,
            y_min: position.y,
            last_move: LastMove::Spawn,
        }
    }

    pub fn pose(&self) -> Pose {
        self.pose
    }

    pub fn shape(&self) -> TetrominoShape {
        self.shape
    }

    pub fn rotation(&self) -> Rotation {
        self.pose.rotation
    }

    pub fn position(&self) -> Point {
        self.pose.position
    }

    pub fn minos(&self) -> Minos {
        self.meta()
            .rotated_minos(self.pose.rotation)
            .map(|p| p + self.pose.position)
    }

    pub fn translate(&mut self, x: i32, y: i32) {
        self.translate_point(Point::new(x, y));
        self.last_move = LastMove::Shift;
    }

    pub fn last_move(&self) -> LastMove {
        self.last_move
    }

    pub fn meta(&self) -> &TetrominoMeta {
        self.shape.meta()
    }

    pub fn possible_minos_after_rotation(&self, clockwise: bool) -> Vec<Minos> {
        let to_rotation = self.pose.rotation.rotate(clockwise);
        let meta = self.meta();
        let basic_rotation_minos = meta.rotated_minos(to_rotation);
        meta.wall_kicks(self.pose.rotation, to_rotation)
            .iter()
            .map(|kick| basic_rotation_minos.map(|p| p + self.pose.position + *kick))
            .collect::<Vec<Minos>>()
    }

    pub fn rotate(&mut self, clockwise: bool, wall_kick_id: usize) {
        let from_rotation = self.pose.rotation;
        let to_rotation = self.pose.rotate_mut(clockwise);
        let wall_kick = self.meta().wall_kicks(from_rotation, to_rotation)[wall_kick_id];
        self.translate_point(wall_kick);
        self.last_move = LastMove::Rotation {
            last_kick: wall_kick_id == LAST_WALL_KICK,
        };
    }

    fn translate_point(&mut self, p: Point) {
        self.pose.position += p;
        if self.pose.position.y < self.y_min {
            self.y_min = self.pose.position.y;
            // lock placements are reset every time a tetromino falls
            self.lock_placements = 0;
        }
    }

    pub fn register_lock_placement(&mut self) -> u32 {
        self.lock_placements += 1;
        self.lock_placements
    }

    pub fn lock_placements(&self) -> u32 {
        self.lock_placements
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wall_kicks_j() {
        assert_eq!(
            TetrominoShape::J
                .meta()
                .wall_kicks(Rotation::North, Rotation::East),
            [
                Point::new(0, 0),
                Point::new(-1, 0),
                Point::new(-1, 1),
                Point::new(0, -2),
                Point::new(-1, -2)
            ]
        );
    }

    #[test]
    fn clockwise_rotation_with_wall_kicks() {
        let tetromino = Tetromino::new(TetrominoShape::J);
        let observed = tetromino.possible_minos_after_rotation(true);

        assert_eq!(
            observed,
            vec![
                [
                    Point::new(4, 21),
                    Point::new(4, 20),
                    Point::new(4, 19),
                    Point::new(5, 21)
                ],
                [
                    Point::new(3, 21),
                    Point::new(3, 20),
                    Point::new(3, 19),
                    Point::new(4, 21)
                ],
                [
                    Point::new(3, 22),
                    Point::new(3, 21),
                    Point::new(3, 20),
                    Point::new(4, 22)
                ],
                [
                    Point::new(4, 19),
                    Point::new(4, 18),
                    Point::new(4, 17),
                    Point::new(5, 19)
                ],
                [
                    Point::new(3, 19),
                    Point::new(3, 18),
                    Point::new(3, 17),
                    Point::new(4, 19)
                ]
            ]
        );
    }

    #[test]
    fn minos() {
        let tetromino = Tetromino::new(TetrominoShape::L);
        assert_eq!(
            tetromino.minos(),
            [
                Point::new(3, 20),
                Point::new(4, 20),
                Point::new(5, 20),
                Point::new(5, 21)
            ]
        );
    }

    #[test]
    fn translate() {
        let mut tetromino = Tetromino::new(TetrominoShape::L);
        tetromino.translate(1, -1);
        assert_eq!(
            tetromino.minos(),
            [
                Point::new(4, 19),
                Point::new(5, 19),
                Point::new(6, 19),
                Point::new(6, 20)
            ]
        );
    }

    #[test]
    fn lock_placements_initial() {
        let tetromino = Tetromino::new(TetrominoShape::L);
        assert_eq!(tetromino.lock_placements(), 0);
    }

    #[test]
    fn register_lock_placement() {
        let mut tetromino = Tetromino::new(TetrominoShape::L);
        assert_eq!(tetromino.register_lock_placement(), 1);
    }

    #[test]
    fn lock_placement_reset_on_translation_below_y_min() {
        let mut tetromino = Tetromino::new(TetrominoShape::L);
        tetromino.register_lock_placement();
        tetromino.translate(0, -1);
        assert_eq!(tetromino.lock_placements(), 0);
    }

    #[test]
    fn lock_placement_not_reset_on_translation_above_y_min() {
        let mut tetromino = Tetromino::new(TetrominoShape::L);
        tetromino.translate(0, -2); // set y_min = -2
        tetromino.translate(0, 1); // translate above y_min
        tetromino.register_lock_placement();
        tetromino.translate(0, -1); // translate back to but not below y_min
        assert_eq!(tetromino.lock_placements(), 1);
    }

    #[test]
    fn lock_placement_not_reset_on_x_translation() {
        let mut tetromino = Tetromino::new(TetrominoShape::L);
        tetromino.register_lock_placement();
        tetromino.translate(1, 0);
        assert_eq!(tetromino.lock_placements(), 1);
    }

    #[test]
    fn normal_minos() {
        assert_eq!(
            TetrominoShape::I.meta().normal_minos(),
            [
                Point::new(0, 0),
                Point::new(1, 0),
                Point::new(2, 0),
                Point::new(3, 0),
            ]
        );
    }
}
