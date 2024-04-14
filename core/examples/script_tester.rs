use std::{env, path::Path};

use deno_core::error::JsError;

fn main() {
    println!("Starting Script Tester..");

    let args: Vec<String> = env::args().collect();

    if args.len() == 2 {
        let path = &args[1];
        if Path::new(path).is_file() {
            let status = core::core::parse_js_file(path).expect("failed script execution");
            let js_error_result = serde_json::from_value(status);
            // Checking for JS errors
            let js_err: JsError = match js_error_result {
                Ok(result) => result,
                Err(_err) => {
                    // If the JSON Value does not deserialize into JsError. Then collection was ok
                    println!("Collected data! No JsError");
                    return;
                }
            };
            panic!("Got JsError: {js_err:?}");
        } else {
            panic!("Not a file")
        }
    }
    panic!("Require JS input file")
}
