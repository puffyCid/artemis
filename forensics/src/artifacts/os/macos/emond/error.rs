use std::fmt;

#[derive(Debug)]
pub enum EmondError {
    Path,
    Plist,
    Rule,
    EventType,
    ActionArray,
    ActionDictionary,
}

impl std::error::Error for EmondError {}

impl fmt::Display for EmondError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmondError::Path => write!(f, "Failed to get directory path"),
            EmondError::Plist => write!(f, "Failed to parse PLIST file"),
            EmondError::Rule => write!(f, "Failed to parse rule file"),
            EmondError::EventType => write!(f, "Failed to parse Emond Event Type"),
            EmondError::ActionArray => write!(f, "Failed to parse Emond Action Array"),
            EmondError::ActionDictionary => write!(f, "Failed to parse Emond Action Dictionary"),
        }
    }
}
