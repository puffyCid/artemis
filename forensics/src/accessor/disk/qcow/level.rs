use std::{
    collections::VecDeque,
    io::{ErrorKind, Read, Seek, SeekFrom},
};

use crate::{
    accessor::{
        disk::qcow::{
            cluster::{QcowCluster, derive_cluster},
            header::{QcowHeader, QcowVersion},
        },
        error::{AccessorError, AccessorResult},
    },
    utils::nom_helper::nom_bu64,
};

/// Host mapping for a guest cluster
#[derive(PartialEq, Debug, Clone, Copy)]
pub(super) enum GuestMapping {
    /// No host cluster. Will be zeros when the QCOW disk
    /// has no backing file
    Unallocated,
    /// Zero cluster
    Zero,
    /// Uncompressed cluster
    Data {
        /// Host file offset of the cluster
        host_offset: u64,
    },
    /// Compressed cluster
    Compressed {
        /// Host file offset of the compressed bytes
        host_offset: u64,
        /// Number of compressed bytes at the `host_offset`
        compressed_size: u64,
    },
}

/// Least recently used (LRU) cache of Level 2 tables
#[derive(Debug)]
pub(super) struct Level2Cache {
    /// Cache of the level 2 table. Oldest entries are evicted
    ///
    /// <https://www.reddit.com/r/rust/comments/ugtle2/can_someone_here_explain_to_me_like_im_five_the/>
    /// <https://www.w3resource.com/rust-tutorial/rust-vecdeque-guide.php>
    tables: VecDeque<(u64, Vec<u8>)>,
    /// Max number of tables retained
    capacity: usize,
}

impl Level2Cache {
    /// Return a `Level2Cache` that can cache
    /// up to 4 L2 tables
    pub(super) fn new() -> Self {
        let max_cache_tables = 4;
        Self::with_capacity(max_cache_tables)
    }

    /// Cache up to the provided `capacity` L2 tables
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            tables: VecDeque::new(),
            capacity,
        }
    }

    /// Return a Level 2 entry and update cache
    fn entry(&mut self, host_offset: u64, index: u64) -> AccessorResult<Option<u64>> {
        let Some(position) = self
            .tables
            .iter()
            .position(|(offset, _)| *offset == host_offset)
        else {
            return Ok(None);
        };

        if position + 1 != self.tables.len()
            && let Some(table) = self.tables.remove(position)
        {
            self.tables.push_back(table);
        }

        let Some((_, bytes)) = self.tables.back() else {
            return Ok(None);
        };

        Ok(Some(level_entry(bytes, index)?))
    }

    /// Store a level 2 table and drop the least recently used table if cache is full
    fn insert(&mut self, host_offset: u64, table: Vec<u8>) {
        if self.capacity == 0 {
            return;
        }

        if let Some(position) = self
            .tables
            .iter()
            .position(|(offset, _)| *offset == host_offset)
        {
            self.tables.remove(position);
        }

        if self.tables.len() == self.capacity {
            self.tables.pop_front();
        }

        self.tables.push_back((host_offset, table));
    }
}

/// Current Level 1 table and sizes required to map guest offsets
#[derive(Debug)]
pub(super) struct QcowMap {
    /// Parsed QCOW header
    pub(super) header: QcowHeader,
    /// Calculated cluster values
    pub(super) cluster: QcowCluster,
    /// Level 1 entries
    pub(super) level1: Vec<u64>,
    /// Length of the QCOW file in bytes
    pub(super) file_len: u64,
}

impl QcowMap {
    /// Read the current Level 1 table
    pub(super) fn load<R: Read + Seek>(
        reader: &mut R,
        header: QcowHeader,
        file_len: u64,
    ) -> AccessorResult<Self> {
        let cluster = derive_cluster(&header)?;

        let level1 = if header.l1_entries == 0 {
            Vec::new()
        } else {
            read_level1(reader, &header, &cluster, file_len)?
        };

        Ok(Self {
            header,
            cluster,
            level1,
            file_len,
        })
    }

