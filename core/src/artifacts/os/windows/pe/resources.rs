use crate::filesystem::files::read_file;
use log::error;
use pelite::{
    Error, PeFile,
    resources::{Directory, Name},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct EventLogResource {
    pub(crate) mui_data: Vec<u8>,
    pub(crate) wevt_data: Vec<u8>,
    pub(crate) message_data: Vec<u8>,
    pub(crate) path: String,
}

/// Read the eventlog resource data from PE file
pub(crate) fn read_eventlog_resource(path: &str) -> Result<EventLogResource, Error> {
    let pe_result = read_file(path);
    let pe_bytes = match pe_result {
        Ok(result) => result,
        Err(err) => {
            error!("[pe] Could not read file {path}: {err:?}");
            return Err(Error::Invalid);
        }
    };

    let pe = PeFile::from_bytes(&pe_bytes)?;
    let message_table = Name::Id(11);
    let mui = Name::Wide(&[77, 85, 73]);
    let wevt_template = Name::Wide(&[87, 69, 86, 84, 95, 84, 69, 77, 80, 76, 65, 84, 69]);
    let mut message_source = EventLogResource {
        mui_data: Vec::new(),
        wevt_data: Vec::new(),
        message_data: Vec::new(),
        path: path.to_string(),
    };

    if let Ok(resources) = pe.resources() {
        let root = resources.root()?;
        for entry in root.entries() {
            if entry.name()? != wevt_template
                && entry.name()? != message_table
                && entry.name()? != mui
            {
                continue;
            }

            if entry.is_dir() {
                if let Some(entry_dir) = entry.entry()?.dir() {
                    if entry.name()? == wevt_template {
                        message_source.wevt_data = read_dir(&entry_dir)?;
                    } else if entry.name()? == message_table {
                        message_source.message_data = read_dir(&entry_dir)?;
                    } else if entry.name()? == mui {
                        message_source.mui_data = read_dir(&entry_dir)?;
                    }
                    continue;
                }

                error!("[pe] Got None value on root resource directory");
                return Err(Error::Invalid);
            }

            if let Some(data) = entry.entry()?.data() {
                if entry.name()? == wevt_template {
                    message_source.wevt_data = data.bytes()?.to_vec();
                } else if entry.name()? == message_table {
                    message_source.message_data = data.bytes()?.to_vec();
                } else if entry.name()? == mui {
                    message_source.mui_data = data.bytes()?.to_vec();
                }
                continue;
            }
            error!("[pe] Got None value on root resource bytes");
            return Err(Error::Invalid);
        }
    }
    Ok(message_source)
}

/// Read nested resource directory
fn read_dir(dir: &Directory<'_>) -> Result<Vec<u8>, Error> {
    let mut res_bytes = Vec::new();
    for entry in dir.entries() {
        let res_entry = entry.entry()?;
        if entry.is_dir() {
            if let Some(entry_dir) = res_entry.dir() {
                return read_dir(&entry_dir);
            }

            error!("[pe] Got None value on resource directory");
            return Err(Error::Invalid);
        }

        if let Some(data) = res_entry.data() {
            res_bytes = data.bytes()?.to_vec();
        } else {
            error!("[pe] Got None value on resource bytes");
            return Err(Error::Invalid);
        }
    }

    Ok(res_bytes)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::read_dir;
    use crate::{
        artifacts::os::windows::pe::resources::read_eventlog_resource, filesystem::files::read_file,
    };
    use pelite::PeFile;
    use std::path::PathBuf;

    #[test]
    fn test_read_eventlog_resource() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\pe\\resources\\wer.dll");

        let results = read_eventlog_resource(test_location.to_str().unwrap()).unwrap();
        assert_eq!(results.wevt_data.len(), 9538);
    }

    #[test]
    fn test_read_eventlog_resource_message_table() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\pe\\resources\\eventlog_provider.dll");

        let results = read_eventlog_resource(test_location.to_str().unwrap()).unwrap();
        assert_eq!(results.message_data.len(), 180);
    }

    #[test]
    fn test_read_dir() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\pe\\resources\\eventlog_provider.dll");

        let bytes = read_file(test_location.to_str().unwrap()).unwrap();
        let pe = PeFile::from_bytes(&bytes).unwrap();

        if let Ok(resources) = pe.resources() {
            let root = resources.root().unwrap();
            for entry in root.entries() {
                if entry.is_dir() {
                    if let Some(entry_dir) = entry.entry().unwrap().dir() {
                        let results = read_dir(&entry_dir).unwrap();
                        assert!(results.len() > 100);
                    }
                }
            }
        }
    }
}
