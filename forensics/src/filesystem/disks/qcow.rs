use crate::filesystem::disks::error::DiskError;
use calf::calf::CalfReader;
use log::error;
use std::{fs::File, io::BufReader};

/// Return a reader to access the OS of a QCOW disk image
pub(crate) fn qcow_reader(path: &str) -> Result<CalfReader<File>, DiskError> {
    let file = match File::open(path) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to open QCOW file: {err:?}");
            return Err(DiskError::Qcow);
        }
    };
    let buf = BufReader::new(file);
    let calf = CalfReader { fs: buf };

    Ok(calf)
}

#[cfg(test)]
mod tests {
    use crate::filesystem::disks::qcow::qcow_reader;
    use calf::{
        bootsector::boot::PartitionType,
        calf::{CalfReaderAction, QcowInfo},
        format::header::CalfHeader,
    };
    use ext4_fs::extfs::{Ext4Reader, Ext4ReaderAction};
    use std::{io::BufReader, path::PathBuf};

    #[test]
    fn test_qcow_reader() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/disks/qcow/test.qcow");

        let mut reader = qcow_reader(test_location.to_str().unwrap()).unwrap();
        let info = QcowInfo {
            header: reader.header().unwrap(),
            level1_table: reader.level1_entries().unwrap(),
        };
        let mut os_reader = reader.os_reader(&info).unwrap();
        let boot_info = os_reader.get_boot_info().unwrap();
        for entry in boot_info.partitions {
            if entry.partition_type != PartitionType::Linux {
                continue;
            }
            let os_reader = reader.os_reader(&info).unwrap();

            let test = BufReader::new(os_reader);

            let mut ext4_reader = Ext4Reader::new(test, 4096, entry.offset_start).unwrap();
            let superblock = ext4_reader.superblock().unwrap();

            assert_eq!(superblock.block_size, 0);
            assert_eq!(superblock.inode_size, 256);
            assert_eq!(superblock.volume_name, "");
            assert_eq!(superblock.last_mount_path, "/mnt/vda1");
            assert_eq!(
                superblock.filesystem_id,
                "1df77bd7-474e-497d-8061-409a5441efa0"
            );
            let root = ext4_reader.root().unwrap();

            for value in root.children {
                let stat_value = ext4_reader.stat(value.inode).unwrap();
                assert_ne!(stat_value.created, 0)
            }
        }
    }
}
