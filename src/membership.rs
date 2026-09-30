//! Product membership settings, resolved through the ordinary sparse catalogue.

use crate::{Bounds, Layer, Result, Rulebook, Setting, SettingKind, SettingType, Snapshot};

/// New-member credential lifetime in whole days (default one).
pub const NEW_CREDENTIAL_DAYS: &str = "membership.new_credential_days";
/// Established-member credential lifetime in whole days (default thirty).
pub const ESTABLISHED_CREDENTIAL_DAYS: &str = "membership.established_credential_days";
/// Probation duration in whole days (default fourteen).
pub const PROBATION_DAYS: &str = "membership.probation_days";

/// Add the cvld membership catalogue without overwriting existing definitions.
/// Members cannot override these community policy settings. The thirty-day
/// credential ceiling bounds revocation exposure; probation fits a day counter.
pub fn define_membership_settings(book: &mut Rulebook) -> Result<()> {
    for (key, default, maximum) in [
        (NEW_CREDENTIAL_DAYS, 1u32, 30u32),
        (ESTABLISHED_CREDENTIAL_DAYS, 30, 30),
        (PROBATION_DAYS, 14, u32::MAX),
    ] {
        if !book.catalog().contains_key(key) {
            book.define(
                key,
                Setting {
                    value_type: SettingType::Integer,
                    nullable: false,
                    default: default.into(),
                    bounds: Bounds {
                        min: Some(1.into()),
                        max: Some(maximum.into()),
                    },
                    lowest_layer: Layer::Community,
                    kind: SettingKind::Technical,
                },
            )?;
        }
    }
    Ok(())
}

/// Membership policy resolved from the same settings publication as gate policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MembershipSettings {
    /// New-member credential days.
    pub new_credential_days: u32,
    /// Established-member credential days.
    pub established_credential_days: u32,
    /// Probation days, committed once at first admission.
    pub probation_days: u32,
}

impl MembershipSettings {
    /// Missing, null, out-of-range or wrongly typed settings fail closed.
    pub fn from_snapshot(snapshot: &Snapshot) -> Result<Self> {
        let days = |key, max| {
            snapshot
                .content
                .get(key)
                .and_then(|v| v.as_u64())
                .filter(|n| *n > 0 && *n <= max)
                .map(|n| n as u32)
                .ok_or(crate::Error::Invalid("membership days"))
        };
        Ok(Self {
            new_credential_days: days(NEW_CREDENTIAL_DAYS, 30)?,
            established_credential_days: days(ESTABLISHED_CREDENTIAL_DAYS, 30)?,
            probation_days: days(PROBATION_DAYS, u32::MAX as u64)?,
        })
    }
}
