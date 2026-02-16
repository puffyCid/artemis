use super::info::get_info;
use crate::{
    artifacts::os::{macos::artifacts::output_data, systeminfo::error::SystemInfoError},
    structs::toml::Output,
    utils::time,
};
use log::error;

/// Get basic sysinfo for a system
pub(crate) fn systeminfo(output: &mut Output, filter: bool) -> Result<(), SystemInfoError> {
    let start_time = time::time_now();

    let system_data = get_info();
    let serde_data_result = serde_json::to_value(system_data);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Failed to serialize systeminfo: {err:?}");
            return Err(SystemInfoError::Serialize);
        }
    };

    let output_name = "systeminfo";
    let status = output_data(&mut serde_data, output_name, output, start_time, filter);

    if let Err(result) = status {
        error!("[forensics] Could not output sysinfo data: {result:?}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{artifacts::os::systeminfo::artifact::systeminfo, structs::toml::Output};

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            endpoint_id: String::from("abcd"),
            output: output.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_systeminfo() {
        let mut output = output_options("system_test", "local", "./tmp", false);

        let status = systeminfo(&mut output, false).unwrap();
        assert_eq!(status, ());
    }
}
