//! Identifier/string strategy helpers.

use proptest::prelude::*;

/// Generate an alphanumeric identifier of exact length.
pub fn alphanumeric_id(len: usize) -> impl Strategy<Value = String> {
    assert!(len > 0, "length must be > 0");
    proptest::string::string_regex(&format!("[A-Za-z0-9]{{{len}}}"))
        .expect("alphanumeric regex should compile")
}

/// Generate an ID with a static prefix.
pub fn prefixed_id(prefix: &'static str, len: usize) -> impl Strategy<Value = String> {
    alphanumeric_id(len).prop_map(move |suffix| format!("{prefix}{suffix}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn prefixed_id_applies_prefix(id in prefixed_id("ord_", 6)) {
            prop_assert!(id.starts_with("ord_"));
            prop_assert_eq!(id.len(), 10);
        }
    }
}
