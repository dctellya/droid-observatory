use serde::{Deserialize, Serialize};

/// A marker over a phase for events a person or the commander caused. Never a lane.
///
/// `Commander` is declared with no producer in v1: no capture holds a commander run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Overlay {
    Human,
    Commander,
}

impl Overlay {
    /// The wire and store text (`human`, `commander`).
    pub fn as_str(self) -> &'static str {
        match self {
            Overlay::Human => "human",
            Overlay::Commander => "commander",
        }
    }
}
