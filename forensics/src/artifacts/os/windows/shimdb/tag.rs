use crate::utils::nom_helper::{Endian, nom_unsigned_two_bytes};
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub(crate) enum Tags {
    String,
    Binary,
    List,
    Stringref,
    Qword,
    Dword,
    Null,
    Word,
    Unkonwn,
}

/// Shimdb is built on tags and data associated with tags. Read tag size (2 bytes) and return the tag type and tag id (Ex: LIST, 0x7801 (`TAG_STRINGTABLE`))
pub(crate) fn get_tag(data: &[u8]) -> nom::IResult<&[u8], (Tags, u16)> {
    let (input, tag) = nom_unsigned_two_bytes(data, Endian::Le)?;

    let min_null_value = 0x1001;
    let max_null_value = 0x1016;

    let min_word_value = 0x3001;
    let max_word_value = 0x3803;

    let min_dword_value = 0x4001;
    let max_dword_value = 0x4801;

    let min_qword_value = 0x5001;
    let max_qword_value = 0x501f;

    let min_stringref_value = 0x6001;
    let max_stringref_value = 0x603b;

    let min_list_value = 0x7001;
    let max_list_value = 0x7803;

    let string_tag = 0x8801;

    let min_binary_value = 0x9002;
    let max_binary_value = 0x9801;

    let mut result = (data, (Tags::Unkonwn, 0));

    if tag == string_tag {
        result = (input, (Tags::String, tag));
    } else if tag >= min_null_value && tag <= max_null_value {
        result = (input, (Tags::Null, tag));
    } else if tag >= min_word_value && tag <= max_word_value {
        result = (input, (Tags::Word, tag));
    } else if tag >= min_dword_value && tag <= max_dword_value {
        result = (input, (Tags::Dword, tag));
    } else if tag >= min_qword_value && tag <= max_qword_value {
        result = (input, (Tags::Qword, tag));
    } else if tag >= min_stringref_value && tag <= max_stringref_value {
        result = (input, (Tags::Stringref, tag));
    } else if tag >= min_list_value && tag <= max_list_value {
        result = (input, (Tags::List, tag));
    } else if tag >= min_binary_value && tag <= max_binary_value {
        result = (input, (Tags::Binary, tag));
    }
    Ok(result)
}

