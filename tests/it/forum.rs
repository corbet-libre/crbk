//! Forum mechanics are code constants; no rulebook layer can set them.

use crbk::*;
use serde_json::json;

fn snapshot(book: &Rulebook) -> Snapshot {
    Snapshot {
        community: "garden".into(),
        kind: SnapshotKind::Settings,
        revision: 1,
        policy_epoch: 1,
        issued: 0,
        content: book.resolve(&Values::new()).unwrap(),
    }
}

fn book() -> Rulebook {
    let mut book = Rulebook::default();
    define_forum_settings(&mut book).unwrap();
    book
}

#[test]
fn a_new_community_projects_into_forum_settings_without_configuration() {
    let settings = ForumSettings::from_snapshot(&snapshot(&book())).unwrap();
    assert_eq!(settings.heartbeat_ms, FORUM_HEARTBEAT_MS);
    assert_eq!(settings.lease_ms, FORUM_LEASE_MS);
    assert!(settings.heartbeat_ms < settings.lease_ms);
    assert_eq!(
        settings.minimum_client_version,
        semver::Version::parse(FORUM_MINIMUM_CLIENT_VERSION).unwrap()
    );
    assert_eq!(settings.room_passes_per_epoch, FORUM_ROOM_PASSES_PER_EPOCH);
}

#[test]
fn the_quota_covers_a_room_entry_every_84_seconds_all_epoch() {
    assert_eq!(FORUM_ROOM_PASSES_PER_EPOCH, 1024);
    assert!(FORUM_ROOM_PASSES_PER_EPOCH * 85 >= FORUM_EPOCH_SECONDS);
    // It fits the wire type the accounting side freezes it into.
    assert!(u16::try_from(FORUM_ROOM_PASSES_PER_EPOCH).is_ok());
}

#[test]
fn the_mechanics_are_not_settings() {
    let mut book = book();
    for key in [
        "forum.heartbeat_ms",
        "forum.lease_ms",
        "forum.minimum_client_version",
        "forum.room_passes_per_epoch",
    ] {
        assert!(!book.catalog().contains_key(key));
        assert!(book.set_community(key, Some(json!(1))).is_err());
        assert!(
            book.set_platform(
                key,
                Some(PlatformValue {
                    value: json!(1),
                    force: false
                })
            )
            .is_err()
        );
    }
    // A snapshot that carries them anyway cannot change the projection.
    let mut snap = snapshot(&book);
    snap.content.insert("forum.lease_ms".into(), json!(1));
    snap.content
        .insert("forum.room_passes_per_epoch".into(), json!(1));
    let settings = ForumSettings::from_snapshot(&snap).unwrap();
    assert_eq!(settings.lease_ms, FORUM_LEASE_MS);
    assert_eq!(settings.room_passes_per_epoch, FORUM_ROOM_PASSES_PER_EPOCH);
}

#[test]
fn clients_are_accepted_by_semver_precedence() {
    let settings = ForumSettings::from_snapshot(&snapshot(&book())).unwrap();
    for (version, accepted) in [
        ("0.1.9", false),
        ("0.2.0-rc.1", false),
        ("0.2.0", true),
        ("0.2.0+build", true),
        ("0.2.1", true),
        ("1.0.0", true),
    ] {
        assert_eq!(settings.accepts_client(&version.parse().unwrap()), accepted);
    }
}
