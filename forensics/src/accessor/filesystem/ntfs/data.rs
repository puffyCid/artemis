use crate::accessor::{
    entry::{
        handle::{DirEntry, DirHandle, FileHandle},
        locator::{DirLocator, FileLocator},
    },
    error::{AccessorError, AccessorResult},
    filesystem::ntfs::{
        attributes::read_named_data,
        volume::NtfsVolume,
        walk::{
            get_file_size, list_children, list_children_handle, ntfs_err, open_by_ref, resolve_file,
        },
        wof::{decompress_wof, is_wof_file},
    },
    io::reader::AccessorReader,
    location::path::InnerPath,
};
use ntfs::{NtfsFile, NtfsReadSeek, attribute_value::NtfsAttributeValue};
use std::{cmp::Ordering, fmt, mem};
use std::{
    io::{self, Read, Seek, SeekFrom},
    sync::Arc,
};

/// A filesystem like accessor that can be used to read files from the raw NTFS
pub(crate) struct NtfsFs<T: Read + Seek + Send> {
    /// Target NTFS volume to read
    pub(crate) volume: Arc<NtfsVolume<T>>,
    /// Drive letter if we want to read a live NTFS filesystem
    pub(crate) drive: char,
}

impl<T: Read + Seek + Send + 'static> NtfsFs<T> {
    /// Create a new `NtfsFs` instance
    pub(crate) fn new(volume: NtfsVolume<T>, drive: char) -> Self {
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
        let (inner_path, attribute_name) = inner_to_ntfs_path(inner, self.drive);
        let display_path = display_ntfs_path(self.drive, &inner_path);

        self.volume.with_reader(|ntfs, reader| {
            let file = resolve_file(ntfs, reader, &inner_path)?;
            read_ntfs_file(reader, &file, &display_path, max_read_size, &attribute_name)
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
                    let data = "";
                    read_ntfs_file(reader, &file, display_path, max_read_size, data)
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
        let (inner_path, _) = inner_to_ntfs_path(inner, self.drive);
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
        let (inner_path, _) = inner_to_ntfs_path(inner, self.drive);
        let display = display_ntfs_path(self.drive, &inner_path);

        list_children(&self.volume, self.drive, &display, &inner_path)
    }

    /// List files and directories from provided `DirHandle`
    pub(crate) fn read_dir_handle(&self, handle: &DirHandle) -> AccessorResult<Vec<DirEntry>> {
        match &handle.locator {
            DirLocator::Ntfs {
                drive,
                dir_ref,
                display_path,
            } => {
                if *drive != self.drive {
                    return Err(AccessorError::invalid_handle(format!(
                        "ntfs source cannot list directory handle for {}",
                        handle.display_path()
                    )));
                }

                list_children_handle(&self.volume, dir_ref, display_path, self.drive)
            }
            _ => Err(AccessorError::invalid_handle(format!(
                "ntfs source cannot list directory handle for {}",
                handle.display_path()
            ))),
        }
    }
}

/// Create a reader to stream large files by accessing the raw NTFS filesystem
pub(crate) struct NtfsStreamReader<T: Read + Seek + Send> {
    /// Target NTFS volume to read
    volume: Arc<NtfsVolume<T>>,
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
    runs: Option<Vec<DataRun>>,
    stream_end: u64,
}

/// Open the file for streaming
fn open_stream_reader<T: Read + Seek + Send>(
    volume: Arc<NtfsVolume<T>>,
    reader: &mut T,
    file: &NtfsFile<'_>,
    display_path: &str,
) -> AccessorResult<NtfsStreamReader<T>> {
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
        runs: map_data_runs(reader, file),
        stream_end: 0,
    })
}

/// How much cache to read in between file reads
const READ_AHEAD: usize = 1024 * 1024;

/// Map $DATA runs to absolute offsets on the volume
///
/// This allows us to avoid constantly having to read the MFT attributes
struct DataRun {
    /// Offset for the file we are reading
    file_offset: u64,
    /// Offset of the NTFS volume
    volume_offset: Option<u64>,
    /// Length of the file
    len: u64,
}

