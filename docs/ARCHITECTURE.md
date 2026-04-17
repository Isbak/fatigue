# Architecture

How the `fatigue` crate is organized and how data flows through it. Read
`CLAUDE.md` first for orientation.

## Crate shape

```
fatigue/
├── src/
│   ├── lib.rs          rlib + cdylib root; re-exports; WASM shims
│   ├── main.rs         CLI binary entrypoint (clap)
│   ├── app_logic.rs    CLI orchestration
│   ├── config.rs       YAML config + validation
│   ├── material.rs     Material + SN curve parameters
│   ├── stress.rs       Stress tensors, principal stresses, Von Mises
│   ├── timeseries.rs   Sensor/timeseries loading, interpolation plumbing
│   ├── interpolate.rs  InterpolationStrategy trait + impls
│   └── rainflow.rs     Rainflow cycle counting
├── benches/benchmark.rs    Criterion benches
└── tests/                   fixtures only (no .rs integration tests yet)
    ├── config.yaml
    ├── stressfile/*.usf
    └── timeseries/*.csv, sensors.json
```

Three build artifacts come out of one source tree:

| Artifact        | Feature flags                         | Produced by        |
| --------------- | ------------------------------------- | ------------------ |
| CLI binary      | `cli` (default)                       | `src/main.rs`      |
| Rust library    | any                                   | `src/lib.rs` rlib  |
| WebAssembly lib | `wasm` (no `cli`)                     | `src/lib.rs` cdylib |

The feature matrix is load-bearing: `cli`-gated modules (`config`, `stress`,
`material`, `timeseries`, `app_logic`) assume filesystem access and YAML.
`wasm`-gated code assumes no filesystem and uses `wasm-bindgen` types.

## Data flow (CLI pipeline)

```
  ┌──────────────────┐
  │  config.yaml     │   untrusted input
  └────────┬─────────┘
           │  config::load_config
           ▼
  ┌──────────────────┐
  │  Config          │   validated domain type
  │  ├─ solution     │     (Config::validate enforces ranges)
  │  ├─ material     │
  │  ├─ safety_factor│
  │  └─ timeseries   │
  └────────┬─────────┘
           │  TimeSeries::parse_input
           ▼
  ┌──────────────────┐
  │  Sensor data     │   CSV time series + JSON sensor metadata
  │  (Vec<Point>,    │
  │   per-loadcase   │
  │   signals)       │
  └────────┬─────────┘
           │  for each interpolation point: read .usf
           ▼
  ┌──────────────────┐       ┌────────────────────────────────┐
  │  Stress tensors  │◄──────│  interpolate.rs                │
  │  at target points│       │  InterpolationStrategy::       │
  │                  │       │    interpolate(points, target) │
  └────────┬─────────┘       │  - Linear (SVD regression)     │
           │                 │  - NearestNeighbor (parallel)  │
           │                 └────────────────────────────────┘
           │  stress::principal_stresses / von_mises
           ▼
  ┌──────────────────┐
  │  Scalar stress   │   one f64 per time step
  │  time history    │
  └────────┬─────────┘
           │  rainflow::rainflow
           ▼
  ┌──────────────────┐
  │  (means, ranges) │   cycle histogram
  └────────┬─────────┘
           │  (future: apply SN curve + Miner's rule from material.rs)
           ▼
       Damage
```

The WASM surface today is smaller — only `run_rainflow(&[f64]) -> Vec<f64>`
in `lib.rs`. It skips the config/I/O layers entirely; the host (JavaScript)
supplies the stress history directly.

## Module responsibilities

### `lib.rs`

- Declare feature-gated modules.
- Re-export the stable library surface: `InterpolationStrategy`, `Linear`,
  `NDInterpolation`.
- Host WASM shims (`#[wasm_bindgen]` functions). Shims must not contain
  business logic — delegate to internal modules.

Keep it thin. If `lib.rs` grows past a screen, move logic into a module.

### `main.rs`

- `clap` argument parsing (currently builder API with `--run`, `--mode`,
  `--rainflow`).
- Dispatch to `app_logic`. No business logic here.

### `app_logic.rs`

- Top-level CLI flow: load config → validate → drive pipeline → print or
  write output.
- This is the only module that should know about the full pipeline shape.
  Individual steps live in their own modules.

### `config.rs`

- Serde-derived structs mirroring the YAML schema (`solution`, `material`,
  `safety_factor`, `timeseries` sections — see `tests/config.yaml` for the
  canonical example).
- `Config::validate` enforces numeric ranges (safety factors, mean
  correction factor, etc.) and cross-field constraints.
- Owns YAML parsing; do not parse YAML elsewhere.

### `material.rs`

- Material properties (Young's modulus, Poisson's ratio, yield, ultimate).
- SN curve parameters (two-slope with knee point, cutoff bounds).
- Pure data + simple derived values. No I/O.

### `stress.rs`

- Stress tensor struct, construction from file (`.usf`), update ops.
- Principal stress computation (via `nalgebra` eigendecomposition).
- Von Mises scalar.
- Unit vector normalization for direction of principal stress.

### `timeseries.rs`

