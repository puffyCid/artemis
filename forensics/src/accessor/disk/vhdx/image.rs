use std::{
    fs::File,
    path::{Path, PathBuf},
};

use crate::accessor::{
    disk::{format::DiskImage, vhdx::reader::VhdxReader},
    error::{AccessorError, AccessorResult},
    location::path::is_absolute_host_path,
};

/// Open VHDX on disk
#[derive(Debug)]
pub(crate) struct VhdxDisk {
    /// Path to VHDX file
    path: PathBuf,
}

impl VhdxDisk {
    /// Return a `VhdxDisk` for VHDX file path
    pub(crate) fn new(vhdx_path: impl Into<PathBuf>) -> AccessorResult<Self> {
        let path = vhdx_path.into();
        let display = path.display().to_string();

        if !is_absolute_host_path(&display) {
            return Err(AccessorError::location(
                display,
                "VHDX image path must be absolute",
            ));
        }

        Ok(Self { path })
    }

    /// Path to the VHDX file
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl DiskImage for VhdxDisk {
    type Reader = VhdxReader<File>;

    fn open_reader(&self) -> AccessorResult<Self::Reader> {
        let file = File::open(&self.path).map_err(|err| AccessorError::io_path(&self.path, err))?;

        VhdxReader::open(file)
    }
}

#[cfg(test)]
mod tests {
    use super::VhdxDisk;
    use crate::accessor::{disk::format::DiskImage, error::AccessorError};
    use std::{
        io::{Read, Seek, SeekFrom},
        path::PathBuf,
    };

    fn test_image() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/disks/vhdx/ntfs.vhdx");
        path
    }

    #[test]
    fn test_vhdx_image_logical_ntfs() {
        let path = test_image();
        let image = VhdxDisk::new(&path).unwrap();
        assert_eq!(image.path(), path.as_path());

        let mut reader = image.open_reader().unwrap();
        assert_eq!(reader.creator(), "QEMU v11.1.0");
        assert_eq!(reader.logical_sector_size(), 512);
        assert_eq!(reader.physical_sector_size(), 512);

        assert_eq!(reader.file_size(), 16 * 1024 * 1024);

        let mut sector = [0_u8; 512];
        reader.read_exact(&mut sector).unwrap();
        assert_eq!(&sector[3..11], b"NTFS    ");

        let end = reader.seek(SeekFrom::End(0)).unwrap();
        assert_eq!(end, 8000000);
    }

    #[test]
    fn test_vhdx_image_matches_raw() {
        let mut raw = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        raw.push("tests/test_data/filesystems/ntfs/test.raw");

        let raw = std::fs::read(raw).unwrap();
        let mut reader = VhdxDisk::new(test_image()).unwrap().open_reader().unwrap();
        let mut guest = Vec::new();
        reader.read_to_end(&mut guest).unwrap();

        assert_eq!(guest.len(), raw.len());
        assert!(guest == raw);
    }

    #[test]
    fn test_vhdx_image_seek() {
        let mut reader = VhdxDisk::new(test_image()).unwrap().open_reader().unwrap();
        reader.seek(SeekFrom::Start(512)).unwrap();

        let mut sector = [0_u8; 512];
        reader.read_exact(&mut sector).unwrap();
        assert_eq!(reader.position(), 1024);

        reader.seek(SeekFrom::Start(0)).unwrap();
        reader.read_exact(&mut sector).unwrap();
        assert_eq!(&sector[3..11], b"NTFS    ");

        let past = reader.seek(SeekFrom::End(10)).unwrap();
        assert_eq!(past, 8000010);
        assert_eq!(reader.read(&mut sector).unwrap(), 0);
    }

    #[test]
    fn test_vhdx_image_relative_path() {
        let err = VhdxDisk::new("test.vhdx").unwrap_err();
        assert!(matches!(err, AccessorError::Location { .. }));
    }

    #[test]
    fn test_vhdx_image_missing_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/disks/vhdx/missing.vhdx");

        let image = VhdxDisk::new(&path).unwrap();
        let err = image.open_reader().unwrap_err();
        assert!(matches!(err, AccessorError::Io { .. }));
    }

    #[test]
    fn test_vhdx_image_not_vhdx() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ntfs/test.raw");
        let image = VhdxDisk::new(&path).unwrap();
        let err = image.open_reader().unwrap_err();

        assert!(matches!(err, AccessorError::Volume { .. }));
    }
}
