use crate::accessor::{
    disk::qcow::header::QcowHeader,
    error::{AccessorError, AccessorResult},
};

/// Provides cluster sizes and guest offsets from a QCOW header.
#[derive(PartialEq, Debug, Clone, Copy)]
pub(super) struct QcowCluster {
    /// Bits per cluster
    ///
    /// Size is `cluster_size` = 1 << `cluster_bits`
    pub(super) cluster_bits: u32,
    /// Guest and host cluster size in bytes
    pub(super) cluster_size: u64,
    /// Level 2 entry size. Normal entries at 8 bytes
    /// and extended entries at 16 bytes
    pub(super) l2_entry_bytes: u64,
    /// Number of level 2 entries in one level 2 table
    pub(super) l2_entries: u64,
    /// Level 2 bits per cluster
    ///
    /// Entries is `l2_entries` = 1 << `l2_bits`
    pub(super) l2_bits: u32,
    /// Active level 1 table size in bytes
    pub(super) l1_bytes: u64,
    /// Largest continuous guest range describe by one mapping
    ///
    /// Equal to `cluster_size` for standard level 2 entries and `cluster_size` / 32 for extended
    /// level 2 entries
    pub(super) subcluster_size: u64,
}

impl QcowCluster {
    /// Return the guest cluster containing the `guest_offset`
    pub(super) fn guest_cluster_index(&self, guest_offset: u64) -> u64 {
        guest_offset >> self.cluster_bits
    }

    /// Return the level 1 entry the describes the `guest_offset`
    pub(super) fn level1_index(&self, guest_offset: u64) -> u64 {
        self.guest_cluster_index(guest_offset) >> self.l2_bits
    }

    /// Return the level 2 entry the describes the `guest_offset`
    pub(super) fn level2_index(&self, guest_offset: u64) -> u64 {
        self.guest_cluster_index(guest_offset) & (self.l2_entries - 1)
    }

    /// Return the byte offset of `guest_offset` within its cluster
    pub(super) fn offset_in_cluster(&self, guest_offset: u64) -> u64 {
        guest_offset & (self.cluster_size - 1)
    }

    /// Return how many guest bytes the active level 1 table can describe
    ///
    /// `None` means the coverage exceed `u64::MAX`
    pub(super) fn level1_coverage_bytes(&self) -> Option<u64> {
        let standard_l2_bytes = 8;
        let l1_entries = self.l1_bytes / standard_l2_bytes;
        l1_entries
            .checked_mul(self.l2_entries)?
            .checked_mul(self.cluster_size)
    }
}

/// Calculate cluster sizes and alignment values from a parsed QCOW header
pub(super) fn derive_cluster(header: &QcowHeader) -> AccessorResult<QcowCluster> {
    let min_cluster_bits = 9;
    let max_cluster_bits = 21;

    if !(min_cluster_bits..=max_cluster_bits).contains(&header.cluster_bits) {
        return Err(AccessorError::volume(format!(
            "Invalid QCOW cluster bits {}. Valid range is {min_cluster_bits} through {max_cluster_bits}",
            header.cluster_bits
        )));
    }

    let cluster_size = 1u64.checked_shl(header.cluster_bits).ok_or_else(|| {
        AccessorError::volume(format!(
            "QCOW cluster bit shift {} overflows",
            header.cluster_bits
        ))
    })?;

    let min_extended_l2_cluster_bits = 14;
    let standard_l2_entry_bytes: u64 = 8;

    let (l2_entry_bytes, subcluster_size) = if header.has_extended_l2() {
        if header.cluster_bits < min_extended_l2_cluster_bits {
            return Err(AccessorError::volume(format!(
                "Invalid QCOW extended L2 cluster bits {}. Extended L2 requires at least {min_extended_l2_cluster_bits}",
                header.cluster_bits
            )));
        }

        let extended_subcluster_count = 32;
        let subcluster_size = cluster_size / extended_subcluster_count;
        let extended_l2_entry_bytes = 16;
        (extended_l2_entry_bytes, subcluster_size)
    } else {
        (standard_l2_entry_bytes, cluster_size)
    };

    let entry_shift = l2_entry_bytes.trailing_zeros();
    let l2_bits = header
        .cluster_bits
        .checked_sub(entry_shift)
        .ok_or_else(|| {
            AccessorError::volume(format!(
                "QCOW cluster bits {} cannot address an L2 entry of {l2_entry_bytes} bytes",
                header.cluster_bits
            ))
        })?;

    let l2_entries = 1u64.checked_shl(l2_bits).ok_or_else(|| {
        AccessorError::volume(format!("QCOW L2 entry count shift {l2_bits} overflows"))
    })?;

    let l1_bytes = u64::from(header.l1_entries)
        .checked_mul(standard_l2_entry_bytes)
        .ok_or_else(|| AccessorError::volume("QCOW L1 table byte length overflows"))?;

    // 32 MB
    let max_l1_bytes = 32 * 1024 * 1024;

    if l1_bytes > max_l1_bytes {
        return Err(AccessorError::volume(format!(
            "QCOW L1 table is {l1_bytes} bytes. Maximum size is {max_l1_bytes}"
        )));
    }

    if header.l1_offset.checked_add(l1_bytes).is_none() {
        return Err(AccessorError::volume(format!(
            "QCOW L1 table at offset {} overflows",
            header.l1_offset
        )));
    }

    if u64::from(header.header_length) > cluster_size {
        return Err(AccessorError::volume(format!(
            "QCOW header length {} exceeds the {cluster_size} byte cluster",
            header.header_length
        )));
    }

    if header.l1_entries > 0 {
        require_cluster_aligned(header.l1_offset, cluster_size, "L1 table")?;
        require_outside_header_cluster(header.l1_offset, cluster_size, "L1 table")?;
    }

    if header.refcount_offset != 0 || header.refcount_clusters > 0 {
        require_cluster_aligned(header.refcount_offset, cluster_size, "refcount table")?;
    }

    if header.refcount_clusters > 0 {
        require_outside_header_cluster(header.refcount_offset, cluster_size, "refcount table")?;
    }

    if header.snapshot_count > 0 {
        require_cluster_aligned(header.snapshot_offset, cluster_size, "snapshot table")?;
        require_outside_header_cluster(header.snapshot_offset, cluster_size, "snapshot table")?;
    }

    check_backing_file(header, cluster_size)?;

    Ok(QcowCluster {
        cluster_bits: header.cluster_bits,
        cluster_size,
        l2_entry_bytes,
        l2_entries,
        l2_bits,
        l1_bytes,
        subcluster_size,
    })
}

