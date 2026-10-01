#![cfg(all(feature = "sql", not(target_arch = "wasm32")))]

//! Shared backend contract and real libSQL transaction failure cases.

mod common;

use common::*;
use crbk::*;
use serde_json::json;

async fn contract(store: &impl Storage) {
    assert!(
        store
            .load("alpha", Selection::Latest)
            .await
            .unwrap()
            .is_none()
    );
    let mut b = decision_book(ActionPolicy {
        all_of: vec![requirement("voucher")],
        ..Default::default()
    });
    b.set_platform("quota", platform(json!(20), false)).unwrap();
    b.publish_template(pin(1), [("quota".into(), json!(null))].into())
        .unwrap();
    b.pin_template(Some(pin(1))).unwrap();
    b.set_community("timeout", Some(json!(15))).unwrap();
    let first = store
        .append("alpha", None, change(b.clone(), 10, 10))
        .await
        .unwrap();
    assert_eq!(first.revision, 1);
    assert_eq!(
        store.load("alpha", Selection::At(10)).await.unwrap(),
        Some(first.clone())
    );
    assert!(
        store
            .load("alpha", Selection::At(9))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .load("beta", Selection::Latest)
            .await
            .unwrap()
            .is_none()
    );
    store
        .append("beta", None, change(book(), 0, 0))
        .await
        .unwrap();
    b.set_community("quota", Some(json!(70))).unwrap();
    let mut next = change(b.clone(), 20, 120);
    next.notice_seconds = 100;
    let second = store.append("alpha", Some(1), next.clone()).await.unwrap();
    assert_eq!(second.revision, 2);
    assert_eq!(
        store.load("alpha", Selection::At(119)).await.unwrap(),
        Some(first.clone())
    );
    assert_eq!(
        store.load("alpha", Selection::At(120)).await.unwrap(),
        Some(second.clone())
    );
    assert_eq!(
        store.load("alpha", Selection::Latest).await.unwrap(),
        Some(second.clone())
    );
    assert_eq!(
        store.load("alpha", Selection::Revision(1)).await.unwrap(),
        Some(first.clone())
    );
    assert!(matches!(
        store.append("alpha", Some(1), next).await,
        Err(Error::Conflict)
    ));
    assert!(
        store
            .append("alpha", Some(2), change(b.clone(), 121, 119))
            .await
            .is_err()
    );
    let mut short = change(b.clone(), 121, 130);
    short.notice_seconds = 10;
    assert!(store.append("alpha", Some(2), short).await.is_err());
    let mut overflow = change(b.clone(), 121, i64::MAX);
    overflow.notice_seconds = u64::MAX;
    assert!(store.append("alpha", Some(2), overflow).await.is_err());
    let mut epoch = change(b, 121, 130);
    epoch.policy_epoch = 0;
    assert!(store.append("alpha", Some(2), epoch).await.is_err());
    assert_eq!(
        store.load("alpha", Selection::Latest).await.unwrap(),
        Some(second)
    );
    assert!(
        store
            .load("alpha", Selection::Revision(99))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .load("alpha", Selection::Revision(u64::MAX))
            .await
            .is_err()
    );
    assert!(store.load("alpha", Selection::At(-1)).await.is_err());
    assert!(store.load("", Selection::Latest).await.is_err());
    assert!(first.snapshot("alpha", 9).is_err());
    let snapshot = first.snapshot("alpha", 10).unwrap();
    assert_eq!(snapshot.content["quota"], json!(null));
    assert_eq!(snapshot.content["timeout"], 15);
    // Retaining a revision preserves the policy behind an already committed deadline.
    assert_eq!(
        first.snapshot("alpha", 130).unwrap().content["quota"],
        json!(null)
    );
}

#[tokio::test]
async fn memory_round_trip_contract() {
    contract(&MemoryStore::default()).await;
}

#[tokio::test]
async fn libsql_round_trip_contract_reopen_migrations_and_indexes() {
    let (dir, db, store) = local().await;
    contract(&store).await;
    store.check_query_plans("alpha").await.unwrap();
    assert_eq!(
        db.migrate(&[crlt::Migration::new(1, "rulebook", SCHEMA)])
            .await
            .unwrap(),
        0
    );
    let reopened = crlt::Db::open(crlt::Config::new(
        format!("file://{}", dir.path().join("rules.db").display()),
        "",
    ))
    .await
    .unwrap();
    reopened
        .migrate(&[crlt::Migration::new(1, "rulebook", SCHEMA)])
        .await
        .unwrap();
    let reopened = LibsqlStore::new(reopened);
    assert_eq!(
        reopened.load("alpha", Selection::Latest).await.unwrap(),
        store.load("alpha", Selection::Latest).await.unwrap()
    );
    let row = db
        .community("alpha")
        .unwrap()
        .query(
            "SELECT payload FROM template_value WHERE revision = ?1 AND entry_key = ?2",
            crlt::params![1_i64, "standard@1/quota"],
        )
        .await
        .unwrap();
    assert_eq!(row[0].get_str(0).unwrap(), "null");
}

