//! Async runtime helper for sync CLI handlers calling async nest-airtable APIs.

use nest_error::{NestError, NestResult};
use nest_task_runtime::{RuntimeConfig, TaskRuntime};

/// Runs an async future from a synchronous CLI command handler.
pub fn block_on_async<F, T>(future: F) -> NestResult<T>
where
    F: std::future::Future<Output = NestResult<T>> + Send + 'static,
    T: Send + 'static,
{
    if tokio::runtime::Handle::try_current().is_ok() {
        return std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| NestError::task(format!("failed to start runtime: {error}")))?;
            runtime.block_on(future)
        })
        .join()
        .map_err(|_| NestError::task("async worker thread panicked"))?;
    }

    let runtime = TaskRuntime::new_owned(RuntimeConfig::default())?;
    runtime.handle().block_on(future)
}
