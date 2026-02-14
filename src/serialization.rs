//! Serialization property templates.
//!
//! Reusable templates for testing serialization roundtrips and determinism.
//!
//! # Example
//!
//! ```rust,ignore
//! use rust_pbt::serialization::assert_json_roundtrip;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize, PartialEq, Debug)]
//! struct Trade {
//!     id: String,
//!     price: f64,
//! }
//!
//! let trade = Trade { id: "123".to_string(), price: 100.0 };
//! assert_json_roundtrip(&trade);
//! ```

use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

/// Assert JSON serialization roundtrip: serialize → deserialize = identity
///
/// # Panics
///
/// Panics if serialization or deserialization fails, or if the roundtrip
/// doesn't preserve the value.
///
/// # Example
///
/// ```rust
/// use rust_pbt::serialization::assert_json_roundtrip;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// struct Trade {
///     id: String,
///     price: f64,
/// }
///
/// let trade = Trade { id: "123".to_string(), price: 100.0 };
/// assert_json_roundtrip(&trade);
/// ```
pub fn assert_json_roundtrip<T>(value: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let json = serde_json::to_string(value).expect("JSON serialization should succeed");
    let decoded: T =
        serde_json::from_str(&json).expect("JSON deserialization should succeed");
    assert_eq!(*value, decoded, "JSON roundtrip should preserve value");
}

/// Assert bincode serialization roundtrip: serialize → deserialize = identity
///
/// # Panics
///
/// Panics if serialization or deserialization fails, or if the roundtrip
/// doesn't preserve the value.
///
/// # Example
///
/// ```rust
/// use rust_pbt::serialization::assert_bincode_roundtrip;
///
/// let value = vec![1, 2, 3, 4, 5];
/// assert_bincode_roundtrip(&value);
/// ```
pub fn assert_bincode_roundtrip<T>(value: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let bytes = bincode::serialize(value).expect("Bincode serialization should succeed");
    let decoded: T =
        bincode::deserialize(&bytes).expect("Bincode deserialization should succeed");
    assert_eq!(*value, decoded, "Bincode roundtrip should preserve value");
}

/// Assert deterministic JSON serialization: same input → same output
///
/// # Panics
///
/// Panics if serializing the same value twice produces different results.
///
/// # Example
///
/// ```rust
/// use rust_pbt::serialization::assert_json_deterministic;
///
/// let value = vec![1, 2, 3];
/// assert_json_deterministic(&value);
/// ```
pub fn assert_json_deterministic<T>(value: &T)
where
    T: Serialize + Debug,
{
    let json1 = serde_json::to_string(value).expect("JSON serialization should succeed");
    let json2 = serde_json::to_string(value).expect("JSON serialization should succeed");
    assert_eq!(
        json1, json2,
        "JSON serialization should be deterministic for {:?}",
        value
    );
}

/// Assert deterministic bincode serialization: same input → same output
///
/// # Panics
///
/// Panics if serializing the same value twice produces different results.
///
/// # Example
///
/// ```rust
/// use rust_pbt::serialization::assert_bincode_deterministic;
///
/// let value = vec![1, 2, 3];
/// assert_bincode_deterministic(&value);
/// ```
pub fn assert_bincode_deterministic<T>(value: &T)
where
    T: Serialize + Debug,
{
    let bytes1 = bincode::serialize(value).expect("Bincode serialization should succeed");
    let bytes2 = bincode::serialize(value).expect("Bincode serialization should succeed");
    assert_eq!(
        bytes1, bytes2,
        "Bincode serialization should be deterministic for {:?}",
        value
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct TestStruct {
        id: i32,
        name: String,
        value: f64,
    }

    #[test]
    fn test_json_roundtrip() {
        let value = TestStruct {
            id: 42,
            name: "test".to_string(),
            value: 3.14,
        };
        assert_json_roundtrip(&value);
    }

    #[test]
    fn test_bincode_roundtrip() {
        let value = TestStruct {
            id: 42,
            name: "test".to_string(),
            value: 3.14,
        };
        assert_bincode_roundtrip(&value);
    }

    #[test]
    fn test_json_deterministic() {
        let value = vec![1, 2, 3, 4, 5];
        assert_json_deterministic(&value);
    }

    #[test]
    fn test_bincode_deterministic() {
        let value = vec![1, 2, 3, 4, 5];
        assert_bincode_deterministic(&value);
    }
}
