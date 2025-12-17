use super::{encoding::base64_decode_standard, error::ArtemisError, strings::extract_utf8_string};
use log::error;
#[cfg(feature = "yarax")]
use yara_x::{Compiler, Scanner};

/// Scan a file using Yara-X
pub(crate) fn scan_file(path: &str, encoded_rule: &str) -> Result<Vec<String>, ArtemisError> {
    #[cfg(not(feature = "yarax"))]
    {
        return Ok(Vec::new());
    }
    #[cfg(feature = "yarax")]
    {
        let rule = rule_decode(encoded_rule)?;
        let compile = compile_rule(&rule)?;

        let rules = compile.build();
        let mut scanner = Scanner::new(&rules);
        let results = scanner.scan_file(path);
        let hits = match results {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Failed to scan file {path}: {err:?}",);
                return Err(ArtemisError::YaraScan);
            }
        };
        let mut matches = Vec::new();
        for hit in hits.matching_rules() {
            matches.push(hit.identifier().to_string());
        }
        Ok(matches)
    }
}

/// Scan bytes using Yara-X
pub(crate) fn scan_bytes(data: &[u8], encoded_rule: &str) -> Result<Vec<String>, ArtemisError> {
    #[cfg(not(feature = "yarax"))]
    {
        return Ok(Vec::new());
    }
    #[cfg(feature = "yarax")]
    {
        let rule = rule_decode(encoded_rule)?;
        let compile = compile_rule(&rule)?;

        let rules = compile.build();
        let mut scanner = Scanner::new(&rules);
        let results = scanner.scan(data);
        let hits = match results {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Failed to scan bytes: {err:?}",);
                return Err(ArtemisError::YaraScan);
            }
        };
        let mut matches = Vec::new();
        for hit in hits.matching_rules() {
            matches.push(hit.identifier().to_string());
        }
        Ok(matches)
    }
}

/// Scan base64 encoded bytes using Yara-X
pub(crate) fn scan_base64_bytes(
    encoded_bytes: &str,
    encoded_rule: &str,
) -> Result<Vec<String>, ArtemisError> {
    let bytes_result = base64_decode_standard(encoded_bytes);
    let bytes = match bytes_result {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to base64 target bytes: {err:?}");
            return Err(ArtemisError::Encoding);
        }
    };

    scan_bytes(&bytes, encoded_rule)
}

/// Base64 decode yara rule
fn rule_decode(rule: &str) -> Result<String, ArtemisError> {
    let bytes_result = base64_decode_standard(rule);
    let rule_bytes = match bytes_result {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to base64 decode rule: {err:?}");
            return Err(ArtemisError::Encoding);
        }
    };

    Ok(extract_utf8_string(&rule_bytes))
}

#[cfg(feature = "yarax")]
/// Attempt to compile the Yara rule
fn compile_rule(rule: &str) -> Result<Compiler<'_>, ArtemisError> {
    let mut compile = Compiler::new();
    compile.error_on_slow_pattern(true);
    let status = compile.add_source(rule);
    if let Err(result) = status {
        error!("[forensics] Failed to add yara rule: {result:?}");
        return Err(ArtemisError::YaraRule);
    }

    Ok(compile)
}

#[cfg(test)]
mod tests {
    use super::scan_bytes;
    use crate::{
        filesystem::files::read_file,
        utils::{
            encoding::base64_encode_standard,
            yara::{scan_base64_bytes, scan_file},
        },
    };
    use std::path::PathBuf;

    #[test]
    #[cfg(feature = "yarax")]
    #[should_panic(expected = "YaraRule")]
    fn test_compile_rule_bad() {
        use super::compile_rule;

        let rule = r#"
        rule hello_world {
        strings:
        $ = "hello, world! Its Rust!"
        condition:
        all of them
        "#;

        let _ = compile_rule(rule).unwrap();
    }

    #[test]
    fn test_scan_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files/test.txt");

        let rule = r#"
        rule hello_world {
        strings:
        $ = "hello, world! Its Rust!"
        condition:
        all of them
        }
        "#;

        let result = scan_file(
            test_location.to_str().unwrap(),
            &base64_encode_standard(rule.as_bytes()),
        )
        .unwrap();

        assert_eq!(result[0], "hello_world");
    }

    #[test]
    fn test_scan_base64_bytes() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files/test.txt");

        let bytes = read_file(test_location.to_str().unwrap()).unwrap();

        let rule = r#"
        rule hello_world {
        strings:
        $ = "hello, world! Its Rust!"
        condition:
        all of them
        }
        "#;

        let result = scan_base64_bytes(
            &base64_encode_standard(&bytes),
            &base64_encode_standard(rule.as_bytes()),
        )
        .unwrap();

        assert_eq!(result[0], "hello_world");
    }

    #[test]
    #[should_panic(expected = "Encoding")]
    fn test_scan_bytes_bad_encoding() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files/test.txt");

        let bytes = read_file(test_location.to_str().unwrap()).unwrap();

        let rule = r#"
        rule hello_world {
        strings:
        $ = "hello, world! Its Rust!"
        condition:
        all of them
        }
        "#;

        let _ = scan_bytes(&bytes, rule).unwrap();
    }
}
