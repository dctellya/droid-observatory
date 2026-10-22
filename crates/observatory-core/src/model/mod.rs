//! Canonical event model (architecture §3). Compiled to wasm for the UI (architecture I9).

mod lane;
mod role;

pub use lane::Lane;
pub use role::Role;
