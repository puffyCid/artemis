use std::fmt;

#[derive(Debug)]
pub enum RemoteError {
    SftpAuth,
    SftpSession,
    SftpUsername,
    RemoteUrl,
    RemoteUpload,
    BadResponse,
    RemotePort,
    SftpHandshake,
    TcpConnect,
    SftpPassword,
    SftpNoAuth,
    SftpChannel,
    CreateFile,
    FileWrite,
    FileClose,
    RemoteApiKey,
    MaxAttempts,
    CompressFailed,
}

impl std::error::Error for RemoteError {}

impl fmt::Display for RemoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RemoteError::SftpAuth => write!(f, "Failed to authenticate to SFTP server"),
            RemoteError::SftpSession => write!(f, "Failed to create SFTP session"),
            RemoteError::SftpUsername => write!(f, "Missing username from TOML"),
            RemoteError::RemoteUrl => write!(f, "Missing url from TOML"),
            RemoteError::RemotePort => write!(f, "Missing port from TOML"),
            RemoteError::SftpHandshake => write!(f, "Failed to establish SFTP handshake"),
            RemoteError::SftpNoAuth => write!(f, "Missing authentication from TOML"),
            RemoteError::SftpChannel => write!(f, "Failed to open SFTP channel"),
            RemoteError::SftpPassword => write!(
                f,
                "Could not authenticate with provided password or username"
            ),
            RemoteError::TcpConnect => write!(f, "Failed to create TCP Connection"),
            RemoteError::CreateFile => write!(f, "Failed to create remote file"),
            RemoteError::FileWrite => write!(f, "Failed to write remote file"),
            RemoteError::FileClose => write!(f, "Failed to close remote file"),
            RemoteError::RemoteUpload => write!(f, "Failed to upload data"),
            RemoteError::RemoteApiKey => write!(f, "Missing API key from TOML"),
            RemoteError::BadResponse => write!(f, "Received non-200 response from server"),
            RemoteError::MaxAttempts => write!(f, "Max attempts (15) reached for trying uploads"),
            RemoteError::CompressFailed => {
                write!(f, "Failed to compress with gzip")
            }
        }
    }
}
