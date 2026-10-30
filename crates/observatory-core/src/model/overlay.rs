use serde::{Deserialize, Serialize};

/// A marker over a phase for events a person or the officer caused. Never a lane.
///
/// `Officer` is declared with no producer in v1: no capture holds an officer run. Until the end
/// of M3 the word `commander` is still read on intake (a store or a capture written before
/// D-035) and mapped to `Officer`; only `officer` is ever written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Overlay {
    Human,
    #[serde(alias = "commander")]
    Officer,
}

impl Overlay {
    /// The wire and store text (`human`, `officer`).
    pub fn as_str(self) -> &'static str {
        match self {
            Overlay::Human => "human",
            Overlay::Officer => "officer",
        }
    }

    /// Reads the wire or store text, accepting the legacy `commander` for `Officer`.
    pub fn parse(text: &str) -> Option<Overlay> {
        match text {
            "human" => Some(Overlay::Human),
            "officer" | "commander" => Some(Overlay::Officer),
            _ => None,
        }
    }
}
