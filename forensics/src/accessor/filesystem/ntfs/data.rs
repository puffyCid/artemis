use crate::accessor::{
    entry::{
        handle::{DirEntry, DirHandle, FileHandle},
        locator::{DirLocator, FileLocator, NtfsEntryRef},
    },
    error::{AccessorError, AccessorResult},
    filesystem::ntfs::{
        volume::NtfsVolume,
        walk::{get_file_size, list_children, ntfs_err, resolve_file},
        wof::{decompress_wof, is_wof_file, read_named_data},
    },
    io::reader::AccessorReader,
    location::path::InnerPath,
};
use ntfs::{NtfsFile, NtfsReadSeek};
use std::fmt;
use std::{
    io::{self, Read, Seek, SeekFrom},
    sync::Arc,
};

/// A filesystem like accessor that can be used to read files from the raw NTFS
pub(crate) struct NtfsFs<R: Read + Seek + Send> {
    /// Target NTFS volume to read
    pub(crate) volume: Arc<NtfsVolume<R>>,
    /// Drive letter if we want to read a live NTFS filesystem
    pub(crate) drive: char,
}

impl<R: Read + Seek + Send + 'static> NtfsFs<R> {
    /// Create a new `NtfsFs` instance
    pub(crate) fn new(volume: NtfsVolume<R>, drive: char) -> Self {
        Self {
            volume: Arc::new(volume),
            drive,
        }
    }

    /// Read a file into memory. Max size is 2GB
    ///
    /// Supports both forward and back slashes. Example: C:\\Users\\test.txt or `C:/Users/test.txt`
    pub(crate) fn read_file(
        &self,
        inner: &InnerPath,
        max_read_size: Option<u64>,
    ) -> AccessorResult<Vec<u8>> {
        let inner_path = inner_to_ntfs_path(inner, self.drive);
        let display_path = display_ntfs_path(self.drive, &inner_path);

        self.volume.with_reader(|ntfs, reader| {
            let file = resolve_file(ntfs, reader, &inner_path)?;
            read_ntfs_file(reader, &file, &display_path, max_read_size)
        })
    }

    /// Read a file into memory by its file reference. Max size is 2GB
    pub(crate) fn read_handle(
        &self,
        handle: &FileHandle,
        max_read_size: Option<u64>,
    ) -> AccessorResult<Vec<u8>> {
        match &handle.locator {
            FileLocator::Ntfs {
                drive,
                file_ref,
                display_path,
            } => {
                if *drive != self.drive {
                    return Err(AccessorError::invalid_handle(format!(
                        "ntfs source cannot read handle for {}",
                        handle.display_path()
                    )));
                }

                self.volume.with_reader(|ntfs, reader| {
                    let file = open_by_ref(ntfs, reader, file_ref)?;
                    read_ntfs_file(reader, &file, display_path, max_read_size)
                })
            }
            _ => Err(AccessorError::invalid_handle(format!(
                "ntfs source cannot read handle for {}",
                handle.display_path()
            ))),
        }
    }

    /// Create an `AccessorReader` to stream a file
    ///
    /// Supports both forward and back slashes. Example: C:\\Users\\test.txt or `C:/Users/test.txt`
    pub(crate) fn reader(&self, inner: &InnerPath) -> AccessorResult<AccessorReader> {
        let inner_path = inner_to_ntfs_path(inner, self.drive);
        let display_path = display_ntfs_path(self.drive, &inner_path);

        let stream = self.volume.with_reader(|ntfs, reader| {
            let file = resolve_file(ntfs, reader, &inner_path)?;
            open_stream_reader(Arc::clone(&self.volume), reader, &file, &display_path)
        })?;

        Ok(AccessorReader::stream(stream))
    }

    /// Create an `AccessorReader` to stream a file by its file reference
    pub(crate) fn reader_handle(&self, handle: &FileHandle) -> AccessorResult<AccessorReader> {
        match &handle.locator {
            FileLocator::Ntfs {
                drive,
                file_ref,
                display_path,
            } => {
                if *drive != self.drive {
                    return Err(AccessorError::invalid_handle(format!(
                        "ntfs source cannot open reader handle for {}",
                        handle.display_path()
                    )));
                }

                let stream = self.volume.with_reader(|ntfs, reader| {
                    let file = open_by_ref(ntfs, reader, file_ref)?;
                    open_stream_reader(Arc::clone(&self.volume), reader, &file, display_path)
                })?;

                Ok(AccessorReader::stream(stream))
            }
            _ => Err(AccessorError::invalid_handle(format!(
                "ntfs source cannot open reader handle for {}",
                handle.display_path()
            ))),
        }
    }

    /// List files and directories in provided path
    pub(crate) fn read_dir(&self, inner: &InnerPath) -> AccessorResult<Vec<DirEntry>> {
        let inner_path = inner_to_ntfs_path(inner, self.drive);
        let display = display_ntfs_path(self.drive, &inner_path);

        list_children(&self.volume, self.drive, &display, &inner_path)
    }

    /// List files and directories from provided `DirHandle`
    pub(crate) fn read_dir_handle(&self, handle: &DirHandle) -> AccessorResult<Vec<DirEntry>> {
        match &handle.locator {
            DirLocator::Ntfs {
                drive,
                display_path,
                ..
            } => {
                if *drive != self.drive {
                    return Err(AccessorError::invalid_handle(format!(
                        "ntfs source cannot list directory handle for {}",
                        handle.display_path()
                    )));
                }
                let inner_path = strip_drive_prefix(display_path, self.drive);
                let display = display_ntfs_path(self.drive, &inner_path);

                list_children(&self.volume, self.drive, &display, &inner_path)
            }
            _ => Err(AccessorError::invalid_handle(format!(
                "ntfs source cannot list directory handle for {}",
                handle.display_path()
            ))),
        }
    }
}