fn map_data_runs<T: Read + Seek>(reader: &mut T, file: &NtfsFile<'_>) -> Option<Vec<DataRun>> {
    let item = file.data(reader, "")?.ok()?;
    let attr = item.to_attribute().ok()?;

    let NtfsAttributeValue::NonResident(value) = attr.value(reader).ok()? else {
        return None;
    };

    let mut runs = Vec::new();
    let mut file_offset = 0;

    for data_runs in value.data_runs() {
        let run = data_runs.ok()?;
        let len = run.allocated_size();

        if len == 0 {
            continue;
        }

        runs.push(DataRun {
            file_offset,
            volume_offset: run.data_position().value().map(|post| post.get()),
            len,
        });

        file_offset += len;
    }

    if runs.is_empty() { None } else { Some(runs) }
}

fn find_run(runs: &[DataRun], offset: u64) -> Option<&DataRun> {
    let index = runs
        .binary_search_by(|run| {
            if offset < run.file_offset {
                Ordering::Greater
            } else if offset >= run.file_offset + run.len {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        })
        .ok()?;

    runs.get(index)
}

impl<T: Read + Seek + Send> NtfsStreamReader<T> {
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
        let mut buf = mem::take(&mut self.cache);
        buf.resize(to_read, 0);

        let bytes = self.read_at(self.position, &mut buf)?;

        buf.truncate(bytes);
        self.cache = buf;
        self.cache_offset = self.position;

        Ok(())
    }

    fn read_from_cache(&mut self, buf: &mut [u8]) -> usize {
        if !self.cache_has_byte(self.position) {
            return 0;
        }

        let offset = (self.position - self.cache_offset) as usize;
        let bytes = (self.cache.len() - offset).min(buf.len());

        buf[..bytes].copy_from_slice(&self.cache[offset..offset + bytes]);
        self.position += bytes as u64;
        self.stream_end = self.position;

        bytes
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        let Some(runs) = self.runs.as_deref() else {
            return self.read_attribute_at(offset, buf);
        };

        self.volume
            .with_reader(|_, reader| {
                let mut total = 0;
                let mut position = offset;

                while total < buf.len() && position < self.size {
                    let Some(run) = find_run(runs, position) else {
                        break;
                    };

                    let within = position - run.file_offset;
                    let available = (run.len - within).min(self.size - position) as usize;
                    let bytes = available.min(buf.len() - total);
                    let target = &mut buf[total..total + bytes];

                    match run.volume_offset {
                        None => target.fill(0),
                        Some(volume_offset) => {
                            reader.seek(SeekFrom::Start(volume_offset + within))?;
                            reader.read_exact(target)?;
                        }
                    }

                    total += bytes;
                    position += bytes as u64;
                }

                Ok(total)
            })
            .map_err(accessor_to_io)
    }

    fn read_attribute_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        self.volume
            .with_reader(|ntfs, reader| {
                let file = ntfs
                    .file(reader, self.file_record_number)
                    .map_err(ntfs_err)?;
                read_data_attribute_bytes(reader, &file, offset, buf)
            })
            .map_err(accessor_to_io)
    }
}

impl<T: Read + Seek + Send> Read for NtfsStreamReader<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // If buffer is empty or file is 0 bytes in size return 0
        if buf.is_empty() || self.position >= self.size {
            return Ok(0);
        }

        let remaining = (self.size - self.position) as usize;
        let want = remaining.min(buf.len());
        let buf = &mut buf[..want];
        let bytes = self.read_from_cache(buf);
        if bytes != 0 {
            return Ok(bytes);
        }

        if self.position != self.stream_end || buf.len() >= READ_AHEAD {
            let bytes = self.read_at(self.position, buf)?;

            if bytes == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!(
                        "no data at offset {} (file size {})",
                        self.position, self.size
                    ),
                ));
            }

            self.position += bytes as u64;
            return Ok(bytes);
        }

        self.refill_cache()?;
        if self.cache.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!(
                    "no data at offset {} (file size {})",
                    self.position, self.size
                ),
            ));
        }

        Ok(self.read_from_cache(buf))
    }
}

