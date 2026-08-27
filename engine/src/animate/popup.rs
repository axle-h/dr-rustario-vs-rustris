//! Short captions drawn over the board, where the thing they are about happened.
//!
//! This is not the background field's `Spell` - that writes a word across the whole window in
//! particles, and it is for the once-a-match moments. A popup is small, local and frequent: it
//! sits on the cells that just went and is gone again in under a second. Puyo Rusto's chain
//! count is what it was built for, since a chain announcing itself step by step *is* the
//! game's feedback, but nothing here knows that - a game says what to say through
//! [`crate::render::GameRender::clear_popup`] and says nothing by default.
//!
//! It never blocks the tick. A popup is decoration and the board carries on underneath it.

use crate::game::{CellId, PlacedCell};
use std::collections::HashMap;
use std::time::Duration;

/// how long a popup lives; a little longer than a chain step, so a chain leaves a trail of
/// them climbing the board rather than replacing one with the next
pub const POPUP_DURATION: Duration = Duration::from_millis(750);

/// how far it drifts upwards over its life, in cells
pub const POPUP_RISE_CELLS: f64 = 1.4;

/// the fraction of its life it spends growing to full size
const GROW: f64 = 0.14;
/// the fraction of its life it spends shrinking away again
const SHRINK: f64 = 0.3;
/// how far past full size the pop overshoots before settling
const OVERSHOOT: f64 = 0.18;

#[derive(Clone, Debug, PartialEq)]
pub struct Popup {
    text: String,
    /// the board cell it is centred on, fractional because it is the middle of a group
    column: f64,
    row: f64,
    /// the commonest of the cells it is about, so a theme can draw the caption in the colour
    /// it draws that cell
    cell: Option<CellId>,
    elapsed: Duration,
}

impl Popup {
    pub fn text(&self) -> &str {
        &self.text
    }

    /// the board cell it is centred on, before it has risen
    pub fn at(&self) -> (f64, f64) {
        (self.column, self.row)
    }

    /// The cell the caption is about, whichever of them there were most of.
    ///
    /// Not an average: a Puyo group that took a nuisance puyo with it would average to a
    /// washed out version of its own colour, where the modal cell is the colour that actually
    /// popped.
    pub fn cell(&self) -> Option<CellId> {
        self.cell
    }

    /// how far through its life it is, 0 to 1
    pub fn progress(&self) -> f64 {
        (self.elapsed.as_secs_f64() / POPUP_DURATION.as_secs_f64()).clamp(0.0, 1.0)
    }

    /// how far it has drifted up from its cell, in cells
    pub fn rise(&self) -> f64 {
        self.progress() * POPUP_RISE_CELLS
    }

