use crate::{artifacts::os::windows::shellitems::items::get_shellitem, runtime::helper::bytes_arg};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};
use common::windows::ShellItem;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct JsShellitem {
    item: ShellItem,
    remaining: Vec<u8>,
}

/// Parse raw shellitem bytes and return remaining bytes
pub(crate) fn js_shellitems(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let bytes = bytes_arg(args, 0, context)?;
    let results = get_shellitem(&bytes);
    let (remaining, item) = match results {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get shellitems: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let data = JsShellitem {
        item,
        remaining: remaining.to_vec(),
    };
    let result = serde_json::to_value(data).unwrap_or_default();
    let value = JsValue::from_json(&result, context)?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::run::execute_script,
        structs::{artifacts::runtime::script::JSScript, toml::Output},
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            timeline: false,
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

    #[tokio::test]
    async fn test_js_shellitems() {
        let test = "dmFyIGw9Y2xhc3MgZXh0ZW5kcyBFcnJvcntjb25zdHJ1Y3RvcihuLG8pe3N1cGVyKCk7dGhpcy5uYW1lPW4sdGhpcy5tZXNzYWdlPW99fTt2YXIgcz1jbGFzcyBleHRlbmRzIGx7fTtmdW5jdGlvbiBwKGUpe3RyeXtyZXR1cm4ganNfZ2xvYihlKX1jYXRjaCh0KXtyZXR1cm4gbmV3IHMoIkdMT0IiLGBmYWlsZWQgdG8gZ2xvYiBwYXR0ZXJuICR7ZX0iICR7dH1gKX19dmFyIHU9Y2xhc3MgZXh0ZW5kcyBse307ZnVuY3Rpb24gaChlKXt0cnl7cmV0dXJuIGpzX2Jhc2U2NF9kZWNvZGUoZSl9Y2F0Y2godCl7cmV0dXJuIG5ldyB1KCJCQVNFNjQiLGBmYWlsZWQgdG8gZGVjb2RlICR7ZX06ICR7dH1gKX19dmFyIGE9Y2xhc3MgZXh0ZW5kcyBse307ZnVuY3Rpb24gQyhlKXtyZXR1cm4ganNfZXh0cmFjdF91dGYxNl9zdHJpbmcoZSl9ZnVuY3Rpb24gYihlLHQpe2lmKHQ8MClyZXR1cm4gbmV3IGEoIk5PTSIsInByb3ZpZGVkIG5lZ2F0aXZlIG51bWJlciIpO2lmKGUubGVuZ3RoPHQpcmV0dXJuIG5ldyBhKCJOT00iLGB3YW50ZWQgJHt0fSBidXQgaW5wdXQgaXMgJHtlLmxlbmd0aH1gKTtsZXQgbj1lLnNsaWNlKDAsdCk7cmV0dXJue3JlbWFpbmluZzplLnNsaWNlKHQpLG5vbW1lZDpufX1mdW5jdGlvbiB2KGUsdCl7aWYodHlwZW9mIGUhPXR5cGVvZiB0KXJldHVybiBuZXcgYSgiTk9NIiwiZGF0YSBhbmQgaW5wdXQgbXVzdCBiZSBtYXRjaGluZyB0eXBlcyIpO2lmKHR5cGVvZiBlPT0ic3RyaW5nIiYmdHlwZW9mIHQ9PSJzdHJpbmciKXRyeXtyZXR1cm4ganNfbm9tX3Rha2VfdW50aWxfc3RyaW5nKGUsdCl9Y2F0Y2gobil7cmV0dXJuIG5ldyBhKCJOT00iLGBjb3VsZCBub3QgdGFrZSB1bnRpbCBzdHJpbmcgJHtufWApfXRyeXtsZXQgbj1qc19ub21fdGFrZV91bnRpbF9ieXRlcyhlLHQpO3JldHVybntub21tZWQ6bixyZW1haW5pbmc6ZS5zbGljZShuLmJ1ZmZlci5ieXRlTGVuZ3RoKX19Y2F0Y2gobil7cmV0dXJuIG5ldyBhKCJOT00iLGBjb3VsZCBub3QgdGFrZSB1bnRpbCBieXRlcyAke259YCl9fXZhciByPWNsYXNzIGV4dGVuZHMgbHt9O2Z1bmN0aW9uIFAoZSl7dHJ5e3JldHVybiBqc19yZWdpc3RyeShlKX1jYXRjaCh0KXtyZXR1cm4gbmV3IHIoIlJFR0lTVFJZIixgZmFpbGVkIHRvIHBhcnNlIHJlZ2lzdHJ5IGZpbGUgJHtlfTogJHt0fWApfX1mdW5jdGlvbiAkKGUpe3RyeXtsZXQgdD1qc19zaGVsbGl0ZW1zKGUpO3JldHVybntpdGVtOnQuaXRlbSxyZW1haW5pbmc6bmV3IFVpbnQ4QXJyYXkodC5yZW1haW5pbmcpfX1jYXRjaCh0KXtyZXR1cm4gbmV3IHIoIlNIRUxMSVRFTVMiLGBmYWlsZWQgdG8gZ2V0IHNoZWxsaXRlbXM6ICR7dH1gKX19ZnVuY3Rpb24geihlKXtsZXQgdD1bXTtmb3IobGV0IG8gb2YgZSkhby5wYXRoLmluY2x1ZGVzKCJcXFNvZnR3YXJlXFxNaWNyb3NvZnRcXFdpbmRvd3NcXEN1cnJlbnRWZXJzaW9uXFxFeHBsb3JlclxcQ29tRGxnMzJcXE9wZW5TYXZlUGlkbE1SVSIpfHx0LnB1c2gobyk7bGV0IG49W107Zm9yKGxldCBvIG9mIHQpZm9yKGxldCBjIG9mIG8udmFsdWVzKXtpZihjLnZhbHVlPT09Ik1SVUxpc3RFeCIpY29udGludWU7bGV0IGY9aChjLmRhdGEpO2lmKGYgaW5zdGFuY2VvZiB1KXtjb25zb2xlLmVycm9yKGBjb3VsZCBub3QgZGVjb2RlIE9wZW5TYXZlIGtleSAke2MudmFsdWV9OiAke2Z9YCk7Y29udGludWV9bGV0IGQ9W107Zm9yKDtmLmxlbmd0aCE9MDspe2xldCBtPSQoZik7aWYobSBpbnN0YW5jZW9mIHIpe2NvbnNvbGUuZXJyb3IoYGNvdWxkIG5vdCBwYXJzZSBPcGVuU2F2ZSBzaGVsbGl0ZW0gZm9yICR7Yy52YWx1ZX06ICR7bX1gKTticmVha31mPW0ucmVtYWluaW5nLGQucHVzaChtLml0ZW0pfWxldCBfPUQoZCk7bi5wdXNoKF8pfXJldHVybiBufWZ1bmN0aW9uIEsoZSl7bGV0IHQ9W107Zm9yKGxldCBvIG9mIGUpIW8ucGF0aC5pbmNsdWRlcygiXFxTb2Z0d2FyZVxcTWljcm9zb2Z0XFxXaW5kb3dzXFxDdXJyZW50VmVyc2lvblxcRXhwbG9yZXJcXENvbURsZzMyXFxMYXN0VmlzaXRlZFBpZGxNUlUiKXx8dC5wdXNoKG8pO2xldCBuPVtdO2ZvcihsZXQgbyBvZiB0KWZvcihsZXQgYyBvZiBvLnZhbHVlcyl7aWYoYy52YWx1ZT09PSJNUlVMaXN0RXgiKWNvbnRpbnVlO2xldCBmPWgoYy5kYXRhKTtpZihmIGluc3RhbmNlb2YgdSl7Y29uc29sZS5lcnJvcihgY291bGQgbm90IGRlY29kZSBMYXN0VmlzaXRlZCBrZXkgJHtjLnZhbHVlfTogJHtmfWApO2NvbnRpbnVlfWxldCBkPXYoZixuZXcgVWludDhBcnJheShbMCwwLDBdKSk7aWYoZCBpbnN0YW5jZW9mIGEpe2NvbnNvbGUuZXJyb3IoYGNvdWxkIG5vdCBub20gVVRGMTYgZmlsZW5hbWU6ICR7ZH1gKTtjb250aW51ZX1sZXQgXz1iKGQucmVtYWluaW5nLDMpO2lmKF8gaW5zdGFuY2VvZiBhKXtjb25zb2xlLmVycm9yKGBjb3VsZCBub3Qgbm9tIGVuZCBvZiBieXRlIHN0cmluZzogJHtkfWApO2NvbnRpbnVlfWxldCBtPV8ucmVtYWluaW5nLE49QyhkLm5vbW1lZCksST1bXTtmb3IoO20ubGVuZ3RoIT0wOyl7bGV0IHk9JChtKTtpZih5IGluc3RhbmNlb2Ygcil7Y29uc29sZS5lcnJvcihgY291bGQgbm90IHBhcnNlIExhc3RWaXNpdGVkIHNoZWxsaXRlbSBmb3IgJHtjLnZhbHVlfTogJHt5fWApO2JyZWFrfW09eS5yZW1haW5pbmcsSS5wdXNoKHkuaXRlbSl9bGV0IEw9RChJKTtMLmZpbGVuYW1lPU4sbi5wdXNoKEwpfXJldHVybiBufWZ1bmN0aW9uIFEoZSl7bGV0IHQ9W107Zm9yKGxldCBvIG9mIGUpIW8ucGF0aC5pbmNsdWRlcygiXFxTb2Z0d2FyZVxcTWljcm9zb2Z0XFxXaW5kb3dzXFxDdXJyZW50VmVyc2lvblxcRXhwbG9yZXJcXFJlY2VudERvY3MiKXx8dC5wdXNoKG8pO2xldCBuPVtdO2ZvcihsZXQgbyBvZiB0KWZvcihsZXQgYyBvZiBvLnZhbHVlcyl7aWYoYy52YWx1ZT09PSJNUlVMaXN0RXgiKWNvbnRpbnVlO2xldCBmPWgoYy5kYXRhKTtpZihmIGluc3RhbmNlb2YgdSl7Y29uc29sZS5lcnJvcihgY291bGQgbm90IGRlY29kZSByZWNlbnQgZG9jcyBrZXkgJHtjLnZhbHVlfTogJHtmfWApO2NvbnRpbnVlfWxldCBkPXYoZixuZXcgVWludDhBcnJheShbMCwwLDBdKSk7aWYoZCBpbnN0YW5jZW9mIGEpe2NvbnNvbGUuZXJyb3IoYGNvdWxkIG5vdCBub20gVVRGMTYgZmlsZW5hbWU6ICR7ZH1gKTtjb250aW51ZX1sZXQgXz1iKGQucmVtYWluaW5nLDMpO2lmKF8gaW5zdGFuY2VvZiBhKXtjb25zb2xlLmVycm9yKGBjb3VsZCBub3Qgbm9tIGVuZCBvZiBzdHJpbmc6ICR7ZH1gKTtjb250aW51ZX1sZXQgbT1fLnJlbWFpbmluZyxOPUMoZC5ub21tZWQpLEk9W107Zm9yKDttLmxlbmd0aCE9MDspe2xldCB5PSQobSk7aWYoeSBpbnN0YW5jZW9mIHIpe2NvbnNvbGUuZXJyb3IoYGNvdWxkIG5vdCBwYXJzZSByZWNlbnQgZG9jcyBzaGVsbGl0ZW0gZm9yICR7Yy52YWx1ZX06ICR7eX1gKTticmVha31tPXkucmVtYWluaW5nLEkucHVzaCh5Lml0ZW0pfWxldCBMPUQoSSk7TC5maWxlbmFtZT1OLG4ucHVzaChMKX1yZXR1cm4gbn1mdW5jdGlvbiBIKGUpe2xldCB0PVAoZSk7aWYodCBpbnN0YW5jZW9mIHIpcmV0dXJuIG5ldyByKCJNUlUiLGBDb3VsZCBub3QgcGFyc2UgUmVnaXN0cnkgJHtlfTogJHt0fWApO2xldCBuPXoodCk7aWYobiBpbnN0YW5jZW9mIHIpcmV0dXJuIG5ldyByKCJNUlUiLGBDb3VsZCBub3QgZ2V0IE9wZW5TYXZlIE1SVSBlbnRyaWVzOiAke259YCk7bGV0IG89W10sYz17bnR1c2VyX3BhdGg6ZSxraW5kOiJPcGVuU2F2ZSIsbXJ1Om59O28ucHVzaChjKTtsZXQgZj1LKHQpO2lmKGYgaW5zdGFuY2VvZiByKXJldHVybiBuZXcgcigiTVJVIixgQ291bGQgbm90IGdldCBMYXN0VmlzaXRlZCBNUlUgZW50cmllczogJHtufWApO2xldCBkPXtudHVzZXJfcGF0aDplLGtpbmQ6Ikxhc3RWaXNpc3RlZCIsbXJ1OmZ9O28ucHVzaChkKTtsZXQgXz1RKHQpO2lmKF8gaW5zdGFuY2VvZiByKXJldHVybiBuZXcgcigiTVJVIixgQ291bGQgbm90IGdldCBSZWNlbnREb2NzIE1SVSBlbnRyaWVzOiAke259YCk7bGV0IG09e250dXNlcl9wYXRoOmUsa2luZDoiUmVjZW50RG9jcyIsbXJ1Ol99O3JldHVybiBvLnB1c2gobSksb31mdW5jdGlvbiBEKGUpe2xldCB0PVtdO2lmKGUubGVuZ3RoPT09MClyZXR1cm57ZmlsZW5hbWU6IiIscGF0aDoiIixtb2RpZmllZDoiMTYwMS0wMS0wMVQwMDowMDowMC4wMDBaIixjcmVhdGVkOiIxNjAxLTAxLTAxVDAwOjAwOjAwLjAwMFoiLGFjY2Vzc2VkOiIxNjAxLTAxLTAxVDAwOjAwOjAwLjAwMFoiLGl0ZW1zOltdfTtmb3IobGV0IGMgb2YgZSl0LnB1c2goYy52YWx1ZS5yZXBsYWNlQWxsKCJcXFxcIiwiIikpO2xldCBuPWVbZS5sZW5ndGgtMV07cmV0dXJue2ZpbGVuYW1lOm4udmFsdWUscGF0aDp0LmpvaW4oIlxcIiksbW9kaWZpZWQ6bi5tb2RpZmllZCxjcmVhdGVkOm4uY3JlYXRlZCxhY2Nlc3NlZDpuLmFjY2Vzc2VkLGl0ZW1zOmV9fWZ1bmN0aW9uIGFlKCl7bGV0IHQ9cCgiQzpcXFVzZXJzXFwqXFxOVFVTRVIuREFUIik7aWYoISh0IGluc3RhbmNlb2YgcykpZm9yKGxldCBuIG9mIHQpe2xldCBvPUgobi5mdWxsX3BhdGgpO2NvbnNvbGUubG9nKEpTT04uc3RyaW5naWZ5KG8pKX19YWUoKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("shellitems"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).await.unwrap();
    }
}