impl<T: Read + Seek + Send> Seek for NtfsStreamReader<T> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::Current(offset) if offset >= 0 => self.position.saturating_add(offset as u64),
            SeekFrom::Current(offset) => self.position.saturating_sub(offset.unsigned_abs()),
            SeekFrom::End(offset) if offset >= 0 => self.size.saturating_add(offset as u64),
            SeekFrom::End(offset) => self.size.saturating_sub(offset.unsigned_abs()),
        };

        if new_pos > self.size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("seek past end of file (size {})", self.size),
            ));
        }

        if !self.cache_has_byte(new_pos) {
            //println!("invalid");
            self.invalidate_cache();
        }

        self.position = new_pos;

        Ok(self.position)
    }
}

impl<T: Read + Seek + Send> fmt::Debug for NtfsStreamReader<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NtfsStreamReader")
            .field("file_record_number", &self.file_record_number)
            .field("size", &self.size)
            .field("position", &self.position)
            .finish_non_exhaustive()
    }
}

/// Read bytes at provided offset for the $DATA attribute
fn read_data_attribute_bytes<T: Read + Seek>(
    reader: &mut T,
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
fn read_ntfs_file<T: Read + Seek>(
    reader: &mut T,
    file: &NtfsFile<'_>,
    display_path: &str,
    max_read_size: Option<u64>,
    attribute_name: &str,
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

    if is_wof_file(reader, file)? && attribute_name.is_empty() {
        return decompress_wof(reader, file);
    }

    read_named_data(reader, file, attribute_name)
}

/// Convert target `InnerPath` value to expected NTFS path and attribute to read
///
/// By default the $DATA attribute is read ('""').
///
/// However if the user provides a ADS attribute we will read that
///
/// Example: C:\Users\test.txt:TEST
pub(crate) fn inner_to_ntfs_path(inner: &InnerPath, drive: char) -> (String, String) {
    if inner.is_empty() {
        return (String::new(), String::new());
    }

    strip_drive_prefix_and_ads(&inner.display(), drive)
}

/// Remove drive characters and ADS if present
fn strip_drive_prefix_and_ads(path: &str, drive: char) -> (String, String) {
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

    let (clean_path, attribute_name) = match remainder.rsplit_once(':') {
        Some((clean_path, attribute_name)) => (clean_path.to_string(), attribute_name.to_string()),
        None => (remainder.to_string(), String::new()),
    };

    (
        clean_path.trim_start_matches(['\\', '/']).to_string(),
        attribute_name,
    )
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
        filesystem::ntfs::{
            data::{NtfsFs, strip_drive_prefix_and_ads},
            volume::NtfsVolume,
            walk::list_children,
        },
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

    #[test]
    fn test_strip_drive_prefix_and_ads() {
        let (path, ads) = strip_drive_prefix_and_ads("C:\\Users\\test.txt:TEST", 'c');
        assert_eq!(path, "Users\\test.txt");
        assert_eq!(ads, "TEST");

        let (path, ads) = strip_drive_prefix_and_ads("C:\\Users\\test.txt", 'C');
        assert_eq!(path, r"Users\test.txt");
        assert_eq!(ads, "");

        let (path, ads) = strip_drive_prefix_and_ads("$Secure:$SDS", 'C');
        assert_eq!(path, "$Secure");
        assert_eq!(ads, "$SDS");
    }

    #[test]
    fn test_ntfs_read_ads() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/filesystems/ntfs/test.raw");

        let reader = test_fs();
        let path = InnerPath::new(PathBuf::from("$Secure:$SDS"));
        let bytes = reader.read_file(&path, None).unwrap();
        assert_eq!(bytes.len(), 262512);
    }
}
