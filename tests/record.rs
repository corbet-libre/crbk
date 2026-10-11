//! Real sparse resolution and persisted configuration for a trusted fixture key.
use crbk::*;
use serde_json::json;

// An application-selected test key and duration, not product defaults.
const KEY: &str = "fixture/record_epoch_seconds";
fn configured() -> Rulebook {
    let mut book = Rulebook::default();
    RecordEpochSettings::define(&mut book, KEY).unwrap();
    book.set_community(KEY, Some(json!(100))).unwrap();
    book
}
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

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn common_period_requires_explicit_configuration_and_original_layer_controls() {
    let mut book = Rulebook::default();
    assert!(RecordEpochSettings::from_snapshot(&snapshot(&book), KEY).is_err());
    assert!(RecordEpochSettings::validate_catalogue(&book, KEY).is_err());
    RecordEpochSettings::define(&mut book, KEY).unwrap();
    let original = book.clone();
    RecordEpochSettings::define(&mut book, KEY).unwrap();
    assert_eq!(book, original);
    assert!(RecordEpochSettings::from_snapshot(&snapshot(&book), KEY).is_err());
    book = configured();
    RecordEpochSettings::validate_catalogue(&book, KEY).unwrap();
    assert_eq!(
        RecordEpochSettings::from_snapshot(&snapshot(&book), KEY)
            .unwrap()
            .period_seconds,
        100
    );
    assert!(
        book.resolve(&Values::from([(KEY.into(), json!(200))]))
            .is_err()
    );
    assert!(
        book.publish_template(
            TemplatePin {
                name: "fixture".into(),
                version: 1
            },
            Values::from([(KEY.into(), json!(200))])
        )
        .is_err()
    );
    book.set_platform(
        KEY,
        Some(PlatformValue {
            value: json!(50),
            force: true,
        }),
    )
    .unwrap();
    assert_eq!(
        RecordEpochSettings::from_snapshot(&snapshot(&book), KEY)
            .unwrap()
            .period_seconds,
        50
    );
    book.set_platform(
        KEY,
        Some(PlatformValue {
            value: json!(50),
            force: false,
        }),
    )
    .unwrap();
    book.set_community(KEY, Some(json!(null))).unwrap();
    assert!(RecordEpochSettings::from_snapshot(&snapshot(&book), KEY).is_err());
    book.set_community(KEY, None).unwrap();
    assert_eq!(
        RecordEpochSettings::from_snapshot(&snapshot(&book), KEY)
            .unwrap()
            .period_seconds,
        50
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn configured_key_and_exact_integer_encoding_never_fall_back() {
    let mut book = configured();
    let original = snapshot(&book);
    assert!(RecordEpochSettings::from_snapshot(&original, "different").is_err());
    for key in [
        "",
        "a/",
        "action/admit",
        "gate/community/a",
        "provider/community/a/b",
    ] {
        assert!(RecordEpochSettings::define(&mut book, key).is_err());
        assert!(RecordEpochSettings::from_snapshot(&original, key).is_err());
    }
    for value in [
        json!(null),
        json!(0),
        json!(-1),
        json!(1.5),
        json!("100"),
        json!(1u64 << 53),
    ] {
        let mut changed = original.clone();
        changed.content.insert(KEY.into(), value.clone());
        assert!(RecordEpochSettings::from_snapshot(&changed, KEY).is_err());
        if !value.is_null() {
            assert!(book.set_community(KEY, Some(value)).is_err());
        }
    }
    for period in [1, (1u64 << 53) - 1] {
        book.set_community(KEY, Some(json!(period))).unwrap();
        assert_eq!(
            RecordEpochSettings::from_snapshot(&snapshot(&book), KEY)
                .unwrap()
                .period_seconds,
            period
        );
    }
    for case in 0..6 {
        let mut definition = configured().catalog()[KEY].clone();
        match case {
            0 => definition.default = json!(100),
            1 => definition.lowest_layer = Layer::Member,
            2 => definition.kind = SettingKind::Template,
            3 => definition.bounds.max = Some(1000.into()),
            4 => definition.nullable = false,
            _ => {
                definition.value_type = SettingType::Number;
            }
        }
        let mut incompatible = Rulebook::default();
        if case == 4 {
            definition.default = json!(100);
        }
        incompatible.define(KEY, definition).unwrap();
        let before = incompatible.clone();
        assert!(RecordEpochSettings::define(&mut incompatible, KEY).is_err());
        assert_eq!(incompatible, before);
    }
}

#[cfg(all(feature = "sql", not(target_arch = "wasm32")))]
#[tokio::test]
async fn configured_period_survives_actual_sql_reopen_and_revision_activation() {
    let directory = tempfile::tempdir().unwrap();
    let url = format!("file://{}", directory.path().join("period.db").display());
    let migrations = [crlt::Migration::new(1, "rulebook", SCHEMA)];
    let db = crlt::Db::open(crlt::Config::new(&url, "")).await.unwrap();
    db.migrate(&migrations).await.unwrap();
    let store = LibsqlStore::new(db);
    let change = |rulebook, at, policy_epoch| Change {
        rulebook,
        announced_at: at,
        effective_at: at,
        notice_seconds: 0,
        policy_epoch,
    };
    store
        .append("garden", None, change(configured(), 100, 1))
        .await
        .unwrap();
    let db = crlt::Db::open(crlt::Config::new(&url, "")).await.unwrap();
    db.migrate(&migrations).await.unwrap();
    let reopened = LibsqlStore::new(db);
    let first = reopened
        .load("garden", Selection::At(100))
        .await
        .unwrap()
        .unwrap();
    RecordEpochSettings::validate_catalogue(&first.change.rulebook, KEY).unwrap();
    assert_eq!(
        RecordEpochSettings::from_snapshot(&first.snapshot("garden", 100).unwrap(), KEY)
            .unwrap()
            .period_seconds,
        100
    );
    let mut next = first.change.rulebook.clone();
    next.set_community(KEY, Some(json!(200))).unwrap();
    reopened
        .append("garden", Some(first.revision), change(next.clone(), 200, 2))
        .await
        .unwrap();
    assert!(matches!(
        reopened
            .append("garden", Some(first.revision), change(next, 201, 3))
            .await,
        Err(Error::Conflict)
    ));
    for (now, expected) in [(199, 100), (200, 200)] {
        let revision = reopened
            .load("garden", Selection::At(now))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            RecordEpochSettings::from_snapshot(&revision.snapshot("garden", now).unwrap(), KEY)
                .unwrap()
                .period_seconds,
            expected
        );
    }
}
