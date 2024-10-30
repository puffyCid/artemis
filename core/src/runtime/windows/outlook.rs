use crate::{
    artifacts::os::windows::outlook::{
        header::FormatType,
        helper::{OutlookReader, OutlookReaderAction},
        reader::{setup_outlook_reader, setup_outlook_reader_windows},
        tables::context::TableInfo,
    },
    filesystem::ntfs::setup::setup_ntfs_parser,
};
use deno_core::{error::AnyError, op2};
use std::io::BufReader;

#[op2]
#[string]
pub(crate) fn get_root_folder(#[string] path: String, use_ntfs: bool) -> Result<String, AnyError> {
    let root = if use_ntfs {
        let mut ntfs_parser = setup_ntfs_parser(&path.chars().next().unwrap_or('C'))?;
        let ntfs_file =
            setup_outlook_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, &path)?;

        let mut reader = OutlookReader {
            fs: ntfs_parser.fs,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        reader.setup(Some(&ntfs_file))?;
        reader.root_folder(Some(&ntfs_file))?
    } else {
        let reader = setup_outlook_reader(&path)?;
        let buf_reader = BufReader::new(reader);

        let mut reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        reader.setup(None)?;
        reader.root_folder(None)?
    };

    let results = serde_json::to_string(&root)?;
    Ok(results)
}

#[op2]
#[string]
pub(crate) fn read_folder(
    #[string] path: String,
    use_ntfs: bool,
    #[bigint] folder: u64,
) -> Result<String, AnyError> {
    let folder = if use_ntfs {
        let mut ntfs_parser = setup_ntfs_parser(&path.chars().next().unwrap_or('C'))?;
        let ntfs_file =
            setup_outlook_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, &path)?;

        let mut reader = OutlookReader {
            fs: ntfs_parser.fs,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        reader.setup(Some(&ntfs_file))?;
        reader.read_folder(Some(&ntfs_file), folder)?
    } else {
        let reader = setup_outlook_reader(&path)?;
        let buf_reader = BufReader::new(reader);

        let mut reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        reader.setup(None)?;
        reader.read_folder(None, folder)?
    };

    let results = serde_json::to_string(&folder)?;
    Ok(results)
}

#[op2]
#[string]
pub(crate) fn read_messages(
    #[string] path: String,
    use_ntfs: bool,
    #[string] table: String,
    #[bigint] offset: u64,
) -> Result<String, AnyError> {
    let message_table: TableInfo = serde_json::from_str(&table)?;

    let messages = if use_ntfs {
        let mut ntfs_parser = setup_ntfs_parser(&path.chars().next().unwrap_or('C'))?;
        let ntfs_file =
            setup_outlook_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, &path)?;

        let mut reader = OutlookReader {
            fs: ntfs_parser.fs,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        reader.setup(Some(&ntfs_file))?;
        // This is difficult
        if message_table.has_branch.is_some() {
            let mut main_count = 0;
            let mut chunks = Vec::new();
            // Each branch has a collection of messages. Ex: Messages 0-20
            for branch in message_table.has_branch.as_ref().unwrap() {
                // If the offset is greater than the current branch message count.
                // Go to next branch. Ex: Branch 1 has messages 0-20. Branch 2 has messages 21-40, etc
                if offset > branch.rows_info.count + main_count {
                    main_count += branch.rows_info.count;
                    continue;
                }

                let mut emails = reader.read_message(Some(&ntfs_file), &message_table, None)?;
                chunks.append(&mut emails);
                if chunks.len() < message_table.rows.len() {
                    continue;
                }
            }
            chunks
        } else {
            reader.read_message(Some(&ntfs_file), &message_table, None)?
        }
    } else {
        let reader = setup_outlook_reader(&path)?;
        let buf_reader = BufReader::new(reader);

        let mut reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        reader.setup(None)?;
        if message_table.has_branch.is_some() {
            let mut main_count = 0;
            let mut chunks = Vec::new();
            // Each branch has a collection of messages. Ex: Messages 0-20
            for branch in message_table.has_branch.as_ref().unwrap() {
                // If the offset is greater than the current branch message count.
                // Go to next branch. Ex: Branch 1 has messages 0-20. Branch 2 has messages 21-40, etc
                if offset > branch.rows_info.count + main_count {
                    main_count += branch.rows_info.count;
                    continue;
                }

                let mut emails = reader.read_message(None, &message_table, None)?;
                chunks.append(&mut emails);
                if chunks.len() < message_table.rows.len() {
                    continue;
                }
                break;
            }
            chunks
        } else {
            reader.read_message(None, &message_table, None)?
        }
    };

    let results = serde_json::to_string(&messages)?;
    Ok(results)
}

#[op2]
#[string]
pub(crate) fn read_attachment(
    #[string] path: String,
    use_ntfs: bool,
    #[bigint] block_id: u64,
    #[bigint] descriptor_id: u64,
) -> Result<String, AnyError> {
    let attachment = if use_ntfs {
        let mut ntfs_parser = setup_ntfs_parser(&path.chars().next().unwrap_or('C'))?;
        let ntfs_file =
            setup_outlook_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, &path)?;

        let mut reader = OutlookReader {
            fs: ntfs_parser.fs,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        reader.setup(Some(&ntfs_file))?;
        reader.read_attachment(Some(&ntfs_file), &block_id, &descriptor_id)?
    } else {
        let reader = setup_outlook_reader(&path)?;
        let buf_reader = BufReader::new(reader);

        let mut reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        reader.setup(None)?;
        reader.read_attachment(None, &block_id, &descriptor_id)?
    };

    let results = serde_json::to_string(&attachment)?;
    Ok(results)
}
