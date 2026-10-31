use crate::{
    accessor::{
        config::AccessorConfig,
        entry::{
            handle::{DirEntry, DirHandle, EntryStat, FileHandle, GlobMatch},
            locator::SourceId,
        },
        error::{AccessorError, AccessorResult},
        filesystem::ext4::{fs::Ext4Fs, volume::Ext4Volume},
        io::reader::AccessorReader,
        location::path::InnerPath,
        source::backend::SourceBackend,
    },
    output::manager::OutputManager,
    structs::artifacts::os::files::FileOptions,
};
use std::{fs::File, path::PathBuf};

#[derive(Debug)]
pub(crate) struct Ext4Source {
    device: PathBuf,
    max_read_size: Option<u64>,
    fs: Ext4Fs<File>,
}

impl Ext4Source {
    pub(crate) fn new(config: &AccessorConfig, device: PathBuf) -> AccessorResult<Self> {
        let file = File::open(&device).map_err(|err| {
            AccessorError::ext4(Some(device.display().to_string()), err.to_string())
        })?;

        let volume = Ext4Volume::open(file, device.display().to_string())?;
        Ok(Self {
            fs: Ext4Fs::new(volume),
            device,
            max_read_size: config.max_read_size,
        })
    }

    pub(crate) fn walk(
        &self,
        inner: &InnerPath,
        options: &FileOptions,
        manager: &mut OutputManager,
        rule: &str,
        evidence: &str,
    ) -> AccessorResult<()> {
        self.fs.walk(inner, options, manager, rule, evidence)
    }
}

impl SourceBackend for Ext4Source {
    fn source_id(&self) -> SourceId {
        SourceId::Ext4(self.device.clone())
    }

    fn read_file(&self, inner: &InnerPath) -> AccessorResult<Vec<u8>> {
        self.fs.read_file(inner, self.max_read_size)
    }

    fn read_dir(&self, inner: &InnerPath) -> AccessorResult<Vec<DirEntry>> {
        self.fs.read_dir(inner)
    }

    fn read_dir_handle(&self, handle: &DirHandle) -> AccessorResult<Vec<DirEntry>> {
        self.fs.read_dir_handle(handle)
    }

    fn globfs(&self, directory: &InnerPath, pattern: &str) -> AccessorResult<Vec<GlobMatch>> {
        self.fs.globfs(directory, pattern)
    }

    fn read_file_handle(&self, handle: &FileHandle) -> AccessorResult<Vec<u8>> {
        self.fs.read_handle(handle, self.max_read_size)
    }

    fn open_reader(&self, inner: &InnerPath) -> AccessorResult<AccessorReader> {
        self.fs.reader(inner)
    }

    fn open_reader_handle(&self, handle: &FileHandle) -> AccessorResult<AccessorReader> {
        self.fs.reader_handle(handle)
    }

    fn stat(&self, inner: &InnerPath) -> AccessorResult<EntryStat> {
        self.fs.stat(inner)
    }

    fn stat_handle(&self, handle: &FileHandle) -> AccessorResult<EntryStat> {
        self.fs.stat_handle(handle)
    }

    fn stat_dir_handle(&self, handle: &DirHandle) -> AccessorResult<EntryStat> {
        self.fs.stat_dir_handle(handle)
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::{
        access::Accessor,
        config::AccessorConfig,
        entry::locator::FileLocator,
        error::AccessorError,
        location::path::InnerPath,
        source::{backend::SourceBackend, ext4::Ext4Source},
    };
    use common::files::EntryKind;
    use std::{fs, io::Read, path::PathBuf};

    fn image_path() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/images/ext4/test.img");
        path
    }

    fn source() -> Ext4Source {
        Ext4Source::new(&AccessorConfig::default(), image_path()).unwrap()
    }

    #[test]
    fn test_ext4_source_read_file() {
        let bytes = source()
            .read_file(&InnerPath::new(PathBuf::from("afile")))
            .unwrap();
        assert_eq!(bytes, b"I like Rust!\n");
    }

    #[test]
    fn test_ext4_source_enforces_max_read_size() {
        let opened = Ext4Source::new(
            &AccessorConfig {
                max_read_size: Some(12),
                ..AccessorConfig::default()
            },
            image_path(),
        )
        .unwrap();

        let err = opened
            .read_file(&InnerPath::new(PathBuf::from("afile")))
            .unwrap_err();
        assert!(matches!(
            err,
            AccessorError::FileTooLarge {
                size: 13,
                limit: 12
            }
        ));
    }

    #[test]
    fn test_ext4_source_read_dir_then_handle() {
        let opened = source();
        let root = opened.read_dir(&InnerPath::empty()).unwrap();
        assert_eq!(
            root.iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            ["lost+found", "test", "afile"]
        );

        let afile = root.iter().find(|entry| entry.name == "afile").unwrap();
        let handle = afile.handle.as_file().unwrap();
        let FileLocator::Ext4 { label, .. } = &handle.locator else {
            panic!("expected an ext4 file handle");
        };
        assert_eq!(label, &image_path().display().to_string());
        assert_eq!(opened.read_file_handle(handle).unwrap(), b"I like Rust!\n");
        assert_eq!(opened.stat_handle(handle).unwrap().meta.inode, 18);

        let mut accessor = Accessor::with_defaults();
        assert_eq!(
            accessor.read_file_handle(handle).unwrap(),
            b"I like Rust!\n"
        );
    }

    #[test]
    fn test_ext4_source_stat_hello_txt() {
        let stat = source()
            .stat(&InnerPath::new(PathBuf::from("test/hello.txt")))
            .unwrap();
        assert_eq!(stat.meta.kind, EntryKind::File);
        assert_eq!(stat.meta.size, 13);
        assert_eq!(stat.meta.inode, 14);
    }

    #[test]
    fn test_ext4_source_reader() {
        let opened = source();
        let mut reader = opened
            .open_reader(&InnerPath::new(PathBuf::from("afile")))
            .unwrap();
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, b"I like Rust!\n");
    }

    #[test]
    fn test_ext4_source_rejects_a_short_file() {
        let dir = PathBuf::from("./tmp/ext4_source_short");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let path = dir.join("short.bin");
        fs::write(&path, [0_u8; 64]).unwrap();

        let err = Ext4Source::new(&AccessorConfig::default(), path).unwrap_err();
        assert!(matches!(err, AccessorError::Ext4 { .. }));
    }
}
