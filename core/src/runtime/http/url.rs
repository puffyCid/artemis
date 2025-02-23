use boa_engine::{js_string, Context, JsError, JsResult, JsValue};
use serde::Serialize;
use url::Url;

use crate::runtime::helper::string_arg;

#[derive(Serialize)]
pub(crate) struct UrlInfo {
    authority: String,
    username: String,
    password: String,
    host: String,
    domain: String,
    port: Option<u16>,
    path: String,
    segments: Vec<String>,
    query: String,
    query_pairs: Vec<String>,
    fragment: String,
    scheme: String,
}

pub(crate) fn js_url_parse(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let url_string = string_arg(args, &0)?;

    let res = match Url::parse(&url_string) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not parse URL {url_string}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let mut info = UrlInfo {
        authority: res.authority().to_string(),
        username: res.username().to_string(),
        password: res.password().unwrap_or_default().to_string(),
        host: res.host_str().unwrap_or_default().to_string(),
        domain: res.domain().unwrap_or_default().to_string(),
        port: res.port(),
        path: res.path().to_string(),
        segments: Vec::new(),
        query: res.query().unwrap_or_default().to_string(),
        fragment: res.fragment().unwrap_or_default().to_string(),
        scheme: res.scheme().to_string(),
        query_pairs: Vec::new(),
    };

    if let Some(segs) = res.path_segments() {
        for seg in segs {
            if seg.is_empty() {
                continue;
            }
            info.segments.push(seg.to_string());
        }
    }

    for (key, value) in res.query_pairs() {
        info.query_pairs.push(format!("{key}={value}"));
    }

    let results = serde_json::to_value(&info).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;
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
    fn test_js_url_parse_bing() {
        let test = "dmFyIHU9Y2xhc3MgZXh0ZW5kcyBFcnJvcntuYW1lO21lc3NhZ2U7Y29uc3RydWN0b3IoZSxyKXtzdXBlcigpLHRoaXMubmFtZT1lLHRoaXMubWVzc2FnZT1yfX07dmFyIG09Y2xhc3MgZXh0ZW5kcyB1e307dmFyIHM9Y2xhc3MgZXh0ZW5kcyB1e307ZnVuY3Rpb24gcCh0KXt0cnl7cmV0dXJuIGVuY29kaW5nLmF0b2IodCl9Y2F0Y2goZSl7cmV0dXJuIG5ldyBzKCJCQVNFNjQiLGBmYWlsZWQgdG8gZGVjb2RlICR7dH06ICR7ZX1gKX19ZnVuY3Rpb24gZyh0KXtyZXR1cm4gZW5jb2RpbmcuZXh0cmFjdF91dGY4X3N0cmluZyh0KX12YXIgbz1jbGFzcyBleHRlbmRzIHV7fTtmdW5jdGlvbiBiKHQsZSl7dHJ5e3JldHVybiBEZW5vLmNvcmUub3BzLmpzX25vbV91bnNpZ25lZF9mb3VyX2J5dGVzKHQsZSl9Y2F0Y2gocil7cmV0dXJuIG5ldyBvKCJOT00iLGBmYWlsZWQgdG8gbm9tIHVuc2lnbmVkIGZvdXIgYnl0ZTogJHtyfWApfX1mdW5jdGlvbiBkKHQsZSl7aWYoZTwwKXJldHVybiBuZXcgbygiTk9NIiwicHJvdmlkZWQgbmVnYXRpdmUgbnVtYmVyIik7aWYodHlwZW9mIHQ9PSJzdHJpbmciKXRyeXtsZXQgcj1EZW5vLmNvcmUub3BzLmpzX25vbV90YWtlX3N0cmluZyh0LGUpO3JldHVybiBKU09OLnBhcnNlKHIpfWNhdGNoKHIpe3JldHVybiBuZXcgbygiTk9NIixgY291bGQgbm90IHRha2Ugc3RyaW5nICR7cn1gKX10cnl7cmV0dXJuIERlbm8uY29yZS5vcHMuanNfbm9tX3Rha2VfYnl0ZXModCxlKX1jYXRjaChyKXtyZXR1cm4gbmV3IG8oIk5PTSIsYGNvdWxkIG5vdCB0YWtlIGJ5dGVzICR7cn1gKX19ZnVuY3Rpb24gaCh0LGUpe2lmKHR5cGVvZiB0IT10eXBlb2YgZSlyZXR1cm4gbmV3IG8oIk5PTSIsImRhdGEgYW5kIGlucHV0IG11c3QgYmUgbWF0Y2hpbmcgdHlwZXMiKTtpZih0eXBlb2YgdD09InN0cmluZyImJnR5cGVvZiBlPT0ic3RyaW5nIil0cnl7bGV0IHI9RGVuby5jb3JlLm9wcy5qc19ub21fdGFrZV91bnRpbF9zdHJpbmcodCxlKTtyZXR1cm4gSlNPTi5wYXJzZShyKX1jYXRjaChyKXtyZXR1cm4gbmV3IG8oIk5PTSIsYGNvdWxkIG5vdCB0YWtlIHVudGlsIHN0cmluZyAke3J9YCl9dHJ5e3JldHVybiBEZW5vLmNvcmUub3BzLmpzX25vbV90YWtlX3VudGlsX2J5dGVzKHQsZSl9Y2F0Y2gocil7cmV0dXJuIG5ldyBvKCJOT00iLGBjb3VsZCBub3QgdGFrZSB1bnRpbCBieXRlcyAke3J9YCl9fWZ1bmN0aW9uIGwodCl7dD10LnJlcGxhY2VBbGwoIl8iLCIrIikucmVwbGFjZUFsbCgiLSIsIi8iKSx0PXQucmVwbGFjZUFsbCgiJTNEIiwiPSIpLnJlcGxhY2VBbGwoIiUyRiIsIisiKTtsZXQgZT1wKHQpO3JldHVybiBlIGluc3RhbmNlb2YgcyYmKGU9cChgJHt0fT1gKSxlIGluc3RhbmNlb2YgcyYmKGU9cChgJHt0fT09YCkpKSxlfWZ1bmN0aW9uIFModCl7cmV0dXJuIHQubGVuZ3RoIT0zMj90OmAke3Quc2xpY2UoMCw4KX0tJHt0LnNsaWNlKDgsMTIpfS0ke3Quc2xpY2UoMTIsMTYpfS0ke3Quc2xpY2UoMTYsMjApfS0ke3Quc2xpY2UoMjApfWB9dmFyIHk9Y2xhc3N7dXJsO2NvbnN0cnVjdG9yKGUpe3RoaXMudXJsPWV9cGFyc2VCaW5nKCl7Zm9yKGxldCBlIG9mIHRoaXMudXJsLnF1ZXJ5X3BhaXJzKXtsZXRbciwuLi5pXT1lLnNwbGl0KCI9Iiksbj1pLmpvaW4oIj0iKTtzd2l0Y2goci50b0xvd2VyQ2FzZSgpKXtjYXNlImZvcm0iOntpZihuPT09IiIpYnJlYWs7dGhpcy51cmxbcl09bjticmVha31jYXNlInNrIjp7aWYobj09PSIiKWJyZWFrO3RoaXMudXJsW3JdPW47YnJlYWt9Y2FzZSJnaHBsIjp7aWYobj09PSIiKWJyZWFrO3RoaXMudXJsW3JdPW47YnJlYWt9Y2FzZSJxIjp7dGhpcy51cmwuc2VhcmNoX3F1ZXJ5PW47YnJlYWt9Y2FzZSJzcCI6e3RoaXMudXJsLnN1Z2dlc3RlZF9wb3NpdGlvbj1uO2JyZWFrfWNhc2UibHEiOnt0aGlzLnVybFtyXT1uO2JyZWFrfWNhc2UicHEiOnt0aGlzLnVybC5wYXJ0aWFsX3F1ZXJ5PW47YnJlYWt9Y2FzZSJzYyI6e3RoaXMudXJsLnN1Z2dlc3Rpb25fY291bnQ9bjticmVha31jYXNlInFzIjp7dGhpcy51cmwuc3VnZ2VzdGlvbl90eXBlPW47YnJlYWt9Y2FzZSJjdmlkIjp7bGV0IGM9bjtpZihjLmxlbmd0aCE9MzIpe3RoaXMudXJsLmNvbnZlcnNhdGlvbl9pZD1uO2JyZWFrfXRoaXMudXJsLmNvbnZlcnNhdGlvbl9pZD1TKGMpO2JyZWFrfWNhc2UiZ2hhY2MiOnt0aGlzLnVybFtyXT1uO2JyZWFrfWNhc2UiZmlsdGVycyI6e2xldCBjPWgobiwnIicpO2lmKGMgaW5zdGFuY2VvZiBvKWJyZWFrO2xldCBhPWgoYy5yZW1haW5pbmcuc2xpY2UoMSksJyInKTtpZihhIGluc3RhbmNlb2YgbylicmVhaztsZXQgZj1sKGEubm9tbWVkKTtpZihmIGluc3RhbmNlb2YgcylicmVhazt0aGlzLnVybFtyXT1nKGYpLnNwbGl0KCIhIik7bGV0IF89YS5yZW1haW5pbmcucmVwbGFjZUFsbCgnIicsIiIpLnRyaW0oKS5zcGxpdCgiICIpO2ZvcihsZXQgJCBvZiBfKXtsZXQgdz0kLnNwbGl0KCI6Iik7dGhpcy51cmxbYGZpbHRlcnNfJHt3LmF0KDApPz8iIn1gXT13LmF0KDEpfWJyZWFrfWNhc2UiZ2hzaCI6e3RoaXMudXJsW3JdPW47YnJlYWt9Y2FzZSB2b2lkIDA6YnJlYWs7ZGVmYXVsdDp7Y29uc29sZS53YXJuKGB1bmtub3duIGJpbmcga2V5OiAke3J9LiBWYWx1ZTogJHtufWApLHRoaXMudXJsW3JdPW47YnJlYWt9fX19fTt2YXIgaz1jbGFzc3t1cmw7Y29uc3RydWN0b3IoZSl7dGhpcy51cmw9ZX1wYXJzZUR1Y2tEdWNrR28oKXtmb3IobGV0IGUgb2YgdGhpcy51cmwucXVlcnlfcGFpcnMpe2xldCByPWUuc3BsaXQoIj0iKSxpPXIuYXQoMCk7c3dpdGNoKGkpe2Nhc2UiaWEiOnt0aGlzLnVybC5zZWFyY2hfdHlwZT1yLmF0KDEpO2JyZWFrfWNhc2UidCI6e2lmKHIuYXQoMSk9PT0iaF8iKXt0aGlzLnVybC50cmFja2VyPSJob21lcGFnZSI7YnJlYWt9dGhpcy51cmwudHJhY2tlcj1yLmF0KDEpO2JyZWFrfWNhc2UicSI6e3RoaXMudXJsLnNlYXJjaF9xdWVyeT1yLmF0KDEpO2JyZWFrfWNhc2UiZGYiOnt0aGlzLnVybC5zZWFyY2hfcGVyaW9kPXIuYXQoMSk7YnJlYWt9Y2FzZSJpYXgiOnt0aGlzLnVybC5zZWFyY2hfdHlwZV9leHBhbmRlZF92aWV3PXIuYXQoMSk7YnJlYWt9Y2FzZSJpYXhtIjp7dGhpcy51cmwuc2VhcmNoX3R5cGVfZXhwYW5kZWRfdmlld19tb2JpbGU9ci5hdCgxKTticmVha31jYXNlImJib3giOnt0aGlzLnVybC5ib3VuZGluZ19ib3g9ci5hdCgxKTticmVha31jYXNlImlhciI6e3RoaXMudXJsLnNlYXJjaF9yZWZlcnJlcj1yLmF0KDEpO2JyZWFrfWNhc2Ugdm9pZCAwOmJyZWFrO2RlZmF1bHQ6e2NvbnNvbGUud2FybihgdW5rbm93biBkdWNrZHVja2dvIGtleTogJHtpfS4gVmFsdWU6ICR7ci5hdCgxKX1gKTticmVha319fX19O2Z1bmN0aW9uIEUodCl7dHJ5e2xldCBlPWVuY29kaW5nLnBhcnNlX3Byb3RvYnVmKHQpO3JldHVybiBKU09OLnBhcnNlKGUpfWNhdGNoKGUpe3JldHVybiBuZXcgcygiUFJPVE9CVUYiLGBmYWlsZWQgdG8gcGFyc2UgcHJvdG9idWY6ICR7ZX1gKX19ZnVuY3Rpb24gTyh0KXtpZih0PT09MHx8dD09PTBuKXJldHVybiBuZXcgRGF0ZShOdW1iZXIodCkpLnRvSVNPU3RyaW5nKCk7bGV0IGU9MTMscj0xZTM7aWYodHlwZW9mIHQ9PSJudW1iZXIiJiZ0LnRvU3RyaW5nKCkubGVuZ3RoPGUpcmV0dXJuIG5ldyBEYXRlKHQqcikudG9JU09TdHJpbmcoKTtpZih0LnRvU3RyaW5nKCkubGVuZ3RoPDE2KXJldHVybiBuZXcgRGF0ZShOdW1iZXIodCkpLnRvSVNPU3RyaW5nKCk7bGV0IG49MTk7aWYodC50b1N0cmluZygpLmxlbmd0aDxuKXtsZXQgYT1CaWdJbnQodCkvQmlnSW50KHIpO3JldHVybiBuZXcgRGF0ZShOdW1iZXIoYSkpLnRvSVNPU3RyaW5nKCl9aWYodC50b1N0cmluZygpLmxlbmd0aD09PW4pe2xldCBhPUJpZ0ludCh0KS9CaWdJbnQocipyKTtyZXR1cm4gbmV3IERhdGUoTnVtYmVyKGEpKS50b0lTT1N0cmluZygpfWNvbnNvbGUud2FybihgUmVjZWl2ZWQgdmVyeSBsYXJnZSBudW1iZXI6ICAke3R9LiBDb252ZXJ0aW5nIHRvIG1heCBOdW1iZXIgdHlwZSB2YWx1ZWApO2xldCBjPUJpZ0ludCh0KS9CaWdJbnQocik7cmV0dXJuIG5ldyBEYXRlKE51bWJlcihjKSkudG9JU09TdHJpbmcoKX12YXIgVT1jbGFzc3t1cmw7Y29uc3RydWN0b3IoZSl7dGhpcy51cmw9ZX1wYXJzZUdvb2dsZSgpe2ZvcihsZXQgZSBvZiB0aGlzLnVybC5xdWVyeV9wYWlycyl7bGV0IHI9ZS5zcGxpdCgiPSIpLGk9ci5hdCgwKTtzd2l0Y2goaSl7Y2FzZSJlaSI6e2xldCBuPXIuYXQoMSk7aWYobj09PXZvaWQgMClicmVhaztsZXQgYz1sKG4pO2lmKGMgaW5zdGFuY2VvZiBzKWJyZWFrO2xldCBhPWIoYywxKTtpZihhIGluc3RhbmNlb2YgbylicmVhazt0aGlzLnVybC5zZWFyY2hfdGltZT1PKGEudmFsdWUpO2JyZWFrfWNhc2UicSI6e3RoaXMudXJsLnNlYXJjaF9xdWVyeT1yLmF0KDEpO2JyZWFrfWNhc2UiaGwiOnt0aGlzLnVybC5ob3N0X2xhbmd1YWdlPXIuYXQoMSk7YnJlYWt9Y2FzZSJ1YWN0Ijp7dGhpcy51cmxbaV09ci5hdCgxKTticmVha31jYXNlInNjYV9lc3YiOnt0aGlzLnVybFtpXT1yLmF0KDEpO2JyZWFrfWNhc2Uib3EiOnt0aGlzLnVybC5vcmlnaW5hbF9xdWVyeT1yLmF0KDEpO2JyZWFrfWNhc2Uic291cmNlIjp7aWYoci5hdCgxKT09PSJocCIpe3RoaXMudXJsLnNvdXJjZV9vZl9zZWFyY2g9ImhvbWVwYWdlIjticmVha310aGlzLnVybC5zb3VyY2Vfb2Zfc2VhcmNoPXIuYXQoMSk7YnJlYWt9Y2FzZSJnc19scCI6e2xldCBuPXIuYXQoMSk7aWYobj09PXZvaWQgMClicmVhaztsZXQgYz1sKG4pO2lmKGMgaW5zdGFuY2VvZiBzKWJyZWFrO2xldCBhPUUoYyk7aWYoYSBpbnN0YW5jZW9mIHMpYnJlYWs7dGhpcy51cmwuZ3NfbHA9YTticmVha31jYXNlInZlZCI6e2xldCBuPXIuYXQoMSk7aWYobj09PXZvaWQgMHx8bi5zdGFydHNXaXRoKCIxIikpYnJlYWs7bGV0IGM9bChuLnN1YnN0cmluZygxKSk7aWYoYyBpbnN0YW5jZW9mIHMpYnJlYWs7bGV0IGE9RShjKTtpZihhIGluc3RhbmNlb2YgcylicmVhazt0aGlzLnVybC5saW5rX3RyYWNraW5nPWE7YnJlYWt9Y2FzZSJzY2xpZW50Ijp7bGV0IG49ci5hdCgxKTtzd2l0Y2gobil7Y2FzZSJnd3Mtd2l6LXNlcnAiOnt0aGlzLnVybC5zZWFyY2hfY2xpZW50PSJHb29nbGUgV2ViIFNlYXJjaCBXaXphcmQsIFNlYXJjaCBFbmdpbmUgUmVzdWx0cyBQYWdlIjticmVha31kZWZhdWx0Ont0aGlzLnVybC5zZWFyY2hfY2xpZW50PW47YnJlYWt9fWJyZWFrfWNhc2Ugdm9pZCAwOmJyZWFrO2RlZmF1bHQ6e2NvbnNvbGUud2FybihgdW5rbm93biBnb29nbGUga2V5OiAke2l9LiBWYWx1ZTogJHtyLmF0KDEpfWApLHRoaXMudXJsW2ldPXIuYXQoMSk7YnJlYWt9fX19fTtmdW5jdGlvbiBBKHQsZSl7cmV0dXJuIHQ9PT0wP2VuY29kaW5nLmJ5dGVzX3RvX2JlX2d1aWQoZSk6ZW5jb2RpbmcuYnl0ZXNfdG9fbGVfZ3VpZChlKX12YXIgeD1jbGFzc3t1cmw7Y29uc3RydWN0b3IoZSl7dGhpcy51cmw9ZX1wYXJzZU91dGxvb2soKXtpZih0aGlzLnVybC51cmwuaW5jbHVkZXMoImluYm94Iikpe2xldCBlPWwodGhpcy51cmwubGFzdF9zZWdtZW50KTtpZihlIGluc3RhbmNlb2YgcylyZXR1cm47bGV0IHI9YihlLDEpO2lmKHIgaW5zdGFuY2VvZiBvKXJldHVybjtsZXQgaT04LG49ZChyLnJlbWFpbmluZyxpKTtpZihuIGluc3RhbmNlb2YgbylyZXR1cm47bGV0IGM9ZyhuLm5vbW1lZC5zbGljZSgzKSk7aWYoaT0xNCxuPWQobi5yZW1haW5pbmcsaSksbiBpbnN0YW5jZW9mIG8pcmV0dXJuO2xldCBhPWcobi5ub21tZWQpO2k9ODtsZXQgZj1kKG4ucmVtYWluaW5nLGkpO2lmKGYgaW5zdGFuY2VvZiBvKXJldHVybjtsZXQgXz1kKGYucmVtYWluaW5nLDE2KTtpZihfIGluc3RhbmNlb2YgbylyZXR1cm47dGhpcy51cmwuZ3VpZD1BKDEsXy5ub21tZWQpLHRoaXMudXJsLmFjY291bnRfaWQ9YCR7Y30tJHthfS0wMGB9fX07dmFyIE49Y2xhc3N7dXJsO2NvbnN0cnVjdG9yKGUpe3RoaXMudXJsPWV9cGFyc2VVcmwoKXtsZXQgZT10aGlzLmV4dHJhY3RVcmwoKTtyZXR1cm4gZSBpbnN0YW5jZW9mIG0/ZTp0aGlzLmV4dHJhY3REYXRhKGUpfWV4dHJhY3REYXRhKGUpe3JldHVybiBlLmRvbWFpbi5pbmNsdWRlcygiZHVja2R1Y2tnby5jb20iKT9uZXcgayhlKS5wYXJzZUR1Y2tEdWNrR28oKTplLmRvbWFpbi5pbmNsdWRlcygiZ29vZ2xlLmNvbSIpP25ldyBVKGUpLnBhcnNlR29vZ2xlKCk6ZS5kb21haW4uaW5jbHVkZXMoIm91dGxvb2subGl2ZS5jb20iKT9uZXcgeChlKS5wYXJzZU91dGxvb2soKTplLmRvbWFpbi5pbmNsdWRlcygiYmluZy5jb20iKSYmbmV3IHkoZSkucGFyc2VCaW5nKCksZX1leHRyYWN0VXJsKCl7dHJ5e2xldCBlPURlbm8uY29yZS5vcHMudXJsX3BhcnNlKHRoaXMudXJsKSxyPUpTT04ucGFyc2UoZSk7cmV0dXJuIHIudXJsPXRoaXMudXJsLHIubGFzdF9zZWdtZW50PXIuc2VnbWVudHMuYXQoLTEpPz8iIixyfWNhdGNoKGUpe3JldHVybiBuZXcgbSgiVVJMX1BBUlNFIixgZmFpbGVkIHRvIHBhcnNlIHVybCBmaWxlICR7dGhpcy51cmx9OiAke2V9YCl9fX07ZnVuY3Rpb24gRCgpe2xldCBlPW5ldyBOKCJodHRwOi8vd3d3LmJpbmcuY29tL3NlYXJjaD9xPWJhcmFjaytvYmFtYStuZXdzJmZpbHRlcnM9ZHRiazolMjJNQ0Z2ZG1WeWRtbGxkeUYwYjNCemRHOXlhV1Z6SVROaFpqRTRPVEl6TFRNMU9UQXRaVFV6WWkxbVpHTmtMVFJrTVRjNU5tRTVZVFJqWWclM0QlM0QlMjIrZHRwcm1vOiUyMm5ld3MlMjIrc2lkOiUyMjNhZjE4OTIzLTM1OTAtZTUzYi1mZGNkLTRkMTc5NmE5YTRjYiUyMiZGT1JNPURFUE5BViIpLnBhcnNlVXJsKCk7aWYoIShlIGluc3RhbmNlb2YgbSkpcmV0dXJuIGNvbnNvbGUubG9nKGUpLGV9RCgpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("url_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
