use crate::{
    artifacts::os::linux::ext4::{
        error::Ext4Error,
        parser::{Ext4Params, ext4_output, get_root, walk_ext4},
    },
    filesystem::disks::qcow::qcow_reader,
    structs::toml::Output,
};
use calf::{
    bootsector::boot::{GptPartition, GuidNames, PartitionType},
    calf::{CalfReader, CalfReaderAction, QcowInfo},
    format::header::CalfHeader,
};
use ext4_fs::extfs::Ext4Reader;
use log::error;
use std::{fs::File, io::BufReader};

/// Parse QCOW disk image
pub(crate) fn qcow_ext4(
    options: &mut Ext4Params,
    output: &mut Output,
    start_time: u64,
) -> Result<(), Ext4Error> {
    let mut reader = match qcow_reader(&options.device.replace("qcow://", "")) {
        Ok(result) => result,
        Err(_err) => {
            return Err(Ext4Error::QcowDevice);
        }
    };

    let header = match reader.header() {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Could not parse the QCOW header: {err:?}");
            return Err(Ext4Error::QcowDevice);
        }
    };

    let level1_table = match reader.level1_entries() {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Could not parse the QCOW level one table: {err:?}");
            return Err(Ext4Error::QcowDevice);
        }
    };

    let info = QcowInfo {
        header,
        level1_table,
    };

    let mut boot_reader = match reader.os_reader(&info) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Could not read the QCOW ext4 boot info: {err:?}");
            return Err(Ext4Error::QcowExt4Boot);
        }
    };

    let boot_info = match boot_reader.get_boot_info() {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Could not get the QCOW ext4 boot info: {err:?}");
            return Err(Ext4Error::QcowExt4Boot);
        }
    };

    if let Some(gpt) = boot_info.gpt_partitions {
        return gpt_partitions(&gpt, options, output, start_time, &mut reader, &info);
    }

    for entry in boot_info.partitions {
        if entry.partition_type != PartitionType::Linux {
            continue;
        }
        let os_reader = match reader.os_reader(&info) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not read the QCOW ext4 linux partition: {err:?}");
                continue;
            }
        };

        let buff_read = BufReader::new(os_reader);

        let mut ext4_reader = match Ext4Reader::new(buff_read, 4096, entry.offset_start) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not setup the QCOW ext4 linux reader: {err:?}");
                continue;
            }
        };

        let root = match get_root(&mut ext4_reader) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not get the QCOW ext4 linux partition root: {err:?}");
                continue;
            }
        };
        options
            .cache
            .push(root.name.trim_end_matches('/').to_string());
        walk_ext4(&root, &mut ext4_reader, options, output);
        if !options.filelist.is_empty() {
            ext4_output(&options.filelist, output, start_time, options.filter);
        }
        options.filelist.clear();
        options.cache.pop();
    }
    Ok(())
}

/// Handle GPT partition layout
fn gpt_partitions(
    partitions: &[GptPartition],
    options: &mut Ext4Params,
    output: &mut Output,
    start_time: u64,
    reader: &mut CalfReader<File>,
    info: &QcowInfo,
) -> Result<(), Ext4Error> {
    for entry in partitions {
        if entry.platform != GuidNames::Linux {
            continue;
        }
        let os_reader = match reader.os_reader(info) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not read the QCOW ext4 linux GPT partition: {err:?}");
                continue;
            }
        };

        let buff_read = BufReader::new(os_reader);

        let mut ext4_reader = match Ext4Reader::new(buff_read, 4096, entry.offset_start) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not setup the QCOW ext4 linux reader: {err:?}");
                continue;
            }
        };

        let root = match get_root(&mut ext4_reader) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not get the QCOW ext4 linux GPT partition root: {err:?}");
                continue;
            }
        };
        options
            .cache
            .push(root.name.trim_end_matches('/').to_string());
        walk_ext4(&root, &mut ext4_reader, options, output);
        if !options.filelist.is_empty() {
            ext4_output(&options.filelist, output, start_time, options.filter);
        }
        options.filelist.clear();
        options.cache.pop();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::linux::ext4::{disks::qcow_ext4, parser::Ext4Params},
        structs::toml::Output,
        utils::regex_options::create_regex,
    };
    use ext4_fs::structs::Ext4Hash;
    use std::path::PathBuf;

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
    fn test_qcow_ext4() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/disks/qcow/test.qcow");

        let mut output = output_options("ext4_qcow_temp", "local", "./tmp", false);
        let mut options = Ext4Params {
            start_path: String::from("/"),
            depth: 99,
            device: test_location.display().to_string(),
            start_path_depth: 0,
            path_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            file_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            cache: Vec::new(),
            filelist: Vec::new(),
            hashing: Ext4Hash {
                md5: true,
                sha1: false,
                sha256: false,
            },
            start_time: 0,
            filter: false,
        };

        qcow_ext4(&mut options, &mut output, 0).unwrap()
    }

    #[test]
    #[should_panic(expected = "QcowExt4Boot")]
    fn test_qcow_ext4_bad_qcow() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/images/ext4/test.img");

        let mut output = output_options("ext4_qcow_temp", "local", "./tmp", false);
        let mut options = Ext4Params {
            start_path: String::from("/"),
            depth: 99,
            device: test_location.display().to_string(),
            start_path_depth: 0,
            path_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            file_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            cache: Vec::new(),
            filelist: Vec::new(),
            hashing: Ext4Hash {
                md5: true,
                sha1: false,
                sha256: false,
            },
            start_time: 0,
            filter: false,
        };

        qcow_ext4(&mut options, &mut output, 0).unwrap()
    }
}
