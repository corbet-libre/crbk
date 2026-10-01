# Coverage contract

CI targets 100% of reachable production lines and branches. Stable Rust runs
the existing native and wasm checks. Nightly Rust is used only for LLVM branch
instrumentation, which currently requires it. Both jobs use the same resolved
Cargo.lock snapshot; all build and test commands after resolution use --locked.

The coverage job executes real tests with cargo-llvm-cov and retains the raw JSON
even when the gate fails. The checker compares integer covered/total counts for
both metrics; rounded percentages cannot pass. An empty report cannot pass.
The report excludes integration-test harness files under tests/, not production
code. No production source exclusions are currently approved.

A failing gate is missing evidence, not permission to lower the threshold or
change domain behavior. Add meaningful failure and round-trip tests. Document
any genuinely unreachable defensive branch precisely before excluding it. Native
coverage does not establish browser execution; keep the actual wasm vectors.

First-party dependencies follow main. Their resolved full revisions remain in
Cargo.lock, with exactly one source per first-party crate. Dependabot maintains
committed snapshots; CI refreshes once per run and retains the tested snapshot.
Auto-merge requires protected main and successful substantive checks on the
exact current Dependabot head. It never executes PR code with write permissions.

## Emitted source metric

The acceptance metric is now 100% of upstream LLVM LCOV's emitted production
source line counters (DA) and branch counters (BRDA), from the same execution as
the retained raw JSON. This is source coverage, not every generic instantiation.
LLVM JSON and LCOV summary totals can count generic copies differently from
the merged source records. The original JSON checker remains diagnostic code;
its stricter instantiation totals are not described as passed.

The source gate requires a nonempty report, an exact file inventory matching the
companion JSON, matching raw summary metadata, complete emitted branch counts
and no duplicate, unknown or zero counters. It applies only the exact source-bound proposal documented below; all branch
arms remain required.
Actual browser/device execution remains separate from native coverage.

## Validated template pin

One defensive error-propagation line is proposed for independent review:
`src/settings.rs:352`, `.transpose()?;`, in `Rulebook::resolve`. The method
first calls `self.validate()`. That validates the same immutable template pin
through `self.template(pin)` at lines398–399. The second lookup has no await,
interior mutability or intervening mutation, and therefore cannot return an
error after validation succeeds. The original guard remains intact.

`deserialized_template_versions_and_pins_are_validated` exercises an imported
missing version and rejects it in validation; existing sparse resolver vectors
exercise valid pinned templates. Native and actual Node Wasm execute these
vectors. The exact source line and the entire settings source hash are bound
in `.github/coverage-exclusions.json`; moved, changed or executed exclusions
fail the guard. No function, file or branch arm is excluded.

Before this proposal, run36920897837 emitted685/686 lines and184/184 branches.
The retained JSON, LCOV and annotated text inventories are checked completely
before exclusions. The separate raw generic totals remain unchanged. This
proposal does not constitute the mandatory independent security acceptance.
