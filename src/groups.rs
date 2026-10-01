use crate::{Error, Result, Values};

/// Decided group defaults, ready to attach to catalogue definitions.
/// This is data only: the caller supplies override ownership and setting kind.
/// Missing transition vectors, fork-cost key, suggest.* and points.* names are
/// deliberately absent. This partial map is not an executable group policy.
pub fn group_defaults() -> Result<Values> {
    serde_json::from_str(include_str!("group-defaults.json")).map_err(|_| Error::Corrupt)
}
