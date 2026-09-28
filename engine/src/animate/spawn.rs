use crate::animate::frames::FrameAnimation;
use crate::animate::mascot::MascotMeta;
use crate::game::PieceId;
use sdl2::rect::Point;
use std::f64::consts::PI;
use std::time::Duration;

const ARC_DURATION: f64 = 0.5; // secs
const ARC_HEIGHT_BLOCKS: f64 = 4.5;

/// The next piece is thrown from the mascot into the board along an arc.
#[derive(Clone, Copy, Debug)]
pub struct SpawnArc {
    pub start: Point,
    pub end: Point,
    pub block_size: u32,
}

#[derive(Clone, Debug)]
pub struct State {
    arc: LinearThrowArc,
    piece: PieceId,
    duration: f64,
    is_hold: bool,
    mascot: Option<FrameAnimation>,
}

impl State {
    pub fn throw_position(&self) -> Point {
        let x = self.arc.distance(self.duration);
        let y = self.arc.height(x);
        Point::new(x.round() as i32, y.round() as i32)
    }

    pub fn piece(&self) -> PieceId {
        self.piece
    }

    pub fn mascot_frame(&self) -> Option<usize> {
        self.mascot.map(|m| m.frame())
    }

    /// how far through the throw the queue should have shuffled along, if the piece came
    /// from the queue rather than the hold box
    pub fn peek_offset(&self) -> Option<f64> {
        if self.is_hold {
            None
        } else {
            Some(self.duration / ARC_DURATION)
        }
    }

    pub fn piece_rotate_angle_degrees(&self) -> f64 {
        360.0 * self.duration / ARC_DURATION
    }
}

#[derive(Clone, Debug)]
pub struct SpawnAnimation {
    state: Option<State>,
    arc: Option<LinearThrowArc>,
    mascot: Option<MascotMeta>,
}

impl SpawnAnimation {
    /// without an arc spawning is instant and this animation never runs
    pub fn new(arc: Option<SpawnArc>, mascot: Option<MascotMeta>) -> Self {
        Self {
            state: None,
            arc: arc.map(|arc| LinearThrowArc::new(arc.start, arc.end, arc.block_size)),
            mascot,
        }
    }

    /// returns true when a spawn animation finishes this update
    pub fn update(&mut self, delta: Duration) -> bool {
        let mut finished = false;
        if let Some(animation) = self.state.as_mut() {
            animation.duration += delta.as_secs_f64();
            if animation.duration > ARC_DURATION {
                finished = true
            } else if let Some(mascot) = animation.mascot.as_mut() {
                if mascot.iteration() == 0 {
                    mascot.update(delta);
                }
            }
        }
        if finished {
            self.state = None;
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.state = None;
    }

    /// start the animation; returns false if this theme spawns instantly
    pub fn spawn(&mut self, piece: PieceId, is_hold: bool) -> bool {
        match self.arc {
            Some(arc) => {
                self.state = Some(State {
                    arc,
                    piece,
                    duration: 0.0,
                    is_hold,
                    mascot: self.mascot.map(|m| m.spawn()),
                });
                true
            }
            None => false,
        }
    }

    pub fn state(&self) -> Option<&State> {
        self.state.as_ref()
    }
}

/// this is a linear function plus half a sin wave (i.e. 0 -> pi)
#[derive(Clone, Copy, Debug)]
struct LinearThrowArc {
    x_start: f64,
    x_end: f64,
    magnitude: f64,
    m: f64,
    c: f64,
}

impl LinearThrowArc {
    fn new(start: Point, end: Point, block_size: u32) -> Self {
        let m = (end.y() as f64 - start.y() as f64) / (end.x() as f64 - start.x() as f64);
        let c = start.y() as f64 - m * start.x() as f64;
        Self {
            x_start: start.x() as f64,
            x_end: end.x() as f64,
            magnitude: block_size as f64 * ARC_HEIGHT_BLOCKS,
            m,
            c,
        }
    }

    fn distance(&self, duration: f64) -> f64 {
        self.x_start + (self.x_end - self.x_start) * duration / ARC_DURATION
    }

    fn height(&self, x: f64) -> f64 {
        let linear = self.m * x + self.c;
        let wave = self.magnitude * (PI * (self.x_end - x) / (self.x_end - self.x_start)).sin();
        // subtract the wave, since a lower y is higher on screen
        linear - wave
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_height() {
        let f = LinearThrowArc::new(Point::new(190, 62), Point::new(120, 72), 7);
        assert_eq!(f.height(155.0), 35.5);
    }

    #[test]
    fn arc_distance() {
        let f = LinearThrowArc::new(Point::new(190, 62), Point::new(120, 72), 7);
        assert_eq!(f.distance(ARC_DURATION / 2.0), 155.0);
    }
}
