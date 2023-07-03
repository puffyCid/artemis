use crate::utils::nom_helper::{nom_unsigned_eight_bytes, Endian};

pub(crate) struct HashTable {
    items: Vec<HashItem>,
}

pub(crate) struct HashItem {
    head_hash_offset: u64,
    tail_hash_offset: u64,
}

impl HashTable {
    /// Parse a `Journal` hash table. There should be only one Field and Data hash table per journal file
    pub(crate) fn parse_hash_table(data: &[u8]) -> nom::IResult<&[u8], HashTable> {
        let mut items: Vec<HashItem> = Vec::new();
        let mut input = data;
        let min_size = 16;
        while !input.is_empty() && input.len() >= min_size {
            let (remaining_input, head_hash_offset) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (remaining_input, tail_hash_offset) =
                nom_unsigned_eight_bytes(remaining_input, Endian::Le)?;
            input = remaining_input;

            let empty = 0;
            if head_hash_offset == empty && tail_hash_offset == 0 {
                continue;
            }

            let item = HashItem {
                head_hash_offset,
                tail_hash_offset,
            };

            items.push(item);
        }

        let hash_table = HashTable { items };

        Ok((input, hash_table))
    }
}

#[cfg(test)]
mod tests {
    use super::HashTable;

    #[test]
    fn test_parse_hash_table() {
        let test_data = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 48, 133, 63, 0, 0, 0, 0, 0, 48, 133, 63, 0, 0, 0, 0, 0,
        ];

        let (_, results) = HashTable::parse_hash_table(&test_data).unwrap();
        assert_eq!(results.items[0].head_hash_offset, 4162864);
        assert_eq!(results.items[0].tail_hash_offset, 4162864);
    }
}
