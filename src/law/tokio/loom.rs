//! Loom-backed model-checking adapter for Tokio law scenarios.
//!
//! Application crates implement [`TokioLoomModel`] for small critical concurrency kernels that
//! can be exhaustively explored with Loom schedules.

use std::sync::Arc;

/// Application-defined model runnable under `loom::model`.
pub trait TokioLoomModel: Send + Sync + 'static {
    /// Execute the model body under Loom instrumentation.
    fn run_model(&self);
}

/// Assert that a Loom model executes successfully under explored schedules.
pub fn assert_loom_model<M>(model: M)
where
    M: TokioLoomModel,
{
    let model = Arc::new(model);
    loom::model(move || {
        model.run_model();
    });
}

#[cfg(test)]
mod tests {
    use loom::sync::Arc;
    use loom::sync::atomic::{AtomicUsize, Ordering};
    use loom::thread;

    use super::*;

    struct CounterModel;

    impl TokioLoomModel for CounterModel {
        fn run_model(&self) {
            let shared = Arc::new(AtomicUsize::new(0));
            let left = Arc::clone(&shared);
            let right = Arc::clone(&shared);

            let t1 = thread::spawn(move || {
                left.fetch_add(1, Ordering::SeqCst);
            });
            let t2 = thread::spawn(move || {
                right.fetch_add(1, Ordering::SeqCst);
            });

            t1.join().expect("first loom thread should join");
            t2.join().expect("second loom thread should join");

            assert_eq!(shared.load(Ordering::SeqCst), 2);
        }
    }

    #[test]
    fn loom_adapter_runs_model() {
        assert_loom_model(CounterModel);
    }
}