    /// Map the guest byte offset to the host cluster that contains it
    pub(super) fn map_guest<R: Read + Seek>(
        &self,
        reader: &mut R,
        cache: &mut Level2Cache,
        guest_offset: u64,
    ) -> AccessorResult<GuestMapping> {
        if self.header.has_extended_l2() {
            return Err(AccessorError::volume(
                "QCOW extended Level 2 entries are not supported",
            ));
        }

        if guest_offset >= self.header.virtual_size {
            return Err(AccessorError::volume(format!(
                "QCOW guest offset {guest_offset} is past the {} byte virtual disk",
                self.header.virtual_size
            )));
        }

        let level1_entries = u64::try_from(self.level1.len()).map_err(|_| {
            AccessorError::volume("QCOW Level 1 table does not fit in a 64-bit index")
        })?;

        let level1_index = self.cluster.level1_index(guest_offset);
        if level1_index >= level1_entries {
            return Ok(GuestMapping::Unallocated);
        }

        let index = usize::try_from(level1_index).map_err(|_| {
            AccessorError::volume(format!(
                "QCOW Level 1 index {level1_index} does not fit in memory"
            ))
        })?;

        let Some(level2_offset) = self.decode_level1(self.level1[index])? else {
            return Ok(GuestMapping::Unallocated);
        };

        let entry = self.read_level2_entry(
            reader,
            cache,
            level2_offset,
            self.cluster.level2_index(guest_offset),
        )?;

        self.decode_level2(entry)
    }

    /// Decode active Level 1 entry. `None`  means the level 2 table is unallocated
    fn decode_level1(&self, entry: u64) -> AccessorResult<Option<u64>> {
        let host_offset_mask = 0x00fffffffffffe00;
        let offset = entry & host_offset_mask;

        if offset == 0 {
            return Ok(None);
        }

        self.require_cluster_offset(offset, "Level 2 table")?;

        Ok(Some(offset))
    }

    /// Read standard level 2 entry. Cache is filled on miss
    fn read_level2_entry<R: Read + Seek>(
        &self,
        reader: &mut R,
        cache: &mut Level2Cache,
        level2_offset: u64,
        level2_index: u64,
    ) -> AccessorResult<u64> {
        if level2_index >= self.cluster.l2_entries {
            return Err(AccessorError::volume(format!(
                "QCOW Level 2 index {level2_index} is outside the {} entry table",
                self.cluster.l2_entries
            )));
        }

        if let Some(entry) = cache.entry(level2_offset, level2_index)? {
            return Ok(entry);
        }

        let table_len = usize::try_from(self.cluster.cluster_size).map_err(|_| {
            AccessorError::volume(format!(
                "QCOW Level 2 table is {} bytes and does not fit in memory",
                self.cluster.cluster_size
            ))
        })?;

        let mut table = vec![0_u8; table_len];
        read_exact_at(reader, level2_offset, &mut table)?;

        let entry = level_entry(&table, level2_index)?;
        cache.insert(level2_offset, table);

        Ok(entry)
    }

    fn decode_level2(&self, entry: u64) -> AccessorResult<GuestMapping> {
        let compressed_flag = 1 << 62;

        if entry & compressed_flag != 0 {
            return self.decode_compressed(entry);
        }

        let zero_flag = 1;
        if entry & zero_flag != 0 {
            if self.header.version == QcowVersion::V2 {
                return Err(AccessorError::volume(
                    "QCOW version 2 Level 2 entry has the zero-cluster bit set",
                ));
            }
            return Ok(GuestMapping::Zero);
        }

        let host_offset_mask = 0x00fffffffffffe00;
        let host_offset = entry & host_offset_mask;
        if host_offset == 0 {
            let copied_flag = 1 << 63;

            if entry & copied_flag != 0 {
                return Err(AccessorError::volume(
                    "QCOW Level 2 entry has a zero host offset and the copied bit set",
                ));
            }

            return Ok(GuestMapping::Unallocated);
        }

        self.require_cluster_offset(host_offset, "host cluster")?;
        Ok(GuestMapping::Data { host_offset })
    }

