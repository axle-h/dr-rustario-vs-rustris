//! The ball an attack crosses the window as.
//!
//! Mean Bean Machine draws one of these for every attack sent: a ball in the popped group's
//! own colour appears over the group, its core flashes white, and it arcs up and across the
//! whole screen to just above the top of the opponent's board, where it bursts and leaves a
//! refugee bean in their tray.
//!
//! It is the one thing here that belongs to **no player**, which is why it lives on
//! [`crate::render::context::ThemeContext`] rather than on a `PlayerAnimations`: every
//! offset a player owns is applied inside that player's own panel, and this crosses between
//! two of them.
//!
//! A flight is held in **cells and player numbers, never pixels**, and resolved through
//! whichever theme is on when it is drawn - so a theme change mid-flight moves both ends
//! rather than leaving the ball flying to where the board used to be. It is decoration: it
//! holds nothing, and a single player match routes no attacks and so never sees one.

use crate::game::CellId;
use std::f64::consts::TAU;
use std::time::Duration;

/// how long a ball takes to cross, whatever it is crossing
const FLIGHT: Duration = Duration::from_millis(350);

/// how high above the straight line between the two ends the arc is thrown, as a fraction of
/// the window height
const ARC: f64 = 0.35;

/// the fraction of the flight the ball spends growing to its full size
const GROW: f64 = 0.25;

/// how big the ball starts, as a fraction of its full size
const START_SCALE: f64 = 0.3;

/// how many times the core flares white while the ball is forming
const CORE_FLARES: f64 = 3.0;

/// One attack on its way across.
#[derive(Clone, Copy, Debug)]
pub struct Flight {
    pub from_player: u32,
    /// where it left, in the sender's own board cells
    pub from_cell: (f64, f64),
    pub to_player: u32,
    /// the popped group's own colour, which is what a theme with no ball art of its own
    /// draws instead
    pub cell: CellId,
    /// how big the attack is, in the receiver's own units - a theme whose ball comes in two
    /// sizes picks between them on this
    pub strength: u32,
    elapsed: Duration,
}

impl Flight {
    /// how far along it is, 0..=1
    pub fn progress(&self) -> f64 {
        (self.elapsed.as_secs_f64() / FLIGHT.as_secs_f64()).clamp(0.0, 1.0)
    }

    /// how big to draw it, as a fraction of a block: it swells out of the group it came from
    pub fn scale(&self) -> f64 {
        let through = (self.progress() / GROW).min(1.0);
        START_SCALE + (1.0 - START_SCALE) * through
    }

    /// How bright the white core still is, 0..=1.
    ///
    /// It **strobes** rather than fading: in the original the ball alternates between mostly
    /// its own colour and a white blowout about three times as it forms, and is its own
    /// colour by the time it is properly under way.
    pub fn core(&self) -> f64 {
        let left = (1.0 - self.progress() / GROW).clamp(0.0, 1.0);
        let phase = (self.progress() / GROW).min(1.0) * CORE_FLARES * TAU;
        left * (0.5 - 0.5 * phase.cos())
    }

    /// where it is, given the two ends in window pixels and how tall the window is
    ///
    /// A quadratic bezier with its control point lifted above the midpoint, so the ball
    /// leaves the board upward rather than sliding sideways across it.
    pub fn at(&self, from: (f64, f64), to: (f64, f64), window_height: u32) -> (f64, f64) {
        let t = self.progress();
        let lift = window_height as f64 * ARC;
        let control = ((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0 - lift);
        let inverse = 1.0 - t;
        (
            inverse * inverse * from.0 + 2.0 * inverse * t * control.0 + t * t * to.0,
            inverse * inverse * from.1 + 2.0 * inverse * t * control.1 + t * t * to.1,
        )
    }
}

/// Every attack in the air, for the whole window.
#[derive(Clone, Debug, Default)]
pub struct AttackBallAnimation {
    flights: Vec<Flight>,
    /// the ones that arrived this frame, for whoever bursts and fills a tray
    arrived: Vec<Flight>,
}

impl AttackBallAnimation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn send(
        &mut self,
        from_player: u32,
        from_cell: (f64, f64),
        to_player: u32,
        cell: CellId,
        strength: u32,
    ) {
        self.flights.push(Flight {
            from_player,
            from_cell,
            to_player,
            cell,
            strength,
            elapsed: Duration::ZERO,
        });
    }

    pub fn update(&mut self, delta: Duration) {
        self.arrived.clear();
        if self.flights.is_empty() {
            return;
        }
        for flight in self.flights.iter_mut() {
            flight.elapsed += delta;
        }
        let (arrived, flying) = self
            .flights
            .drain(..)
            .partition::<Vec<_>, _>(|f| f.elapsed >= FLIGHT);
        self.arrived = arrived;
        self.flights = flying;
    }

    pub fn reset(&mut self) {
        self.flights.clear();
        self.arrived.clear();
    }

    pub fn flights(&self) -> &[Flight] {
        &self.flights
    }

    /// the balls that landed this frame; each is reported exactly once
    pub fn arrived(&self) -> &[Flight] {
        &self.arrived
    }

    pub fn is_empty(&self) -> bool {
        self.flights.is_empty()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn sent() -> AttackBallAnimation {
        let mut balls = AttackBallAnimation::new();
        balls.send(0, (2.0, 6.0), 1, CellId(3), 12);
        balls
    }

    #[test]
    fn a_ball_arrives_exactly_once_and_then_is_gone() {
        let mut balls = sent();
        balls.update(FLIGHT / 2);
        assert_eq!(balls.flights().len(), 1);
        assert!(balls.arrived().is_empty());
        balls.update(FLIGHT);
        assert_eq!(balls.arrived().len(), 1, "it landed");
        assert!(balls.flights().is_empty());
        balls.update(Duration::from_millis(16));
        assert!(balls.arrived().is_empty(), "and only says so once");
    }

    /// the arc leaves the board upward rather than sliding across it, so the ball is above
    /// the straight line between the two ends the whole way
    #[test]
    fn a_ball_arcs_over_the_line_between_the_two_boards() {
        let mut balls = sent();
        balls.update(FLIGHT / 2);
        let flight = balls.flights()[0];
        let (_, y) = flight.at((100.0, 500.0), (900.0, 500.0), 1000);
        assert!(y < 500.0 - 100.0, "well above the two ends: {y}");
    }

    #[test]
    fn a_ball_swells_out_of_the_group_and_its_core_flares_and_goes() {
        let mut balls = sent();
        assert!(balls.flights()[0].scale() < 0.5, "it starts small");
        balls.update(FLIGHT / 16);
        assert!(
            balls.flights()[0].core() > 0.0,
            "the core flares as it forms"
        );
        balls.update(FLIGHT / 2);
        assert_eq!(balls.flights()[0].scale(), 1.0, "and is a whole ball");
        assert_eq!(balls.flights()[0].core(), 0.0, "the core long gone");
    }
}
