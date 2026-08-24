//! The race clock of a sprint.

use crate::font::FontType;
use crate::high_score::table::format_millis;
use crate::render::font::FontRender;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::time::Duration;

/// font em height as a fraction of the window height
const FONT_PCT: f64 = 1.0 / 24.0;
const BADGE_COLOR: Color = Color::RGBA(0, 0, 0, 0xa0);
const TEXT_COLOR: Color = Color::WHITE;

/// Draws the match's race clock in a translucent badge, out of everyone's eye-line and in
/// window space no theme's art reaches: tucked into the bottom-left corner for a single
/// player, or bottom-centre, between the players, for a multiplayer race.
pub struct TimerRender<'a> {
    font: FontRender<'a>,
    padding: u32,
    window: (u32, u32),
    centred: bool,
}

impl<'a> TimerRender<'a> {
    pub fn new(
        canvas: &mut WindowCanvas,
        texture_creator: &'a TextureCreator<WindowContext>,
        window: (u32, u32),
        players: u32,
    ) -> Result<Self, String> {
        let font_size = (((window.1 as f64) * FONT_PCT).round() as u32).max(8);
        let font = FontRender::from_font(
            canvas,
            texture_creator,
            FontType::Retro,
            font_size,
            TEXT_COLOR,
        )?;
        Ok(Self {
            font,
            padding: (font_size / 5).max(2),
            window,
            centred: players > 1,
        })
    }

    pub fn draw(&self, canvas: &mut WindowCanvas, time: Duration) -> Result<(), String> {
        let text = format_millis(time.as_millis().min(u32::MAX as u128) as u32);
        let (text_width, text_height) = self.font.string_size(&text);
        let badge_width = text_width + 2 * self.padding;
        let badge_height = text_height + self.padding;
        let margin = 2 * self.padding;
        let (window_width, window_height) = self.window;
        let x = if self.centred {
            (window_width - badge_width) as i32 / 2
        } else {
            margin as i32
        };
        let y = window_height as i32 - badge_height as i32 - margin as i32;
        let badge = Rect::new(x, y, badge_width, badge_height);

        canvas.set_blend_mode(BlendMode::Blend);
        canvas.set_draw_color(BADGE_COLOR);
        canvas.fill_rect(badge)?;
        self.font.render_string_in_center(canvas, badge, &text)
    }
}
