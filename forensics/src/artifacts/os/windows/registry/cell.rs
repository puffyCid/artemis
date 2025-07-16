use super::{
    keys::{nk::NameKey, vk::ValueKey},
    lists::{lh::HashLeaf, li::LeafItem},
};
use crate::{
    artifacts::os::windows::registry::{
        error::RegistryError, hbin::HiveBin, header::RegHeader, parser::ParamsReader,
    },
    filesystem::ntfs::reader::read_bytes,
    utils::nom_helper::{Endian, nom_data, nom_signed_four_bytes},
};
use common::windows::KeyValue;
use log::{error, warn};
use nom::{
    Needed, Parser, bytes::complete::take, combinator::peek, error::ErrorKind,
    number::complete::le_u16,
};
use ntfs::NtfsFile;
use std::{collections::HashSet, io::BufReader, mem::size_of};

#[derive(Debug, PartialEq)]
pub(crate) enum CellType {
    Nk,
    Vk,
    Sk,
    Lf,
    Lh,
    Li,
    Ri,
    Db,
    Unknown,
}

/// Check for a cell type from provided bytes
pub(crate) fn get_cell_type(data: &[u8]) -> nom::IResult<&[u8], CellType> {
    // Take a peek at the cell data to determine the type, but do not nom the data
    let (cell_data, cell_type_data) = peek(take(size_of::<u16>())).parse(data)?;
    let (_, cell_type) = le_u16(cell_type_data)?;

    let cell = match cell_type {
        0x6b6e => CellType::Nk,
        0x686c => CellType::Lh,
        0x6b73 => CellType::Sk,
        0x6b76 => CellType::Vk,
        0x6972 => CellType::Ri,
        0x666c => CellType::Lf,
        0x6264 => CellType::Db,
        0x696c => CellType::Li,
        _ => {
            error!("[registry] Unknown cell: {cell_type}");
            CellType::Unknown
        }
    };

    Ok((cell_data, cell))
}

/// Check if a cell is allocated. Negative number = allocated, postive number = unallocated
pub(crate) fn is_allocated(data: &[u8]) -> nom::IResult<&[u8], (bool, u32)> {
    let (list_data, mut list_size) = nom_signed_four_bytes(data, Endian::Le)?;
    let cell_allocated = 0;

    // If the size is a postive number then the cell is unallocated (deleted)
    // We currently do not parse deleted cells
    if list_size >= cell_allocated {
        return Ok((list_data, (false, 0)));
    }
    // Allocated cells have a negative cell size
    let size_inverse = -1;
    list_size *= size_inverse;
    Ok((list_data, (true, list_size as u32)))
}

impl<T: std::io::Seek + std::io::Read> ParamsReader<T> {
    /// Parse Registry header info
    pub(crate) fn get_header(
        &mut self,
        use_ntfs: Option<&NtfsFile<'_>>,
    ) -> Result<RegHeader, RegistryError> {
        let header = RegHeader::read_header(&mut self.reader, use_ntfs)?;
        self.minor_version = header.minor_version;
        Ok(header)
    }

    /// Get the ROOT key
    pub(crate) fn root_key(
        &mut self,
        use_ntfs: Option<&NtfsFile<'_>>,
    ) -> Result<NameKey, RegistryError> {
        let hbin = HiveBin::read_hive_bin(&mut self.reader, use_ntfs)?;
        self.size = hbin.size;
        // Skip hbin header
        let root = 32;
        NameKey::read_name_key(&mut self.reader, use_ntfs, root, self.size)
    }

    /// List child Registry Keys associated with `ParamsReader.offset`
    pub(crate) fn list_keys(
        &mut self,
        use_ntfs: Option<&NtfsFile<'_>>,
    ) -> Result<Vec<NameKey>, RegistryError> {
        let mut offset_tracker = HashSet::new();
        let mut names = Vec::new();
        walk_registry_list(
            &mut self.reader,
            use_ntfs,
            self.minor_version,
            &mut offset_tracker,
            self.offset,
            self.size,
            &mut names,
        )?;
        Ok(names)
    }

