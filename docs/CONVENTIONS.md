# Coding Conventions

Rules for humans and agents writing code in this repo. Read `CLAUDE.md` first
for orientation; this document is the detailed reference.

## Error handling

- **Return `anyhow::Result<T>`** from any fallible function outside of trait
  implementations that dictate a different signature.
- **Attach context** at every propagation point where the error would
  otherwise be ambiguous:

  ```rust
  let tensors = read_stress_tensors_from_file(&path)
      .with_context(|| format!("reading stress tensors from {}", path.display()))?;
  ```

- **Do not return `Result<T, String>`.** It exists in `interpolate.rs` as
  legacy; migrate to `anyhow::Result<T>` when you touch those call sites.
- **Do not use `Box<dyn std::error::Error>`** in new code.
- **No `.unwrap()` / `.expect()` / `panic!` in non-test code.** Exceptions:
  - `debug_assert!` / `assert!` for genuine internal invariants that, if
    violated, indicate a bug rather than bad input.
  - Test code (`#[cfg(test)]` modules, `tests/`, `benches/`).
- **Validate at system boundaries, not inside them.** Parse untrusted input
  (YAML, CSV, JSON, CLI args, WASM inputs) into validated domain types once,
  then trust those types downstream. Range checks on numeric parameters
  (e.g., safety factor ∈ [1.0, 2.0]) belong in `Config::validate`.

## Panics

- The only acceptable panics in non-test code are from `debug_assert!` /
  `assert!` guarding invariants. Even those should be rare.
- If user input can reach a code path, it must not panic — return an error.

## Module layout

- One concept per file. Do not create a new module for a single function;
  add it to the closest existing module.
- `lib.rs` stays thin: module declarations, re-exports, and WASM shims only.
- `main.rs` stays thin: `clap` parsing, then delegate to `app_logic`.
- New I/O belongs in `timeseries.rs` or `config.rs`, not in numerical
  modules (`stress.rs`, `interpolate.rs`, `rainflow.rs`).

## Feature gates

- `#[cfg(feature = "cli")]` — anything that reads files, parses YAML, or uses
  `clap`.
- `#[cfg(feature = "wasm")]` — anything annotated with `#[wasm_bindgen]` or
  that uses `wasm-bindgen` types.
- `#[cfg(any(feature = "cli", feature = "wasm"))]` — numerical kernels that
  both surfaces use (currently `rainflow`, `interpolate`).
- The library must build under `--no-default-features --features wasm`. Test
  this locally when you add or move code across feature boundaries.

## Public API

- Every `pub` item needs a rustdoc comment with a one-line summary.
- Public traits and their required methods need an `# Examples` section with
  compilable doctest code, unless the trait is purely internal plumbing.
- Public top-level functions (those re-exported from `lib.rs`) need at least
  one `# Examples` block.
- Use `#[must_use]` on functions that return values the caller should not
  ignore (e.g., builders, validated configs).

## Naming

- Modules and files: `snake_case`.
- Types (structs, enums, traits): `UpperCamelCase`.
- Functions, methods, fields, locals: `snake_case`.
- Constants and statics: `SCREAMING_SNAKE_CASE`.
- No Hungarian prefixes, no `get_` prefix for simple field accessors.

## Performance

- Hoist compiled regexes and other construction-heavy values into
  `std::sync::LazyLock` (or `once_cell::sync::Lazy` if `LazyLock` is not
  available on the MSRV) instead of rebuilding them in loops.
- Prefer iterators. Only `.collect()` when the allocation is needed
  downstream or when ownership transfer requires it.
- Use `rayon` only when input size and per-item work justify it. Small
  vectors ( ≲ a few hundred cheap ops) should stay sequential.
- Pre-allocate `Vec::with_capacity` / `HashMap::with_capacity` when the size
  is known or can be estimated cheaply.
- Avoid cloning `Point` / tensor data inside parallel closures; restructure
  so the parallel iterator borrows instead.

## Numerical code

- Use `approx::assert_relative_eq!` / `assert_abs_diff_eq!` for float
  equality in tests. Never use `==` on `f64`.
- Guard divisions by values that can be zero. Use `Result` for domain errors
  (zero norm, degenerate matrix) rather than returning `NaN`.
- Prefer `nalgebra` types over raw arrays for linear algebra. Don't invent
  parallel abstractions.

## Dependencies

- No new runtime dependency without explicit user approval. State the need,
  the alternatives considered, and the added binary size before adding.
- `dev-dependencies` for test/bench-only tools (e.g., `criterion`,
  `wasm-bindgen-test`) do not require the same bar but should still be
  justified.
- Keep versions in sync with what's already in `Cargo.toml`; do not mix
  major versions of the same crate.

## Style

- `cargo fmt --all` is source of truth. Do not hand-format.
- `cargo clippy --all-features --all-targets -- -D warnings` must be clean.
  Do not `#[allow(...)]` a lint without a one-line comment explaining why.
- Lines: soft 100-column target; `rustfmt` decides hard cases.
- Comments: only when WHY is non-obvious (hidden invariant, subtle
  constraint, workaround). Do not narrate WHAT the code does.

## Unsafe

- Zero `unsafe` blocks today. Keep it that way.
- If you believe `unsafe` is required, stop and raise it with the user
  before writing the code. Include: what operation, what invariant the
  caller must uphold, why no safe alternative suffices.

## Commits

- Imperative mood, module-scope prefix, ≤72 chars for the subject line:

  ```
  timeseries: hoist sensor-name regex into LazyLock
  config: reject non-numeric mean-correction factor with context
  ```

- One logical change per commit. Refactors go in separate commits from
  behavior changes.
- Do not commit: `target/`, editor settings outside `.vscode/` tracked set,
  generated `pkg/` output from `wasm-pack`, `*.usf` output files.

## Reviews

- Treat unresolved clippy warnings, missing tests, and undocumented `pub`
  items as review blockers.
- Reviewers should run `cargo test` and `cargo clippy --all-features` locally
  before approving non-trivial changes.
