use super::{
    application::extensions::application_functions, compression::extensions::decompress_functions,
    decryption::extensions::decrypt_functions, encoding::extensions::encoding_functions,
    environment::extensions::env_functions, error::RuntimeError,
    filesystem::extensions::filesystem_functions, http::extensions::http_functions,
    linux::extensions::linux_functions, macos::extensions::macos_functions,
    nom::extensions::nom_functions, system::extensions::system_functions,
    time::extensions::time_functions, unix::extensions::unix_functions,
    windows::extensions::windows_functions,
};
use boa_engine::{
    Context, JsValue, Source,
    context::ContextBuilder,
    job::{FutureJob, JobQueue, NativeJob},
    js_str,
    property::Attribute,
};
use boa_runtime::Console;
use log::error;
use serde_json::Value;
use std::{cell::RefCell, collections::VecDeque, future::Future, pin::Pin, rc::Rc};
use tokio::task;

/// Execute non-async scripts
pub(crate) fn run_script(script: &str, args: &[String]) -> Result<Value, RuntimeError> {
    let mut context = Context::default();

    let console = Console::init(&mut context);
    let status = context.register_global_property(Console::NAME, console, Attribute::all());
    if status.is_err() {
        let err = status.unwrap_err();
        error!("[runtime] Could not register console property: {err:?}");
        return Err(RuntimeError::ExecuteScript);
    }
    if !args.is_empty() {
        let serde_value = serde_json::to_value(args).unwrap_or_default();
        let value = JsValue::from_json(&serde_value, &mut context).unwrap_or_default();
        let status =
            context.register_global_property(js_str!("STATIC_ARGS"), value, Attribute::all());
        if status.is_err() {
            let err = status.unwrap_err();
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
    let value = match result.to_json(&mut context) {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not serialize script value: {err:?}");
            return Err(RuntimeError::ScriptResult);
        }
    };

    Ok(value)
}

/// Queue to handle async scripts
struct Queue {
    futures: RefCell<Vec<FutureJob>>,
    jobs: RefCell<VecDeque<NativeJob>>,
}

// From boa example: https://github.com/boa-dev/boa/blob/294ebd8788914cf2b807e743377aa03c58d7d534/examples/src/bin/tokio_event_loop.rs
impl Queue {
    fn new() -> Self {
        Self {
            futures: RefCell::default(),
            jobs: RefCell::default(),
        }
    }

    fn drain_jobs(&self, context: &mut Context) {
        let jobs = std::mem::take(&mut *self.jobs.borrow_mut());
        for job in jobs {
            if let Err(err) = job.call(context) {
                error!("[runtime] Failed drain async jobs: {err:?}");
            }
        }
    }
}

impl JobQueue for Queue {
    fn enqueue_promise_job(&self, job: NativeJob, _context: &mut Context) {
        self.jobs.borrow_mut().push_back(job);
    }

    fn enqueue_future_job(&self, future: FutureJob, _context: &mut Context) {
        self.futures.borrow_mut().push(future);
    }

    /// Run jobs will block Rust execution until script is done. However, the script may still be run as async
    fn run_jobs(&self, context: &mut Context) {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
        {
            Ok(result) => result,
            Err(err) => {
                error!("[runtime] Failed to run job: {err:?}");
                return;
            }
        };

        task::LocalSet::default().block_on(&runtime, self.run_jobs_async(context));
    }

    /// Run jobs async will not block Rust execution while script runs
    fn run_jobs_async<'a, 'ctx, 'fut>(
        &'a self,
        context: &'ctx mut Context,
    ) -> Pin<Box<dyn Future<Output = ()> + 'fut>>
    where
        'a: 'fut,
        'ctx: 'fut,
    {
        Box::pin(async move {
            // If we have no jobs just return
            if self.jobs.borrow().is_empty() && self.futures.borrow().is_empty() {
                return;
            }
            let mut join_set = task::JoinSet::new();
            loop {
                for future in std::mem::take(&mut *self.futures.borrow_mut()) {
                    join_set.spawn_local(future);
                }

                if self.jobs.borrow().is_empty() {
                    let Some(job) = join_set.join_next().await else {
                        // Both queues are empty. We can exit.
                        return;
                    };

                    // Important to schedule the returned `job` into the job queue, since that's
                    // what allows updating the `Promise` seen by ECMAScript for when the future
                    // completes.
                    match job {
                        Ok(job) => self.enqueue_promise_job(job, context),
                        Err(err) => error!("[runtime] Failed to queue async job: {err:?}"),
                    }

                    continue;
                }

                // We have some jobs pending on the microtask queue. Try to poll the pending
                // tasks once to see if any of them finished, and run the pending microtasks
                // otherwise.
                let Some(job) = join_set.try_join_next() else {
                    // No completed jobs. Run the microtask queue once.
                    self.drain_jobs(context);

                    task::yield_now().await;
                    continue;
                };

                // Important to schedule the returned `job` into the job queue, since that's
                // what allows updating the `Promise` seen by ECMAScript for when the future
                // completes.
                match job {
                    Ok(job) => self.enqueue_promise_job(job, context),
                    Err(err) => error!("[runtime] Failed to queue next async job: {err:?}"),
                }

                // Only one macrotask can be executed before the next drain of the microtask queue.
                self.drain_jobs(context);
            }
        })
    }
}

/// Execute async scripts
pub(crate) fn run_async_script(script: &str, args: &[String]) -> Result<Value, RuntimeError> {
    let queue = Queue::new();
    let mut context = match ContextBuilder::new().job_queue(Rc::new(queue)).build() {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not create async context: {err:?}");
            return Err(RuntimeError::ExecuteScript);
        }
    };

    let console = Console::init(&mut context);
    let status = context.register_global_property(Console::NAME, console, Attribute::all());
    if status.is_err() {
        let err = status.unwrap_err();
        error!("[runtime] Could not register console property: {err:?}");
        return Err(RuntimeError::ExecuteScript);
    }

    if !args.is_empty() {
        let serde_value = serde_json::to_value(args).unwrap_or_default();
        let value = JsValue::from_json(&serde_value, &mut context).unwrap_or_default();
        let status =
            context.register_global_property(js_str!("STATIC_ARGS"), value, Attribute::all());
        if status.is_err() {
            let err = status.unwrap_err();
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
    context.run_jobs();
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
                let value = match js_value.to_json(&mut context) {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[runtime] Could not serialize promise value: {err:?}");
                        return Err(RuntimeError::ScriptResult);
                    }
                };
                return Ok(value);
            }
        }
    }

    let value = match result.to_json(&mut context) {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not serialize script value: {err:?}");
            return Err(RuntimeError::ScriptResult);
        }
    };

    Ok(value)
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
    unix_functions(context);
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
