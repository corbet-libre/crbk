//! Persistence mechanics only; real member authorization is tested by the host.
#![cfg(all(feature = "sql", not(target_arch = "wasm32")))]
mod common;
use common::*;
use crbk::*;
use std::{sync::Arc, time::Duration};

struct Cut {
    checks: usize,
    releases: usize,
    deny: bool,
    fail_release: usize,
}
impl CommitGuard for Cut {
    async fn check(&mut self) -> Result<()> {
        self.checks += 1;
        if self.deny {
            Err(Error::Storage)
        } else {
            Ok(())
        }
    }
    fn release(&mut self) -> Result<()> {
        self.releases += 1;
        if self.releases == self.fail_release {
            Err(Error::Storage)
        } else {
            Ok(())
        }
    }
}
fn cut(deny: bool, fail_release: usize) -> Cut {
    Cut {
        checks: 0,
        releases: 0,
        deny,
        fail_release,
    }
}

#[tokio::test]
async fn unsupported_checked_backend_never_falls_back_to_original_append() {
    let store = MemoryStore::default();
    let mut guard = cut(false, 0);
    assert!(matches!(
        store
            .append_checked("alpha", None, change(book(), 0, 0), &mut guard)
            .await,
        Err(Error::Storage)
    ));
    assert_eq!(guard.checks, 0);
    assert!(
        store
            .load("alpha", Selection::Latest)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn final_refusal_and_precommit_cut_roll_back_real_staged_revision() {
    for (deny, release) in [(true, 0), (false, 1)] {
        let (_dir, _db, store) = local().await;
        let before = store
            .append("alpha", None, change(book(), 0, 0))
            .await
            .unwrap();
        let mut guard = cut(deny, release);
        assert!(matches!(
            store
                .append_checked("alpha", Some(1), change(book(), 1, 1), &mut guard)
                .await,
            Err(Error::Storage)
        ));
        assert_eq!(guard.checks, 1);
        assert_eq!(
            store.load("alpha", Selection::Latest).await.unwrap(),
            Some(before)
        );
    }
}

#[tokio::test]
async fn failed_acknowledgment_cut_reports_uncertainty_without_erasing_commit() {
    let (_dir, _db, store) = local().await;
    let mut guard = cut(false, 2);
    assert!(matches!(
        store
            .append_checked("alpha", None, change(book(), 0, 0), &mut guard)
            .await,
        Err(Error::Storage)
    ));
    assert_eq!((guard.checks, guard.releases), (1, 2));
    assert_eq!(
        store
            .load("alpha", Selection::Latest)
            .await
            .unwrap()
            .unwrap()
            .revision,
        1
    );
    assert!(matches!(
        store
            .append_checked("alpha", None, change(book(), 0, 0), &mut cut(false, 0))
            .await,
        Err(Error::Conflict)
    ));
}

struct Waiting(Arc<tokio::sync::Notify>);
impl CommitGuard for Waiting {
    async fn check(&mut self) -> Result<()> {
        self.0.notify_one();
        std::future::pending().await
    }
    fn release(&mut self) -> Result<()> {
        panic!("cancelled owner check cannot reach commit")
    }
}
#[tokio::test]
async fn cancelled_owner_read_drops_real_staged_transaction_without_append() {
    let (_dir, _db, store) = local().await;
    let entered = Arc::new(tokio::sync::Notify::new());
    let mut guard = Waiting(entered.clone());
    let mut write = Box::pin(store.append_checked("alpha", None, change(book(), 0, 0), &mut guard));
    tokio::time::timeout(Duration::from_secs(5), async {
        tokio::select! { _ = entered.notified() => {}, result = &mut write => panic!("unexpected completed write: {result:?}") }
    }).await.unwrap();
    drop(write);
    assert!(
        tokio::time::timeout(
            Duration::from_secs(5),
            store.load("alpha", Selection::Latest)
        )
        .await
        .unwrap()
        .unwrap()
        .is_none()
    );
}
