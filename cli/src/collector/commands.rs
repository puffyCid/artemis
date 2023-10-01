use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub(crate) enum CommandArgs {
    /// Collect processes
    Processes {
        /// MD5 hash processes
        #[arg(long)]
        md5: bool,
        /// SHA1 hash processes
        #[arg(long)]
        sha1: bool,
        /// SHA256 hash processes
        #[arg(long)]
        sha256: bool,
        /// Parse ELF binaries
        #[arg(long)]
        metadata: bool,
    },
    /// Pull filelisting
    Filelisting {
        /// MD5 hash files
        #[arg(long)]
        md5: bool,
        /// SHA1 hash files
        #[arg(long)]
        sha1: bool,
        /// SHA256 hash files
        #[arg(long)]
        sha256: bool,
        /// Parse executable binaries
        #[arg(long)]
        metadata: bool,
        /// Start path for listing
        #[arg(long, default_value_t = String::from("/"))]
        start_path: String,
        /// Depth for file listing. Max is 255
        #[arg(long, default_value_t = 2)]
        depth: u8,
        /// Regex to only include entries that match
        #[arg(long, default_value = None)]
        regex_filter: Option<String>,
    },
    /// Get systeminfo
    Systeminfo {},
    /// Parse Firefox History
    Firefoxhistory {},
    /// Parse Chromium History
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

    /// Parse Prefetch
    Prefetch {
        /// Alternative drive letter to use
        #[arg(long, default_value = None)]
        alt_drive: Option<char>,
    },
    /// Parse EventLogs
    Eventlogs {
        /// Alternative drive letter to use
        #[arg(long, default_value = None)]
        alt_drive: Option<char>,
    },
    /// Parse NTFS to get filelisting
    Rawfilelisting {
        /// Drive letter to parse
        #[arg(long, default_value_t = 'C')]
        drive_letter: char,
        /// Start path for listing
        #[arg(long, default_value_t = String::from("C:\\"))]
        start_path: String,
        /// Depth for file listing. Max is 255
        #[arg(long, default_value_t = 1)]
        depth: u8,
        /// Parse deleted $INDX entries
        #[arg(long)]
        recover_indx: bool,
        /// MD5 hash files
        #[arg(long)]
        md5: bool,
        /// SHA1 hash files
        #[arg(long)]
        sha1: bool,
        /// SHA256 hash files
        #[arg(long)]
        sha256: bool,
        /// Parse PE binaries
        #[arg(long)]
        metadata: bool,
        /// Regex to only include entries that match path
        #[arg(long, default_value = None)]
        path_regex: Option<String>,
        /// Regex to only include entries that match filename
        #[arg(long, default_value = None)]
        filename_regex: Option<String>,
    },
    /// Parse ShimDatabase
    Shimdb {
        /// Alternative drive letter to use
        #[arg(long, default_value = None)]
        alt_drive: Option<char>,
    },
    /// Parse Registry
    Registry {
        /// Paser user Registry files
        #[arg(long)]
        user_hives: bool,
        /// Parse System Registry files
        #[arg(long)]
        system_hives: bool,
        /// Alternative drive letter to use
        #[arg(long, default_value = None)]
        alt_drive: Option<char>,
        /// Regex to only include entries that match path
        #[arg(long, default_value = None)]
        path_regex: Option<String>,
    },
    /// Parse Userassist
    Userassist {
        /// Alternative drive letter to use
        #[arg(long, default_value = None)]
        alt_drive: Option<char>,
    },
    /// Parse Shimcache
    Shimcache {
        /// Alternative drive letter to use
        #[arg(long, default_value = None)]
        alt_drive: Option<char>,
    },
    /// Parse Shellbags
    Shellbags {
        /// Try to resolve GUIDs to directory names
        #[arg(long)]
        resolve_guids: bool,
        /// Alternative drive letter to use
        #[arg(long, default_value = None)]
        alt_drive: Option<char>,
    },
    /// Parse Amcache
    Amcache {
        /// Alternative drive letter to use
        #[arg(long, default_value = None)]
        alt_drive: Option<char>,
    },
    /// Parse Shortcuts
    Shortcuts {
        /// Path to directory containing Shortcut files
        #[arg(long)]
        path: String,
    },
    /// Parse UsnJrnl
    Usnjrnl {
        /// Alternative drive letter to use
        #[arg(long, default_value = None)]
        alt_drive: Option<char>,
    },
    /// Parse BITS
    Bits {
        /// Try to parse deleted BITS entries
        #[arg(long)]
        carve: bool,
        /// Alternative BITS path to use
        #[arg(long, default_value = None)]
        alt_path: Option<String>,
    },
    /// Parse SRUM
    Srum {
        /// Alternative SRUM file path
        #[arg(long, default_value = None)]
        alt_path: Option<String>,
    },
    /// Parse Users
    Users {
        /// Alternative drive letter to use
        #[arg(long, default_value = None)]
        alt_drive: Option<char>,
    },
    /// Parse Windows Search
    Search {
        /// Alternative Search file path
        #[arg(long, default_value = None)]
        alt_path: Option<String>,
    },
    /// Parse Windows Tasks
    Tasks {
        /// Alternative drive letter to use
        #[arg(long, default_value = None)]
        alt_drive: Option<char>,
    },
    /// Parse Windows Services
    Services {
        /// Alternative drive letter to use
        #[arg(long, default_value = None)]
        alt_drive: Option<char>,
    },
    /// Parse Jumplists
    Jumplists {
        /// Alternative drive letter to use
        #[arg(long, default_value = None)]
        alt_drive: Option<char>,
    },
    /// Parse RecycleBin
    Recyclebin {
        /// Alternative drive letter to use
        #[arg(long, default_value = None)]
        alt_drive: Option<char>,
    },
}
