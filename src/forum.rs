//! Forum mechanics: code constants, projected as the typed settings Forum consumes.

use crate::{Error, Result, Rulebook, Snapshot};

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
/// Length of the common epoch, in seconds (settlement refuses any other period).
pub const FORUM_EPOCH_SECONDS: u64 = 86_400;
/// Total blind room passes issued to a member in one common epoch, across rooms: an
/// anti-abuse brake honest members never notice, so a code constant. A pass is spent per
/// room entry and a room is kept for far longer than a presence lease, so 1 024 entries
/// in the 86 400 s common epoch (one every 84 s, all day, without ever staying) is beyond
/// any human; only automation hits the brake. 1 024 is also the most the blind-pass issuer
/// supports (`cmbl` MAX_PASSES; cgth refuses anything above), so it is the highest value.
pub const FORUM_ROOM_PASSES_PER_EPOCH: u64 = 1024;

/// Forum defines no rulebook settings: heartbeat, lease, minimum client version and the
/// room-pass quota are code constants. Kept so callers need no change; adds nothing.
pub fn define_forum_settings(_book: &mut Rulebook) -> Result<()> {
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
    /// Every field is a code constant; the snapshot is accepted as the authenticated
    /// settings of a community and nothing in it can change them.
    pub fn from_snapshot(_snapshot: &Snapshot) -> Result<Self> {
        Ok(Self {
            heartbeat_ms: FORUM_HEARTBEAT_MS,
            lease_ms: FORUM_LEASE_MS,
            minimum_client_version: semver::Version::parse(FORUM_MINIMUM_CLIENT_VERSION)
                .map_err(|_| Error::Invalid("forum minimum client version"))?,
            room_passes_per_epoch: FORUM_ROOM_PASSES_PER_EPOCH,
        })
    }

    /// Whether a parsed runtime version meets the published minimum.
    /// Prereleases follow SemVer precedence; build metadata grants no authority.
    pub fn accepts_client(&self, version: &semver::Version) -> bool {
        !version.cmp_precedence(&self.minimum_client_version).is_lt()
    }
}
