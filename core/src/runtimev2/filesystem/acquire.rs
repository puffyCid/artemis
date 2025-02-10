use crate::{
    filesystem::acquire::{acquire_file, acquire_file_remote},
    output::files::remote::RemoteType,
    runtimev2::helper::{string_arg, value_arg},
    structs::toml::Output,
};
use boa_engine::{js_string, Context, JsError, JsResult, JsValue};
use log::error;

/// Acquire file from system
pub(crate) fn js_acquire_file(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let out = value_arg(args, &1, context)?;

    let output_result = serde_json::from_value(out);
    let output: Output = match output_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed deserialize output format data: {err:?}");
            return Err(JsError::from_opaque(
                js_string!(format!("Could not deserialize output format: {err:?}")).into(),
            ));
        }
    };

    if output.output == "local" {
        let status = acquire_file(&path, output);
        if status.is_err() {
            error!("[runtime] Failed to acquire file {path}");
            let err = js_string!(format!(
                "Could not acquire file {path}: {:?}",
                status.unwrap_err()
            ))
            .into();
            return Err(JsError::from_opaque(err));
        }
    } else if output.output == "gcp" {
        let status = acquire_file_remote(&path, output, RemoteType::Gcp);
        if status.is_err() {
            error!("[runtime] Failed to acquire file for upload {path}");
            let err = js_string!(format!(
                "Could not acquire file {path} for gcp upload: {:?}",
                status.unwrap_err()
            ))
            .into();
            return Err(JsError::from_opaque(err));
        }
    } else if output.output == "aws" {
        let status = acquire_file_remote(&path, output, RemoteType::Aws);
        if status.is_err() {
            error!("[runtime] Failed to acquire file for upload {path}");
            let err = js_string!(format!(
                "Could not acquire file {path} for aws upload: {:?}",
                status.unwrap_err()
            ))
            .into();
            return Err(JsError::from_opaque(err));
        }
    } else if output.output == "azure" {
        let status = acquire_file_remote(&path, output, RemoteType::Azure);
        if status.is_err() {
            error!("[runtime] Failed to acquire file for upload {path}");
            let err = js_string!(format!(
                "Could not acquire file {path} for azure upload: {:?}",
                status.unwrap_err()
            ))
            .into();
            return Err(JsError::from_opaque(err));
        }
    } else {
        return Err(JsError::from_opaque(
            js_string!(format!("Unknown acquire type {}", output.output)).into(),
        ));
    }

    let sucess = true;
    Ok(JsValue::Boolean(sucess))
}

#[cfg(test)]
mod tests {
    use crate::{
        runtimev2::run::execute_script,
        structs::{artifacts::runtime::script::JSScript, toml::Output},
        utils::{
            encoding::{base64_decode_standard, base64_encode_standard},
            strings::extract_utf8_string,
        },
    };
    use httpmock::MockServer;
    use serde_json::json;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

