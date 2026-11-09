//! The crate's integration tests in one binary.
//!
//! Every former `tests/<name>.rs` is a module here, so a warm edit loop links one test
//! executable instead of one per file. The test names keep their module prefix
//! (`it::<name>::<test>`). The targets the wasm CI leg drives by name (`policies`,
//! `settings`, `membership`, `group_defaults`) and the two `required-features = ["sql"]`
//! targets (`storage`, `turso`) stay separate; `tests/common` stays at its path because
//! those targets still declare `mod common;` themselves.
mod catalog;
mod commit_guard;
#[path = "../common/mod.rs"]
mod common;
mod forum;
mod record;
mod settlement_settings;
mod waves;
mod writer_reads;
