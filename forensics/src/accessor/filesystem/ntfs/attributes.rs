use crate::{
    accessor::{
        error::{AccessorError, AccessorResult},
        filesystem::ntfs::walk::ntfs_err,
    },
    utils::nom_helper::{Endian, nom_unsigned_four_bytes},
};
use common::{files::ReparseType, windows::ADSInfo};
use ntfs::{
    Ntfs, NtfsAttributeType, NtfsFile, NtfsReadSeek, attribute_value::NtfsAttributeValue,
    structured_values::NtfsAttributeList,
};
use std::io::{Read, Seek};

/// Walk the attribute list and grab the `ReparsePoint` attribute
pub(crate) fn read_reparse_data<T: Read + Seek>(
    reader: &mut T,
    file: &NtfsFile<'_>,
) -> AccessorResult<Vec<u8>> {
    let mut attrs = file.attributes();
    while let Some(item) = attrs.next(reader) {
        let item = item.map_err(ntfs_err)?;
        let attr = item.to_attribute().map_err(ntfs_err)?;
        if attr.ty().map_err(ntfs_err)? != NtfsAttributeType::ReparsePoint {
            continue;
        }
        let mut value = attr.value(reader).map_err(ntfs_err)?;
        return read_value_bytes(&mut value, reader, attr.value_length());
    }

    // No Reparse attribute
    Ok(Vec::new())
}

/// Read attribute data
pub(crate) fn read_named_data<T: Read + Seek>(
    reader: &mut T,
    file: &NtfsFile<'_>,
    stream_name: &str,
) -> AccessorResult<Vec<u8>> {
    // Read a specified attribute name
    // Such as a ADS attribute or $UsnJrnl:$J
    if !stream_name.is_empty() {
        return read_attribute_data(reader, file, stream_name);
    }

    // Read the default $DATA attribute. The attribute name is empty '""'
    let Some(item) = file.data(reader, stream_name) else {
        return Err(AccessorError::Ntfs {
            path: None,
            reason: format!("file has no `{stream_name}` data stream"),
        });
    };

    let item = item.map_err(ntfs_err)?;
    let attr = item.to_attribute().map_err(ntfs_err)?;
    let mut value = attr.value(reader).map_err(ntfs_err)?;

    read_value_bytes(&mut value, reader, attr.value_length())
}

/// Read a provided NTFS attribute name
///
/// Can be used to read Alternative Data Streams
pub(crate) fn read_attribute_data<T: Read + Seek>(
    reader: &mut T,
    file: &NtfsFile<'_>,
    stream_name: &str,
) -> AccessorResult<Vec<u8>> {
    // We need to walk the raw Attribute List
    // In case the attribute we want to really large
    let attrs_raw = file.attributes_raw();
    for item in attrs_raw {
        let item = item.map_err(ntfs_err)?;

        // Large attribute, need to iterate through the AttributeList to find it
        if item.ty().map_err(ntfs_err)? == NtfsAttributeType::AttributeList {
            let list = item
                .structured_value::<_, NtfsAttributeList<'_, '_>>(reader)
                .map_err(ntfs_err)?;
            let mut attr_bytes = Vec::new();
            let mut found = false;

            let mut list_iter = list.entries();
            while let Some(entry) = list_iter.next(reader) {
                let entry = entry.map_err(ntfs_err)?;

                if entry.name().to_string_lossy() != stream_name {
                    continue;
                }

                // We found our attribute
                found = true;
                let temp_file = entry.to_file(file.ntfs(), reader).map_err(ntfs_err)?;

                let entry_attr = entry.to_attribute(&temp_file).map_err(ntfs_err)?;
                let mut attr_value = entry_attr.value(reader).map_err(ntfs_err)?;

                // If the attribute is resident. We can just read all of it
                if entry_attr.is_resident() {
                    return read_value_bytes(&mut attr_value, reader, entry_attr.value_length());
                }

                let mut bytes = read_data_runs(&mut attr_value, reader, stream_name)?;
                if bytes.is_empty() {
                    continue;
                }

                let logical = entry_attr.value_length() as usize;
                // We skip sparse data when reading attribute data
                // Sparse data is treated as 0 bytes when calling `value_length()`
                if logical > 0 && bytes.len() > logical {
                    bytes.truncate(logical);
                }

                attr_bytes.append(&mut bytes);
            }

            // Attribute was found in the Attribute list
            if found {
                return Ok(attr_bytes);
            }
        } else if item.ty().map_err(ntfs_err)? == NtfsAttributeType::Data {
            if item.name().map_err(ntfs_err)?.to_string_lossy() != stream_name {
                continue;
            }
            let mut attr_value = item.value(reader).map_err(ntfs_err)?;

            // If the attribute is resident. We can just read all of it
            if item.is_resident() {
                return read_value_bytes(&mut attr_value, reader, item.value_length());
            }

            let mut bytes = read_data_runs(&mut attr_value, reader, stream_name)?;

            let logical = item.value_length() as usize;
            // We skip sparse data when reading attribute data
            // Sparse data is treated as 0 bytes when calling `value_length()`
            if logical > 0 && bytes.len() > logical {
                bytes.truncate(logical);
            }
            return Ok(bytes);
        }
    }

    Err(AccessorError::Ntfs {
        path: None,
        reason: format!("file has no `{stream_name}` data stream"),
    })
}

