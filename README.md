# crbk

**Rulebook: sparse layered settings and the decisions on them.**

Part of `cvld`, the permanent door of the cmtymeet trust stack. Native Rust;
interfaces may change. [Implemented contract](docs/CONTRACT.md).

## API

- `Rulebook`: typed setting catalogue, platform values with force switches,
  immutable template versions, explicit template pins, community deviations.
  `resolve(&member_values)` checks transient member latitude and returns flat JSON.
- `Snapshot::may(subject, action, gate_results, now)`: pure yes/no plus missing
  gate groups, alternatives and remaining counts; all/any/k-of-n, membership,
  expiry, maximum proof age, gate and provider switches.
- `Storage::{append, load}`: atomic revision comparison, immutable revisions,
  prospective activation and minimum notice. `MemoryStore` and `LibsqlStore`
  implement the same contract. `Selection::At(now)` reads active configuration;
  `Latest` includes scheduled changes for editing.
- `Revision::snapshot`: serializable flat community content for signing by `csgn`.
  No member overrides, identities or gate results enter storage or snapshots.

```rust
use crbk::{Bounds, Layer, Rulebook, Setting, SettingKind, SettingType, Values};
use serde_json::json;

let mut rules = Rulebook::default();
rules.define("example_limit", Setting {
    value_type: SettingType::Integer,
    nullable: true,
    default: json!(10),
    bounds: Bounds { min: Some(0.into()), max: Some(100.into()) },
    lowest_layer: Layer::Member,
    kind: SettingKind::Template,
})?;
rules.set_community("example_limit", Some(json!(null)))?;
assert!(rules.resolve(&Values::new())?["example_limit"].is_null());
rules.set_community("example_limit", None)?;
assert_eq!(rules.resolve(&Values::new())?["example_limit"], 10);
# Ok::<(), crbk::Error>(())
```

The example is illustrative. The optional `define_membership_settings` catalogue
provides credential/probation day settings; the generic rulebook ships no gate choices,
provider choices or notice durations. Register control settings with `gate_key`,
`provider_key` and `action_key`. Gates/providers require a resolved boolean `true`;
missing/null switches are off. Missing/null actions deny. An explicitly configured
empty `ActionPolicy` permits. Member overrides cannot change control settings.

## Storage integration

The service opens a `crlt::Db`, incorporates `crbk::SCHEMA` in its complete
`crlt::Migration` list, then constructs `LibsqlStore::new(db)`. Migration version
numbers belong to the service so sibling crates can share a database. Deploy one
database per community. Every application table also carries `community_id`;
crlt injects the namespace and enforces indexed plans for every query.

```rust,no_run
# #[cfg(all(feature = "sql", not(target_arch = "wasm32")))]
# mod sql_example {
use crbk::{Change, LibsqlStore, Rulebook, Selection, Storage, SCHEMA};
use crlt::{Config, Db, Migration};

# async fn example(url: String, token: String) -> Result<(), Box<dyn std::error::Error>> {
let db = Db::open(Config::new(url, token)).await?;
db.migrate(&[Migration::new(1, "rulebook", SCHEMA)]).await?;
let store = LibsqlStore::new(db);
store.append("example", None, Change {
    rulebook: Rulebook::default(), announced_at: 100, effective_at: 160,
    notice_seconds: 60, policy_epoch: 1,
}).await?;
assert!(store.load("example", Selection::At(159)).await?.is_none());
let active = store.load("example", Selection::At(160)).await?.unwrap();
let unsigned = active.snapshot("example", 160)?;
# let _ = unsigned;
# Ok(())
# }
# }
```

Authentication, role authorization, trusted clocks, notice delivery and community
routing belong to the service. Supply already verified proof metadata. A global
proof must already be bound to the subject by the caller; `crbk` cannot validate
a passport or infer cross-community identity. There is no crypto, request logger,
login tracking, raw gate data, environment-reading production code or signing key.

## Dependency survey

The optional `define_forum_settings` catalogue has **no usable defaults**.
`forum.heartbeat_ms`, `forum.lease_ms`, `forum.minimum_client_version`, and
`forum.room_passes_per_epoch` must be explicitly configured by the platform or
community. `ForumSettings::from_snapshot` projects the original authenticated
settings snapshot, rejects null/missing values, requires positive exact wire
integers (at most 2^53−1), and requires heartbeat shorter than lease. The quota
is total per member/common epoch, independent of rooms. Forum still clamps
leases to its storage TTL and current credential/authority deadline; this
projection neither authenticates snapshots nor performs issuance.

