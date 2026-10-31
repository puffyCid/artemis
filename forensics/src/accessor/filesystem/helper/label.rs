use crate::accessor::io::reader::directory_from_display;

/// Paths for a single filelisting record
pub(crate) struct LabeledPath {
    /// Full path to the entry
    pub(crate) full_path: String,
    /// Full path to the entry
    /// With the accessor prefix
    pub(crate) display_path: String,
    /// Parent folder of the entry
    pub(crate) directory: String,
    /// Drive associated with the entry
    pub(crate) drive: String,
}

/// Turn a filesystem path into `LabeledPath`
pub(crate) struct PathLabel {
    /// Return a `LabeledPath` from a string
    render: Box<dyn Fn(&str) -> LabeledPath>,
}

impl PathLabel {
    /// Return a `LabeledPath` from a driver letter
    pub(crate) fn drive(drive: char) -> Self {
        Self::custom(move |ntfs_display| {
            let scheme_path = format!("ntfs:{ntfs_display}");

            LabeledPath {
                full_path: ntfs_display.to_string(),
                display_path: scheme_path.clone(),
                directory: directory_from_display(&scheme_path),
                drive: format!("{drive}:"),
            }
        })
    }

    /// Return a `PathLabel` from a custom input string
    pub(crate) fn custom(render: impl Fn(&str) -> LabeledPath + 'static) -> Self {
        Self {
            render: Box::new(render),
        }
    }

    /// Render the input as a `LabeledPath`
    pub(crate) fn render(&self, filesystem_path: &str) -> LabeledPath {
        // Call the function for the `PathLabel` render field
        // <https://doc.rust-lang.org/reference/expressions/field-expr.html#field-access-expressions>
        (self.render)(filesystem_path)
    }
}
