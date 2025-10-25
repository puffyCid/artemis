use super::{
    application::extensions::application_functions, compression::extensions::decompress_functions,
    decryption::extensions::decrypt_functions, encoding::extensions::encoding_functions,
    environment::extensions::env_functions, error::RuntimeError,
    filesystem::extensions::filesystem_functions, http::extensions::http_functions,
    linux::extensions::linux_functions, macos::extensions::macos_functions,
    nom::extensions::nom_functions, system::extensions::system_functions,
    time::extensions::time_functions, windows::extensions::windows_functions,
};
use boa_engine::{
    Context, JsError, JsResult, JsValue, Source,
    context::ContextBuilder,
    job::{GenericJob, Job, JobExecutor, NativeAsyncJob, PromiseJob, TimeoutJob},
    js_str, js_string,
    property::Attribute,
};
use boa_runtime::Console;
use futures_concurrency::future::FutureGroup;
use futures_lite::{StreamExt, future};
use log::{error, warn};
use serde_json::Value;
use std::{cell::RefCell, collections::VecDeque, rc::Rc};
use tokio::task;

/// Execute non-async scripts
pub(crate) fn run_script(script: &str, args: &[String]) -> Result<Value, RuntimeError> {
    let mut context = Context::default();

    let console = Console::init(&mut context);
    let status = context.register_global_property(Console::NAME, console, Attribute::all());
    if let Err(err) = status {
        error!("[runtime] Could not register console property: {err:?}");
        return Err(RuntimeError::ExecuteScript);
    }

    if !args.is_empty() {
        let serde_value = serde_json::to_value(args).unwrap_or_default();
        let value = JsValue::from_json(&serde_value, &mut context).unwrap_or_default();
        let status =
            context.register_global_property(js_str!("STATIC_ARGS"), value, Attribute::all());
        if let Err(err) = status {
            error!("[runtime] Could not register static args property: {err:?}");
            return Err(RuntimeError::ExecuteScript);
        }
    }

    setup_runtime(&mut context);

    let result = match context.eval(Source::from_bytes(script.as_bytes())) {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not execute script: {err:?}");
            // A script should never halt execution
            return Ok(serde_json::to_value(format!("{err:?}")).unwrap_or_default());
        }
    };
    if result.is_undefined() {
        return Ok(Value::Null);
    }
    if let Ok(Some(value)) = result.to_json(&mut context) {
        return Ok(value);
    }
    error!(
        "[runtime] Could not serialize script value: {:?}",
        result.to_json(&mut context)
    );
    Err(RuntimeError::ScriptResult)
}

/// Queue to handle async scripts
struct Queue {
    async_jobs: RefCell<VecDeque<NativeAsyncJob>>,
    promise_jobs: RefCell<VecDeque<PromiseJob>>,
    timeout_jobs: RefCell<VecDeque<TimeoutJob>>,
    generic_jobs: RefCell<VecDeque<GenericJob>>,
}

// https://github.com/boa-dev/boa/blob/main/examples/src/bin/module_fetch_async.rs
impl Queue {
    fn new() -> Self {
        Self {
            async_jobs: RefCell::default(),
            promise_jobs: RefCell::default(),
            timeout_jobs: RefCell::default(),
            generic_jobs: RefCell::default(),
        }
    }

    fn drain_jobs(&self, context: &mut Context) {
        let jobs = std::mem::take(&mut *self.promise_jobs.borrow_mut());
        for job in jobs {
            if let Err(err) = job.call(context) {
                error!("[runtime] Failed drain async jobs: {err:?}");
            }
        }
    }
}

