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
        /// Parse binaries
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
        /// Base64 encoded Yara rule to only include entries that match
        #[arg(long, default_value = None)]
        yara_rule: Option<String>,
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

    /// windows: Parse Prefetch
    Prefetch {
        /// Alternative Prefetch directory to use
        #[arg(long, default_value = None)]
        alt_dir: Option<String>,
    },
    /// windows: Parse EventLogs
    Eventlogs {
        /// Alternative full path to an Event Log
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
    /// windows: Parse NTFS to get filelisting
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
    /// windows: Parse ShimDatabase
    Shimdb {
        /// Alternative full path to SDB file
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
    /// windows: Parse Registry
    Registry {
        /// Paser user Registry files
        #[arg(long)]
        user_hives: bool,
        /// Parse System Registry files
        #[arg(long)]
        system_hives: bool,
        /// Alternative full path to a Registry file
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
        /// Regex to only include entries that match path
        #[arg(long, default_value = None)]
        path_regex: Option<String>,
    },
    /// windows: Parse Userassist
    Userassist {
        /// Alternative full path to NTUSER.DAT Registry file
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
        /// Enable Folder Description lookups
        #[arg(long)]
        resolve_descriptions: Option<bool>,
    },
    /// windows: Parse Shimcache
    Shimcache {
        /// Alternative full path to SYSTEM Registry file
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
    /// windows: Parse Shellbags
    Shellbags {
        /// Try to resolve GUIDs to directory names
        #[arg(long)]
        resolve_guids: bool,
        /// Alternative full path to NTUSER.DAT or UsrClass.dat Registry file
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
    /// windows: Parse Amcache
    Amcache {
        /// Alternative full path to Amcache.hve
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
    /// windows: Parse Shortcuts
    Shortcuts {
        /// Path to directory containing Shortcut files
        #[arg(long)]
        path: String,
    },
    /// windows: Parse UsnJrnl
    Usnjrnl {
        /// Alternative drive letter to use
        #[arg(long, default_value = None)]
        alt_drive: Option<char>,
        /// Alternative path to UsnJrnl
        #[arg(long, default_value = None)]
        alt_path: Option<String>,
    },
    /// windows: Parse BITS
    Bits {
        /// Try to parse deleted BITS entries
        #[arg(long)]
        carve: bool,
        /// Alternative BITS file to use
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
    /// windows: Parse SRUM
    Srum {
        /// Alternative SRUM file path
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
    /// windows: Parse Users
    UsersWindows {
        /// Alternative full path to SAM Registry file
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
    /// windows: Parse Windows Search
    Search {
        /// Alternative Search file path
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
    /// windows: Parse Windows Tasks
    Tasks {
        /// Alternative full path to Schedule Task file
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
    /// windows: Parse Windows Services
    Services {
        /// Alternative full path to SYSTEM Registry file
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
    /// windows: Parse Jumplists
    Jumplists {
        /// Alternative full path to Jumplist file
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
    /// windows: Parse RecycleBin
    Recyclebin {
        /// Alternative full path to RecycleBin file
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
    /// windows: Parse WMI Repository
    Wmipersist {
        /// Alternative directory containing the WMI repository files
        #[arg(long, default_value = None)]
        alt_dir: Option<String>,
    },
    /// macos: Parse ExecPolicy
    Execpolicy {
        /// Alternative file path
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
    /// macos: Collect local users
    UsersMacos {
        /// Alternative path to users
        #[arg(long, default_value = None)]
        alt_path: Option<String>,
    },
    /// macos: Parse FsEvents entries
    Fsevents {
        /// Alternative file path
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
    /// macos: Parse Emond persistence. Removed in Ventura
    Emond {
        /// Alternative path to Emond
        #[arg(long, default_value = None)]
        alt_path: Option<String>,
    },
    /// macos: Parse LoginItems
    Loginitems {
        /// Alternative file path
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
    /// macos: Parse Launch Daemons and Agents
    Launchd {
        /// Alternative file path
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
    /// macos: Collect local groups
    GroupsMacos {
        /// Alternative path to groups
        #[arg(long, default_value = None)]
        alt_path: Option<String>,
    },
    /// macos: Collect Safari History
    SafariHistory {},
    /// macos: Collect Safari Downloads
    SafariDownloads {},
    /// macos: Parse the Unified Logs
    Unifiedlogs {
        /// Log sources to parse. Can be combination of: Persist, Special, Signpost, or HighVolume
        #[arg(long, value_delimiter = ',')]
        sources: Vec<String>,
        /// Use a log archive path instead of local files
        #[arg(long, default_value = None)]
        logarchive_path: Option<String>,
    },
    /// macos: Parse Sudo log entries from Unified Logs
    SudologsMacos {
        /// Use a log archive path instead of local files
        #[arg(long, default_value = None)]
        logarchive_path: Option<String>,
    },
    /// macos: Parse the Spotlight database
    Spotlight {
        /// Alternative path to a Spotlight database
        #[arg(long, default_value = None)]
        alt_path: Option<String>,
        /// Include additional known Spotlight database locations
        #[arg(long)]
        include_additional: bool,
    },
    /// unix: Parse Shellhistory
    Shellhistory {},
    /// unix: Parse Cron Jobs
    Cron {},
    /// linux: Grab Sudo logs
    SudologsLinux {
        /// Alternative Sudo log directory to use
        #[arg(long, default_value = None)]
        alt_path: Option<String>,
    },
    /// linux: Parse systemd Journal files
    Journals {
        /// Alternative Journal log directory to use
        #[arg(long, default_value = None)]
        alt_path: Option<String>,
    },
    /// linux: Parse Logon files
    Logons {
        /// Alternative logon file to use
        #[arg(long, default_value = None)]
        alt_file: Option<String>,
    },
}
