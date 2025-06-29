use std::{env, path::Path};

#[tokio::main]
async fn main() {
    println!("Starting Artemis Core collector...");

    let args: Vec<String> = env::args().collect();

    if args.len() == 2 {
        let path = &args[1];
        if Path::new(path).is_file() {
            forensics::core::parse_toml_file(path).await.unwrap();
            println!("Collected data!")
        } else {
            println!("Not a file")
        }
    } else {
        println!("Require TOML input file")
    }
}
