# Migration Guide: 0.1 -> 0.2

This release is intentionally API-breaking.

## Module Renames

- `rust_pbt::numeric` -> `rust_pbt::generators::numeric` and `rust_pbt::law::invariants`
- `rust_pbt::collections` -> `rust_pbt::generators::collections`
- `rust_pbt::arithmetic` -> `rust_pbt::law::algebraic`
- `rust_pbt::stateful` -> `rust_pbt::law::stateful`
- `rust_pbt::serialization` -> `rust_pbt::law::serialization`

## Domain Modules Removed From Core

`kitchensink-testing` no longer ships built-in domain packs.

If you previously used domain helpers (`financial`, `trading`, `options`), define them in:

- the crate under test (`src/pbt.rs`, `tests/`), or
- companion crates (e.g. `kitchensink-testing-finance`, `kitchensink-testing-trading`).

## Prelude Changes

`rust_pbt::prelude::*` now exports a curated subset from:

- `generators`
- `law`
- mutation core types when `mutation` feature is enabled

## Mutation Testing

New workflow uses resumable run state:

- `cargo pbt mutate run`
- `cargo pbt mutate resume <run-id>`
- `cargo pbt mutate status <run-id>`
- `cargo pbt mutate report <run-id>`