/// Require offset to start on a cluster boundary
fn require_cluster_aligned(offset: u64, cluster_size: u64, table: &str) -> AccessorResult<()> {
    if offset & (cluster_size - 1) != 0 {
        return Err(AccessorError::volume(format!(
            "QCOW {table} offset {offset} is not aligned to the {cluster_size} byte cluster"
        )));
    }

    Ok(())
}

/// Require a metadata table to start after the header cluster
fn require_outside_header_cluster(
    offset: u64,
    cluster_size: u64,
    table: &str,
) -> AccessorResult<()> {
    if offset < cluster_size {
        return Err(AccessorError::volume(format!(
            "QCOW {table} offset {offset} overlaps the header cluster"
        )));
    }

    Ok(())
}

/// Check that the backing file in the QCOW header is located in the first cluster
fn check_backing_file(header: &QcowHeader, cluster_size: u64) -> AccessorResult<()> {
    let Some(backing) = header.backing_file else {
        return Ok(());
    };

    if backing.offset < u64::from(header.header_length) {
        return Err(AccessorError::volume(format!(
            "QCOW backing file at offset {} overlaps the header",
            backing.offset
        )));
    }

    let end = backing
        .offset
        .checked_add(u64::from(backing.size))
        .ok_or_else(|| {
            AccessorError::volume(format!(
                "QCOW backing file at offset {} overflows",
                backing.offset
            ))
        })?;

    if end > cluster_size {
        return Err(AccessorError::volume(format!(
            "QCOW backing file name extends past the {cluster_size} byte cluster"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::derive_cluster;
    use crate::accessor::{
        disk::qcow::header::parse_header,
        error::{AccessorError, AccessorResult},
    };
    use std::{fs::read, path::PathBuf};

    fn empty_header(version: u32, len: usize) -> Vec<u8> {
        let mut bytes = vec![0; len];
        bytes[..4].copy_from_slice(b"QFI\xfb");
        bytes[4..8].copy_from_slice(&version.to_be_bytes());

        if version >= 3 && len >= 104 {
            bytes[100..104].copy_from_slice(&(len as u32).to_be_bytes());
        }
        bytes
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
    }

    fn volume_reason<T>(result: AccessorResult<T>) -> String {
        match result {
            Err(AccessorError::Volume { reason }) => reason,
            Err(err) => panic!("expected a volume error, got {err}"),
            Ok(_) => panic!("expected a volume error"),
        }
    }

    #[test]
    fn test_derive_cluster_version3_fixture() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/disks/qcow/headers/header_version3.raw");
        let bytes = read(test_location).unwrap();

        let header = parse_header(&bytes).unwrap();
        let cluster = derive_cluster(&header).unwrap();

        assert_eq!(cluster.cluster_bits, 16);
        assert_eq!(cluster.cluster_size, 65536);
        assert_eq!(cluster.l2_entry_bytes, 8);
        assert_eq!(cluster.l2_entries, 8192);
        assert_eq!(cluster.l2_bits, 13);

        assert_eq!(cluster.l1_bytes, 1_280);
        assert_eq!(cluster.subcluster_size, 65536);
        assert_eq!(cluster.level1_coverage_bytes(), Some(header.virtual_size));

        let guest_offset = 536870912;
        assert_eq!(cluster.guest_cluster_index(guest_offset), 8_192);
        assert_eq!(cluster.level1_index(guest_offset), 1);
        assert_eq!(cluster.level2_index(guest_offset), 0);
        assert_eq!(cluster.offset_in_cluster(guest_offset), 0);
    }

    #[test]
    fn test_derive_cluster_version2_small_clusters() {
        let mut bytes = empty_header(2, 72);
        put_u32(&mut bytes, 20, 9);

        let header = parse_header(&bytes).unwrap();
        let cluster = derive_cluster(&header).unwrap();

        assert_eq!(cluster.cluster_size, 512);
        assert_eq!(cluster.l2_entries, 64);
        assert_eq!(cluster.l2_bits, 6);
        assert_eq!(cluster.subcluster_size, 512);

        assert_eq!(cluster.l1_bytes, 0);
        assert_eq!(cluster.level1_coverage_bytes(), Some(0));
    }

    #[test]
    fn test_derive_cluster_extended_l2_sizes() {
        let mut bytes = empty_header(3, 112);
        put_u32(&mut bytes, 20, 14);
        put_u64(&mut bytes, 72, 1 << 4);
        put_u32(&mut bytes, 36, 1);
        put_u64(&mut bytes, 40, 16384);

        let header = parse_header(&bytes).unwrap();
        let cluster = derive_cluster(&header).unwrap();
        assert!(header.has_extended_l2());

        assert_eq!(cluster.cluster_size, 16384);
        assert_eq!(cluster.l2_entry_bytes, 16);
        assert_eq!(cluster.l2_bits, 10);

        assert_eq!(cluster.l2_entries, 1024);
        assert_eq!(cluster.subcluster_size, 512);
        assert_eq!(cluster.l1_bytes, 8);
    }

    #[test]
    fn test_derive_cluster_allows_short_l1_coverage() {
        let mut bytes = empty_header(3, 112);
        put_u32(&mut bytes, 20, 9);
        put_u64(&mut bytes, 24, 1000000);
        put_u32(&mut bytes, 36, 1);
        put_u64(&mut bytes, 40, 512);

        let header = parse_header(&bytes).unwrap();
        let cluster = derive_cluster(&header).unwrap();

        assert_eq!(cluster.level1_coverage_bytes(), Some(32768));
        assert!(cluster.level1_coverage_bytes().unwrap() < header.virtual_size);
    }

    #[test]
    fn test_derive_cluster_rejects_invalid_layout() {
        let cases: Vec<(Vec<u8>, &str)> = vec![
            (
                {
                    let mut bytes = empty_header(3, 112);
                    put_u32(&mut bytes, 20, 8);
                    bytes
                },
                "cluster bits",
            ),
            (
                {
                    let mut bytes = empty_header(3, 112);
                    put_u32(&mut bytes, 20, 22);
                    bytes
                },
                "cluster bits",
            ),
            (
                {
                    let mut bytes = empty_header(3, 112);
                    put_u32(&mut bytes, 20, 13);
                    put_u64(&mut bytes, 72, 1 << 4);
                    bytes
                },
                "extended L2",
            ),
            (
                {
                    let mut bytes = empty_header(3, 112);
                    put_u32(&mut bytes, 20, 9);
                    put_u32(&mut bytes, 36, 1);
                    put_u64(&mut bytes, 40, 513);
                    bytes
                },
                "not aligned",
            ),
            (
                {
                    let mut bytes = empty_header(3, 112);
                    put_u32(&mut bytes, 20, 9);
                    put_u32(&mut bytes, 36, 1);
                    bytes
                },
                "overlaps the header cluster",
            ),
            (
                {
                    let mut bytes = empty_header(3, 112);
                    put_u32(&mut bytes, 20, 9);
                    put_u32(&mut bytes, 36, 4194305);
                    put_u64(&mut bytes, 40, 512);

                    bytes
                },
                "Maximum size",
            ),
            (
                {
                    let mut bytes = empty_header(3, 520);
                    put_u32(&mut bytes, 20, 9);
                    bytes
                },
                "header length",
            ),
            (
                {
                    let mut bytes = empty_header(3, 112);
                    put_u32(&mut bytes, 20, 9);
                    put_u64(&mut bytes, 8, 400);
                    put_u32(&mut bytes, 16, 200);

                    bytes
                },
                "backing file name",
            ),
            (
                {
                    let mut bytes = empty_header(3, 112);
                    put_u32(&mut bytes, 20, 9);
                    put_u64(&mut bytes, 48, 100);
                    bytes
                },
                "refcount table",
            ),
        ];
        for (bytes, expected) in cases {
            let header = parse_header(&bytes).unwrap();
            let reason = volume_reason(derive_cluster(&header));

            assert!(
                reason.contains(expected),
                "reason `{reason}` does not contain `{expected}`"
            );
        }
    }
}
