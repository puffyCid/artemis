use crate::{
    filesystem::acquire::{acquire_file, acquire_file_remote},
    structs::toml::Output,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2(fast)]
/// Acquire file from system
pub(crate) fn js_acquire_file(
    #[string] path: String,
    #[string] output_format: String,
) -> Result<bool, AnyError> {
    let sucess = true;

    let output_result = serde_json::from_str(&output_format);
    let output: Output = match output_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed deserialize output format data: {err:?}");
            return Err(err.into());
        }
    };

    if output.output == "local" {
        let status = acquire_file(&path, output);
        if status.is_err() {
            error!("[runtime] Failed to acquire file {path}");
            return Err(status.unwrap_err().into());
        }
    } else if output.output == "gcp" {
        let status = acquire_file_remote(&path, output);
        if status.is_err() {
            error!("[runtime] Failed to acquire file for upload{path}");
            return Err(status.unwrap_err().into());
        }
    } else {
        return Err(AnyError::msg(format!(
            "Unknown acquire type: {}",
            output.output
        )));
    }

    Ok(sucess)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::deno::execute_script,
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
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvRGVuby9hcnRlbWlzLWFwaS9zcmMvdXRpbHMvZXJyb3IudHMKdmFyIEVycm9yQmFzZSA9IGNsYXNzIGV4dGVuZHMgRXJyb3IgewogIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsKICAgIHN1cGVyKCk7CiAgICB0aGlzLm5hbWUgPSBuYW1lOwogICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsKICB9Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9EZW5vL2FydGVtaXMtYXBpL3NyYy9maWxlc3lzdGVtL2Vycm9ycy50cwp2YXIgRmlsZUVycm9yID0gY2xhc3MgZXh0ZW5kcyBFcnJvckJhc2Ugewp9OwoKLy8gLi4vLi4vUHJvamVjdHMvRGVuby9hcnRlbWlzLWFwaS9zcmMvZmlsZXN5c3RlbS9hY3F1aXJlLnRzCmZ1bmN0aW9uIGFjcXVpcmVGaWxlKHBhdGgsIG91dHB1dCkgewogIHRyeSB7CiAgICBjb25zdCBvdXRwdXRfc3RyaW5nID0gSlNPTi5zdHJpbmdpZnkob3V0cHV0KTsKICAgIGNvbnN0IHN0YXR1cyA9IERlbm8uY29yZS5vcHMuanNfYWNxdWlyZV9maWxlKAogICAgICBwYXRoLAogICAgICBvdXRwdXRfc3RyaW5nCiAgICApOwogICAgcmV0dXJuIHN0YXR1czsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgRmlsZUVycm9yKGBBQ1FVSVJFYCwgYGZhaWxlZCB0byBhY3F1aXJlIGZpbGU6ICR7ZXJyfWApOwogIH0KfQoKLy8gLi4vLi4vUHJvamVjdHMvRGVuby9hcnRlbWlzLWFwaS9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiBnbG9iKHBhdHRlcm4pIHsKICB0cnkgewogICAgY29uc3QgcmVzdWx0ID0gZnMuZ2xvYihwYXR0ZXJuKTsKICAgIGNvbnN0IGRhdGEgPSBKU09OLnBhcnNlKHJlc3VsdCk7CiAgICByZXR1cm4gZGF0YTsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgRmlsZUVycm9yKCJHTE9CIiwgYGZhaWxlZCB0byBnbG9iIHBhdHRlcm4gJHtwYXR0ZXJufSIgJHtlcnJ9YCk7CiAgfQp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3Qgb3V0ID0gewogICAgbmFtZTogImpzX2FjcXVpcmUiLAogICAgZGlyZWN0b3J5OiAiLi90bXAiLAogICAgZm9ybWF0OiAianNvbiIgLyogSlNPTiAqLywKICAgIGNvbXByZXNzOiBmYWxzZSwKICAgIGVuZHBvaW50X2lkOiAiYWRiY2QiLAogICAgY29sbGVjdGlvbl9pZDogMCwKICAgIG91dHB1dDogImxvY2FsIiAvKiBMT0NBTCAqLwogIH07CiAgY29uc3QgcGF0aCA9ICIuLi8qIjsKICBjb25zdCBnbG9icyA9IGdsb2IocGF0aCk7CiAgaWYgKGdsb2JzIGluc3RhbmNlb2YgRmlsZUVycm9yKSB7CiAgICByZXR1cm47CiAgfQogIGZvciAoY29uc3QgZW50cnkgb2YgZ2xvYnMpIHsKICAgIGNvbnN0IHN0YXR1cyA9IGFjcXVpcmVGaWxlKGVudHJ5LmZ1bGxfcGF0aCwgb3V0KTsKICAgIGNvbnNvbGUubG9nKHN0YXR1cyk7CiAgICBicmVhazsKICB9Cn0KbWFpbigpOwo=";
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

        let test = "Ly8gLi4vLi4vUHJvamVjdHMvRGVuby9hcnRlbWlzLWFwaS9zcmMvdXRpbHMvZXJyb3IudHMKdmFyIEVycm9yQmFzZSA9IGNsYXNzIGV4dGVuZHMgRXJyb3IgewogIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsKICAgIHN1cGVyKCk7CiAgICB0aGlzLm5hbWUgPSBuYW1lOwogICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsKICB9Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9EZW5vL2FydGVtaXMtYXBpL3NyYy9maWxlc3lzdGVtL2Vycm9ycy50cwp2YXIgRmlsZUVycm9yID0gY2xhc3MgZXh0ZW5kcyBFcnJvckJhc2Ugewp9OwoKLy8gLi4vLi4vUHJvamVjdHMvRGVuby9hcnRlbWlzLWFwaS9zcmMvZmlsZXN5c3RlbS9hY3F1aXJlLnRzCmZ1bmN0aW9uIGFjcXVpcmVGaWxlKHBhdGgsIG91dHB1dCkgewogIHRyeSB7CiAgICBjb25zdCBvdXRwdXRfc3RyaW5nID0gSlNPTi5zdHJpbmdpZnkob3V0cHV0KTsKICAgIGNvbnN0IHN0YXR1cyA9IERlbm8uY29yZS5vcHMuanNfYWNxdWlyZV9maWxlKAogICAgICBwYXRoLAogICAgICBvdXRwdXRfc3RyaW5nCiAgICApOwogICAgcmV0dXJuIHN0YXR1czsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgRmlsZUVycm9yKGBBQ1FVSVJFYCwgYGZhaWxlZCB0byBhY3F1aXJlIGZpbGU6ICR7ZXJyfWApOwogIH0KfQoKLy8gLi4vLi4vUHJvamVjdHMvRGVuby9hcnRlbWlzLWFwaS9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiBnbG9iKHBhdHRlcm4pIHsKICB0cnkgewogICAgY29uc3QgcmVzdWx0ID0gZnMuZ2xvYihwYXR0ZXJuKTsKICAgIGNvbnN0IGRhdGEgPSBKU09OLnBhcnNlKHJlc3VsdCk7CiAgICByZXR1cm4gZGF0YTsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgRmlsZUVycm9yKCJHTE9CIiwgYGZhaWxlZCB0byBnbG9iIHBhdHRlcm4gJHtwYXR0ZXJufSIgJHtlcnJ9YCk7CiAgfQp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3Qgb3V0ID0gewogICAgbmFtZTogImpzX2FjcXVpcmUiLAogICAgZGlyZWN0b3J5OiAiLi90bXAiLAogICAgZm9ybWF0OiAianNvbiIgLyogSlNPTiAqLywKICAgIGNvbXByZXNzOiBmYWxzZSwKICAgIGVuZHBvaW50X2lkOiAiYWRiY2QiLAogICAgY29sbGVjdGlvbl9pZDogMCwKICAgIG91dHB1dDogImdjcCIgLyogR0NQICovLAogICAgdXJsOiAiaHR0cDovLzEyNy4wLjAuMTpSRVBMQUNFUE9SVC9nY3AtYnVja2V0IiwKICAgIGFwaV9rZXk6ICJld29nSUNKMGVYQmxJam9nSW5ObGNuWnBZMlZmWVdOamIzVnVkQ0lzQ2lBZ0luQnliMnBsWTNSZmFXUWlPaUFpWm1GclpXMWxJaXdLSUNBaWNISnBkbUYwWlY5clpYbGZhV1FpT2lBaVptRnJaVzFsSWl3S0lDQWljSEpwZG1GMFpWOXJaWGtpT2lBaUxTMHRMUzFDUlVkSlRpQlFVa2xXUVZSRklFdEZXUzB0TFMwdFhHNU5TVWxGZG5kSlFrRkVRVTVDWjJ0eGFHdHBSemwzTUVKQlVVVkdRVUZUUTBKTGEzZG5aMU5zUVdkRlFVRnZTVUpCVVVNM1ZrcFVWWFE1VlhNNFkwdHFUWHBGWmxsNWFtbFhRVFJTTkM5Tk1tSlRNVWRDTkhRM1RsaHdPVGhETTFORE5tUldUWFpFZFdsamRFZGxkWEpVT0dwT1luWktXa2gwUTFOMVdVVjJkVTVOYjFObWJUYzJiM0ZHZGtGd09FZDVNR2w2TlhONGFscHRVMjVZZVVOa1VFVnZka2RvVEdFd1ZucE5ZVkU0Y3l0RFRFOTVVelUyV1hsRFJrZGxTbHB4WjNSNlNqWkhVak5sY1c5WlUxYzVZamxWVFhaclFuQmFUMFJUWTNSWFUwNUhhak5RTjJwU1JrUlBOVlp2VkhkRFVVRlhZa1p1VDJwRVprZzFWV3huY0RKUVMxTlJibE5LVUROQlNreFJUa1pPWlRkaWNqRllZbkpvVmk4dlpVOHJkRFV4YlVsd1IxTkVRMVYyTTBVd1JFUkdZMWRFVkVnNVkxaEVWRlJzVWxwV1JXbFNNa0ozY0ZwUFQydEZMMW93TDBKV2JtaGFXVXczTVc5YVZqTTBZa3RtVjJwUlNYUTJWaTlwYzFOTllXaGtjMEZCVTBGRGNEUmFWRWQwZDJsV2RVNWtPWFI1WWtGblRVSkJRVVZEWjJkRlFrRkxWRzFxWVZNMmRHdExPRUpzVUZoRGJGUlJNblp3ZWk5T05uVjRSR1ZUTXpWdFdIQnhZWE54YzJ0V2JHRkJhV1JuWnk5elYzRndhbGhFWWxoeU9UTnZkRWxOVEd4WGMwMHJXREJEY1UxRVoxTllTMlZxVEZNeWFuZzBSMFJxU1RGYVZGaG5LeXN3UVUxS09ITktOelJ3VjNwV1JFOW1iVU5GVVM4M2QxaHpNeXRqWW01WWFFdHlhVTg0V2pBek5uRTVNbEZqTVN0T09EZFRTVE00Ym10SFlUQkJRa2c1UTA0NE0waHRVWEYwTkdaQ04xVmtTSHAxU1ZKbEwyMWxNbEJIYUVseE5WcENlbW8yYUROQ2NHOVFSM3BGVUN0NE0ydzVXVzFMT0hRdk1XTk9NSEJ4U1N0a1VYZFpaR2RtUjJwaFkydE1kUzh5Y1VnNE1FMURSamRKZVZGaGMyVmFWVTlLZVV0eVEweDBVMFF2U1dsNGRpOW9la1JGVlZCbVQwTnFSa1JuVkhCNlpqTmpkM1JoT0N0dlJUUjNTRU52TVdsSk1TODBWR3hRYTNkdFdIZzBjVk5ZZEcxM05HRlJVSG8zU1VSUmRrVkRaMWxGUVRoTFRsUm9RMDh5WjNORE1razVVRkZFVFM4NFEzY3dUems0TTFkRFJGa3JiMmtyTjBwUWFVNUJTbmQyTlVSWlFuRkZXa0l4VVZsa2FqQTJXVVF4Tmxoc1F5OUlRVnBOYzAxcmRURnVZVEpVVGpCa2NtbDNaVzVSVVZkNmIyVjJNMmN5VXpkblVrUnZVeTlHUTBwVFNUTnFTaXRyYW1kMFlVRTNVVzE2Ykdkck1WUjRUMFJPSzBjeFNEa3hTRmMzZERCc04xWnVUREkzU1ZkNVdXOHljVkpTU3pOcWVuaHhWV2xRVlVObldVVkJlREJ2VVhNeWNtVkNVVWROVmxwdVFYQkVNV3BsY1RkdU5FMTJUa3hqVUhaME9HSXZaVlU1YVZWMk5sazBUV293VTNWdkwwRlZPR3haV2xodE9IVmlZbkZCYkhkNk1sWlRWblZ1UkRKMFQzQnNTSGxOVlhKMFEzUlBZa0ZtVmtSVlFXaERibVJMWVVFNVowRndaMlppTTNoM01VbExZblZSTVhVMFNVWXhSa3BzTTFaMGRXMW1VVzR2TDB4cFNERkNNM0pZYUdOa2VXOHpMM1pKZEhSRmF6UTRVbUZyVlV0RGJGVTRRMmRaUlVGNlZqZFhNME5QVDJ4RVJHTlJaRGt6TlVSa2RFdENSbEpCVUZKUVFXeHpjRkZWYm5wTmFUVmxVMGhOUkM5SlUweEVXVFZKYVZGSVlrbElPRE5FTkdKMldIRXdXRGR4VVc5VFFsTk9VRGRFZG5ZelNGbDFjVTFvWmpCRVlXVm5jbXhDZFVwc2JFWldWbkU1Y1ZCV1VtNUxlSFF4U1d3eVNHZDRUMEoyWW1oUFZDczVhVzR4UW5wQksxbEtPVGxWZWtNNE5VOHdVWG93TmtFclEyMTBTRVY1TkdGYU1tdHFOV2hJYWtWRFoxbEZRVzFPVXpRclFUaEdhM056T0Vwek1WSnBaVXN5VEc1cFFuaE5aMjFaYld3emNHWldURXRIYm5wdGJtYzNTRElyWTNkUVRHaFFTWHBKZFhkNWRGaDVkMmd5WW5waWMxbEZabGw0TTBWdlJWWm5UVVZ3VUdodllYSlJibGxRZFd0eVNrODBaM2RGTW04MVZHVTJWRFZ0U2xOYVIyeFJTbEZxT1hFMFdrSXlSR1o2WlhRMlNVNXpTekJ2UnpoWVZrZFlVM0JSZGxGb00xSlZXV1ZyUTFwUmEwSkNSbU53Y1Zkd1lrbEZjME5uV1VGdVRUTkVVV1l6UmtwdlUyNVlZVTFvY2xaQ1NXOTJhV00xYkRCNFJtdEZTSE5yUVdwR1ZHVjJUemcyUm5ONk1VTXlZVk5sVWt0VGNVZEdiMDlSTUhSdFNucENSWE14VWpaTGNXNUlTVzVwWTBSVVVYSkxhRUZ5WjB4WVdEUjJNME5rWkdwbVZGSkthMFpYUkdKRkwwTnJka3RhVGs5eVkyWXhibWhoUjBOUWMzQlNTbW95UzFWcmFqRkdhR3c1UTI1alpHNHZVbk5aUlU5T1luZFJVMnBKWmsxUWEzWjRSaXM0U0ZFOVBWeHVMUzB0TFMxRlRrUWdVRkpKVmtGVVJTQkxSVmt0TFMwdExWeHVJaXdLSUNBaVkyeHBaVzUwWDJWdFlXbHNJam9nSW1aaGEyVkFaM05sY25acFkyVmhZMk52ZFc1MExtTnZiU0lzQ2lBZ0ltTnNhV1Z1ZEY5cFpDSTZJQ0ptWVd0bGJXVWlMQW9nSUNKaGRYUm9YM1Z5YVNJNklDSm9kSFJ3Y3pvdkwyRmpZMjkxYm5SekxtZHZiMmRzWlM1amIyMHZieTl2WVhWMGFESXZZWFYwYUNJc0NpQWdJblJ2YTJWdVgzVnlhU0k2SUNKb2RIUndjem92TDI5aGRYUm9NaTVuYjI5bmJHVmhjR2x6TG1OdmJTOTBiMnRsYmlJc0NpQWdJbUYxZEdoZmNISnZkbWxrWlhKZmVEVXdPVjlqWlhKMFgzVnliQ0k2SUNKb2RIUndjem92TDNkM2R5NW5iMjluYkdWaGNHbHpMbU52YlM5dllYVjBhREl2ZGpFdlkyVnlkSE1pTEFvZ0lDSmpiR2xsYm5SZmVEVXdPVjlqWlhKMFgzVnliQ0k2SUNKb2RIUndjem92TDNkM2R5NW5iMjluYkdWaGNHbHpMbU52YlM5eWIySnZkQzkyTVM5dFpYUmhaR0YwWVM5NE5UQTVMMlpoYTJWdFpTSXNDaUFnSW5WdWFYWmxjbk5sWDJSdmJXRnBiaUk2SUNKbmIyOW5iR1ZoY0dsekxtTnZiU0lLZlFvPSIKICB9OwoKICBjb25zdCBwYXRoID0gIi4uLyoiOwogIGNvbnN0IHJlc3VsdHMgPSBnbG9iKHBhdGgpOwogIGlmIChyZXN1bHRzIGluc3RhbmNlb2YgRmlsZUVycm9yKSB7CiAgICByZXR1cm47CiAgfQogIGZvciAoY29uc3QgZW50cnkgb2YgcmVzdWx0cykgewogICAgaWYgKCFlbnRyeS5pc19maWxlKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgY29uc3Qgc3RhdHVzID0gYWNxdWlyZUZpbGUoZW50cnkuZnVsbF9wYXRoLCBvdXQpOwogICAgY29uc29sZS5sb2coc3RhdHVzKTsKICAgIGJyZWFrOwogIH0KfQptYWluKCk7Cg==";
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
