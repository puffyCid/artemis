use super::{
    database::{get_data, parse_db},
    header::SdbHeader,
    tag::{generate_tags, get_tag},
};
use crate::artifacts::os::windows::shimdb::{
    indexes::get_indexes_data, stringtable::get_stringtable_data, tags::list::parse_list,
};
use common::windows::{DatabaseData, ShimData};
use log::error;
use std::collections::HashMap;

/// Parse the bytes of a sdb file
pub(crate) fn parse_shimdb(data: &[u8]) -> nom::IResult<&[u8], ShimData> {
    let (mut input, header) = SdbHeader::parse_header(data)?;

    let mut shim_data = ShimData {
        indexes: Vec::new(),
        db_data: DatabaseData {
            sdb_version: String::new(),
            compile_time: String::new(),
            compiler_version: String::new(),
            name: String::new(),
            platform: 0,
            database_id: String::new(),
            additional_metadata: HashMap::new(),
            list_data: Vec::new(),
        },
        sdb_path: String::new(),
    };

    let tag_values = generate_tags();
    let database_tag = 0x7001;
    let stringtable_tag = 0x7801;

    let mut database_data: Vec<u8> = Vec::new();
    let mut stringtable_data: Vec<u8> = Vec::new();
    let mut indexes_data: Vec<u8> = Vec::new();

    // Overview of SDB struct is three (3) "root" lists:
    //   1. INDEXES list entries
    //   2. DATABASE list entries
    //   3. STRINGTABLE list
    while !input.is_empty() {
        let (sdb_data, (_tag, tag_value)) = get_tag(input)?;

        // In order to fully parse the database list we need the stringtable, store data with `database_data` until stringtable data is found
        if tag_value == database_tag {
            let (sdb_data, db_data) = get_data(sdb_data)?;

            input = sdb_data;
            database_data = db_data;
            continue;
        } else if tag_value == stringtable_tag {
            let (_sdb_data, table_data) = get_stringtable_data(sdb_data)?;

            stringtable_data = table_data;
            //stringtable is the last data/list in a sdb file
            break;
        } else {
            // indexes list mainly contains binary data
            let (sdb_data, index_data) = get_indexes_data(sdb_data)?;

            indexes_data = index_data;
            input = sdb_data;
        }
    }
    let index_tag_result = parse_list(&indexes_data, &stringtable_data, &tag_values);
    match index_tag_result {
        Ok((_, mut result)) => shim_data.indexes.append(&mut result),
        Err(err) => {
            error!("[shimdb] Failed to parse indexes list: {err:?}");
        }
    }

    let db_result = parse_db(&database_data, &stringtable_data, &tag_values);
    match db_result {
        Ok((_, mut result)) => {
            result.sdb_version = format!("{}.{}", header.major_version, header.minor_version);
            shim_data.db_data = result;
        }
        Err(err) => {
            error!("[shimdb] Failed to parse database list: {err:?}");
        }
    }

    Ok((data, shim_data))
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::shimdb::shims::parse_shimdb, filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_shimdb() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/shimdb/win10/sysmain.sdb");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) = parse_shimdb(&buffer).unwrap();

        assert_eq!(result.db_data.additional_metadata.len(), 0);
        assert_eq!(result.db_data.compile_time, "2016-01-01T00:00:00.000Z");
        assert_eq!(result.db_data.platform, 6);
        assert_eq!(result.db_data.compiler_version, "3.0.0.9");
        assert_eq!(
            result.db_data.name,
            "Microsoft Windows Application Compatibility Fix Database"
        );
        assert_eq!(result.db_data.sdb_version, "3.0");
        assert_eq!(
            result.db_data.database_id,
            "11111111-1111-1111-1111-111111111111"
        );
        assert_eq!(result.db_data.list_data.len(), 13581);

        assert_eq!(
            result.db_data.list_data[0].list_data[0]
                .get("TAG_MODULE")
                .unwrap(),
            "FWCWSP64.dll"
        );
        assert_eq!(
            result.db_data.list_data[13580]
                .data
                .get("TAG_NAME")
                .unwrap(),
            "TARGETPATH:{7C5A40EF-A0FB-4BFC-874A-C0F2E0B9FA8E}\\Microsoft Office\\Office15\\FIRSTRUN.EXE"
        );
        assert_eq!(
            result.db_data.list_data[13580]
                .data
                .get("TAG_APP_NAME")
                .unwrap(),
            "AUMID ShellLink Color Overrides For Desktop Tiles"
        );
        assert_eq!(
            result.db_data.list_data[13580]
                .data
                .get("TAG_VENDOR")
                .unwrap(),
            "Microsoft"
        );

        assert_eq!(
            result.db_data.list_data[13580].list_data[0]
                .get("TAG_DATA_DWORD")
                .unwrap(),
            "4473924"
        );
        assert_eq!(
            result.db_data.list_data[13580].list_data[0]
                .get("TAG_DATA_VALUETYPE")
                .unwrap(),
            "4"
        );
        assert_eq!(
            result.db_data.list_data[13580].list_data[0]
                .get("TAG_NAME")
                .unwrap(),
            "BackgroundColor"
        );
        assert_eq!(result.indexes.len(), 1);
    }

    #[test]
    fn test_parse_custom_shimdb() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/shimdb/AtomicShimx86.sdb");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) = parse_shimdb(&buffer).unwrap();
        assert_eq!(result.indexes.len(), 1);
        assert_eq!(result.db_data.compile_time, "2017-12-06T21:15:08.000Z");
        assert_eq!(result.db_data.sdb_version, "2.1");
        assert_eq!(result.db_data.compiler_version, "2.1.0.3");
        assert_eq!(result.db_data.name, "AtomicShim");
        assert_eq!(
            result.db_data.list_data[0]
                .data
                .get("TAG_APP_NAME")
                .unwrap(),
            "AtomicTest"
        );
        assert_eq!(
            result.db_data.list_data[0].data.get("TAG_NAME").unwrap(),
            "AtomicTest.exe"
        );
        assert_eq!(
            result.db_data.list_data[0].list_data[1]
                .get("TAG_COMMAND_LINE")
                .unwrap(),
            "C:\\Tools\\AtomicTest.dll"
        );
    }

    #[test]
    fn test_parse_custom_shimdb2() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/shimdb/T1546.011CompatDatabase.sdb");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) = parse_shimdb(&buffer).unwrap();
        assert_eq!(result.indexes.len(), 1);
        assert_eq!(result.db_data.compile_time, "2016-01-01T00:00:00.000Z");
        assert_eq!(result.db_data.sdb_version, "2.3");
        assert_eq!(result.db_data.compiler_version, "3.0.0.9");
        assert_eq!(result.db_data.name, "T1138CompatDatabase");
        assert_eq!(
            result.db_data.list_data[0]
                .data
                .get("TAG_APP_NAME")
                .unwrap(),
            "T1138"
        );
        assert_eq!(
            result.db_data.list_data[0].data.get("TAG_NAME").unwrap(),
            "calc.exe"
        );
        assert_eq!(
            result.db_data.list_data[0].list_data[0]
                .get("TAG_FILE_VERSION")
                .unwrap(),
            "10.0.18362.1 (WinBuild.160101.0800)"
        );
    }
}
