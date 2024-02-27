use super::{error::SpotlightError, light::parse_spotlight};
use crate::{
    structs::{artifacts::os::macos::SpotlightOptions, toml::Output},
    utils::time::time_now,
};

/// Dump the Spotlight database. Requires root
pub(crate) fn grab_spotlight(
    options: &SpotlightOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), SpotlightError> {
    let paths = if let Some(alt_path) = &options.alt_path {
        vec![format!("{alt_path}/*")]
    } else {
        let mut additional_stores = &false;
        if let Some(extra) = &options.include_additional {
            additional_stores = extra;
        }

        let mut default_paths = vec![String::from(
            "/System/Volumes/Data/.Spotlight-V100/Store-V*/*/*",
        )];
        if *additional_stores {
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

    let start_time = time_now();
    for glob in paths {
        let _ = parse_spotlight(&glob, output, &start_time, filter);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::macos::spotlight::parser::grab_spotlight,
        structs::{artifacts::os::macos::SpotlightOptions, toml::Output},
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_grab_spotlight() {
        let mut output = output_options("spotlight_test", "local", "./tmp", false);

        grab_spotlight(
            &SpotlightOptions {
                alt_path: None,
                include_additional: Some(true),
            },
            &mut output,
            &false,
        )
        .unwrap();
    }
}
