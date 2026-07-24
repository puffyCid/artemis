use crate::accessor::location::path::InnerPath;
use std::path::PathBuf;

/// Apply a consistent glob separator
pub(crate) fn normalize_glob_pattern(pattern: &str) -> String {
    pattern.replace('\\', "/")
}

/// Max directory depth to descend for a pattern
///
/// Recursive globs '**' do not have a depth cap
pub(crate) fn glob_max_depth(path: &str) -> Option<usize> {
    if is_recursive(path) {
        None
    } else {
        Some(path_component_count(path))
    }
}

/// Determine if our normalized glob pattern is a recursive glob
pub(crate) fn is_recursive(path: &str) -> bool {
    path.split('/').any(|p| p == "**")
}

/// Determine depth of starting directory
pub(crate) fn path_component_count(path: &str) -> usize {
    if path.is_empty() {
        0
    } else {
        path.split('/').count()
    }
}

/// Determine if we should descend to next directory if doing recursive glob or nested glob pattern
pub(crate) fn descend(depth: usize, max_depth: Option<usize>) -> bool {
    match max_depth {
        None => true,
        Some(max) => depth < max,
    }
}

/// Builds the path to compare against our glob pattern
pub(crate) fn join_relative(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}/{name}")
    }
}

/// Combine starting directory with any directory matches from glob
pub(crate) fn append_inner_path(base: &InnerPath, name: &str) -> InnerPath {
    if base.is_empty() {
        InnerPath::new(PathBuf::from(name))
    } else {
        InnerPath::new(base.as_path().join(name))
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::{
        filesystem::helper::glob::{
            append_inner_path, descend, glob_max_depth, is_recursive, join_relative,
            normalize_glob_pattern, path_component_count,
        },
        location::path::InnerPath,
    };
    use std::path::PathBuf;

    #[test]
    fn test_normalize_glob_pattern() {
        assert_eq!(normalize_glob_pattern("\\test\\hello"), "/test/hello");
    }

    #[test]
    fn test_glob_max_depth() {
        assert_eq!(glob_max_depth("/*/*"), Some(3));
        assert_eq!(glob_max_depth("/**/*.txt"), None);
    }

    #[test]
    fn test_is_recursive() {
        assert!(is_recursive("/**/*"));
        assert!(!is_recursive("/*/*.txt"))
    }

    #[test]
    fn test_path_component_count() {
        assert_eq!(path_component_count(""), 0);
        assert_eq!(path_component_count("path"), 1);
        assert_eq!(path_component_count("/test/test.txt"), 3);
    }

    #[test]
    fn test_descend() {
        assert!(descend(1, None));
        assert!(!descend(6, Some(4)));
    }

    #[test]
    fn test_join_relative() {
        assert_eq!(join_relative("test/test", "hello"), "test/test/hello");
        assert_eq!(join_relative("", "hello"), "hello");
    }

    #[test]
    fn test_append_inner_path() {
        let inner = InnerPath::new(PathBuf::from("/test/test"));

        assert!(
            append_inner_path(&inner, "test")
                .display()
                .starts_with("/test/test")
        );

        assert_eq!(
            append_inner_path(&InnerPath::empty(), "name").display(),
            "name"
        );
    }
}
