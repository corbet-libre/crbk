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

fn configured() -> Rulebook {
    let mut book = Rulebook::default();
    define_forum_settings(&mut book).unwrap();
    // Synthetic fixture values, not production defaults.
    for (key, value) in [
        (FORUM_HEARTBEAT_MS, json!(3000)),
        (FORUM_LEASE_MS, json!(10000)),
        (FORUM_MINIMUM_CLIENT_VERSION, json!("1.2.0+owner")),
        (FORUM_ROOM_PASSES_PER_EPOCH, json!(7)),
    ] {
        book.set_community(key, Some(value)).unwrap();
    }
    book
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn explicit_configuration_resolves_with_original_layer_authority() {
    let mut book = Rulebook::default();
    assert!(ForumSettings::from_snapshot(&snapshot(&book)).is_err());
    define_forum_settings(&mut book).unwrap();
    let original = book.clone();
    define_forum_settings(&mut book).unwrap();
    assert_eq!(book, original);
    assert!(ForumSettings::from_snapshot(&snapshot(&book)).is_err());
    book = configured();
    let projection = ForumSettings::from_snapshot(&snapshot(&book)).unwrap();
    assert_eq!(projection.heartbeat_ms, 3000);
    assert_eq!(projection.lease_ms, 10000);
    assert_eq!(projection.room_passes_per_epoch, 7);
    for (version, accepted) in [
        ("1.1.99", false),
        ("1.2.0-rc.1", false),
        ("1.2.0", true),
        ("1.2.0+different", true),
        ("1.2.1", true),
        ("2.0.0", true),
    ] {
        assert_eq!(
            projection.accepts_client(&version.parse().unwrap()),
            accepted
        );
    }
    for key in [
        FORUM_HEARTBEAT_MS,
        FORUM_LEASE_MS,
        FORUM_ROOM_PASSES_PER_EPOCH,
        FORUM_MINIMUM_CLIENT_VERSION,
    ] {
        assert!(
            book.resolve(&Values::from([(key.into(), json!(null))]))
                .is_err()
        );
    }
    book.set_platform(
        FORUM_ROOM_PASSES_PER_EPOCH,
        Some(PlatformValue {
            value: json!(3),
            force: true,
        }),
    )
    .unwrap();
    assert_eq!(
        ForumSettings::from_snapshot(&snapshot(&book))
            .unwrap()
            .room_passes_per_epoch,
        3
    );
    // An explicit null terminates inheritance and disables usable configuration.
    book.set_community(FORUM_LEASE_MS, Some(json!(null)))
        .unwrap();
    assert!(ForumSettings::from_snapshot(&snapshot(&book)).is_err());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn missing_or_inconsistent_signed_values_fail_closed() {
    let valid = snapshot(&configured());
    for key in [
        FORUM_HEARTBEAT_MS,
        FORUM_LEASE_MS,
        FORUM_ROOM_PASSES_PER_EPOCH,
        FORUM_MINIMUM_CLIENT_VERSION,
    ] {
        let mut missing = valid.clone();
        missing.content.remove(key);
        assert!(ForumSettings::from_snapshot(&missing).is_err());
    }
    for key in [
        FORUM_HEARTBEAT_MS,
        FORUM_LEASE_MS,
        FORUM_ROOM_PASSES_PER_EPOCH,
    ] {
        for bad in [
            json!(null),
            json!(0),
            json!(-1),
            json!(1.5),
            json!("3"),
            json!(1u64 << 53),
        ] {
            let mut invalid = valid.clone();
            invalid.content.insert(key.into(), bad.clone());
            assert!(ForumSettings::from_snapshot(&invalid).is_err());
            if !bad.is_null() {
                assert!(configured().set_community(key, Some(bad)).is_err());
            }
        }
    }
    for bad in [
        json!(null),
        json!(1),
        json!("1.2"),
        json!("01.2.0"),
        json!("^1.2.0"),
        json!("1.2.0 "),
    ] {
        let mut invalid = valid.clone();
        invalid
            .content
            .insert(FORUM_MINIMUM_CLIENT_VERSION.into(), bad);
        assert!(ForumSettings::from_snapshot(&invalid).is_err());
    }
    for heartbeat in [10000, 10001] {
        let mut invalid = valid.clone();
        invalid
            .content
            .insert(FORUM_HEARTBEAT_MS.into(), json!(heartbeat));
        assert!(ForumSettings::from_snapshot(&invalid).is_err());
    }
    let mut edge = valid;
    edge.content
        .insert(FORUM_LEASE_MS.into(), json!((1u64 << 53) - 1));
    edge.content
        .insert(FORUM_ROOM_PASSES_PER_EPOCH.into(), json!((1u64 << 53) - 1));
    assert!(ForumSettings::from_snapshot(&edge).is_ok());
}