    /// List all Registry Key values associated with `ParamsReader.offset`
    pub(crate) fn list_values(
        &mut self,
        use_ntfs: Option<&NtfsFile<'_>>,
        number_values: u32,
    ) -> Result<Vec<KeyValue>, RegistryError> {
        walk_registry_values(
            &mut self.reader,
            use_ntfs,
            self.minor_version,
            self.offset,
            self.size,
            number_values,
        )
    }
}

/// Walk the Registry Value list
fn walk_registry_values<T: std::io::Seek + std::io::Read>(
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'_>>,
    minor_version: u32,
    offset: u32,
    size: u32,
    number_values: u32,
) -> Result<Vec<KeyValue>, RegistryError> {
    let real_offset = offset + size;
    let mut value_data = match read_bytes(real_offset as u64, size as u64, ntfs_file, reader) {
        Ok(result) => result,
        Err(err) => {
            error!("[registry] Could not read value bytes: {err:?}");
            return Err(RegistryError::ReadRegistry);
        }
    };

    // Get the size of the list and check if its allocated (negative numbers = allocated, postive number = unallocated)
    let (list_data, (allocated, value_size)) = match is_allocated(&value_data) {
        Ok(result) => result,
        Err(_err) => {
            error!("[registry] Could not determine allocation for value bytes");
            return Err(RegistryError::Parser);
        }
    };
    if !allocated {
        return Ok(Vec::new());
    }

    if value_size > list_data.len() as u32 {
        let large_value_data =
            match read_bytes(real_offset as u64, value_size as u64, ntfs_file, reader) {
                Ok(result) => result,
                Err(err) => {
                    error!("[registry] Could not read larger value bytes: {err:?}");
                    return Err(RegistryError::ReadRegistry);
                }
            };

        value_data = large_value_data;
    }

    let values = match parse_values(
        reader,
        ntfs_file,
        minor_version,
        &value_data,
        number_values,
        size,
    ) {
        Ok((_, result)) => result,
        Err(_err) => {
            error!("[registry] Could not parse value bytes");
            return Err(RegistryError::Parser);
        }
    };

    Ok(values)
}

