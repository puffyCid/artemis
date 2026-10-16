use crate::accessor::{
    disk::{
        format::{DiskFormat, DiskImage},
        qcow::reader::QcowReader,
    },
    error::{AccessorError, AccessorResult},
    location::path::is_absolute_host_path,
};
use std::{
    fs::File,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub(crate) struct QcowDisk {
    path: PathBuf,
    format: DiskFormat,
}

impl QcowDisk {
    /// Return a `QcowDisk` structure for provided file path
    pub(crate) fn new(qcow_path: impl Into<PathBuf>) -> AccessorResult<Self> {
        let path = qcow_path.into();
        let display = path.display().to_string();

        if !is_absolute_host_path(&display) {
            return Err(AccessorError::location(
                display,
                "QCOW image path must be absolute",
            ));
        }

        Ok(Self {
            path,
            format: DiskFormat::Qcow,
        })
    }

    /// Path to the QCOW disk image
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// `DiskFormat` value
    pub(crate) fn format(&self) -> DiskFormat {
        self.format
    }
}

impl DiskImage for QcowDisk {
    type Reader = QcowReader<File>;

    fn open_reader(&self) -> AccessorResult<Self::Reader> {
        let file = File::open(&self.path).map_err(|err| AccessorError::io_path(&self.path, err))?;

        QcowReader::open(file)
    }
}

#[cfg(test)]
mod tests {
    use super::QcowDisk;
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
        path.push("tests/test_data/disks/qcow/ntfs.qcow");
        path
    }

    #[test]
    fn test_qcow_image_logical_ntfs() {
        let path = test_image();
        let image = QcowDisk::new(&path).unwrap();
        assert_eq!(image.format(), DiskFormat::Qcow);
        assert_eq!(image.path(), path.as_path());

        let mut reader = image.open_reader().unwrap();
        let mut sector = [0_u8; 512];
        reader.read_exact(&mut sector).unwrap();

        assert_eq!(&sector[3..11], b"NTFS    ");
        let end = reader.seek(SeekFrom::End(0)).unwrap();
        assert_eq!(end, 8000000);
    }

    #[test]
    fn test_qcow_image_matches_raw() {
        let mut raw = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        raw.push("tests/test_data/filesystems/ntfs/test.raw");

        let raw = std::fs::read(raw).unwrap();
        let mut reader = QcowDisk::new(test_image()).unwrap().open_reader().unwrap();
        let mut guest = Vec::new();
        reader.read_to_end(&mut guest).unwrap();

        assert_eq!(guest.len(), raw.len());
        assert!(guest == raw);
    }

    #[test]
    fn test_qcow_image_relative_path() {
        let err = QcowDisk::new("test.qcow").unwrap_err();
        assert!(matches!(err, AccessorError::Location { .. }));
    }

    #[test]
    fn test_qcow_image_missing_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/disks/qcow/missing.qcow");

        let image = QcowDisk::new(&path).unwrap();
        let err = image.open_reader().unwrap_err();
        assert!(matches!(err, AccessorError::Io { .. }));
    }

    #[test]
    fn test_qcow_image_not_qcow() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ntfs/test.raw");
        let image = QcowDisk::new(&path).unwrap();
        let err = image.open_reader().unwrap_err();

        assert!(matches!(err, AccessorError::Volume { .. }));
    }
}
