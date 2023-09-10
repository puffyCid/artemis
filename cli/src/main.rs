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
    parse_args(&args)
}

/// Parse the support `artemis` options
fn parse_args(args: &Args) {
    println!("[artemis] Starting artemis collection!");

    if let Some(toml) = &args.toml {
        if !toml.is_empty() {
            let collection_results = artemis_core::core::parse_toml_file(toml);
            match collection_results {
                Ok(_) => info!("[artemis] Collection success"),
                Err(err) => {
                    println!("[artemis] Failed to collect artifacts: {err:?}");
                    return;
                }
            }
        }
    } else if let Some(data) = &args.decode {
        if !data.is_empty() {
            let toml_data_results = general_purpose::STANDARD.decode(data);
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
    } else if let Some(js) = &args.javascript {
        if !js.is_empty() {
            let collection_results = artemis_core::core::parse_js_file(js);
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

#[cfg(test)]
mod tests {
    use crate::{parse_args, Args};
    use std::path::PathBuf;

    #[test]
    #[cfg(target_os = "linux")]
    fn test_parse_args_toml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("../artemis-core/tests/test_data/linux/systeminfo.toml");
        let args = Args {
            toml: Some(test_location.display().to_string()),
            decode: None,
            javascript: None,
        };

        parse_args(&args);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_parse_args_decode() {
        let args = Args {
            toml: None,
            decode: Some(String::from("c3lzdGVtID0gImxpbnV4IgoKW291dHB1dF0KbmFtZSA9ICJzeXN0ZW1pbmZvX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiYWJkYyIKY29sbGVjdGlvbl9pZCA9IDEKb3V0cHV0ID0gImxvY2FsIgoKW1thcnRpZmFjdHNdXQphcnRpZmFjdF9uYW1lID0gInN5c3RlbWluZm8iCg==")),
            javascript: None,
        };

        parse_args(&args);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_parse_args_toml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("../artemis-core/tests/test_data/windows/systeminfo.toml");
        let args = Args {
            toml: Some(test_location.display().to_string()),
            decode: None,
            javascript: None,
        };

        parse_args(&args);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_parse_args_decode() {
        let args = Args {
            toml: None,
            decode: Some(String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInN5c3RlbWluZm9fY29sbGVjdGlvbiIKZGlyZWN0b3J5ID0gIi4vdG1wIgpmb3JtYXQgPSAianNvbiIKY29tcHJlc3MgPSBmYWxzZQplbmRwb2ludF9pZCA9ICJhYmRjIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAic3lzdGVtaW5mbyIK")),
            javascript: None,
        };

        parse_args(&args);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_parse_args_toml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("../artemis-core/tests/test_data/macos/systeminfo.toml");
        let args = Args {
            toml: Some(test_location.display().to_string()),
            decode: None,
            javascript: None,
        };

        parse_args(&args);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_parse_args_decode() {
        let args = Args {
            toml: None,
            decode: Some(String::from("c3lzdGVtID0gIm1hY29zIgoKW291dHB1dF0KbmFtZSA9ICJzeXN0ZW1pbmZvX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiYWJkYyIKY29sbGVjdGlvbl9pZCA9IDEKb3V0cHV0ID0gImxvY2FsIgoKW1thcnRpZmFjdHNdXQphcnRpZmFjdF9uYW1lID0gInN5c3RlbWluZm8iCg==")),
            javascript: None,
        };

        parse_args(&args);
    }

    #[test]
    fn test_parse_args_js() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("../artemis-core/tests/test_data/deno_scripts/vanilla.js");
        let args = Args {
            toml: None,
            decode: None,
            javascript: Some(test_location.display().to_string()),
        };

        parse_args(&args);
    }
}