/// Parse Registry values
fn parse_values<'a, T: std::io::Seek + std::io::Read>(
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'_>>,
    minor_version: u32,
    reg_data: &'a [u8],
    number_values: u32,
    hbin_size: u32,
) -> nom::IResult<&'a [u8], Vec<KeyValue>> {
    // Get the size of the list and check if its allocated (negative numbers = allocated, postive number = unallocated)
    let (list_data, (allocated, size)) = is_allocated(reg_data)?;
    if !allocated {
        return Ok((reg_data, Vec::new()));
    }

    // Size includes the size itself. We nommed that away above in `is_allocated`
    let adjust_cell_size = 4;
    if size < adjust_cell_size {
        return Err(nom::Err::Incomplete(Needed::Unknown));
    }

    // Grab all data associated with the list based on list size
    let (_, mut list_data) = take(size - adjust_cell_size)(list_data)?;

    let mut value_count = 0;
    let mut key_values: Vec<KeyValue> = Vec::new();
    // Go through each value offset in the list
    while value_count < number_values && !list_data.is_empty() {
        // Get the value key offset
        let (input, vk_offset) = nom_signed_four_bytes(list_data, Endian::Le)?;
        list_data = input;

        let empty_offset = 0;
        let unallocated = -1;
        if vk_offset == empty_offset || vk_offset == unallocated {
            value_count += 1;
            continue;
        }
        // Go to the value key offset
        let vk_data = match read_bytes(
            (vk_offset as u32 + hbin_size) as u64,
            hbin_size as u64,
            ntfs_file,
            reader,
        ) {
            Ok(result) => result,
            Err(err) => {
                error!("[registry] Failed to read value key bytes: {err:?}");
                return Err(nom::Err::Failure(nom::error::Error::new(
                    &[],
                    ErrorKind::Fail,
                )));
            }
        };

        // Get the size of the valeu key and check if its allocated (negative numbers = allocated, postive number = unallocated)
        let (vk_data, (allocated, size)) = match is_allocated(&vk_data) {
            Ok(result) => result,
            Err(_err) => {
                error!("[registry] Failed to determine if value key is allocated");
                return Err(nom::Err::Failure(nom::error::Error::new(
                    &[],
                    ErrorKind::Fail,
                )));
            }
        };
        if !allocated {
            value_count += 1;
            continue;
        }

        // Size includes the size itself. We nommed that away
        let adjust_cell_size = 4;
        if size < adjust_cell_size {
            return Err(nom::Err::Incomplete(Needed::Unknown));
        }
        let (_, vk_data) = match nom_data(vk_data, (size - adjust_cell_size) as u64) {
            Ok(result) => result,
            Err(_err) => {
                error!("[registry] Failed to get value key bytes");
                return Err(nom::Err::Failure(nom::error::Error::new(
                    &[],
                    ErrorKind::Fail,
                )));
            }
        };
        // Check for the value key signature (vk)
        let (vk_data, cell_type) = match get_cell_type(vk_data) {
            Ok(result) => result,
            Err(_err) => {
                error!("[registry] Failed to determine if value key cell type");
                return Err(nom::Err::Failure(nom::error::Error::new(
                    &[],
                    ErrorKind::Fail,
                )));
            }
        };
        if cell_type != CellType::Vk {
            warn!("[registry] Got non Vk cell type while iterating value list: {cell_type:?}");
            value_count += 1;
            continue;
        }
        // Parse the Value key data
        //let (_, value_key) = ValueKey::parse_value_key(reg_data, vk_data, minor_version)?;
        let (_, value_key) = match ValueKey::value_key_reader(
            reader,
            ntfs_file,
            vk_data,
            minor_version,
            hbin_size,
        ) {
            Ok(result) => result,
            Err(_err) => {
                error!("[registry] Failed to parse if value key data");
                return Err(nom::Err::Failure(nom::error::Error::new(
                    &[],
                    ErrorKind::Fail,
                )));
            }
        };
        let value = KeyValue {
            value: value_key.value_name,
            data: value_key.data,
            data_type: value_key.data_type,
        };
        key_values.push(value);
        value_count += 1;
    }

    Ok((reg_data, key_values))
}

/// Iterate through the Registry data based on provided offset
pub(crate) fn walk_registry_list<T: std::io::Seek + std::io::Read>(
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'_>>,
    minor_version: u32,
    offset_tracker: &mut HashSet<u32>,
    offset: u32,
    size: u32,
    names: &mut Vec<NameKey>,
) -> Result<(), RegistryError> {
    // Skip hbin header
    let real_offset = offset + size;
    if let Some(_value) = offset_tracker.get(&real_offset) {
        error!(
            "[registry] Detected duplicate Registry offset: {offset}. This triggers infinite loops, stopping parsing and exiting early."
        );
        return Err(RegistryError::ReadRegistry);
    }
    let mut list_bytes = match read_bytes(real_offset as u64, size as u64, ntfs_file, reader) {
        Ok(result) => result,
        Err(err) => {
            error!("[registry] Could not read key list bytes: {err:?}");
            return Err(RegistryError::ReadRegistry);
        }
    };
    offset_tracker.insert(real_offset);

    // Get the size of the list and check if its allocated (negative numbers = allocated, postive number = unallocated)
    let (list_data, (allocated, list_size)) = match is_allocated(&list_bytes) {
        Ok(result) => result,
        Err(_err) => {
            error!("[registry] Could not determine allocation for list bytes");
            return Err(RegistryError::Parser);
        }
    };
    if !allocated {
        return Ok(());
    }

    if list_size > list_data.len() as u32 {
        let large_list_data =
            match read_bytes(real_offset as u64, list_size as u64, ntfs_file, reader) {
                Ok(result) => result,
                Err(err) => {
                    error!("[registry] Could not read larger list bytes: {err:?}");
                    return Err(RegistryError::ReadRegistry);
                }
            };

        list_bytes = large_list_data;
    }

    if let Err(_err) = parse_list(
        reader,
        ntfs_file,
        minor_version,
        offset_tracker,
        names,
        &list_bytes,
        offset,
        size,
    ) {
        error!("[registry] Failed to completely parse Registry key list at offset: {real_offset}");
    }
    Ok(())
}

