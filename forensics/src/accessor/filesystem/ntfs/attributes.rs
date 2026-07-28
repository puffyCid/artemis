use crate::accessor::{
    error::{AccessorError, AccessorResult},
    filesystem::ntfs::walk::ntfs_err,
};
use ntfs::{
    NtfsAttributeType, NtfsFile, NtfsReadSeek, attribute_value::NtfsAttributeValue,
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

#[cfg(test)]
mod tests {
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
}
