//! Real sparse resolution and portable Forum policy projection.

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
    assert_eq!(
        settings.room_passes_per_epoch,
        FORUM_ROOM_PASSES_PER_EPOCH_DEFAULT
    );
}

#[test]
fn the_quota_is_a_community_setting_within_bounds() {
    let mut book = book();
    book.set_community(FORUM_ROOM_PASSES_PER_EPOCH, Some(json!(7)))
        .unwrap();
    let settings = ForumSettings::from_snapshot(&snapshot(&book)).unwrap();
    assert_eq!(settings.room_passes_per_epoch, 7);
    for bad in [
        json!(null),
        json!(0),
        json!(-1),
        json!(1.5),
        json!("3"),
        json!(FORUM_ROOM_PASSES_PER_EPOCH_MAX + 1),
    ] {
        assert!(book
            .set_community(FORUM_ROOM_PASSES_PER_EPOCH, Some(bad))
            .is_err());
    }
    book.set_community(
        FORUM_ROOM_PASSES_PER_EPOCH,
        Some(json!(FORUM_ROOM_PASSES_PER_EPOCH_MAX)),
    )
    .unwrap();
}

#[test]
fn the_mechanics_are_not_settings() {
    let mut book = book();
    for key in [
        "forum.heartbeat_ms",
        "forum.lease_ms",
        "forum.minimum_client_version",
    ] {
        assert!(!book.catalog().contains_key(key));
        assert!(book.set_community(key, Some(json!(1))).is_err());
    }
    // A snapshot that carries them anyway cannot change the projection.
    let mut snap = snapshot(&book);
    snap.content.insert("forum.lease_ms".into(), json!(1));
    snap.content.insert("forum.heartbeat_ms".into(), json!(1));
    let settings = ForumSettings::from_snapshot(&snap).unwrap();
    assert_eq!(settings.lease_ms, FORUM_LEASE_MS);
    assert_eq!(settings.heartbeat_ms, FORUM_HEARTBEAT_MS);
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
        assert_eq!(
            settings.accepts_client(&version.parse().unwrap()),
            accepted
        );
    }
}

#[test]
fn an_unusable_quota_in_a_snapshot_is_refused() {
    let valid = snapshot(&book());
    for bad in [
        json!(null),
        json!(0),
        json!(-1),
        json!(1.5),
        json!("3"),
        json!(1u64 << 53),
    ] {
        let mut invalid = valid.clone();
        invalid
            .content
            .insert(FORUM_ROOM_PASSES_PER_EPOCH.into(), bad);
        assert!(ForumSettings::from_snapshot(&invalid).is_err());
    }
    let mut missing = valid;
    missing.content.remove(FORUM_ROOM_PASSES_PER_EPOCH);
    assert!(ForumSettings::from_snapshot(&missing).is_err());
}
