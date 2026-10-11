//! Projection of an application-configured record publication period.
//! No setting name, duration or authenticated authority is supplied here.

use crate::{Bounds, Error, Layer, Result, Rulebook, Setting, SettingKind, SettingType, Snapshot};

const MAX_WIRE_INTEGER: u64 = (1 << 53) - 1;

/// Explicit common record-epoch duration from current settings.
/// The caller must authenticate the original snapshot through Policy/Charter.
/// Parsing this projection is neither signature nor record-proof verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordEpochSettings {
    /// Positive Unix-second duration, independent of enrollment checkpoints.
    pub period_seconds: u64,
}

fn configured_key(key: &str) -> Result<()> {
    for part in key.split('/') {
        crate::identifier(part)?;
    }
    if matches!(key.split('/').next(), Some("gate" | "provider" | "action")) {
        return Err(Error::Invalid("record period control key"));
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
            max: Some(MAX_WIRE_INTEGER.into()),
        },
        lowest_layer: Layer::Community,
        kind: SettingKind::Technical,
    }
}

impl RecordEpochSettings {
    /// Define the trusted application's key without choosing a usable default.
    /// An existing definition must match exactly; immutable catalogues are never
    /// replaced, and member/template overrides remain forbidden by Rulebook.
    pub fn define(book: &mut Rulebook, key: &str) -> Result<()> {
        configured_key(key)?;
        if book.catalog().contains_key(key) {
            Self::validate_catalogue(book, key)
        } else {
            book.define(key, definition())
        }
    }

    /// Check the publisher's catalogue before resolving the common snapshot.
    /// Flat signed snapshots carry values, not these definition controls.
    pub fn validate_catalogue(book: &Rulebook, key: &str) -> Result<()> {
        configured_key(key)?;
        if book.catalog().get(key) != Some(&definition()) {
            return Err(Error::Invalid("record period definition"));
        }
        book.validate()
    }

    /// Read the independently configured key from already authenticated settings.
    /// Missing, null, fractional, zero and oversized values refuse. No submitted
    /// member descriptor, alternate key or fallback selects the duration.
    pub fn from_snapshot(snapshot: &Snapshot, key: &str) -> Result<Self> {
        configured_key(key)?;
        let period_seconds = snapshot
            .content
            .get(key)
            .and_then(serde_json::Value::as_u64)
            .filter(|value| (1..=MAX_WIRE_INTEGER).contains(value))
            .ok_or(Error::Invalid("record period missing or invalid"))?;
        Ok(Self { period_seconds })
    }
}
