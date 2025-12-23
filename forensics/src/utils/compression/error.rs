use std::fmt;

#[derive(Debug)]
pub enum CompressionError {
    GzipReadFile,
    GzipDecompress,
    ZstdDecompress,
    Lz4Decompress,
    XzDecompress,
    CompressCreate,
    GzipFinish,
    XpressNoMoreData,
    XpressBadOffset,
    XpressBadPrefix,
    XpressNoChild,
    XpressNoChildNode,
    LzntBadFormat,
    Lz77BadLength,
    #[cfg(target_os = "windows")]
    HuffmanCompression,
    HuffmanCompressionNone,
    HuffmanCompressionDefault,
    ZlibDecompress,
    RtfCorrupted,
    SnappyDecompress,
    LzvnUndefined,
    LzvnBadOffset,
}

impl std::error::Error for CompressionError {}

impl fmt::Display for CompressionError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompressionError::GzipReadFile => write!(f, "Could not read file"),
            CompressionError::GzipDecompress => write!(f, "Could not decompress gzip data"),
            CompressionError::ZstdDecompress => write!(f, "Could not decompress zstd data"),
            CompressionError::Lz4Decompress => write!(f, "Could not decompress lz4 data"),
            CompressionError::XzDecompress => write!(f, "Could not decompress xz data"),
            CompressionError::CompressCreate => write!(f, "Could not create file for compression"),
            CompressionError::GzipFinish => write!(f, "Could not complete gzip compression"),
            #[cfg(target_os = "windows")]
            CompressionError::HuffmanCompression => {
                write!(f, "Failed to decompress huffman compressed data")
            }
            CompressionError::XpressNoMoreData => write!(f, "No more xpress huffman data"),
            CompressionError::XpressBadOffset => write!(f, "Bad xpress offset"),
            CompressionError::XpressBadPrefix => write!(f, "Bad xpress prefix"),
            CompressionError::XpressNoChild => write!(f, "No xpress child"),
            CompressionError::XpressNoChildNode => write!(f, "No xpress child node"),
            CompressionError::LzntBadFormat => write!(f, "Failed to decompress lznt"),
            CompressionError::Lz77BadLength => write!(f, "Failed to decompress lz77"),
            CompressionError::HuffmanCompressionDefault => {
                write!(f, "Huffman default not supported")
            }
            CompressionError::HuffmanCompressionNone => {
                write!(f, "Huffman none not supported")
            }
            CompressionError::ZlibDecompress => write!(f, "Failed to decompress zlib"),
            CompressionError::RtfCorrupted => write!(f, "Failed to decompress RTF"),
            CompressionError::SnappyDecompress => write!(f, "Failed to decompress snappy"),
            CompressionError::LzvnUndefined => write!(f, "Got undefined lzvn code"),
            CompressionError::LzvnBadOffset => {
                write!(f, "Bad offset provided to compressed lzvn data")
            }
        }
    }
}
