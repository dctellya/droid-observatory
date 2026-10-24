//! The hosted lane (architecture §4.5, D-033): `droid exec --mission --input-format
//! stream-jsonrpc --output-format stream-jsonrpc`. Blocking IO only; the shell owns the
//! process and its pipes and hands the lines to the spool.
//!
//! - `Spool` appends stdout as printed; `RpcSource` tails it with `SourcePtr::Rpc` pointers.
//! - `RpcMapper` maps the lines through mapping v1 to the events of the session file.
//! - `ControlChannel` writes pause, message and the answers to stdin and matches responses.

mod control;
mod envelope;
mod mapper;
mod open;
mod protocol;
mod source;
mod spool;

pub use control::{Ack, AckOutcome, ControlChannel, ControlError, Sent};
pub use mapper::RpcMapper;
pub use open::{
    AskedQuestion, OpenPermission, OpenQuestion, OpenRequests, Prompt, WaitingPermission,
};
pub use protocol::{
    FACTORY_API_VERSION, HostControl, RESUME_TEXT, RpcRequest, RpcResponse, encode,
    encode_initialize, method,
};
pub use source::RpcSource;
pub use spool::{Spool, spool_path};
