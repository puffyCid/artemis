/**
 * Windows System Resource Utilization Monitor (`SRUM`) is a service that tracks application resource usage  
 * The service tracks things like time running, bytes sent, bytes received, energy usage, and lots more  
 *
 * This service was introduced in Windows 8 and is stored in an ESE database at `C:\\Windows\System32\\sru\\SRUDB.dat`  
 * On Windows 8 some of the data can be found in the Registry too (temporary storage before writing to SRUDB.dat), but in later versions of Windows the data is no longer in the Registry
 *
 * References:  
 * `https://github.com/libyal/esedb-kb/blob/main/documentation/System%20Resource%20Usage%20Monitor%20(SRUM).asciidoc`  
 * `https://velociraptor.velocidex.com/digging-into-the-system-resource-usage-monitor-srum-afbadb1a375`
 *
 * Other parsers:  
 * `https://github.com/Velocidex/velociraptor`  
 * `https://ericzimmerman.github.io/`
 */
use super::{
    error::SrumError,
    resource::{get_srum, parse_srum},
};
use crate::{
    structs::{artifacts::os::windows::SrumOptions, toml::Output},
    utils::environment::get_systemdrive,
};
use log::error;
use serde_json::Value;

/**
 * Grab the `SRUM` data from the default or an alternative path  
 * We then dump all of the tables associated with `SRUM`
 */
pub(crate) async fn grab_srum(
    options: &SrumOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), SrumError> {
    let path = if let Some(alt) = &options.alt_file {
        alt.clone()
    } else {
        let systemdrive_result = get_systemdrive();
        let systemdrive = match systemdrive_result {
            Ok(result) => result,
            Err(err) => {
                error!("[srum] Could not get systemdrive: {err:?}");
                return Err(SrumError::Systemdrive);
            }
        };
        format!("{systemdrive}:\\Windows\\System32\\sru\\SRUDB.dat")
    };

    parse_srum(&path, output, filter).await
}

/**
 * Grab the `SRUM` data from the provided path  
 * We then dump a single provided table associated with `SRUM` along with the `SruDbIdMapTable` index
 */
pub(crate) fn grab_srum_path(path: &str, table: &str) -> Result<Value, SrumError> {
    get_srum(path, table)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::{
        artifacts::os::windows::srum::parser::grab_srum_path,
        structs::artifacts::os::windows::SrumOptions, structs::toml::Output,
    };

    use super::grab_srum;
    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            timeline: false,
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
    fn test_grab_srum_path() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";

        let results = grab_srum_path(test_path, "{5C8CF1C7-7257-4F13-B223-970EF5939312}").unwrap();
        assert_eq!(results.is_null(), false)
    }

    #[test]
    fn test_grab_srum() {
        let options = SrumOptions { alt_file: None };
        let mut output = output_options("srum_test", "local", "./tmp", false);

        grab_srum(&options, &mut output, false).unwrap();
    }
}