/// Create `HashMap` of known tags for fast loopkups
pub(crate) fn generate_tags() -> HashMap<u16, String> {
    let mut tags: HashMap<u16, String> = HashMap::new();

    // From https://www.geoffchappell.com/studies/windows/win32/apphelp/sdb/tag.htm?tx=46
    // NULL tags -- boolean values
    tags.insert(0x1001, String::from("TAG_INCLUDE"));
    tags.insert(0x1002, String::from("TAG_GENERAL"));
    tags.insert(0x1003, String::from("TAG_MATCH_LOGIC_NOT"));
    tags.insert(0x1004, String::from("TAG_APPLY_ALL_SHIMS"));
    tags.insert(0x1005, String::from("TAG_USE_SERVICE_PACK_FILES"));
    tags.insert(0x1006, String::from("TAG_MITIGATION_OS"));

    // Undocumented
    tags.insert(0x1007, String::from("TAG_BLOCK_UPGRADE_TRACE_PCA"));

    tags.insert(0x1008, String::from("TAG_INCLUDEEXCLUDEDLL"));

    // Rest are all undocumented
    tags.insert(0x1009, String::from("TAG_RAC_EVENT_OFF"));
    tags.insert(0x100A, String::from("TAG_TELEMETRY_OFF"));
    tags.insert(0x100B, String::from("TAG_SHIM_ENGINE_OFF"));
    tags.insert(0x100C, String::from("TAG_LAYER_PROPAGATION_OFF"));
    tags.insert(0x100D, String::from("TAG_REINSTALL_UPGRADE_FORCE_CACHE"));
    tags.insert(0x100E, String::from("TAG_MONITORING_OFF"));
    tags.insert(0x100F, String::from("TAG_QUIRK_OFF"));
    tags.insert(0x1010, String::from("TAG_ELEVATED_PROP_OFF"));
    tags.insert(0x1011, String::from("TAG_UPGRADE_ACTION_BLOCK_WEBSETUP"));
    tags.insert(
        0x1012,
        String::from("TAG_UPGRADE_ACTION_PROCEED_TO_MEDIASETUP"),
    );

    // WORD tags -- 2 bytes of data
    tags.insert(0x3001, String::from("TAG_MATCH_MODE"));
    // Undocumented
    tags.insert(0x3002, String::from("TAG_QUIRK_COMPONENT_CODE_ID"));
    tags.insert(0x3003, String::from("TAG_QUIRK_CODE_ID"));

    tags.insert(0x3801, String::from("TAG_TAG"));
    tags.insert(0x3802, String::from("TAG_INDEX_TAG"));
    tags.insert(0x3803, String::from("TAG_INDEX_KEY"));

    // DWORD tags -- 4 bytes of data
    tags.insert(0x4001, String::from("TAG_SIZE"));
    tags.insert(0x4002, String::from("TAG_OFFSET"));
    tags.insert(0x4003, String::from("TAG_CHECKSUM"));
    tags.insert(0x4004, String::from("TAG_SHIM_TAGID"));
    tags.insert(0x4005, String::from("TAG_PATCH_TAGID"));
    tags.insert(0x4006, String::from("TAG_MODULE_TYPE"));
    tags.insert(0x4007, String::from("TAG_VERDATEHI"));
    tags.insert(0x4008, String::from("TAG_VERDATELO"));
    tags.insert(0x4009, String::from("TAG_VERFILEOS"));
    tags.insert(0x400A, String::from("TAG_VERFILETYPE"));
    tags.insert(0x400B, String::from("TAG_PE_CHECKSUM"));
    tags.insert(0x400C, String::from("TAG_PREVOSMAJORVER"));
    tags.insert(0x400D, String::from("TAG_PREVOSMINORVER"));
    tags.insert(0x400E, String::from("TAG_PREVOSPLATFORMID"));
    tags.insert(0x400F, String::from("TAG_PREVOSBUILDNO"));
    tags.insert(0x4010, String::from("TAG_PROBLEMSEVERITY"));
    tags.insert(0x4011, String::from("TAG_LANGID"));
    tags.insert(0x4012, String::from("TAG_VER_LANGUAGE"));
    tags.insert(0x4014, String::from("TAG_ENGINE"));
    tags.insert(0x4015, String::from("TAG_HTMLHELPID"));
    tags.insert(0x4016, String::from("TAG_INDEX_FLAGS"));
    tags.insert(0x4017, String::from("TAG_FLAGS"));
    tags.insert(0x4018, String::from("TAG_DATA_VALUETYPE"));
    tags.insert(0x4019, String::from("TAG_DATA_DWORD"));
    tags.insert(0x401A, String::from("TAG_LAYER_TAGID"));
    tags.insert(0x401B, String::from("TAG_MSI_TRANSFORM_TAGID"));
    tags.insert(0x401C, String::from("TAG_LINKER_VERSION"));
    tags.insert(0x401D, String::from("TAG_LINK_DATE"));
    tags.insert(0x401E, String::from("TAG_UPTO_LINK_DATE"));
    tags.insert(0x401F, String::from("TAG_OS_SERVICE_PACK"));
    tags.insert(0x4020, String::from("TAG_FLAG_TAGID"));
    tags.insert(0x4021, String::from("TAG_RUNTIME_PLATFORM"));
    tags.insert(0x4022, String::from("TAG_OS_SKU"));
    tags.insert(0x4023, String::from("TAG_OS_PLATFORM_DEPRECATED"));
    tags.insert(0x4024, String::from("TAG_APP_NAME_RC_ID"));
    tags.insert(0x4025, String::from("TAG_VENDOR_NAME_RC_ID"));
    tags.insert(0x4026, String::from("TAG_SUMMARY_MSG_RC_ID"));
    tags.insert(0x4027, String::from("TAG_VISTA_SKU"));
    tags.insert(0x4028, String::from("TAG_DESCRIPTION_RC_ID"));
    tags.insert(0x4029, String::from("TAG_PARAMETER1_RC_ID"));

    tags.insert(0x4801, String::from("TAG_TAGID"));

    // Rest are all undocumented
    tags.insert(0x4030, String::from("TAG_CONTEXT_TAGID"));
    tags.insert(0x4031, String::from("TAG_EXE_WRAPPER"));
    tags.insert(0x4032, String::from("TAG_URL_ID_EXE_TYPE"));
    tags.insert(0x4033, String::from("TAG_FROM_LINK_DATE"));
    tags.insert(0x4034, String::from("TAG_URL_ID_REVISION_EQ"));
    tags.insert(0x4035, String::from("TAG_REVISION_LE"));
    tags.insert(0x4036, String::from("TAG_REVISION_GE"));
    tags.insert(0x4037, String::from("TAG_DATE_EQ"));
    tags.insert(0x4038, String::from("TAG_DATE_LE"));
    tags.insert(0x4039, String::from("TAG_DATE_GE"));
    tags.insert(0x403A, String::from("TAG_CPU_MODEL_EQ"));
    tags.insert(0x403B, String::from("TAG_CPU_MODEL_LE"));
    tags.insert(0x403C, String::from("TAG_CPU_MODEL_GE"));
    tags.insert(0x403D, String::from("TAG_CPU_FAMILY_EQ"));
    tags.insert(0x403E, String::from("TAG_CPU_FAMILY_LE"));
    tags.insert(0x403F, String::from("TAG_CPU_FAMILY_GE"));
    tags.insert(0x4040, String::from("TAG_CREATOR_REVISION_EQ"));
    tags.insert(0x4041, String::from("TAG_CREATOR_REVISION_LE"));
    tags.insert(0x4042, String::from("TAG_CREATOR_REVISION_GE"));
    tags.insert(0x4043, String::from("TAG_SIZE_OF_IMAGE"));
    tags.insert(0x4044, String::from("TAG_SHIM_CLASS"));
    tags.insert(0x4045, String::from("TAG_PACKAGEID_ARCHITECTURE"));
    tags.insert(0x4046, String::from("TAG_REINSTALL_UPGRADE_TYPE"));
    tags.insert(0x4047, String::from("TAG_BLOCK_UPGRADE_TYPE"));
    tags.insert(0x4048, String::from("TAG_ROUTING_MODE"));
    tags.insert(0x4049, String::from("TAG_OS_VERSION_VALUE"));
    tags.insert(0x404A, String::from("TAG_CRC_CHECKSUM"));
    tags.insert(0x404B, String::from("TAG_URL_ID"));
    tags.insert(0x404C, String::from("TAG_QUIRK_TAGID"));
    tags.insert(0x404E, String::from("TAG_MIGRATION_DATA_TYPE"));
    tags.insert(0x404F, String::from("TAG_UPGRADE_DATA"));
    tags.insert(0x4050, String::from("TAG_MIGRATION_DATA_TAGID"));
    tags.insert(0x4051, String::from("TAG_REG_VALUE_TYPE"));
    tags.insert(0x4052, String::from("TAG_REG_VALUE_DATA_DWORD"));
    tags.insert(0x4053, String::from("TAG_TEXT_ENCODING"));
    tags.insert(0x4054, String::from("TAG_UX_BLOCKTYPE_OVERRIDE"));
    tags.insert(0x4055, String::from("TAG_EDITION"));
    tags.insert(0x4056, String::from("TAG_FW_LINK_ID"));
    tags.insert(0x4057, String::from("TAG_KB_ARTICLE_ID"));
    tags.insert(0x4058, String::from("TAG_UPGRADE_MODE"));

    // QWORD -- 8 bytes of data
    tags.insert(0x5001, String::from("TAG_TIME"));
    tags.insert(0x5002, String::from("TAG_BIN_FILE_VERSION"));
    tags.insert(0x5003, String::from("TAG_BIN_PRODUCT_VERSION"));
    tags.insert(0x5004, String::from("TAG_MODTIME"));
    tags.insert(0x5005, String::from("TAG_FLAG_MASK_KERNEL"));
    tags.insert(0x5006, String::from("TAG_UPTO_BIN_PRODUCT_VERSION"));
    tags.insert(0x5007, String::from("TAG_DATA_QWORD"));
    tags.insert(0x5008, String::from("TAG_FLAG_MASK_USER"));
    tags.insert(0x5009, String::from("TAG_FLAGS_NTVDM1"));
    tags.insert(0x500A, String::from("TAG_FLAGS_NTVDM2"));
    tags.insert(0x500B, String::from("TAG_FLAGS_NTVDM3"));
    tags.insert(0x500C, String::from("TAG_FLAG_MASK_SHELL"));
    tags.insert(0x500D, String::from("TAG_UPTO_BIN_FILE_VERSION"));
    tags.insert(0x500E, String::from("TAG_FLAG_MASK_FUSION"));
    tags.insert(0x500F, String::from("TAG_FLAG_PROCESSPARAM"));
    tags.insert(0x5010, String::from("TAG_FLAG_LUA"));
    tags.insert(0x5011, String::from("TAG_FLAG_INSTALL"));

    // Rest are all undocumented
    tags.insert(0x5012, String::from("TAG_FROM_BIN_PRODUCT_VERSION"));
    tags.insert(0x5013, String::from("TAG_FROM_BIN_FILE_VERSION"));
    tags.insert(0x5014, String::from("TAG_PACKAGEID_VERSION"));
    tags.insert(0x5015, String::from("TAG_FROM_PACKAGEID_VERSION"));
    tags.insert(0x5016, String::from("TAG_UPTO_PACKAGEID_VERSION"));
    tags.insert(0x5017, String::from("TAG_OSMAXVERSIONTESTED"));
    tags.insert(0x5018, String::from("TAG_FROM_OSMAXVERSIONTESTED"));
    tags.insert(0x5019, String::from("TAG_UPTO_OSMAXVERSIONTESTED"));
    tags.insert(0x501A, String::from("TAG_FLAG_MASK_WINRT"));
    tags.insert(0x501B, String::from("TAG_REG_VALUE_DATA_QWORD"));
    tags.insert(
        0x501C,
        String::from("TAG_QUIRK_ENABLED_UPTO_VERSION_VERSION_LT"),
    );
    tags.insert(0x501D, String::from("TAG_SOURCE_OS"));
    tags.insert(0x501E, String::from("TAG_SOURCE_OS_LTE"));
    tags.insert(0x501F, String::from("TAG_SOURCE_OS_GTE"));

    // String ref -- 4 bytes of data that represent the offset of the string in bytes. Offset is from the start of the string table (0x7801)
    tags.insert(0x6001, String::from("TAG_NAME"));
    tags.insert(0x6002, String::from("TAG_DESCRIPTION"));
    tags.insert(0x6003, String::from("TAG_MODULE"));
    tags.insert(0x6004, String::from("TAG_API"));
    tags.insert(0x6005, String::from("TAG_VENDOR"));
    tags.insert(0x6006, String::from("TAG_APP_NAME"));
    tags.insert(0x6008, String::from("TAG_COMMAND_LINE"));
    tags.insert(0x6009, String::from("TAG_COMPANY_NAME"));
    tags.insert(0x600A, String::from("TAG_DLLFILE"));
    tags.insert(0x600B, String::from("TAG_WILDCARD_NAME"));
    tags.insert(0x6010, String::from("TAG_PRODUCT_NAME"));
    tags.insert(0x6011, String::from("TAG_PRODUCT_VERSION"));
    tags.insert(0x6012, String::from("TAG_FILE_DESCRIPTION"));
    tags.insert(0x6013, String::from("TAG_FILE_VERSION"));
    tags.insert(0x6014, String::from("TAG_ORIGINAL_FILENAME"));
    tags.insert(0x6015, String::from("TAG_INTERNAL_NAME"));
    tags.insert(0x6016, String::from("TAG_LEGAL_COPYRIGHT"));
    tags.insert(0x6017, String::from("TAG_16BIT_DESCRIPTION"));
    tags.insert(0x6018, String::from("TAG_APPHELP_DETAILS"));
    tags.insert(0x6019, String::from("TAG_LINK_URL"));
    tags.insert(0x601A, String::from("TAG_LINK_TEXT"));
    tags.insert(0x601B, String::from("TAG_APPHELP_TITLE"));
    tags.insert(0x601C, String::from("TAG_APPHELP_CONTACT"));
    tags.insert(0x601D, String::from("TAG_SXS_MANIFEST"));
    tags.insert(0x601E, String::from("TAG_DATA_STRING"));
    tags.insert(0x601F, String::from("TAG_MSI_TRANSFORM_FILE"));
    tags.insert(0x6020, String::from("TAG_16BIT_MODULE_NAME"));
    tags.insert(0x6021, String::from("TAG_LAYER_DISPLAYNAME"));
    tags.insert(0x6022, String::from("TAG_COMPILER_VERSION"));
    tags.insert(0x6023, String::from("TAG_ACTION_TYPE"));
    tags.insert(0x6024, String::from("TAG_EXPORT_NAME"));

    // Rest are all undocumented
    tags.insert(0x6025, String::from("TAG_URL_VENDOR_ID"));
    tags.insert(0x6026, String::from("TAG_DEVICE_ID"));
    tags.insert(0x6027, String::from("TAG_SUB_VENDOR_ID"));
    tags.insert(0x6028, String::from("TAG_SUB_SYSTEM_ID"));
    tags.insert(0x6029, String::from("TAG_PACKAGEID_NAME"));
    tags.insert(0x602A, String::from("TAG_PACKAGEID_PUBLISHER"));
    tags.insert(0x602B, String::from("TAG_PACKAGEID_LANGUAGE"));
    tags.insert(0x602C, String::from("TAG_URL"));
    tags.insert(0x602D, String::from("TAG_MANUFACTURER"));
    tags.insert(0x602E, String::from("TAG_MODEL"));
    tags.insert(0x602F, String::from("TAG_DATE"));
    tags.insert(0x6030, String::from("TAG_REG_VALUE_NAME"));
    tags.insert(0x6031, String::from("TAG_REG_VALUE_DATA_SZ"));
    tags.insert(0x6032, String::from("TAG_MIGRATION_DATA_TEXT"));
    tags.insert(0x6033, String::from("TAG_APP_STORE_PRODUCT_ID"));
    tags.insert(0x6034, String::from("TAG_MORE_INFO_URL"));

    // Seen in sysmain.sdb on Windows 11 ARM
    // Values: TH1, RS1, RS2, RS4, RS5.
    // Per Copilot these are likely Windows release codenames for Windows 10
    // TH1 = Threshold 1, RS1 = Redstone 1
    // https://en.wikipedia.org/wiki/List_of_Microsoft_codenames
    tags.insert(0x6036, String::from("TAG_OS_CODENAME"));

    // List tags -- 4 bytes that contain the size of list. List contains child tags
    tags.insert(0x7001, String::from("TAG_DATABASE"));
    tags.insert(0x7002, String::from("TAG_LIBRARY"));
    tags.insert(0x7003, String::from("TAG_INEXCLUDE"));
    tags.insert(0x7004, String::from("TAG_SHIM"));
    tags.insert(0x7005, String::from("TAG_PATCH"));
    tags.insert(0x7006, String::from("TAG_APP"));
    tags.insert(0x7007, String::from("TAG_EXE"));
    tags.insert(0x7008, String::from("TAG_MATCHING_FILE"));
    tags.insert(0x7009, String::from("TAG_SHIM_REF"));
    tags.insert(0x700A, String::from("TAG_PATCH_REF"));
    tags.insert(0x700B, String::from("TAG_LAYER"));
    tags.insert(0x700C, String::from("TAG_FILE"));
    tags.insert(0x700D, String::from("TAG_APPHELP"));
    tags.insert(0x700E, String::from("TAG_LINK"));
    tags.insert(0x700F, String::from("TAG_DATA"));
    tags.insert(0x7010, String::from("TAG_MSI_TRANSFORM"));
    tags.insert(0x7011, String::from("TAG_MSI_TRANSFORM_REF"));
    tags.insert(0x7012, String::from("TAG_MSI_PACKAGE"));
    tags.insert(0x7013, String::from("TAG_FLAG"));
    tags.insert(0x7014, String::from("TAG_MSI_CUSTOM_ACTION"));
    tags.insert(0x7015, String::from("TAG_FLAG_REF"));
    tags.insert(0x7016, String::from("TAG_ACTION"));
    tags.insert(0x7017, String::from("TAG_LOOKUP"));

    tags.insert(0x7801, String::from("TAG_STRINGTABLE"));
    tags.insert(0x7802, String::from("TAG_INDEXES"));
    tags.insert(0x7803, String::from("TAG_INDEX"));

    // Rest are all undocumented
    tags.insert(0x7018, String::from("TAG_CONTEXT"));
    tags.insert(0x7019, String::from("TAG_CONTEXT_REF"));
    tags.insert(0x701A, String::from("TAG_KDEVICE"));
    tags.insert(0x701C, String::from("TAG_KDRIVER"));
    tags.insert(0x701E, String::from("TAG_DEVICE_MATCHING_DEVICE"));
    tags.insert(0x701F, String::from("TAG_ACPI"));
    tags.insert(0x7020, String::from("TAG_SPC_BIOS"));
    tags.insert(0x7021, String::from("TAG_CPU"));
    tags.insert(0x7022, String::from("TAG_OEM"));
    tags.insert(0x7023, String::from("TAG_KFLAG"));
    tags.insert(0x7024, String::from("TAG_KFLAG_REF"));
    tags.insert(0x7025, String::from("TAG_KSHIM"));
    tags.insert(0x7026, String::from("TAG_KSHIM_REF"));
    tags.insert(0x7027, String::from("TAG_REINSTALL_UPGRADE"));
    tags.insert(0x7028, String::from("TAG_KDATA"));
    tags.insert(0x7029, String::from("TAG_BLOCK_UPGRADE"));
    tags.insert(0x702A, String::from("TAG_SPC"));
    tags.insert(0x702B, String::from("TAG_QUIRK"));
    tags.insert(0x702C, String::from("TAG_QUIRK_REF"));
    tags.insert(0x702D, String::from("TAG_BIOS_BLOCK"));
    tags.insert(0x702E, String::from("TAG_MATCHING_INFO_BLOCK"));
    tags.insert(0x702F, String::from("TAG_DEVICE_BLOCK"));
    tags.insert(0x7030, String::from("TAG_MIGRATION_DATA"));
    tags.insert(0x7031, String::from("TAG_MIGRATION_DATA_REF"));
    tags.insert(0x7032, String::from("TAG_MATCHING_REG"));
    tags.insert(0x7033, String::from("TAG_MATCHING_TEXT"));
    tags.insert(0x7034, String::from("TAG_MACHINE_BLOCK"));
    tags.insert(0x7035, String::from("TAG_OS_UPGRADE"));
    tags.insert(0x7036, String::from("TAG_PACKAGE"));
    tags.insert(0x7037, String::from("TAG_PICK_ONE"));
    tags.insert(0x7038, String::from("TAG_MATCH_PLUGIN"));
    tags.insert(0x7039, String::from("TAG_MIGRATION_SHIM"));
    tags.insert(0x703A, String::from("TAG_UPGRADE_DRIVER_BLOCK"));
    tags.insert(0x703C, String::from("TAG_MIGRATION_SHIM_REF"));
    tags.insert(0x703D, String::from("TAG_CONTAINS_FILE"));
    tags.insert(0x703E, String::from("TAG_CONTAINS_HWID"));
    tags.insert(0x703F, String::from("TAG_DRIVER_PACKAGE_BLOCK"));

    // Strings -- followed by 4 byte string size includes null terminated string
    tags.insert(0x8801, String::from("TAG_STRINGTABLE_ITEM"));

    // Binary -- followed by 4 byte size and 1 byte padding (Shimdb v2+)
    tags.insert(0x9002, String::from("TAG_PATCH_BITS"));
    tags.insert(0x9003, String::from("TAG_FILE_BITS"));
    tags.insert(0x9004, String::from("TAG_EXE_ID"));
    tags.insert(0x9005, String::from("TAG_DATA_BITS"));
    tags.insert(0x9006, String::from("TAG_MSI_PACKAGE_ID"));
    tags.insert(0x9007, String::from("TAG_DATABASE_ID"));
    tags.insert(0x9801, String::from("TAG_INDEX_BITS"));

    // Rest are undocumted
    tags.insert(0x9008, String::from("TAG_CONTEXT_PLATFORM_ID"));
    tags.insert(0x9009, String::from("TAG_CONTEXT_BRANCH_ID"));
    tags.insert(0x9010, String::from("TAG_FIX_ID"));
    tags.insert(0x9011, String::from("TAG_APP_ID"));
    tags.insert(0x9012, String::from("TAG_REG_VALUE_DATA_BINARY"));
    tags.insert(0x9013, String::from("TAG_TEXT"));

    tags
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::shimdb::tag::{Tags, generate_tags, get_tag};

    #[test]
    fn test_generate_tags() {
        let result = generate_tags();
        assert_eq!(result.len(), 261)
    }

    #[test]
    fn test_get_tag() {
        let test_data = [
            2, 120, 14, 88, 2, 0, 3, 120, 196, 39, 1, 0, 2, 56, 7, 112, 3, 56, 1, 96, 22, 64, 1, 0,
            0, 0, 1, 152, 176, 39, 1, 0, 69, 88, 69, 46, 69, 84, 71, 33, 42, 1, 7, 0, 69, 46, 49,
            69, 82, 83, 73, 33, 168, 1, 7, 0, 69, 46, 50, 69, 82, 83, 73, 33, 38, 2, 7, 0, 69, 46,
            65, 69, 82, 83, 73, 33, 164, 2, 7, 0, 69, 46, 66, 69,
        ];
        let (_, (tag, value)) = get_tag(&test_data).unwrap();
        assert_eq!(tag, Tags::List);
        assert_eq!(value, 0x7802)
    }
}
