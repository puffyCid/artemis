use super::error::ArtemisError;
use log::error;
use regex::Regex;

/// Check if string matches provided regex
pub(crate) fn regex_check(reg: &Regex, path: &str) -> bool {
    if reg.as_str() == "" {
        return true;
    }

    reg.is_match(path)
}

/// Create a compiled Regex
pub(crate) fn create_regex(input: &str) -> Result<Regex, ArtemisError> {
    let regex_result = Regex::new(input);
    let regex = match regex_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Bad regex {input}, error: {err:?}");
            return Err(ArtemisError::Regex);
        }
    };

    Ok(regex)
}

#[cfg(test)]
mod tests {
    use crate::utils::regex_options::{create_regex, regex_check};

    #[test]
    fn test_create_regex() {
        let reg = String::from(r".*");
        let regex = create_regex(&reg).unwrap();
        assert_eq!(regex.as_str(), ".*");
    }

    #[test]
    fn test_regex_check() {
        let reg = String::from(r".*");
        let regex = create_regex(&reg).unwrap();
        assert_eq!(regex.as_str(), ".*");

        let path = "Downloads\\file.exe";
        let result = regex_check(&regex, path);
        assert_eq!(result, true);
    }
}
