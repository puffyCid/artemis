use crate::{
    artifacts::os::windows::securitydescriptor::sid::grab_sid,
    utils::{
        nom_helper::{
            nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
        },
        uuid::format_guid_le_bytes,
    },
};
use log::warn;
use nom::{bytes::complete::take, Needed};
use serde::Serialize;
use std::mem::size_of;

#[derive(Debug, PartialEq, Serialize, Clone)]
pub(crate) struct AccessControlEntry {
    pub(crate) ace_type: AceTypes,
    pub(crate) flags: Vec<AceFlags>,
    pub(crate) access_rights: Vec<AccessMask>,
    pub(crate) sid: String,
    pub(crate) account: String,
    /**Only if Object data_type and ACE_OBJECT_TYPE_PRESENT object flag */
    pub(crate) object_flags: ObjectFlag,
    /**Only if Object data_type and ACE_INHERITED_OBJECT_TYPE_PRESENT object flag */
    pub(crate) object_type_guid: String,
    pub(crate) inherited_object_type_guid: String,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub(crate) enum AceTypes {
    AccessAllowedAceType,
    AccessDeniedAceType,
    SystemAuditAceType,
    SystemAlarmAceType,
    Reserved,
    AccessAllowedObjectType,
    AccessDeniedObjectType,
    SystemAuditObjectType,
    SystemAlarmObjectType,
    AccessAllowedAceTypeCallback,
    AccessDeniedAceTypeCallback,
    SystemAuditAceTypeCallback,
    SystemAlarmAceTypeCallback,
    AccessAllowedObjectTypeCallback,
    AccessDeniedObjectTypeCallback,
    SystemAuditObjectTypeCallback,
    SystemAlarmObjectTypeCallback,
    SystemMandatoryLabel,
    Unknown,
    Ace,
    Object,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub(crate) enum ObjectFlag {
    ObjectType,
    InheritedObjectType,
    None,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub(crate) enum AceFlags {
    ObjectInherit,
    ContainerInherit,
    NoPropagateInherit,
    InheritOnly,
    SuccessfulAccess,
    FailedAccess,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub(crate) enum AccessItem {
    Folder,
    NonFolder,
    Mandatory,
    Registry,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub(crate) enum AccessMask {
    Delete,
    ReadControl,
    WriteDac,
    WriteOwner,
    Synchronize,
    _AccessSystemSecurity,
    _MaximumAllowed,
    GenericAll,
    GenericExecute,
    GenericWrite,
    GenericRead,
    FileReadData,
    FileWriteData,
    FileReadEa,
    FileWriteEa,
    FileExecute,
    FileReadAttributes,
    FileWriteAttributes,
    AppendMsg,
    FileListDirectory,
    FileAddFile,
    FileAddSubdirectory,
    MandatoryNoWriteUp,
    MandatoryNoReadUp,
    MandatoryNoExecuteUp,
    // Registry related. https://learn.microsoft.com/en-us/windows/win32/sysinfo/registry-key-security-and-access-rights
    AllAccess,
    CreateLink,
    CreateSubKey,
    EnumerateSubKeys,
    Execute,
    Notify,
    QueryValue,
    Read,
    SetValue,
    Wow64Key32,
    Wow64Key64,
    Write,
}

impl AccessControlEntry {
    /**
     * Parse the raw Windows Acess Control List (ACL) data  
     * Must specify the ACL item either: `NonFolder` or `Folder` or `Registry`  
     */
    pub(crate) fn parse_acl<'a>(
        data: &'a [u8],
        item: &AccessItem,
    ) -> nom::IResult<&'a [u8], Vec<AccessControlEntry>> {
        // Nom header of ACL
        let (input, _revision) = nom_unsigned_one_byte(data, Endian::Le)?;
        let (input, _padding) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, count) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, _padding) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let adjust_size = 8;
        if size < adjust_size {
            return Err(nom::Err::Incomplete(Needed::Unknown));
        }
        // Size includes the header too, but we already nom'd that away
        let (remaining_input, input) = take(size - adjust_size)(input)?;

        let mut entry_count = 0;
        let mut entries: Vec<AccessControlEntry> = Vec::new();
        let mut entries_data = input;
        // Loop and get all the ACL entries
        while entry_count < count {
            let (input, ace_type_value) = nom_unsigned_one_byte(entries_data, Endian::Le)?;
            let (input, flags_value) = nom_unsigned_one_byte(input, Endian::Le)?;
            let (input, size) = nom_unsigned_two_bytes(input, Endian::Le)?;

            let (ace_type, is_ace) = AccessControlEntry::get_ace_type(&ace_type_value);
            let flags = AccessControlEntry::get_ace_flags(&flags_value);

            let adjust_entry_size = 4;
            if size < adjust_size {
                return Err(nom::Err::Incomplete(Needed::Unknown));
            }
            // Size includes the header of the entry, but we already nom'd that away
            let (input, ace_entry_data) = take(size - adjust_entry_size)(input)?;
            entries_data = input;

            let endian = if item == &AccessItem::Registry {
                Endian::Le
            } else {
                Endian::Be
            };

            let (ace_entry_data, rights_data) = nom_unsigned_four_bytes(ace_entry_data, endian)?;

            let access_rights = if ace_type == AceTypes::SystemMandatoryLabel {
                AccessControlEntry::get_access_rights(&rights_data, &AccessItem::Mandatory)
            } else {
                AccessControlEntry::get_access_rights(&rights_data, item)
            };
            let mut ace_entry = AccessControlEntry {
                ace_type,
                flags,
                access_rights,
                sid: String::new(),
                account: String::new(),
                object_flags: ObjectFlag::None,
                object_type_guid: String::new(),
                inherited_object_type_guid: String::new(),
            };

            // Parse last bit of data depending on ACE type
            if is_ace == AceTypes::Ace {
                let (_, sid) = grab_sid(ace_entry_data)?;
                ace_entry.sid = sid;
            } else if is_ace == AceTypes::Object {
                let (ace_entry_data, object_flags_data) =
                    nom_unsigned_four_bytes(ace_entry_data, Endian::Le)?;
                let object_flag = AccessControlEntry::get_object_flag(&object_flags_data);
                if object_flag == ObjectFlag::ObjectType {
                    let (ace_entry_data, guid_data) = take(size_of::<u128>())(ace_entry_data)?;
                    ace_entry.object_type_guid = format_guid_le_bytes(guid_data);

                    let (_, sid) = grab_sid(ace_entry_data)?;
                    ace_entry.sid = sid;
                } else if object_flag == ObjectFlag::InheritedObjectType {
                    let (ace_entry_data, guid_data) = take(size_of::<u128>())(ace_entry_data)?;
                    ace_entry.inherited_object_type_guid = format_guid_le_bytes(guid_data);

                    let (_, sid) = grab_sid(ace_entry_data)?;
                    ace_entry.sid = sid;
                }
            }

            entries.push(ace_entry);
            entry_count += 1;
        }
        Ok((remaining_input, entries))
    }

    /// Determine what rights are associated with the ACL
    fn get_access_rights(rights_data: &u32, item: &AccessItem) -> Vec<AccessMask> {
        let mut rights = Vec::new();

        // Generic rights
        let gen_read = 0x80000000;
        let gen_write = 0x40000000;
        let gen_exec = 0x20000000;
        let gen_all = 0x10000000;
        // Standard rights
        let delete = 0x10000;
        let read_acl = 0x20000;
        let write_dac = 0x40000;
        let write_owner = 0x80000;
        let sync = 0x100000;
        // Non-folder rights
        let read = 0x1;
        let write = 0x2;
        let append_msg = 0x4;
        // Folder rights
        let list_dir = 0x1;
        let add_file = 0x2;
        let add_dir = 0x4;
        // Shared by both folder and non-folder
        let read_prop = 0x8;
        let write_prop = 0x10;
        let execute = 0x20;
        let read_attr = 0x80;
        let write_attr = 0x100;
        // Mandatory rights
        let no_writeup = 0x1;
        let no_readup = 0x2;
        let no_executeup = 0x4;

        // Lots of rights...now need to check them all

        // Standard
        if (rights_data & delete) == delete {
            rights.push(AccessMask::Delete);
        }
        if (rights_data & read_acl) == read_acl {
            rights.push(AccessMask::ReadControl);
        }
        if (rights_data & write_dac) == write_dac {
            rights.push(AccessMask::WriteDac);
        }
        if (rights_data & write_owner) == write_owner {
            rights.push(AccessMask::WriteOwner);
        }
        if (rights_data & sync) == sync {
            rights.push(AccessMask::Synchronize);
        }

        if item == &AccessItem::NonFolder {
            if (rights_data & read) == read {
                rights.push(AccessMask::FileReadData);
            }
            if (rights_data & write) == write {
                rights.push(AccessMask::FileWriteData);
            }
            if (rights_data & append_msg) == append_msg {
                rights.push(AccessMask::AppendMsg);
            }
            if (rights_data & execute) == execute {
                rights.push(AccessMask::FileExecute);
            }
        } else if item == &AccessItem::Folder {
            if (rights_data & list_dir) == list_dir {
                rights.push(AccessMask::FileListDirectory);
            }
            if (rights_data & add_file) == add_file {
                rights.push(AccessMask::FileAddFile);
            }
            if (rights_data & add_dir) == add_dir {
                rights.push(AccessMask::FileAddSubdirectory);
            }
        } else if item == &AccessItem::Mandatory {
            if (rights_data & no_writeup) == no_writeup {
                rights.push(AccessMask::MandatoryNoWriteUp);
            }
            if (rights_data & no_readup) == no_readup {
                rights.push(AccessMask::MandatoryNoReadUp);
            }
            if (rights_data & no_executeup) == no_executeup {
                rights.push(AccessMask::MandatoryNoExecuteUp);
            }
        } else if item == &AccessItem::Registry {
            let all_access = 0xf003f;
            let create_link = 0x20;
            let create_sub_key = 0x4;
            let enumerate_sub_keys = 0x8;
            let execute = 0x20019;
            let notify = 0x10;
            let query_value = 0x1;
            let set_value = 0x2;
            let wow_32key = 0x200;
            let wow_64key = 0x100;
            let write = 0x20006;

            if (rights_data & all_access) == all_access {
                rights.push(AccessMask::AllAccess);
            }
            if (rights_data & create_link) == create_link {
                rights.push(AccessMask::CreateLink);
            }
            if (rights_data & create_sub_key) == create_sub_key {
                rights.push(AccessMask::CreateSubKey);
            }
            if (rights_data & enumerate_sub_keys) == enumerate_sub_keys {
                rights.push(AccessMask::EnumerateSubKeys);
            }
            if (rights_data & execute) == execute {
                // Read and Execute have same value and mean the same thing
                rights.push(AccessMask::Execute);
                rights.push(AccessMask::Read);
            }
            if (rights_data & notify) == notify {
                rights.push(AccessMask::Notify);
            }
            if (rights_data & query_value) == query_value {
                rights.push(AccessMask::QueryValue);
            }
            if (rights_data & set_value) == set_value {
                rights.push(AccessMask::SetValue);
            }
            if (rights_data & wow_32key) == wow_32key {
                rights.push(AccessMask::Wow64Key32);
            }
            if (rights_data & wow_64key) == wow_64key {
                rights.push(AccessMask::Wow64Key64);
            }
            if (rights_data & write) == write {
                rights.push(AccessMask::Write);
            }

            return rights;
        }

        // Generic
        if (rights_data & gen_read) == gen_read {
            rights.push(AccessMask::GenericRead);
        }
        if (rights_data & gen_write) == gen_write {
            rights.push(AccessMask::GenericWrite);
        }
        if (rights_data & gen_exec) == gen_exec {
            rights.push(AccessMask::GenericExecute);
        }
        if (rights_data & gen_all) == gen_all {
            rights.push(AccessMask::GenericAll);
        }

        if (rights_data & read_prop) == read_prop {
            rights.push(AccessMask::FileReadEa);
        }
        if (rights_data & write_prop) == write_prop {
            rights.push(AccessMask::FileWriteEa);
        }
        if (rights_data & read_attr) == read_attr {
            rights.push(AccessMask::FileReadAttributes);
        }
        if (rights_data & write_attr) == write_attr {
            rights.push(AccessMask::FileWriteAttributes);
        }

        rights
    }

    /// Determine the ACE type
    fn get_ace_type(ace_type: &u8) -> (AceTypes, AceTypes) {
        match ace_type {
            0 => (AceTypes::AccessAllowedAceType, AceTypes::Ace),
            1 => (AceTypes::AccessDeniedAceType, AceTypes::Ace),
            2 => (AceTypes::SystemAuditAceType, AceTypes::Ace),
            3 => (AceTypes::SystemAlarmAceType, AceTypes::Ace),
            4 => (AceTypes::Reserved, AceTypes::Reserved),
            5 => (AceTypes::AccessAllowedObjectType, AceTypes::Object),
            6 => (AceTypes::AccessDeniedObjectType, AceTypes::Object),
            7 => (AceTypes::SystemAuditObjectType, AceTypes::Object),
            8 => (AceTypes::SystemAlarmObjectType, AceTypes::Object),
            9 => (AceTypes::AccessAllowedAceTypeCallback, AceTypes::Ace),
            10 => (AceTypes::AccessDeniedAceTypeCallback, AceTypes::Ace),
            11 => (AceTypes::AccessAllowedObjectTypeCallback, AceTypes::Object),
            12 => (AceTypes::AccessDeniedObjectTypeCallback, AceTypes::Object),
            13 => (AceTypes::SystemAuditAceTypeCallback, AceTypes::Ace),
            14 => (AceTypes::SystemAlarmAceTypeCallback, AceTypes::Ace),
            15 => (AceTypes::SystemAuditObjectTypeCallback, AceTypes::Object),
            16 => (AceTypes::SystemAlarmObjectTypeCallback, AceTypes::Object),
            17 => (AceTypes::SystemMandatoryLabel, AceTypes::Object),
            _ => {
                warn!("[acl] Unknown ACE Type: {ace_type}");
                (AceTypes::Unknown, AceTypes::Unknown)
            }
        }
    }

    /// Determine the ACE flags
    fn get_ace_flags(ace_flags: &u8) -> Vec<AceFlags> {
        let object_inherit = 1;
        let container_inherit = 2;
        let no_propagate = 4;
        let inherit = 8;
        let success_access = 0x40;
        let failed_access = 0x80;

        let mut flags = Vec::new();
        if (object_inherit & ace_flags) == object_inherit {
            flags.push(AceFlags::ObjectInherit);
        }
        if (container_inherit & ace_flags) == container_inherit {
            flags.push(AceFlags::ContainerInherit);
        }
        if (no_propagate & ace_flags) == no_propagate {
            flags.push(AceFlags::NoPropagateInherit);
        }
        if (inherit & ace_flags) == inherit {
            flags.push(AceFlags::InheritOnly);
        }
        if (success_access & ace_flags) == success_access {
            flags.push(AceFlags::SuccessfulAccess);
        }
        if (failed_access & ace_flags) == failed_access {
            flags.push(AceFlags::FailedAccess);
        }

        flags
    }

    /// Determine the ACE Object flag
    fn get_object_flag(object_flags: &u32) -> ObjectFlag {
        match object_flags {
            1 => ObjectFlag::ObjectType,
            2 => ObjectFlag::InheritedObjectType,
            _ => ObjectFlag::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AccessControlEntry;
    use crate::artifacts::os::windows::securitydescriptor::acl::{
        AccessMask, AceFlags, AceTypes, ObjectFlag,
    };

    #[test]
    fn test_parse_acl() {
        let test = [
            2, 0, 108, 0, 4, 0, 0, 0, 1, 0, 20, 0, 63, 0, 15, 0, 1, 1, 0, 0, 0, 0, 0, 5, 7, 0, 0,
            0, 1, 0, 20, 0, 63, 0, 15, 0, 1, 1, 0, 0, 0, 0, 0, 5, 2, 0, 0, 0, 0, 0, 36, 0, 63, 0,
            15, 0, 1, 5, 0, 0, 0, 0, 0, 5, 21, 0, 0, 0, 245, 64, 37, 10, 23, 232, 153, 20, 156,
            149, 218, 53, 233, 3, 0, 0, 0, 0, 24, 0, 63, 0, 15, 0, 1, 2, 0, 0, 0, 0, 0, 5, 32, 0,
            0, 0, 32, 2, 0, 0,
        ];

        let (_, results) =
            AccessControlEntry::parse_acl(&test, &super::AccessItem::NonFolder).unwrap();
        assert_eq!(results[0].ace_type, AceTypes::AccessDeniedAceType);
        assert_eq!(results[0].flags.len(), 0);
        assert_eq!(
            results[0].access_rights,
            vec![
                AccessMask::GenericExecute,
                AccessMask::GenericAll,
                AccessMask::FileWriteAttributes
            ]
        );
        assert_eq!(results[0].sid, "S-1-5-7");
        assert_eq!(results[0].account, "");
        assert_eq!(results[0].object_flags, ObjectFlag::None);
        assert_eq!(results[0].object_type_guid, "");
        assert_eq!(results[0].inherited_object_type_guid, "");

        assert_eq!(results[1].ace_type, AceTypes::AccessDeniedAceType);
        assert_eq!(results[1].flags.len(), 0);
        assert_eq!(
            results[1].access_rights,
            vec![
                AccessMask::GenericExecute,
                AccessMask::GenericAll,
                AccessMask::FileWriteAttributes
            ]
        );
        assert_eq!(results[1].sid, "S-1-5-2");
        assert_eq!(results[1].account, "");
        assert_eq!(results[1].object_flags, ObjectFlag::None);
        assert_eq!(results[1].object_type_guid, "");
        assert_eq!(results[1].inherited_object_type_guid, "");

        assert_eq!(results[2].ace_type, AceTypes::AccessAllowedAceType);
        assert_eq!(results[2].flags.len(), 0);
        assert_eq!(
            results[2].access_rights,
            vec![
                AccessMask::GenericExecute,
                AccessMask::GenericAll,
                AccessMask::FileWriteAttributes
            ]
        );
        assert_eq!(
            results[2].sid,
            "S-1-5-21-170213621-345630743-903517596-1001"
        );
        assert_eq!(results[2].account, "");
        assert_eq!(results[2].object_flags, ObjectFlag::None);
        assert_eq!(results[2].object_type_guid, "");
        assert_eq!(results[2].inherited_object_type_guid, "");
    }

    #[test]
    fn test_get_access_rights() {
        let test = 0x3f000f00;
        let results = AccessControlEntry::get_access_rights(&test, &super::AccessItem::NonFolder);
        assert_eq!(
            results,
            [
                AccessMask::GenericExecute,
                AccessMask::GenericAll,
                AccessMask::FileWriteAttributes
            ]
        );
    }

    #[test]
    fn test_get_ace_type() {
        let test = 13;
        let (results, result_type) = AccessControlEntry::get_ace_type(&test);
        assert_eq!(results, AceTypes::SystemAuditAceTypeCallback);
        assert_eq!(result_type, AceTypes::Ace);
    }

    #[test]
    fn test_get_ace_flags() {
        let test = 1;
        let results = AccessControlEntry::get_ace_flags(&test);
        assert_eq!(results, [AceFlags::ObjectInherit]);
    }

    #[test]
    fn test_get_object_flag() {
        let test = 1;
        let results = AccessControlEntry::get_object_flag(&test);
        assert_eq!(results, ObjectFlag::ObjectType);
    }
}
