use forensics::core::parse_toml_file;
use glob::glob;
use rusqlite::Connection;
use std::fs::read;
use std::path::PathBuf;

#[test]
fn test_sqlite_tester() {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/collections/sqlite.toml");

    parse_toml_file(&test_location.display().to_string()).unwrap();
    let mut output_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    output_location.push("tmp/sqlite_collection/*");

    let results = glob(output_location.to_str().unwrap()).unwrap();
    let mut sqlite_count = 0;
    for result in results {
        let value = &result.unwrap();
        if value.to_str().unwrap().contains("report_") {
            let bytes = read(value).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            if text.contains("\"total_output_files\":0,") {
                panic!("missing sqlite results??");
            }
            continue;
        }

        if value.display().to_string().contains(".sqlite") {
            sqlite_count += 1;
            validate_output(value);
        }
    }
    assert_eq!(sqlite_count, 1);
}

fn validate_output(path: &PathBuf) {
    let conn = Connection::open(path).unwrap();

    let results: i64 = conn
        .query_row("select count(*) from processes", [], |row| row.get(0))
        .unwrap();
    assert_ne!(results, 0);

    let results: i64 = conn
        .query_row("select count(*) from systeminfo", [], |row| row.get(0))
        .unwrap();
    assert_ne!(results, 0);

    let results: i64 = conn
        .query_row("select count(*) from files", [], |row| row.get(0))
        .unwrap();
    assert_ne!(results, 0);
}