    #[test]
    fn test_js_acquire_file_local() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvRGVuby9hcnRlbWlzLWFwaS9zcmMvdXRpbHMvZXJyb3IudHMKdmFyIEVycm9yQmFzZSA9IGNsYXNzIGV4dGVuZHMgRXJyb3IgewogIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsKICAgIHN1cGVyKCk7CiAgICB0aGlzLm5hbWUgPSBuYW1lOwogICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsKICB9Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9EZW5vL2FydGVtaXMtYXBpL3NyYy9maWxlc3lzdGVtL2Vycm9ycy50cwp2YXIgRmlsZUVycm9yID0gY2xhc3MgZXh0ZW5kcyBFcnJvckJhc2Ugewp9OwoKLy8gLi4vLi4vUHJvamVjdHMvRGVuby9hcnRlbWlzLWFwaS9zcmMvZmlsZXN5c3RlbS9hY3F1aXJlLnRzCmZ1bmN0aW9uIGFjcXVpcmVGaWxlKHBhdGgsIG91dHB1dCkgewogIHRyeSB7CiAgICBjb25zdCBzdGF0dXMgPSBqc19hY3F1aXJlX2ZpbGUoCiAgICAgIHBhdGgsCiAgICAgIG91dHB1dAogICAgKTsKICAgIHJldHVybiBzdGF0dXM7CiAgfSBjYXRjaCAoZXJyKSB7CiAgICByZXR1cm4gbmV3IEZpbGVFcnJvcihgQUNRVUlSRWAsIGBmYWlsZWQgdG8gYWNxdWlyZSBmaWxlOiAke2Vycn1gKTsKICB9Cn0KCi8vIC4uLy4uL1Byb2plY3RzL0Rlbm8vYXJ0ZW1pcy1hcGkvc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gZ2xvYihwYXR0ZXJuKSB7CiAgdHJ5IHsKICAgIGNvbnN0IHJlc3VsdCA9IGpzX2dsb2IocGF0dGVybik7CiAgICByZXR1cm4gcmVzdWx0OwogIH0gY2F0Y2ggKGVycikgewogICAgcmV0dXJuIG5ldyBGaWxlRXJyb3IoIkdMT0IiLCBgZmFpbGVkIHRvIGdsb2IgcGF0dGVybiAke3BhdHRlcm59IiAke2Vycn1gKTsKICB9Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBvdXQgPSB7CiAgICBuYW1lOiAianNfYWNxdWlyZSIsCiAgICBkaXJlY3Rvcnk6ICIuL3RtcCIsCiAgICBmb3JtYXQ6ICJqc29uIiAvKiBKU09OICovLAogICAgY29tcHJlc3M6IGZhbHNlLAogICAgZW5kcG9pbnRfaWQ6ICJhZGJjZCIsCiAgICBjb2xsZWN0aW9uX2lkOiAwLAogICAgb3V0cHV0OiAibG9jYWwiIC8qIExPQ0FMICovCiAgfTsKICBjb25zdCBwYXRoID0gIi4uLyoiOwogIGNvbnN0IGdsb2JzID0gZ2xvYihwYXRoKTsKICBpZiAoZ2xvYnMgaW5zdGFuY2VvZiBGaWxlRXJyb3IpIHsKICAgIHJldHVybjsKICB9CiAgZm9yIChjb25zdCBlbnRyeSBvZiBnbG9icykgewogICAgaWYgKCFlbnRyeS5pc19maWxlKSB7CiAgICAgICBjb250aW51ZTsKICAgICB9CiAgICBjb25zdCBzdGF0dXMgPSBhY3F1aXJlRmlsZShlbnRyeS5mdWxsX3BhdGgsIG91dCk7CiAgICBjb25zb2xlLmxvZyhgYWNxIHN1Y2Nlc3M6ICR7c3RhdHVzfWApOwogICAgYnJlYWs7CiAgfQp9Cm1haW4oKTsKCgo=";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("acquire_result"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_acquire_file_gcp() {
        let server = MockServer::start();
        let port = server.port();

