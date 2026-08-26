use crate::game::geometry::Point as CellPoint;
use sdl2::rect::{Point, Rect};

/// Where each cell of a board is drawn within a theme, in theme pixels.
#[derive(Debug, Copy, Clone)]
pub struct BoardGeometry {
    raw_block_size: u32,
    block_size: u32,
    columns: u32,
    /// rows the game simulates
    rows: u32,
    /// rows actually drawn, counted from the bottom
    visible_rows: u32,
    offset: Point,
}

impl BoardGeometry {
    /// `block_overlap` is added to the raw sprite size to get the cell pitch (negative to
    /// overlap sprites, positive for a gap). `offset` is where the *visible* board starts.
    pub fn new<P: Into<Point>>(
        raw_block_size: u32,
        block_overlap: i32,
        offset: P,
        columns: u32,
        rows: u32,
        visible_rows: u32,
    ) -> Self {
        let block_size = (raw_block_size as i32 + block_overlap) as u32;
        Self {
            raw_block_size,
            block_size,
            columns,
            rows,
            visible_rows: visible_rows.min(rows),
            offset: offset.into(),
        }
    }

    fn i_to_x(&self, i: i32) -> i32 {
        i * self.block_size as i32 + self.offset.x()
    }

    fn j_to_y(&self, j: i32) -> i32 {
        (j - self.hidden_rows() as i32) * self.block_size as i32 + self.offset.y()
    }

    pub fn block_size(&self) -> u32 {
        self.block_size
    }

    pub fn raw_block_size(&self) -> u32 {
        self.raw_block_size
    }

    pub fn columns(&self) -> u32 {
        self.columns
    }

    pub fn rows(&self) -> u32 {
        self.rows
    }

    pub fn visible_rows(&self) -> u32 {
        self.visible_rows
    }

    pub fn hidden_rows(&self) -> u32 {
        self.rows - self.visible_rows
    }

    pub fn is_visible(&self, point: CellPoint) -> bool {
        point.y >= self.hidden_rows() as i32
    }

    /// height of the visible board
    pub fn height(&self) -> u32 {
        self.block_size * self.visible_rows
    }

    pub fn width(&self) -> u32 {
        self.block_size * self.columns
    }

    pub fn offset(&self) -> Point {
        self.offset
    }

    pub fn point<P: Into<CellPoint>>(&self, point: P) -> Point {
        let point = point.into();
        Point::new(self.i_to_x(point.x), self.j_to_y(point.y))
    }

    pub fn raw_block<P: Into<CellPoint>>(&self, point: P) -> Rect {
        let point = point.into();
        Rect::new(
            self.i_to_x(point.x),
            self.j_to_y(point.y),
            self.raw_block_size,
            self.raw_block_size,
        )
    }

    /// the full cell including any overlap/gap
    pub fn block<P: Into<CellPoint>>(&self, point: P) -> Rect {
        let point = point.into();
        Rect::new(
            self.i_to_x(point.x),
            self.j_to_y(point.y),
            self.block_size,
            self.block_size,
        )
    }

    pub fn row_snip(&self, j: u32) -> Rect {
        Rect::new(
            self.offset.x(),
            self.j_to_y(j as i32),
            self.width(),
            self.block_size,
        )
    }

    pub fn game_snip(&self) -> Rect {
        Rect::new(
            self.offset.x(),
            self.offset.y(),
            self.width(),
            self.height(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nes_dr_mario() {
        // these are the real nes dr mario coordinates
        let geometry = BoardGeometry::new(7, 1, (96, 72), 8, 16, 16);
        assert_eq!(geometry.game_snip(), Rect::new(96, 72, 64, 128));
        assert_eq!(geometry.raw_block((0, 15)), Rect::new(96, 192, 7, 7));
        assert_eq!(geometry.raw_block((4, 12)), Rect::new(128, 168, 7, 7));
    }

    #[test]
    fn hidden_rows_sit_above_the_board() {
        let geometry = BoardGeometry::new(8, 0, (0, 0), 10, 40, 22);
        assert_eq!(geometry.hidden_rows(), 18);
        assert_eq!(geometry.raw_block((0, 18)), Rect::new(0, 0, 8, 8));
        assert_eq!(geometry.raw_block((0, 17)), Rect::new(0, -8, 8, 8));
        assert!(!geometry.is_visible(CellPoint::new(0, 17)));
        assert_eq!(geometry.height(), 22 * 8);
    }
}
