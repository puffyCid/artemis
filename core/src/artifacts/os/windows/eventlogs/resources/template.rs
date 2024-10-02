/*
 * Format is mixture of https://github.com/libyal/libfwevt/blob/main/documentation/Windows%20Event%20manifest%20binary%20format.asciidoc
 * and binary xml https://github.com/libyal/libevtx/blob/main/documentation/Windows%20XML%20Event%20Log%20(EVTX).asciidoc#4-binary-xml
 */

#[cfg(test)]
mod tests {
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_parse_template() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\pe\\resources\\wevt_template.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
    }
}