/// Create a reader to stream large files by accessing the raw NTFS filesystem
pub(crate) struct NtfsStreamReader<R: Read + Seek + Send> {
    /// Target NTFS volume to read
    volume: Arc<NtfsVolume<R>>,
    /// Target file by file reference
    file_record_number: u64,
    /// Size of the file
    size: u64,
    /// Position of the reader
    position: u64,
    /// Small look ahead cache
    cache: Vec<u8>,
    /// Offset where our cache read to
    cache_offset: u64,
}

/// Open the file for streaming
fn open_stream_reader<R: Read + Seek + Send>(
    volume: Arc<NtfsVolume<R>>,
    reader: &mut R,
    file: &NtfsFile<'_>,
    display_path: &str,
) -> AccessorResult<NtfsStreamReader<R>> {
    if file.is_directory() {
        return Err(AccessorError::not_a_file(display_path));
    }

    // WOF files cannot be streamed. Since they are compressed
    if is_wof_file(reader, file)? {
        return Err(AccessorError::Ntfs {
            path: Some(display_path.to_string()),
            reason: String::from(
                "WOF-compressed files cannot be streamed; use read_file to decompress",
            ),
        });
    }

    let size = get_file_size(file.ntfs(), reader, file.file_record_number())?;

    Ok(NtfsStreamReader {
        volume,
        file_record_number: file.file_record_number(),
        size,
        position: 0,
        cache: Vec::new(),
        cache_offset: 0,
    })
}

/// How much cache to read in between file reads
const READ_AHEAD: usize = 1024 * 1024;

impl<R: Read + Seek + Send> NtfsStreamReader<R> {
    /// Reset the cache data
    fn invalidate_cache(&mut self) {
        self.cache.clear();
        self.cache_offset = 0;
    }

    /// Check if we can use our cache for reading next data
    fn cache_has_byte(&self, offset: u64) -> bool {
        !self.cache.is_empty()
            && offset >= self.cache_offset
            && offset < self.cache_offset + self.cache.len() as u64
    }

