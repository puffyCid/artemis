use std::io::Read;

use miniz_oxide::{
    DataFormat, MZError, MZFlush, MZStatus,
    inflate::stream::{InflateState, inflate},
};
use ruzstd::decoding::StreamingDecoder;

use crate::accessor::{
    disk::qcow::header::Compression,
    error::{AccessorError, AccessorResult},
};

/// Decompress a QCOW cluster to provided `decom_cluster_size` bytes
pub(super) fn decompress_cluster(
    compression: Compression,
    src: &[u8],
    decom_cluster_size: u64,
) -> AccessorResult<Vec<u8>> {
    if src.is_empty() {
        return Err(AccessorError::volume(
            "QCOW compressed cluster does not contain a compressed stream",
        ));
    }

    let cluster_size = checked_cluster_size(decom_cluster_size)?;
    match compression {
        Compression::Zlib => decom_zlib(src, cluster_size),
        Compression::Zstd => decom_zstd(src, cluster_size),
    }
}

/// Check for valid max cluster size
fn checked_cluster_size(cluster_size: u64) -> AccessorResult<usize> {
    let max_size = 1 << 21;
    if cluster_size == 0 || cluster_size > max_size {
        return Err(AccessorError::volume(format!(
            "QCOW compressed cluster size {cluster_size} is outside the supported range"
        )));
    }

    usize::try_from(cluster_size).map_err(|_| {
        AccessorError::volume(format!(
            "QCOW compressed cluster size {cluster_size} does not fit in memory"
        ))
    })
}

/// Decompress zlib compressed QCOW data
fn decom_zlib(src: &[u8], decom_size: usize) -> AccessorResult<Vec<u8>> {
    let mut state = InflateState::new_boxed(DataFormat::Raw);
    let mut out = vec![0_u8; decom_size];

    let result = inflate(&mut state, src, &mut out, MZFlush::None);
    let written = result.bytes_written;
    let status = result.status;

    if written == decom_size
        && matches!(
            status,
            Ok(MZStatus::Ok | MZStatus::StreamEnd) | Err(MZError::Buf)
        )
    {
        return Ok(out);
    }

    Err(AccessorError::volume(format!(
        "QCOW zlib cluster decompressed to {written} bytes instead of {decom_size}: {status:?}"
    )))
}