- Parse sensor definitions (`sensors.json`).
- Parse CSV time-series files (one per loadcase).
- Build `Point` structs (coordinates + metadata) that feed
  `interpolate::InterpolationStrategy`.
- Glue between config (where paths come from) and numerical kernels (which
  receive preprocessed data).

### `interpolate.rs`

- Extension point: the `InterpolationStrategy` trait.

  ```rust
  pub trait InterpolationStrategy {
      fn interpolate(
          &self,
          points: &HashMap<Point, f64>,
          target: &Vec<Vec<f64>>,
      ) -> Result<Vec<f64>, String>;
  }
  ```

- `Linear` — multivariate linear regression via SVD (handles
  arbitrary-dimensional input points).
- `NearestNeighbor` — parallel search via `rayon`; Euclidean distance.

  To add a strategy: implement the trait, parallelize over `target` with
  `rayon` if work per target is non-trivial, return
  `Err("reason".to_string())` on degenerate input. When the error type is
  migrated to `anyhow::Result`, update all impls together.

- The `Result<_, String>` return type is legacy; `CONVENTIONS.md` calls for
  migrating it to `anyhow::Result` when touched.

### `rainflow.rs`

- ASTM E1049-85 rainflow cycle counting.
- `VecDeque`-based implementation; extracts reversals, counts full cycles,
  handles residuals as half cycles.
- Returns `(means, ranges)` in parallel vectors.
- Available under both `cli` and `wasm`.

## Cross-cutting concerns

### Error handling

- **Target model:** `anyhow::Result<T>` with `.context(...)` at every
  propagation point where the caller needs to know what was being attempted.
- **Current state (tech debt):** some modules still use
  `Box<dyn std::error::Error>` (`app_logic::run`) or
  `Result<_, String>` (`interpolate`). Migrate when touching.
- Validation errors (range violations, schema mismatches) are returned from
  `Config::validate`, not panicked.

### Parallelism

- `rayon` is used inside `interpolate.rs` across target points.
- Parallelism is currently scoped to interpolation. Do not introduce
  `rayon` elsewhere without a benchmark showing it helps for realistic
  input sizes.
- Thread pool is the global `rayon` pool; no custom pools.

### Numerics

- `nalgebra` provides `DMatrix`/`DVector` and SVD. Do not roll custom
  linear algebra.
- All float equality in tests uses `approx`; see `docs/TESTING.md`.
- Guard against division by zero and degenerate matrices in numerical
  kernels — surface as errors, do not return `NaN`.

### I/O boundaries

- File I/O is confined to `config.rs` (YAML), `timeseries.rs` (CSV/JSON),
  and `stress.rs` (`.usf` stress tensor files). Do not read files from
  `interpolate.rs`, `rainflow.rs`, `material.rs`, or `stress.rs` kernels.
- Paths come from the validated `Config` — never from environment
  variables, never hardcoded.

## Design decisions on record

- **Why `anyhow` instead of `thiserror`.** `fatigue` is primarily an
  application (CLI), not a library with a stable public error API.
  `anyhow` minimizes boilerplate and composes cleanly with `?`. Internal
  kernels that become stable library surface may migrate to `thiserror`
  later.
- **Why SVD for multivariate linear interpolation.** Handles
  arbitrary-dimensional input without bespoke per-dimension code, tolerates
  rank deficiency gracefully, and `nalgebra` provides it out of the box.
- **Why `VecDeque` for rainflow.** The algorithm inspects a sliding window
  of three reversals and pops from the front; `VecDeque` gives O(1) on
  both ends without allocation churn.
- **Why `rayon` only in interpolation.** Interpolation is the measurable
  hot spot (see `benches/benchmark.rs`); rainflow is O(n) with tiny
  per-element work and doesn't benefit. Premature parallelism elsewhere
  would add overhead without gain.
- **Why two crate types (`rlib` + `cdylib`).** One source tree serves both
  Rust consumers and the browser-facing WASM bundle. Feature gates keep
  the WASM surface minimal.
- **Why `default = ["cli"]`.** The common consumer is `cargo install
  fatigue`; defaulting to CLI avoids friction. WASM users opt in with
  `--no-default-features --features wasm`.

## Out of scope (do not add without discussion)

- Graphical user interface.
- Network I/O, HTTP clients, remote config loading.
- Persistent storage (databases, caches).
- Plugin systems / dynamic loading.
- Alternative async runtimes.

## Extension points

When adding a feature, prefer extending these existing seams over creating
new top-level modules:

- **New interpolation method** → `impl InterpolationStrategy` in
  `interpolate.rs`.
- **New stress criterion** → method on the stress tensor type in
  `stress.rs`.
- **New config field** → add to the appropriate section struct in
  `config.rs`, extend `Config::validate`, add a fixture in `tests/`.
- **New output format** → dispatch in `app_logic.rs` based on
  `solution.output` from the config.
- **New WASM-exported function** → `#[cfg(feature = "wasm")]
  #[wasm_bindgen]` shim in `lib.rs`, delegates to an internal module.

If a feature doesn't fit any of these, raise it in review before writing
code — it may indicate a missing abstraction, or scope creep.
