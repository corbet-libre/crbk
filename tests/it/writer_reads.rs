//! Real committed revision reads while another owner retains the same DB writer.
#![cfg(all(feature = "sql", not(target_arch = "wasm32")))]
use crate::common::*;
use crbk::*;
use std::time::Duration;

async fn database(connections: usize) -> (tempfile::TempDir, crlt::Db, LibsqlStore) {
    let dir = tempfile::tempdir().unwrap();
    let mut config = crlt::Config::new(
        format!("file://{}", dir.path().join("rules.db").display()),
        "",
    );
    config.max_connections = connections;
    config.timeout = Duration::from_millis(500);
    config.busy_timeout = Duration::from_millis(100);
    let db = crlt::Db::open(config).await.unwrap();
    db.migrate(&[crlt::Migration::new(1, "rulebook", SCHEMA)])
        .await
        .unwrap();
    let store = LibsqlStore::new(db.clone());
    (dir, db, store)
}

#[tokio::test]
async fn committed_revision_loads_under_an_independent_owner_writer() {
    let (_dir, db, store) = database(2).await;
    let first = store
        .append("alpha", None, change(book(), 0, 0))
        .await
        .unwrap();
    let writer = db.community("alpha").unwrap().tx().await.unwrap();
    for selection in [Selection::Latest, Selection::At(0), Selection::Revision(1)] {
        assert_eq!(
            store.load("alpha", selection).await.unwrap(),
            Some(first.clone())
        );
    }
    assert!(
        store
            .load("beta", Selection::Latest)
            .await
            .unwrap()
            .is_none()
    );
    writer.rollback().await.unwrap();
}

struct ReadCommitted {
    store: LibsqlStore,
    before: Revision,
    checked: bool,
}
impl CommitGuard for ReadCommitted {
    async fn check(&mut self) -> Result<()> {
        // append_checked has already staged the NEXT complete revision under
        // its real writer. These independent reads must see only the previous
        // committed header and its immutable child rows, never the staged one.
        for selection in [Selection::Latest, Selection::At(1), Selection::Revision(1)] {
            assert_eq!(
                self.store.load("alpha", selection).await?,
                Some(self.before.clone())
            );
        }
        assert!(
            self.store
                .load("alpha", Selection::Revision(2))
                .await?
                .is_none()
        );
        self.checked = true;
        Ok(())
    }
    fn release(&mut self) -> Result<()> {
        assert!(self.checked);
        Ok(())
    }
}

#[tokio::test]
async fn append_publication_never_exposes_uncommitted_or_mixed_revision_rows() {
    let (_dir, _db, store) = database(2).await;
    let first = store
        .append("alpha", None, change(book(), 0, 0))
        .await
        .unwrap();
    let mut changed = book();
    changed
        .set_community("quota", Some(serde_json::json!(70)))
        .unwrap();
    changed
        .set_community("timeout", Some(serde_json::json!(15)))
        .unwrap();
    let mut guard = ReadCommitted {
        store: store.clone(),
        before: first.clone(),
        checked: false,
    };
    let second = store
        .append_checked("alpha", Some(1), change(changed, 1, 1), &mut guard)
        .await
        .unwrap();
    assert!(guard.checked);
    assert_eq!(
        store.load("alpha", Selection::Latest).await.unwrap(),
        Some(second)
    );
    assert_eq!(
        store.load("alpha", Selection::Revision(1)).await.unwrap(),
        Some(first)
    );
}

#[tokio::test]
async fn single_connection_owner_writer_refuses_boundedly_without_a_cached_result() {
    let (_dir, db, store) = database(1).await;
    let first = store
        .append("alpha", None, change(book(), 0, 0))
        .await
        .unwrap();
    let writer = db.community("alpha").unwrap().tx().await.unwrap();
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        store.load("alpha", Selection::Latest),
    )
    .await
    .unwrap();
    assert!(matches!(result, Err(Error::Storage)));
    writer.rollback().await.unwrap();
    assert_eq!(
        store.load("alpha", Selection::Latest).await.unwrap(),
        Some(first)
    );
}