/// Decompress zstandard compressed QCOW data
fn decom_zstd(src: &[u8], decom_size: usize) -> AccessorResult<Vec<u8>> {
    let mut decoder = StreamingDecoder::new(src).map_err(|err| {
        AccessorError::volume(format!(
            "QCOW zstd cluster could not be decompressed: {err}"
        ))
    })?;

    let mut out = vec![0_u8; decom_size.saturating_add(1)];
    let mut filled = 0;
    while filled < out.len() {
        match decoder.read(&mut out[filled..]) {
            Ok(0) => break,
            Ok(read) => filled += read,
            Err(err) => {
                return Err(AccessorError::volume(format!(
                    "QCOW zstd cluster could not be decompressed: {err}"
                )));
            }
        }
    }

    if filled != decom_size {
        return Err(AccessorError::volume(format!(
            "QCOW zstd cluster decompressed to {filled} bytes instead of {decom_size}"
        )));
    }

    out.truncate(decom_size);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::decompress_cluster;
    use crate::accessor::{disk::qcow::header::Compression, error::AccessorError};
    use miniz_oxide::{
        DataFormat,
        deflate::{
            compress_to_vec_zlib,
            core::{CompressionStrategy, CompressorOxide, TDEFLFlush, TDEFLStatus, compress},
        },
    };
    use ruzstd::encoding::{CompressionLevel, compress_to_vec};

    fn guest_cluster() -> Vec<u8> {
        let mut cluster = vec![0x5A; 512];
        cluster[0] = 0x11;
        cluster[100] = 0x22;
        cluster[511] = 0x33;

        cluster
    }

    fn raw_zlib(plain: &[u8]) -> Vec<u8> {
        let mut compressor =
            CompressorOxide::with_params(DataFormat::Raw, 6, CompressionStrategy::Default, 12);
        let mut output = vec![0_u8; plain.len() + 64];
        let mut input = plain;
        let mut out_pos = 0;

        loop {
            let (status, bytes_in, bytes_out) = compress(
                &mut compressor,
                input,
                &mut output[out_pos..],
                TDEFLFlush::Finish,
            );
            out_pos += bytes_out;

            match status {
                TDEFLStatus::Done => {
                    output.truncate(out_pos);
                    return output;
                }
                TDEFLStatus::Okay => {
                    input = &input[bytes_in..];
                    if output.len() - out_pos < 64 {
                        output.resize(output.len() * 2, 0);
                    }
                }
                other => panic!("raw zlib failed: {other:?}"),
            }
        }
    }

    fn volume_reason(compression: Compression, src: &[u8], cluster_size: u64) -> String {
        match decompress_cluster(compression, src, cluster_size) {
            Err(AccessorError::Volume { reason }) => reason,
            Err(err) => panic!("expected a volume error, got {err}"),
            Ok(_) => panic!("expected a volume error"),
        }
    }

    #[test]
    fn test_decompress_zlib_cluster() {
        let plain = guest_cluster();
        let compressed = raw_zlib(&plain);

        let result = decompress_cluster(
            Compression::Zlib,
            &compressed,
            u64::try_from(plain.len()).unwrap(),
        )
        .unwrap();

        assert_eq!(result, plain);
    }

    #[test]
    fn test_decompress_zlib_ignores_sector_padding() {
        let plain = guest_cluster();
        let mut compressed = raw_zlib(&plain);
        compressed.resize(compressed.len().next_multiple_of(512), 0xA5);

        let result = decompress_cluster(
            Compression::Zlib,
            &compressed,
            u64::try_from(plain.len()).unwrap(),
        )
        .unwrap();

        assert_eq!(result, plain);
    }

    #[test]
    fn test_decompress_rejects_zlib_wrapper() {
        let plain = guest_cluster();
        let wrapped = compress_to_vec_zlib(&plain, 6);

        let reason = volume_reason(
            Compression::Zlib,
            &wrapped,
            u64::try_from(plain.len()).unwrap(),
        );

        assert!(reason.contains("zlib"), "{reason}");
    }

    #[test]
    fn test_decompress_rejects_truncated_zlib() {
        let plain = guest_cluster();
        let compressed = raw_zlib(&plain);

        let reason = volume_reason(
            Compression::Zlib,
            &compressed[..compressed.len() / 2],
            u64::try_from(plain.len()).unwrap(),
        );

        assert!(reason.contains("zlib"), "{reason}");
    }

    #[test]
    fn test_decompress_zstd_cluster() {
        let plain = guest_cluster();
        let compressed = compress_to_vec(plain.as_slice(), CompressionLevel::Fastest);

        let result = decompress_cluster(
            Compression::Zstd,
            &compressed,
            u64::try_from(plain.len()).unwrap(),
        )
        .unwrap();

        assert_eq!(result, plain);
    }

    #[test]
    fn test_decompress_zstd_ignores_sector_padding() {
        let plain = guest_cluster();
        let mut compressed = compress_to_vec(plain.as_slice(), CompressionLevel::Fastest);
        compressed.resize(compressed.len().next_multiple_of(512), 0xA5);

        let result = decompress_cluster(
            Compression::Zstd,
            &compressed,
            u64::try_from(plain.len()).unwrap(),
        )
        .unwrap();

        assert_eq!(result, plain);
    }

    #[test]
    fn test_decompress_rejects_truncated_zstd() {
        let plain = guest_cluster();
        let compressed = compress_to_vec(plain.as_slice(), CompressionLevel::Fastest);

        let reason = volume_reason(
            Compression::Zstd,
            &compressed[..compressed.len() / 2],
            u64::try_from(plain.len()).unwrap(),
        );

        assert!(reason.contains("zstd"), "{reason}");
    }

    #[test]
    fn test_decompress_rejects_zstd_longer_than_cluster() {
        let plain = guest_cluster();
        let compressed = compress_to_vec(plain.as_slice(), CompressionLevel::Fastest);
        let reason = volume_reason(Compression::Zstd, &compressed, 64);

        assert!(reason.contains("64"), "{reason}");
    }

    #[test]
    fn test_decompress_rejects_empty_stream() {
        let reason = volume_reason(Compression::Zlib, &[], 512);
        assert!(reason.contains("compressed stream"), "{reason}");
    }
}