/// Read non-resident data runs
pub(crate) fn read_data_runs<T: Read + Seek>(
    value: &mut NtfsAttributeValue<'_, '_>,
    reader: &mut T,
    stream_name: &str,
) -> AccessorResult<Vec<u8>> {
    if let NtfsAttributeValue::NonResident(non_resident) = value {
        let mut out = Vec::new();
        let mut chunk = vec![0u8; 65536].into_boxed_slice();

        for data_run in non_resident.data_runs() {
            let mut run = data_run.map_err(ntfs_err)?;

            // Skip sparse data
            if run.data_position().value().is_none() {
                continue;
            }

            loop {
                let bytes = run.read(reader, &mut chunk).map_err(ntfs_err)?;
                if bytes == 0 {
                    break;
                }
                out.extend_from_slice(&chunk[..bytes]);
            }
        }

        return Ok(out);
    }

    Err(AccessorError::Ntfs {
        path: None,
        reason: format!("file has no non-resident `{stream_name}` stream"),
    })
}

/// Get the attribute bytes
pub(crate) fn read_value_bytes<T: Read + Seek>(
    value: &mut NtfsAttributeValue<'_, '_>,
    reader: &mut T,
    size: u64,
) -> AccessorResult<Vec<u8>> {
    let mut out = Vec::with_capacity(size as usize);
    let mut chunk = vec![0u8; 65536].into_boxed_slice();

    loop {
        let bytes = value.read(reader, &mut chunk).map_err(ntfs_err)?;
        if bytes == 0 {
            break;
        }
        out.extend_from_slice(&chunk[..bytes]);
    }

    Ok(out)
}

/// List all ADS attributes for provided file
pub(super) fn list_ads_names<T: Read + Seek>(
    ntfs: &Ntfs,
    reader: &mut T,
    file: &NtfsFile<'_>,
) -> AccessorResult<Vec<ADSInfo>> {
    let mut ads = Vec::new();

    for item in file.attributes_raw() {
        let item = item.map_err(ntfs_err)?;
        let ty = item.ty().map_err(ntfs_err)?;

        // Walk the AttributeList if we have lots of attributes for a file
        if ty == NtfsAttributeType::AttributeList {
            let list = item
                .structured_value::<_, NtfsAttributeList<'_, '_>>(reader)
                .map_err(ntfs_err)?;
            let mut list_iter = list.entries();

            while let Some(entry) = list_iter.next(reader) {
                let entry = entry.map_err(ntfs_err)?;
                let temp_file = entry.to_file(ntfs, reader).map_err(ntfs_err)?;
                let attr = entry.to_attribute(&temp_file).map_err(ntfs_err)?;

                if attr.ty().map_err(ntfs_err)? != NtfsAttributeType::Data {
                    continue;
                }

                let name = attr.name().map_err(ntfs_err)?.to_string_lossy();
                if name.is_empty() {
                    continue;
                }

                ads.push(ADSInfo {
                    name,
                    size: attr.value_length(),
                });
            }
            continue;
        }

        if ty != NtfsAttributeType::Data {
            continue;
        }
        let name = item.name().map_err(ntfs_err)?.to_string_lossy();
        if name.is_empty() {
            continue;
        }

        ads.push(ADSInfo {
            name,
            size: item.value_length(),
        });
    }

    Ok(ads)
}

/// Get the Reparse value type
pub(crate) fn get_reparse_type(data: &[u8]) -> AccessorResult<ReparseType> {
    let tag = match nom_unsigned_four_bytes(data, Endian::Le) {
        Ok((_, result)) => result,
        Err(_err) => return Ok(ReparseType::None),
    };

    Ok(reparse_type(tag))
}

