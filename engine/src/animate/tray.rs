//! Attacks arriving in the tray.
//!
//! An attack is *routed* the moment the chain that earned it ends, and the receiving game
//! puts it in its tray there and then - so without this the icons appear a third of a second
//! before the ball that is carrying them across the window lands. This holds the new ones
//! back until it does, and then slides them into their slots from over the middle of the
//! board, which is what Mean Bean Machine does.
//!
//! Decoration: it holds nothing and changes no rule - [`crate::game::Game::pending_attacks`]
//! still says what is queued, and this only delays *drawing* part of it by the flight time.
//! A theme with no [`crate::render::PendingLayout`] never sees any of it.

use std::time::Duration;

/// how long an icon takes to slide into its slot
const SLIDE: Duration = Duration::from_millis(250);

#[derive(Clone, Debug, Default)]
pub struct TrayAnimation {
    /// the tray had this many icons when the attack left; everything past it is in the air
    in_flight: Option<usize>,
    /// ... and once it lands, the slots from here on are sliding in
    sliding: Option<(usize, Duration)>,
}

impl TrayAnimation {
    pub fn new() -> Self {
        Self::default()
    }

    /// an attack is on its way; `held` is how many icons the tray had before it
    pub fn expect(&mut self, held: usize) {
        // an attack already in the air keeps its own count: the earlier one is the one whose
        // icons are furthest along, and the later ball reveals whatever is left
        self.in_flight.get_or_insert(held);
    }

    /// the attack landed: whatever it brought slides in
    pub fn arrive(&mut self) {
        if let Some(held) = self.in_flight.take() {
            self.sliding = Some((held, Duration::ZERO));
        }
    }

    pub fn update(&mut self, delta: Duration) {
        if let Some((_, elapsed)) = self.sliding.as_mut() {
            *elapsed += delta;
            if *elapsed >= SLIDE {
                self.sliding = None;
            }
        }
    }

    pub fn reset(&mut self) {
        self.in_flight = None;
        self.sliding = None;
    }

    /// how many of the `queued` icons to draw at all; the rest are still crossing the window
    pub fn visible(&self, queued: usize) -> usize {
        self.in_flight.map_or(queued, |held| held.min(queued))
    }

    /// How far into its slot an icon is, 0.0 out over the board and 1.0 home.
    ///
    /// `None` for an icon that has always been there, which is the answer for every icon on
    /// every frame but the quarter second after an attack lands.
    pub fn slide(&self, index: usize) -> Option<f64> {
        let (from, elapsed) = self.sliding?;
        (index >= from).then(|| elapsed.as_secs_f64() / SLIDE.as_secs_f64())
    }

    pub fn is_idle(&self) -> bool {
        self.in_flight.is_none() && self.sliding.is_none()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn an_attack_in_the_air_is_not_in_the_tray_yet() {
        let mut tray = TrayAnimation::new();
        tray.expect(2);
        assert_eq!(tray.visible(5), 2, "three of the five are still crossing");
        tray.arrive();
        assert_eq!(tray.visible(5), 5, "and then they are all there");
    }

    #[test]
    fn the_new_icons_slide_and_the_old_ones_do_not() {
        let mut tray = TrayAnimation::new();
        tray.expect(2);
        tray.arrive();
        assert_eq!(tray.slide(1), None, "an icon that was already there");
        assert_eq!(
            tray.slide(2),
            Some(0.0),
            "one that just arrived, out over the board"
        );
        tray.update(SLIDE / 2);
        assert_eq!(tray.slide(2), Some(0.5));
        tray.update(SLIDE);
        assert_eq!(tray.slide(2), None, "and it is home");
        assert!(tray.is_idle());
    }

    /// a tray that never had an attack routed to it draws exactly what the game says
    #[test]
    fn an_untouched_tray_hides_nothing() {
        let tray = TrayAnimation::new();
        assert_eq!(tray.visible(4), 4);
        assert_eq!(tray.slide(0), None);
        assert!(tray.is_idle());
    }

    /// a second ball behind the first does not push the first one's icons back into the air
    #[test]
    fn the_earliest_attack_in_the_air_is_the_one_that_counts() {
        let mut tray = TrayAnimation::new();
        tray.expect(1);
        tray.expect(3);
        assert_eq!(tray.visible(5), 1);
    }
}
