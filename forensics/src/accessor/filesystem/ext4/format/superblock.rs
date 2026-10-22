use crate::{
    accessor::error::{AccessorError, AccessorResult},
    utils::{
        nom_helper::{nom_take, nom_u8, nom_u16, nom_u32, nom_u64},
        strings::extract_utf8_string,
        uuid::format_guid_be_bytes,
    },
};

/// Superblock metadata for the EXT4 filesystem
///
/// <https://docs.kernel.org/next/filesystems/ext4/super.html>
///
/// <https://github.com/Dicklesworthstone/frankenfs/blob/main/EXISTING_EXT4_BTRFS_STRUCTURE.md>
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SuperBlock {
    /// Total inode coount
    inodes_count: u32,
    /// Total block count
    blocks_count_low: u32,
    /// Number of blocks that can only be allocated by the superuser
    reserved_blocks_count_low: u32,
    /// Free blocks count
    free_blocks_count_low: u32,
    /// Free inods count
    free_inodes_count: u32,
    /// First data block. Must be at least 1 for 1k block filesystem
    ///
    /// Otherwise 0 for all other block sizes
    first_data_block: u32,
    /// Log size
    log_block_size: u32,
    /// Log size
    log_cluster_size: u32,
    /// Blocks per group
    pub(super) blocks_per_group: u32,
    /// Clusters per group, if big alloc flag is enabled.
    ///
    /// Otherwise clusters_per_group must equal blocks_per_group.
    clusters_per_group: u32,
    /// Inodes per group
    inodes_per_group: u32,
    /// Last mount time in UNIXEPOCH seconds
    mount_time: u32,
    /// Last write time in UNIXEPOCH seconds
    write_time: u32,
    /// Number of mounts since last fsck
    mount_count: u16,
    /// Number of mounts beyond which a fsck is needed
    max_mount_count: u16,
    /// EXT4 signature. 0xEF53
    magic: u16,
    /// Filesystem state
    state: u16,
    /// Behavior when detecting errors
    errors: u16,
    /// Minor revision level for the ext4 filesystem
    minor_revision: u16,
    /// Time since last check in UNIXEPOCH seconds
    last_check: u32,
    /// Maximum time between checks in seconds
    check_interval: u32,
    /// OS associated with ext4 creation
    creator_os: u32,
    /// Revision level for the ext4 filesystem
    revision: u32,
    /// Default UID for reserved blocks
    default_reserved_uid: u16,
    /// Default GID for reserved blocks
    default_reserved_gid: u16,
    /// First non-reserved inode
    first_inode: u32,
    /// Size of inode structure in bytes
    pub(super) inode_size: u16,
    /// Block group number of this superblock
    block_group_number: u16,
    /// Compatible feature set flags
    compatible_features: u32,
    /// Incompatible feature set flags
    incompatible_features: u32,
    /// Readonly compatible freature set
    read_only_features: u32,
    /// UUID for the volume
    uuid: [u8; 16],
    /// Volume name
    volume_name_raw: [u8; 16],
    /// Directory where the filesystem was last mounted
    last_mounted_raw: [u8; 64],
    /// Compression algorithm
    algorithm_usage_bitmap: u32,
    /// Number of blocks to try to preallocate for files
    prealloc_blocks: u8,
    /// Number of blocks to try to preallocated for directories
    prealloc_dir_blocks: u8,
    /// Number of reserved GDT entries for filesystem expansion
    reserved_gdt_blocks: u16,
    /// UUID of the journal superblock
    journal_uuid: [u8; 16],
    /// Inode number of the journal file
    journal_inode: u32,
    /// Device number of the journal file
    journal_device: u32,
    /// Start of list of orphaned inodes to delete
    last_orphan: u32,
    /// Hash tree seed
    hash_seed: [u32; 4],
    /// Hash algorithm to use for directory hashes
    default_hash_version: u8,
    /// If this value is 0 or 1 then journal blocks contains
    /// duplicate copy of inode block and size
    journal_backup_type: u8,
    /// Size of group descriptors in bytes
    pub(super) descriptor_size: u16,
    /// Default mount options
    default_mount_options: u32,
    /// First metablock block group
    first_meta_block_group: u32,
    /// When filesystem was created in UNIXEPOCH seconds
    creation_time: u32,
    /// Backup copy of the journal inode block array
    journal_blocks: [u32; 17],
    /// High 32 bits of the block count
    blocks_count_high: u32,
    /// High 32 bits of the reserved block count
    reserved_blocks_count_high: u32,
    /// High 32 bits of the free block count
    free_blocks_count_high: u32,
    /// All inodes have least provided bytes
    min_extra_inode_size: u16,
    /// New inodes should reserve provide bytes
    wanted_extra_inode_size: u16,
    /// Misc flags
    flags: u32,
    /// RAID stride
    raid_stride: u16,
    /// Number of seconds to wait in multi-mount prevention
    mmp_interval: u16,
    /// Block number for multi-mount protection data
    mmp_block: u64,
    /// RAID stripe width
    raid_stripe_width: u32,
    /// Size of a flexible block group
    log_groups_per_flex: u8,
    /// Metadata checksum algorithm
    checksum_type: u8,
    /// Versioning level for encryption
    encryption_level: u8,
    /// Padding to next 32 bits
    reserved_padding: u8,
    /// Number of kilobytes written to this filesystem
    /// over its lifetime
    kilobytes_written: u64,
    /// Inode number of the active snapshot
    snapshot_inode: u32,
    /// Sequential ID of active snapshot
    snapshot_id: u32,
    /// Number of blocks reserved for active snapshots future
    /// use
    snapshot_reserved_blocks: u64,
    /// Inode number of the head of the on-disk snapshot list
    snapshot_list: u32,
    /// Number of errors seen
    error_count: u32,
    /// First time an error happened in UNIXEPOCH seconds
    first_error_time: u32,
    /// Inode involved in the first error
    first_error_inode: u32,
    // Number of block involved of first error
    first_error_block: u64,
    /// Name of function where error happened
    first_error_function: [u8; 32],
    /// Line number where error happened
    first_error_line: u32,
    /// Time of most recent error UNIXEPOCH seconds
    last_error_time: u32,
    /// Inode involved in most recent error
    last_error_inode: u32,
    /// Line number where error happened
    last_error_line: u32,
    /// Number of block involved in most recent error
    last_error_block: u64,
    /// Name of function where error happened
    last_error_function: [u8; 32],
    /// ASCII string of mount options
    mount_options_raw: [u8; 64],
    /// Inode number of user quota file
    user_quota_inode: u32,
    /// Inode number of group quota file
    group_quota_inode: u32,
    /// Overhead blocks in filesystem
    overhead_blocks: u32,
    /// Block groups containing superblock backup
    backup_block_groups: [u32; 2],
    /// Encryption algorithm used
    encryption_algorithms: [u8; 4],
    /// Salt for algorithm key
    encryption_password_salt: [u8; 16],
    /// Inode number of lost+found
    lost_and_found_inode: u32,
    /// Inode that tracks project quotas
    project_quota_inode: u32,
    /// Checksum seed used for metadata CSUM calculations
    checksum_seed: u32,
    /// Upper 8 bits of write time
    write_time_high: u8,
    /// Upper 8 bits of mount time
    mount_time_high: u8,
    /// Upper 8 bits of creation time
    creation_time_high: u8,
    /// Upper 8 bits of last check time
    last_check_high: u8,
    /// Upper 8 bits of first error time
    first_error_time_high: u8,
    /// Upper 8 bits of last error time
    last_error_time_high: u8,
    /// First error code
    first_error_code: u8,
    /// Last error code
    last_error_code: u8,
    /// Filename encoding
    encoding: u16,
    /// Filename encoding flags
    encoding_flags: u16,
    /// Orphan file inode number
    orphan_file_inode: u32,
    /// Padding to end of the block
    reserved: [u8; 376],
    /// Superblock checksum
    checksum: u32,
}

