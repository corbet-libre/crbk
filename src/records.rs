//! Common record-frontier epoch duration in the original signed catalogue.

use crate::{Bounds, Error, Layer, Result, Rulebook, Setting, SettingKind, SettingType, Snapshot};

/// Community-wide frontier publication interval, in whole seconds.
pub const RECORD_EPOCH_SECONDS: &str = "records.epoch_seconds";
/// Decided product default: one whole community snapshot every twenty-four hours.
pub const DEFAULT_RECORD_EPOCH_SECONDS: u64 = 24 * 60 * 60;
const MAX_WIRE_INTEGER: u64 = (1 << 53) - 1;

/// Define the original community setting without replacing existing definitions.
/// A member or template cannot select a different epoch or freshness allowance.
pub fn define_record_settings(book: &mut Rulebook) -> Result<()> {
    if !book.catalog().contains_key(RECORD_EPOCH_SECONDS) {
        book.define(
            RECORD_EPOCH_SECONDS,
            Setting {
                value_type: SettingType::Integer,
                nullable: false,
                default: DEFAULT_RECORD_EPOCH_SECONDS.into(),
                bounds: Bounds {
                    min: Some(1.into()),
                    max: Some(MAX_WIRE_INTEGER.into()),
                },
                lowest_layer: Layer::Community,
                kind: SettingKind::Technical,
            },
        )?;
    }
    Ok(())
}

/// Values from the authenticated common settings publication. This projection
/// neither verifies the publication nor establishes which frontier is newest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordSettings {
    /// Whole seconds between common epochs. The original frontier owner applies
    /// its clock bounds, checked arithmetic and signed epoch schedule.
    pub epoch_seconds: u64,
}

impl RecordSettings {
    /// Refuse missing or malformed signed values; defaults are materialized by
    /// the Rulebook resolver, never silently substituted at the relying party.
    pub fn from_snapshot(snapshot: &Snapshot) -> Result<Self> {
        let epoch_seconds = snapshot
            .content
            .get(RECORD_EPOCH_SECONDS)
            .and_then(|value| value.as_u64())
            .filter(|seconds| *seconds > 0 && *seconds <= MAX_WIRE_INTEGER)
            .ok_or(Error::Invalid("record epoch seconds"))?;
        Ok(Self { epoch_seconds })
    }
}
