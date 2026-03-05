use common::windows::Amcache;
use std::path::PathBuf;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

#[test]
#[cfg(target_os = "windows")]
fn test_amcache_parser() {
    use forensics::core::parse_toml_file;
    use glob::glob;
    use std::fs::read;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/windows/amcache.toml");

    parse_toml_file(&test_location.display().to_string()).unwrap();
    let mut output_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    output_location.push("tmp/amcache_collection/*");

    let results = glob(output_location.to_str().unwrap()).unwrap();
    for result in results {
        let value = &result.unwrap();
        if value.to_str().unwrap().contains("report_") {
            let bytes = read(value).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            if text.contains("\"output_count\":0,") {
                panic!("missing Amcache??");
            }
            continue;
        }
        let output_file = value.to_str().unwrap();

        if output_file.contains("\\amcache_") && output_file.ends_with(".jsonl") {
            validate_output(value);
        }
        if output_file.ends_with(".log") && !output_file.starts_with("status_") {
            check_errors(value);
        }
    }
}

fn validate_output(output: &PathBuf) {
    // Output is in JSONL based on the TOML file above!
    let file = File::open(output).unwrap();
    let reader = BufReader::new(file);
    for (_, line) in reader.lines().enumerate() {
        let value = line.unwrap();
        println!("{value}");
        let info: Amcache = serde_json::from_str(&value).unwrap();
        assert!(!info.name.is_empty());
        assert_ne!(info.last_modified, "1970-01-01T00:00:00.000Z");
        assert!(!info.reg_path.is_empty())
    }
}

fn check_errors(output: &PathBuf) {
    let file = File::open(output).unwrap();
    let reader = BufReader::new(file);

    let mut count = 0;
    for (_, line) in reader.lines().enumerate() {
        let value = line.unwrap();
        println!("{value}");
        count += 1;
    }

    if count != 0 {
        panic!("check");
    }
}
