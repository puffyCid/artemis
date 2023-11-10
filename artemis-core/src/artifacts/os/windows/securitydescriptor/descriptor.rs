use super::{acl::parse_acl, sid::grab_sid};
use crate::utils::nom_helper::{
    nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
};
use common::windows::{AccessControlEntry, AccessItem::Registry};
use nom::bytes::complete::take;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub(crate) struct Descriptor {
    pub(crate) control_flags: Vec<ControlFlags>,
    pub(crate) sacls: Vec<AccessControlEntry>,
    pub(crate) dacls: Vec<AccessControlEntry>,
    pub(crate) owner_sid: String,
    pub(crate) group_sid: String,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub(crate) enum ControlFlags {
    OwnerDefaulted,
    GroupDefaulted,
    DaclPresent,
    DaclDefaulted,
    SaclPresent,
    SaclDefaulted,
    DaclAutoInheritReq,
    SaclAutoInheritReq,
    DaclAutoInherited,
    SaclAutoInherited,
    DaclProtected,
    SaclProtected,
    ResourceManagerControlValid,
    SelfRelative,
}

impl Descriptor {
    /// Parse the Security Descriptor data. Typically contains  ACLs and SIDs
    pub(crate) fn parse_descriptor(data: &[u8]) -> nom::IResult<&[u8], Descriptor> {
        let (input, _revision) = nom_unsigned_one_byte(data, Endian::Le)?;
        let (input, _padding) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, control_flags) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, owner_sid_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, group_sid_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, sacl_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, dacl_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let empty = 0;
        let mut security_info = Descriptor {
            control_flags: Descriptor::get_flags(&control_flags),
            sacls: Vec::new(),
            dacls: Vec::new(),
            owner_sid: String::new(),
            group_sid: String::new(),
        };

        if sacl_offset != empty {
            let (acl_start, _) = take(sacl_offset)(data)?;
            let (_, acl) = parse_acl(acl_start, &Registry)?;
            security_info.sacls = acl;
        }
        if dacl_offset != empty {
            let (acl_start, _) = take(dacl_offset)(data)?;
            let (_, acl) = parse_acl(acl_start, &Registry)?;
            security_info.dacls = acl;
        }

        if owner_sid_offset != empty {
            let (sid_start, _) = take(owner_sid_offset)(data)?;
            let (_, sid) = grab_sid(sid_start)?;
            security_info.owner_sid = sid;
        }
        if group_sid_offset != empty {
            let (sid_start, _) = take(group_sid_offset)(data)?;
            let (_, sid) = grab_sid(sid_start)?;
            security_info.group_sid = sid;
        }

        Ok((input, security_info))
    }

    /// Get the Control Flags associated with the security descriptor
    fn get_flags(control: &u16) -> Vec<ControlFlags> {
        let own_default = 0x1;
        let group_default = 0x2;
        let dacl_present = 0x4;
        let dacl_default = 0x8;
        let sacl_present = 0x10;
        let sacl_default = 0x20;
        let dacl_auto_req = 0x100;
        let sacl_auto_req = 0x200;
        let dacl_auto = 0x400;
        let sacl_auto = 0x800;
        let dacl_protected = 0x1000;
        let sacl_protected = 0x2000;
        let rm_control = 0x4000;
        let relative = 0x8000;

        let mut flags = Vec::new();

        if (control & own_default) == own_default {
            flags.push(ControlFlags::OwnerDefaulted);
        }
        if (control & group_default) == group_default {
            flags.push(ControlFlags::GroupDefaulted);
        }
        if (control & dacl_present) == dacl_present {
            flags.push(ControlFlags::DaclPresent);
        }
        if (control & dacl_default) == dacl_default {
            flags.push(ControlFlags::DaclDefaulted);
        }
        if (control & sacl_present) == sacl_present {
            flags.push(ControlFlags::SaclPresent);
        }
        if (control & sacl_default) == sacl_default {
            flags.push(ControlFlags::SaclDefaulted);
        }
        if (control & dacl_auto_req) == dacl_auto_req {
            flags.push(ControlFlags::DaclAutoInheritReq);
        }
        if (control & sacl_auto_req) == sacl_auto_req {
            flags.push(ControlFlags::SaclAutoInheritReq);
        }
        if (control & dacl_auto) == dacl_auto {
            flags.push(ControlFlags::DaclAutoInherited);
        }
        if (control & sacl_auto) == sacl_auto {
            flags.push(ControlFlags::SaclAutoInherited);
        }
        if (control & dacl_protected) == dacl_protected {
            flags.push(ControlFlags::DaclProtected);
        }
        if (control & sacl_protected) == sacl_protected {
            flags.push(ControlFlags::SaclProtected);
        }
        if (control & rm_control) == rm_control {
            flags.push(ControlFlags::ResourceManagerControlValid);
        }
        if (control & relative) == relative {
            flags.push(ControlFlags::SelfRelative);
        }

        flags
    }
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::securitydescriptor::descriptor::ControlFlags::{
        DaclAutoInherited, DaclPresent, DaclProtected, OwnerDefaulted, SaclAutoInherited,
        SaclPresent, SelfRelative,
    };
    use crate::artifacts::os::windows::securitydescriptor::descriptor::Descriptor;
    use common::windows::AccessMask::{
        EnumerateSubKeys, Execute, Notify, QueryValue, Read, ReadControl,
    };
    use common::windows::AceTypes;

    #[test]
    fn test_parse_descriptor() {
        let test = [
            1, 0, 20, 156, 196, 0, 0, 0, 212, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0, 0, 2, 0, 176, 0, 6, 0,
            0, 0, 0, 2, 24, 0, 25, 0, 2, 0, 1, 2, 0, 0, 0, 0, 0, 5, 32, 0, 0, 0, 33, 2, 0, 0, 0, 2,
            24, 0, 63, 0, 15, 0, 1, 2, 0, 0, 0, 0, 0, 5, 32, 0, 0, 0, 32, 2, 0, 0, 0, 2, 20, 0, 63,
            0, 15, 0, 1, 1, 0, 0, 0, 0, 0, 5, 18, 0, 0, 0, 0, 2, 20, 0, 63, 0, 15, 0, 1, 1, 0, 0,
            0, 0, 0, 3, 0, 0, 0, 0, 0, 2, 24, 0, 25, 0, 2, 0, 1, 2, 0, 0, 0, 0, 0, 15, 2, 0, 0, 0,
            1, 0, 0, 0, 0, 2, 56, 0, 25, 0, 2, 0, 1, 10, 0, 0, 0, 0, 0, 15, 3, 0, 0, 0, 0, 4, 0, 0,
            176, 49, 128, 63, 108, 188, 99, 76, 60, 224, 80, 209, 151, 12, 161, 98, 15, 1, 203, 25,
            126, 122, 166, 192, 250, 230, 151, 241, 25, 163, 12, 206, 1, 2, 0, 0, 0, 0, 0, 5, 32,
            0, 0, 0, 32, 2, 0, 0, 1, 1, 0, 0, 0, 0, 0, 5, 18, 0, 0, 0,
        ];
        let (_, results) = Descriptor::parse_descriptor(&test).unwrap();

        assert_eq!(results.group_sid, "S-1-5-18");
        assert_eq!(
            results.control_flags,
            vec![
                DaclPresent,
                SaclPresent,
                DaclAutoInherited,
                SaclAutoInherited,
                DaclProtected,
                SelfRelative
            ]
        );
        assert_eq!(results.owner_sid, "S-1-5-32-544");
        assert_eq!(results.dacls[0].ace_type, AceTypes::AccessAllowedAceType);
        assert_eq!(
            results.dacls[0].access_rights,
            vec![
                ReadControl,
                EnumerateSubKeys,
                Execute,
                Read,
                Notify,
                QueryValue
            ]
        );
        assert_eq!(results.dacls[0].sid, "S-1-5-32-545");
        assert_eq!(results.dacls.len(), 6);
    }

    #[test]
    fn test_get_flags() {
        let test = 0x1;
        let results = Descriptor::get_flags(&test);
        assert_eq!(results[0], OwnerDefaulted)
    }
}
