//! Actual Rulebook resolution and stored revisions for an application-fixed key.
use crbk::*;
use serde_json::json;
const KEY: &str = "fixture/outcome_allowance";
fn configured() -> Rulebook {
    let mut book = Rulebook::default();
    OutcomeQuotaSettings::define(&mut book, KEY).unwrap();
    book.set_community(KEY, Some(json!(7))).unwrap();
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
fn common_quota_has_no_default_or_member_template_override() {
    let mut book = Rulebook::default();
    assert!(OutcomeQuotaSettings::from_snapshot(&snapshot(&book), KEY).is_err());
    OutcomeQuotaSettings::define(&mut book, KEY).unwrap();
    let original = book.clone();
    OutcomeQuotaSettings::define(&mut book, KEY).unwrap();
    assert_eq!(book, original);
    assert!(OutcomeQuotaSettings::from_snapshot(&snapshot(&book), KEY).is_err());
    book = configured();
    OutcomeQuotaSettings::validate_catalogue(&book, KEY).unwrap();
    assert_eq!(
        OutcomeQuotaSettings::from_snapshot(&snapshot(&book), KEY)
            .unwrap()
            .quota,
        7
    );
    assert!(
        book.resolve(&Values::from([(KEY.into(), json!(8))]))
            .is_err()
    );
    assert!(
        book.publish_template(
            TemplatePin {
                name: "fixture".into(),
                version: 1
            },
            Values::from([(KEY.into(), json!(8))])
        )
        .is_err()
    );
    book.set_platform(
        KEY,
        Some(PlatformValue {
            value: json!(3),
            force: true,
        }),
    )
    .unwrap();
    assert_eq!(
        OutcomeQuotaSettings::from_snapshot(&snapshot(&book), KEY)
            .unwrap()
            .quota,
        3
    );
    book.set_platform(
        KEY,
        Some(PlatformValue {
            value: json!(3),
            force: false,
        }),
    )
    .unwrap();
    book.set_community(KEY, Some(json!(null))).unwrap();
    assert!(OutcomeQuotaSettings::from_snapshot(&snapshot(&book), KEY).is_err());
    book.set_community(KEY, None).unwrap();
    assert_eq!(
        OutcomeQuotaSettings::from_snapshot(&snapshot(&book), KEY)
            .unwrap()
            .quota,
        3
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn exact_key_integer_bounds_and_immutable_catalogue_refuse_substitutions() {
    let original = snapshot(&configured());
    for key in [
        "",
        "a/",
        "action/admit",
        "gate/community/a",
        "provider/community/a/b",
        "different",
    ] {
        assert!(OutcomeQuotaSettings::from_snapshot(&original, key).is_err());
    }
    for value in [
        json!(null),
        json!(0),
        json!(-1),
        json!(7.0),
        json!(1.5),
        json!("7"),
        json!(1025),
        json!(u64::MAX),
    ] {
        let mut changed = original.clone();
        changed.content.insert(KEY.into(), value.clone());
        assert!(OutcomeQuotaSettings::from_snapshot(&changed, KEY).is_err());
        if !value.is_null() {
            assert!(configured().set_community(KEY, Some(value)).is_err());
        }
    }
    for quota in [1, 1024] {
        let mut book = configured();
        book.set_community(KEY, Some(json!(quota))).unwrap();
        assert_eq!(
            OutcomeQuotaSettings::from_snapshot(&snapshot(&book), KEY)
                .unwrap()
                .quota,
            quota
        );
    }
    for case in 0..6 {
        let mut definition = configured().catalog()[KEY].clone();
        match case {
            0 => definition.default = json!(7),
            1 => definition.lowest_layer = Layer::Member,
            2 => definition.kind = SettingKind::Template,
            3 => definition.bounds.max = Some(1023.into()),
            4 => definition.nullable = false,
            _ => definition.value_type = SettingType::Number,
        }
        let mut book = Rulebook::default();
        if book.define(KEY, definition).is_ok() {
            assert!(OutcomeQuotaSettings::validate_catalogue(&book, KEY).is_err());
            assert!(OutcomeQuotaSettings::define(&mut book, KEY).is_err());
        }
    }
}
#[cfg(all(feature = "sql", not(target_arch = "wasm32")))]
#[tokio::test]
async fn real_sql_reopen_and_exact_revision_activation_preserve_quota() {
    let directory = tempfile::tempdir().unwrap();
    let url = format!("file://{}", directory.path().join("outcomes.db").display());
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
    OutcomeQuotaSettings::validate_catalogue(&first.change.rulebook, KEY).unwrap();
    let mut next = first.change.rulebook.clone();
    next.set_community(KEY, Some(json!(9))).unwrap();
    reopened
        .append("garden", Some(1), change(next.clone(), 200, 2))
        .await
        .unwrap();
    assert!(matches!(
        reopened
            .append("garden", Some(1), change(next, 201, 3))
            .await,
        Err(Error::Conflict)
    ));
    for (now, quota, epoch) in [(199, 7, 1), (200, 9, 2)] {
        let active = reopened
            .load("garden", Selection::At(now))
            .await
            .unwrap()
            .unwrap();
        let snapshot = active.snapshot("garden", now).unwrap();
        assert_eq!(snapshot.policy_epoch, epoch);
        assert_eq!(
            OutcomeQuotaSettings::from_snapshot(&snapshot, KEY)
                .unwrap()
                .quota,
            quota
        );
    }
}
