use serde_json::Value;
use std::{env, path::Path};

fn main() {
    println!("Starting Script Tester..");

    let args: Vec<String> = env::args().collect();

    if args.len() == 2 {
        let path = &args[1];
        if Path::new(path).is_file() {
            let status = forensics::core::parse_js_file(path).expect("failed script execution");
            if status != Value::Null {
                println!("{status:?}");
            }
            return;
        } else {
            panic!("Not a file")
        }
    }
    panic!("Require JS input file")
}
