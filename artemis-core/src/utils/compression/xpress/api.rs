use crate::utils::compression::{decompress::XpressType, error::CompressionError};
use log::error;
use ntapi::{
    ntrtl::{RtlDecompressBufferEx, RtlGetCompressionWorkSpaceSize},
    winapi::um::winnt::PVOID,
};

/// Decompress LZXPRESS HUFFMAN using Windows API
pub(crate) fn decompress_huffman_api(
    data: &mut [u8],
    xpress_format: &XpressType,
    decompress_size: u32,
) -> Result<Vec<u8>, CompressionError> {
    let mut buffer_workspace_size: u32 = 0;
    let mut frag_workspace_size: u32 = 0;
    let mut decom_size = 0;

    let mut decompress_data = Vec::with_capacity(decompress_size as usize);

    let success = 0;
    let format = match xpress_format {
        XpressType::XpressHuffman => 4,
        XpressType::Lz77 => 3,
        XpressType::_Lznt => 2,
        XpressType::_Default => 1,
        XpressType::_None => 0,
    };

    // Make two calls to Windows APIs to decompress the data
    #[allow(unsafe_code)]
    unsafe {
        let status = RtlGetCompressionWorkSpaceSize(
            format,
            &mut buffer_workspace_size,
            &mut frag_workspace_size,
        );
        if status != success {
            error!("[compression] Failed to get lzxpress huffmane workspace size: {status}");
            return Err(CompressionError::HuffmanCompression);
        }

        let frag_result = frag_workspace_size.try_into();
        let frag_size: usize = match frag_result {
            Ok(result) => result,
            Err(err) => {
                error!("[compression] Failed to get fragment workspace size data: {err:?}");
                return Err(CompressionError::HuffmanCompression);
            }
        };
        let mut frag_data_size: Vec<PVOID> = Vec::with_capacity(frag_size);

        let status = RtlDecompressBufferEx(
            format,
            decompress_data.as_mut_ptr(),
            decompress_size,
            data.as_mut_ptr(),
            data.len() as u32,
            &mut decom_size,
            frag_data_size.as_mut_ptr().cast::<std::ffi::c_void>(),
        );
        if status != success {
            error!("[compression] Failed to decompress data: {status}");
            return Err(CompressionError::HuffmanCompression);
        }
        decompress_data.set_len(decom_size as usize);

        Ok(decompress_data)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        filesystem::files::read_file,
        utils::compression::{decompress::XpressType, xpress::api::decompress_huffman_api},
    };
    use std::path::PathBuf;

    #[test]
    fn test_decompress_huffman_api() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/compression/lz_huffman.raw");
        let mut bytes = read_file(&test_location.display().to_string()).unwrap();

        let decom_data =
            decompress_huffman_api(&mut bytes, &XpressType::XpressHuffman, 153064).unwrap();
        assert_eq!(decom_data.len(), 153064);
    }
}