    /// How big it is drawn, as a fraction of its full size.
    ///
    /// It pops in past full size and settles back, then shrinks away at the end - which is how
    /// it disappears, since the font's texture is shared and cannot be faded per popup.
    pub fn scale(&self) -> f64 {
        let progress = self.progress();
        if progress < GROW {
            let t = progress / GROW;
            t * (1.0 + OVERSHOOT)
        } else if progress > 1.0 - SHRINK {
            (1.0 - progress) / SHRINK
        } else {
            let t = ((progress - GROW) / GROW).min(1.0);
            1.0 + OVERSHOOT * (1.0 - t)
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct PopupAnimation {
    popups: Vec<Popup>,
}

impl PopupAnimation {
    pub fn new() -> Self {
        Self::default()
    }

    /// say `text` over the middle of `cells`
    pub fn add(&mut self, text: String, cells: &[PlacedCell]) {
        if cells.is_empty() {
            return;
        }
        let column = cells.iter().map(|(p, _)| p.x as f64).sum::<f64>() / cells.len() as f64;
        let row = cells.iter().map(|(p, _)| p.y as f64).sum::<f64>() / cells.len() as f64;
        let mut counts: HashMap<CellId, usize> = HashMap::new();
        for (_, id) in cells {
            *counts.entry(*id).or_default() += 1;
        }
        // ties break on the cell id, so the same group always gives the same answer
        let cell = counts
            .into_iter()
            .max_by_key(|(id, count)| (*count, id.0))
            .map(|(id, _)| id);
        self.popups.push(Popup {
            text,
            column,
            row,
            cell,
            elapsed: Duration::ZERO,
        });
    }

    pub fn update(&mut self, delta: Duration) {
        for popup in self.popups.iter_mut() {
            popup.elapsed += delta;
        }
        self.popups.retain(|popup| popup.elapsed < POPUP_DURATION);
    }

    pub fn reset(&mut self) {
        self.popups.clear();
    }

    pub fn active(&self) -> &[Popup] {
        &self.popups
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::geometry::Point;
    use crate::game::CellId;

    fn cells(points: &[(i32, i32)]) -> Vec<PlacedCell> {
        points
            .iter()
            .map(|(x, y)| (Point::new(*x, *y), CellId(0)))
            .collect()
    }

    fn colored(points: &[(i32, i32, u16)]) -> Vec<PlacedCell> {
        points
            .iter()
            .map(|(x, y, id)| (Point::new(*x, *y), CellId(*id)))
            .collect()
    }

    /// the caption is drawn in the colour of what popped, and a group that took a nuisance
    /// puyo with it is still that group's colour
    #[test]
    fn a_popup_is_about_the_cell_there_were_most_of() {
        let mut popups = PopupAnimation::new();
        popups.add(
            "1 chain".to_string(),
            &colored(&[(0, 0, 7), (1, 0, 7), (2, 0, 7), (3, 0, 1)]),
        );
        assert_eq!(popups.active()[0].cell(), Some(CellId(7)));
    }

    #[test]
    fn a_popup_sits_over_the_middle_of_what_it_is_about() {
        let mut popups = PopupAnimation::new();
        popups.add(
            "1 chain".to_string(),
            &cells(&[(2, 5), (3, 5), (2, 7), (3, 7)]),
        );
        assert_eq!(popups.active()[0].at(), (2.5, 6.0));
        assert_eq!(popups.active()[0].text(), "1 chain");
    }

    /// a chain fires one step at a time, so the popups stack up rather than replacing each
    /// other - and each one rises away on its own clock
    #[test]
    fn every_step_of_a_chain_gets_its_own_popup() {
        let mut popups = PopupAnimation::new();
        popups.add("1 chain".to_string(), &cells(&[(0, 0)]));
        popups.update(POPUP_DURATION / 2);
        popups.add("2 chain".to_string(), &cells(&[(1, 0)]));
        assert_eq!(popups.active().len(), 2);
        assert!(popups.active()[0].rise() > popups.active()[1].rise());

        // the first is gone once its life is up, the second is still climbing
        popups.update(POPUP_DURATION / 2 + Duration::from_millis(1));
        assert_eq!(popups.active().len(), 1);
        assert_eq!(popups.active()[0].text(), "2 chain");
    }

    #[test]
    fn a_popup_pops_in_holds_and_shrinks_away() {
        let mut popups = PopupAnimation::new();
        popups.add("3 chain".to_string(), &cells(&[(0, 0)]));
        assert_eq!(popups.active()[0].scale(), 0.0);

        // it overshoots full size on the way in ...
        popups.update(POPUP_DURATION.mul_f64(GROW));
        assert!(popups.active()[0].scale() > 1.0);

        // ... settles ...
        popups.update(POPUP_DURATION.mul_f64(GROW));
        assert!((popups.active()[0].scale() - 1.0).abs() < 1e-9);

        // ... and is drawn as nothing by the time it goes
        popups.update(POPUP_DURATION.mul_f64(1.0 - 2.0 * GROW) - Duration::from_millis(1));
        assert!(popups.active()[0].scale() < 0.02);
    }

    #[test]
    fn a_popup_over_nothing_is_not_drawn_at_all() {
        let mut popups = PopupAnimation::new();
        popups.add("nowhere".to_string(), &[]);
        assert!(popups.active().is_empty());
    }

    #[test]
    fn resetting_forgets_them_all() {
        let mut popups = PopupAnimation::new();
        popups.add("1 chain".to_string(), &cells(&[(0, 0)]));
        popups.reset();
        assert!(popups.active().is_empty());
    }
}
