# crbk implemented contract

Native Rust leaf library, LGPL-3.0-only WITH LGPL-3.0-linking-exception. It owns
settings and decisions under `cplc`, within the community side of `cvld` v0.4.
It neither authenticates subjects nor verifies cryptographic proofs. Signing is
`csgn`'s responsibility. No login dates, request logs, raw gate inputs, member
history, signing keys or cryptographic primitives are stored or implemented.

## Catalogue and resolution

Each definition has a key, JSON type, nullable flag, default, inclusive numeric
bounds, deepest allowed override layer, and technical/template kind. Defaults and
every supplied layer are validated, including values hidden by force. Integer
bounds use exact integer comparisons through the full JSON signed/unsigned range.
Catalogue definitions cannot be replaced or removed from a committed lineage;
new definitions may be added. Invalid deserialized drafts also fail validation.

The resolution order is:

```text
forced platform value
otherwise first present: member -> community -> pinned template -> platform -> default
```

Presence is a map entry / database row, not JSON truthiness. Null is a value when
the definition permits it and terminates the walk. False, zero, empty strings,
arrays and objects also terminate. Whole settings replace each other; objects
are never recursively merged. `None` in setters removes an override, whereas
`Some(Value::Null)` stores explicit null. Rows are explicit deviations; values
equal to an inherited value remain explicit so their intent survives later edits.

Template versions are published once and explicitly pinned by name/version.
Publication freezes the platform/default fallback for all currently defined
template-kind settings that permit a template layer. This base is metadata in
`template`; sparse explicit deviations live in `template_value`. Technical
settings cannot be placed in templates and follow live platform fallbacks.
Platform force always wins, including over pinned bases. A setting introduced
after template publication falls through until a new template version includes
it. Changing pins is an explicit revisioned change. Unpinning restores live
fallbacks. Member latitude is validated input to `resolve`, never permanent data.

## Gate decisions

Control keys are `gate/<level>/<gate>`, `provider/<level>/<gate>/<provider>` and
`action/<action>`, where level is `global` or `community`. The catalogue requires
boolean switches and typed action policies and forbids member control overrides.
Unknown/null gates or providers are off. Unknown/null actions deny. Providers
are scoped to both gate and level. No confidence scores or trust anchors exist.

`Snapshot::may(subject, action, results, now)` is pure and returns a serializable
`Decision { allowed, missing }`. A policy is the conjunction of all-of, any-of,
optional k-of-n, and optional allowed membership states. Empty all/any groups
and empty membership constraints impose no restriction; an explicitly configured
empty policy permits. Thresholds require `1 <= k <= n` with distinct gate/level
pairs; multiple results or provider variants never increase the count.
Future-issued snapshots and empty decision scopes are rejected.

Only a matching subject, scope, community (for community results), enabled gate,
enabled provider and unexpired proof counts. Expiry is exclusive. A maximum age
is inclusive and requires verified `proven_at` metadata; absent or future times
fail that requirement. Negative times and future proof times fail. The published
gate format lacks an issuance field, so callers unable to supply verified
issuance metadata cannot satisfy maximum-age policies. The optional metadata is
transient proof time, not a login date. The caller verifies global presentations
and binds them to this subject before supplying results; `crbk` does not link
global identities to community pseudonyms.

Missing information retains conjunct groups, remaining counts and alternatives.
Disabled gate/provider alternatives are marked for configuration diagnosis and
must not be offered as active lobby steps. Verdicts never echo subjects or proof
payloads. Malformed policies return errors rather than allowing access.

`GateResults` is the minimal local adapter trait for `cgts`; `Subject` and
`MembershipState` are local membership inputs until facade APIs exist. Services
translate richer enrolment state into pending/admitted/lapsed/released.

## Revisions, notice and snapshots

`Storage::append(community, expected_latest, Change)` validates the entire draft,
checks the expected latest revision and writes atomically. `None` requires an
empty namespace. Revisions are positive, contiguous and append-only. Stale
writers fail with `Conflict`. Published templates and catalogue definitions are
immutable across revisions. No implicit retries occur on ambiguous write errors.

The caller supplies trusted announcement time, activation time, minimum notice
and policy epoch. Activation must be at least announcement plus notice and
strictly later than the preceding revision's activation; announcement times and
epochs cannot decrease. Overflow and negative times fail. Changes cannot be
inserted ahead of an already announced future revision or cancel/rewrite it.
This deliberately simple queue preserves promises already published.

