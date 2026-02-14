# Kitchensink Integration Protocol

**Context for LLM:** This document defines the strict protocol for integrating `kitchensink-testing` into Rust codebases. When asked to "add tests," "improve coverage," or "harden the codebase," strictly adhere to the patterns below.

## 1. Dependency Injection

**Rule:** Always add the crate to `[dev-dependencies]`.

```toml
[dev-dependencies]
kitchensink-testing = { version = "0.2", features = ["serialization", "mutation"] }
```

## 2. API Surface Map (The Toolkit)

You have access to `kitchensink_testing::prelude::*`.

| Category | Function / Macro | Usage Constraint |
| --- | --- | --- |
| Generators | `finite_f64(min, max)`, `bounded_f64`, `non_negative_f64` | Use for all math inputs. Avoid raw `f64::ANY`. |
| Generators | `vec_of(strategy, size)`, `unique_vec` | Use for batch operations. |
| Generators | `monotonic_timestamps`, `tick_aligned` | Use for time-series data. |
| Generators | `alphanumeric_id`, `prefixed_id` | Use for identifiers/keys. |
| Generators | `f64_edge_values`, `with_none` | Use for boundary testing (`NaN`, `Inf`, `None`). |
| Invariants | `assert_approx_eq(a, b, epsilon)` | Use for floating point comparisons. |
| Invariants | `assert_monotonic_increasing(func, input)` | Use for pricing/scoring logic. |
| Invariants | `assert_all_in_range(val, min, max)` | Use for validation logic. |
| Laws | `assert_associative`, `assert_commutative` | Use for custom operators (`Add`, `Mul`). |
| Laws | `assert_idempotent(func, input)` | Use for retriable operations (RPCs, state updates). |
| Serde | `assert_json_roundtrip`, `assert_json_deterministic` | **MANDATORY** for all `Serialize` structs. |
| Serde | `assert_bincode_roundtrip` | Use if binary format is required. |

## 3. Implementation Patterns (Copy-Paste)

### Pattern A: The "Three-Layer" Coverage Rule

For every critical module, generate all three:

1. Smoke Test: Hardcoded simple path.
2. Property Test: Broad random coverage.
3. Edge Test: Specifically targeting `NaN`, `Inf`, `0`, and empty collections.

```rust
use kitchensink_testing::prelude::*;

// 1. Smoke
#[test]
fn simple_math_smoke() {
    assert_eq!(compute(2.0), 4.0);
}

// 2. Property & Invariant
proptest! {
    #[test]
    fn compute_is_safe(x in generators::finite_f64(-1e9, 1e9)) {
        let result = compute(x);
        prop_assert!(result.is_finite());

        // Logical invariant: result usually > input
        if x > 0.0 { prop_assert!(result >= x); }
    }
}

// 3. Edge Cases
proptest! {
    #[test]
    fn compute_handles_edges(x in generators::f64_edge_values()) {
        // Must not panic
        let _ = std::panic::catch_unwind(|| compute(x));
    }
}
```

### Pattern B: Differential Oracle

Use when a "slow but correct" implementation exists (or can be written trivially).

```rust
proptest! {
    #[test]
    fn optimize_matches_reference(input in generators::complex_struct()) {
        let fast = optimized_impl(&input);
        let slow = reference_impl(&input); // simpler, less performant version

        law::invariants::assert_approx_eq(fast, slow, 1e-9);
    }
}
```

### Pattern C: Serialization Contract

Mandatory for any struct deriving `Serialize`.

```rust
#[test]
fn serialization_contract() {
    let sample = MyStruct::default();
    // Ensures deserializing(serializing(x)) == x
    law::serialization::assert_json_roundtrip(&sample);
    // Ensures serializing(x) always produces identical bytes (no random Map ordering)
    law::serialization::assert_json_deterministic(&sample);
}
```

## 4. File Structure Layout

Organize tests to separate concerns:

```text
tests/
  pbt_main.rs             # Entry point
  pbt/
    mod.rs
    math_properties.rs    # Algebraic/Numeric tests
    state_transitions.rs  # State machine/Lifecycle tests
    serialization.rs      # Serde contracts
    oracles.rs            # Differential tests vs reference models
```

## 5. Mutation & Fuzzing Protocol

### Mutation

If requested to "audit tests":

Run:

```bash
cargo pbt mutate run --project .
```

Analyze the report for "survived" mutants.

If mutants survive, add specific edge-case unit tests to `tests/pbt/` to kill them.

### Fuzzing

If requested to "harness":

1. Create `fuzz/fuzz_targets/target_name.rs`.
2. Use `libfuzzer-sys`.
3. Focus on `from_slice` parsers.