/// Determine Reparse Type
fn reparse_type(tag: u32) -> ReparseType {
    match tag {
        0x00000000 => ReparseType::Reserved,
        0x00000001 => ReparseType::ReservedOne,
        0x00000002 => ReparseType::ReservedTwo,
        0xA0000003 => ReparseType::MountPoint,
        0xC0000004 => ReparseType::HierarchicalStorageManagement,
        0x80000005 => ReparseType::DriveExtender,
        0x80000006 => ReparseType::HierarchicalStorageManagement2,
        0x80000007 => ReparseType::SingleInstanceStorage,
        0x80000008 => ReparseType::Wim,
        0x80000009 => ReparseType::ClusteredSharedVolume,
        0x8000000A => ReparseType::DistributedFileSystem,
        0x8000000B => ReparseType::FilterManager,
        0xA000000C => ReparseType::SymbolicLink,
        0xA0000010 => ReparseType::IisCache,
        0x80000012 => ReparseType::DistributedFileSystemReplication,
        0x80000013 => ReparseType::Dedup,
        0xC0000014 => ReparseType::Appxstrm,
        0x80000014 => ReparseType::NetworkFileSystem,
        0x80000015 => ReparseType::FilePlaceholder,
        0x80000016 => ReparseType::DynamicFilter,
        0x80000017 => ReparseType::Wof,
        0x80000018 => ReparseType::WindowsContainerIsolation,
        0x90001018 => ReparseType::WindowsContainerIsolation1,
        0xA0000019 => ReparseType::GlobalReparse,
        0x9000001A => ReparseType::Cloud,
        0x9000101A => ReparseType::Cloud1,
        0x9000201A => ReparseType::Cloud2,
        0x9000301A => ReparseType::Cloud3,
        0x9000401A => ReparseType::Cloud4,
        0x9000501A => ReparseType::Cloud5,
        0x9000601A => ReparseType::Cloud6,
        0x9000701A => ReparseType::Cloud7,
        0x9000801A => ReparseType::Cloud8,
        0x9000901A => ReparseType::Cloud9,
        0x9000A01A => ReparseType::CloudA,
        0x9000B01A => ReparseType::CloudB,
        0x9000C01A => ReparseType::CloudC,
        0x9000D01A => ReparseType::CloudD,
        0x9000E01A => ReparseType::CloudE,
        0x9000F01A => ReparseType::CloudF,
        0x8000001B => ReparseType::AppExecLink,
        0x9000001C => ReparseType::ProjectedFileSystem,
        0xA000001D => ReparseType::LinuxSymbolicLink,
        0x8000001E => ReparseType::StorageSync,
        0x90000027 => ReparseType::StorageSyncFolder,
        0xA000001F => ReparseType::WindowsContainerTombstone,
        0x80000020 => ReparseType::Unhandled,
        0x80000021 => ReparseType::Onedrive,
        0xA0000022 => ReparseType::ProjectFileSystemTombstone,
        0x80000023 => ReparseType::AfUnix,
        0x80000024 => ReparseType::LinuxFifo,
        0x80000025 => ReparseType::LinuxChar,
        0x80000026 => ReparseType::LinuxBlock,
        0xA0000027 => ReparseType::LinuxLink,
        0xA0001027 => ReparseType::LinuxLink1,
        _ => ReparseType::None,
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::filesystem::ntfs::attributes::reparse_type;
    use common::files::ReparseType;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_read_usnjrnl() {
        use crate::accessor::filesystem::ntfs::attributes::read_named_data;
        use crate::accessor::filesystem::ntfs::{volume::NtfsVolume, walk::resolve_file};

        let volume = NtfsVolume::open_live_drive('c').unwrap();
        let bytes = volume
            .with_reader(|ntfs, reader| {
                let file = resolve_file(ntfs, reader, "$Extend\\$UsnJrnl").unwrap();
                read_named_data(reader, &file, "$J")
            })
            .unwrap();

        // The UsnJrnl "should" be ~30 MB in size
        assert!(bytes.len() > 1024 * 1024 * 10);

        // The UsnJrnl has sparse data that is often ~10GB in size
        // We should be skipping sparse data
        assert!(bytes.len() < 1024 * 1024 * 1024);
    }

    #[test]
    fn test_reparse_type() {
        let test = [
            0x00000000, 0x00000001, 0x00000002, 0xA0000003, 0xC0000004, 0x80000005, 0x80000006,
            0x80000007, 0x80000008, 0x80000009, 0x8000000A, 0x8000000B, 0xA000000C, 0xA0000010,
            0x80000012, 0x80000013, 0xC0000014, 0x80000014, 0x80000015, 0x80000016, 0x80000017,
            0x80000018, 0x90001018, 0xA0000019, 0x9000001A, 0x9000101A, 0x9000201A, 0x9000301A,
            0x9000401A, 0x9000501A, 0x9000601A, 0x9000701A, 0x9000801A, 0x9000901A, 0x9000A01A,
            0x9000B01A, 0x9000C01A, 0x9000D01A, 0x9000E01A, 0x9000F01A, 0x8000001B, 0x9000001C,
            0xA000001D, 0x8000001E, 0x90000027, 0xA000001F, 0x80000020, 0x80000021, 0xA0000022,
            0x80000023, 0x80000024, 0x80000025, 0x80000026, 0xA0000027, 0xA0001027,
        ];
        for entry in test {
            assert_ne!(reparse_type(entry), ReparseType::None);
        }
        assert_eq!(reparse_type(0xff), ReparseType::None);
    }
}
