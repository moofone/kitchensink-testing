# rust-pbt

A comprehensive property-based testing framework for Rust with custom strategies, property templates, and reference implementation oracles.

## Features

- **Custom Strategies**: Domain-specific `proptest` strategies for prices, timestamps, candles, orders
- **Property Templates**: Reusable assertion patterns (commutative, associative, idempotent, etc.)
- **Reference Implementations**: Slow-but-correct oracles for testing optimized code
- **Feature-gated**: Only include what you need

## Installation

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
rust-pbt = "0.1"
```

### Feature Flags

```toml
# Minimal (default): numeric, collections, arithmetic, stateful
rust-pbt = "0.1"

# Financial domain
rust-pbt = { version = "0.1", features = ["financial"] }

# Serialization testing
rust-pbt = { version = "0.1", features = ["serialization"] }

# Temporal/timestamp strategies
rust-pbt = { version = "0.1", features = ["temporal"] }

# Trading domain (candles, orders) - requires temporal and financial
rust-pbt = { version = "0.1", features = ["trading"] }

# Everything
rust-pbt = { version = "0.1", features = ["full"] }
```

## Quick Start

### Basic Properties

```rust
use rust_pbt::prelude::*;

proptest! {
    #[test]
    fn addition_is_commutative(a in 0..100i32, b in 0..100i32) {
        assert_commutative(a, b, |x, y| x + y);
    }

    #[test]
    fn multiplication_is_associative(a in 1..10i32, b in 1..10i32, c in 1..10i32) {
        assert_associative(a, b, c, |x, y| x * y);
    }
}
```

### Numeric Properties

```rust
use rust_pbt::numeric::{assert_bounded, assert_finite, assert_non_negative};

#[test]
fn test_calculation() {
    let result = calculate_something();
    assert_finite(result);
    assert_non_negative(result);
    assert_bounded(result, 0.0, 100.0);
}
```

### Collection Properties

```rust
use rust_pbt::collections::{assert_sorted, assert_no_duplicates};

#[test]
fn test_sorting() {
    let sorted = my_sort_function(&[3, 1, 2]);
    assert_sorted(&sorted);
    assert_no_duplicates(&sorted);
}
```

### Financial Domain

```rust
use rust_pbt::prelude::*;

proptest! {
    #[test]
    fn pnl_sign_correct(
        entry in valid_price(),
        exit in valid_price(),
        qty in 0.01..100.0f64
    ) {
        let pnl = (exit - entry) * qty;
        assert_pnl_sign_correct(entry, exit, qty, true, pnl);
    }
}
```

### Trading Domain

```rust
use rust_pbt::trading::{valid_candle, Candle};

proptest! {
    #[test]
    fn candle_ohlc_constraints(candle in valid_candle()) {
        prop_assert!(candle.high >= candle.open);
        prop_assert!(candle.high >= candle.close);
        prop_assert!(candle.low <= candle.open);
        prop_assert!(candle.low <= candle.close);
    }
}
```

### Serialization Testing

```rust
use rust_pbt::serialization::{assert_json_roundtrip, assert_bincode_roundtrip};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Trade {
    id: String,
    price: f64,
}

#[test]
    let trade = Trade { id: "123".to_string(), price: 100.0 };
    assert_json_roundtrip(&trade);
    assert_bincode_roundtrip(&trade);
}
```

## Available Modules

### Core (default)

| Module | Description |
|--------|-------------|
| `numeric` | Numeric property helpers (bounded, finite, non-negative) |
| `collections` | Collection properties (sorted, no duplicates, size preserved) |
| `arithmetic` | Arithmetic properties (commutative, associative, identity, distributive) |
| `stateful` | Stateful properties (idempotent, involutive, state invariants) |

### Optional

| Module | Feature | Description |
|--------|---------|-------------|
| `financial` | `financial` | Price strategies, PNL validation, fee checks |
| `serialization` | `serialization` | JSON/Bincode roundtrip testing |
| `temporal` | `temporal` | Timestamp strategies, monotonic sequences |
| `trading` | `trading` | OHLCV candles, order strategies |

## API Reference

### Numeric Properties

```rust
assert_bounded(value, min, max)    // value ∈ [min, max]
assert_non_negative(value)          // value ≥ 0
assert_positive(value)              // value > 0
assert_finite(value)                // not NaN or infinity
assert_approx_eq(a, b, tolerance)   // |a - b| ≤ tolerance
```

### Collection Properties

```rust
assert_size_preserved(input, output)     // input.len() == output.len()
assert_no_duplicates(values)              // all elements unique
assert_sorted(values)                     // ascending order
assert_sorted_descending(values)          // descending order
assert_same_elements(a, b)                // same elements (order-independent)
assert_all_satisfy(values, predicate)     // all pass predicate
```

### Arithmetic Properties

```rust
assert_commutative(a, b, f)     // f(a, b) == f(b, a)
assert_associative(a, b, c, f)  // f(f(a, b), c) == f(a, f(b, c))
assert_identity(a, id, f)       // f(a, id) == a
assert_distributive(a, b, c, f, g)  // f(a, g(b, c)) == g(f(a, b), f(a, c))
```

### Stateful Properties

```rust
assert_idempotent(value, f)              // f(f(x)) == f(x)
assert_involutive(value, f)              // f(f(x)) == x
assert_state_invariant(state, is_valid) // state satisfies invariant
assert_valid_state_transition(from, event, to, is_valid) // transition valid
assert_valid_state_sequence(states, is_valid) // all transitions valid
```

### Financial Properties

```rust
assert_pnl_sign_correct(entry, exit, qty, is_long, pnl)
assert_fee_reduces_profit(gross, fees, net)
assert_position_size_conserved(entry, fills, remaining)
assert_total_pnl_conservation(realized, unrealized, total)
assert_no_overfill(order_size, fills)
```

### Strategies

```rust
// Financial
valid_price()                    // [0.01, 1_000_000.0]
price_range(min, max)            // [min, max)
price_with_tick(min, max, tick)  // tick-aligned prices
profitable_long_pair()           // (entry, exit) where exit > entry
profitable_short_pair()          // (entry, exit) where entry > exit

// Temporal
valid_timestamp()                // 2020-2025 in milliseconds
valid_timestamp_seconds()        // 2020-2025 in seconds
monotonic_timestamps(n, gap)     // n timestamps with min gap
timestamp_pair()                 // (start, end) where end > start

// Trading
valid_candle()                   // Valid OHLCV candle
bullish_candle()                 // close > open
bearish_candle()                 // open > close
candle_sequence(n, gap)          // n candles with monotonic timestamps
valid_order()                    // Valid order with constraints
pending_order()                  // Order in pending state
filled_order()                   // Order fully filled
```

## Integration with Mutation Testing

This framework pairs well with `cargo-mutants`:

```bash
# Install
cargo install cargo-mutants

# Run mutation testing
cargo mutants

# Check specific crate
cargo mutants -p your-crate
```

Use property tests to catch mutants that unit tests miss!

## Environment Variables

- `PROPTEST_CASES`: Number of test cases (default: 256)
  ```bash
  PROPTEST_CASES=10000 cargo test  # thorough testing
  PROPTEST_CASES=100 cargo test    # quick CI runs
  ```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

## Contributing

Contributions welcome! Please ensure all tests pass and add tests for new features.

## Credits

Extracted from the [trading-backend-poc](https://github.com/moofone/trading-backend-poc) project.
