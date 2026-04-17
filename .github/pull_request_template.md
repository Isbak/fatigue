<!--
Thanks for contributing to fatigue. Please fill in the sections below.
For orientation see CLAUDE.md, docs/CONVENTIONS.md, docs/TESTING.md,
docs/ARCHITECTURE.md.
-->

## Summary

<!-- What does this PR do, in 1–3 sentences? Focus on the "why". -->

## Changes

<!-- Bullet list of the concrete changes. Reference files where useful. -->

-
-

## Related issues

<!-- e.g. Closes #123, Refs #456. Leave blank if none. -->

## Type of change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would alter existing behavior)
- [ ] Refactor (no behavior change)
- [ ] Documentation only
- [ ] CI / tooling

## Checklist

### Build and verify

- [ ] `cargo fmt --all -- --check` is clean
- [ ] `cargo clippy --all-features --all-targets -- -D warnings` is clean
- [ ] `cargo test` passes
- [ ] If WASM-gated code was touched:
      `cargo build --no-default-features --features wasm` succeeds

### Code quality

- [ ] No new `.unwrap()`, `.expect()`, or `panic!` in non-test code
- [ ] No new `unsafe` blocks (or: `unsafe` was discussed and approved in
      advance — link the discussion)
- [ ] Fallible functions return `anyhow::Result<T>` with `.context(...)` at
      propagation points
- [ ] Feature gates (`#[cfg(feature = "cli")]`, `#[cfg(feature = "wasm")]`)
      applied correctly to new code
- [ ] Untrusted input (config, sensor files, stress files, CLI args) is
      validated at the boundary, not deep in the call stack

### Tests

- [ ] New public functions have happy-path and error-path tests
- [ ] New `Config::validate` rejection branches have tests
- [ ] New parsers have malformed-input tests asserting the error
- [ ] Float assertions use `approx::assert_relative_eq!` /
      `assert_abs_diff_eq!`, never `==`
- [ ] Fixtures (if any) live under `tests/` with descriptive names

### Documentation

- [ ] New `pub` items have rustdoc comments
- [ ] New top-level library exports and trait methods have `# Examples`
- [ ] If architecture changed: `docs/ARCHITECTURE.md` updated
- [ ] If conventions changed: `docs/CONVENTIONS.md` updated
- [ ] If the command set or project layout changed: `CLAUDE.md` and
      `README.md` updated

## Test plan

<!--
How was this verified? Include concrete steps a reviewer can run.
Example:
- `cargo test interpolate::linear` — covers the new SVD path
- Ran `cargo run -- --run tests/config.yaml` and confirmed output matches
  the expected JSON shape
- Benchmarked with `cargo bench bench_linear_interpolation` — no
  regression (±3 %)
-->

-
-

## Screenshots / output

<!-- Paste CLI output, benchmark numbers, or before/after diffs if relevant. -->

## Notes for reviewers

<!--
Anything you want reviewers to pay special attention to: tradeoffs you
made, alternatives you considered and rejected, follow-up work you plan
to do in a later PR.
-->
