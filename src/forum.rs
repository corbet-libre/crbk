//! Required Forum settings in the original sparse community catalogue.

use crate::{Bounds, Error, Layer, Result, Rulebook, Setting, SettingKind, SettingType, Snapshot};

/// Interval between member presence heartbeats, in milliseconds.
pub const FORUM_HEARTBEAT_MS: &str = "forum.heartbeat_ms";
/// Maximum presence lease requested by Forum, in milliseconds.
pub const FORUM_LEASE_MS: &str = "forum.lease_ms";
/// Minimum supported cmsg runtime version, as a complete SemVer string.
pub const FORUM_MINIMUM_CLIENT_VERSION: &str = "forum.minimum_client_version";
/// Total blind room passes issued to a member in one common epoch, across rooms.
pub const FORUM_ROOM_PASSES_PER_EPOCH: &str = "forum.room_passes_per_epoch";

// Exact integer ceiling shared by native and JavaScript wire consumers.
const MAX_WIRE_INTEGER: u64 = (1 << 53) - 1;

/// Add required Forum settings without replacing an existing catalogue.
/// Null means unconfigured, never a usable product default. Platform/community
/// owners must supply explicit values before Forum can start. Member overrides
/// and template overrides are forbidden by the ordinary resolver.
pub fn define_forum_settings(book: &mut Rulebook) -> Result<()> {
    for key in [
        FORUM_HEARTBEAT_MS,
        FORUM_LEASE_MS,
        FORUM_ROOM_PASSES_PER_EPOCH,
        FORUM_MINIMUM_CLIENT_VERSION,
    ] {
        if !book.catalog().contains_key(key) {
            let numeric = key != FORUM_MINIMUM_CLIENT_VERSION;
            book.define(
                key,
                Setting {
                    value_type: if numeric {
                        SettingType::Integer
                    } else {
                        SettingType::String
                    },
                    nullable: true,
                    default: serde_json::Value::Null,
                    bounds: if numeric {
                        Bounds {
                            min: Some(1.into()),
                            max: Some(MAX_WIRE_INTEGER.into()),
                        }
                    } else {
                        Bounds::default()
                    },
                    lowest_layer: Layer::Community,
                    kind: SettingKind::Technical,
                },
            )?;
        }
    }
    Ok(())
}

/// Typed settings projected from an authenticated settings snapshot.
/// This projection does not authenticate a snapshot. Use original Charter or
/// Policy verification first and retain its scope, epoch and time authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForumSettings {
    /// Positive heartbeat interval, strictly shorter than the lease.
    pub heartbeat_ms: u64,
    /// Positive lease duration. Forum additionally applies its storage TTL,
    /// absolute clock bounds and current credential/authority expiry.
    pub lease_ms: u64,
    /// Required runtime version; compare SemVer precedence, ignoring build data.
    pub minimum_client_version: semver::Version,
    /// Positive TOTAL quota per member/common epoch; never a per-room quota.
    pub room_passes_per_epoch: u64,
}

impl ForumSettings {
    /// Refuse absent, unconfigured, wrongly typed, out-of-range or inconsistent
    /// settings. There are no fallback timing, version or quota values.
    pub fn from_snapshot(snapshot: &Snapshot) -> Result<Self> {
        let positive = |key| {
            snapshot
                .content
                .get(key)
                .and_then(|v| v.as_u64())
                .filter(|n| *n > 0 && *n <= MAX_WIRE_INTEGER)
                .ok_or(Error::Invalid("forum positive integer"))
        };
        let heartbeat_ms = positive(FORUM_HEARTBEAT_MS)?;
        let lease_ms = positive(FORUM_LEASE_MS)?;
        if heartbeat_ms >= lease_ms {
            return Err(Error::Invalid("forum heartbeat must precede lease"));
        }
        let minimum_client_version = snapshot
            .content
            .get(FORUM_MINIMUM_CLIENT_VERSION)
            .and_then(|v| v.as_str())
            .and_then(|v| semver::Version::parse(v).ok())
            .ok_or(Error::Invalid("forum minimum client version"))?;
        Ok(Self {
            heartbeat_ms,
            lease_ms,
            minimum_client_version,
            room_passes_per_epoch: positive(FORUM_ROOM_PASSES_PER_EPOCH)?,
        })
    }

    /// Whether a parsed runtime version meets the published minimum.
    /// Prereleases follow SemVer precedence; build metadata grants no authority.
    pub fn accepts_client(&self, version: &semver::Version) -> bool {
        !version.cmp_precedence(&self.minimum_client_version).is_lt()
    }
}
