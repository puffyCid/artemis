use base64::{engine::general_purpose, Engine};
use clap::Parser;
use log::info;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Full path to TOML collector
    #[clap(short, long, value_parser)]
    toml: Option<String>,

    /// Base64 encoded TOML file
    #[clap(short, long, value_parser)]
    decode: Option<String>,

    /// Full path to JavaScript file
    #[clap(short, long, value_parser)]
    javascript: Option<String>,
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
    } else if let Some(data) = args.decode {
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
    } else if let Some(js) = args.javascript {
        if !js.is_empty() {
            let collection_results = artemis_core::core::parse_js_file(&js);
            match collection_results {
                Ok(_) => info!("[artemis] JavaScript execution success"),
                Err(err) => {
                    println!("[artemis] Failed to run JavaScript: {err:?}");
                    return;
                }
            }
        }
    } else {
        println!("[artemis] No valid command args provided!");
        return;
    }
    println!("[artemis] Finished artemis collection!");
}
