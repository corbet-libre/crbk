use super::*;

#[tokio::test]
async fn poisoned_memory_store_refuses_reads_and_writes() {
    let store = MemoryStore::default();
    let result = std::panic::catch_unwind(|| {
        let _guard = store.communities.lock().unwrap();
        panic!("simulated storage owner panic");
    });
    assert!(result.is_err());
    assert!(matches!(
        store.load("alpha", Selection::Latest).await,
        Err(Error::Storage)
    ));
    let change = Change {
        rulebook: Rulebook::default(),
        announced_at: 0,
        effective_at: 0,
        notice_seconds: 0,
        policy_epoch: 1,
    };
    assert!(matches!(
        store.append("alpha", None, change).await,
        Err(Error::Storage)
    ));
}
