use crate::{
    structs::{artifacts::os::windows::MftOptions, toml::Output},
    utils::{environment::get_systemdrive, time::time_now},
};

use super::{error::MftError, master::parse_mft};

/**
 * Only NTFS 3.1 or higher supported
 * TODO:
 *
 * 1.5 Verify size is right? <--- done
 * 2. Add limit to cache. 1k directories <-- done
 * 3. Check for recursive parent mfts. Cache should stop that?
 *    - Check for recursive attribute list
 * 5. Fix clippy
 * 6. Missing entries when compared to mft_dump
 * 7. Remove dupes?
 * 8. Do not include base_extensions (ATTRIBUTE_LIST) entries in the final output. Instead combine them with base_entries
 *    1. Requires that we parse the MFT twice? :/
 *    2. First parse MFT and only grab the base_extensions. Cache them.
 *    3. Next parse the MFT and only grab the base_entries.
 *    4. Combine the base_extensions with the base_entry. Via index and sequence matching?
 *    5. Once combined you have all of your attributes
 * 9. Remove panics
 */

pub(crate) fn grab_mft(
    options: &MftOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), MftError> {
    let start_time = time_now();

    let path = if let Some(file) = &options.alt_file {
        return parse_mft(file, output, filter, &start_time);
    } else {
        // Check if alternative drive letter provided
        if let Some(drive) = &options.alt_drive {
            format!("{drive}\\$MFT")
        } else {
            // Otherwise try to get the SystemDrive
            let drive = get_systemdrive().unwrap_or('C');
            format!("{drive}\\$MFT")
        }
    };

    parse_mft(&path, output, filter, &start_time)
}

#[cfg(test)]
mod tests {
    use super::grab_mft;
    use crate::structs::{artifacts::os::windows::MftOptions, toml::Output};

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
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

    #[test]
    fn test_grab_mft() {
        let options = MftOptions {
            alt_drive: None,
            alt_file: None,
        };
        let mut output = output_options("mft_temp", "local", "./tmp", false);
        grab_mft(&options, &mut output, &false).unwrap();
    }
}
