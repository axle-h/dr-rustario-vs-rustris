//! One board size and position shared by every theme a player can switch to.

use crate::config::VideoConfig;
use crate::render::Theme;
use crate::scale::{fit, player_area, ScaleMode};
use sdl2::rect::{Point, Rect};

/// Where a theme's playfield sits inside its own background, in window pixels once scaled.
struct Margins {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    /// how far the top of the art may rise above the player's area
    slack: i32,
}

impl Margins {
    fn new(theme: &Theme, scale: f64) -> Self {
        let scaled = |value: i32| (value as f64 * scale).round() as i32;
        let playfield = theme.playfield_snip();
        let (width, height) = theme.background_size();
        Self {
            left: scaled(playfield.x()),
            top: scaled(playfield.y()),
            right: scaled(width as i32 - playfield.right()),
            bottom: scaled(height as i32 - playfield.bottom()),
            slack: scaled(theme.top_slack() as i32),
        }
    }
}

/// The biggest cell `themes` can reach with the window to themselves. A theme that renders
/// its art at its final size is built to this, so it can match them however many players end
/// up sharing the screen: it is drawn at its built size or smaller, but never larger.
pub fn reference_block_size(
    themes: &[&Theme],
    window_size: (u32, u32),
    config: VideoConfig,
) -> u32 {
    let area = player_area(0, 1, window_size);
    themes
        .iter()
        .map(|theme| max_block_size(theme, area, config))
        .min()
        .unwrap_or(1)
}

/// the biggest cell one theme can reach in `area`
fn max_block_size(theme: &Theme, area: Rect, config: VideoConfig) -> u32 {
    let scale = fit(
        area,
        theme.background_size(),
        theme.top_slack(),
        theme.scale_mode(),
        config.integer_scale,
    );
    (theme.geometry().block_size() as f64 * scale)
        .floor()
        .max(1.0) as u32
}

/// The themes one player cycles through, laid out together: they take the largest cell pitch
/// all of them can hold and put the playfield in exactly the same place, so changing theme
/// mid-game neither moves nor resizes the board.
#[derive(Clone, Debug)]
pub struct BoardLayout {
    /// cell pitch in window pixels, per theme in the order they were given
    block_sizes: Vec<u32>,
    /// where the middle of the playfield goes, one per player
    centres: Vec<Point>,
    /// the window each player's theme is drawn into
    areas: Vec<Rect>,
}

impl BoardLayout {
    pub fn new(
        themes: &[&Theme],
        players: u32,
        window_size: (u32, u32),
        config: VideoConfig,
    ) -> Self {
        let areas = (0..players)
            .map(|player| player_area(player, players, window_size))
            .collect::<Vec<Rect>>();
        // every player has the same room, give or take a rounding pixel
        let smallest = areas
            .iter()
            .copied()
            .min_by_key(|a| (a.width(), a.height()))
            .unwrap_or_else(|| Rect::new(0, 0, window_size.0, window_size.1));

        let block_sizes = Self::block_sizes(themes, smallest, config);
        let centres = areas
            .iter()
            .map(|area| Self::place(themes, &block_sizes, *area))
            .collect();

        Self {
            block_sizes,
            centres,
            areas,
        }
    }

    /// The cell pitch each theme takes. They aim at the biggest cell every one of them can
    /// hold, but pixel art only looks right at a whole multiple of its own cell pitch, so a
    /// theme drawn at a different source size takes the multiple nearest that target instead.
    fn block_sizes(themes: &[&Theme], area: Rect, config: VideoConfig) -> Vec<u32> {
        let largest = themes
            .iter()
            .map(|theme| max_block_size(theme, area, config))
            .collect::<Vec<u32>>();
        let target = largest.iter().copied().min().unwrap_or(1);
        themes
            .iter()
            .zip(largest)
            .map(|(theme, largest)| {
                let source = theme.geometry().block_size().max(1);
                if theme.scale_mode() != ScaleMode::Source || !config.integer_scale {
                    return target.min(largest).max(1);
                }
                let steps = ((target as f64 / source as f64).round() as u32).max(1);
                (steps * source).min(largest).max(source.min(largest))
            })
            .collect()
    }

    /// where one theme may put the middle of its playfield to keep its art inside `area`
    fn bounds(theme: &Theme, block_size: u32, area: Rect) -> (i32, i32, i32, i32) {
        let margins = Margins::new(theme, Self::scale_of(theme, block_size));
        let half_width = (theme.geometry().columns() * block_size) as i32 / 2;
        let half_height = (theme.geometry().visible_rows() * block_size) as i32 / 2;
        (
            area.left() + margins.left + half_width,
            area.right() - margins.right - half_width,
            area.top() + margins.top - margins.slack + half_height,
            area.bottom() - margins.bottom - half_height,
        )
    }

    /// the middle of the playfield, as near the middle of `area` as every theme's art allows
    fn place(themes: &[&Theme], block_sizes: &[u32], area: Rect) -> Point {
        let mut min_x = i32::MIN;
        let mut max_x = i32::MAX;
        let mut min_y = i32::MIN;
        let mut max_y = i32::MAX;
        for (theme, block_size) in themes.iter().zip(block_sizes.iter().copied()) {
            let (theme_min_x, theme_max_x, theme_min_y, theme_max_y) =
                Self::bounds(theme, block_size, area);
            min_x = min_x.max(theme_min_x);
            max_x = max_x.min(theme_max_x);
            min_y = min_y.max(theme_min_y);
            max_y = max_y.min(theme_max_y);
        }
        let centre = area.center();
        Point::new(
            Self::nearest(centre.x(), min_x, max_x),
            Self::nearest(centre.y(), min_y, max_y),
        )
    }

    /// `wanted` clamped into `[min, max]`; when the themes leave no common range at all, split
    /// the difference and let each of them clamp its own art back into the window
    fn nearest(wanted: i32, min: i32, max: i32) -> i32 {
        if min > max {
            (min + max) / 2
        } else {
            wanted.clamp(min, max)
        }
    }

    fn scale_of(theme: &Theme, block_size: u32) -> f64 {
        block_size as f64 / theme.geometry().block_size().max(1) as f64
    }

    pub fn scale(&self, index: usize, theme: &Theme) -> f64 {
        Self::scale_of(theme, self.block_sizes[index])
    }

    /// where this theme's playfield goes: the group's shared centre, at this theme's size,
    /// nudged back inside the window in the rare case the group cannot agree on one point
    pub fn playfield(&self, index: usize, theme: &Theme, player: u32) -> Rect {
        let block_size = self.block_sizes[index];
        let area = self.areas[player as usize];
        let (min_x, max_x, min_y, max_y) = Self::bounds(theme, block_size, area);
        let centre = self.centres[player as usize];
        Rect::from_center(
            Point::new(
                Self::nearest(centre.x(), min_x, max_x),
                Self::nearest(centre.y(), min_y, max_y),
            ),
            theme.geometry().columns() * block_size,
            theme.geometry().visible_rows() * block_size,
        )
    }
}
