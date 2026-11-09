//! Opt-in round trip against a caller-supplied disposable Turso database.
#![cfg(all(feature = "sql", not(target_arch = "wasm32")))]

use crate::common::*;
use crbk::*;

#[tokio::test]
async fn optional_real_turso_round_trip() {
    let (Ok(url), Ok(token)) = (std::env::var("TURSO_URL"), std::env::var("TURSO_TOKEN")) else {
        eprintln!("skipped: TURSO_URL and TURSO_TOKEN are not both set");
        return;
    };
    if url.is_empty() || token.is_empty() {
        eprintln!("skipped: TURSO_URL or TURSO_TOKEN is empty");
        return;
    }
    let db = crlt::Db::open(crlt::Config::new(url, token))
        .await
        .expect("open disposable test database");
    db.migrate(&[crlt::Migration::new(1, "rulebook", SCHEMA)])
        .await
        .expect("apply test schema");
    let store = LibsqlStore::new(db);
    let community = format!(
        "crbk-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let saved = store
        .append(&community, None, change(book(), 0, 0))
        .await
        .unwrap();
    assert_eq!(
        store.load(&community, Selection::At(0)).await.unwrap(),
        Some(saved)
    );
    store.check_query_plans(&community).await.unwrap();
}