#[tokio::test]
async fn simultaneous_writers_cannot_lose_updates() {
    let (_dir, _db, store) = local().await;
    store
        .append("alpha", None, change(book(), 0, 0))
        .await
        .unwrap();
    let a = store.clone();
    let b = store.clone();
    let (a, b) = tokio::join!(
        a.append("alpha", Some(1), change(book(), 1, 1)),
        b.append("alpha", Some(1), change(book(), 1, 1))
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(matches!(a, Err(Error::Conflict)) || matches!(b, Err(Error::Conflict)));
    assert_eq!(
        store
            .load("alpha", Selection::Latest)
            .await
            .unwrap()
            .unwrap()
            .revision,
        2
    );
}

#[tokio::test]
async fn immutable_definitions_and_templates_reject_forged_drafts() {
    let store = MemoryStore::default();
    let mut b = book();
    b.publish_template(pin(1), Values::new()).unwrap();
    store
        .append("alpha", None, change(b.clone(), 0, 0))
        .await
        .unwrap();
    for path in ["catalog", "templates"] {
        let mut value = serde_json::to_value(&b).unwrap();
        value[path] = json!({});
        let forged = serde_json::from_value(value).unwrap();
        assert!(
            store
                .append("alpha", Some(1), change(forged, 1, 1))
                .await
                .is_err()
        );
    }
    assert_eq!(
        store
            .load("alpha", Selection::Latest)
            .await
            .unwrap()
            .unwrap()
            .revision,
        1
    );
}

#[tokio::test]
async fn database_failure_rolls_back_all_tables() {
    let dir = tempfile::tempdir().unwrap();
    let db = crlt::Db::open(crlt::Config::new(
        format!("file://{}", dir.path().join("failure.db").display()),
        "",
    ))
    .await
    .unwrap();
    let schema = SCHEMA.replace(
        "entry_key TEXT NOT NULL, payload TEXT NOT NULL,",
        "entry_key TEXT NOT NULL, payload TEXT NOT NULL CHECK (payload != '77'),",
    );
    db.migrate(&[crlt::Migration::new(1, "failure", &schema)])
        .await
        .unwrap();
    let store = LibsqlStore::new(db);
    let mut b = book();
    b.set_community("quota", Some(json!(77))).unwrap();
    assert!(matches!(
        store.append("alpha", None, change(b, 0, 0)).await,
        Err(Error::Storage)
    ));
    assert!(
        store
            .load("alpha", Selection::Latest)
            .await
            .unwrap()
            .is_none()
    );
    store
        .append("alpha", None, change(book(), 0, 0))
        .await
        .unwrap();
}

#[tokio::test]
async fn corrupt_values_fail_closed_without_leaking_payloads() {
    let (_dir, db, store) = local().await;
    let mut b = book();
    b.set_community("quota", Some(json!(2))).unwrap();
    store.append("alpha", None, change(b, 0, 0)).await.unwrap();
    db.community("alpha")
        .unwrap()
        .execute(
            "UPDATE community_value SET payload = ?1 WHERE revision = ?2 AND entry_key = ?3",
            crlt::params!["\"private-input\"", 1_i64, "quota"],
        )
        .await
        .unwrap();
    let error = store.load("alpha", Selection::Latest).await.unwrap_err();
    assert!(matches!(error, Error::Corrupt));
    assert!(!format!("{error:?} {error}").contains("private-input"));
}

#[tokio::test]
async fn prospective_boundaries_reject_without_changing_published_revision() {
    let (_dir, _db, sql) = local().await;
    async fn exercise(store: &impl Storage) {
        for key in ["", "bad\0community"] {
            assert!(store.append(key, None, change(book(), 0, 0)).await.is_err());
            assert!(store.load(key, Selection::Latest).await.is_err());
        }
        assert!(store.load("alpha", Selection::Revision(0)).await.is_err());
        let mut initial = change(book(), 10, 20);
        initial.policy_epoch = 2;
        store.append("alpha", None, initial).await.unwrap();
        let mut failures = vec![change(book(), -1, 21), change(book(), 9, 21), change(book(), 21, 20)];
        let mut epoch = change(book(), 21, 21);
        epoch.policy_epoch = u64::MAX;
        failures.push(epoch);
        let mut overflow = change(book(), i64::MAX, i64::MAX);
        overflow.notice_seconds = 1;
        failures.push(overflow);
        failures.push(change(book(), 21, 21));
        for invalid in failures {
            assert!(store.append("alpha", Some(1), invalid).await.is_err());
            assert_eq!(store.load("alpha", Selection::Latest).await.unwrap().unwrap().revision, 1);
        }
    }
    exercise(&MemoryStore::default()).await;
    exercise(&sql).await;
}

#[tokio::test]
async fn malformed_imported_template_rows_cannot_be_loaded_or_extended() {
    for (table, key, payload) in [
        ("template", "missing-separator", r#"{"base":{},"values":{}}"#),
        ("template", "standard@invalid", r#"{"base":{},"values":{}}"#),
        ("template_value", "missing-slash", "10"),
        ("template_value", "standard@1/quota", "10"),
    ] {
        let (_dir, db, store) = local().await;
        store.append("alpha", None, change(book(), 0, 0)).await.unwrap();
        db.community("alpha").unwrap().execute(
            &format!("INSERT INTO {table} (revision, entry_key, payload) VALUES (?1, ?2, ?3)"),
            crlt::params![1_i64, key, payload],
        ).await.unwrap();
        assert!(matches!(store.load("alpha", Selection::Latest).await, Err(Error::Corrupt)));
        assert!(matches!(store.append("alpha", Some(1), change(book(), 1, 1)).await, Err(Error::Corrupt)));
    }
}

#[tokio::test]
async fn incomplete_import_schema_and_invalid_pin_rows_fail_closed() {
    for omit in ["rulebook_revision", "setting_catalog", "template"] {
        let directory = tempfile::tempdir().unwrap();
        let url = format!("file://{}", directory.path().join("import.db").display());
        let db = crlt::Db::open(crlt::Config::new(url, "")).await.unwrap();
        let schema = SCHEMA.split(';').filter(|statement| {
            !statement.contains(&format!("CREATE TABLE {omit} ("))
                && !(omit == "rulebook_revision" && statement.contains("CREATE UNIQUE INDEX"))
        }).collect::<Vec<_>>().join(";");
        db.migrate(&[crlt::Migration::new(1, "incomplete", &schema)]).await.unwrap();
        let store = LibsqlStore::new(db.clone());
        assert!(matches!(store.check_query_plans("alpha").await, Err(Error::Storage)));
        if omit != "rulebook_revision" {
            db.community("alpha").unwrap().execute(
                "INSERT INTO rulebook_revision (revision, announced_at, effective_at, notice_seconds, policy_epoch) VALUES (?1, ?2, ?3, ?4, ?5)",
                crlt::params![1_i64, 0_i64, 0_i64, 0_i64, 1_i64],
            ).await.unwrap();
        }
        assert!(matches!(store.load("alpha", Selection::Latest).await, Err(Error::Storage)));
    }
    let schema = SCHEMA.replace(" CHECK (entry_key = 'pin')", "").replace(" CHECK (revision > 0)", "");
    let directory = tempfile::tempdir().unwrap();
        let url = format!("file://{}", directory.path().join("import.db").display());
        let db = crlt::Db::open(crlt::Config::new(url, "")).await.unwrap();
    db.migrate(&[crlt::Migration::new(1, "import", &schema)]).await.unwrap();
    let store = LibsqlStore::new(db.clone());
    store.append("alpha", None, change(book(), 0, 0)).await.unwrap();
    let scope = db.community("alpha").unwrap();
    scope.execute(
        "INSERT INTO community_template (revision, entry_key, payload) VALUES (?1, ?2, ?3)",
        crlt::params![1_i64, "foreign", r#"{"name":"standard","version":1}"#],
    ).await.unwrap();
    assert!(matches!(store.load("alpha", Selection::Latest).await, Err(Error::Corrupt)));
    db.community("zero").unwrap().execute(
        "INSERT INTO rulebook_revision (revision, announced_at, effective_at, notice_seconds, policy_epoch) VALUES (?1, ?2, ?3, ?4, ?5)",
        crlt::params![0_i64, 0_i64, 0_i64, 0_i64, 1_i64],
    ).await.unwrap();
    assert!(matches!(store.load("zero", Selection::Latest).await, Err(Error::Corrupt)));
}

#[tokio::test]
async fn maximum_imported_revision_cannot_wrap_or_overwrite() {
    let (_dir, db, store) = local().await;
    store.append("alpha", None, change(Rulebook::default(), 0, 0)).await.unwrap();
    db.community("alpha").unwrap().execute(
        "UPDATE rulebook_revision SET revision = ?1 WHERE revision = ?2",
        crlt::params![i64::MAX, 1_i64],
    ).await.unwrap();
    assert!(matches!(
        store.append("alpha", Some(i64::MAX as u64), change(Rulebook::default(), 1, 1)).await,
        Err(Error::Invalid("revision overflow"))
    ));
    assert_eq!(store.load("alpha", Selection::Latest).await.unwrap().unwrap().revision, i64::MAX as u64);
}
