use crate::accessor::location::scheme::{Scheme, strip_scheme};
use std::{
    fmt::Debug,
    fs::File,
    io::{self, Cursor, Read, Seek, SeekFrom},
};

/// Combines Read + Seek + Debug into a single trait object
pub(crate) trait ReadSeek: Read + Seek + Debug {}
impl<T: Read + Seek + Debug> ReadSeek for T {}

#[derive(Debug)]
/// Location metadata for an opened `AccessorReader`
pub(crate) struct ReaderLocation {
    /// Location including scheme. Examples: `host:/var/log/syslog`, `ntfs:C:\$MFT`, `zip:file.zip!path/to/tex.txt`
    display_path: String,
    /// Location with the scheme stripped. Examples: `/var/log/syslog`, `C:\$MFT`, `file.zip!path/to/tex.txt`
    full_path: String,
    /// Target filename. For zip entries this is the inner file (`tex.txt`), not the archive name. If ADS is provided for NTFS, then this will be the ADS name
    filename: String,
}

impl ReaderLocation {
    /// Return a new `ReaderLocation` based on provided input string
    pub(crate) fn from_display(input: impl Into<String>) -> Self {
        let display_path = input.into();
        let full_path = strip_scheme(&display_path).to_string();
        let filename = filename_from_display(&display_path);
        Self {
            display_path,
            full_path,
            filename,
        }
    }

    /// Return a new `ReaderLocation` based on provided `Scheme` and input string
    ///
    /// Example: `from_scheme(Scheme::Host, C:\\Users\\dev\\NTUSER.DAT)`
    pub(crate) fn from_scheme(scheme: Scheme, input: impl Into<String>) -> Self {
        let full_path = input.into();
        Self::from_display(format!("{}:{full_path}", scheme.as_str()))
    }

    /// Location of the file. Includes the scheme in the path
    ///
    /// Example: `ntfs:C:\\Users\\dev\NTUSER.DAT`
    pub(crate) fn display_path(&self) -> &str {
        &self.display_path
    }

    /// Location of the file with no `Scheme`
    ///
    /// Example: `C:\\Users\\dev\NTUSER.DAT`
    pub(crate) fn full_path(&self) -> &str {
        &self.full_path
    }

    /// Filename associated with the reader
    ///
    /// Zip entries return the inner filename
    pub(crate) fn filename(&self) -> &str {
        &self.filename
    }
}

/// Filename of the target file.
///
/// Zip locations use the inner entry after `!` (`zip:file.zip!./path/to/tex.txt` → `tex.txt`).
/// NTFS ADS streams use the attribute name (`ntfs:C:\$Secure:$SDS` returns `$SDS`).
fn filename_from_display(display_path: &str) -> String {
    let value = match display_path.split_once('!') {
        Some((_, inner)) => inner,
        None => strip_scheme(display_path),
    };

    let target = value.trim_start_matches("./").trim_end_matches(['/', '\\']);

    let name = target
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(target)
        .to_string();

    ads_filename(&name)
}

/// Return the ADS stream name if available
fn ads_filename(name: &str) -> String {
    let Some((base, ads)) = name.rsplit_once(':') else {
        return name.to_string();
    };

    if ads.is_empty() {
        return name.to_string();
    }

    // If ADS length is 1. Then we probably got drive letter (`C:`)
    if base.len() == 1
        && base
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic())
    {
        return name.to_string();
    }

    ads.to_string()
}

/// Backend source for `AccessorReader`
#[derive(Debug)]
pub(crate) enum SourceReader {
    /// `AccessorReader` for a file on a live host
    Host(File),
    /// `AccessorReader` for a file read into memory
    Memory(Cursor<Vec<u8>>),
    /// Stream a large file without reading the entire file into memory
    Stream(Box<dyn ReadSeek + Send>),
}

/// An abstract reader that can be used to read data
#[derive(Debug)]
pub(crate) struct AccessorReader {
    /// Reader to stream target file
    source: SourceReader,
    /// Path metadata about the target
    pub(crate) location: ReaderLocation,
}

impl Read for AccessorReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match &mut self.source {
            SourceReader::Host(file) => file.read(buf),
            SourceReader::Memory(cursor) => cursor.read(buf),
            SourceReader::Stream(stream) => stream.read(buf),
        }
    }
}

impl Seek for AccessorReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match &mut self.source {
            SourceReader::Host(file) => file.seek(pos),
            SourceReader::Memory(cursor) => cursor.seek(pos),
            SourceReader::Stream(stream) => stream.seek(pos),
        }
    }
}

impl AccessorReader {
    /// Read all bytes from current position
    pub(crate) fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        Read::read_to_end(self, buf)
    }

    /// Seek to absolute offset
    pub(crate) fn seek_from_start(&mut self, offset: u64) -> io::Result<u64> {
        self.seek(SeekFrom::Start(offset))
    }

    /// Return current offset
    pub(crate) fn position(&mut self) -> io::Result<u64> {
        self.stream_position()
    }

    /// Read provided bytes from absolute offset
    pub(crate) fn read_bytes(&mut self, offset: u64, length: usize) -> io::Result<Vec<u8>> {
        self.seek_from_start(offset)?;
        let mut buf = vec![0u8; length];
        Read::read_exact(self, &mut buf)?;

        Ok(buf)
    }

    /// Create host file reader
    pub(crate) fn host(file: File, location: ReaderLocation) -> Self {
        Self {
            source: SourceReader::Host(file),
            location,
        }
    }

    /// Create an in-memory reader
    pub(crate) fn memory(bytes: Vec<u8>, location: ReaderLocation) -> Self {
        Self {
            source: SourceReader::Memory(Cursor::new(bytes)),
            location,
        }
    }

    /// Stream large files without reading into memory
    ///
    /// Used for raw disk access
    pub(crate) fn stream(reader: impl ReadSeek + Send + 'static, location: ReaderLocation) -> Self {
        Self {
            source: SourceReader::Stream(Box::new(reader)),
            location,
        }
    }
}
