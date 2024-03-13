use crate::runtime::time::conversion::{
    js_cocoatime_to_unixepoch, js_fat_time_to_unixepoch, js_filetime_to_unixepoch,
    js_hfs_to_unixepoch, js_ole_automationtime_to_unixepoch, js_time_now,
    js_webkit_time_to_uniexepoch,
};
use deno_core::Op;

pub(crate) fn time_functions() -> Vec<deno_core::OpDecl> {
    vec![
        js_time_now::DECL,
        js_filetime_to_unixepoch::DECL,
        js_cocoatime_to_unixepoch::DECL,
        js_hfs_to_unixepoch::DECL,
        js_ole_automationtime_to_unixepoch::DECL,
        js_webkit_time_to_uniexepoch::DECL,
        js_fat_time_to_unixepoch::DECL,
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
