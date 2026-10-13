//! The standing and voucher parameters a resolved rulebook yields. The catalogue
//! owns every number; `czkp` only turns them into the tables the circuits bind,
//! so a publisher and a device that read the same verified rulebook compute the
//! same policy digest.
use crate::{Error, Result, Rulebook, Snapshot, Values};
use czkp::standing::{Params, Setting};
use serde_json::Value;

fn setting(value: &Value) -> Option<Setting> {
    match value {
        Value::Bool(flag) => Some(Setting::Flag(*flag)),
        Value::Number(number) => number
            .as_u64()
            .map(Setting::Integer)
            .or_else(|| number.as_f64().map(Setting::Number)),
        _ => None,
    }
}

/// The parameters of whole resolved values (a verified snapshot's content).
pub fn standing_params(values: &Values) -> Result<Params> {
    Params::from_rulebook(|key| values.get(key).and_then(setting))
        .map_err(|_| Error::Invalid("standing parameters"))
}

impl Snapshot {
    /// The standing and voucher parameters this verified snapshot publishes.
    pub fn standing_params(&self) -> Result<Params> {
        standing_params(&self.content)
    }
}

impl Rulebook {
    /// The parameters the rulebook resolves to without any member latitude.
    pub fn standing_params(&self) -> Result<Params> {
        standing_params(&self.resolve(&Default::default())?)
    }
}
