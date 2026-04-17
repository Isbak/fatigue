# CLAUDE.md

Primary agent entrypoint for the `fatigue` crate. Read this first. For deeper
context see `docs/CONVENTIONS.md` and `docs/TESTING.md`.

## What this project is

Safe, fast structural fatigue assessment tool written in Rust. Hybrid crate:

- **Library** (`rlib`) — reusable numerical kernels (rainflow counting,
  interpolation, stress tensor ops).
- **CLI binary** (`fatigue`) — gated by the `cli` feature (default).
- **WebAssembly** (`cdylib`) — gated by the `wasm` feature, exposes a subset
  of the library to JavaScript via `wasm-bindgen`.

Version: `0.1.0`. Edition: `2021`. Pre-1.0, APIs may change.

## Module map (`src/`)

| File             | Responsibility                                                       |
| ---------------- | -------------------------------------------------------------------- |
| `lib.rs`         | Library root, module declarations, WASM shims. Keep thin.            |
| `main.rs`        | CLI entrypoint. `clap` arg parsing, delegates to `app_logic`.        |
| `app_logic.rs`   | CLI orchestration (load config → run pipeline → write output).       |
| `config.rs`      | YAML config structs, parsing, validation. `cli` feature only.        |
| `material.rs`    | Material properties, fatigue parameters. `cli` feature only.         |
| `stress.rs`      | Stress tensors, principal stresses, Von Mises. `cli` feature only.   |
| `timeseries.rs`  | Time series loading (CSV/JSON), sensor data, interpolation plumbing. |
| `interpolate.rs` | `InterpolationStrategy` trait + `Linear` / `NearestNeighbor` impls.  |
| `rainflow.rs`    | Rainflow cycle counting. Available under both `cli` and `wasm`.      |

Fixtures live in `tests/` (YAML config, `.usf` stress files, CSV/JSON sensor
data). Benchmarks live in `benches/benchmark.rs`.

## Commands

```bash
# Build
cargo build                                              # CLI (default)
cargo build --no-default-features --features wasm        # WASM only

# Test — must pass before any commit
cargo test

# Lint — must be clean before any commit
cargo clippy --all-features --all-targets -- -D warnings

# Format — must be clean before any commit
cargo fmt --all -- --check

# Bench (optional, local)
cargo bench

# WASM package (requires wasm-pack installed)
wasm-pack build --target web -- --no-default-features --features wasm
```

## Non-negotiables

1. **No `unsafe`.** The codebase has zero `unsafe` blocks today. If one seems
   necessary, stop and ask the user before introducing it.
2. **No `.unwrap()` / `.expect()` / `panic!` in non-test code.** Use
   `anyhow::Result<T>` and the `?` operator. Attach context with
   `.context("what was being attempted")`. Existing violations are tracked as
   tech debt — do not add new ones.
3. **Feature gates are load-bearing.** Code that touches the filesystem, YAML
   config, or CLI must be under `#[cfg(feature = "cli")]`. Code exposed to
   JavaScript must be under `#[cfg(feature = "wasm")]`. The library must
   compile under `--no-default-features` with only `wasm`.
4. **Don't break the WASM build.** The WASM surface is small (currently
   `run_rainflow` in `lib.rs`); if you extend it, gate it and add a
   `wasm-bindgen-test`.
5. **Validate untrusted input at boundaries.** YAML configs, sensor files,
   stress-tensor files, and CLI args are untrusted. Validate once on ingest;
   trust internal callers after that.
6. **No new dependencies without justification.** Prefer standard library and
   existing deps (nalgebra, rayon, serde, anyhow, clap) over pulling in
   something new.

## Before finishing a task

Run, in order, from repo root:

```bash
cargo fmt --all -- --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test
```

All three must pass. If you touched WASM-gated code:

```bash
cargo build --no-default-features --features wasm
```

Also must succeed.

## Task patterns

- **Bug fix** — write a failing test first, fix, confirm it passes, confirm
  no other tests regressed.
- **New feature** — thread it through `config.rs` (if user-configurable),
  implement in the appropriate module, add unit tests in that module's
  `#[cfg(test)]` block, add an integration fixture under `tests/` if it
  affects end-to-end behavior.
- **Refactor** — keep it tightly scoped to the task at hand. Do not mix
  refactoring with behavior changes in the same commit.

## Known tech debt (safe to fix when touching nearby code)

- `.unwrap()` calls in `config.rs:238` and `timeseries.rs:361/381/410/412`.
- Silent JSON parse failure returning `Vec::new()` in `timeseries.rs:228–233`.
- Dead code: `app_logic::run`, `TimeSeries::interpolate`.
- `Regex::new(...).unwrap()` in a validation loop at `timeseries.rs:267` —
  hoist to a `LazyLock` static.
- `Result<_, String>` in `interpolate.rs` — migrate to `anyhow::Result` when
  touched.

## Commit style

Imperative mood, module-scope prefix:

```
timeseries: propagate JSON parse errors instead of swallowing
config: replace unwrap on f64 parse with context-bearing error
ci: add clippy -D warnings and fmt --check
```

One logical change per commit. Do not commit generated files, `target/`, or
editor scratch.
