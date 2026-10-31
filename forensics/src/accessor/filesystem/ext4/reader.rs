use crate::accessor::filesystem::{
    ext4::{data::read_file_at, format::inode::Inode, volume::Ext4Volume},
    helper::err::accessor_to_io,
};
use std::{
    io::{self, Read, Seek, SeekFrom},
    sync::Arc,
};

/// Reader for streaming a file on ext4 filesystem
pub(crate) struct Ext4FileReader<R: Read + Seek + Send> {
    /// Opened ext4 volume reader
    volume: Arc<Ext4Volume<R>>,
    /// Inode number we are reading
    inode: u64,
    /// Generation value for the inode number
    ///
    /// If this number changes while we are streaming.  
    /// It means the file has been deleted and the inode number reused
    ///
    /// That means our reader position and file size are now misaligned and may have different data
    generation: u32,
    /// Current position of the reader
    position: u64,
}

impl<R: Read + Seek + Send> Ext4FileReader<R> {
    /// Return a `Ext4FileReader` for a inode target
    pub(crate) fn new(volume: Arc<Ext4Volume<R>>, inode: u64, generation: u32) -> Self {
        Self {
            volume,
            inode,
            generation,
            position: 0,
        }
    }

    /// Return our current `Inode` value
    fn current_inode(&self) -> io::Result<Inode> {
        let inode = self.volume.read_inode(self.inode).map_err(accessor_to_io)?;
        if inode.generation != self.generation {
            return Err(io::Error::other(format!(
                "inode {} generation is {:#x}, reader opened generation {:#x}",
                self.inode, inode.generation, self.generation
            )));
        }

        Ok(inode)
    }
}

impl<R: Read + Seek + Send> Read for Ext4FileReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let inode = self.current_inode()?;
        if buf.is_empty() || self.position >= inode.size {
            return Ok(0);
        }

        let bytes = read_file_at(&self.volume, self.inode, &inode, self.position, buf)
            .map_err(accessor_to_io)?;

        self.position = self
            .position
            .checked_add(u64::try_from(bytes).map_err(|err| io::Error::other(err.to_string()))?)
            .ok_or_else(|| io::Error::other("reader position overflows"))?;

        Ok(bytes)
    }
}

impl<R: Read + Seek + Send> Seek for Ext4FileReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let inode = self.current_inode()?;

        let size = i64::try_from(inode.size)
            .map_err(|_| io::Error::other("inode size exceeds seek range"))?;

        let position = i64::try_from(self.position)
            .map_err(|_| io::Error::other("reader position exceeds seek range"))?;

        let next = match pos {
            SeekFrom::Start(offset) => i64::try_from(offset)
                .map_err(|_| io::Error::other("seek offset exceeds seek range"))?,
            SeekFrom::Current(offset) => position
                .checked_add(offset)
                .ok_or_else(|| io::Error::other("seek offset overflows"))?,
            SeekFrom::End(offset) => size
                .checked_add(offset)
                .ok_or_else(|| io::Error::other("seek offset overflows"))?,
        };

        if next < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek offset is before the start of the file",
            ));
        }

        self.position =
            u64::try_from(next).map_err(|_| io::Error::other("seek offset exceeds seek range"))?;

        Ok(self.position)
    }
}

impl<R: Read + Seek + Send> std::fmt::Debug for Ext4FileReader<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ext4FileReader")
            .field("inode", &self.inode)
            .field("generation", &self.generation)
            .field("position", &self.position)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::Ext4FileReader;
    use crate::accessor::filesystem::ext4::volume::Ext4Volume;
    use std::{
        fs::File,
        io::{Read, Seek, SeekFrom},
        path::PathBuf,
        sync::Arc,
    };

    fn volume() -> Arc<Ext4Volume<File>> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/images/ext4/test.img");
        let volume = Ext4Volume::open(File::open(path).unwrap(), "test.img").unwrap();
        Arc::new(volume)
    }

    #[test]
    fn test_seek_end_reads_the_last_bytes() {
        let volume = volume();
        let generation = volume.read_inode(14).unwrap().generation;
        let mut reader = Ext4FileReader::new(Arc::clone(&volume), 14, generation);

        assert_eq!(reader.seek(SeekFrom::End(-2)).unwrap(), 11);
        let mut buf = [0_u8; 2];
        assert_eq!(reader.read(&mut buf).unwrap(), 2);
        assert_eq!(&buf, b"!\n");

        let err = reader.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(err, 0);
        assert!(
            reader.seek(SeekFrom::Current(-1)).is_err(),
            "seek before the start"
        );
    }

    #[test]
    fn test_read_rejects_a_different_generation() {
        let volume = volume();
        let mut reader = Ext4FileReader::new(volume, 14, 0);

        let err = reader.read(&mut [0_u8; 1]).unwrap_err();
        assert!(err.to_string().contains("generation"), "{err}");
    }
}
