use super::sector_reader::SectorReader;
use crate::filesystem::error::FileSystemError;
use log::error;
use ntfs::Ntfs;
use std::{fs::File, io::BufReader};

pub(crate) struct NtfsParser {
    pub(crate) ntfs: Ntfs,
    pub(crate) fs: BufReader<SectorReader<File>>,
}

/// Setup NTFS parser by opening drive letter and creating Sector Reader
pub(crate) fn setup_ntfs_parser(drive_letter: &char) -> Result<NtfsParser, FileSystemError> {
    let drive_path = format!("\\\\.\\{drive_letter}:");

    let fs_result = File::open(drive_path);
    let fs = match fs_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Failed to open drive: {drive_letter}, error: {err:?}");
            return Err(FileSystemError::OpenFile);
        }
    };

    // Size used for reader setup
    let reader_size = 4096;
    let sector_reader_result = SectorReader::new(fs, reader_size);
    let sector_reader = match sector_reader_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Failed to setup sector reader, error: {err:?}");
            return Err(FileSystemError::NtfsSectorReader);
        }
    };

    let mut fs = BufReader::new(sector_reader);

    let ntfs = get_ntfs(&mut fs)?;

    let ntfs_parser = NtfsParser { ntfs, fs };
    Ok(ntfs_parser)
}

/// Create NTFS object
fn get_ntfs(fs: &mut BufReader<SectorReader<File>>) -> Result<Ntfs, FileSystemError> {
    let ntfs_result = Ntfs::new(fs);
    match ntfs_result {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[artemis-core] Failed to start NTFS parser, error: {err:?}");
            Err(FileSystemError::NtfsNew)
        }
    }
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{get_ntfs, setup_ntfs_parser};
    use crate::filesystem::ntfs::sector_reader::SectorReader;
    use std::{fs::File, io::BufReader};

    #[test]
    fn test_setup_ntfs_parser() {
        let result = setup_ntfs_parser(&'C').unwrap();
        assert_eq!(result.fs.capacity(), 8192);
        assert!(result.ntfs.size() > 10);
    }

    #[test]
    fn test_get_ntfs() {
        let drive_path = "\\\\.\\C:";
        let fs = File::open(drive_path).unwrap();

        // Size used for reader setup
        let reader_size = 4096;
        let sector_reader = SectorReader::new(fs, reader_size).unwrap();
        let mut fs = BufReader::new(sector_reader);

        let result = get_ntfs(&mut fs).unwrap();
        assert!(result.size() > 10);
    }
}