impl SuperBlock {
    /**
     * Superblock incompatible feature flags
     */
    /// Compression flag
    const COMPRESSION: u32 = 0x1;
    /// Directory entries record the file type
    const FILETYPE: u32 = 0x2;
    /// Filesystem needs recovery
    const RECOVER: u32 = 0x4;
    /// Filesystem has separate journal device
    const JOURNAL_DEV: u32 = 0x8;
    /// Meta block groups
    const META_BG: u32 = 0x10;
    /// Files in the filesystem use extents
    pub(super) const EXTENTS: u32 = 0x40;
    /// Enable a filesystem size of 2^64 blocks
    const BIT64: u32 = 0x80;
    /// Multiple mount protection
    const MMP: u32 = 0x100;
    /// Flexible block groups
    const FLEX_BG: u32 = 0x200;
    /// Inodes can be used to store large extended
    /// attribute values
    const EA_INODE: u32 = 0x400;
    /// Data in directory entry
    const DIR_DATA: u32 = 0x1000;
    /// Metadata checksum seed is store in superblock
    const CSUM_SEED: u32 = 0x2000;
    /// Large directory (larger than 2 GB)
    const LARGE_DIR: u32 = 0x4000;
    /// Data in inode
    const INLINE_DATA: u32 = 0x8000;
    /// Encrypted indoes can be present
    const ENCRYPT: u32 = 0x10000;
    /// Directories can be marked case insensitive
    const CASEFOLD: u32 = 0x20000;