    /// Update our cache
    fn refill_cache(&mut self) -> io::Result<()> {
        let remaining = self.size - self.position;
        let to_read = READ_AHEAD.min(remaining as usize);
        let mut buf = std::mem::take(&mut self.cache);
        buf.resize(to_read, 0);

        let bytes = self
            .volume
            .with_reader(|ntfs, reader| {
                let file = ntfs
                    .file(reader, self.file_record_number)
                    .map_err(ntfs_err)?;
                read_data_attribute_bytes(reader, &file, self.position, &mut buf)
            })
            .map_err(accessor_to_io)?;

        buf.truncate(bytes);
        self.cache = buf;
        self.cache_offset = self.position;

        Ok(())
    }
}

impl<R: Read + Seek + Send> Read for NtfsStreamReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // If buffer is empty or file is 0 bytes in size return 0
        if buf.is_empty() || self.position >= self.size {
            return Ok(0);
        }

        let mut total = 0;
        while total < buf.len() && self.position < self.size {
            if !self.cache_has_byte(self.position) {
                self.refill_cache()?;

                if self.cache.is_empty() {
                    break;
                }
            }

            let offset = (self.position - self.cache_offset) as usize;
            let in_cache = self.cache.len() - offset;

            let remaining = (self.size - self.position) as usize;
            let want = buf.len() - total;

            let bytes = in_cache.min(remaining).min(want);

            buf[total..total + bytes].copy_from_slice(&self.cache[offset..offset + bytes]);
            self.position += bytes as u64;
            total += bytes;
        }

        Ok(total)
    }
}

impl<R: Read + Seek + Send> Seek for NtfsStreamReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::Current(offset) => {
                if offset >= 0 {
                    self.position.saturating_add(offset as u64)
                } else {
                    self.position.saturating_sub(offset.unsigned_abs())
                }
            }
            SeekFrom::End(offset) => {
                if offset >= 0 {
                    self.size.saturating_add(offset as u64)
                } else {
                    self.size.saturating_sub(offset.unsigned_abs())
                }
            }
        };

        if new_pos > self.size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("seek past end of file (size {})", self.size),
            ));
        }
        self.position = new_pos;
        self.invalidate_cache();

        Ok(self.position)
    }
}

impl<R: Read + Seek + Send> fmt::Debug for NtfsStreamReader<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NtfsStreamReader")
            .field("file_record_number", &self.file_record_number)
            .field("size", &self.size)
            .field("position", &self.position)
            .finish_non_exhaustive()
    }
}

/// Read bytes at provided offset for the $DATA attribute
fn read_data_attribute_bytes<R: Read + Seek>(
    reader: &mut R,
    file: &NtfsFile<'_>,
    offset: u64,
    buf: &mut [u8],
) -> AccessorResult<usize> {
    let Some(item) = file.data(reader, "") else {
        return Err(AccessorError::Ntfs {
            path: None,
            reason: String::from("file has no default $DATA stream"),
        });
    };

    let item = item.map_err(ntfs_err)?;
    let attr = item.to_attribute().map_err(ntfs_err)?;
    let mut value = attr.value(reader).map_err(ntfs_err)?;

    value
        .seek(reader, SeekFrom::Start(offset))
        .map_err(ntfs_err)?;

    value.read(reader, buf).map_err(ntfs_err)
}

/// Handle `AccessorError` errors to `io::Error`
fn accessor_to_io(err: AccessorError) -> io::Error {
    io::Error::other(err.to_string())
}

