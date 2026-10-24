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

fn configured(k: u64) -> Rulebook {
    let mut book = book();
    // Synthetic fixture value, not a product default.
    book.set_community(FORUM_ROOM_PASSES_PER_EPOCH, Some(json!(k)))
        .unwrap();
    book
}

#[test]
fn the_quota_has_no_default_and_an_unconfigured_one_is_refused_by_name() {
    let error = ForumSettings::from_snapshot(&snapshot(&book())).unwrap_err();
    assert!(error.to_string().contains(FORUM_ROOM_PASSES_PER_EPOCH));
}

#[test]
fn a_configured_community_projects_into_forum_settings() {
    let settings = ForumSettings::from_snapshot(&snapshot(&configured(7))).unwrap();
    assert_eq!(settings.room_passes_per_epoch, 7);
    assert_eq!(settings.heartbeat_ms, FORUM_HEARTBEAT_MS);
    assert_eq!(settings.lease_ms, FORUM_LEASE_MS);
    assert!(settings.heartbeat_ms < settings.lease_ms);
    assert_eq!(
        settings.minimum_client_version,
        semver::Version::parse(FORUM_MINIMUM_CLIENT_VERSION).unwrap()
    );
    let edge = configured((1u64 << 53) - 1);
    assert!(ForumSettings::from_snapshot(&snapshot(&edge)).is_ok());
}

#[test]
fn the_quota_rejects_unusable_values() {
    for bad in [
        json!(0),
        json!(-1),
        json!(1.5),
        json!("3"),
        json!(1u64 << 53),
    ] {
        assert!(
            book()
                .set_community(FORUM_ROOM_PASSES_PER_EPOCH, Some(bad))
                .is_err()
        );
    }
}

#[test]
fn the_mechanics_are_not_settings() {
    let mut book = configured(7);
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
    let settings = ForumSettings::from_snapshot(&snapshot(&configured(7))).unwrap();
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

#[test]
fn an_unusable_quota_in_a_snapshot_is_refused() {
    let valid = snapshot(&configured(7));
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
