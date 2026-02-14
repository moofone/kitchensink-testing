//! Serialization law assertions.

use serde::{Serialize, de::DeserializeOwned};
use std::fmt::Debug;

/// Assert JSON roundtrip identity.
pub fn assert_json_roundtrip<T>(value: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let json = serde_json::to_string(value).expect("JSON serialization should succeed");
    let decoded: T = serde_json::from_str(&json).expect("JSON deserialization should succeed");
    assert_eq!(*value, decoded, "JSON roundtrip should preserve value");
}

/// Assert JSON determinism for two serializations of same value.
pub fn assert_json_deterministic<T>(value: &T)
where
    T: Serialize + Debug,
{
    let first = serde_json::to_string(value).expect("JSON serialization should succeed");
    let second = serde_json::to_string(value).expect("JSON serialization should succeed");
    assert_eq!(first, second, "JSON serialization should be deterministic");
}

/// Assert bincode roundtrip identity.
#[cfg(feature = "serialization")]
#[cfg_attr(docsrs, doc(cfg(feature = "serialization")))]
pub fn assert_bincode_roundtrip<T>(value: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let bytes = bincode::serialize(value).expect("bincode serialization should succeed");
    let decoded: T = bincode::deserialize(&bytes).expect("bincode deserialization should succeed");
    assert_eq!(*value, decoded, "bincode roundtrip should preserve value");
}

/// Assert bincode determinism.
#[cfg(feature = "serialization")]
#[cfg_attr(docsrs, doc(cfg(feature = "serialization")))]
pub fn assert_bincode_deterministic<T>(value: &T)
where
    T: Serialize + Debug,
{
    let first = bincode::serialize(value).expect("bincode serialization should succeed");
    let second = bincode::serialize(value).expect("bincode serialization should succeed");
    assert_eq!(
        first, second,
        "bincode serialization should be deterministic"
    );
}
