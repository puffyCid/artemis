use common::windows::{ServiceType, ServicesData};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

#[test]
#[cfg(target_os = "windows")]
fn test_services_parser() {
    use forensics::core::parse_toml_file;
    use glob::glob;
    use std::fs::read;
    use std::path::PathBuf;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/windows/services.toml");

    parse_toml_file(&test_location.display().to_string()).unwrap();

    let mut output_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    output_location.push("tmp/services_collection/*");
    let results = glob(output_location.to_str().unwrap()).unwrap();
    for result in results {
        let value = &result.unwrap();
        if value.to_str().unwrap().contains("report_") {
            let bytes = read(value).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            if text.contains("\"output_count\":0,") {
                panic!("missing Services??");
            }
        }
        let output_file = value.to_str().unwrap();

        if output_file.contains("\\services_") && output_file.ends_with(".jsonl") {
            validate_output(value);
        }
    }
}

fn validate_output(output: &PathBuf) {
    // Output is in JSONL based on the TOML file above!
    let file = File::open(output).unwrap();
    let reader = BufReader::new(file);
    for (_, line) in reader.lines().enumerate() {
        let value = line.unwrap();
        let info: ServicesData = serde_json::from_str(&value).unwrap();
        println!("{info:?}");
        assert!(!info.name.is_empty());
        assert!(!info.reg_path.is_empty());
        assert_ne!(info.modified, "1970-01-01T00:00:00Z");
        if info.name == "bam" {
            assert_eq!(info.path, "system32\\drivers\\bam.sys");
            assert!(info.service_type.contains(&ServiceType::KernelDriver));
        }
    }
}
