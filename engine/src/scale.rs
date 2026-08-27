use sdl2::rect::{Point, Rect};

const PLAYER_BUFFER_PCT: f64 = 0.002;

/// How a theme's art may be resized to the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScaleMode {
    /// pixel art drawn at a source resolution: it scales up, by whole pixels when the config
    /// asks for integer scaling
    Source,
    /// art already rendered at its final size: it only ever scales down, and smoothly
    Native,
}

/// The area of the window one player's theme is drawn into.
pub fn player_area(player: u32, players: u32, (window_width, window_height): (u32, u32)) -> Rect {
    let inset_x = (PLAYER_BUFFER_PCT * window_width as f64).round() as i32;
    let inset_y = (PLAYER_BUFFER_PCT * window_height as f64).round() as i32;
    let chunk = window_width / players;
    Rect::new(
        (chunk * player) as i32 + inset_x,
        inset_y,
        chunk.saturating_sub(2 * inset_x as u32).max(1),
        window_height.saturating_sub(2 * inset_y as u32).max(1),
    )
}

/// The largest scale at which art of `size` source pixels fits `area`. The top `top_slack`
/// source pixels are empty in every theme that has them, so they are allowed to fall outside
/// the area rather than hold the whole board back.
pub fn fit(
    area: Rect,
    (width, height): (u32, u32),
    top_slack: u32,
    mode: ScaleMode,
    integer: bool,
) -> f64 {
    let required_height = height.saturating_sub(top_slack).max(1);
    let by_width = area.width() as f64 / width.max(1) as f64;
    let by_height = area.height() as f64 / required_height as f64;
    let scale = by_width.min(by_height);
    match mode {
        // native art is already the right size: it may shrink to share the window, never grow
        ScaleMode::Native => scale.min(1.0),
        ScaleMode::Source if integer => (scale.floor()).max(1.0),
        ScaleMode::Source => scale,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scale {
    players: u32,
    scale: f64,
    integer_scale: Option<u32>,
    window_width: u32,
    window_height: u32,
    block_size: f64,
}

impl Scale {
    pub fn new(
        players: u32,
        (window_width, window_height): (u32, u32),
        block_size: u32,
        scale: f64,
    ) -> Self {
        let integer_scale = if scale >= 1.0 && (scale - scale.round()).abs() < f64::EPSILON {
            Some(scale.round() as u32)
        } else {
            None
        };
        Self {
            players,
            scale,
            integer_scale,
            window_width,
            window_height,
            block_size: block_size as f64 * scale,
        }
    }

    /// the exact vertical strip of the window belonging to a player, with no buffer offset.
    /// used to clip full-window drawing (scene backdrop, fades) to one player's side.
    pub fn player_clip(&self, player: u32) -> Rect {
        let x = (self.window_width as u64 * player as u64 / self.players as u64) as i32;
        let next_x = (self.window_width as u64 * (player as u64 + 1) / self.players as u64) as i32;
        Rect::new(x, 0, (next_x - x) as u32, self.window_height)
    }

    pub fn scale_and_offset_rect(&self, rect: Rect, offset_x: i32, offset_y: i32) -> Rect {
        Rect::new(
            self.scale_coordinate(rect.x) + offset_x,
            self.scale_coordinate(rect.y) + offset_y,
            self.scale_length(rect.width()),
            self.scale_length(rect.height()),
        )
    }

    pub fn scale_rect(&self, rect: Rect) -> Rect {
        self.scale_and_offset_rect(rect, 0, 0)
    }

    pub fn scale_and_offset_point(&self, point: Point, offset_x: i32, offset_y: i32) -> Point {
        Point::new(
            self.scale_coordinate(point.x) + offset_x,
            self.scale_coordinate(point.y) + offset_y,
        )
    }

    pub fn offset_proportional_to_block_size(
        &self,
        rect: Rect,
        offset_x: f64,
        offset_y: f64,
    ) -> Rect {
        let block_size = self.block_size;
        Rect::new(
            (rect.x as f64 + offset_x * block_size).round() as i32,
            (rect.y as f64 + offset_y * block_size).round() as i32,
            rect.width(),
            rect.height(),
        )
    }

    /// how much the theme's art is being scaled by, for something drawn on the window rather
    /// than into a theme's own textures and so scaled by hand
    pub fn factor(&self) -> f64 {
        self.scale
    }

    pub fn window_size(&self) -> (u32, u32) {
        (self.window_width, self.window_height)
    }

    pub fn scale_length(&self, value: u32) -> u32 {
        if let Some(integer_scale) = self.integer_scale {
            value * integer_scale
        } else {
            (value as f64 * self.scale).round() as u32
        }
    }

    pub fn scale_coordinate(&self, value: i32) -> i32 {
        if let Some(integer_scale) = self.integer_scale {
            value * integer_scale as i32
        } else {
            (value as f64 * self.scale).round() as i32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_player_buffer_is_window_pixels_not_source_pixels() {
        // two 210x183 backgrounds fit side by side at 3x in 1280 wide: 1260 of art and 12 of
        // buffer. Charging the buffer at 3x each would cost a whole step.
        let area = player_area(1, 2, (1280, 720));
        assert_eq!(area.width(), 634);
        assert_eq!(area.left(), 643);
        assert_eq!(fit(area, (210, 183), 16, ScaleMode::Source, true), 3.0);
    }

    #[test]
    fn the_empty_gap_above_a_board_may_fall_off_the_top() {
        let area = player_area(0, 1, (1280, 720));
        // 183 * 4 is taller than the window, but 183 - 16 of it is not
        assert_eq!(fit(area, (210, 183), 0, ScaleMode::Source, true), 3.0);
        assert_eq!(fit(area, (210, 183), 16, ScaleMode::Source, true), 4.0);
    }

    #[test]
    fn smooth_scaling_fills_more_of_the_window_than_the_integer_step_below() {
        let area = player_area(0, 1, (1280, 720));
        // the tallest rustris theme, which nothing may be clipped off
        let smooth = fit(area, (209, 190), 0, ScaleMode::Source, false);
        let integer = fit(area, (209, 190), 0, ScaleMode::Source, true);
        assert_eq!(integer, 3.0);
        assert!(smooth > 3.7 && smooth < 3.8, "{smooth}");
    }

    #[test]
    fn native_art_shrinks_but_never_grows() {
        let area = player_area(0, 1, (1280, 720));
        assert_eq!(fit(area, (400, 693), 0, ScaleMode::Native, true), 1.0);
        assert_eq!(fit(area, (400, 1436), 0, ScaleMode::Native, true), 0.5);
    }

    #[test]
    fn source_art_scales_smoothly_when_integer_scaling_is_off() {
        let area = player_area(0, 1, (1280, 720));
        let scale = fit(area, (210, 183), 16, ScaleMode::Source, false);
        assert!(scale > 4.2 && scale < 4.4, "{scale}");
    }
}
