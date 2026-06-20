use forensics::core::parse_toml_file;
use glob::glob;
use std::fs::read;
use std::path::PathBuf;

#[test]
fn test_parquest_tester() {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/linux/parquet.toml");

    parse_toml_file(&test_location.display().to_string()).unwrap();
    let mut output_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    output_location.push("tmp/parquet_tester/*");

    let results = glob(output_location.to_str().unwrap()).unwrap();
    let mut par_count = 0;
    for result in results {
        let value = &result.unwrap();
        if value.to_str().unwrap().contains("report_") {
            let bytes = read(value).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            if text.contains("\"total_output_files\":0,") {
                panic!("missing parquest results??");
            }
            continue;
        }

        if value.display().to_string().contains(".parquet") {
            par_count += 1;
        }
    }
    assert_ne!(par_count, 0);
}