    fn decode_compressed(&self, entry: u64) -> AccessorResult<GuestMapping> {
        let copied_flag = 1 << 63;
        if entry & copied_flag != 0 {
            return Err(AccessorError::volume(
                "QCOW compressed Level 2 entry has the copied bit set",
            ));
        }

        let compressed_flag = 1 << 62;
        let compressed_descriptor_mask = compressed_flag - 1;
        let descriptor = entry & compressed_descriptor_mask;

        let sector_bits = self.cluster.cluster_bits.checked_sub(8).ok_or_else(|| {
            AccessorError::volume(format!(
                "QCOW cluster bits {} cannot describe a compressed cluster",
                self.cluster.cluster_bits
            ))
        })?;

        let offset_bits = 62_u32.checked_sub(sector_bits).ok_or_else(|| {
            AccessorError::volume(format!(
                "QCOW cluster bits {} cannot describe a compressed cluster",
                self.cluster.cluster_bits
            ))
        })?;

        let Some(offset_bit) = 1_u64.checked_shl(offset_bits) else {
            return Err(AccessorError::volume(format!(
                "QCOW cluster bits {} cannot describe a compressed cluster",
                self.cluster.cluster_bits
            )));
        };

        let host_offset = descriptor & (offset_bit - 1);
        if host_offset >> 56 != 0 {
            return Err(AccessorError::volume(format!(
                "QCOW compressed cluster offset {host_offset} sets bits above bit 55"
            )));
        }

        let sectors = (descriptor >> offset_bits).checked_add(1).ok_or_else(|| {
            AccessorError::volume("QCOW compressed cluster sector count overflows")
        })?;

        let sector_size = 512;
        let compressed_size = sectors.checked_mul(sector_size).ok_or_else(|| {
            AccessorError::volume("QCOW compressed cluster byte length overflows")
        })?;

        self.require_host_range(host_offset, compressed_size, "compressed cluster")?;

        Ok(GuestMapping::Compressed {
            host_offset,
            compressed_size,
        })
    }

    /// Get the cluster aligned host range that is after the header cluster of the QCOW disk
    fn require_cluster_offset(&self, offset: u64, kind: &str) -> AccessorResult<()> {
        if offset < self.cluster.cluster_size {
            return Err(AccessorError::volume(format!(
                "QCOW {kind} offset {offset} overlaps the header cluster"
            )));
        }

        if offset & (self.cluster.cluster_size - 1) != 0 {
            return Err(AccessorError::volume(format!(
                "QCOW {kind} offset {offset} is not aligned to the {} byte cluster",
                self.cluster.cluster_size
            )));
        }

        self.require_host_range(offset, self.cluster.cluster_size, kind)
    }

    /// Get the offset to start the of host cluster range
    fn require_host_range(&self, offset: u64, length: u64, kind: &str) -> AccessorResult<()> {
        if offset < self.cluster.cluster_size {
            return Err(AccessorError::volume(format!(
                "QCOW {kind} offset {offset} overlaps the header cluster"
            )));
        }

        let end = offset.checked_add(length).ok_or_else(|| {
            AccessorError::volume(format!("QCOW {kind} offset {offset} overflows"))
        })?;

        if end > self.file_len {
            return Err(AccessorError::volume(format!(
                "QCOW {kind} offset {offset} extends past the {} byte image",
                self.file_len
            )));
        }

        Ok(())
    }
}

/// Read the level 1 table
fn read_level1<R: Read + Seek>(
    reader: &mut R,
    header: &QcowHeader,
    cluster: &QcowCluster,
    file_len: u64,
) -> AccessorResult<Vec<u64>> {
    let end = header
        .l1_offset
        .checked_add(cluster.l1_bytes)
        .ok_or_else(|| {
            AccessorError::volume(format!(
                "QCOW Level 1 table at offset {} overflows",
                header.l1_offset
            ))
        })?;

    if end > file_len {
        return Err(AccessorError::volume(format!(
            "QCOW Level 1 table at offset {} extends past the {file_len} byte image",
            header.l1_offset
        )));
    }

    let table_len = usize::try_from(cluster.l1_bytes).map_err(|_| {
        AccessorError::volume(format!(
            "QCOW Level 1 table is {} bytes and does not fit in memory",
            cluster.l1_bytes
        ))
    })?;

    let mut bytes = vec![0_u8; table_len];
    read_exact_at(reader, header.l1_offset, &mut bytes)?;

    let mut entries = Vec::new();
    for index in 0..header.l1_entries {
        entries.push(level_entry(&bytes, u64::from(index))?);
    }

    Ok(entries)
}

