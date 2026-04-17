# Testing Guide

How to test changes in this repo. Read `CLAUDE.md` for project orientation
and `docs/CONVENTIONS.md` for style rules.

## Layers

### 1. Unit tests (in-module)

Live in `#[cfg(test)] mod tests { ... }` at the bottom of each source file.
This is the default location for tests — use it for anything that exercises
a single module's logic.

```rust
// src/rainflow.rs
#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn zero_signal_produces_no_cycles() {
        let (means, ranges) = rainflow(&[0.0; 16]);
        assert!(means.is_empty());
        assert!(ranges.is_empty());
    }
}
```

Every new public function gets at least:

- one happy-path test asserting a known output,
- one error-path test asserting the error kind or message.

Error-path tests must check the error, not just `is_err()`. Use
`anyhow::Error::downcast_ref` or assert on `format!("{err:#}")` content.

### 2. Integration tests (`tests/*.rs`)

Currently absent — `tests/` today holds **fixtures**, not tests. When
end-to-end behavior needs coverage, add Rust files at the top level of
`tests/` that drive the crate through its public API:

```
tests/
├── config.yaml                       # existing fixture
├── stressfile/                       # existing fixtures
├── timeseries/                       # existing fixtures
└── end_to_end.rs                     # new integration test
```

Each `tests/*.rs` file is compiled as a separate crate with only the public
API of `fatigue` available. Use relative paths into `tests/` for fixtures:

```rust
// tests/end_to_end.rs
use std::path::PathBuf;

#[test]
fn loads_example_config_and_runs_pipeline() {
    let config_path: PathBuf = ["tests", "config.yaml"].iter().collect();
    let cfg = fatigue::config::Config::from_path(&config_path)
        .expect("example config should load");
    cfg.validate().expect("example config should validate");
    // drive further pipeline steps here
}
```

### 3. WASM tests

The `wasm` feature exposes `run_rainflow` (and whatever else is added). Add
`wasm-bindgen-test` coverage whenever you extend the WASM surface:

```toml
# Cargo.toml (dev-dependencies, when first WASM test is added)
wasm-bindgen-test = "0.3"
```

```rust
// tests/wasm.rs (or module under src/ gated on cfg(target_arch = "wasm32"))
#![cfg(all(target_arch = "wasm32", feature = "wasm"))]

use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn run_rainflow_returns_means_then_ranges() {
    let out = fatigue::run_rainflow(&[0.0, 1.0, -1.0, 0.5, -0.5, 0.0]);
    assert!(!out.is_empty());
}
```

Run with:

```bash
wasm-pack test --headless --firefox --no-default-features --features wasm
```

### 4. Benchmarks (`benches/`)

Criterion-based. Add an entry when introducing a new hot path or changing
an existing one. Benchmarks are not a substitute for tests — a benchmarked
function still needs unit tests.

```bash
cargo bench
```

Benchmarks should not gate CI (variance and hardware sensitivity), but
regressions > 10 % on the rainflow or interpolation benches warrant
investigation.

## Fixtures

- All test input lives under `tests/`. Do not create fixtures in `target/`,
  `/tmp/`, or absolute paths.
- Reference fixtures with `PathBuf` built from segment arrays, not string
  concatenation, so tests pass on non-Unix.
- Keep fixtures minimal — the smallest input that exercises the code path.
  If a new fixture duplicates an existing one with minor tweaks, extend the
  existing one instead.
- Name fixtures by what they test, not by sequence number. Prefer
  `config_missing_material.yaml` over `config_7.yaml`.

## Floating-point assertions

Never compare `f64` with `==`. Use `approx`:

```rust
use approx::{assert_relative_eq, assert_abs_diff_eq};

assert_relative_eq!(computed, expected, max_relative = 1e-9);
assert_abs_diff_eq!(near_zero, 0.0, epsilon = 1e-12);
```

Choose `max_relative` for values spanning many magnitudes, `epsilon` for
values near zero.

## Determinism

- Seed any `rand` usage in tests with a fixed seed via `StdRng::seed_from_u64`.
- Do not rely on `HashMap` iteration order. Collect into a sorted `Vec` before
  asserting.
- Do not use wall-clock time in assertions.

## Running subsets

```bash
cargo test                               # all tests
cargo test --lib                         # unit tests only
cargo test --test end_to_end             # one integration file
cargo test interpolate                   # path filter — all tests whose
                                         # name contains "interpolate"
cargo test -- --nocapture                # show println! output
cargo test -- --test-threads=1           # serial, useful for file-I/O tests
```

## Feature-matrix testing

Before merging changes that touch feature-gated code, verify all three
builds:

```bash
cargo test                                                 # default (cli)
cargo test --no-default-features --features wasm           # wasm only
cargo test --all-features                                  # both
```

If any of these fail to compile, the `#[cfg(...)]` guards are wrong.

## Coverage expectations

Not a hard percentage. The bar is:

- Every public function has at least one test.
- Every `Config::validate` branch (each rejection reason) has a test.
- Every parser (`timeseries.rs`, `stress.rs` file readers) has a malformed-
  input test asserting the error, not just success cases.
- Every new `InterpolationStrategy` impl has unit tests + at least one
  bench entry.

## When a test is hard to write

That is usually a design signal. Options, in order of preference:

1. Extract the logic into a pure function that takes data, not paths.
2. Introduce a small trait for the I/O boundary and test with an in-memory
   impl.
3. Only as a last resort, write a fixture-driven integration test.

Do not add mocking frameworks. Hand-rolled test doubles are fine.

## CI

CI runs `cargo test`, `cargo clippy --all-features -- -D warnings`, and
`cargo fmt --all -- --check`. A red CI blocks merge. Fix the underlying
issue — do not skip hooks or add `#[ignore]` without an accompanying issue
link in a comment above the attribute.
