use super::{
    command::js_command,
    connections::js_connections,
    cpu::js_cpu,
    disks::js_disks,
    memory::js_memory,
    output::{js_output_results, js_raw_dump},
    processes::js_get_processes,
    systeminfo::{
        js_get_systeminfo, js_hostname, js_kernel_version, js_os_version, js_platform, js_uptime,
    },
};
use boa_engine::{Context, JsString, NativeFunction};

/// Link system functions `BoaJS`
pub(crate) fn system_functions(context: &mut Context) {
    let _ = context.register_global_callable(
        JsString::from("js_command"),
        2,
        NativeFunction::from_fn_ptr(js_command),
    );

    let _ = context.register_global_callable(
        JsString::from("js_cpu"),
        0,
        NativeFunction::from_fn_ptr(js_cpu),
    );

    let _ = context.register_global_callable(
        JsString::from("js_disks"),
        0,
        NativeFunction::from_fn_ptr(js_disks),
    );

    let _ = context.register_global_callable(
        JsString::from("js_memory"),
        0,
        NativeFunction::from_fn_ptr(js_memory),
    );

    let _ = context.register_global_callable(
        JsString::from("js_output_results"),
        3,
        NativeFunction::from_fn_ptr(js_output_results),
    );

    let _ = context.register_global_callable(
        JsString::from("js_raw_dump"),
        3,
        NativeFunction::from_fn_ptr(js_raw_dump),
    );

    let _ = context.register_global_callable(
        JsString::from("js_get_processes"),
        2,
        NativeFunction::from_fn_ptr(js_get_processes),
    );

    let _ = context.register_global_callable(
        JsString::from("js_get_systeminfo"),
        0,
        NativeFunction::from_fn_ptr(js_get_systeminfo),
    );

    let _ = context.register_global_callable(
        JsString::from("js_uptime"),
        0,
        NativeFunction::from_fn_ptr(js_uptime),
    );

    let _ = context.register_global_callable(
        JsString::from("js_hostname"),
        0,
        NativeFunction::from_fn_ptr(js_hostname),
    );

    let _ = context.register_global_callable(
        JsString::from("js_os_version"),
        0,
        NativeFunction::from_fn_ptr(js_os_version),
    );

    let _ = context.register_global_callable(
        JsString::from("js_kernel_version"),
        0,
        NativeFunction::from_fn_ptr(js_kernel_version),
    );

    let _ = context.register_global_callable(
        JsString::from("js_platform"),
        0,
        NativeFunction::from_fn_ptr(js_platform),
    );

    let _ = context.register_global_callable(
        JsString::from("js_connections"),
        0,
        NativeFunction::from_fn_ptr(js_connections),
    );
}

#[cfg(test)]
mod tests {
    use super::system_functions;
    use boa_engine::Context;

    #[test]
    fn test_system_functions() {
        let mut context = Context::default();
        system_functions(&mut context);
    }
}
