//! View-models the UI renders as they are; no component computes anything itself (architecture I2).

mod board;
mod library;

pub use board::{BoardView, LaneView, PanelView};
pub use library::{LibraryEntry, MissionRow, library};
