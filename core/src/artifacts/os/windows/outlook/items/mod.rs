use super::{error::OutlookError, tables::property::PropertyContext};
use std::collections::HashMap;

pub(crate) fn extract_name_id_map(
    context: &[PropertyContext],
) -> Result<HashMap<u32, String>, OutlookError> {
    Ok(HashMap::new())
}
