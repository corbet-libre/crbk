use crate::{Error, Result, Values};

/// Decided group defaults, ready to attach to catalogue definitions.
/// This is data only: the caller supplies override ownership and setting kind.
/// Includes the behaviour curve, the public split minimum, the exclusion-fork
/// cost and the fission/fusion suggestion thresholds from the Groups design.
/// `seats.fork_cost` and the points.* names remain deliberately absent.
pub fn group_defaults() -> Result<Values> {
    serde_json::from_str(include_str!("group-defaults.json")).map_err(|_| Error::Corrupt)
}
