use std::{env, path::Path};

fn main() {
    println!("Starting Artemis Daemon...");

    let args: Vec<String> = env::args().collect();

    if args.len() == 2 {
        let path = args[1].clone();
        if Path::new(&path).is_file() {
            daemon::start::start_daemon(Some(path), None);
            println!("Started daemon example")
        } else {
            println!("Not a file")
        }
    } else {
        println!("Require TOML input file")
    }
}
