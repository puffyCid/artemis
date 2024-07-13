use super::{
    error::FileError,
    filelisting::{get_filelist, FileArgs},
};
use crate::structs::{artifacts::os::files::FileOptions, toml::Output};
use common::files::Hashes;
use log::error;

/// Get a filelisting based on provided options
pub(crate) fn filelisting(
    output: &mut Output,
    filter: &bool,
    options: &FileOptions,
) -> Result<(), FileError> {
    let hashes = Hashes {
        md5: options.md5.unwrap_or(false),
        sha1: options.sha1.unwrap_or(false),
        sha256: options.sha256.unwrap_or(false),
    };
    let args = FileArgs {
        start_directory: options.start_path.clone(),
        depth: options.depth.unwrap_or(1) as usize,
        metadata: options.metadata.unwrap_or(false),
        yara: options.yara.as_ref().unwrap_or(&String::new()).to_string(),
        path_filter: options
            .regex_filter
            .as_ref()
            .unwrap_or(&String::new())
            .to_string(),
    };
    let artifact_result = get_filelist(&args, &hashes, output, filter);
    match artifact_result {
        Ok(results) => Ok(results),
        Err(err) => {
            error!("[artemis-core] Failed to get file listing: {err:?}");
            Err(FileError::Filelisting)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::files::artifact::filelisting,
        structs::{artifacts::os::files::FileOptions, toml::Output},
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
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
    #[cfg(target_family = "unix")]
    fn test_filelisting_unix() {
        let mut output = output_options("file_test", "local", "./tmp", false);

        let file_config = FileOptions {
            start_path: String::from("/"),
            depth: Some(1),
            metadata: Some(false),
            md5: Some(false),
            sha1: Some(false),
            sha256: Some(false),
            regex_filter: Some(String::new()),
            yara: None,
        };
        let status = filelisting(&mut output, &false, &file_config).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_filelisting_windows() {
        let mut output = output_options("file_test", "local", "./tmp", false);

        let file_config = FileOptions {
            start_path: String::from("C:\\"),
            depth: Some(1),
            metadata: Some(false),
            md5: Some(false),
            sha1: Some(false),
            sha256: Some(false),
            regex_filter: Some(String::new()),
        };
        let status = filelisting(&mut output, &false, &file_config).unwrap();
        assert_eq!(status, ());
    }
}
