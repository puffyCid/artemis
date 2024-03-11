// Full credit to: https://github.com/ForensicRS/frnsc-prefetch/blob/main/src/decompress/xpress_huff.rs - MIT License - 2024-03-07

use crate::utils::compression::error::CompressionError;
use std::{cell::RefCell, cmp::Ordering, rc::Rc};

// Inspired by https://raw.githubusercontent.com/Velocidex/go-prefetch/master/lzxpress.go

/// Decompress Huffman xpress compressed data
pub(crate) fn decompress_xpress_huffman(
    in_buf: &[u8],
    out_buf: &mut Vec<u8>,
) -> Result<(), CompressionError> {
    let mut in_index = 0;
    let mut out_index = 0;
    let output_size = out_buf.capacity();
    loop {
        let chunk_size = (output_size - out_index).min(65536);
        (in_index, out_index) = decompress_chunk(in_index, in_buf, out_index, out_buf, chunk_size)?;
        if in_index >= in_buf.len() || out_index >= output_size {
            break;
        }
    }
    if output_size < out_buf.len() {
        out_buf.resize(output_size, 0);
    }
    Ok(())
}

fn decompress_chunk(
    in_index: usize,
    in_buf: &[u8],
    out_index: usize,
    out_buf: &mut Vec<u8>,
    chunk_size: usize,
) -> Result<(usize, usize), CompressionError> {
    if in_index + 256 > in_buf.len() {
        return Ok((0, 0));
    }
    let root = prefix_code_tree_rebuild(&in_buf[in_index..])?;
    let mut bstr = BitStream::new(in_buf, in_index + 256);
    let mut i = out_index;
    while i < out_index + chunk_size {
        let mut symbol = match prefix_code_tree_decode_symbol(&mut bstr, root.clone()) {
            Ok(v) => v,
            Err(e) => match e {
                CompressionError::XpressNoMoreData => return Ok((bstr.index, i)),
                _ => return Err(e),
            },
        };
        if symbol < 256 {
            out_buf.push(symbol as u8);
            i += 1;
        } else {
            symbol -= 256;
            let mut length = symbol & 15;
            symbol >>= 4;

            let mut offset = 0;
            if symbol != 0 {
                offset = bstr.lookup(symbol) as i32;
            }
            offset |= 1 << symbol;
            offset = -offset;

            if length == 15 {
                length = (bstr.source[bstr.index] as u32) + 15;
                bstr.index += 1;
                if length == 270 {
                    length = u16::from_le_bytes(
                        bstr.source[bstr.index..bstr.index + 2]
                            .try_into()
                            .unwrap_or_default(),
                    ) as u32;
                    bstr.index += 2;
                }
            }
            bstr.skip(symbol)?;

            length += 3;
            loop {
                if (i as i32 + offset) < 0 {
                    return Err(CompressionError::XpressBadOffset);
                }
                let position = (i as i32 + offset) as usize;
                out_buf.push(out_buf[position]);
                i += 1;
                length -= 1;
                if length == 0 {
                    break;
                }
            }
        }
    }
    Ok((bstr.index, i))
}

struct BitStream<'a> {
    pub(crate) source: &'a [u8],
    pub(crate) index: usize,
    pub(crate) mask: u32,
    pub(crate) bits: u32,
}
impl<'a> BitStream<'a> {
    pub(crate) fn new(source: &'a [u8], in_pos: usize) -> Self {
        let mask = ((u16::from_le_bytes(source[in_pos..in_pos + 2].try_into().unwrap_or_default())
            as u32)
            << 16)
            + (u16::from_le_bytes(
                source[in_pos + 2..in_pos + 4]
                    .try_into()
                    .unwrap_or_default(),
            ) as u32);
        Self {
            source,
            index: in_pos + 4,
            bits: 32,
            mask,
        }
    }

