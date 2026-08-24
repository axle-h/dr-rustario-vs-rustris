//! The app's screens: each one a state machine ticked once per frame by the launcher,
//! replacing the old blocking loops so a browser main loop can drive them.

mod high_scores;
mod match_screen;
mod menu;
mod name_entry;

pub use high_scores::HighScoreViewScreen;
pub use match_screen::MatchScreen;
pub use menu::MenuScreen;
pub use name_entry::{NameEntryExit, NameEntryScreen};
