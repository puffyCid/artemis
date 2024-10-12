use std::fmt;

#[derive(Debug)]
pub enum EventLogsError {
    DefaultDrive,
    Parser,
    Serialize,
    EventLogServices,
    OnlyWevtTemplate,
    NoMessageTable,
    NoWevtTemplate,
}

impl std::error::Error for EventLogsError {}

impl fmt::Display for EventLogsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventLogsError::DefaultDrive => write!(f, "Failed to get default driver letter"),
            EventLogsError::Parser => write!(f, "Failed to parse eventlogs"),
            EventLogsError::Serialize => write!(f, "Failed to serialize eventlogs"),
            EventLogsError::EventLogServices => write!(f, "Failed to parse registry for services"),
            EventLogsError::OnlyWevtTemplate => write!(
                f,
                "Only have WEVT_TEMPLATE resource. Cannot add eventlog strings with just this."
            ),
            EventLogsError::NoMessageTable => write!(f, "No MESSAGETABLE resource found"),
            EventLogsError::NoWevtTemplate => write!(f, "No WEVT_TEMPLATE resource found"),
        }
    }
}
