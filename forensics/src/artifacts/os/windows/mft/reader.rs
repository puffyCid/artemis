use super::error::MftError;
use crate::filesystem::{
    files::file_reader,
    ntfs::{raw_files::raw_reader, sector_reader::SectorReader},
};
use log::error;
use ntfs::{Ntfs, NtfsFile};
use std::{fs::File, io::BufReader};

/// Setup Windows MFT reader using NTFS parser
pub(crate) fn setup_mft_reader_windows<'a>(
    ntfs_file: &'a Ntfs,
    fs: &mut BufReader<SectorReader<File>>,
    path: &str,
) -> Result<NtfsFile<'a>, MftError> {
    let reader_result = raw_reader(path, ntfs_file, fs);
    let ntfs_file = match reader_result {
        Ok(result) => result,
        Err(err) => {
            error!("[mft] Could not setup reader: {err:?}");
            return Err(MftError::ReadFile);
        }
    };

    Ok(ntfs_file)
}

/// Setup MFT using normal reader
pub(crate) fn setup_mft_reader(path: &str) -> Result<File, MftError> {
    let reader_result = file_reader(path);
    let reader = match reader_result {
        Ok(reader) => reader,
        Err(err) => {
            error!("[mft] Could not setup API reader: {err:?}");
            return Err(MftError::ReadFile);
        }
    };

    Ok(reader)
}
