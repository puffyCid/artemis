use server::server::start;
use std::{env, path::Path};
fn main() {
    println!("Starting basic server on loopback IP");
    let args: Vec<String> = env::args().collect();

    if args.len() == 2 {
        let path = &args[1];
        if Path::new(path).is_file() {
            start(path);
        } else {
            println!("Not server config file")
        }
    } else {
        println!("Require TOML config input file. See tests for an example")
    }
}
