#!/usr/bin/env bash
set -euo pipefail
: "${CI:?CI only}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export CARGO_HTTP_USER_AGENT=rulebook-ci
rustc --version
cargo --version
sha256sum Cargo.lock
python3 .github/check-first-party.py --archive
cargo fmt --all --check
node --test .github/check-pins.test.mjs
cargo metadata --locked --all-features --format-version 1 > dependency-metadata.json
node .github/check-pins.mjs dependency-metadata.json
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --no-fail-fast
cargo clippy --locked --all-targets --no-default-features -- -D warnings
cargo test --locked --no-default-features --no-fail-fast
