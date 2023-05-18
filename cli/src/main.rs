use base64::{engine::general_purpose, Engine};
use clap::Parser;
use log::info;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Full path tol TOML collector
    #[clap(short, long, value_parser)]
    toml: Option<String>,

    /// Base64 encoded TOML file
    #[clap(short, long, value_parser)]
    data: Option<String>,
}

fn main() {
    let args = Args::parse();
    println!("[artemis] Starting artemis collection!");

    if let Some(toml) = args.toml {
        if !toml.is_empty() {
            let collection_results = artemis_core::core::parse_toml_file(&toml);
            match collection_results {
                Ok(_) => info!("[artemis] Collection success"),
                Err(err) => {
                    println!("[artemis] Failed to collect artifacts: {err:?}");
                    return;
                }
            }
        }
    } else if let Some(data) = args.data {
        if !data.is_empty() {
            let toml_data_results = general_purpose::STANDARD.decode(&data);
            let toml_data = match toml_data_results {
                Ok(results) => results,
                Err(err) => {
                    println!(
                        "[artemis] Failed to base64 decode TOML collector {data}, error: {err:?}",
                    );
                    return;
                }
            };
            let collection_results = artemis_core::core::parse_toml_data(&toml_data);
            match collection_results {
                Ok(_) => info!("[artemis] Collection success"),
                Err(err) => {
                    println!("[artemis] Failed to collect artifacts: {err:?}");
                    return;
                }
            }
        }
    } else {
        println!("[artemis] No TOML file or data provided!");
        return;
    }
    println!("[artemis] Finished artemis collection!");
}
