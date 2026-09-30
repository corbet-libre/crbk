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
```

Authentication, role authorization, trusted clocks, notice delivery and community
routing belong to the service. Supply already verified proof metadata. A global
proof must already be bound to the subject by the caller; `crbk` cannot validate
a passport or infer cross-community identity. There is no crypto, request logger,
login tracking, raw gate data, environment-reading production code or signing key.

## Dependency survey

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

The previous crlt pin, `9c076b1`, is an ancestor of this revision. The only
intervening change corrects an upstream query-plan test; the API and dependency
graph are unchanged. The manifest and lockfile use the same full revision so
consumers pinned to it can share one `crlt::Db` type and connection pool. Other
cvld leaves are not dependencies of this crate and need no additional pins here.

## Validation

GitHub Actions runs stable Rust formatting, strict Clippy and tests, retaining
the dependency lock as an artifact. A metadata check rejects floating Corbet Git
dependencies and duplicate Corbet packages, including transitive leaves. Current
upstream heads are checked when updating pins; CI validates the locked graph
without following moving branches. Do not run Cargo on the workstation. Tests
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
