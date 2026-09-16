use common::files::Attributes;

/// Return file attributes for Windows
pub(crate) fn windows_attributes(value: u32) -> Vec<Attributes> {
    let mut attributes = Vec::new();
    if (value & 0x1) != 0 {
        attributes.push(Attributes::ReadOnly);
    }
    if (value & 0x2) != 0 {
        attributes.push(Attributes::Hidden);
    }
    if (value & 0x4) != 0 {
        attributes.push(Attributes::System);
    }
    if (value & 0x10) != 0 {
        attributes.push(Attributes::Directory);
    }
    if (value & 0x20) != 0 {
        attributes.push(Attributes::Archive);
    }
    if (value & 0x40) != 0 {
        attributes.push(Attributes::Device);
    }
    if (value & 0x80) != 0 {
        attributes.push(Attributes::Normal);
    }
    if (value & 0x100) != 0 {
        attributes.push(Attributes::Temporary);
    }
    if (value & 0x200) != 0 {
        attributes.push(Attributes::Sparse);
    }
    if (value & 0x400) != 0 {
        attributes.push(Attributes::ReparsePoint);
    }
    if (value & 0x800) != 0 {
        attributes.push(Attributes::Compressed);
    }
    if (value & 0x1000) != 0 {
        attributes.push(Attributes::Offline);
    }
    if (value & 0x2000) != 0 {
        attributes.push(Attributes::NotContentIndexed);
    }
    if (value & 0x4000) != 0 {
        attributes.push(Attributes::Encrypted);
    }
    if (value & 0x8000) != 0 {
        attributes.push(Attributes::IntegritySystem);
    }
    if (value & 0x10000) != 0 {
        attributes.push(Attributes::Virtual);
    }
    if (value & 0x20000) != 0 {
        attributes.push(Attributes::NoScrubData);
    }
    if (value & 0x40000) != 0 {
        attributes.push(Attributes::ExtendedAttributes);
        attributes.push(Attributes::RecallOnOpen);
    }
    if (value & 0x80000) != 0 {
        attributes.push(Attributes::Pinned);
    }
    if (value & 0x100000) != 0 {
        attributes.push(Attributes::Unpinned);
    }
    if (value & 0x400000) != 0 {
        attributes.push(Attributes::RecallOnDataAccess);
    }

    attributes
}

/// Return file attributes for Unix systems
pub(crate) fn unix_attributes(value: u32) -> Vec<Attributes> {
    let mut attributes = Vec::new();

    if (value & 0o400) != 0 {
        attributes.push(Attributes::UserRead);
    }
    if (value & 0o200) != 0 {
        attributes.push(Attributes::UserWrite);
    }
    if (value & 0o100) != 0 {
        attributes.push(Attributes::UserExecute);
    }
    if (value & 0o40) != 0 {
        attributes.push(Attributes::GroupRead);
    }
    if (value & 0o20) != 0 {
        attributes.push(Attributes::GroupWrite);
    }
    if (value & 0o10) != 0 {
        attributes.push(Attributes::GroupExecute);
    }
    if (value & 0o4) != 0 {
        attributes.push(Attributes::OtherRead);
    }
    if (value & 0o2) != 0 {
        attributes.push(Attributes::OtherWrite);
    }
    if (value & 0o1) != 0 {
        attributes.push(Attributes::OtherExecute);
    }
    if (value & 0o4000) != 0 {
        attributes.push(Attributes::SetUid);
    }
    if (value & 0o2000) != 0 {
        attributes.push(Attributes::SetGid);
    }
    if (value & 0o1000) != 0 {
        attributes.push(Attributes::Sticky);
    }

    attributes
}
