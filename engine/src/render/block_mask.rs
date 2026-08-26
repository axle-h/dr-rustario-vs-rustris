use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Texture, WindowCanvas};

const BASE_TRANSPARENCY: u8 = 0;
/// where the alpha byte of an `ARGB8888` pixel lands once SDL has written the packed 32-bit
/// value into a byte buffer: last on a little-endian machine, first on a big-endian one
#[cfg(target_endian = "little")]
const ALPHA_BYTE: usize = 3;
#[cfg(target_endian = "big")]
const ALPHA_BYTE: usize = 0;

#[derive(Debug, Clone)]
pub struct BlockMask {
    width: u32,
    height: u32,
    mask: Vec<bool>,
}

impl BlockMask {
    pub fn from_texture(
        canvas: &mut WindowCanvas,
        texture: &mut Texture,
        rect: Rect,
    ) -> Result<Self, String> {
        let mut pixels = vec![];
        canvas
            .with_texture_canvas(texture, |c| {
                pixels = c.read_pixels(rect, PixelFormatEnum::ARGB8888).unwrap()
            })
            .map_err(|e| e.to_string())?;

        Ok(Self::from_pixels(&pixels, rect.width(), rect.height()))
    }

    /// `pixels` is `ARGB8888`, which SDL packs into a 32-bit word and then writes out in the
    /// machine's byte order, so the alpha byte is the last of the four here rather than the
    /// first. Reading the first byte instead reads *blue*, which quietly masks out every
    /// sprite with none in it: a red pill or a yellow tetromino comes out empty.
    fn from_pixels(pixels: &[u8], width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            mask: pixels
                .chunks(4)
                .map(|chunk| chunk[ALPHA_BYTE] > BASE_TRANSPARENCY)
                .collect(),
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn is_set(&self, x: u32, y: u32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        // the mask is row major, so the stride is the width
        self.mask[y as usize * self.width as usize + x as usize]
    }

    /// every lattice point of the mask that is set. The last row and column are included even
    /// when the mask does not divide exactly by `spacing`, so an edge is never dropped.
    pub fn lattice(&self, offset: Point, spacing: u32) -> Vec<Point> {
        let mut result = vec![];
        for y in Self::steps(self.height, spacing) {
            for x in Self::steps(self.width, spacing) {
                if self.is_set(x, y) {
                    result.push(Point::new(offset.x() + x as i32, offset.y() + y as i32));
                }
            }
        }
        result
    }

    /// the lattice points where the mask is set and at least one 4-neighbour is not: the
    /// outline of the silhouette, which is all a particle field ever draws
    pub fn edges(&self, offset: Point, spacing: u32) -> Vec<Point> {
        let spacing = spacing.max(1);
        let mut result = vec![];
        for y in Self::steps(self.height, spacing) {
            for x in Self::steps(self.width, spacing) {
                if !self.is_set(x, y) {
                    continue;
                }
                let is_edge = [
                    (x.checked_sub(spacing), Some(y)),
                    (Some(x + spacing), Some(y)),
                    (Some(x), y.checked_sub(spacing)),
                    (Some(x), Some(y + spacing)),
                ]
                .into_iter()
                .any(|neighbour| match neighbour {
                    (Some(nx), Some(ny)) => !self.is_set(nx, ny),
                    // off the top or left of the mask: the block is at the boundary
                    _ => true,
                });
                if is_edge {
                    result.push(Point::new(offset.x() + x as i32, offset.y() + y as i32));
                }
            }
        }
        result
    }

    /// `spacing` apart from 0 to the last row or column inclusive, distributed evenly so that
    /// both edges are sampled without the final step landing on top of the one before it: a
    /// doubled bottom row shows up as a solid line of particles against a dotted outline.
    fn steps(size: u32, spacing: u32) -> impl Iterator<Item = u32> {
        let spacing = spacing.max(1);
        let last = size.saturating_sub(1) as u64;
        let steps = if last == 0 {
            0
        } else {
            ((last as f64 / spacing as f64).round() as u64).max(1)
        };
        (0..=steps).map(move |i| match i.checked_mul(last).and_then(|v| v.checked_div(steps)) {
            Some(value) => value as u32,
            // a mask one pixel across has a single step, at zero
            None => 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// a 7x3 mask, deliberately not square and not a multiple of the spacing:
    /// ```text
    /// # # . . . # #
    /// # # . . . # #
    /// . . . . . # #
    /// ```
    fn mask() -> BlockMask {
        let rows = [
            [true, true, false, false, false, true, true],
            [true, true, false, false, false, true, true],
            [false, false, false, false, false, true, true],
        ];
        BlockMask {
            width: 7,
            height: 3,
            mask: rows.into_iter().flatten().collect(),
        }
    }

    /// one `ARGB8888` pixel as SDL writes it out: the packed word in the machine's byte order
    fn pixel(a: u8, r: u8, g: u8, b: u8) -> [u8; 4] {
        u32::from_be_bytes([a, r, g, b]).to_ne_bytes()
    }

    #[test]
    fn a_sprite_with_no_blue_in_it_still_has_a_mask() {
        let pixels: Vec<u8> = [
            pixel(0, 0, 0, 0),
            pixel(255, 255, 0, 0),
            pixel(255, 255, 255, 0),
            pixel(255, 0, 0, 255),
        ]
        .into_iter()
        .flatten()
        .collect();
        let mask = BlockMask::from_pixels(&pixels, 4, 1);
        // transparent, then opaque red, yellow and blue: everything but the first
        assert!(!mask.is_set(0, 0));
        assert!(mask.is_set(1, 0), "an opaque red pixel is part of its sprite");
        assert!(mask.is_set(2, 0), "an opaque yellow pixel is part of its sprite");
        assert!(mask.is_set(3, 0));
    }

    #[test]
    fn steps_reach_both_edges_without_doubling_up_at_the_end() {
        // an exact fit is the plain lattice
        assert_eq!(BlockMask::steps(7, 2).collect::<Vec<u32>>(), vec![0, 2, 4, 6]);
        // and an inexact one is spread evenly rather than tacking the last index on next to
        // the one before it
        assert_eq!(BlockMask::steps(8, 2).collect::<Vec<u32>>(), vec![0, 1, 3, 5, 7]);
        assert_eq!(BlockMask::steps(3, 5).collect::<Vec<u32>>(), vec![0, 2]);
        assert_eq!(BlockMask::steps(1, 5).collect::<Vec<u32>>(), vec![0]);
        for size in 1..40u32 {
            for spacing in 1..12u32 {
                let steps = BlockMask::steps(size, spacing).collect::<Vec<u32>>();
                assert_eq!(*steps.first().unwrap(), 0, "{size} {spacing}");
                assert_eq!(*steps.last().unwrap(), size - 1, "{size} {spacing}");
                assert!(
                    steps.windows(2).all(|w| w[0] < w[1]),
                    "{size} {spacing} {steps:?}"
                );
            }
        }
    }

    #[test]
    fn lattice_indexes_a_non_square_mask_by_its_width() {
        // spacing 1 is every set pixel, which only comes out right with a width stride
        let observed = mask().lattice(Point::new(0, 0), 1);
        assert_eq!(observed.len(), 10);
        assert!(observed.contains(&Point::new(6, 2)));
        assert!(!observed.contains(&Point::new(0, 2)));
    }

    #[test]
    fn lattice_keeps_the_last_partial_row_and_column() {
        // 3 / 2 = 1 whole step, so the bottom row is only reached by the last-index step
        let observed = mask().lattice(Point::new(0, 0), 2);
        assert!(observed.contains(&Point::new(6, 2)), "{observed:?}");
        assert!(observed.contains(&Point::new(6, 0)), "{observed:?}");
    }

    #[test]
    fn lattice_offsets_every_point() {
        let observed = mask().lattice(Point::new(10, 20), 1);
        assert!(observed.contains(&Point::new(10, 20)));
        assert!(observed.contains(&Point::new(16, 22)));
    }

    #[test]
    fn edges_drops_the_interior() {
        // a solid 5x5 block: every point but the middle 3x3 is an edge
        let solid = BlockMask {
            width: 5,
            height: 5,
            mask: vec![true; 25],
        };
        let observed = solid.edges(Point::new(0, 0), 1);
        assert_eq!(observed.len(), 16);
        assert!(!observed.contains(&Point::new(2, 2)));
        assert!(observed.contains(&Point::new(0, 0)));
        assert!(observed.contains(&Point::new(4, 4)));
    }

    #[test]
    fn every_point_of_a_thin_mask_is_an_edge() {
        let observed = mask().edges(Point::new(0, 0), 1);
        assert_eq!(observed.len(), mask().lattice(Point::new(0, 0), 1).len());
    }
}
