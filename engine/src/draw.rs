use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas, RenderTarget};

/// `(x_offset, width)` of each row of a filled rounded rectangle, relative to its top-left; the
/// radius is clamped to half the smaller side so corners never overlap.
fn rounded_rect_rows(width: u32, height: u32, radius: u32) -> Vec<(u32, u32)> {
    let radius = radius.min(width / 2).min(height / 2) as i64;
    let (w, h) = (width as i64, height as i64);
    (0..h)
        .map(|row| {
            // distance of this row's centre from the corner circle's centre, in the corner bands
            let dy = if row < radius {
                radius - row
            } else if row >= h - radius {
                row - (h - radius - 1)
            } else {
                0
            };
            let dx = radius - (((radius * radius - dy * dy).max(0) as f64).sqrt().round() as i64);
            (dx as u32, (w - 2 * dx).max(0) as u32)
        })
        .collect()
}

pub trait CanvasExt {
    /// fills `rect`, rounding the corners by `radius` (alpha-blended)
    fn fill_rounded_rect(&mut self, rect: Rect, radius: u32, color: Color) -> Result<(), String>;
}

impl<T: RenderTarget> CanvasExt for Canvas<T> {
    fn fill_rounded_rect(&mut self, rect: Rect, radius: u32, color: Color) -> Result<(), String> {
        let previous_color = self.draw_color();
        let previous_blend = self.blend_mode();
        self.set_draw_color(color);
        self.set_blend_mode(BlendMode::Blend);
        let rects: Vec<Rect> = rounded_rect_rows(rect.width(), rect.height(), radius)
            .into_iter()
            .enumerate()
            .filter(|(_, (_, w))| *w > 0)
            .map(|(row, (dx, w))| Rect::new(rect.x() + dx as i32, rect.y() + row as i32, w, 1))
            .collect();
        let result = self.fill_rects(&rects);
        self.set_draw_color(previous_color);
        self.set_blend_mode(previous_blend);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_radius_is_plain_rect() {
        assert_eq!(rounded_rect_rows(10, 4, 0), vec![(0, 10); 4]);
    }

    #[test]
    fn rows_are_vertically_symmetric_and_full_width_in_the_middle() {
        let rows = rounded_rect_rows(40, 20, 6);
        assert_eq!(rows.len(), 20);
        for i in 0..20 {
            assert_eq!(rows[i], rows[19 - i], "row {i}");
        }
        assert_eq!(rows[10], (0, 40));
        assert!(rows[0].0 > 0 && rows[0].1 < 40);
        assert!(rows[0].0 >= rows[1].0);
    }

    #[test]
    fn radius_is_clamped_to_half_height() {
        let rows = rounded_rect_rows(40, 10, 100);
        assert_eq!(rows.len(), 10);
        assert!(rows.iter().all(|(dx, w)| *dx <= 5 && dx * 2 + w == 40));
    }
}
