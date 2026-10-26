# crbk agent notes

Work in a lane (`~/agents/scratch/lane-new <lane>`); never edit `~/agents/scratch/ws`.
Run commands from the lane root (`<lane>/ws`); `<lane>` below is your lane name.

Build and test (quote the verdict line, never infer from a pipe):
- `~/agents/scratch/cbuild <lane> test -p crbk` — green, exit 0 (all suites
  `ok`; `tests/turso.rs` runs only with `TURSO_URL`+`TURSO_TOKEN` against a
  disposable test database, otherwise skipped)
- `fmt -p crbk -- --check` is red at HEAD from a pre-existing drift
  (`src/catalog.rs:4` import order); `clippy --all-targets -- -D warnings` is
  red at HEAD from `tests/forum.rs:39` (constant assertion under local
  clippy; CI uses the locked toolchain). Fix code, never lint.
- Full gate (locked deps, coverage, advisories) runs in CI only: `.ci/verify.sh`.

Land: `cargo fmt` in touched repos, rebase on main (never force-push),
`cbuild <lane> xtask land --dry`, then `xtask land`.

Invariants (rule -> enforcing test):
- Forum mechanics are code constants, never settings
  -> `tests/forum.rs::the_mechanics_are_not_settings`
- Room-pass quota is 1024, the issuer maximum, per 2026-10-26 decision
  -> `tests/forum.rs::the_quota_covers_a_room_entry_every_84_seconds_all_epoch`
- Clients accepted by SemVer precedence, build metadata ignored
  -> `tests/forum.rs::clients_are_accepted_by_semver_precedence`
- Immutable definitions/templates reject forged drafts
  -> `tests/storage.rs::immutable_definitions_and_templates_reject_forged_drafts`
- Simultaneous writers cannot lose updates
  -> `tests/storage.rs::simultaneous_writers_cannot_lose_updates`
- Poisoned store refuses; corrupt values fail closed without leaking payloads
  -> `src/tests/storage.rs::poisoned_memory_store_refuses_reads_and_writes`,
  `tests/storage.rs::corrupt_values_fail_closed_without_leaking_payloads`
- Committed reads never expose uncommitted or mixed-revision rows
  -> `tests/writer_reads.rs::append_publication_never_exposes_uncommitted_or_mixed_revision_rows`
- `forbid(unsafe_code)` (`src/lib.rs:2`); hostile input refused
  (`deny_unknown_fields`, `src/catalog.rs:81,109`)

Pointers: `src/lib.rs`, `src/catalog.rs`, `src/forum.rs`, `tests/`,
`docs/CONTRACT.md`, `.ci/verify.sh`.
Commits: explicit paths, imperative, author per `~/agents/AGENT.md`; no AI trailers.
`publish=false`; LGPL-3.0-only WITH linking exception; no registry publication.
