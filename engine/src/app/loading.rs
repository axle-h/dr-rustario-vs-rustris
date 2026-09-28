//! The progress bar shown while every theme of every game is built in `Shell::new`. They are
//! all built up front and stay built because the title screen's sprite race draws from all of
//! them.
//!
//! Presenting the first frame is what maps a Wayland window, and each draw pumps the event queue
//! so the compositor's configure and frame callbacks are answered. It pumps rather than polls so
//! the load's events, such as a pad's one `ControllerDeviceAdded`, reach the main loop.
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use sdl2::EventPump;

/// [`crate::app::App::new`] paints frame zero in this too, or the window flashes.
pub const BACKGROUND: Color = Color::RGB(0x08, 0x08, 0x0c);
const TRACK: Color = Color::RGB(0x2a, 0x2a, 0x36);
const FILL: Color = Color::RGB(0xe8, 0xe8, 0xf0);

/// the bar's share of the window's width, and its thickness relative to that
const BAR_WIDTH: f64 = 0.55;
const BAR_HEIGHT: f64 = 0.055;
const BAR_Y: f64 = 0.5;
/// the track's border, and the gap between the track and the fill
const BORDER: u32 = 3;

/// A progress bar over `steps` themes. A wrong count only makes the bar fill early or late,
/// since [`Loading::step`] saturates.
pub struct Loading {
    steps: u32,
    done: u32,
}

impl Loading {
    pub fn new(steps: u32) -> Self {
        Self {
            steps: steps.max(1),
            done: 0,
        }
    }

    /// one more theme is built; redraw
    pub fn step(
        &mut self,
        canvas: &mut WindowCanvas,
        events: &mut EventPump,
    ) -> Result<(), String> {
        self.done = (self.done + 1).min(self.steps);
        self.draw(canvas, events)
    }

    /// Draw and present the bar. It pumps first so a resize the compositor asked for is settled
    /// before the window is measured.
    pub fn draw(&self, canvas: &mut WindowCanvas, events: &mut EventPump) -> Result<(), String> {
        events.pump_events();
        let (width, height) = canvas.window().size();
        canvas.set_draw_color(BACKGROUND);
        canvas.clear();

        let bar_width = ((width as f64 * BAR_WIDTH) as u32).max(BORDER * 8);
        let bar_height = ((bar_width as f64 * BAR_HEIGHT) as u32).max(BORDER * 4);
        let x = (width.saturating_sub(bar_width) / 2) as i32;
        let y = ((height as f64 * BAR_Y) as u32).saturating_sub(bar_height / 2) as i32;

        canvas.set_draw_color(TRACK);
        canvas.fill_rect(Rect::new(x, y, bar_width, bar_height))?;

        // inset on every side, so an empty bar still reads as a bar
        let inset = BORDER * 2;
        if bar_width > inset && bar_height > inset {
            let full = bar_width - inset;
            let done = (full as f64 * self.done as f64 / self.steps as f64) as u32;
            if done > 0 {
                canvas.set_draw_color(FILL);
                canvas.fill_rect(Rect::new(
                    x + BORDER as i32,
                    y + BORDER as i32,
                    done,
                    bar_height - inset,
                ))?;
            }
        }

        canvas.present();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// zero steps is clamped to one, so sizing the fill never divides by zero
    #[test]
    fn a_bar_over_no_steps_is_still_a_bar() {
        let loading = Loading::new(0);
        assert_eq!(loading.steps, 1);
    }

    /// stepping past the count leaves the bar full
    #[test]
    fn stepping_past_the_end_saturates() {
        let mut loading = Loading::new(2);
        loading.done = 2;
        loading.done = (loading.done + 1).min(loading.steps);
        assert_eq!(loading.done, 2);
    }
}
