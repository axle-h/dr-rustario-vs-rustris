use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Neg, Sub};

/// A point on the board grid. `y` grows downwards from row 0, the top simulated row, which may
/// be hidden.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Ord for Point {
    fn cmp(&self, other: &Self) -> Ordering {
        self.x.cmp(&other.x).then_with(|| self.y.cmp(&other.y))
    }
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for Point {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Point {{ x: {}, y: {} }}", self.x, self.y)
    }
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Point {
        Point { x, y }
    }

    pub const fn from_u32(x: u32, y: u32) -> Point {
        Point {
            x: x as i32,
            y: y as i32,
        }
    }

    pub const fn x(&self) -> i32 {
        self.x
    }

    pub const fn y(&self) -> i32 {
        self.y
    }

    pub const fn translate(&self, x: i32, y: i32) -> Point {
        Point {
            x: self.x + x,
            y: self.y + y,
        }
    }

    pub fn translate_mut(&mut self, dx: i32, dy: i32) {
        self.x += dx;
        self.y += dy;
    }

    /// rotate about the origin by a quarter turn
    pub fn rotate(&self, clockwise: bool) -> Point {
        if clockwise {
            Point {
                x: self.y,
                y: -self.x,
            }
        } else {
            Point {
                x: -self.y,
                y: self.x,
            }
        }
    }
}

impl From<(i32, i32)> for Point {
    fn from((x, y): (i32, i32)) -> Self {
        Point::new(x, y)
    }
}

impl From<(u32, u32)> for Point {
    fn from((x, y): (u32, u32)) -> Self {
        Point::from_u32(x, y)
    }
}

impl Neg for Point {
    type Output = Point;

    fn neg(self) -> Self::Output {
        Point::new(-self.x, -self.y)
    }
}

impl Sub for Point {
    type Output = Point;

    fn sub(self, rhs: Self) -> Self::Output {
        Point::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Add for Point {
    type Output = Point;

    fn add(self, rhs: Self) -> Self::Output {
        self.translate(rhs.x, rhs.y)
    }
}

impl AddAssign for Point {
    fn add_assign(&mut self, rhs: Self) {
        self.translate_mut(rhs.x, rhs.y);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum Rotation {
    #[default]
    North,
    East,
    South,
    West,
}

impl Rotation {
    pub fn rotate(&self, clockwise: bool) -> Rotation {
        match (self, clockwise) {
            (Rotation::North, true) | (Rotation::South, false) => Rotation::East,
            (Rotation::East, true) | (Rotation::West, false) => Rotation::South,
            (Rotation::South, true) | (Rotation::North, false) => Rotation::West,
            (Rotation::West, true) | (Rotation::East, false) => Rotation::North,
        }
    }

    /// clockwise angle in degrees
    pub fn angle(&self) -> f64 {
        match self {
            Rotation::North => 0.0,
            Rotation::East => 90.0,
            Rotation::South => 180.0,
            Rotation::West => 270.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Pose {
    pub position: Point,
    pub rotation: Rotation,
}

impl Pose {
    pub fn new(position: Point, rotation: Rotation) -> Self {
        Self { position, rotation }
    }

    pub fn from_position(position: Point) -> Self {
        Self {
            position,
            rotation: Rotation::default(),
        }
    }

    pub fn translate(&self, x: i32, y: i32) -> Self {
        Self {
            position: self.position.translate(x, y),
            rotation: self.rotation,
        }
    }

    pub fn rotate(&self, clockwise: bool) -> Self {
        Self {
            position: self.position,
            rotation: self.rotation.rotate(clockwise),
        }
    }

    pub fn rotate_mut(&mut self, clockwise: bool) -> Rotation {
        self.rotation = self.rotation.rotate(clockwise);
        self.rotation
    }
}

impl Add<Point> for Pose {
    type Output = Self;

    fn add(self, rhs: Point) -> Self::Output {
        Self {
            position: self.position + rhs,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clockwise_rotation() {
        assert_eq!(Rotation::North.rotate(true), Rotation::East);
        assert_eq!(Rotation::East.rotate(true), Rotation::South);
        assert_eq!(Rotation::South.rotate(true), Rotation::West);
        assert_eq!(Rotation::West.rotate(true), Rotation::North);
    }

    #[test]
    fn anticlockwise_rotation() {
        assert_eq!(Rotation::North.rotate(false), Rotation::West);
        assert_eq!(Rotation::West.rotate(false), Rotation::South);
        assert_eq!(Rotation::South.rotate(false), Rotation::East);
        assert_eq!(Rotation::East.rotate(false), Rotation::North);
    }

    #[test]
    fn point_arithmetic() {
        let p = Point::new(1, 2);
        assert_eq!(p + Point::new(3, 4), Point::new(4, 6));
        assert_eq!(p - Point::new(3, 4), Point::new(-2, -2));
        assert_eq!(-p, Point::new(-1, -2));
        assert_eq!(p.rotate(true), Point::new(2, -1));
        assert_eq!(p.rotate(false), Point::new(-2, 1));
    }
}
