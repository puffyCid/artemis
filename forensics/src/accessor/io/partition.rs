use std::io::{self, ErrorKind, Read, Seek, SeekFrom};

/// Reader for a partition on a logical disk
#[derive(Debug)]
pub(crate) struct PartitionReader<R> {
    /// Logical disk that contains the partition
    inner: R,
    /// Offset of the partition on `inner`
    byte_offset: u64,
    /// Length of the partition
    byte_length: u64,
    /// Current position of `PartitionReader` relative to
    /// the start of the partition
    position: u64,
}

impl<R: Read + Seek> PartitionReader<R> {
    /// Return a new `PartitionReader`
    pub(crate) fn new(inner: R, byte_offset: u64, byte_length: u64) -> io::Result<Self> {
        byte_offset.checked_add(byte_length).ok_or_else(|| {
            io::Error::new(
                ErrorKind::InvalidInput,
                "Partition byte range overflows u64::MAX",
            )
        })?;

        Ok(Self {
            inner,
            byte_offset,
            byte_length,
            position: 0,
        })
    }

    /// Return reason for read or seek errors
    fn invalid(reason: &str) -> io::Error {
        io::Error::new(ErrorKind::InvalidInput, reason)
    }

    /// Calculates the partition position for the reader
    fn partition_position(base: u64, delta: i64) -> io::Result<u64> {
        let position = if delta >= 0 {
            let step =
                u64::try_from(delta).map_err(|_err| Self::invalid("Partition seek overflows"))?;
            base.checked_add(step)
        } else {
            let step = delta.unsigned_abs();
            base.checked_sub(step)
        };

        position.ok_or_else(|| Self::invalid("Invalid seek to a negative or overflowing position"))
    }

    /// Calculates the disk position for the reader
    fn disk_position(&self, position: u64) -> io::Result<u64> {
        self.byte_offset
            .checked_add(position)
            .ok_or_else(|| Self::invalid("Partition offset overflows u64::MAX"))
    }

    /// Return number of bytes to read
    ///
    /// Should help prevent `PartitionReader` from reading
    /// beyond the partition
    fn read_len(remaining: u64, buf_len: usize) -> usize {
        match usize::try_from(remaining) {
            Ok(len) => len.min(buf_len),
            Err(_err) => buf_len,
        }
    }
}

impl<R: Read + Seek> Read for PartitionReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() || self.position >= self.byte_length {
            return Ok(0);
        }

        let remaining = self.byte_length - self.position;
        let length = Self::read_len(remaining, buf.len());
        let disk_position = self.disk_position(self.position)?;

        self.inner.seek(SeekFrom::Start(disk_position))?;
        let read = self.inner.read(&mut buf[..length])?;

        self.position += read as u64;

        Ok(read)
    }
}

impl<R: Read + Seek> Seek for PartitionReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_position = match pos {
            SeekFrom::Start(position) => position,
            SeekFrom::Current(delta) => Self::partition_position(self.position, delta)?,
            SeekFrom::End(delta) => Self::partition_position(self.byte_length, delta)?,
        };

        if new_position > self.byte_length {
            return Err(Self::invalid("Seek past the end of the partition"));
        }

        let disk_position = self.disk_position(new_position)?;
        self.inner.seek(SeekFrom::Start(disk_position))?;

        self.position = new_position;
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::io::partition::PartitionReader;
    use std::io::{Cursor, Read, Seek, SeekFrom};

    fn reader(disk: &[u8], byte_offset: u64, byte_length: u64) -> PartitionReader<Cursor<Vec<u8>>> {
        PartitionReader::new(Cursor::new(disk.to_vec()), byte_offset, byte_length).unwrap()
    }

    #[test]
    fn test_partition_reader() {
        let disk = b"DISK-HEADER-HELLO-AFTER";
        let mut partition = reader(disk, 12, 5);
        let mut buf = [0u8; 8];
        let read = partition.read(&mut buf).unwrap();

        assert_eq!(read, 5);
        assert_eq!(&buf[..read], b"HELLO");
        assert_eq!(partition.read(&mut buf).unwrap(), 0);
    }

    #[test]
    fn test_partition_reader_seek() {
        let test = b"test1234567";
        let mut partition = reader(test, 4, 5);
        let pos = partition.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(pos, 0);

        let mut buf = [0u8; 3];
        let read = partition.read(&mut buf).unwrap();
        assert_eq!(read, 3);
        assert_eq!(&buf[0..], b"123");
    }

    #[test]
    fn test_read_stops_at_partition_end() {
        let disk = b"AAAAHELLOZZ";
        let mut partition = reader(disk, 4, 5);
        partition.seek(SeekFrom::Start(3)).unwrap();
        let mut buf = [0u8; 8];
        let read = partition.read(&mut buf).unwrap();

        assert_eq!(read, 2);
        assert_eq!(&buf[..read], b"LO");
        assert_eq!(partition.read(&mut buf).unwrap(), 0);
    }

    #[test]
    fn test_seek_end_is_partition_length() {
        let mut partition = reader(b"01234567WXYZ!!", 8, 4);
        assert_eq!(partition.seek(SeekFrom::End(0)).unwrap(), 4);
        assert_eq!(partition.seek(SeekFrom::End(-2)).unwrap(), 2);

        let mut buf = [0u8; 4];
        let read = partition.read(&mut buf).unwrap();
        assert_eq!(&buf[..read], b"YZ");
    }

    #[test]
    fn test_seek_current_and_failed_seek_keeps_position() {
        let mut partition = reader(b"0123456789", 0, 10);
        assert_eq!(partition.seek(SeekFrom::Start(4)).unwrap(), 4);
        assert_eq!(partition.seek(SeekFrom::Current(2)).unwrap(), 6);
        assert_eq!(partition.seek(SeekFrom::Current(-1)).unwrap(), 5);

        assert!(partition.seek(SeekFrom::Current(-6)).is_err());
        assert!(partition.seek(SeekFrom::Start(11)).is_err());
        assert!(partition.seek(SeekFrom::End(1)).is_err());
        assert_eq!(partition.seek(SeekFrom::Current(0)).unwrap(), 5);
    }

    #[test]
    fn test_short_disk_returns_available_bytes() {
        let mut partition = reader(&[0u8; 10], 0, 100);
        let mut buf = [1u8; 16];

        assert_eq!(partition.read(&mut buf).unwrap(), 10);
        assert_eq!(partition.read(&mut buf).unwrap(), 0);
    }

    #[test]
    fn test_empty_partition_and_overflow() {
        let mut partition = reader(b"abc", 1, 0);
        let mut buf = [0u8; 1];

        assert_eq!(partition.read(&mut buf).unwrap(), 0);
        assert_eq!(partition.seek(SeekFrom::End(0)).unwrap(), 0);
        assert!(PartitionReader::new(Cursor::new(Vec::<u8>::new()), u64::MAX, 1).is_err());
    }
}