`Selection::Latest` is for editing the announced queue; `At(now)` returns the
latest effective revision; `Revision(n)` retains the original policy for existing
commitments. `Revision::snapshot` rejects issuance before activation and returns
community, kind=settings, revision, policy epoch, issuance and flat content.
The caller passes that unsigned payload to `csgn`, which supplies a key ID and
signature. Shared snapshots omit member latitude and all proof data.

No API rewrites a deadline already committed by a consuming library. Consumers
must retain that deadline and its revision when making the commitment, and use
new policy only prospectively; `crbk` does not store per-member commitments or
run a scheduler. Authorization, notice delivery, epoch invalidation and clock
trust belong to `cvld`/`cplc`, not to this leaf library.

## Storage and indexes

`Storage` has `load` and atomic `append`, implemented by `MemoryStore` and
`LibsqlStore`. The latter uses the usable `crlt` Git API, never direct libSQL.
The manifest and lockfile pin crlt to the full `main` revision
`6b94dacd7fa04aa8847c62c6471a1fc5c0f6c9dc` (verified 2026-09-30). The former
`9c076b1` pin is its ancestor; no unpublished crlt changes or API adaptations are
required. Consumers must use the same revision to share one database type and
pool. No other cvld leaf is pulled into this crate's dependency graph.
The composition root supplies an opened database and applies `SCHEMA` within
its complete migration list. Use one database per community in deployment;
community keys in every table provide additional isolation, tested with two
communities in a single file. Platform/template input is copied into each
community's configuration; cross-database distribution belongs to the service.

| Table | Content and index |
|---|---|
| `rulebook_revision` | Configuration revision, announcement, activation, notice, epoch; primary key `(community_id, revision)`, unique index `(community_id, effective_at)` |
| `setting_catalog` | Per-setting definition JSON |
| `platform_value` | Value JSON and force switch |
| `template` | Name/version metadata with captured base JSON |
| `template_value` | Sparse per-template setting values |
| `community_template` | Optional explicit name/version pin |
| `community_value` | Sparse community setting values |

The six setting tables use `(community_id, revision, entry_key)` primary keys,
community-scoped foreign keys to the revision, and non-null JSON text payloads.
Explicit null in value tables is text `null`, never SQL NULL. No override means
no row for that entry in the revision. Prior immutable configuration revisions
remain available; these are policy records, never member or request history.
There is no member table. Each revision copies sparse configuration, trading
storage for simple atomic reads and immutable commitments; size is bounded by
the service's catalogue and crlt row/time limits.

Every read shape uses the community/revision or community/effective-time index.
`LibsqlStore::check_query_plans` checks the shapes with crlt's EXPLAIN API; crlt
also rejects unindexed statements at execution. Read transactions preserve a
consistent revision. Errors are static/redacted and do not attach upstream SQL,
credentials, values or subject data. Services must not enable SQL/HTTP dependency
debug tracing for member-facing traffic.

## Verification and integration limits

Stable Rust CI runs formatting, strict all-target Clippy and tests, with a retained
dependency lock. Tests cover both backends, real libSQL files, every read-chain
presence/null combination, exact bounds, force, pinning, policy combinations,
time/identity/provider mismatches, concurrency, notice, rollback, corruption,
namespace isolation, persistence and indexed query plans. The optional Turso
test skips unless both required environment variables are nonempty; no credentials
are put in the repository or CI. No workstation Cargo runs or registry publishing.
CI also checks resolved Cargo metadata for full revision pins and exactly one
package per Corbet dependency; moving branch/tag pins and split revisions fail.

Integration still belongs upstream: authenticated gate/membership adapters,
signature production/verification, delivery of notices and platform revisions,
retention of committed deadlines, and community database routing. This server
crate has no Wasm or browser requirement and performs no remote provider checks.

## Membership catalogue

`define_membership_settings` installs optional cvld settings: one day for new
credentials, thirty days for established credentials, and fourteen probation
days. All use normal platform/community resolution and force rules; member
latitude is forbidden. Credential caps may be shortened and never exceed thirty
days. `MembershipSettings::from_snapshot` rejects missing/null/invalid values.
Consumers commit probation only at first admission and retain no admission date.