    pub(crate) fn lookup(&self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        self.mask >> (32 - n)
    }
    pub(crate) fn skip(&mut self, n: u32) -> Result<(), CompressionError> {
        self.mask <<= n;
        self.bits = self.bits.saturating_sub(n);
        if self.bits < 16 {
            if (self.index + 2) > self.source.len() {
                return Err(CompressionError::XpressNoMoreData);
            }
            self.mask += (u16::from_le_bytes(
                self.source[self.index..self.index + 2]
                    .try_into()
                    .unwrap_or_default(),
            ) as u32)
                << (16 - self.bits);
            self.index += 2;
            self.bits += 16;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
struct PrefixCodeNode {
    pub(crate) id: u32,
    pub(crate) symbol: u32,
    pub(crate) leaf: bool,
    pub(crate) child: [Option<Rc<RefCell<PrefixCodeNode>>>; 2],
}

#[derive(Clone, Debug, Default)]
struct PrefixCodeSymbol {
    pub(crate) id: u32,
    pub(crate) symbol: u32,
    pub(crate) length: u32,
}

impl Ord for PrefixCodeSymbol {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.length, &self.symbol).cmp(&(other.length, &other.symbol))
    }
}

impl PartialOrd for PrefixCodeSymbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PrefixCodeSymbol {
    fn eq(&self, other: &Self) -> bool {
        (self.length, &self.symbol) == (other.length, &other.symbol)
    }
}

impl Eq for PrefixCodeSymbol {}

fn prefix_code_tree_add_leaf(
    tree_nodes: &[Rc<RefCell<PrefixCodeNode>>],
    leaf_index: usize,
    mask: u32,
    bits: u32,
) -> Result<u32, CompressionError> {
    let mut node = match tree_nodes.first() {
        Some(v) => v.clone(),
        None => return Err(CompressionError::XpressBadPrefix),
    };
    let mut i = leaf_index + 1;
    let mut child_index;
    let mut bits = bits;
    while bits > 1 {
        bits -= 1;
        child_index = (mask >> bits) & 1;
        let mut nt = node.borrow_mut();
        if nt.child[child_index as usize].is_none() {
            nt.child[child_index as usize] = Some(tree_nodes[i].clone());
            let mut i_node = tree_nodes[i].borrow_mut();
            i_node.leaf = false;
            i += 1;
        }
        let new_node_opt = nt.child[child_index as usize].as_ref();
        let new_node = match new_node_opt {
            Some(result) => result.clone(),
            None => return Err(CompressionError::XpressNoChildNode),
        };
        drop(nt);
        node = new_node;
    }
    let mut nt = node.borrow_mut();
    nt.child[(mask & 1) as usize] = Some(tree_nodes[leaf_index].clone());
    Ok(i as u32)
}

fn prefix_code_tree_rebuild(input: &[u8]) -> Result<Rc<RefCell<PrefixCodeNode>>, CompressionError> {
    let mut tree_nodes: Vec<Rc<RefCell<PrefixCodeNode>>> = (0..1024)
        .map(|_| Rc::new(RefCell::new(PrefixCodeNode::default())))
        .collect();
    let mut symbol_info: Vec<PrefixCodeSymbol> =
        (0..512).map(|_| PrefixCodeSymbol::default()).collect();
    for i in 0..256 {
        let mut value = input[i] as usize;

        symbol_info[2 * i].id = (2 * i) as u32;
        symbol_info[2 * i].symbol = (2 * i) as u32;
        symbol_info[2 * i].length = (value & 0xf) as u32;

        value >>= 4;
        symbol_info[(2 * i) + 1].id = ((2 * i) + 1) as u32;
        symbol_info[(2 * i) + 1].symbol = ((2 * i) + 1) as u32;
        symbol_info[(2 * i) + 1].length = (value & 0xf) as u32;
    }
    symbol_info.sort_by(|a, b| {
        if a.length < b.length {
            return Ordering::Less;
        }
        if a.symbol < b.symbol {
            return Ordering::Less;
        }
        Ordering::Equal
    });
    let mut i = 0;
    while i < 512 && symbol_info[i].length == 0 {
        i += 1;
    }
    let mut mask = 0;
    let mut bits = 1;

    let mut j = 1;
    for symbol in symbol_info.iter().take(512).skip(i) {
        let mut tree_node_j = tree_nodes[j].borrow_mut();
        tree_node_j.id = j as u32;
        tree_node_j.symbol = symbol.symbol;
        tree_node_j.leaf = true;
        drop(tree_node_j);

        mask <<= symbol.length - bits;
        bits = symbol.length;
        j = prefix_code_tree_add_leaf(&tree_nodes, j, mask, bits)? as usize;
        mask += 1;
    }
    let root = tree_nodes.remove(0);
    Ok(root)
}

fn prefix_code_tree_decode_symbol(
    bstr: &mut BitStream<'_>,
    root: Rc<RefCell<PrefixCodeNode>>,
) -> Result<u32, CompressionError> {
    let mut node = root;
    loop {
        let bit = bstr.lookup(1);
        bstr.skip(1)?;
        let nt = node.borrow();
        let new_node = match &nt.child[bit as usize] {
            Some(v) => v.clone(),
            None => return Err(CompressionError::XpressNoChild),
        };
        drop(nt);
        node = new_node;
        if node.borrow().leaf {
            break;
        }
    }
    let nt = node.borrow();
    Ok(nt.symbol)
}

#[cfg(test)]
mod tests {
    use crate::{
        filesystem::files::read_file,
        utils::compression::xpress::huffman::decompress_xpress_huffman,
    };
    use std::path::PathBuf;

    #[test]
    fn basic_huffman_decoding() {
        let uncompressed = b"abcdefghijklmnopqrstuvwxyz";
        let encoded: [u8; 276] = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x50, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
            0x55, 0x55, 0x55, 0x45, 0x44, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0xd8, 0x52, 0x3e, 0xd7, 0x94, 0x11, 0x5b, 0xe9, 0x19, 0x5f,
            0xf9, 0xd6, 0x7c, 0xdf, 0x8d, 0x04, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut decoded_value = Vec::with_capacity(uncompressed.len());
        decompress_xpress_huffman(&encoded, &mut decoded_value).unwrap();
        assert_eq!(uncompressed, &decoded_value[..]);
    }

    #[test]
    fn basic_huffman_decoding2() {
        let uncompressed = b"abcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabc";
        let encoded: [u8; 263] = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0xa8, 0xdc, 0x00, 0x00, 0xff, 0x26, 0x01,
        ];

        let mut decoded_value = Vec::with_capacity(uncompressed.len());
        decompress_xpress_huffman(&encoded, &mut decoded_value).unwrap();
        assert_eq!(uncompressed, &decoded_value[..]);
    }

    #[test]
    fn test_decompress_xpress_huffman_prefetch() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/compression/lz_huffman.raw");
        let mut bytes = read_file(&test_location.display().to_string()).unwrap();

        let mut out: Vec<u8> = Vec::with_capacity(153064);

        decompress_xpress_huffman(&mut bytes, &mut out).unwrap();
        assert_eq!(out.len(), 153064);
    }
}