/// Read a 8 byte level entry
fn level_entry(bytes: &[u8], index: u64) -> AccessorResult<u64> {
    let start = index
        .checked_mul(8)
        .ok_or_else(|| AccessorError::volume(format!("QCOW entry index {index} overflows")))?;

    let start = usize::try_from(start).map_err(|_| {
        AccessorError::volume(format!("QCOW entry offset {start} does not fit in memory"))
    })?;

    let end = start
        .checked_add(8)
        .ok_or_else(|| AccessorError::volume(format!("QCOW entry at offset {start} overflows")))?;

    let Some(chunk) = bytes.get(start..end) else {
        return Err(AccessorError::volume(format!(
            "QCOW entry {index} extends past the {} byte table",
            bytes.len()
        )));
    };

    let mut buf = [0_u8; 8];
    buf.copy_from_slice(chunk);

    let (_, entry) = nom_bu64(&buf, "QCOW level entry truncated")?;
    Ok(entry)
}

/// Seek and read bytes at provided offset
fn read_exact_at<R: Read + Seek>(
    reader: &mut R,
    offset: u64,
    buf: &mut [u8],
) -> AccessorResult<()> {
    reader.seek(SeekFrom::Start(offset))?;
    match reader.read_exact(buf) {
        Ok(result) => Ok(result),
        Err(err) if err.kind() == ErrorKind::UnexpectedEof => Err(AccessorError::volume(format!(
            "QCOW read of {} bytes at offset {offset} extends past the image",
            buf.len()
        ))),
        Err(err) => Err(err.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::{GuestMapping, Level2Cache, QcowMap};
    use crate::accessor::{
        disk::qcow::header::{QcowVersion, parse_header},
        error::{AccessorError, AccessorResult},
    };
    use std::{fs::read, io::Cursor, path::PathBuf};

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
    }

    fn write_header(
        image: &mut [u8],
        version: u32,
        cluster_bits: u32,
        virtual_size: u64,
        l1_entries: u32,
        l1_offset: u64,
    ) {
        image[..4].copy_from_slice(b"QFI\xfb");
        put_u32(image, 4, version);
        put_u32(image, 20, cluster_bits);
        put_u64(image, 24, virtual_size);
        put_u32(image, 36, l1_entries);
        put_u64(image, 40, l1_offset);

        if version >= 3 {
            put_u32(image, 100, 112);
        }
    }

    fn standard_image() -> Vec<u8> {
        let mut image = vec![0_u8; 2048];
        write_header(&mut image, 3, 9, 32_768, 1, 512);

        put_u64(&mut image, 512, 1024 | (1 << 63));
        put_u64(&mut image, 1024, 1536 | (1 << 63));

        image
    }

    fn open_map(bytes: Vec<u8>) -> (QcowMap, Cursor<Vec<u8>>) {
        let header = parse_header(&bytes).unwrap();
        let file_len = u64::try_from(bytes.len()).unwrap();
        let mut cursor = Cursor::new(bytes);

        let map = QcowMap::load(&mut cursor, header, file_len).unwrap();
        (map, cursor)
    }

    fn map_at(bytes: Vec<u8>, guest_offset: u64) -> AccessorResult<GuestMapping> {
        let (map, mut cursor) = open_map(bytes);
        let mut cache = Level2Cache::new();

        map.map_guest(&mut cursor, &mut cache, guest_offset)
    }

    fn volume_reason<T>(result: AccessorResult<T>) -> String {
        match result {
            Err(AccessorError::Volume { reason }) => reason,
            Err(err) => panic!("expected a volume error, got {err}"),
            Ok(_) => panic!("expected a volume error"),
        }
    }

    #[test]
    fn test_map_version3_fixture() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let header_bytes =
            read(test_location.join("tests/test_data/disks/qcow/headers/header_version3.raw"))
                .unwrap();
        let level1_bytes =
            read(test_location.join("tests/test_data/disks/qcow/levels/level1_version3.raw"))
                .unwrap();
        let level2_bytes =
            read(test_location.join("tests/test_data/disks/qcow/levels/level2_version3.raw"))
                .unwrap();

        let header = parse_header(&header_bytes).unwrap();
        let cluster = crate::accessor::disk::qcow::cluster::derive_cluster(&header).unwrap();

        assert_eq!(
            level2_bytes.len(),
            usize::try_from(cluster.cluster_size).unwrap()
        );

        let level2_offset =
            u64::from_be_bytes(level1_bytes[..8].try_into().unwrap()) & 0x00fffffffffffe00;
        let data_offset =
            u64::from_be_bytes(level2_bytes[..8].try_into().unwrap()) & 0x00fffffffffffe00;
        let data_16 =
            u64::from_be_bytes(level2_bytes[128..136].try_into().unwrap()) & 0x00fffffffffffe00;

        let file_len = header
            .l1_offset
            .saturating_add(u64::try_from(level1_bytes.len()).unwrap())
            .max(level2_offset.saturating_add(cluster.cluster_size))
            .max(data_offset.saturating_add(cluster.cluster_size))
            .max(data_16.saturating_add(cluster.cluster_size));

        let mut image = vec![0_u8; usize::try_from(file_len).unwrap()];
        image[..header_bytes.len()].copy_from_slice(&header_bytes);

        let l1_at = usize::try_from(header.l1_offset).unwrap();
        image[l1_at..l1_at + level1_bytes.len()].copy_from_slice(&level1_bytes);

        let l2_at = usize::try_from(level2_offset).unwrap();
        image[l2_at..l2_at + level2_bytes.len()].copy_from_slice(&level2_bytes);

        let (map, mut cursor) = open_map(image);
        assert_eq!(map.level1[0] & 0x00fffffffffffe00, level2_offset);

        let mut cache = Level2Cache::new();
        assert_eq!(
            map.map_guest(&mut cursor, &mut cache, 0).unwrap(),
            GuestMapping::Data {
                host_offset: data_offset
            }
        );

        assert_eq!(
            map.map_guest(&mut cursor, &mut cache, 65_536).unwrap(),
            GuestMapping::Unallocated
        );
        assert_eq!(
            map.map_guest(&mut cursor, &mut cache, 16 * 65_536).unwrap(),
            GuestMapping::Data {
                host_offset: data_16
            }
        );
    }

    #[test]
    fn test_map_unallocated_level1_and_short_table() {
        let mut image = standard_image();
        put_u64(&mut image, 512, 0);
        assert_eq!(map_at(image, 0).unwrap(), GuestMapping::Unallocated);

        let mut image = vec![0_u8; 2048];
        write_header(&mut image, 3, 9, 100000, 1, 512);

        put_u64(&mut image, 512, 1024);
        put_u64(&mut image, 1024, 1536);

        assert_eq!(map_at(image, 40000).unwrap(), GuestMapping::Unallocated);
    }

    #[test]
    fn test_map_unallocated_level2_entry() {
        let mut image = standard_image();
        put_u64(&mut image, 1024, 0);
        assert_eq!(map_at(image, 0).unwrap(), GuestMapping::Unallocated);
    }

    #[test]
    fn test_map_data_cluster_and_same_cluster_offset() {
        let image = standard_image();
        assert_eq!(
            map_at(image.clone(), 0).unwrap(),
            GuestMapping::Data { host_offset: 1536 }
        );

        assert_eq!(
            map_at(image, 100).unwrap(),
            GuestMapping::Data { host_offset: 1536 }
        );
    }

    #[test]
    fn test_map_ignores_reserved_level2_bits() {
        let mut image = standard_image();
        put_u64(&mut image, 1024, 1536 | (1 << 56) | 0x1e);

        assert_eq!(
            map_at(image, 0).unwrap(),
            GuestMapping::Data { host_offset: 1536 }
        );
    }

    #[test]
    fn test_map_explicit_zero_ignores_preallocated_host_offset() {
        let mut image = standard_image();
        put_u64(&mut image, 1024, 1 | (1 << 40));

        assert_eq!(map_at(image, 0).unwrap(), GuestMapping::Zero);
    }

    #[test]
    fn test_map_rejects_version2_zero_bit() {
        let mut image = vec![0_u8; 2048];
        write_header(&mut image, 2, 9, 32768, 1, 512);
        put_u64(&mut image, 512, 1024);
        put_u64(&mut image, 1024, 1);

        let (map, mut cursor) = open_map(image);
        assert_eq!(map.header.version, QcowVersion::V2);
        let reason = volume_reason(map.map_guest(&mut cursor, &mut Level2Cache::new(), 0));

        assert!(reason.contains("zero-cluster"), "{reason}");
    }
    #[test]
    fn test_map_compressed_cluster() {
        let mut image = vec![0_u8; 2560];
        write_header(&mut image, 3, 9, 32768, 1, 512);
        put_u64(&mut image, 512, 1024);

        put_u64(&mut image, 1024, 1536 | (1 << 61) | (1 << 62));

        assert_eq!(
            map_at(image, 0).unwrap(),
            GuestMapping::Compressed {
                host_offset: 1536,
                compressed_size: 1024
            }
        );
    }

    #[test]
    fn test_map_rejects_invalid_entries() {
        let cases: Vec<(Vec<u8>, &str)> = vec![
            (
                {
                    let mut image = standard_image();
                    put_u64(&mut image, 1024, 1 << 63);
                    image
                },
                "copied bit",
            ),
            (
                {
                    let mut image = vec![0_u8; 3072];
                    write_header(&mut image, 3, 10, 1024, 1, 1024);

                    put_u64(&mut image, 1024, 2048);
                    put_u64(&mut image, 2048, 1536);
                    image
                },
                "not aligned",
            ),
            (
                {
                    let mut image = vec![0_u8; 3072];
                    write_header(&mut image, 3, 10, 1024, 1, 1024);
                    put_u64(&mut image, 1024, 512);
                    image
                },
                "overlaps the header cluster",
            ),
            (
                {
                    let mut image = standard_image();
                    put_u64(&mut image, 1024, 1536 | (1 << 62) | (1 << 63));
                    image
                },
                "copied bit",
            ),
            (
                {
                    let mut image = standard_image();
                    put_u64(&mut image, 1024, (1 << 56) | (1 << 62));
                    image
                },
                "bits above bit 55",
            ),
            (
                {
                    let mut image = standard_image();
                    put_u64(&mut image, 1024, 1800 | (1 << 61) | (1 << 62));
                    image
                },
                "extends past",
            ),
            (
                {
                    let mut image = vec![0_u8; 1024];
                    write_header(&mut image, 3, 9, 32_768, 1, 512);

                    put_u64(&mut image, 512, 1024);
                    image
                },
                "Level 2 table",
            ),
            (
                {
                    let mut image = standard_image();
                    image.truncate(512);
                    image
                },
                "Level 1 table",
            ),
        ];

        for (image, expected) in cases {
            let reason = if expected == "Level 1 table" {
                let header = parse_header(&image).unwrap();
                let file_len = u64::try_from(image.len()).unwrap();

                volume_reason(QcowMap::load(&mut Cursor::new(image), header, file_len))
            } else {
                volume_reason(map_at(image, 0))
            };

            assert!(
                reason.contains(expected),
                "reason `{reason}` does not contain `{expected}`"
            );
        }
    }

    #[test]
    fn test_map_rejects_extended_l2_and_past_virtual_size() {
        let mut image = vec![0_u8; 16_392];
        write_header(&mut image, 3, 14, 1, 1, 16384);

        put_u64(&mut image, 72, 1 << 4);
        let reason = volume_reason(map_at(image, 0));
        assert!(reason.contains("extended Level 2"), "{reason}");

        let reason = volume_reason(map_at(standard_image(), 32768));
        assert!(reason.contains("past the"), "{reason}");
    }

    #[test]
    fn test_load_empty_level1() {
        let mut image = vec![0_u8; 512];
        write_header(&mut image, 3, 9, 32768, 0, 0);
        let (map, mut cursor) = open_map(image);

        assert!(map.level1.is_empty());
        let mut cache = Level2Cache::new();

        assert_eq!(
            map.map_guest(&mut cursor, &mut cache, 0).unwrap(),
            GuestMapping::Unallocated
        );
    }
}
