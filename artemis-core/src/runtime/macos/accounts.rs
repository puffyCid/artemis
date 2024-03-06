use crate::{
    artifacts::os::macos::accounts::{groups::grab_groups, users::grab_users},
    structs::artifacts::os::macos::{MacosGroupsOptions, MacosUsersOptions},
};
use deno_core::{error::AnyError, op2};

#[op2]
#[string]
/// Expose parsing Users to `Deno`
pub(crate) fn get_users(#[string] path: String) -> Result<String, AnyError> {
    let mut user_path = path;
    if user_path.is_empty() {
        user_path = String::from("/var/db/dslocal/nodes/Default/users");
    }
    let users = grab_users(&MacosUsersOptions {
        alt_path: Some(user_path),
    });
    let results = serde_json::to_string(&users)?;
    Ok(results)
}

#[op2]
#[string]
/// Expose parsing Groups to `Deno`
pub(crate) fn get_groups(#[string] path: String) -> Result<String, AnyError> {
    let mut group_path = path;
    if group_path.is_empty() {
        group_path = String::from("/var/db/dslocal/nodes/Default/users");
    }
    let groups = grab_groups(&MacosGroupsOptions {
        alt_path: Some(group_path),
    });
    let results = serde_json::to_string(&groups)?;
    Ok(results)
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use crate::{
        runtime::deno::execute_script, structs::artifacts::runtime::script::JSScript,
        structs::toml::Output,
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_grab_users() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfdXNlcnMoKSB7CiAgICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfdXNlcnMoKTsKICAgIGNvbnN0IHVzZXJzID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiB1c2VyczsKfQpmdW5jdGlvbiBnZXRfZ3JvdXBzKCkgewogICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X2dyb3VwcygpOwogICAgY29uc3QgZ3JvdXBzID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBncm91cHM7Cn0KZnVuY3Rpb24gZ2V0VXNlcnMoKSB7CiAgICByZXR1cm4gZ2V0X3VzZXJzKCk7Cn0KZnVuY3Rpb24gZ2V0R3JvdXBzKCkgewogICAgcmV0dXJuIGdldF9ncm91cHMoKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgdXNlcnMgPSBnZXRVc2VycygpOwogICAgY29uc3QgZ3JvdXBzID0gZ2V0R3JvdXBzKCk7CiAgICBjb25zdCBhY2NvdW50cyA9IHsKICAgICAgICB1c2VycywKICAgICAgICBncm91cHMKICAgIH07CiAgICByZXR1cm4gYWNjb3VudHM7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("users"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_grab_groups() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfdXNlcnMoKSB7CiAgICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfdXNlcnMoKTsKICAgIGNvbnN0IHVzZXJzID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiB1c2VyczsKfQpmdW5jdGlvbiBnZXRfZ3JvdXBzKCkgewogICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X2dyb3VwcygpOwogICAgY29uc3QgZ3JvdXBzID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBncm91cHM7Cn0KZnVuY3Rpb24gZ2V0VXNlcnMoKSB7CiAgICByZXR1cm4gZ2V0X3VzZXJzKCk7Cn0KZnVuY3Rpb24gZ2V0R3JvdXBzKCkgewogICAgcmV0dXJuIGdldF9ncm91cHMoKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgdXNlcnMgPSBnZXRVc2VycygpOwogICAgY29uc3QgZ3JvdXBzID0gZ2V0R3JvdXBzKCk7CiAgICBjb25zdCBhY2NvdW50cyA9IHsKICAgICAgICB1c2VycywKICAgICAgICBncm91cHMKICAgIH07CiAgICByZXR1cm4gYWNjb3VudHM7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("groups"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
