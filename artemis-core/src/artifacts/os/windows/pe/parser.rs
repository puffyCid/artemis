use crate::{filesystem::files::read_file, utils::encoding::base64_encode_standard};
use pelite::PeFile;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub(crate) struct PeInfo {
    pub(crate) imports: Vec<String>,
    pub(crate) sections: Vec<String>,
    pub(crate) cert: String,
    pub(crate) pdb: String,
    pub(crate) product_version: String,
    pub(crate) file_version: String,
    pub(crate) product_name: String,
    pub(crate) company_name: String,
    pub(crate) file_description: String,
    pub(crate) internal_name: String,
    pub(crate) legal_copyright: String,
    pub(crate) original_filename: String,
    pub(crate) manifest: String,
    pub(crate) icons: Vec<String>,
}

/// Read a `PE` file at provided path
pub(crate) fn parse_pe_file(path: &str) -> Result<PeInfo, pelite::Error> {
    let read_result = read_file(path);
    let data = match read_result {
        Ok(result) => result,
        Err(_) => return Err(pelite::Error::Overflow),
    };

    let mut info = PeInfo {
        imports: Vec::new(),
        sections: Vec::new(),
        cert: String::new(),
        pdb: String::new(),
        product_version: String::new(),
        file_version: String::new(),
        product_name: String::new(),
        company_name: String::new(),
        file_description: String::new(),
        internal_name: String::new(),
        legal_copyright: String::new(),
        original_filename: String::new(),
        manifest: String::new(),
        icons: Vec::new(),
    };

    let file_result = PeFile::from_bytes(&data);
    let file = match file_result {
        Ok(result) => result,
        Err(_) => return Err(pelite::Error::Invalid),
    };

    let imports_results = file.imports();
    if let Ok(imports) = imports_results {
        for import in imports.iter() {
            info.imports.push(import.dll_name()?.to_string());
        }
    }

    for section in file.section_headers().iter() {
        info.sections
            .push(section.name().unwrap_or_default().to_string());
    }

    let resources_result = file.resources();
    if let Ok(resources) = resources_result {
        let manifest_result = resources.manifest();
        if let Ok(result) = manifest_result {
            info.manifest = result.to_string();
        }

        let version_result = resources.version_info();
        if let Ok(result) = version_result {
            for (_, value) in result.file_info().strings {
                info.file_version = value
                    .get(&String::from("FileVersion"))
                    .unwrap_or(&String::new())
                    .to_string();
                info.company_name = value
                    .get(&String::from("CompanyName"))
                    .unwrap_or(&String::new())
                    .to_string();
                info.file_description = value
                    .get(&String::from("FileDescription"))
                    .unwrap_or(&String::new())
                    .to_string();
                info.original_filename = value
                    .get(&String::from("OriginalFilename"))
                    .unwrap_or(&String::new())
                    .to_string();
                info.internal_name = value
                    .get(&String::from("InternalName"))
                    .unwrap_or(&String::new())
                    .to_string();
                info.legal_copyright = value
                    .get(&String::from("LegalCopyright"))
                    .unwrap_or(&String::new())
                    .to_string();
                info.product_name = value
                    .get(&String::from("ProductName"))
                    .unwrap_or(&String::new())
                    .to_string();
                info.product_version = value
                    .get(&String::from("ProductVersion"))
                    .unwrap_or(&String::new())
                    .to_string();
            }
        }
        for icons in resources.icons() {
            let (_, icon_data) = match icons {
                Ok(result) => result,
                Err(_) => continue,
            };

            for entries in icon_data.entries() {
                let image_result = icon_data.image(entries.nId);
                let image = match image_result {
                    Ok(result) => result,
                    Err(_) => continue,
                };
                info.icons.push(base64_encode_standard(image));
            }
        }
    }

    let debug_result = file.debug();
    if let Ok(result) = debug_result {
        if let Some(pdb) = result.pdb_file_name() {
            info.pdb = pdb.to_string();
        }
    }

    let cert_result = file.security();
    if let Ok(result) = cert_result {
        info.cert = base64_encode_standard(result.certificate_data());
    }

    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::parse_pe_file;

    #[test]
    fn test_parse_pe_file() {
        let test = "C:\\Windows\\explorer.exe";
        let result = parse_pe_file(test).unwrap();
        assert_eq!(
            result.legal_copyright,
            "© Microsoft Corporation. All rights reserved."
        );
        assert!(result.imports.len() > 130);
        assert_eq!(result.pdb, "explorer.pdb");
    }
}
