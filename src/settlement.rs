//! Configured common outcome-token allowance. No key name or usable quota
//! default is chosen here, and parsing supplies no authenticated authority.
use crate::{Bounds, Error, Layer, Result, Rulebook, Setting, SettingKind, SettingType, Snapshot};

// The original maintained blind issuer's bounded per-member epoch allowance.
const MAX_OUTCOME_PASSES: u16 = 1024;

/// Common per-member allowance from the independently authenticated settings.
/// This is a rate limit, not evidence that a contact has resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutcomeQuotaSettings {
    pub quota: u16,
}

fn configured_key(key: &str) -> Result<()> {
    for part in key.split('/') {
        crate::identifier(part)?;
    }
    if matches!(key.split('/').next(), Some("gate" | "provider" | "action")) {
        return Err(Error::Invalid("outcome quota control key"));
    }
    Ok(())
}
fn definition() -> Setting {
    Setting {
        value_type: SettingType::Integer,
        nullable: true,
        default: serde_json::Value::Null,
        bounds: Bounds {
            min: Some(1.into()),
            max: Some(MAX_OUTCOME_PASSES.into()),
        },
        lowest_layer: Layer::Community,
        kind: SettingKind::Technical,
    }
}
impl OutcomeQuotaSettings {
    /// Define only the application's configured key. Existing immutable
    /// definitions must match exactly; member/template overrides are forbidden.
    pub fn define(book: &mut Rulebook, key: &str) -> Result<()> {
        configured_key(key)?;
        if book.catalog().contains_key(key) {
            Self::validate_catalogue(book, key)
        } else {
            book.define(key, definition())
        }
    }
    /// Check the actual publisher catalogue before resolving its common values.
    /// Flat signed snapshots do not contain these definition controls.
    pub fn validate_catalogue(book: &Rulebook, key: &str) -> Result<()> {
        configured_key(key)?;
        if book.catalog().get(key) != Some(&definition()) {
            return Err(Error::Invalid("outcome quota definition"));
        }
        book.validate()
    }
    /// Read the exact expected key from already authenticated current settings.
    /// Missing/null, fractional, zero, negative and oversized values refuse.
    pub fn from_snapshot(snapshot: &Snapshot, key: &str) -> Result<Self> {
        configured_key(key)?;
        let quota = snapshot
            .content
            .get(key)
            .and_then(serde_json::Value::as_u64)
            .filter(|value| (1..=u64::from(MAX_OUTCOME_PASSES)).contains(value))
            .ok_or(Error::Invalid("outcome quota missing or invalid"))?;
        Ok(Self {
            quota: quota as u16,
        })
    }
}
