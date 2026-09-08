//! Super Puzzle Fighter II Turbo's rules for the engine.
//!
//! The ruleset is the PlayStation port's, `SLUS_004.18`, read out of the executable rather
//! than off a strategy guide: [docs/super-puzzle-fighter-rules.md] is the reference every
//! module here cites, and it says which of its findings are certain and which are open.
//! Where we deliberately do something else - the rotation, which is Puyo Puyo's - the rules
//! doc's *Deliberate deviations* says so.
//!
//! [docs/super-puzzle-fighter-rules.md]: https://github.com/ax-h/dr-rustario-vs-rustris/blob/main/docs/super-puzzle-fighter-rules.md

pub mod game;
pub mod options;
pub mod render;
pub mod theme;
