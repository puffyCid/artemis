use super::error::CronError;
use crate::filesystem::directory::is_directory;
use crate::filesystem::files::list_files;
use log::{error, warn};
use serde::Serialize;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

#[derive(Debug, Serialize)]
pub(crate) struct CronFile {
    pub(crate) cron_data: Vec<Cron>,
    pub(crate) path: String,
    pub(crate) contents: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Cron {
    pub(crate) hour: String,
    pub(crate) min: String,
    pub(crate) day: String,
    pub(crate) month: String,
    pub(crate) weekday: String,
    pub(crate) command: String,
}

impl Cron {
    /// Parse all Cron files
    pub(crate) fn parse_cron() -> Result<Vec<CronFile>, CronError> {
        #[cfg(target_os = "macos")]
        let start_path = "/private/var/at/jobs/";

        #[cfg(target_os = "linux")]
        let start_path = "/var/spool/cron/crontabs/";

        if !is_directory(start_path) {
            return Ok(Vec::new());
        }

        let cron_file_result = list_files(start_path);
        let cron_files = match cron_file_result {
            Ok(result) => result,
            Err(err) => {
                error!("[cron] Failed to get cron files at {start_path}: {err:?}");
                return Err(CronError::ReadPath);
            }
        };

        let mut cron_vec: Vec<CronFile> = Vec::new();
        // Loop through all files found in directory
        for cron_entry in cron_files {
            let cron_data_result = Cron::get_cron_data(&cron_entry);
            let cron_data = match cron_data_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[cron] Failed to parse cron data {err:?}");
                    continue;
                }
            };
            cron_vec.push(cron_data);
        }
        Ok(cron_vec)
    }

    /// Read cron file line by line
    fn get_cron_data(path: &str) -> Result<CronFile, CronError> {
        let cron_file_result = File::open(path);
        let cron_file = match cron_file_result {
            Ok(results) => results,
            Err(err) => {
                error!("[cron] Failed to open cron file {path}, error: {err:?}");
                return Err(CronError::FileRead);
            }
        };

        let cron_reader = BufReader::new(cron_file);
        let mut cron_file = CronFile {
            cron_data: Vec::new(),
            path: String::new(),
            contents: Vec::new(),
        };
        let mut cron_contents: Vec<String> = Vec::new();

        // Parse each line
        for (_, entry) in cron_reader.lines().enumerate() {
            let line_entry = entry;
            let cron_entry = match line_entry {
                Ok(result) => result,
                Err(err) => {
                    warn!("[cron] Failed to read cron line in file {path}, error: {err:?}");
                    continue;
                }
            };
            // Also track commented and other lines
            cron_contents.push(cron_entry.clone());

            if cron_entry.starts_with('#') || cron_entry.is_empty() {
                continue;
            }

            let cron_result = Cron::get_cron_entry(&cron_entry);
            match cron_result {
                Ok(cron_data) => {
                    cron_file.cron_data.push(cron_data);
                }
                Err(err) => warn!("[cron] Failed to parse cron line entry: {err:?}"),
            }
        }
        cron_file.path = path.to_string();
        cron_file.contents = cron_contents;
        Ok(cron_file)
    }

    // Parse each cron field
    fn get_cron_entry(cron_line: &str) -> Result<Cron, CronError> {
        let mut cron_data = Cron {
            hour: String::new(),
            min: String::new(),
            day: String::new(),
            month: String::new(),
            weekday: String::new(),
            command: String::new(),
        };

        // Attempt to breakdown each field
        let cron_fields: Vec<&str> = cron_line.splitn(6, ' ').collect();

        for (key, fields) in cron_fields.iter().map(|&s| s.to_string()).enumerate() {
            match key {
                0 => cron_data.min = fields,
                1 => cron_data.hour = fields,
                2 => cron_data.day = fields,
                3 => cron_data.month = fields,
                4 => cron_data.weekday = fields,
                5 => cron_data.command = fields,
                _ => continue,
            }
        }
        Ok(cron_data)
    }
}

#[cfg(test)]
mod tests {
    use super::Cron;
    use std::path::PathBuf;

    #[test]
    fn test_get_cron_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/unix/cron/catalina");

        let results = Cron::get_cron_data(&test_location.display().to_string()).unwrap();
        assert_eq!(results.cron_data.len(), 1);
        assert_eq!(results.cron_data[0].day, "*");
        assert_eq!(results.cron_data[0].min, "*");
        assert_eq!(results.cron_data[0].hour, "*");
        assert_eq!(results.cron_data[0].weekday, "*");
        assert_eq!(results.cron_data[0].hour, "*");
        assert_eq!(results.cron_data[0].command,  "/Users/catalina/Library/Python/3.8/lib/python/site-packages/poisonapple/auxiliary/poisonapple.sh Cron # test");
    }

    #[test]
    fn test_get_cron_entry() {
        let data = "10 * 9 * 12 ping 8.8.8.8";
        let result = Cron::get_cron_entry(data).unwrap();
        assert_eq!(result.min, "10");
        assert_eq!(result.hour, "*");
        assert_eq!(result.day, "9");
        assert_eq!(result.month, "*");
        assert_eq!(result.weekday, "12");
        assert_eq!(result.command, "ping 8.8.8.8");
    }

    #[test]
    #[ignore = "requires live cron data"]
    fn test_parse_cron() {
        let result = Cron::parse_cron().unwrap();
        assert!(result.len() >= 1);
    }
}
