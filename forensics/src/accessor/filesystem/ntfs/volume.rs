// Full credit to: https://github.com/ColinFinck/ntfs/blob/master/examples/ntfs-shell/sector_reader.rs - MIT/Apache License - 2022-11-07

use crate::accessor::error::{AccessorError, AccessorResult};
use ntfs::Ntfs;
use std::{
    fs::File,
    io::{self, BufReader, Read, Seek, SeekFrom},
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

const VOLUME_SECTOR_SIZE: u16 = 4096;

/// `SectorReader` encapsulates any reader and only performs read and seek operations on it
/// on boundaries of the given sector size.
///
/// This can be very useful for readers that only accept sector-sized reads (like reading
/// from a raw partition on Windows).
/// The sector size must be a power of two.
///
/// This reader does not keep any buffer.
pub(crate) struct SectorReader<R>
where
    R: Read + Seek,
{
    /// The inner reader stream.
    inner: R,
    /// The sector size set at creation.
    sector_size: u16,
    /// The current stream position as requested by the caller through `read` or `seek`.
    /// The implementation will internally make sure to only read/seek on sector boundaries.
    stream_position: u64,
    /// This buffer is only part of the struct as a small performance optimization (keeping it allocated between reads).
    temp_buf: Vec<u8>,
}

impl<R> SectorReader<R>
where
    R: Read + Seek,
{
    pub(crate) fn new(inner: R, sector_size: u16) -> io::Result<Self> {
        if !sector_size.is_power_of_two() {
            return Err(io::Error::other("sector_size is not a power of two"));
        }

        Ok(Self {
            inner,
            sector_size,
            stream_position: 0,
            temp_buf: Vec::new(),
        })
    }

    fn align_down_to_sector_size(&self, n: u64) -> u64 {
        n / self.sector_size as u64 * self.sector_size as u64
    }

    fn align_up_to_sector_size(&self, n: u64) -> u64 {
        self.align_down_to_sector_size(n) + self.sector_size as u64
    }
}

impl<R> Read for SectorReader<R>
where
    R: Read + Seek,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        // We can only read from a sector boundary, and `self.stream_position` specifies the position where the
        // caller thinks we are.
        // Align down to a sector boundary to determine the position where we really are (see our `seek` implementation).
        let aligned_postition = self.align_down_to_sector_size(self.stream_position);

        // We have to read more bytes now to make up for the alignment difference.
        // We can also only read in multiples of the sector size, so align up to the next sector boundary.
        let start = (self.stream_position - aligned_postition) as usize;
        let end = start + buf.len();
        let aligend_bytes_to_read = self.align_up_to_sector_size(end as u64) as usize;

        // Perform the sector-sized read and copy the actually requested bytes into the given buffer.
        self.temp_buf.resize(aligend_bytes_to_read, 0);
        self.inner.read_exact(&mut self.temp_buf)?;
        buf.copy_from_slice(&self.temp_buf[start..end]);

        // We are done.
        self.stream_position += buf.len() as u64;
        Ok(buf.len())
    }
}

impl<R> Seek for SectorReader<R>
where
    R: Read + Seek,
{
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(n) => Some(n),
            // This is unsupported, because it's not safely possible under Windows.
            // We cannot seek to the end to determine the raw partition size.
            // Which makes it impossible to set `self.stream_position`.
            SeekFrom::End(_) => {
                return Err(io::Error::other(
                    "SeekFrom::End is unsupported for SectorReader",
                ));
            }
            SeekFrom::Current(n) => {
                if n >= 0 {
                    self.stream_position.checked_add(n as u64)
                } else {
                    self.stream_position.checked_sub(n.wrapping_neg() as u64)
                }
            }
        };

        match new_pos {
            Some(n) => {
                // We can only seek on sector boundaries, so align down the requested seek position and seek to that.
                let aligned_n = self.align_down_to_sector_size(n);
                self.inner.seek(SeekFrom::Start(aligned_n))?;

                // Make the caller believe that we seeked to the actually requested position.
                // Our `read` implementation will cover the difference.
                self.stream_position = n;
                Ok(self.stream_position)
            }
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid seek to a negative or overflowing position",
            )),
        }
    }
}

/// Parsed NTFS volume backed by any [`Read`] + [`Seek`] source
///
/// Used for live raw drives (Windows), disk images (any OS), and future image formats
/// All reads that touch the underlying byte source go through [`Self::with_reader`]
pub(crate) struct NtfsVolume<R: Read + Seek + Send> {
    display_id: String,
    ntfs: Ntfs,
    reader: Mutex<R>,
}

impl<R: Read + Seek + Send> NtfsVolume<R> {
    /// Create a `NtfsVolume` reader from a provided reader
    pub(crate) fn open(mut reader: R, display_id: impl Into<String>) -> AccessorResult<Self> {
        let display_id_value = display_id.into();
        let mut ntfs = Ntfs::new(&mut reader).map_err(|err| AccessorError::Ntfs {
            path: Some(display_id_value.clone()),
            reason: err.to_string(),
        })?;
        ntfs.read_upcase_table(&mut reader)
            .map_err(|err| AccessorError::Ntfs {
                path: Some(display_id_value.clone()),
                reason: err.to_string(),
            })?;

        Ok(Self {
            display_id: display_id_value,
            ntfs,
            reader: Mutex::new(reader),
        })
    }

    /// Return active `display_id`
    pub(crate) fn display_id(&self) -> &str {
        &self.display_id
    }

    /// Return information about the `NTFS` volume
    pub(crate) fn ntfs(&self) -> &Ntfs {
        &self.ntfs
    }

    /// Access the `NTFS` reader
    pub(crate) fn with_reader<F, T>(&self, operation: F) -> AccessorResult<T>
    where
        F: FnOnce(&Ntfs, &mut R) -> AccessorResult<T>,
    {
        let mut reader = self.lock_reader()?;
        operation(&self.ntfs, &mut reader)
    }

    /// Ensure our `NTFS` reader is properly locked. Should always be safe since artemis will always be single-threaded
    fn lock_reader(&self) -> AccessorResult<MutexGuard<'_, R>> {
        self.reader.lock().map_err(|err| AccessorError::Ntfs {
            path: Some(self.display_id.clone()),
            reason: format!("ntfs volume reader lock poisoned: {err:?}"),
        })
    }
}

impl NtfsVolume<BufReader<File>> {
    /// Open raw logical NTFS images. Example: A logical image of the C drive
    pub(crate) fn open_image(path: PathBuf) -> AccessorResult<Self> {
        let file = File::open(&path).map_err(|err| AccessorError::io_path(&path, err))?;
        Self::open(BufReader::new(file), format!("ntfs:{}", path.display()))
    }
}

impl NtfsVolume<BufReader<SectorReader<File>>> {
    /// Open the live drive Volume on a Windows system
    pub(crate) fn open_live_drive(drive: char) -> AccessorResult<Self> {
        if !drive.is_ascii_alphabetic() {
            return Err(AccessorError::location(
                format!("raw:{drive}:"),
                "raw source drive letter must be alphabetic",
            ));
        }

        let drive_upper = drive.to_ascii_uppercase();
        let device_path = format!(r"\\.\{drive_upper}:");
        let file =
            File::open(&device_path).map_err(|err| AccessorError::io_path(&device_path, err))?;
        let sector_reader =
            SectorReader::new(file, VOLUME_SECTOR_SIZE).map_err(AccessorError::from)?;
        Self::open(BufReader::new(sector_reader), format!("raw:{drive}:"))
    }
}
