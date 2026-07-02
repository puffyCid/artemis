use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
};

/// An abstract reader that can be used to read data
#[derive(Debug)]
pub(crate) enum AccessorReader {
    Host(File),
}
impl Read for AccessorReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Host(file) => file.read(buf),
        }
    }
}

impl Seek for AccessorReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match self {
            Self::Host(file) => file.seek(pos),
        }
    }
}

impl AccessorReader {
    /// Read all bytes from current position
    pub(crate) fn read_to_end(&mut self) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        Read::read_to_end(self, &mut buf)?;
        Ok(buf)
    }

    /// Seek to absolute offset
    pub(crate) fn seek_from_start(&mut self, offset: u64) -> io::Result<u64> {
        self.seek(SeekFrom::Start(offset))
    }

    /// Return current offset
    pub(crate) fn position(&mut self) -> io::Result<u64> {
        self.seek(SeekFrom::Current(0))
    }

    /// Read provided bytes from absolute offset
    pub(crate) fn read_bytes(&mut self, offset: u64, length: usize) -> io::Result<Vec<u8>> {
        self.seek_from_start(offset)?;
        let mut buf = vec![0u8; length];
        Read::read_exact(self, &mut buf)?;

        Ok(buf)
    }
}
