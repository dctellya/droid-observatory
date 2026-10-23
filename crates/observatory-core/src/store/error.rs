use std::path::PathBuf;

use crate::ingest::IngestError;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("cannot open the store {}: {source}", path.display())]
    Open {
        path: PathBuf,
        source: rusqlite::Error,
    },
    #[error("cannot create {}: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("store query failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("no event with id {id} in the store")]
    UnknownEvent { id: String },
    #[error("event {id} has a pointer the store cannot read back: {message}")]
    BadRow { id: String, message: String },
    #[error("{} is not valid UTF-8 and cannot be stored as a pointer", path.display())]
    NonUtf8Path { path: PathBuf },
    #[error(transparent)]
    Ingest(#[from] IngestError),
}
