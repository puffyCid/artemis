use crate::{
    artifacts::os::triage::{error::TriageError, reader::TriageReader},
    filesystem::metadata::glob_paths,
    structs::{
        artifacts::triage::{ArtemisTriage, Targets, TriageOptions},
        toml::{ArtemisToml, Output},
    },
    utils::encoding::base64_decode_standard,
};
use log::{error, warn};
use std::{
    fs::{File, create_dir_all},
    io::BufReader,
};
use zip::ZipWriter;

pub(crate) fn triage(output: &mut Output, options: &TriageOptions) -> Result<(), TriageError> {
    let triage = decode_triage(&options.triage)?;

    for target in triage.targets {
        if !target.recursive && !target.file_mask.starts_with("regex") {
            glob_files(&target, output)?;
            continue;
        }
    }
    Ok(())
}

fn decode_triage(encoded: &str) -> Result<ArtemisTriage, TriageError> {
    let bytes = match base64_decode_standard(encoded) {
        Ok(result) => result,
        Err(err) => {
            error!("[triage] Failed to base64 decode: {err:?}");
            return Err(TriageError::Decode);
        }
    };

    let triage = match ArtemisToml::parse_triage_toml(&bytes) {
        Ok(result) => result,
        Err(err) => {
            error!("[triage] Failed to parse triage toml: {err:?}");
            return Err(TriageError::Toml);
        }
    };

    Ok(triage)
}

fn glob_files(target: &Targets, output: &mut Output) -> Result<(), TriageError> {
    let glob_string = format!("{}{}", target.path, target.file_mask);
    let paths = glob_paths(&glob_string).unwrap_or_default();
    let zip_output = format!("{}/{}", output.directory, output.name);
    if let Err(err) = create_dir_all(&zip_output) {
        error!("[triage] Could not create output directory: {err:?}");
        return Err(TriageError::Output);
    }
    let zip_file = match File::create(format!("{zip_output}.zip")) {
        Ok(result) => result,
        Err(err) => {
            error!("[triage] Could not create zip file: {err:?}");
            return Err(TriageError::Output);
        }
    };
    let zip = ZipWriter::new(zip_file);

    let mut acq = TriageReader {
        fs: None,
        zip,
        path: String::new(),
    };
    for path in paths {
        if !path.is_file {
            continue;
        }
        println!("{}", path.full_path);

        let reader = match File::open(&path.full_path) {
            Ok(result) => result,
            Err(err) => {
                warn!("[triage] Could not read file {}: {err:?}", path.full_path);
                continue;
            }
        };
        let buf = BufReader::new(reader);
        acq.fs = Some(buf);
        acq.path = path.full_path;
        let hash = acq.acquire_file()?;
    }
    if let Err(err) = acq.zip.finish() {
        warn!("[triage] Failed to finish zipping file: {err:?}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::triage::artifact::{decode_triage, triage},
        structs::{artifacts::triage::TriageOptions, toml::Output},
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            endpoint_id: String::from("abcd"),
            output: output.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_triage() {
        let mut output = output_options("triage_test", "local", "./tmp", false);
        let options = TriageOptions {
            triage: String::from(
                "ZGVzY3JpcHRpb24gPSAiRFdBZ2VudCBMb2cgRmlsZXMiCmF1dGhvciA9ICJSb24gUmFkZXIiCnZlcnNpb24gPSAxLjAKaWQgPSAiZTc4ZDI2NTItZGY0Ny00ZWRjLWEwZWEtNDgzNWMzMjJhZDQ4IgpyZWNyZWF0ZV9kaXJlY3RvcmllcyA9IHRydWUKCltbdGFyZ2V0c11dCm5hbWUgPSAiRFdBZ2VudCBMb2cgRmlsZXMiCmNhdGVnb3J5ID0gIkxvZ3MiCnBhdGggPSAiQzpcXFByb2dyYW1EYXRhXFxEV0FnZW50KlxcIgpmaWxlX21hc2sgPSAiKi5sb2cqIgpyZWN1cnNpdmUgPSBmYWxzZQphbHdheXNfYWRkX3RvX3F1ZXVlID0gZmFsc2UK",
            ),
        };

        triage(&mut output, &options).unwrap();
    }

    #[test]
    fn test_triage_linux() {
        let mut output = output_options("triage_test", "local", "./tmp", false);
        let options = TriageOptions {
            triage: String::from(
                "ZGVzY3JpcHRpb24gPSAiQmFzaCBIaXN0b3J5IgphdXRob3IgPSAiUHVmZnlDaWQiCnZlcnNpb24gPSAxLjAKaWQgPSAiMTAxN2QyNGItYzdiMS00ZDRkLWI0MTYtMWIyM2E0NGRjNjMxIgpyZWNyZWF0ZV9kaXJlY3RvcmllcyA9IHRydWUKCltbdGFyZ2V0c11dCm5hbWUgPSAiRGVmYXVsdCBiYXNoIGxvY2F0aW9uIgpjYXRlZ29yeSA9ICJTaGVsbCIKcGF0aCA9ICIvaG9tZS8qLyIKZmlsZV9tYXNrID0gIiouYmFzaF9oaXN0b3J5IgpyZWN1cnNpdmUgPSBmYWxzZQphbHdheXNfYWRkX3RvX3F1ZXVlID0gZmFsc2UK",
            ),
        };

        triage(&mut output, &options).unwrap();
    }

    #[test]
    fn test_decode_triage() {
        let encoding = "ZGVzY3JpcHRpb24gPSAiRFdBZ2VudCBMb2cgRmlsZXMiCmF1dGhvciA9ICJSb24gUmFkZXIiCnZlcnNpb24gPSAxLjAKaWQgPSAiZTc4ZDI2NTItZGY0Ny00ZWRjLWEwZWEtNDgzNWMzMjJhZDQ4IgpyZWNyZWF0ZV9kaXJlY3RvcmllcyA9IHRydWUKCltbdGFyZ2V0c11dCm5hbWUgPSAiRFdBZ2VudCBMb2cgRmlsZXMiCmNhdGVnb3J5ID0gIkxvZ3MiCnBhdGggPSAiQzpcXFByb2dyYW1EYXRhXFxEV0FnZW50KlxcIgpmaWxlX21hc2sgPSAiKi5sb2cqIgpyZWN1cnNpdmUgPSBmYWxzZQphbHdheXNfYWRkX3RvX3F1ZXVlID0gZmFsc2UK";
        let triage = decode_triage(encoding).unwrap();
        assert_eq!(triage.author, "Ron Rader");
        assert_eq!(triage.targets[0].path, "C:\\ProgramData\\DWAgent*\\")
    }
}
