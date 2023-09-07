use uuid::Uuid;

/// Create a UUID and return as a string
pub(crate) fn generate_uuid() -> String {
    Uuid::new_v4().hyphenated().to_string()
}

#[cfg(test)]
mod tests {
    use super::generate_uuid;

    #[test]
    fn test_generate_uuid() {
        let result = generate_uuid();
        assert!(!result.is_empty())
    }
}
