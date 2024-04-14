use crate::runtime::time::conversion::{
    js_cocoatime_to_unixepoch, js_fat_time_to_unixepoch, js_filetime_to_unixepoch,
    js_hfs_to_unixepoch, js_ole_automationtime_to_unixepoch, js_time_now,
    js_webkit_time_to_uniexepoch,
};

pub(crate) fn time_functions() -> Vec<deno_core::OpDecl> {
    vec![
        js_time_now(),
        js_filetime_to_unixepoch(),
        js_cocoatime_to_unixepoch(),
        js_hfs_to_unixepoch(),
        js_ole_automationtime_to_unixepoch(),
        js_webkit_time_to_uniexepoch(),
        js_fat_time_to_unixepoch(),
    ]
}

#[cfg(test)]
mod tests {
    use super::time_functions;

    #[test]
    fn test_time_functions() {
        let results = time_functions();
        assert!(results.len() > 1)
    }
}
