//! Tokio-focused law assertions.
//!
//! This surface is intentionally trait-driven: application crates define scenario traits for their
//! own executor, state model, and instrumentation, then reuse these assertions to enforce shared
//! concurrency contracts.

pub mod io;
#[cfg(feature = "tokio-loom")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-loom")))]
pub mod loom;
pub mod sync;
pub mod task;
pub mod time;
