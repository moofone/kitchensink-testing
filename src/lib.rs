//! # kitchensink-testing
//!
//! `kitchensink-testing` is a domain-agnostic property-based testing toolkit organized around:
//! - `generators`: reusable strategy primitives
//! - `law`: reusable law/invariant assertions
//! - `law::tokio`: trait-driven Tokio concurrency laws (feature `tokio-laws`)
//! - `mutation`: resumable mutation orchestration (feature `mutation`)
//!
//! Domain-specific generators and laws belong in the crate under test (or companion crates),
//! not in `kitchensink-testing` core.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

pub mod chaos;
pub mod generators;
pub mod law;
pub mod prelude;

#[cfg(feature = "mutation")]
#[cfg_attr(docsrs, doc(cfg(feature = "mutation")))]
pub mod mutation;

/// Re-export `proptest` for convenience.
pub use proptest;
