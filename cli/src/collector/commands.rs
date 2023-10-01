use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub(crate) enum CommandArgs {
    /// Collect processes
    Processes {
        /// MD5 hash processes
        #[arg(long, default_value_t = true)]
        md5: bool,
        /// SHA1 hash processes
        #[arg(long, default_value_t = false)]
        sha1: bool,
        /// SHA256 hash processes
        #[arg(long, default_value_t = false)]
        sha256: bool,
        /// Parse ELF binaries
        #[arg(long, default_value_t = false)]
        metadata: bool,
    },
    /// Pull filelisting
    Filelisting {
        /// MD5 hash files
        #[arg(long, default_value_t = true)]
        md5: bool,
        /// SHA1 hash files
        #[arg(long, default_value_t = false)]
        sha1: bool,
        /// SHA256 hash files
        #[arg(long, default_value_t = false)]
        sha256: bool,
        /// Parse ELF binaries
        #[arg(long, default_value_t = false)]
        metadata: bool,
        /// Start path for listing
        #[arg(long, default_value_t = String::from("/"))]
        start_path: String,
        /// Depth for file listing. Max is 255
        #[arg(long, default_value_t = 2)]
        depth: u8,
        #[arg(long, default_value = None)]
        regex_filter: Option<String>,
    },
    /// Parse Firefox History and Downloads
    Firefoxhistory {},
    /// Parse Chromium History and Downloads
    Chromiumhistory {},
    /// Parse Firefox Downloads
    Firefoxdownloads {},
    /// Parse Chromium Downloads
    Chromiumdownloads {},
    #[cfg(target_family = "unix")]
    /// Parse Shellhistory
    Shellhistory {},
    #[cfg(target_family = "unix")]
    /// Parse Cron Jobs
    Cron {},
    #[cfg(target_family = "unix")]
    /// Grab Sudo logs
    Sudologs {},
    #[cfg(target_os = "linux")]
    /// Parse systemd Journal files
    Journals {},
    #[cfg(target_os = "linux")]
    /// Parse Logon files
    Logons {},
    /// Get systeminfo
    Systeminfo {},
}