/// Parse Registry cell list data
fn parse_list<'a, T: std::io::Seek + std::io::Read>(
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'_>>,
    minor_version: u32,
    offset_tracker: &mut HashSet<u32>,
    names: &mut Vec<NameKey>,
    reg_data: &'a [u8],
    offset: u32,
    hbin_size: u32,
) -> nom::IResult<&'a [u8], ()> {
    // Get the size of the list and check if its allocated (negative numbers = allocated, postive number = unallocated)
    let (list_data, (allocated, size)) = is_allocated(reg_data)?;
    if !allocated {
        return Ok((list_data, ()));
    }
    // Size includes the size itself. We nommed that away
    let adjust_cell_size = 4;
    if size < adjust_cell_size {
        return Err(nom::Err::Incomplete(Needed::Unknown));
    }
    // Grab all data associated with the list based on list size
    let (_, list_data) = take(size - adjust_cell_size)(list_data)?;

    let (list_data, cell_type) = get_cell_type(list_data)?;
    if cell_type == CellType::Lh || cell_type == CellType::Lf {
        HashLeaf::read_hash_leaf(
            reader,
            ntfs_file,
            list_data,
            minor_version,
            offset_tracker,
            hbin_size,
            names,
        )?;
    } else if cell_type == CellType::Nk {
        if let Ok(value) = NameKey::read_name_key(reader, ntfs_file, offset, hbin_size) {
            names.push(value);
        }
    } else if cell_type == CellType::Li || cell_type == CellType::Ri {
        LeafItem::read_leaf_item(
            reader,
            ntfs_file,
            list_data,
            minor_version,
            offset_tracker,
            hbin_size,
            names,
        )?;
    } else {
        error!("[registry] Got unknown cell type: {cell_type:?}.");
        return Err(nom::Err::Failure(nom::error::Error::new(
            reg_data,
            ErrorKind::Fail,
        )));
    }
    offset_tracker.remove(&(offset + hbin_size));

    Ok((reg_data, ()))
}

#[cfg(test)]
mod tests {
    use super::{CellType, get_cell_type, is_allocated};
    use crate::artifacts::os::windows::registry::{
        parser::ParamsReader, reader::setup_registry_reader,
    };
    use std::{collections::HashSet, io::BufReader, path::PathBuf};

    #[test]
    fn test_get_cell_type() {
        let mut test_data = [0x6e, 0x6b];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Nk);

