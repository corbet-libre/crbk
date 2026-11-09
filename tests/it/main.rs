//! The crate's integration tests in one binary.
//!
//! Every former `tests/<name>.rs` is a module here, so a warm edit loop links one test
//! executable instead of one per file. The test names keep their module prefix
//! (`it::<name>::<test>`), and the platform gates stay inside the moved files: `policies`,
//! `settings`, `membership` and `group_defaults` also run on wasm32, while `storage`,
//! `turso`, `commit_guard` and `writer_reads` are native `sql` tests.
mod catalog;
mod commit_guard;
#[path = "../common/mod.rs"]
mod common;
mod forum;
mod group_defaults;
mod membership;
mod policies;
mod record;
mod settings;
mod settlement_settings;
mod storage;
mod turso;
mod waves;
mod writer_reads;
