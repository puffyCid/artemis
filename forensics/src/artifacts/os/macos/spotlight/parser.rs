/**
 * macOS `Spotlight` is an indexing service for tracking files and content.
 * The `Spotlight` database can contain a huge amount of metadata associated with the indexed content such as:
 * - Timestamps
 * - Partial file content
 * - File type and much more
 *
 * References:
 * `https://forensicsandsecurity.com/papers/SpotlightMacForensicsSlides.pdf`
 * `https://en.wikipedia.org/wiki/Spotlight_(Apple)`
 * `https://github.com/libyal/dtformats/blob/main/documentation/Apple%20Spotlight%20store%20database%20file%20format.asciidoc`
 *
 * Other parsers:
 * `https://github.com/ydkhatri/spotlight_parser`
 * `https://github.com/ydkhatri/mac_apt`
 */
use super::{error::SpotlightError, light::parse_spotlight};
use crate::{output::manager::OutputManager, structs::artifacts::os::macos::SpotlightOptions};
use tracing::error;

/// Dump the Spotlight database. Requires root
pub(crate) fn grab_spotlight(
    options: &SpotlightOptions,
    manager: &mut OutputManager,
) -> Result<(), SpotlightError> {
    let paths = if let Some(alt_dir) = &options.alt_dir {
        vec![format!("{alt_dir}/*")]
    } else {
        let mut additional_stores = false;
        if let Some(extra) = &options.include_additional {
            additional_stores = *extra;
        }

        let mut default_paths = vec![String::from(
            "/System/Volumes/Data/.Spotlight-V100/Store-V*/*/*",
        )];
        if additional_stores {
            default_paths.append(&mut vec![
                String::from("/Users/*/Library/Caches/com.apple.helpd/index.spotlightV*/*"),
                String::from("/Users/*/Library/Metadata/CoreSpotlight/index.spotlightV*/*"),
                String::from("/Users/*/Library/Developer/Xcode/DocumentationCache/*/*/DeveloperDocumentation.index/*"),
                String::from("/Users/*/Library/Metadata/CoreSpotlight/*/index.spotlightV*/*"),
                String::from("/Users/*/Library/Caches/com.apple.helpd/*/index.spotlightV*/*"),
            ]);
        }
        default_paths
    };

    for glob in paths {
        if let Err(err) = parse_spotlight(&glob, manager, options) {
            error!["Could not parse spotlight for '{glob}': {err:?}"];
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        artifacts::os::macos::spotlight::parser::grab_spotlight, output::manager::OutputManager,
        structs::artifacts::os::macos::SpotlightOptions,
    };
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputConfig {
        OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Csv,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        }
    }

    #[test]
    fn test_grab_spotlight() {
        let output = output_options("spotlight_test", "./tmp", false);
        let mut manage = OutputManager::new(output).unwrap();
        grab_spotlight(
            &SpotlightOptions {
                alt_dir: None,
                include_additional: Some(true),
            },
            &mut manage,
        )
        .unwrap();
    }
}