/// Read the entire file into memory. Handles WOF compression
fn read_ntfs_file<R: Read + Seek>(
    reader: &mut R,
    file: &NtfsFile<'_>,
    display_path: &str,
    max_read_size: Option<u64>,
) -> AccessorResult<Vec<u8>> {
    if file.is_directory() {
        return Err(AccessorError::not_a_file(display_path));
    }

    let size = get_file_size(file.ntfs(), reader, file.file_record_number())?;
    if let Some(limit) = max_read_size
        && size > limit
    {
        return Err(AccessorError::file_too_large(size, limit));
    }

    if is_wof_file(reader, file)? {
        return decompress_wof(reader, file);
    }

    read_named_data(reader, file, "")
}

/// Returns a `NtfsFile` by its file reference
fn open_by_ref<'n, R: Read + Seek>(
    ntfs: &'n ntfs::Ntfs,
    reader: &mut R,
    file_ref: &NtfsEntryRef,
) -> AccessorResult<NtfsFile<'n>> {
    ntfs.file(reader, file_ref.file_record_number)
        .map_err(ntfs_err)
}

/// Convert target `InnerPath` value to expected NTFS path
pub(crate) fn inner_to_ntfs_path(inner: &InnerPath, drive: char) -> String {
    if inner.is_empty() {
        return String::new();
    }

    strip_drive_prefix(&inner.display(), drive)
}

/// Remove drive characters if present
fn strip_drive_prefix(path: &str, drive: char) -> String {
    let trimmed = path.trim();
    let lower = format!("{}:", drive.to_ascii_lowercase());
    let upper = format!("{}:", drive.to_ascii_uppercase());

    let remainder = if let Some(rest) = trimmed.strip_prefix(&lower) {
        rest
    } else if let Some(rest) = trimmed.strip_prefix(&upper) {
        rest
    } else {
        trimmed
    };

    remainder.trim_start_matches(['\\', '/']).to_string()
}

