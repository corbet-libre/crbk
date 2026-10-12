#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod forum;
mod groups;
mod membership;
mod policy;
mod record;
mod settings;
mod settlement;
mod storage;
mod waves;

pub use forum::*;
pub use groups::*;
pub use membership::*;
pub use policy::*;
pub use record::*;
pub use settings::*;
pub use settlement::*;
pub use storage::*;
pub use waves::*;

/// Failures contain no supplied values, subjects, proofs or database details.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A definition, value or policy violates the catalogue.
    #[error("invalid rulebook input: {0}")]
    Invalid(&'static str),
    /// The expected revision is stale.
    #[error("rulebook revision conflict")]
    Conflict,
    /// A revision or template is unavailable.
    #[error("rulebook entry not found")]
    NotFound,
    /// A backend operation failed; reconcile before retrying uncertain writes.
    #[error("rulebook storage failure")]
    Storage,
    /// Stored data is not a valid rulebook.
    #[error("invalid stored rulebook")]
    Corrupt,
}

/// Crate result with redacted errors.
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 200
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
    {
        return Err(Error::Invalid("identifier"));
    }
    Ok(())
}
