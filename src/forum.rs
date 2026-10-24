//! Required Forum settings in the original sparse community catalogue.

use crate::{Bounds, Error, Layer, Result, Rulebook, Setting, SettingKind, SettingType, Snapshot};

/// Interval between member presence heartbeats, in milliseconds. A technical mechanic
/// (it either works or it does not), so a code constant, never a rulebook setting.
/// 15 s keeps presence traffic low while a lost beat is noticed within one lease.
pub const FORUM_HEARTBEAT_MS: u64 = 15_000;
/// Maximum presence lease requested by Forum, in milliseconds: four heartbeats, so it
/// tolerates Tor latency spikes of several seconds and a lost beat. Must not exceed the
/// relay's `maximum_lease_ms` ceiling (checked when the relay loads its configuration).
pub const FORUM_LEASE_MS: u64 = 60_000;
/// Minimum supported cmsg runtime version, as a complete SemVer string: the cmsg
/// release this release requires.
pub const FORUM_MINIMUM_CLIENT_VERSION: &str = "0.2.0";
/// Total blind room passes issued to a member in one common epoch, across rooms.
pub const FORUM_ROOM_PASSES_PER_EPOCH: &str = "forum.room_passes_per_epoch";
/// Default of [`FORUM_ROOM_PASSES_PER_EPOCH`]: sized so an active honest member never hits
/// it within one epoch; a community may lower or raise it.
pub const FORUM_ROOM_PASSES_PER_EPOCH_DEFAULT: u64 = 32;
/// Upper bound of the room-pass quota.
pub const FORUM_ROOM_PASSES_PER_EPOCH_MAX: u64 = 256;

// Exact integer ceiling shared by native and JavaScript wire consumers.
const MAX_WIRE_INTEGER: u64 = (1 << 53) - 1;

/// Add the Forum policy setting (the room-pass quota k) without replacing an existing
/// catalogue. Heartbeat, lease and minimum client version are code constants, not
/// settings. Member and template overrides are forbidden by the ordinary resolver.
pub fn define_forum_settings(book: &mut Rulebook) -> Result<()> {
    if !book.catalog().contains_key(FORUM_ROOM_PASSES_PER_EPOCH) {
        book.define(
            FORUM_ROOM_PASSES_PER_EPOCH,
            Setting {
                value_type: SettingType::Integer,
                nullable: false,
                default: serde_json::Value::from(FORUM_ROOM_PASSES_PER_EPOCH_DEFAULT),
                bounds: Bounds {
                    min: Some(1.into()),
                    max: Some(FORUM_ROOM_PASSES_PER_EPOCH_MAX.into()),
                },
                lowest_layer: Layer::Community,
                kind: SettingKind::Technical,
            },
        )?;
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
    /// Heartbeat, lease and minimum client version are the code constants; only the
    /// room-pass quota is read from the snapshot, which must hold a positive exact wire
    /// integer for it.
    pub fn from_snapshot(snapshot: &Snapshot) -> Result<Self> {
        let room_passes_per_epoch = snapshot
            .content
            .get(FORUM_ROOM_PASSES_PER_EPOCH)
            .and_then(|v| v.as_u64())
            .filter(|n| *n > 0 && *n <= MAX_WIRE_INTEGER)
            .ok_or(Error::Invalid("forum positive integer"))?;
        Ok(Self {
            heartbeat_ms: FORUM_HEARTBEAT_MS,
            lease_ms: FORUM_LEASE_MS,
            minimum_client_version: semver::Version::parse(FORUM_MINIMUM_CLIENT_VERSION)
                .map_err(|_| Error::Invalid("forum minimum client version"))?,
            room_passes_per_epoch,
        })
    }

    /// Whether a parsed runtime version meets the published minimum.
    /// Prereleases follow SemVer precedence; build metadata grants no authority.
    pub fn accepts_client(&self, version: &semver::Version) -> bool {
        !version.cmp_precedence(&self.minimum_client_version).is_lt()
    }
}