    /// List of all known incompatible flags
    const KNOWN: u32 = Self::COMPRESSION
        | Self::FILETYPE
        | Self::RECOVER
        | Self::JOURNAL_DEV
        | Self::META_BG
        | Self::EXTENTS
        | Self::BIT64
        | Self::MMP
        | Self::FLEX_BG
        | Self::EA_INODE
        | Self::DIR_DATA
        | Self::CSUM_SEED
        | Self::LARGE_DIR
        | Self::INLINE_DATA
        | Self::ENCRYPT
        | Self::CASEFOLD;

    /**
     * Superblock compatible feature flags
     */
    /// Directory preallocation flag
    const DIR_PREALLOC: u32 = 0x1;
    /// Imagic Inodes flag
    const IMAGIC_INODES: u32 = 0x0002;
    /// Has a journal flag
    const HAS_JOURNAL: u32 = 0x0004;
    /// Supports extended attributes
    const EXT_ATTR: u32 = 0x0008;
    /// Has reserved GDT blocks for filesystem expansion
    const RESIZE_INODE: u32 = 0x0010;
    /// Has directory indexes
    const DIR_INDEX: u32 = 0x0020;
    /// Lazy BG flag
    const LAZY_BG: u32 = 0x0040;
    /// Exclude inode flag. Not used
    const EXCLUDE_INODE: u32 = 0x0080;
    /// Exclude bitmap flag
    const EXCLUDE_BITMAP: u32 = 0x0100;
    /// Sparse superblock version 2 flag
    const SPARSE_SUPER2: u32 = 0x0200;
    /// Fast commits supported
    const FAST_COMMIT: u32 = 0x0400;
    /// Stable inode numbers. Used for encryption
    const STABLE_INODES: u32 = 0x0800;
    /// Orphan file allocated flag
    const ORPHAN_FILE: u32 = 0x1000;

    /**
     * Superblock read-only feature flags
     */
    /// Sparse superblocks
    const SPARSE_SUPER: u32 = 0x0001;
    /// Filesystem has been used to store a file greater than 2GB
    const LARGE_FILE: u32 = 0x0002;
    /// Unused
    const BTREE_DIR: u32 = 0x0004;
    /// Filesystem has files whose sizes are represented in units of
    /// logical blocks and not 512 byte sectors
    const HUGE_FILE: u32 = 0x0008;
    /// Group descriptors have checksums
    const GDT_CSUM: u32 = 0x0010;
    /// Old ext3 subdirectory no longer applies
    const DIR_NLINK: u32 = 0x0020;
    /// Large inodes exist
    const EXTRA_ISIZE: u32 = 0x0040;
    /// Filesystem has a snapshot
    const HAS_SNAPSHOT: u32 = 0x0080;
    /// Quota flag
    const QUOTA: u32 = 0x0100;
    /// Filesystem supports big alloc
    const BIGALLOC: u32 = 0x0200;
    /// Filesystem supports metadata checksumming
    const METADATA_CSUM: u32 = 0x0400;
    /// Filesystem supports replicas
    const REPLICA: u32 = 0x0800;
    /// Read only filesystem
    const READONLY: u32 = 0x1000;
    /// Filesystem tracks project quotas
    const PROJECT: u32 = 0x2000;
    /// Used by Android ext4?
    const SHARED_BLOCKS: u32 = 0x4000;
    /// Verity inodes may be present
    const VERITY: u32 = 0x8000;
    /// Indicates orphan file may have valid orphan entries
    const ORPHAN_PRESENT: u32 = 0x10000;

    /// Return whether a compatible feature bit is set
    pub(crate) fn has_compatible(&self, feature: u32) -> bool {
        self.compatible_features & feature != 0
    }

    /// Return whether an incompatible feature bit is set
    pub(crate) fn has_incompatible(&self, feature: u32) -> bool {
        self.incompatible_features & feature != 0
    }

    /// Return whether a read only compatible feature bit is set
    pub(crate) fn has_read_only(&self, feature: u32) -> bool {
        self.read_only_features & feature != 0
    }

    /// Return incompatible bits that artemis does not recognize
    pub(crate) fn unknown_incompatible(&self) -> u32 {
        self.incompatible_features & !Self::KNOWN
    }