/// Convert to a NTFS path
pub(crate) fn display_ntfs_path(drive: char, inner_path: &str) -> String {
    if inner_path.is_empty() {
        format!("{drive}:\\")
    } else {
        format!("{drive}:\\{inner_path}")
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::{
        entry::{handle::FileHandle, locator::FileLocator},
        error::AccessorError,
        filesystem::ntfs::{data::NtfsFs, volume::NtfsVolume, walk::list_children},
        location::path::InnerPath,
    };
    use std::{
        io::{Read, Seek, SeekFrom},
        path::PathBuf,
    };

    fn test_fs() -> NtfsFs<std::io::BufReader<std::fs::File>> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ntfs/test.raw");
        let volume = NtfsVolume::open_image(path).unwrap();
        NtfsFs::new(volume, 'C')
    }

    fn hello_path() -> InnerPath {
        InnerPath::new(PathBuf::from("hello/hello world.txt"))
    }

    fn main_ts_path() -> InnerPath {
        InnerPath::new(PathBuf::from("main.ts"))
    }

    #[test]
    fn test_ntfs_read() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/filesystems/ntfs/test.raw");

        let reader = test_fs();
        let bytes = reader.read_file(&main_ts_path(), Some(1000)).unwrap();
        assert_eq!(bytes.len(), 514);
    }

    #[test]
    fn test_ntfs_reader() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/filesystems/ntfs/test.raw");

        let volume = NtfsVolume::open_image(test_location).unwrap();
        let result = list_children(&volume, 'C', &"", &"").unwrap();
        let reader = test_fs();

        for entry in result {
            if !entry.is_file() || entry.meta.size == 0 {
                continue;
            }

            let mut file_reader = reader
                .reader_handle(entry.handle.as_file().unwrap())
                .unwrap();
            let mut buf = [0; 10];
            let bytes = file_reader.read(&mut buf).unwrap();

            assert_eq!(buf.len(), bytes);
        }
    }

    #[test]
    fn test_stream_partial_read_small_file() {
        let fs = test_fs();
        let mut stream = fs.reader(&hello_path()).unwrap();
        let mut buf = [0u8; 10];
        let result = stream.read(&mut buf).unwrap();

        assert_eq!(result, 10);
        assert_eq!(&buf[..result], b"hello worl");

        let result = stream.read(&mut buf).unwrap();

        assert_eq!(result, 2);
        assert_eq!(&buf[..result], b"d\n");
        assert_eq!(stream.read(&mut buf).unwrap(), 0);
    }

    #[test]
    fn test_stream_chunked_read_matches_full() {
        let fs = test_fs();
        let expected = fs.read_file(&main_ts_path(), None).unwrap();
        let mut stream = fs.reader(&main_ts_path()).unwrap();
        let mut results = Vec::new();
        let mut chunk = [0u8; 64];

        loop {
            let bytes = stream.read(&mut chunk).unwrap();
            if bytes == 0 {
                break;
            }
            results.extend_from_slice(&chunk[..bytes]);
        }

        assert_eq!(results, expected);
    }

    #[test]
    fn test_stream_eof_returns_zero() {
        let fs = test_fs();
        let mut stream = fs.reader(&hello_path()).unwrap();
        let mut buf = [0u8; 64];

        let results = stream.read(&mut buf).unwrap();

        assert_eq!(results, 12);
        assert_eq!(stream.read(&mut buf).unwrap(), 0);
    }

    #[test]
    fn test_empty_buffer_read_returns_zero() {
        let fs = test_fs();
        let mut stream = fs.reader(&hello_path()).unwrap();
        let mut buf = [];
        assert_eq!(stream.read(&mut buf).unwrap(), 0);
    }

    #[test]
    fn test_seek_start_then_read_tail() {
        let fs = test_fs();
        let full = fs.read_file(&hello_path(), None).unwrap();
        let mut stream = fs.reader(&hello_path()).unwrap();
        stream.seek(SeekFrom::Start(6)).unwrap();

        let mut tail = Vec::new();
        stream.read_to_end(&mut tail).unwrap();

        assert_eq!(tail, &full[6..]);
        assert_eq!(tail, b"world\n");
    }

    #[test]
    fn test_seek_current_and_end() {
        let fs = test_fs();
        let mut stream = fs.reader(&hello_path()).unwrap();
        stream.seek(SeekFrom::End(-5)).unwrap(); // "world\n"
        let mut buf = [0u8; 8];
        let size = stream.read(&mut buf).unwrap();

        assert_eq!(size, 5);
        assert_eq!(&buf[..size], b"orld\n");
    }

    #[test]
    fn test_seek_past_eof_errors() {
        let fs = test_fs();
        let mut stream = fs.reader(&hello_path()).unwrap();
        let err = stream.seek(SeekFrom::Start(13)).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn test_seek_back_to_start_rereads_same_bytes() {
        let fs = test_fs();
        let mut stream = fs.reader(&hello_path()).unwrap();
        let mut first = [0u8; 12];
        let mut second = [0u8; 12];
        stream.read_exact(&mut first).unwrap();
        stream.seek(SeekFrom::Start(0)).unwrap();
        stream.read_exact(&mut second).unwrap();

        assert_eq!(first, second);
        assert_eq!(&first, b"hello world\n");
    }

    #[test]
    fn test_read_handle_matches_read_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ntfs/test.raw");
        let volume = NtfsVolume::open_image(path).unwrap();
        let entries = list_children(&volume, 'C', "", "").unwrap();
        let main = entries
            .iter()
            .find(|e| e.name == "main.ts")
            .expect("main.ts in test image");

        let fs = NtfsFs::new(volume, 'C');
        let by_path = fs.read_file(&main_ts_path(), None).unwrap();
        let by_handle = fs
            .read_handle(main.handle.as_file().unwrap(), None)
            .unwrap();
        assert_eq!(by_handle, by_path);
    }

    #[test]
    fn test_reader_handle_matches_reader() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ntfs/test.raw");
        let volume = NtfsVolume::open_image(path).unwrap();
        let entries = list_children(&volume, 'C', "", "hello").unwrap();
        let hello = entries
            .iter()
            .find(|e| e.name == "hello world.txt")
            .expect("hello world.txt in test image");

        let fs = NtfsFs::new(volume, 'C');
        let mut by_path = fs.reader(&hello_path()).unwrap();
        let mut by_handle = fs.reader_handle(hello.handle.as_file().unwrap()).unwrap();
        let mut file_path = Vec::new();
        let mut file_handle = Vec::new();

        by_path.read_to_end(&mut file_path).unwrap();
        by_handle.read_to_end(&mut file_handle).unwrap();
        assert_eq!(file_path, b"hello world\n");
        assert_eq!(file_handle, b"hello world\n");
    }

    #[test]
    fn test_read_file_respects_max_size() {
        let fs = test_fs();
        let err = fs.read_file(&main_ts_path(), Some(100)).unwrap_err();

        assert!(matches!(
            err,
            AccessorError::FileTooLarge {
                size: 514,
                limit: 100
            }
        ));
    }

    #[test]
    fn test_read_file_directory_errors() {
        let fs = test_fs();
        let err = fs
            .read_file(&InnerPath::new(PathBuf::from("hello")), None)
            .unwrap_err();

        assert!(matches!(err, AccessorError::NotAFile { .. }));
    }

    #[test]
    fn test_reader_directory_errors() {
        let fs = test_fs();
        let err = fs
            .reader(&InnerPath::new(PathBuf::from("hello")))
            .unwrap_err();
        assert!(matches!(err, AccessorError::NotAFile { .. }));
    }

    #[test]
    fn test_read_file_not_found() {
        let fs = test_fs();
        let err = fs
            .read_file(&InnerPath::new(PathBuf::from("does/not/exist.txt")), None)
            .unwrap_err();

        assert!(matches!(err, AccessorError::NotFound { .. }));
    }

    #[test]
    fn test_path_forward_slashes() {
        let fs = test_fs();
        let bytes = fs.read_file(&hello_path(), None).unwrap();

        assert_eq!(bytes, b"hello world\n");
    }

    #[test]
    fn test_path_with_drive_prefix_stripped() {
        let fs = test_fs();
        let bytes = fs
            .read_file(
                &InnerPath::new(PathBuf::from("C:\\hello\\hello world.txt")),
                None,
            )
            .unwrap();

        assert_eq!(bytes, b"hello world\n");
    }

    #[test]
    fn test_wrong_drive_handle_errors() {
        let fs = test_fs();
        let entries = list_children(fs.volume.as_ref(), 'C', "", "").unwrap();
        let main = entries
            .iter()
            .find(|e| e.name == "main.ts")
            .unwrap()
            .handle
            .as_file()
            .unwrap()
            .clone();

        // Rebuild handle with wrong drive letter
        let bad_handle = match &main.locator {
            FileLocator::Ntfs {
                file_ref,
                display_path,
                ..
            } => FileHandle::new(FileLocator::Ntfs {
                drive: 'D',
                file_ref: file_ref.clone(),
                display_path: display_path.clone(),
            }),
            _ => panic!("expected ntfs locator"),
        };

        let err = fs.read_handle(&bad_handle, None).unwrap_err();
        assert!(matches!(err, AccessorError::InvalidHandle { .. }));
    }

    #[test]
    fn test_list_and_stream_smoke() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ntfs/test.raw");
        let volume = NtfsVolume::open_image(path).unwrap();
        let entries = list_children(&volume, 'C', "", "").unwrap();
        let fs = NtfsFs::new(volume, 'C');

        for entry in entries {
            if !entry.is_file() || entry.meta.size == 0 {
                continue;
            }
            let mut stream = fs.reader_handle(entry.handle.as_file().unwrap()).unwrap();
            let mut buf = [0u8; 10];
            let bytes = stream.read(&mut buf).unwrap();
            let expect = (entry.meta.size as usize).min(buf.len());

            assert_eq!(bytes, expect);
        }
    }
}
