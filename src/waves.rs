//! Wave policy settings, resolved through the ordinary sparse catalogue.

use crate::{Bounds, Layer, Result, Rulebook, Setting, SettingKind, SettingType, Snapshot};

/// Days after a Close before the sender's introduction returns (default seven).
pub const WAVE_WAITING_PERIOD_DAYS: &str = "waves.waiting_period_days";

/// Add the wave catalogue without overwriting an existing definition. The
/// community administrators set it; members cannot override it.
pub fn define_wave_settings(book: &mut Rulebook) -> Result<()> {
    if !book.catalog().contains_key(WAVE_WAITING_PERIOD_DAYS) {
        book.define(
            WAVE_WAITING_PERIOD_DAYS,
            Setting {
                value_type: SettingType::Integer,
                nullable: false,
                default: 7.into(),
                bounds: Bounds {
                    min: Some(1.into()),
                    max: Some(30.into()),
                },
                lowest_layer: Layer::Community,
                kind: SettingKind::Technical,
            },
        )?;
    }
    Ok(())
}

/// Wave policy read at runtime from the verified settings publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaveSettings {
    /// Waiting period in whole days, one to thirty.
    pub waiting_period_days: u32,
}

impl WaveSettings {
    /// Missing, null, out-of-range or wrongly typed settings fail closed.
    pub fn from_snapshot(snapshot: &Snapshot) -> Result<Self> {
        snapshot
            .content
            .get(WAVE_WAITING_PERIOD_DAYS)
            .and_then(|v| v.as_u64())
            .filter(|n| (1..=30).contains(n))
            .map(|n| Self {
                waiting_period_days: n as u32,
            })
            .ok_or(crate::Error::Invalid("wave waiting period"))
    }
    /// The waiting period in seconds.
    pub fn waiting_period_seconds(&self) -> u64 {
        u64::from(self.waiting_period_days) * 86_400
    }
}