The maintained MIT/Apache-2.0 [semver](https://docs.rs/semver/latest/semver/struct.Version.html)
crate was selected on 2026-10-02 for parsing and minimum-version precedence.
`ForumSettings::accepts_client` ignores build metadata and compares prereleases
according to SemVer. No local version parser or string-order comparison exists.

Surveyed crates.io documentation and upstream GitHub sources on 2026-09-30:

| Candidate | Decision and reason |
|---|---|
| [config](https://docs.rs/config/latest/config/) (MIT/Apache-2.0) | Application configuration sources and merging. Does not supply catalogue authority, pinned templates, policy revisions or gate decisions; adopting it would still require the complete domain layer. |
| [Figment](https://docs.rs/figment/latest/figment/) (MIT/Apache-2.0) | Maintained provider/profile composition and extraction. Recursive configuration merging is not the required whole-setting, null-terminating read chain; no persistence or prospective policy lifecycle. |
| [Cedar](https://github.com/cedar-policy/cedar) (Apache-2.0) | Maintained authorization language, broader than the required fixed all/any/k-of-n vocabulary. A parser/evaluator plus translations would add complexity without implementing sparse settings or missing-gate groups. |
| [serde_json](https://docs.rs/serde_json/latest/serde_json/) and [Serde](https://github.com/serde-rs/serde) (MIT/Apache-2.0) | Selected for JSON values, explicit null, typed policies, validation boundaries and snapshots. Only the small contract-specific resolution and decision rules are local. |
| [crlt](https://github.com/corbet-foss/crlt) (LGPL-3.0-only WITH LGPL-3.0-linking-exception) | Selected at `6b94dacd7fa04aa8847c62c6471a1fc5c0f6c9dc`, the current `main` verified with `git ls-remote` on 2026-09-30. Delegates execution to official libSQL with scoped transactions, migrations and index enforcement. No direct database client fallback is needed. |
| [libsql](https://github.com/tursodatabase/libsql) (MIT) | Official client, used through crlt. Direct use was the fallback only if crlt lacked a usable API. |

`thiserror` supplies typed redacted errors. Tokio and tempfile are test-only direct
dependencies for async real-file round trips. No GPL-only or AGPL-only direct
dependency is introduced. No usable `corbet-foss/cgts` or `corbet-foss/cmbr` API
was available: `GateResults` and `Subject`/`MembershipState` are the documented
local boundary until those facades expose shared types. `csgn` consumes serialized
snapshots; signing is intentionally not a dependency of this crate.

The crlt manifest follows main. CI resolves one shared lock snapshot and checks
that each first-party crate appears at exactly one source revision, so consumers
share one `crlt::Db` type and connection pool. Other cvld leaves are not direct
dependencies of this crate.

## Validation

GitHub Actions runs stable Rust formatting, strict Clippy and tests, retaining
the dependency lock as an artifact. A metadata check requires first-party main
declarations and rejects duplicate packages, including transitive leaves. The
resolver refreshes the lock once; every substantive check uses that exact snapshot. Do not run Cargo on the workstation. Tests
cover the full sparse read chain, nulls, force, exact integer bounds, template
pinning, member authority, gate decisions, prospective revisions, stale writers,
namespace isolation, database reopen, query plans, corruption and transaction
rollback. Both storage implementations run the same round-trip scenario.

`tests/turso.rs` runs only when both `TURSO_URL` and `TURSO_TOKEN` are nonempty;
otherwise it prints a skip reason. Run it against a **disposable test database**:
it applies schema and retains isolated test rows. No credentials are stored in
the repository or configured in CI. Never publish this crate to a registry.

## License

Copyright 2026 Julian Y. Richard Corbet. Licensed under [LGPL-3.0-only](LICENSES/LGPL-3.0-only.txt) [WITH LGPL-3.0-linking-exception](LICENSES/LGPL-3.0-linking-exception.txt). See [LICENSE.md](LICENSE.md).

## Continuous verification

Dependency updates follow main and are tested against one CI-resolved lockfile.
Line and branch coverage target 100%; failures remain blocking. See
[the coverage contract](docs/COVERAGE.md) for measurement and exclusions.

## Portable policy evaluation

The default `sql` feature preserves the native `LibsqlStore` integration through
crlt. Device consumers use `default-features = false` to reuse the same
`Snapshot::may` evaluator, sparse settings, membership rules and serialization
without linking a database driver. SQL is unavailable on wasm32. The in-memory
revision store and pure revision/snapshot types remain available.

CI executes the same all/any/threshold, scope, provider, expiry, proof-age and
settings vectors on native Rust and wasm32 with the wasm-bindgen Node runner.
This adds no policy evaluator and changes no gate semantics.

`define_record_settings` materializes `records.epoch_seconds` in the original signed settings catalogue (decided default: 86400 seconds). `RecordSettings::from_snapshot` requires that resolved value and rejects missing/malformed values. Community/platform authority applies; members cannot override it. This interval is neither a frontier proof nor permission to use a stale epoch. The frontier owner supplies the signed common schedule and enforces its own checked clock bounds.
