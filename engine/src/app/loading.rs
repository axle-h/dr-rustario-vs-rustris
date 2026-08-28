//! The bar the game shows while it builds its themes.
//!
//! It is here for two reasons and the first is not the obvious one. **A Wayland toplevel is not
//! mapped until the client commits a buffer**, and nothing used to present a frame between
//! creating the window and finishing the last theme - so for the whole of a load the window did
//! not exist as far as the compositor was concerned. On a PortMaster handheld that is what the
//! session's `swaymsg [app_id=...] fullscreen enable` helper could not find:
//!
//! ```text
//! Attempting to fullscreen app_id=dr-rustario-vs-rustris.aarch64
//! [{ "success": false, "parse_error": false, "error": "No matching node." }]
//! Failed, waiting to try again
//! ```
//!
//! Presenting frame zero the moment the window exists is what fixes that. The bar is the part
//! the player sees, and on a handheld - which decodes some thirty megabytes of embedded png and
//! ogg to build twelve themes - it is the difference between a progress bar and a black screen
//! that looks like a hang.
//!
//! Every theme is still built up front, deliberately: the title screen's sprite race draws from
//! all of them at once, and so does the particle field's silhouette bank, so there is no theme
//! this can afford to defer.
//!
//! The second reason is the event queue, and it is why the bar takes an [`EventPump`] to draw
//! itself. Presenting frame zero is not enough on its own: a Wayland client is handed its size
//! in a `configure` it has to acknowledge, and the compositor paces its commits with frame
//! callbacks - and both of those reach SDL only when the queue is dispatched. A load that
//! draws a frame per theme and never pumps acknowledges nothing, so on a PortMaster handheld,
//! whose session fullscreens the window from the outside *while* the load is running, every
//! frame after the first goes nowhere and the bar is a black screen.
//!
//! It pumps rather than polls, and that matters: the queue is left standing so the events of
//! the load are still there for the first poll of the main loop. `SDL_GAMECONTROLLERCONFIG`
//! pads announce themselves once, as `ControllerDeviceAdded`, and draining them here would
//! throw away the only notice the game gets of a pad that was plugged in before it started.
//!
//! It draws in flat rectangles and nothing else, because it runs before a single theme, font or
//! sprite sheet exists to draw with.

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use sdl2::EventPump;

/// What the screen is cleared to, and the two colours of the bar.
///
/// The background is `pub` because [`crate::app::App::new`] paints frame zero with it before
/// there is a `Loading` to ask - the two have to be the same colour or the window flashes.
pub const BACKGROUND: Color = Color::RGB(0x08, 0x08, 0x0c);
const TRACK: Color = Color::RGB(0x2a, 0x2a, 0x36);
const FILL: Color = Color::RGB(0xe8, 0xe8, 0xf0);

/// how much of the window's width the bar takes, and how thick it is relative to that
const BAR_WIDTH: f64 = 0.55;
const BAR_HEIGHT: f64 = 0.055;
/// where its middle sits down the window
const BAR_Y: f64 = 0.5;
/// the track's border, which is also the gap between the track and the fill
const BORDER: u32 = 3;

/// A progress bar over a job of `steps` steps.
///
/// `steps` is how many themes are about to be built. It is only ever used to size the fill, so
/// a count that turns out to be wrong makes the bar arrive early or late and nothing worse -
/// [`Loading::step`] saturates rather than running off the end.
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

    /// one more set of sprites is built; redraw
    pub fn step(
        &mut self,
        canvas: &mut WindowCanvas,
        events: &mut EventPump,
    ) -> Result<(), String> {
        self.done = (self.done + 1).min(self.steps);
        self.draw(canvas, events)
    }

    /// Draw the bar as it stands and put it on the screen.
    ///
    /// This presents, which is half the point of it: the first call is what maps the window.
    /// The other half is the pump, which comes first so that a resize the compositor asked for
    /// is settled before the window is measured for it - see this module's own docs, since a
    /// load that skips it draws every one of these frames into the dark.
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

        // the fill is inset by the border on every side, so an empty bar still reads as a bar
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

    /// a step count of nothing would divide by zero when the fill is sized
    #[test]
    fn a_bar_over_no_steps_is_still_a_bar() {
        let loading = Loading::new(0);
        assert_eq!(loading.steps, 1);
    }

    /// the caller's count and the number of themes actually built are worked out in different
    /// places, so they are allowed to disagree - the bar fills up and stays there
    #[test]
    fn stepping_past_the_end_saturates() {
        let mut loading = Loading::new(2);
        loading.done = 2;
        loading.done = (loading.done + 1).min(loading.steps);
        assert_eq!(loading.done, 2);
    }
}