        test_data = [0x6c, 0x68];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Lh);

        test_data = [0x73, 0x6b];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Sk);

        test_data = [0x76, 0x6b];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Vk);

        test_data = [0x72, 0x69];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Ri);

        test_data = [0x6c, 0x66];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Lf);

        test_data = [0x64, 0x62];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Db);

        test_data = [0x6c, 0x69];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Li);

        test_data = [0x66, 0x6a];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Unknown);
    }

    #[test]
    fn test_is_allocated() {
        let test_data = [12, 12, 12, 12];
        let (_, (allocated, size)) = is_allocated(&test_data).unwrap();
        assert_eq!(allocated, false);
        assert_eq!(size, 0);
    }

    #[test]
    fn test_registry_reader() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/NTUSER.DAT");

        let reader = setup_registry_reader(test_location.to_str().unwrap()).unwrap();
        let buf_reader = BufReader::new(reader);

        let mut param_reader = ParamsReader {
            reader: buf_reader,
            offset: 0,
            size: 0,
            minor_version: 0,
            start_path: String::new(),
            path_regex: None,
            filter: false,
            registry_path: String::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashSet::new(),
        };

        let header = param_reader.get_header(None).unwrap();
        assert_eq!(header.filename, "\\??\\C:\\Users\\Default\\NTUSER.DAT");

        let root = param_reader.root_key(None).unwrap();
        assert_eq!(root.key_name, "ROOT");
        assert_eq!(root.key_values_offset, -1);
        assert_eq!(root.subkeys_list_offset, 1712);
        assert_eq!(root.number_key_values, 0);
        assert_eq!(root.number_subkeys, 10);

        if root.subkeys_list_offset < 0 {
            return;
        }

        param_reader.offset = root.subkeys_list_offset as u32;
        let names = param_reader.list_keys(None).unwrap();
        assert_eq!(names.len(), 10);
        assert_eq!(names[3].key_name, "Environment");

        param_reader.offset = names[3].key_values_offset as u32;
        let values = param_reader.list_values(None, 3).unwrap();
        assert_eq!(values.len(), 3);
        assert_eq!(
            values[0].data,
            "%USERPROFILE%\\AppData\\Local\\Microsoft\\WindowsApps;"
        );
        assert_eq!(values[0].value, "Path");
        assert_eq!(values[0].data_type, "REG_EXPAND_SZ");

        assert_eq!(values[1].data, "%USERPROFILE%\\AppData\\Local\\Temp");
        assert_eq!(values[1].value, "TEMP");
        assert_eq!(values[1].data_type, "REG_EXPAND_SZ");

        assert_eq!(values[2].data, "%USERPROFILE%\\AppData\\Local\\Temp");
        assert_eq!(values[2].value, "TMP");
        assert_eq!(values[2].data_type, "REG_EXPAND_SZ");
    }

    fn get_keys<T: std::io::Seek + std::io::Read>(param_reader: &mut ParamsReader<T>) {
        let names = param_reader.list_keys(None).unwrap();

        for name in names {
            if name.key_values_offset != -1 {
                param_reader.offset = name.key_values_offset as u32;
                let values = param_reader
                    .list_values(None, name.number_key_values)
                    .unwrap();
                assert_eq!(values.len() as u32, name.number_key_values);
            }

            if name.subkeys_list_offset == -1 {
                continue;
            }

            param_reader.offset = name.subkeys_list_offset as u32;
            get_keys(param_reader);
        }
    }

    #[test]
    fn test_walk_entire_registry() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/NTUSER.DAT");

        let reader = setup_registry_reader(test_location.to_str().unwrap()).unwrap();
        let buf_reader = BufReader::new(reader);

        let mut param_reader = ParamsReader {
            reader: buf_reader,
            offset: 0,
            size: 0,
            minor_version: 0,
            start_path: String::new(),
            path_regex: None,
            filter: false,
            registry_path: String::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashSet::new(),
        };

        let header = param_reader.get_header(None).unwrap();
        assert_eq!(header.filename, "\\??\\C:\\Users\\Default\\NTUSER.DAT");

        let root = param_reader.root_key(None).unwrap();
        assert_eq!(root.key_name, "ROOT");
        assert_eq!(root.key_values_offset, -1);
        assert_eq!(root.subkeys_list_offset, 1712);
        assert_eq!(root.number_key_values, 0);
        assert_eq!(root.number_subkeys, 10);

        if root.subkeys_list_offset < 0 {
            return;
        }

        param_reader.offset = root.subkeys_list_offset as u32;
        let names = param_reader.list_keys(None).unwrap();
        for name in names {
            if name.subkeys_list_offset == -1 {
                continue;
            }
            param_reader.offset = name.subkeys_list_offset as u32;
            get_keys(&mut param_reader);
        }
    }
}
