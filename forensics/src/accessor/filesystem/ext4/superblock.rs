use crate::utils::{strings::extract_utf8_string, uuid::format_guid_be_bytes};
use nom::{
    bytes::complete::take,
    number::complete::{le_u8, le_u16, le_u32, le_u64, le_u128},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SuperBlock {
    pub number_inodes: u32,
    pub number_blocks: u32,
    pub reserved_blocks: u32,
    pub unallocated_blocks: u32,
    pub unallocated_inodes: u32,
    pub root_group_block_number: u32,
    pub block_size: u32,
    pub fragment_size: u32,
    pub number_blocks_per_block_group: u32,
    pub number_fragments_per_block_group: u32,
    pub number_inodes_per_block_group: u32,
    pub last_mount_time: u32,
    pub last_write_time: u32,
    pub current_mount_count: u16,
    pub max_mount_count: u16,
    pub signature: u16,
    pub filesystem_flags: Vec<FsFlags>,
    pub error_status: ErrorStatus,
    pub minor_version: u16,
    pub last_consistency_time: u32,
    pub consistency_interval: u32,
    pub creator_os: CreatorOs,
    pub format_revision: FormatRevision,
    pub uid: u16,
    pub gid: u16,
    /**If major version is FormatRevision.DynamicRevision */
    pub first_nonreserved_inode: u32,
    /**If major version is FormatRevision.DynamicRevision */
    pub inode_size: u16,
    /**If major version is FormatRevision.DynamicRevision */
    pub block_group: u16,
    /**If major version is FormatRevision.DynamicRevision */
    pub compatible_feature_flags: Vec<FeatureFlags>,
    /**If major version is FormatRevision.DynamicRevision */
    pub incompatible_features_flags: Vec<IncompatFlags>,
    /**If major version is FormatRevision.DynamicRevision */
    pub read_only_flags: Vec<ReadOnlyFlags>,
    /**If major version is FormatRevision.DynamicRevision */
    pub filesystem_id: String,
    /**If major version is FormatRevision.DynamicRevision */
    pub volume_name: String,
    /**If major version is FormatRevision.DynamicRevision */
    pub last_mount_path: String,
    /**If major version is FormatRevision.DynamicRevision */
    pub bitmap_algorithm: u32,
    /**If FeatureFlags.PreAlloc enabled*/
    pub preallocated_blocks_per_file: u8,
    /**If FeatureFlags.PreAlloc enabled*/
    pub preallocated_blocks_per_directory: u8,
    /**If FeatureFlags.PreAlloc enabled*/
    pub reserved_gdt: u16,
    /**If FeatureFlags.HasJournal enabled */
    pub journal_id: String,
    /**If FeatureFlags.HasJournal enabled */
    pub journal_inode: u32,
    /**If FeatureFlags.HasJournal enabled */
    pub journal_device: u32,
    /**If FeatureFlags.HasJournal enabled */
    pub orphan_inode_list: u32,
    /**If FeatureFlags.HasJournal enabled */
    pub hash_tree_seed: Vec<u32>,
    /**If FeatureFlags.HasJournal enabled */
    pub hash_version: u8,
    /**If FeatureFlags.HasJournal enabled */
    pub journal_backup_type: u8,
    /**If FeatureFlags.HasJournal enabled */
    pub group_descriptor_size: u16,
    /**If FeatureFlags.HasJournal enabled */
    pub default_mount_options: u32,
    /**If FeatureFlags.HasJournal enabled */
    pub first_metablock_group: u32,
    /**If FeatureFlags.HasJournal enabled */
    pub filesystem_creation: u32,
    /**If FeatureFlags.HasJournal enabled */
    pub backup_journal_inodes: Vec<u32>,
    /**If IncompatFlags.Bit64 enabled */
    pub upper_number_blocks: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub upper_number_reserved_blocks: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub upper_unallocated_blocks: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub minimum_inode_size: u16,
    /**If IncompatFlags.Bit64 enabled */
    pub reserved_inode_size: u16,
    /**If IncompatFlags.Bit64 enabled */
    pub misc_flags: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub raid_stride: u16,
    /**If IncompatFlags.Bit64 enabled */
    pub mount_protection_update_interval: u16,
    /**If IncompatFlags.Bit64 enabled */
    pub block_mount_protection: u64,
    /**If IncompatFlags.Bit64 enabled */
    pub blocks_on_all_data_disks: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub flex_block_group_size: u8,
    /**If IncompatFlags.Bit64 enabled */
    pub checksum_type: Checksum,
    /**If IncompatFlags.Bit64 enabled */
    pub encryption_level: u8,
    /**If IncompatFlags.Bit64 enabled */
    _padding: u8,
    /**If IncompatFlags.Bit64 enabled */
    pub s_kbytes_written: u64,
    /**If IncompatFlags.Bit64 enabled */
    pub inode_snapshot_list: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub active_snapshot_id: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub snapshot_reserved_blocks_count: u64,
    /**If IncompatFlags.Bit64 enabled */
    pub inode_snapshot_list_count: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub error_count: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub first_error_time: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub first_error_ino: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub first_error_block: u64,
    /**If IncompatFlags.Bit64 enabled */
    pub first_error_function: Vec<u8>,
    /**If IncompatFlags.Bit64 enabled */
    pub first_error_line: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub last_error_time: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub last_error_ino: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub last_error_line: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub last_error_block: u64,
    /**If IncompatFlags.Bit64 enabled */
    pub last_error_func: Vec<u8>,
    /**If IncompatFlags.Bit64 enabled */
    pub mount_options: Vec<u8>,
    /**If IncompatFlags.Bit64 enabled */
    pub usr_quota_inum: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub grp_quota_inum: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub overhead_clusters: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub backup_bgs: Vec<u32>,
    /**If IncompatFlags.Bit64 enabled */
    pub encrypt_algorithms: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub encrypt_password_salt: u128,
    /**If IncompatFlags.Bit64 enabled */
    pub lpf_ino: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub prj_quota_inum: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub checksum_seed: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub wtime_hi: u8,
    /**If IncompatFlags.Bit64 enabled */
    pub mtime_hi: u8,
    /**If IncompatFlags.Bit64 enabled */
    pub mfs_time_hi: u8,
    /**If IncompatFlags.Bit64 enabled */
    pub lastcheck_hi: u8,
    /**If IncompatFlags.Bit64 enabled */
    pub first_error_time_hi: u8,
    /**If IncompatFlags.Bit64 enabled */
    pub last_error_time_hi: u8,
    /**If IncompatFlags.Bit64 enabled */
    pub first_error_code: u8,
    /**If IncompatFlags.Bit64 enabled */
    pub last_error_code: u8,
    /**If IncompatFlags.Bit64 enabled */
    pub encoding: u16,
    /**If IncompatFlags.Bit64 enabled */
    pub encoding_flags: u16,
    /**If IncompatFlags.Bit64 enabled */
    pub orphan_file_inum: u32,
    /**If IncompatFlags.Bit64 enabled */
    pub reserved: Vec<u8>,
    /**If IncompatFlags.Bit64 enabled */
    pub checksum: u32,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum FsFlags {
    Clean,
    HasErrors,
    RecoveringOrphanInodes,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum ErrorStatus {
    Continue,
    RemountReadOnly,
    Panic,
    Unknown,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum CreatorOs {
    Linux,
    GnuHurd,
    Masix,
    FreeBsd,
    Lites,
    Unknown,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum FormatRevision {
    OldRevision,
    DynamicRevision,
    Unknown,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum FeatureFlags {
    PreAlloc,
    ImagicInodes,
    HasJournal,
    ExtAttr,
    ResizeInode,
    DirIndex,
    LazyBlockGroup,
    ExcludeInode,
    ExcludeBitmap,
    SparseSuper2,
    FastCommit,
    StableInodes,
    OrphanFiles,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum IncompatFlags {
    Compression,
    FileType,
    Recover,
    JournalDevice,
    MetaBlock,
    Extents,
    Bit64,
    MultipleMountProtection,
    FlexibleBlockGroups,
    ExtendedAttributeInodes,
    DataDirectory,
    ChecksumSeedSuperblock,
    LargeDirectory,
    InlineData,
    Encrypt,
    CaseFold,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum ReadOnlyFlags {
    SparseSuper,
    LargeFile,
    BtreeDirectory,
    HugeFile,
    GroupDescriptorChecksums,
    DirectoryNlink,
    ExtraIsize,
    HasSnapshot,
    Quota,
    BigAlloc,
    MetadataChecksum,
    Replicas,
    ReadOnly,
    Project,
    SharedBlocks,
    Verity,
    OrphanPresent,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Checksum {
    /**0x1edc6f41 */
    Castagnoli,
}

impl SuperBlock {
    /// Pare the block data associated with the superblock
    fn parse_block(data: &[u8]) -> nom::IResult<&[u8], SuperBlock> {
        let (input, number_inodes) = le_u32(data)?;
        let (input, number_blocks) = le_u32(input)?;
        let (input, reserved_blocks) = le_u32(input)?;
        let (input, unallocated_blocks) = le_u32(input)?;
        let (input, unallocated_inodes) = le_u32(input)?;
        let (input, root_group_block_number) = le_u32(input)?;

        let (input, block_size) = le_u32(input)?;
        let (input, fragment_size) = le_u32(input)?;
        let (input, number_blocks_per_block_group) = le_u32(input)?;
        let (input, number_fragments_per_block_group) = le_u32(input)?;
        let (input, number_inodes_per_block_group) = le_u32(input)?;

        let (input, last_mount_time) = le_u32(input)?;
        let (input, last_write_time) = le_u32(input)?;
        let (input, current_mount_count) = le_u16(input)?;
        let (input, max_mount_count) = le_u16(input)?;
        let (input, signature) = le_u16(input)?;
        let (input, filesystem_flags) = le_u16(input)?;
        let (input, error_status) = le_u16(input)?;
        let (input, minor_version) = le_u16(input)?;

        let (input, last_consistency_time) = le_u32(input)?;
        let (input, consistency_interval) = le_u32(input)?;
        let (input, creator_os) = le_u32(input)?;
        let (input, format_revision) = le_u32(input)?;

        let (input, uid) = le_u16(input)?;
        let (input, gid) = le_u16(input)?;

        // Next values are only set for FormatRevision.DynamicRevision flag
        let (input, first_nonreserved_inode) = le_u32(input)?;
        let (input, inode_size) = le_u16(input)?;
        let (input, block_group) = le_u16(input)?;
        let (input, compatible_feature_flags) = le_u32(input)?;
        let (input, incompatible_features_flags) = le_u32(input)?;
        let (input, read_only_flags) = le_u32(input)?;
        let uuid_size: u8 = 16;
        let (input, filesystem_id_data) = take(uuid_size)(input)?;
        let (input, volume_name_data) = take(uuid_size)(input)?;
        let mount_path_length: u8 = 64;
        let (input, mount_path_data) = take(mount_path_length)(input)?;
        let (input, bitmap_algorithm) = le_u32(input)?;

        // Next values are only set for FeatureFlags.PreAlloc
        let (input, preallocated_blocks_per_file) = le_u8(input)?;
        let (input, preallocated_blocks_per_directory) = le_u8(input)?;
        let (input, reserved_gdt) = le_u16(input)?;

        // Next values are only set for FeatureFlags.HasJournal
        let (input, journal_id_data) = take(uuid_size)(input)?;
        let (input, journal_inode) = le_u32(input)?;
        let (input, journal_device) = le_u32(input)?;
        let (mut input, orphan_inode_list) = le_u32(input)?;

        let mut hash_tree_seed = Vec::new();
        let max_seed = 4;
        let mut count = 0;
        while count < max_seed {
            let (remaining, seed) = le_u32(input)?;
            input = remaining;
            hash_tree_seed.push(seed);

            count += 1;
        }
        let (input, hash_version) = le_u8(input)?;
        let (input, journal_backup_type) = le_u8(input)?;
        let (input, group_descriptor_size) = le_u16(input)?;
        let (input, default_mount_options) = le_u32(input)?;
        let (input, first_metablock_group) = le_u32(input)?;
        let (mut input, filesystem_creation) = le_u32(input)?;

        let max_inodes = 17;
        count = 0;
        let mut backup_journal_inodes = Vec::new();
        while count < max_inodes {
            let (remaining, inode) = le_u32(input)?;
            input = remaining;
            backup_journal_inodes.push(inode);

            count += 1;
        }

        // Next values are only set for IncompatFlags.Bit64
        let (input, upper_number_blocks) = le_u32(input)?;
        let (input, upper_number_reserved_blocks) = le_u32(input)?;
        let (input, upper_unallocated_blocks) = le_u32(input)?;
        let (input, minimum_inode_size) = le_u16(input)?;
        let (input, reserved_inode_size) = le_u16(input)?;

        let (input, misc_flags) = le_u32(input)?;
        let (input, raid_stride) = le_u16(input)?;
        let (input, mount_protection_update_interval) = le_u16(input)?;
        let (input, block_mount_protection) = le_u64(input)?;
        let (input, blocks_on_all_data_disks) = le_u32(input)?;

        let (input, flex_block_group_size) = le_u8(input)?;
        //Checksum type should always be Castagnoli
        let (input, _checksum_type_data) = le_u8(input)?;
        let (input, encryption_level) = le_u8(input)?;
        let (input, _padding) = le_u8(input)?;

        let (input, s_kbytes_written) = le_u64(input)?;
        let (input, inode_snapshot_list) = le_u32(input)?;
        let (input, active_snapshot_id) = le_u32(input)?;
        let (input, snapshot_reserved_blocks_count) = le_u64(input)?;
        let (input, inode_snapshot_list_count) = le_u32(input)?;
        let (input, error_count) = le_u32(input)?;
        let (input, first_error_time) = le_u32(input)?;
        let (input, first_error_ino) = le_u32(input)?;

        let (input, first_error_block) = le_u64(input)?;
        let error_func_length: u8 = 32;
        let (input, first_error_function) = take(error_func_length)(input)?;
        let (input, first_error_line) = le_u32(input)?;
        let (input, last_error_time) = le_u32(input)?;
        let (input, last_error_ino) = le_u32(input)?;
        let (input, last_error_line) = le_u32(input)?;
        let (input, last_error_block) = le_u64(input)?;
        let (input, last_error_func) = take(error_func_length)(input)?;

        let mount_options_length: u8 = 64;
        let (input, mount_options) = take(mount_options_length)(input)?;
        let (input, usr_quota_inum) = le_u32(input)?;
        let (input, grp_quota_inum) = le_u32(input)?;
        let (mut input, overhead_clusters) = le_u32(input)?;

        let backup_bgs_limit = 2;
        count = 0;
        let mut backup_bgs = Vec::new();
        while count < backup_bgs_limit {
            let (remaining, backup) = le_u32(input)?;
            input = remaining;
            backup_bgs.push(backup);
            count += 1;
        }

        let (input, encrypt_algorithms) = le_u32(input)?;
        let (input, encrypt_password_salt) = le_u128(input)?;
        let (input, lpf_ino) = le_u32(input)?;
        let (input, prj_quota_inum) = le_u32(input)?;
        let (input, checksum_seed) = le_u32(input)?;

        let (input, wtime_hi) = le_u8(input)?;
        let (input, mtime_hi) = le_u8(input)?;
        let (input, mfs_time_hi) = le_u8(input)?;
        let (input, lastcheck_hi) = le_u8(input)?;
        let (input, first_error_time_hi) = le_u8(input)?;
        let (input, last_error_time_hi) = le_u8(input)?;
        let (input, first_error_code) = le_u8(input)?;
        let (input, last_error_code) = le_u8(input)?;

        let (input, encoding) = le_u16(input)?;
        let (input, encoding_flags) = le_u16(input)?;
        let (input, orphan_file_inum) = le_u32(input)?;

        let reserved_length: u16 = 376;
        let (input, reserved_data) = take(reserved_length)(input)?;
        let (input, checksum) = le_u32(input)?;

        let block_value = SuperBlock {
            number_inodes,
            number_blocks,
            reserved_blocks,
            unallocated_blocks,
            unallocated_inodes,
            root_group_block_number,
            block_size,
            fragment_size,
            number_blocks_per_block_group,
            number_fragments_per_block_group,
            number_inodes_per_block_group,
            last_mount_time,
            last_write_time,
            current_mount_count,
            max_mount_count,
            signature,
            filesystem_flags: SuperBlock::filesystem_flags(filesystem_flags),
            error_status: SuperBlock::error_handling_flags(error_status),
            minor_version,
            last_consistency_time,
            consistency_interval,
            creator_os: SuperBlock::creator(creator_os),
            format_revision: SuperBlock::revision(format_revision),
            uid,
            gid,
            first_nonreserved_inode,
            inode_size,
            block_group,
            compatible_feature_flags: SuperBlock::compat_flabs(compatible_feature_flags),
            incompatible_features_flags: SuperBlock::incompat_flabs(incompatible_features_flags),
            read_only_flags: SuperBlock::readonly_flags(read_only_flags),
            filesystem_id: format_guid_be_bytes(filesystem_id_data),
            volume_name: extract_utf8_string(volume_name_data),
            last_mount_path: extract_utf8_string(mount_path_data),
            bitmap_algorithm,
            preallocated_blocks_per_file,
            preallocated_blocks_per_directory,
            reserved_gdt,
            journal_id: format_guid_be_bytes(journal_id_data),
            journal_inode,
            journal_device,
            orphan_inode_list,
            hash_tree_seed,
            hash_version,
            journal_backup_type,
            group_descriptor_size,
            default_mount_options,
            first_metablock_group,
            filesystem_creation,
            backup_journal_inodes,
            upper_number_blocks,
            upper_number_reserved_blocks,
            upper_unallocated_blocks,
            minimum_inode_size,
            reserved_inode_size,
            misc_flags,
            raid_stride,
            mount_protection_update_interval,
            block_mount_protection,
            blocks_on_all_data_disks,
            flex_block_group_size,
            checksum_type: Checksum::Castagnoli,
            encryption_level,
            _padding,
            s_kbytes_written,
            inode_snapshot_list,
            active_snapshot_id,
            snapshot_reserved_blocks_count,
            inode_snapshot_list_count,
            error_count,
            first_error_time,
            first_error_ino,
            first_error_block,
            first_error_function: first_error_function.to_vec(),
            first_error_line,
            last_error_time,
            last_error_ino,
            last_error_line,
            last_error_block,
            last_error_func: last_error_func.to_vec(),
            mount_options: mount_options.to_vec(),
            usr_quota_inum,
            grp_quota_inum,
            overhead_clusters,
            backup_bgs,
            encrypt_algorithms,
            encrypt_password_salt,
            lpf_ino,
            prj_quota_inum,
            checksum_seed,
            wtime_hi,
            mtime_hi,
            mfs_time_hi,
            lastcheck_hi,
            first_error_time_hi,
            last_error_time_hi,
            first_error_code,
            last_error_code,
            encoding,
            encoding_flags,
            orphan_file_inum,
            reserved: reserved_data.to_vec(),
            checksum,
        };

        Ok((input, block_value))
    }

    /// Determine the EXT4 filesystem files
    fn filesystem_flags(data: u16) -> Vec<FsFlags> {
        let mut flags = Vec::new();
        if data == 0x1 {
            flags.push(FsFlags::Clean);
        } else if data == 0x2 {
            flags.push(FsFlags::HasErrors);
        } else if data == 0x3 {
            flags.push(FsFlags::RecoveringOrphanInodes);
        }

        flags
    }

    /// Determine the EXT4 error handling flags
    fn error_handling_flags(data: u16) -> ErrorStatus {
        match data {
            0x1 => ErrorStatus::Continue,
            0x2 => ErrorStatus::RemountReadOnly,
            0x3 => ErrorStatus::Panic,
            _ => ErrorStatus::Unknown,
        }
    }

    /// Determine the OS the created the filesystem. Likely Linux
    fn creator(data: u32) -> CreatorOs {
        match data {
            0x0 => CreatorOs::Linux,
            0x1 => CreatorOs::GnuHurd,
            0x2 => CreatorOs::Masix,
            0x3 => CreatorOs::FreeBsd,
            0x4 => CreatorOs::Lites,
            _ => CreatorOs::Unknown,
        }
    }

    /// EXT4 format revision
    fn revision(data: u32) -> FormatRevision {
        match data {
            0x0 => FormatRevision::OldRevision,
            0x1 => FormatRevision::DynamicRevision,
            _ => FormatRevision::Unknown,
        }
    }

    /// Determine compatibility flags for the EXT4 filesystem
    fn compat_flabs(data: u32) -> Vec<FeatureFlags> {
        let mut flags = Vec::new();
        if (data & 0x1) == 0x1 {
            flags.push(FeatureFlags::PreAlloc);
        }
        if (data & 0x2) == 0x2 {
            flags.push(FeatureFlags::ImagicInodes);
        }
        if (data & 0x4) == 0x4 {
            flags.push(FeatureFlags::HasJournal);
        }
        if (data & 0x8) == 0x8 {
            flags.push(FeatureFlags::ExtAttr);
        }
        if (data & 0x10) == 0x10 {
            flags.push(FeatureFlags::ResizeInode);
        }
        if (data & 0x20) == 0x20 {
            flags.push(FeatureFlags::DirIndex);
        }
        if (data & 0x40) == 0x40 {
            flags.push(FeatureFlags::LazyBlockGroup);
        }
        if (data & 0x80) == 0x80 {
            flags.push(FeatureFlags::ExcludeInode);
        }
        if (data & 0x100) == 0x100 {
            flags.push(FeatureFlags::ExcludeBitmap);
        }
        if (data & 0x200) == 0x200 {
            flags.push(FeatureFlags::SparseSuper2);
        }
        if (data & 0x400) == 0x400 {
            flags.push(FeatureFlags::FastCommit);
        }
        if (data & 0x800) == 0x800 {
            flags.push(FeatureFlags::StableInodes);
        }
        if (data & 0x1000) == 0x1000 {
            flags.push(FeatureFlags::OrphanFiles);
        }

        flags
    }

    /// Determine incompatibility flags for the EXT4 filesystem
    fn incompat_flabs(data: u32) -> Vec<IncompatFlags> {
        let mut flags = Vec::new();
        if (data & 0x1) == 0x1 {
            flags.push(IncompatFlags::Compression);
        }
        if (data & 0x2) == 0x2 {
            flags.push(IncompatFlags::FileType);
        }
        if (data & 0x4) == 0x4 {
            flags.push(IncompatFlags::Recover);
        }
        if (data & 0x8) == 0x8 {
            flags.push(IncompatFlags::JournalDevice);
        }
        if (data & 0x10) == 0x10 {
            flags.push(IncompatFlags::MetaBlock);
        }
        if (data & 0x40) == 0x40 {
            flags.push(IncompatFlags::Extents);
        }
        if (data & 0x80) == 0x80 {
            flags.push(IncompatFlags::Bit64);
        }
        if (data & 0x100) == 0x100 {
            flags.push(IncompatFlags::MultipleMountProtection);
        }
        if (data & 0x200) == 0x200 {
            flags.push(IncompatFlags::FlexibleBlockGroups);
        }
        if (data & 0x400) == 0x400 {
            flags.push(IncompatFlags::ExtendedAttributeInodes);
        }
        if (data & 0x1000) == 0x1000 {
            flags.push(IncompatFlags::DataDirectory);
        }
        if (data & 0x2000) == 0x2000 {
            flags.push(IncompatFlags::ChecksumSeedSuperblock);
        }
        if (data & 0x4000) == 0x4000 {
            flags.push(IncompatFlags::LargeDirectory);
        }
        if (data & 0x8000) == 0x8000 {
            flags.push(IncompatFlags::InlineData);
        }
        if (data & 0x10000) == 0x10000 {
            flags.push(IncompatFlags::Encrypt);
        }
        if (data & 0x20000) == 0x20000 {
            flags.push(IncompatFlags::CaseFold);
        }

        flags
    }

    /// Determine if the EXT4 filesystem is readonly
    fn readonly_flags(data: u32) -> Vec<ReadOnlyFlags> {
        let mut flags = Vec::new();
        if (data & 0x1) == 0x1 {
            flags.push(ReadOnlyFlags::SparseSuper);
        }
        if (data & 0x2) == 0x2 {
            flags.push(ReadOnlyFlags::LargeFile);
        }
        if (data & 0x4) == 0x4 {
            flags.push(ReadOnlyFlags::BtreeDirectory);
        }
        if (data & 0x8) == 0x8 {
            flags.push(ReadOnlyFlags::HugeFile);
        }
        if (data & 0x10) == 0x10 {
            flags.push(ReadOnlyFlags::GroupDescriptorChecksums);
        }
        if (data & 0x20) == 0x20 {
            flags.push(ReadOnlyFlags::DirectoryNlink);
        }
        if (data & 0x40) == 0x40 {
            flags.push(ReadOnlyFlags::ExtraIsize);
        }
        if (data & 0x80) == 0x80 {
            flags.push(ReadOnlyFlags::HasSnapshot);
        }
        if (data & 0x100) == 0x100 {
            flags.push(ReadOnlyFlags::Quota);
        }
        if (data & 0x200) == 0x200 {
            flags.push(ReadOnlyFlags::BigAlloc);
        }
        if (data & 0x400) == 0x400 {
            flags.push(ReadOnlyFlags::MetadataChecksum);
        }
        if (data & 0x800) == 0x800 {
            flags.push(ReadOnlyFlags::Replicas);
        }
        if (data & 0x1000) == 0x1000 {
            flags.push(ReadOnlyFlags::ReadOnly);
        }
        if (data & 0x2000) == 0x2000 {
            flags.push(ReadOnlyFlags::Project);
        }
        if (data & 0x4000) == 0x4000 {
            flags.push(ReadOnlyFlags::SharedBlocks);
        }
        if (data & 0x8000) == 0x8000 {
            flags.push(ReadOnlyFlags::Verity);
        }
        if (data & 0x10000) == 0x10000 {
            flags.push(ReadOnlyFlags::OrphanPresent);
        }

        flags
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::filesystem::ext4::superblock::IncompatFlags::{
        Bit64, Extents, FileType, FlexibleBlockGroups, Recover,
    };
    use crate::accessor::filesystem::ext4::superblock::{
        CreatorOs, ErrorStatus, FormatRevision, SuperBlock,
    };
    use std::{fs::read, path::PathBuf};

    #[test]
    fn test_parse_superblock_debian12() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/filesystems/ext4/debian/bookworm/firstblock.raw");
        let test = read(test_location.to_str().unwrap()).unwrap();

        let (_, result) = SuperBlock::parse_block(&test).unwrap();
        assert_eq!(result.block_size, 2);
        assert_eq!(result.filesystem_id, "e7513b50-d6ad-4b29-9d25-b2b45b6346dc");
        assert_eq!(
            result.incompatible_features_flags,
            vec![FileType, Recover, Extents, Bit64, FlexibleBlockGroups]
        );
        assert_eq!(
            result.hash_tree_seed,
            [2876337945, 3243118446, 1830577543, 3948791912]
        );
        assert_eq!(result.checksum, 1082325001);
    }

    #[test]
    fn test_filesystem_flags() {
        let test = [1, 2, 3];
        for entry in test {
            assert_eq!(SuperBlock::filesystem_flags(entry).len(), 1);
        }
    }

    #[test]
    fn test_error_handling_flags() {
        let test = [1, 2, 3];
        for entry in test {
            assert_ne!(
                SuperBlock::error_handling_flags(entry),
                ErrorStatus::Unknown
            );
        }
    }

    #[test]
    fn test_creator() {
        let test = [0, 1, 2, 3, 4];
        for entry in test {
            assert_ne!(SuperBlock::creator(entry), CreatorOs::Unknown);
        }
    }

    #[test]
    fn test_revision() {
        let test = [0, 1];
        for entry in test {
            assert_ne!(SuperBlock::revision(entry), FormatRevision::Unknown);
        }
    }

    #[test]
    fn test_compat_flabs() {
        let test = [
            0x1, 0x2, 0x4, 0x8, 0x10, 0x20, 0x40, 0x80, 0x100, 0x200, 0x400, 0x800, 0x1000,
        ];
        for entry in test {
            assert!(!SuperBlock::compat_flabs(entry).is_empty());
        }
    }

    #[test]
    fn test_incompat_flabs() {
        let test = [
            0x1, 0x2, 0x4, 0x8, 0x10, 0x40, 0x80, 0x100, 0x200, 0x400, 0x1000, 0x2000, 0x4000,
            0x8000, 0x10000, 0x20000,
        ];
        for entry in test {
            assert!(!SuperBlock::incompat_flabs(entry).is_empty());
        }
    }

    #[test]
    fn test_readonly_flags() {
        let test = [
            0x1, 0x2, 0x4, 0x8, 0x10, 0x20, 0x40, 0x80, 0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000,
            0x4000, 0x8000, 0x10000,
        ];
        for entry in test {
            assert!(!SuperBlock::readonly_flags(entry).is_empty())
        }
    }
}