    /// Return the OS creator of the file ext4 filesystem
    pub(crate) fn creator(&self) -> &str {
        match self.creator_os {
            0 => "Linux",
            1 => "Hurd",
            2 => "Masix",
            3 => "FreeBSD",
            4 => "Lites",
            _ => "Unknown",
        }
    }

    /// Superblock state
    pub(crate) fn state(&self) -> Vec<&str> {
        let mut state = Vec::new();
        if self.state & 1 != 0 {
            state.push("Cleanly Unmounted");
        }

        if self.state & 2 != 0 {
            state.push("Errors Detected");
        }

        if self.state & 4 != 0 {
            state.push("Orphans Being Recovered");
        }

        state
    }

    /// Return error policy of filesystem
    pub(crate) fn error_policy(&self) -> &str {
        match self.errors {
            1 => "Continue",
            2 => "Remount read-only",
            3 => "Panic",
            _ => "Unknown",
        }
    }

    /// Return the superblock revision
    pub(crate) fn revision(&self) -> &str {
        match self.revision {
            0 => "Original",
            1 => "Version 2 with Dynamic Inode Sizes",
            _ => "Unknown",
        }
    }

    /// Combine low and high block counts based on 64-bit feature flag
    pub(crate) fn blocks_count(&self) -> u64 {
        combine_u32(
            self.blocks_count_low,
            self.blocks_count_high,
            self.has_incompatible(Self::BIT64),
        )
    }

    /// Combine low and high free block counts based on 64-bit feature flag
    pub(crate) fn free_blocks_count(&self) -> u64 {
        combine_u32(
            self.free_blocks_count_low,
            self.free_blocks_count_high,
            self.has_incompatible(Self::BIT64),
        )
    }

    /// Return the filesystem UUID
    pub(crate) fn filesystem_id(&self) -> String {
        format_guid_be_bytes(&self.uuid)
    }

    /// Return the volume name
    pub(crate) fn volume_name(&self) -> String {
        extract_utf8_string(&self.volume_name_raw)
    }

    /// Return the last mount path
    pub(crate) fn last_mounted(&self) -> String {
        extract_utf8_string(&self.last_mounted_raw)
    }
}

