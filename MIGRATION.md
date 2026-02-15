# Migration Guide: 0.1 -> 0.2

This release is intentionally API-breaking.

## Module Renames

- `kitchensink_testing::numeric` -> `kitchensink_testing::generators::numeric` and `kitchensink_testing::law::invariants`
- `kitchensink_testing::collections` -> `kitchensink_testing::generators::collections`
- `kitchensink_testing::arithmetic` -> `kitchensink_testing::law::algebraic`
- `kitchensink_testing::stateful` -> `kitchensink_testing::law::stateful`
- `kitchensink_testing::serialization` -> `kitchensink_testing::law::serialization`

## Domain Modules Removed From Core

`kitchensink-testing` no longer ships built-in domain packs.

If you previously used domain helpers (`financial`, `trading`, `options`), define them in:

- the crate under test (`src/pbt.rs`, `tests/`), or
- companion crates (e.g. `kitchensink-testing-finance`, `kitchensink-testing-trading`).

## Prelude Changes

`kitchensink_testing::prelude::*` now exports a curated subset from:

- `generators`
- `law`
- mutation core types when `mutation` feature is enabled

## Mutation Testing

New workflow uses resumable run state:

- `cargo kitchensink mutate run` (auto-resumes latest interrupted run; if latest compatible run is complete but has survivors, re-tests survivors first)
- `cargo kitchensink mutate resume <run-id>`
- `cargo kitchensink mutate survivors <run-id>` (re-runs only survivors from a specific run)
- `cargo kitchensink mutate status <run-id>`
- `cargo kitchensink mutate report <run-id>`
