use super::error::OutlookError;
use crate::filesystem::{
    files::file_reader,
    ntfs::{raw_files::raw_reader, sector_reader::SectorReader},
};
use log::error;
use ntfs::{Ntfs, NtfsFile};
use std::{fs::File, io::BufReader};

/// Setup Windows Outlook reader using NTFS parser
pub(crate) fn setup_outlook_reader_windows<'a>(
    ntfs_file: &'a Ntfs,
    fs: &mut BufReader<SectorReader<File>>,
    path: &str,
) -> Result<NtfsFile<'a>, OutlookError> {
    let reader_result = raw_reader(path, ntfs_file, fs);
    let ntfs_file = match reader_result {
        Ok(result) => result,
        Err(err) => {
            error!("[outlook] Could not setup reader: {err:?}");
            return Err(OutlookError::ReadFile);
        }
    };

    Ok(ntfs_file)
}

/// Setup Outlook using normal reader
pub(crate) fn setup_outlook_reader(path: &str) -> Result<File, OutlookError> {
    let reader_result = file_reader(path);
    let reader = match reader_result {
        Ok(reader) => reader,
        Err(err) => {
            error!("[outlook] Could not setup API reader: {err:?}");
            return Err(OutlookError::ReadFile);
        }
    };

    Ok(reader)
}