/// Parse the superblock metadata
pub(crate) fn parse_superblock(data: &[u8]) -> AccessorResult<SuperBlock> {
    let superblock_size = 1024;
    if data.len() < superblock_size {
        return Err(ext4_error(format!(
            "superblock is truncated: found {} bytes, need {superblock_size}",
            data.len()
        )));
    }

    let input = &data[..superblock_size];
    let (input, inodes_count) = nom_u32(input, "superblock inode count is truncated")?;
    let (input, blocks_count_low) = nom_u32(input, "superblock block count low is truncated")?;
    let (input, reserved_blocks_count_low) =
        nom_u32(input, "superblock reserved block count low is truncated")?;
    let (input, free_blocks_count_low) =
        nom_u32(input, "superblock free block count low is truncated")?;

    let (input, free_inodes_count) = nom_u32(input, "superblock free inode count is truncated")?;
    let (input, first_data_block) = nom_u32(input, "superblock first data block is truncated")?;
    let (input, log_block_size) = nom_u32(input, "superblock log block size is truncated")?;

    let (input, log_cluster_size) = nom_u32(input, "superblock log cluster size is truncated")?;
    let (input, blocks_per_group) = nom_u32(input, "superblock blocks per group is truncated")?;
    let (input, clusters_per_group) = nom_u32(input, "superblock clusters per group is truncated")?;

    let (input, inodes_per_group) = nom_u32(input, "superblock inodes per group is truncated")?;
    let (input, mount_time) = nom_u32(input, "superblock mount time is truncated")?;
    let (input, write_time) = nom_u32(input, "superblock write time is truncated")?;

    let (input, mount_count) = nom_u16(input, "superblock mount count is truncated")?;
    let (input, max_mount_count) = nom_u16(input, "superblock maximum mount count is truncated")?;
    let (input, magic) = nom_u16(input, "superblock magic is truncated")?;

    let ext4_sig = 0xef53;
    if magic != ext4_sig {
        return Err(ext4_error(format!(
            "invalid superblock magic {magic:#06x}, expected {ext4_sig:#06x}"
        )));
    }

    let (input, state) = nom_u16(input, "superblock filesystem state is truncated")?;
    let (input, errors) = nom_u16(input, "superblock error policy is truncated")?;
    let (input, minor_revision) = nom_u16(input, "superblock minor revision is truncated")?;
    let (input, last_check) = nom_u32(input, "superblock last check time is truncated")?;
    let (input, check_interval) = nom_u32(input, "superblock check interval is truncated")?;
    let (input, creator_os) = nom_u32(input, "superblock creator OS is truncated")?;
    let (input, revision) = nom_u32(input, "superblock revision is truncated")?;
    if revision > 1 {
        return Err(ext4_error(format!(
            "unsupported superblock revision {revision}"
        )));
    }

    let (input, default_reserved_uid) =
        nom_u16(input, "superblock default reserved UID is truncated")?;
    let (input, default_reserved_gid) =
        nom_u16(input, "superblock default reserved GID is truncated")?;
    let (input, first_inode) = nom_u32(input, "superblock first inode is truncated")?;
    let (input, inode_size) = nom_u16(input, "superblock inode size is truncated")?;
    let (input, block_group_number) = nom_u16(input, "superblock block group number is truncated")?;
    let (input, compatible_features) =
        nom_u32(input, "superblock compatible features is truncated")?;
    let (input, incompatible_features) =
        nom_u32(input, "superblock incompatible features is truncated")?;
    let (input, read_only_features) = nom_u32(
        input,
        "superblock read-only compatible features is truncated",
    )?;

    let (input, uuid) = nom_take(input, 16_u16, "superblock filesystem UUID is truncated")?;
    let uuid: [u8; 16] = uuid
        .try_into()
        .map_err(|_err| AccessorError::volume("superblock filesystem UUID is truncated"))?;
    let (input, volume_name_raw) = nom_take(input, 16_u16, "superblock volume name is truncated")?;
    let volume_name_raw: [u8; 16] = volume_name_raw
        .try_into()
        .map_err(|_err| AccessorError::volume("superblock volume name is truncated"))?;
    let (input, last_mounted_raw) =
        nom_take(input, 64_u16, "superblock last mount path is truncated")?;

    let last_mounted_raw: [u8; 64] = last_mounted_raw
        .try_into()
        .map_err(|_err| AccessorError::volume("superblock last mount path is truncated"))?;
    let (input, algorithm_usage_bitmap) =
        nom_u32(input, "superblock algorithm usage bitmap is truncated")?;
    let (input, prealloc_blocks) =
        nom_u8(input, "superblock file preallocation blocks is truncated")?;
    let (input, prealloc_dir_blocks) = nom_u8(
        input,
        "superblock directory preallocation blocks is truncated",
    )?;

    let (input, reserved_gdt_blocks) =
        nom_u16(input, "superblock reserved GDT blocks is truncated")?;
    let (input, journal_uuid) = nom_take(input, 16_u16, "superblock journal UUID is truncated")?;
    let journal_uuid: [u8; 16] = journal_uuid
        .try_into()
        .map_err(|_err| AccessorError::volume("superblock journal UUID is truncated"))?;
    let (input, journal_inode) = nom_u32(input, "superblock journal inode is truncated")?;
    let (input, journal_device) = nom_u32(input, "superblock journal device is truncated")?;
    let (mut input, last_orphan) = nom_u32(input, "superblock last orphan is truncated")?;

    let mut hash_seed = [0; 4];
    for value in &mut hash_seed {
        let (remaining, parsed) = nom_u32(input, "superblock directory hash seed is truncated")?;
        *value = parsed;
        input = remaining;
    }

    let (input, default_hash_version) =
        nom_u8(input, "superblock default hash version is truncated")?;
    let (input, journal_backup_type) =
        nom_u8(input, "superblock journal backup type is truncated")?;
    let (input, descriptor_size) = nom_u16(input, "superblock group descriptor size is truncated")?;
    let (input, default_mount_options) =
        nom_u32(input, "superblock default mount options is truncated")?;
    let (input, first_meta_block_group) =
        nom_u32(input, "superblock first meta block group is truncated")?;
    let (mut input, creation_time) = nom_u32(input, "superblock creation time is truncated")?;

    let mut journal_blocks = [0; 17];
    for value in &mut journal_blocks {
        let (remaining, parsed) = nom_u32(input, "superblock journal backup blocks is truncated")?;
        *value = parsed;
        input = remaining;
    }

    let (input, blocks_count_high) = nom_u32(input, "superblock block count high is truncated")?;
    let (input, reserved_blocks_count_high) =
        nom_u32(input, "superblock reserved block count high is truncated")?;
    let (input, free_blocks_count_high) =
        nom_u32(input, "superblock free block count high is truncated")?;
    let (input, min_extra_inode_size) =
        nom_u16(input, "superblock minimum extra inode size is truncated")?;
    let (input, wanted_extra_inode_size) =
        nom_u16(input, "superblock wanted extra inode size is truncated")?;

    let (input, flags) = nom_u32(input, "superblock filesystem flags is truncated")?;
    let (input, raid_stride) = nom_u16(input, "superblock RAID stride is truncated")?;
    let (input, mmp_interval) = nom_u16(input, "superblock MMP interval is truncated")?;
    let (input, mmp_block) = nom_u64(input, "superblock MMP block is truncated")?;
    let (input, raid_stripe_width) = nom_u32(input, "superblock RAID stripe width is truncated")?;
    let (input, log_groups_per_flex) =
        nom_u8(input, "superblock log groups per flex is truncated")?;
    let (input, checksum_type) = nom_u8(input, "superblock checksum type is truncated")?;

    if read_only_features & SuperBlock::METADATA_CSUM != 0 && checksum_type != 1 {
        return Err(ext4_error(format!(
            "unsupported metadata checksum type {checksum_type}, expected crc32c (1)"
        )));
    }

    let (input, encryption_level) = nom_u8(input, "superblock encryption level is truncated")?;
    let (input, reserved_padding) = nom_u8(input, "superblock reserved padding is truncated")?;
    let (input, kilobytes_written) = nom_u64(input, "superblock kilobytes written is truncated")?;
    let (input, snapshot_inode) = nom_u32(input, "superblock snapshot inode is truncated")?;
    let (input, snapshot_id) = nom_u32(input, "superblock snapshot ID is truncated")?;
    let (input, snapshot_reserved_blocks) =
        nom_u64(input, "superblock snapshot reserved blocks is truncated")?;
    let (input, snapshot_list) = nom_u32(input, "superblock snapshot list inode is truncated")?;
    let (input, error_count) = nom_u32(input, "superblock error count is truncated")?;
    let (input, first_error_time) = nom_u32(input, "superblock first error time is truncated")?;
    let (input, first_error_inode) = nom_u32(input, "superblock first error inode is truncated")?;
    let (input, first_error_block) = nom_u64(input, "superblock first error block is truncated")?;
    let (input, first_error_function) = nom_take(
        input,
        32_u16,
        "superblock first error function is truncated",
    )?;

    let first_error_function: [u8; 32] = first_error_function
        .try_into()
        .map_err(|_err| AccessorError::volume("superblock first error function is truncated"))?;
    let (input, first_error_line) = nom_u32(input, "superblock first error line is truncated")?;
    let (input, last_error_time) = nom_u32(input, "superblock last error time is truncated")?;
    let (input, last_error_inode) = nom_u32(input, "superblock last error inode is truncated")?;
    let (input, last_error_line) = nom_u32(input, "superblock last error line is truncated")?;
    let (input, last_error_block) = nom_u64(input, "superblock last error block is truncated")?;
    let (input, last_error_function) =
        nom_take(input, 32_u16, "superblock last error function is truncated")?;
    let last_error_function: [u8; 32] = last_error_function
        .try_into()
        .map_err(|_err| AccessorError::volume("superblock last error function is truncated"))?;

    let (input, mount_options_raw) =
        nom_take(input, 64_u16, "superblock mount options is truncated")?;
    let mount_options_raw: [u8; 64] = mount_options_raw
        .try_into()
        .map_err(|_err| AccessorError::volume("superblock mount options is truncated"))?;
    let (input, user_quota_inode) = nom_u32(input, "superblock user quota inode is truncated")?;
    let (input, group_quota_inode) = nom_u32(input, "superblock group quota inode is truncated")?;
    let (mut input, overhead_blocks) = nom_u32(input, "superblock overhead blocks is truncated")?;

    let mut backup_block_groups = [0; 2];
    for value in &mut backup_block_groups {
        let (remaining, parsed) = nom_u32(input, "superblock backup block groups is truncated")?;
        *value = parsed;
        input = remaining;
    }

    let (input, encryption_algorithms) =
        nom_take(input, 4_u8, "superblock encryption algorithms is truncated")?;
    let encryption_algorithms: [u8; 4] = encryption_algorithms
        .try_into()
        .map_err(|_err| AccessorError::volume("superblock encryption algorithms is truncated"))?;
    let (input, encryption_password_salt) = nom_take(
        input,
        16_u16,
        "superblock encryption password salt is truncated",
    )?;
    let encryption_password_salt: [u8; 16] =
        encryption_password_salt.try_into().map_err(|_err| {
            AccessorError::volume("superblock encryption password salt is truncated")
        })?;

    let (input, lost_and_found_inode) = nom_u32(input, "superblock lost+found inode is truncated")?;
    let (input, project_quota_inode) =
        nom_u32(input, "superblock project quota inode is truncated")?;
    let (input, checksum_seed) = nom_u32(input, "superblock checksum seed is truncated")?;
    let (input, write_time_high) = nom_u8(input, "superblock write time high is truncated")?;
    let (input, mount_time_high) = nom_u8(input, "superblock mount time high is truncated")?;
    let (input, creation_time_high) = nom_u8(input, "superblock creation time high is truncated")?;
    let (input, last_check_high) = nom_u8(input, "superblock last check time high is truncated")?;
    let (input, first_error_time_high) =
        nom_u8(input, "superblock first error time high is truncated")?;
    let (input, last_error_time_high) =
        nom_u8(input, "superblock last error time high is truncated")?;

    let (input, first_error_code) = nom_u8(input, "superblock first error code is truncated")?;
    let (input, last_error_code) = nom_u8(input, "superblock last error code is truncated")?;
    let (input, encoding) = nom_u16(input, "superblock filename encoding is truncated")?;
    let (input, encoding_flags) =
        nom_u16(input, "superblock filename encoding flags is truncated")?;
    let (input, orphan_file_inode) = nom_u32(input, "superblock orphan file inode is truncated")?;
    let (input, reserved) = nom_take(input, 376_u16, "superblock reserved bytes is truncated")?;
    let reserved: [u8; 376] = reserved
        .try_into()
        .map_err(|_err| AccessorError::volume("superblock reserved bytes is truncated"))?;
    let (_input, checksum) = nom_u32(input, "superblock checksum is truncated")?;

    Ok(SuperBlock {
        inodes_count,
        blocks_count_low,
        reserved_blocks_count_low,
        free_blocks_count_low,
        free_inodes_count,
        first_data_block,
        log_block_size,
        log_cluster_size,
        blocks_per_group,
        clusters_per_group,
        inodes_per_group,
        mount_time,
        write_time,
        mount_count,
        max_mount_count,
        magic,
        state,
        errors,
        minor_revision,
        last_check,
        check_interval,
        creator_os,
        revision,
        default_reserved_uid,
        default_reserved_gid,
        first_inode,
        inode_size,
        block_group_number,
        compatible_features,
        incompatible_features,
        read_only_features,
        uuid,
        volume_name_raw,
        last_mounted_raw,
        algorithm_usage_bitmap,
        prealloc_blocks,
        prealloc_dir_blocks,
        reserved_gdt_blocks,
        journal_uuid,
        journal_inode,
        journal_device,
        last_orphan,
        hash_seed,
        default_hash_version,
        journal_backup_type,
        descriptor_size,
        default_mount_options,
        first_meta_block_group,
        creation_time,
        journal_blocks,
        blocks_count_high,
        reserved_blocks_count_high,
        free_blocks_count_high,
        min_extra_inode_size,
        wanted_extra_inode_size,
        flags,
        raid_stride,
        mmp_interval,
        mmp_block,
        raid_stripe_width,
        log_groups_per_flex,
        checksum_type,
        encryption_level,
        reserved_padding,
        kilobytes_written,
        snapshot_inode,
        snapshot_id,
        snapshot_reserved_blocks,
        snapshot_list,
        error_count,
        first_error_time,
        first_error_inode,
        first_error_block,
        first_error_function,
        first_error_line,
        last_error_time,
        last_error_inode,
        last_error_line,
        last_error_block,
        last_error_function,
        mount_options_raw,
        user_quota_inode,
        group_quota_inode,
        overhead_blocks,
        backup_block_groups,
        encryption_algorithms,
        encryption_password_salt,
        lost_and_found_inode,
        project_quota_inode,
        checksum_seed,
        write_time_high,
        mount_time_high,
        creation_time_high,
        last_check_high,
        first_error_time_high,
        last_error_time_high,
        first_error_code,
        last_error_code,
        encoding,
        encoding_flags,
        orphan_file_inode,
        reserved,
        checksum,
    })
}

