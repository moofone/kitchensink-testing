# kitchensink-testing

[![Crate](https://img.shields.io/badge/crate-kitchensink--testing-blue)]()
[![Features](https://img.shields.io/badge/features-mutation%20%7C%20properties%20%7C%20fuzzing-green)]()

Reusable testing infrastructure for high-assurance Rust. This crate unifies Property-Based Testing (PBT), Mutation Testing, and Fuzzing into a single workflow.

## Feature Matrix

| Surface            | Purpose                                                                | Status       |
| :----------------- | :--------------------------------------------------------------------- | :----------- |
| **Generators**     | Domain-specific input strategies (tick-aligned, monotonic, edge-heavy) | ✅ Available |
| **Algebraic Laws** | Verify math properties (commutativity, associativity, identity)        | ✅ Available |
| **Invariant Laws** | Verify logic bounds (monotonicity, range, approximations)              | ✅ Available |
| **Stateful Laws**  | Verify state machines (transitions, idempotence, sequence validity)    | ✅ Available |
| **Serialization**  | Guarantee JSON/Bincode roundtrip stability and determinism             | ✅ Available |
| **Chaos / Retry**  | Validate transient retry, permanent failures, and fallback behavior      | ✅ Available |
| **Tokio Laws**     | Trait-driven task/time/sync/io concurrency contracts for Tokio systems   | ✅ Optional  |
| **Tokio + Loom**   | Exhaustive schedule checking adapter for selected Tokio concurrency kernels | ✅ Optional  |
| **Mutation**       | Orchestrate `cargo kitchensink` mutation runs to find gaps in test logic | ✅ Available |
| **Fuzzing**        | Harness support for `libfuzzer-sys`                                    | 🚧 Optional  |

## Installation

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
kitchensink-testing = "0.2"
# Enable Tokio law surface:
# kitchensink-testing = { version = "0.2", features = ["tokio-laws"] }
# Enable Loom adapter on top:
# kitchensink-testing = { version = "0.2", features = ["tokio-laws", "tokio-loom"] }
```

Install the CLI binary:

```bash
cargo install --locked --path . --force --bin cargo-kitchensink

# Install mutation engine dependency for `cargo kitchensink mutate` workflows:
cargo install cargo-mutants
```

Run mutation workflows with either:

```bash
cargo kitchensink mutate run --project .
cargo-kitchensink mutate run --project .
```

The direct `cargo-kitchensink ...` form is equivalent; use whichever works in your environment.

`mutate run` now auto-resumes the latest incomplete run for the same `--project`/`--run-root`. If the latest compatible run is already completed but still has survivors, it re-tests those survivors automatically (and on resume, survivors are scheduled before pending mutants). Use `mutate resume <run-id>` to target a specific run explicitly.

To rerun only survivors from a specific run id, use:

```bash
cargo kitchensink mutate survivors <run-id> --project .
```
