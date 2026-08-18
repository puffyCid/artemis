use crate::accessor::{
    error::{AccessorError, AccessorResult},
    location::path::InnerPath,
};
use glob::Pattern;
use std::path::PathBuf;

/// A structure we use to determine if we should descend into directory when globbing
///
/// Globbing files and directories can cause extremely long runtimes if not properly "gated".
/// For example the pattern `C:/Users/*/AppData/Local/Microsoft/Windows/UsrClass.dat`, should only descend into the
/// directories `AppData/Local/...`. All other directories need to be ignored.
///
/// When descending, we need to make sure the directory we want to descend into matches our glob pattern. The
/// `DescendGuard` should prevent unrelated directory descent by checking to make sure the directory is part of the glob pattern
/// and path.
///
/// For recursive globbing, there is **no** guard!!!
pub(crate) struct DescendGuard {
    /// Per component matchers for the glob pattern
    ///
    /// This is `None` for recursive globbing
    components: Option<Vec<Pattern>>,
}

impl DescendGuard {
    /// Create a `DescendGuard` from a normalized glob pattern
    pub(crate) fn new(normalized: &str) -> AccessorResult<Self> {
        // If its a recursive glob then we descend all directories
        if is_recursive(normalized) {
            return Ok(Self { components: None });
        }

        let mut components = Vec::new();
        let component_iter = path_components(normalized);
        // Extract the directories into individual pattern components
        for component in component_iter {
            let pattern = Pattern::new(component)
                .map_err(|err| AccessorError::bad_glob(component, err.to_string()))?;
            components.push(pattern);
        }

        Ok(Self {
            components: Some(components),
        })
    }

    /// Only descend if a directory is a prefix of the pattern
    ///
    /// For example for the glob `C:/Users/*/AppData/Local/app/test.txt`
    /// The directory `C:/Users/dev/AppData/Local` we would descend. But
    /// the directory `C:/Users/dev/AppData/Roaming` we would reject
    ///
    /// Recursive globbing **ALWAYS DESCENDS!**
    pub(crate) fn should_descend(
        &self,
        relative: &str,
        depth: usize,
        max_depth: Option<usize>,
    ) -> bool {
        // Quick check if descent depth is larger than our current glob pattern
        if !descend(depth, max_depth) {
            return false;
        }

        // If None. Its a recursive glob. We always descend those
        let Some(pattern_components) = &self.components else {
            return true;
        };

        // Extract the path into component parts
        let path_parts: Vec<&str> = path_components(relative).collect();

        if path_parts.len() > pattern_components.len() {
            return false;
        }
        // Compare each component against our glob
        path_parts
            .iter()
            .zip(pattern_components.iter())
            .all(|(part, pattern)| pattern.matches(part))
    }
}

/// Apply a consistent glob separator
pub(crate) fn normalize_glob_pattern(pattern: &str) -> String {
    pattern.replace('\\', "/").trim_matches('/').to_string()
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
    path_components(path).count()
}

/// Extract normalized path into individual components
fn path_components(normalized: &str) -> impl Iterator<Item = &str> {
    normalized.split('/').filter(|part| !part.is_empty())
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
            DescendGuard, append_inner_path, descend, glob_max_depth, is_recursive, join_relative,
            normalize_glob_pattern, path_component_count,
        },
        location::path::InnerPath,
    };
    use std::path::PathBuf;

    #[test]
    fn test_normalize_glob_pattern() {
        assert_eq!(normalize_glob_pattern("\\test\\hello"), "test/hello");
    }

    #[test]
    fn test_glob_max_depth() {
        assert_eq!(glob_max_depth("/*/*"), Some(2));
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
        assert_eq!(path_component_count("/test/test.txt"), 2);
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

    #[test]
    fn test_descend_guard() {
        let normalized =
            normalize_glob_pattern("*\\AppData\\Local\\Microsoft\\Windows\\[uU]srClass.dat");
        let guard = DescendGuard::new(&normalized).unwrap();
        let max_depth = glob_max_depth(&normalized);

        assert!(guard.should_descend("dev", path_component_count("dev"), max_depth));
        // Unrelated child directories should be ignored.
        assert!(guard.should_descend(
            "dev/AppData",
            path_component_count("dev/AppData"),
            max_depth
        ));

        assert!(!guard.should_descend(
            "dev/Documents",
            path_component_count("dev/Documents"),
            max_depth
        ));

        // Nested unrelated directories are never followed
        assert!(!guard.should_descend(
            "dev/Documents/hayabusa-sample-evtx",
            path_component_count("dev/Documents/hayabusa-sample-evtx"),
            max_depth
        ));

        assert!(guard.should_descend(
            "dev/AppData/Local/Microsoft/Windows",
            path_component_count("dev/AppData/Local/Microsoft/Windows"),
            max_depth
        ));

        assert!(!guard.should_descend(
            "dev/AppData/Test/Microsoft/Windows",
            path_component_count("dev/AppData/Local/Microsoft/Windows"),
            max_depth
        ));
    }

    #[test]
    fn test_descend_guard_recursive() {
        let normalized = normalize_glob_pattern("**\\*.evtx");
        let guard = DescendGuard::new(&normalized).unwrap();
        let max_depth = glob_max_depth(&normalized);

        assert!(guard.should_descend("test", 1, max_depth));
        assert!(guard.should_descend("test/deep/unrelated", 3, max_depth));
    }
}
