use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use crate::accessor::{
    disk::format::{DiskFormat, DiskImage},
    error::{AccessorError, AccessorResult},
    location::path::is_absolute_host_path,
};

/// Raw disk file
///
/// Commonly acquired with `dd` command
#[derive(Debug)]
pub(super) struct RawDisk {
    /// Path for the raw disk file
    path: PathBuf,
}

impl RawDisk {
    /// Return a `RawDisk` structure for provided file path
    ///
    /// Path must be absolute path to the raw disk
    pub(super) fn new(path: impl Into<PathBuf>) -> AccessorResult<Self> {
        let path = path.into();
        let display = path.display().to_string();
        if !is_absolute_host_path(&display) {
            return Err(AccessorError::location(
                display,
                "Raw image path must be absolute",
            ));
        }

        Ok(Self { path })
    }

    /// Path for raw disk image
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// Format for this image
    pub(super) fn format(&self) -> DiskFormat {
        DiskFormat::Raw
    }
}

impl DiskImage for RawDisk {
    type Reader = BufReader<File>;

    fn open_reader(&self) -> AccessorResult<Self::Reader> {
        let file = File::open(&self.path).map_err(|err| AccessorError::io_path(&self.path, err))?;
        Ok(BufReader::new(file))
    }
}

#[cfg(test)]
mod tests {
    use super::RawDisk;
    use crate::accessor::{
        disk::format::{DiskFormat, DiskImage},
        error::AccessorError,
    };
    use std::{
        io::{Read, Seek, SeekFrom},
        path::PathBuf,
    };

    fn test_image() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ntfs/test.raw");
        path
    }

    #[test]
    fn test_raw_image_logical_ntfs() {
        let path = test_image();
        let image = RawDisk::new(&path).unwrap();

        assert_eq!(image.format(), DiskFormat::Raw);
        assert_eq!(image.path(), path.as_path());

        let mut reader = image.open_reader().unwrap();
        let mut sector = [0_u8; 512];
        reader.read_exact(&mut sector).unwrap();

        assert_eq!(&sector[3..11], b"NTFS    ");

        let end = reader.seek(SeekFrom::End(0)).unwrap();
        assert_eq!(end, 8000000);
    }

    #[test]
    fn test_raw_image_relative_path() {
        let err = RawDisk::new("test.raw").unwrap_err();
        assert!(matches!(err, AccessorError::Location { .. }));
    }

    #[test]
    fn test_raw_image_missing_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ntfs/missing.raw");
        let image = RawDisk::new(&path).unwrap();

        let err = image.open_reader().unwrap_err();
        assert!(matches!(err, AccessorError::Io { .. }));
    }
}