/// Use upper 8 bits if required
fn combine_u32(low: u32, high: u32, include_high: bool) -> u64 {
    u64::from(low)
        | if include_high {
            u64::from(high) << 32
        } else {
            0
        }
}

/// Map parsing errors to `AccessorError`
fn ext4_error(reason: impl Into<String>) -> AccessorError {
    AccessorError::ext4(None, reason)
}

#[cfg(test)]
mod tests {
    use super::{SuperBlock, parse_superblock};
    use crate::accessor::error::{AccessorError, AccessorResult};
    use std::{fs::read, path::PathBuf};

    fn fixture() -> Vec<u8> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ext4/debian/bookworm/firstblock.raw");
        read(path).unwrap()
    }

    fn ext4_reason<T>(result: AccessorResult<T>) -> String {
        match result {
            Err(AccessorError::Ext4 { reason, .. }) => reason,
            Err(err) => panic!("expected EXT4 error, got {err}"),
            Ok(_) => panic!("expected EXT4 error"),
        }
    }

    #[test]
    fn test_parse_superblock_debian12() {
        let result = parse_superblock(&fixture()).unwrap();

        assert_eq!(result.magic, 0xef53);
        assert_eq!(result.log_block_size, 2);
        assert_eq!(
            result.filesystem_id(),
            "e7513b50-d6ad-4b29-9d25-b2b45b6346dc"
        );

        assert_eq!(result.incompatible_features, 0x2c6);
        assert!(result.has_incompatible(SuperBlock::EXTENTS));
        assert!(result.has_incompatible(SuperBlock::BIT64));
        assert!(result.has_incompatible(SuperBlock::FLEX_BG));
        assert!(result.has_read_only(SuperBlock::METADATA_CSUM));

        assert_eq!(
            result.hash_seed,
            [2876337945, 3243118446, 1830577543, 3948791912]
        );

        assert_eq!(result.checksum, 1082325001);

        assert_eq!(result.creator(), "Linux");
        assert_eq!(result.state(), ["Cleanly Unmounted"]);
        assert_eq!(result.error_policy(), "Continue");
    }

    #[test]
    fn test_parse_superblock_rejects_truncated_data() {
        let reason = ext4_reason(parse_superblock(&[0; 16]));
        assert!(reason.contains("truncated"));
        assert!(reason.contains("1024"));
    }

    #[test]
    fn test_parse_superblock_rejects_invalid_magic() {
        let mut data = fixture();
        data[0x38..0x3a].copy_from_slice(&0_u16.to_le_bytes());

        let reason = ext4_reason(parse_superblock(&data));
        assert!(reason.contains("invalid superblock magic"));
        assert!(reason.contains("0xef53"));
    }

    #[test]
    fn test_parse_superblock_rejects_unknown_revision() {
        let mut data = fixture();
        data[0x4c..0x50].copy_from_slice(&2_u32.to_le_bytes());

        assert!(ext4_reason(parse_superblock(&data)).contains("unsupported superblock revision 2"));
    }

    #[test]
    fn test_parse_superblock_rejects_unknown_checksum_type() {
        let mut data = fixture();
        data[0x175] = 2;

        assert!(ext4_reason(parse_superblock(&data)).contains("checksum type 2"));
    }

    #[test]
    fn test_feature_masks_preserve_unknown_bits() {
        let mut data = fixture();
        let unknown = 1 << 31;
        let features = u32::from_le_bytes(data[0x60..0x64].try_into().unwrap()) | unknown;
        data[0x60..0x64].copy_from_slice(&features.to_le_bytes());

        let result = parse_superblock(&data).unwrap();
        assert_eq!(result.unknown_incompatible(), unknown);
    }

    #[test]
    fn test_64_bit_block_counts_only_use_high_bits_when_enabled() {
        let mut data = fixture();
        data[0x04..0x08].copy_from_slice(&5_u32.to_le_bytes());
        data[0x150..0x154].copy_from_slice(&2_u32.to_le_bytes());

        let result = parse_superblock(&data).unwrap();
        assert_eq!(result.blocks_count(), (2_u64 << 32) | 5);

        let mut without_64_bit = data;
        let features =
            u32::from_le_bytes(without_64_bit[0x60..0x64].try_into().unwrap()) & !SuperBlock::BIT64;
        without_64_bit[0x60..0x64].copy_from_slice(&features.to_le_bytes());

        let result = parse_superblock(&without_64_bit).unwrap();
        assert_eq!(result.blocks_count(), 5);
    }

    #[test]
    fn test_fixed_arrays_preserve_raw_bytes() {
        let result: SuperBlock = parse_superblock(&fixture()).unwrap();

        assert_eq!(result.uuid.len(), 16);
        assert_eq!(result.hash_seed.len(), 4);
        assert_eq!(result.journal_blocks.len(), 17);
        assert_eq!(result.encryption_algorithms.len(), 4);
        assert_eq!(result.reserved.len(), 376);
    }
}