        let test = "Ly8gLi4vLi4vUHJvamVjdHMvRGVuby9hcnRlbWlzLWFwaS9zcmMvdXRpbHMvZXJyb3IudHMKdmFyIEVycm9yQmFzZSA9IGNsYXNzIGV4dGVuZHMgRXJyb3IgewogIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsKICAgIHN1cGVyKCk7CiAgICB0aGlzLm5hbWUgPSBuYW1lOwogICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsKICB9Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9EZW5vL2FydGVtaXMtYXBpL3NyYy9maWxlc3lzdGVtL2Vycm9ycy50cwp2YXIgRmlsZUVycm9yID0gY2xhc3MgZXh0ZW5kcyBFcnJvckJhc2Ugewp9OwoKLy8gLi4vLi4vUHJvamVjdHMvRGVuby9hcnRlbWlzLWFwaS9zcmMvZmlsZXN5c3RlbS9hY3F1aXJlLnRzCmZ1bmN0aW9uIGFjcXVpcmVGaWxlKHBhdGgsIG91dHB1dCkgewogIHRyeSB7CiAgICBjb25zdCBzdGF0dXMgPSBqc19hY3F1aXJlX2ZpbGUoCiAgICAgIHBhdGgsCiAgICAgIG91dHB1dAogICAgKTsKICAgIHJldHVybiBzdGF0dXM7CiAgfSBjYXRjaCAoZXJyKSB7CiAgICByZXR1cm4gbmV3IEZpbGVFcnJvcihgQUNRVUlSRWAsIGBmYWlsZWQgdG8gYWNxdWlyZSBmaWxlOiAke2Vycn1gKTsKICB9Cn0KCi8vIC4uLy4uL1Byb2plY3RzL0Rlbm8vYXJ0ZW1pcy1hcGkvc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gZ2xvYihwYXR0ZXJuKSB7CiAgdHJ5IHsKICAgIGNvbnN0IHJlc3VsdCA9IGpzX2dsb2IocGF0dGVybik7CiAgICByZXR1cm4gcmVzdWx0OwogIH0gY2F0Y2ggKGVycikgewogICAgcmV0dXJuIG5ldyBGaWxlRXJyb3IoIkdMT0IiLCBgZmFpbGVkIHRvIGdsb2IgcGF0dGVybiAke3BhdHRlcm59IiAke2Vycn1gKTsKICB9Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBvdXQgPSB7CiAgICBuYW1lOiAianNfYWNxdWlyZSIsCiAgICBkaXJlY3Rvcnk6ICIuL3RtcCIsCiAgICBmb3JtYXQ6ICJqc29uIiAvKiBKU09OICovLAogICAgY29tcHJlc3M6IGZhbHNlLAogICAgZW5kcG9pbnRfaWQ6ICJhZGJjZCIsCiAgICBjb2xsZWN0aW9uX2lkOiAwLAogICAgb3V0cHV0OiAiZ2NwIiAvKiBHQ1AgKi8sCiAgICB1cmw6ICJodHRwOi8vMTI3LjAuMC4xOlJFUExBQ0VQT1JUL2djcC1idWNrZXQiLAogICAgYXBpX2tleTogImV3b2dJQ0owZVhCbElqb2dJbk5sY25acFkyVmZZV05qYjNWdWRDSXNDaUFnSW5CeWIycGxZM1JmYVdRaU9pQWlabUZyWlcxbElpd0tJQ0FpY0hKcGRtRjBaVjlyWlhsZmFXUWlPaUFpWm1GclpXMWxJaXdLSUNBaWNISnBkbUYwWlY5clpYa2lPaUFpTFMwdExTMUNSVWRKVGlCUVVrbFdRVlJGSUV0RldTMHRMUzB0WEc1TlNVbEZkbmRKUWtGRVFVNUNaMnR4YUd0cFJ6bDNNRUpCVVVWR1FVRlRRMEpMYTNkbloxTnNRV2RGUVVGdlNVSkJVVU0zVmtwVVZYUTVWWE00WTB0cVRYcEZabGw1YW1sWFFUUlNOQzlOTW1KVE1VZENOSFEzVGxod09UaERNMU5ETm1SV1RYWkVkV2xqZEVkbGRYSlVPR3BPWW5aS1draDBRMU4xV1VWMmRVNU5iMU5tYlRjMmIzRkdka0Z3T0VkNU1HbDZOWE40YWxwdFUyNVllVU5rVUVWdmRrZG9UR0V3Vm5wTllWRTRjeXREVEU5NVV6VTJXWGxEUmtkbFNscHhaM1I2U2paSFVqTmxjVzlaVTFjNVlqbFZUWFpyUW5CYVQwUlRZM1JYVTA1SGFqTlFOMnBTUmtSUE5WWnZWSGREVVVGWFlrWnVUMnBFWmtnMVZXeG5jREpRUzFOUmJsTktVRE5CU2t4UlRrWk9aVGRpY2pGWVluSm9WaTh2WlU4cmREVXhiVWx3UjFORVExVjJNMFV3UkVSR1kxZEVWRWc1WTFoRVZGUnNVbHBXUldsU01rSjNjRnBQVDJ0Rkwxb3dMMEpXYm1oYVdVdzNNVzlhVmpNMFlrdG1WMnBSU1hRMlZpOXBjMU5OWVdoa2MwRkJVMEZEY0RSYVZFZDBkMmxXZFU1a09YUjVZa0ZuVFVKQlFVVkRaMmRGUWtGTFZHMXFZVk0yZEd0TE9FSnNVRmhEYkZSUk1uWndlaTlPTm5WNFJHVlRNelZ0V0hCeFlYTnhjMnRXYkdGQmFXUm5aeTl6VjNGd2FsaEVZbGh5T1ROdmRFbE5UR3hYYzAwcldEQkRjVTFFWjFOWVMyVnFURk15YW5nMFIwUnFTVEZhVkZobkt5c3dRVTFLT0hOS056UndWM3BXUkU5bWJVTkZVUzgzZDFoek15dGpZbTVZYUV0eWFVODRXakF6Tm5FNU1sRmpNU3RPT0RkVFNUTTRibXRIWVRCQlFrZzVRMDQ0TTBodFVYRjBOR1pDTjFWa1NIcDFTVkpsTDIxbE1sQkhhRWx4TlZwQ2VtbzJhRE5DY0c5UVIzcEZVQ3Q0TTJ3NVdXMUxPSFF2TVdOT01IQnhTU3RrVVhkWlpHZG1SMnBoWTJ0TWRTOHljVWc0TUUxRFJqZEplVkZoYzJWYVZVOUtlVXR5UTB4MFUwUXZTV2w0ZGk5b2VrUkZWVkJtVDBOcVJrUm5WSEI2WmpOamQzUmhPQ3R2UlRSM1NFTnZNV2xKTVM4MFZHeFFhM2R0V0hnMGNWTllkRzEzTkdGUlVIbzNTVVJSZGtWRFoxbEZRVGhMVGxSb1EwOHlaM05ETWtrNVVGRkVUUzg0UTNjd1R6azRNMWREUkZrcmIya3JOMHBRYVU1QlNuZDJOVVJaUW5GRldrSXhVVmxrYWpBMldVUXhObGhzUXk5SVFWcE5jMDFyZFRGdVlUSlVUakJrY21sM1pXNVJVVmQ2YjJWMk0yY3lVemRuVWtSdlV5OUdRMHBUU1ROcVNpdHJhbWQwWVVFM1VXMTZiR2RyTVZSNFQwUk9LMGN4U0RreFNGYzNkREJzTjFadVRESTNTVmQ1V1c4eWNWSlNTek5xZW5oeFZXbFFWVU5uV1VWQmVEQnZVWE15Y21WQ1VVZE5WbHB1UVhCRU1XcGxjVGR1TkUxMlRreGpVSFowT0dJdlpWVTVhVlYyTmxrMFRXb3dVM1Z2TDBGVk9HeFpXbGh0T0hWaVluRkJiSGQ2TWxaVFZuVnVSREowVDNCc1NIbE5WWEowUTNSUFlrRm1Wa1JWUVdoRGJtUkxZVUU1WjBGd1oyWmlNM2gzTVVsTFluVlJNWFUwU1VZeFJrcHNNMVowZFcxbVVXNHZMMHhwU0RGQ00zSllhR05rZVc4ekwzWkpkSFJGYXpRNFVtRnJWVXREYkZVNFEyZFpSVUY2VmpkWE0wTlBUMnhFUkdOUlpEa3pOVVJrZEV0Q1JsSkJVRkpRUVd4emNGRlZibnBOYVRWbFUwaE5SQzlKVTB4RVdUVkphVkZJWWtsSU9ETkVOR0oyV0hFd1dEZHhVVzlUUWxOT1VEZEVkbll6U0ZsMWNVMW9aakJFWVdWbmNteENkVXBzYkVaV1ZuRTVjVkJXVW01TGVIUXhTV3d5U0dkNFQwSjJZbWhQVkNzNWFXNHhRbnBCSzFsS09UbFZla000TlU4d1VYb3dOa0VyUTIxMFNFVjVOR0ZhTW10cU5XaElha1ZEWjFsRlFXMU9VelFyUVRoR2EzTnpPRXB6TVZKcFpVc3lURzVwUW5oTloyMVpiV3d6Y0daV1RFdEhibnB0Ym1jM1NESXJZM2RRVEdoUVNYcEpkWGQ1ZEZoNWQyZ3lZbnBpYzFsRlpsbDRNMFZ2UlZablRVVndVR2h2WVhKUmJsbFFkV3R5U2s4MFozZEZNbTgxVkdVMlZEVnRTbE5hUjJ4UlNsRnFPWEUwV2tJeVJHWjZaWFEyU1U1elN6QnZSemhZVmtkWVUzQlJkbEZvTTFKVldXVnJRMXBSYTBKQ1JtTndjVmR3WWtsRmMwTm5XVUZ1VFRORVVXWXpSa3B2VTI1WVlVMW9jbFpDU1c5MmFXTTFiREI0Um10RlNITnJRV3BHVkdWMlR6ZzJSbk42TVVNeVlWTmxVa3RUY1VkR2IwOVJNSFJ0U25wQ1JYTXhValpMY1c1SVNXNXBZMFJVVVhKTGFFRnlaMHhZV0RSMk0wTmtaR3BtVkZKS2EwWlhSR0pGTDBOcmRrdGFUazl5WTJZeGJtaGhSME5RYzNCU1Ntb3lTMVZyYWpGR2FHdzVRMjVqWkc0dlVuTlpSVTlPWW5kUlUycEpaazFRYTNaNFJpczRTRkU5UFZ4dUxTMHRMUzFGVGtRZ1VGSkpWa0ZVUlNCTFJWa3RMUzB0TFZ4dUlpd0tJQ0FpWTJ4cFpXNTBYMlZ0WVdsc0lqb2dJbVpoYTJWQVozTmxjblpwWTJWaFkyTnZkVzUwTG1OdmJTSXNDaUFnSW1Oc2FXVnVkRjlwWkNJNklDSm1ZV3RsYldVaUxBb2dJQ0poZFhSb1gzVnlhU0k2SUNKb2RIUndjem92TDJGalkyOTFiblJ6TG1kdmIyZHNaUzVqYjIwdmJ5OXZZWFYwYURJdllYVjBhQ0lzQ2lBZ0luUnZhMlZ1WDNWeWFTSTZJQ0pvZEhSd2N6b3ZMMjloZFhSb01pNW5iMjluYkdWaGNHbHpMbU52YlM5MGIydGxiaUlzQ2lBZ0ltRjFkR2hmY0hKdmRtbGtaWEpmZURVd09WOWpaWEowWDNWeWJDSTZJQ0pvZEhSd2N6b3ZMM2QzZHk1bmIyOW5iR1ZoY0dsekxtTnZiUzl2WVhWMGFESXZkakV2WTJWeWRITWlMQW9nSUNKamJHbGxiblJmZURVd09WOWpaWEowWDNWeWJDSTZJQ0pvZEhSd2N6b3ZMM2QzZHk1bmIyOW5iR1ZoY0dsekxtTnZiUzl5YjJKdmRDOTJNUzl0WlhSaFpHRjBZUzk0TlRBNUwyWmhhMlZ0WlNJc0NpQWdJblZ1YVhabGNuTmxYMlJ2YldGcGJpSTZJQ0puYjI5bmJHVmhjR2x6TG1OdmJTSUtmUW89IgogIH07CgogIGNvbnN0IHBhdGggPSAiLi4vKiI7CiAgY29uc3QgcmVzdWx0cyA9IGdsb2IocGF0aCk7CiAgaWYgKHJlc3VsdHMgaW5zdGFuY2VvZiBGaWxlRXJyb3IpIHsKICAgIHJldHVybjsKICB9CiAgZm9yIChjb25zdCBlbnRyeSBvZiByZXN1bHRzKSB7CiAgICBpZiAoIWVudHJ5LmlzX2ZpbGUpIHsKICAgICAgY29udGludWU7CiAgICB9CiAgICBjb25zdCBzdGF0dXMgPSBhY3F1aXJlRmlsZShlbnRyeS5mdWxsX3BhdGgsIG91dCk7CiAgICBjb25zb2xlLmxvZyhgYWNxIHN1Y2Nlc3MgYW5kIHVwbG9hZCBzdWNjZXNzOiAke3N0YXR1c31gKTsKICAgIGJyZWFrOwogIH0KfQptYWluKCk7Cgo=";
        let data = base64_decode_standard(&test).unwrap();
        let temp_script = extract_utf8_string(&data).replace("REPLACEPORT", &format!("{port}"));
        let update_script = base64_encode_standard(temp_script.as_bytes());

        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("acquire_gcp_result"),
            script: update_script.to_string(),
        };

        let mock_me = server.mock(|when, then| {
            when.any_request();
            then.status(200)
                .header("content-type", "application/json")
                .header("Location", format!("http://127.0.0.1:{port}"))
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });
        execute_script(&mut output, &script).unwrap();
        mock_me.assert_hits(5);
    }
}