// https://github.com/boa-dev/boa/blob/main/examples/src/bin/module_fetch_async.rs
impl JobExecutor for Queue {
    /// Run jobs will block Rust execution until script is done. However, the script may still be run as async
    fn run_jobs(self: Rc<Self>, context: &mut Context) -> JsResult<()> {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
        {
            Ok(result) => result,
            Err(err) => {
                error!("[runtime] Failed to run job: {err:?}");
                let issue = format!("Failed to run job: {err:?}");

                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };
        task::LocalSet::default().block_on(&runtime, self.run_jobs_async(&RefCell::new(context)))
    }

    /// Run jobs async will not block Rust execution while script runs
    async fn run_jobs_async(self: Rc<Self>, context: &RefCell<&mut Context>) -> JsResult<()> {
        let mut group = FutureGroup::new();
        loop {
            for job in std::mem::take(&mut *self.async_jobs.borrow_mut()) {
                group.insert(job.call(context));
            }

            if group.is_empty()
                && self.promise_jobs.borrow().is_empty()
                && self.timeout_jobs.borrow().is_empty()
                && self.generic_jobs.borrow().is_empty()
            {
                // All queues are empty. We can exit.
                return Ok(());
            }

            // We have some jobs pending on the microtask queue. Try to poll the pending
            // tasks once to see if any of them finished, and run the pending microtasks
            // otherwise.
            if let Some(Err(err)) = future::poll_once(group.next()).await.flatten() {
                error!("[runtime] Failed to queue async job: {err:?}");
            };

            self.drain_jobs(&mut context.borrow_mut());
            task::yield_now().await;
        }
    }

    fn enqueue_job(self: Rc<Self>, job: Job, _context: &mut Context) {
        match job {
            Job::PromiseJob(job) => self.promise_jobs.borrow_mut().push_back(job),
            Job::AsyncJob(job) => self.async_jobs.borrow_mut().push_back(job),
            Job::TimeoutJob(job) => self.timeout_jobs.borrow_mut().push_back(job),
            Job::GenericJob(job) => self.generic_jobs.borrow_mut().push_back(job),
            _ => warn!("[runtime] Unsupported job {job:?}"),
        }
    }
}

/// Execute async scripts
pub(crate) fn run_async_script(script: &str, args: &[String]) -> Result<Value, RuntimeError> {
    let queue = Queue::new();
    let mut context = match ContextBuilder::new().job_executor(Rc::new(queue)).build() {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not create async context: {err:?}");
            return Err(RuntimeError::ExecuteScript);
        }
    };

    let console = Console::init(&mut context);
    let status = context.register_global_property(Console::NAME, console, Attribute::all());
    if let Err(err) = status {
        error!("[runtime] Could not register console property: {err:?}");
        return Err(RuntimeError::ExecuteScript);
    }

    if !args.is_empty() {
        let serde_value = serde_json::to_value(args).unwrap_or_default();
        let value = JsValue::from_json(&serde_value, &mut context).unwrap_or_default();
        let status =
            context.register_global_property(js_str!("STATIC_ARGS"), value, Attribute::all());
        if let Err(err) = status {
            error!("[runtime] Could not register static args property: {err:?}");
            return Err(RuntimeError::ExecuteScript);
        }
    }

    setup_runtime(&mut context);

    let result = match context.eval(Source::from_bytes(script.as_bytes())) {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not execute script: {err:?}");
            // A script should never halt execution
            return Ok(serde_json::to_value(format!("{err:?}")).unwrap_or_default());
        }
    };

    // Run and wait for our script to complete
    let _ = context.run_jobs();
    if result.is_undefined() {
        return Ok(Value::Null);
    } else if result.is_promise() {
        // Handle async/await promises
        if let Some(promise) = result.as_promise() {
            // Wait for promise to resolve
            if let Ok(js_value) = promise.await_blocking(&mut context) {
                if js_value.is_undefined() {
                    return Ok(Value::Null);
                }
                if let Ok(Some(value)) = js_value.to_json(&mut context) {
                    return Ok(value);
                }
                error!(
                    "[runtime] Could not serialize async promise script value: {:?}",
                    result.to_json(&mut context)
                );
                return Err(RuntimeError::ScriptResult);
            }
        }
    }

    if let Ok(Some(value)) = result.to_json(&mut context) {
        return Ok(value);
    }
    error!(
        "[runtime] Could not serialize async script value: {:?}",
        result.to_json(&mut context)
    );
    Err(RuntimeError::ScriptResult)
}

/// Register and create our custom JavaScript runtime
fn setup_runtime(context: &mut Context) {
    filesystem_functions(context);
    encoding_functions(context);
    application_functions(context);
    linux_functions(context);
    nom_functions(context);
    http_functions(context);
    env_functions(context);
    decompress_functions(context);
    decrypt_functions(context);
    system_functions(context);
    time_functions(context);
    windows_functions(context);
    macos_functions(context);
}

#[cfg(test)]
mod tests {
    use super::run_script;

    #[test]
    fn test_run_script() {
        let script = "console.info('look im running JS!')";
        let _ = run_script(script, &[]).unwrap();
    }

    #[test]
    fn test_run_async_script() {
        let script = "console.warn(`true + true = ${true + true}. Classic JS, gotta love it`)";
        let _ = run_script(script, &[]).unwrap();
    }
}
