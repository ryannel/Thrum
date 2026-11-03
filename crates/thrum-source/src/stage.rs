//! Provisional exact-byte staging of a declared native source tree.
//!
//! A staged tree has no F3 semantic closure and no accepted-source authority.
//! Its later durable consumer must verify every staged hash again.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use crate::budget::{
    BudgetError, Limits, SourceSpacePlan, SourceWriterWindow, StageBudget,
    create_private_directory, ensure_directory,
};
use crate::{InventoryError, LimitExceeded, SourcePath, scan_declared_tree};

const COPY_CHUNK: usize = 4 * 1024 * 1024;
const MAX_LFS_POINTER_BYTES: usize = 1024;
const LFS_VERSION_LINES: [&str; 3] = [
    "version https://git-lfs.github.com/spec/v1",
    "version https://hawser.github.com/spec/v1",
    "version http://git-media.io/v/2",
];
static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);

/// A provisional source copy failed before semantic or durable admission.
#[derive(Debug)]
pub enum StageError {
    /// File membership or portability preflight failed.
    Inventory(InventoryError),
    /// Temporary-stage preflight or a live space check failed.
    Budget(BudgetError),
    /// A source file changed, disappeared, grew, or became unsafe during copy.
    SourceDrift(SourcePath),
    /// A declared file contains an unresolved Git LFS pointer, not asset bytes.
    UnresolvedLfsPointer(SourcePath),
    /// A stage path or source file operation failed.
    Io {
        /// Path associated with the failed operation.
        path: PathBuf,
        /// Original filesystem error.
        source: std::io::Error,
    },
}

impl StageError {
    /// Specific source-capacity refusal from inventory, when present.
    pub fn limit_exceeded(&self) -> Option<LimitExceeded> {
        match self {
            Self::Inventory(error) => error.limit_exceeded(),
            _ => None,
        }
    }
}

/// Exact-byte metadata of one file in a provisional stage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagedFile {
    path: SourcePath,
    length: u64,
    sha256: [u8; 32],
}

impl StagedFile {
    /// Portable relative source path.
    pub fn path(&self) -> &SourcePath {
        &self.path
    }

    /// Byte length copied into the stage.
    pub fn length(&self) -> u64 {
        self.length
    }

    /// SHA-256 observed while copying these bytes.
    pub fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }
}

/// An isolated stage and its sorted file metadata; not an admitted source.
pub struct StagedTree {
    owner: Arc<StageDirectory>,
    files: Vec<StagedFile>,
    copied_bytes: u64,
}

impl StagedTree {
    #[cfg(test)]
    pub(crate) fn test_directory(&self) -> &Path {
        &self.owner.path
    }

    /// Files in portable path order.
    pub fn files(&self) -> &[StagedFile] {
        &self.files
    }

    /// Total file bytes written, excluding temporary metadata overhead.
    pub fn copied_bytes(&self) -> u64 {
        self.copied_bytes
    }

    /// Open one staged file for a later validator or durable-store consumer.
    ///
    /// The returned lease keeps the stage alive. The consumer must compare
    /// length and SHA-256 on read; this provisional directory is not a durable
    /// accepted-source store and external tampering is outside its guarantee.
    pub fn open(&self, path: &SourcePath) -> Result<StagedLease, StageError> {
        if self
            .files
            .binary_search_by(|file| file.path.cmp(path))
            .is_err()
        {
            return Err(StageError::SourceDrift(path.clone()));
        }
        let full = self.owner.path.join(path.as_str());
        let metadata = fs::symlink_metadata(&full).map_err(|source| StageError::Io {
            path: full.clone(),
            source,
        })?;
        if !metadata.file_type().is_file() {
            return Err(StageError::SourceDrift(path.clone()));
        }
        let file = File::open(&full).map_err(|source| StageError::Io { path: full, source })?;
        Ok(StagedLease {
            file,
            _owner: Arc::clone(&self.owner),
        })
    }
}

/// A read handle that prevents stage cleanup until the final reader drops.
pub struct StagedLease {
    file: File,
    _owner: Arc<StageDirectory>,
}

impl Read for StagedLease {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl Seek for StagedLease {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

struct StageDirectory {
    path: PathBuf,
}

impl StageDirectory {
    fn create(engine_dir: &Path) -> Result<Self, StageError> {
        let parent = engine_dir.join("staging");
        ensure_directory(&parent).map_err(StageError::Budget)?;
        for _ in 0..100 {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let sequence = NEXT_STAGE.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!("{}-{nonce:x}-{sequence:x}", std::process::id()));
            match create_private_directory(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(source) => return Err(StageError::Io { path, source }),
            }
        }
        Err(StageError::Io {
            path: parent,
            source: std::io::Error::new(std::io::ErrorKind::AlreadyExists, "stage IDs exhausted"),
        })
    }
}

impl Drop for StageDirectory {
    fn drop(&mut self) {
        // A failed cleanup remains charged as temporary use by the next
        // preflight. Accepted-source authority never depends on this tree.
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Stage exact source bytes after temporary-use and observed-space preflight.
///
/// The v1 generated-root policy and complete declaration are checked before
/// writing. A cooperative cross-process source lock spans scan/copy/reread.
/// Ordinary drift refuses this attempt without modifying working files or a
/// previously returned stage. No F3 schema or dependency closure is implied.
pub fn stage_declared_tree(
    project_root: &Path,
    declared: &[SourcePath],
) -> Result<StagedTree, StageError> {
    stage_declared_tree_with_plan(project_root, declared, SourceSpacePlan::DEFAULT)
}

/// Stage a declared tree under a caller-selected native space plan.
///
/// This provisional capture prices its temporary copy and observed free-space
/// headroom. Main and WAL targets apply when a later accepted store can price
/// missing blobs from authenticated staged hashes. Neither phase can promise
/// a physical peak; that store owns SQLite failures and root/outcome publication.
pub fn stage_declared_tree_with_plan(
    project_root: &Path,
    declared: &[SourcePath],
    plan: SourceSpacePlan,
) -> Result<StagedTree, StageError> {
    stage_declared_tree_with(project_root, declared, plan.into(), |_| {})
}

pub(crate) fn stage_declared_tree_with(
    project_root: &Path,
    declared: &[SourcePath],
    limits: Limits,
    mut after_file_copy: impl FnMut(&SourcePath),
) -> Result<StagedTree, StageError> {
    let window = SourceWriterWindow::acquire(project_root).map_err(StageError::Budget)?;
    let inventory = scan_declared_tree(project_root, declared).map_err(StageError::Inventory)?;
    let mut budget = StageBudget::reserve_with_limits(project_root, &inventory, window, limits)
        .map_err(StageError::Budget)?;
    let owner = StageDirectory::create(budget.engine_dir())?;
    let mut buffer = vec![0_u8; COPY_CHUNK];
    let mut hashes = Vec::with_capacity(inventory.files().len());
    let mut copied_bytes = 0_u64;
    for source_file in inventory.files() {
        let source_path = project_root.join(source_file.path().as_str());
        let staged_path = owner.path.join(source_file.path().as_str());
        let parent = staged_path.parent().expect("validated relative file path");
        fs::create_dir_all(parent).map_err(|source| StageError::Io {
            path: parent.to_owned(),
            source,
        })?;
        let mut input = open_regular(&source_path, source_file.path())?;
        let starting_metadata = input.metadata().map_err(|source| StageError::Io {
            path: source_path.clone(),
            source,
        })?;
        if starting_metadata.len() != source_file.length()
            || starting_metadata
                .modified()
                .map_err(|source| StageError::Io {
                    path: source_path.clone(),
                    source,
                })?
                != source_file.modified()
        {
            return Err(StageError::SourceDrift(source_file.path().clone()));
        }
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged_path)
            .map_err(|source| StageError::Io {
                path: staged_path.clone(),
                source,
            })?;
        let mut hash = Sha256::new();
        let mut length = 0_u64;
        let mut pointer_prefix = [0_u8; MAX_LFS_POINTER_BYTES];
        let mut pointer_prefix_len = 0_usize;
        loop {
            let count = input.read(&mut buffer).map_err(|source| StageError::Io {
                path: source_path.clone(),
                source,
            })?;
            if count == 0 {
                break;
            }
            length = length
                .checked_add(count as u64)
                .ok_or_else(|| StageError::SourceDrift(source_file.path().clone()))?;
            if length > source_file.length() {
                return Err(StageError::SourceDrift(source_file.path().clone()));
            }
            if source_file.length() <= MAX_LFS_POINTER_BYTES as u64 {
                pointer_prefix[pointer_prefix_len..pointer_prefix_len + count]
                    .copy_from_slice(&buffer[..count]);
                pointer_prefix_len += count;
            }
            budget
                .before_write(project_root, count as u64)
                .map_err(StageError::Budget)?;
            output
                .write_all(&buffer[..count])
                .map_err(|source| StageError::Io {
                    path: staged_path.clone(),
                    source,
                })?;
            hash.update(&buffer[..count]);
            copied_bytes += count as u64;
            budget
                .after_write(project_root, count as u64)
                .map_err(StageError::Budget)?;
        }
        if length != source_file.length() {
            return Err(StageError::SourceDrift(source_file.path().clone()));
        }
        if looks_like_lfs_pointer(&pointer_prefix[..pointer_prefix_len]) {
            return Err(StageError::UnresolvedLfsPointer(source_file.path().clone()));
        }
        output.flush().map_err(|source| StageError::Io {
            path: staged_path.clone(),
            source,
        })?;
        hashes.push(<[u8; 32]>::from(hash.finalize()));
        after_file_copy(source_file.path());
    }
    let after = scan_declared_tree(project_root, declared).map_err(StageError::Inventory)?;
    for (before, after) in inventory.files().iter().zip(after.files()) {
        if before.length() != after.length() || before.modified() != after.modified() {
            return Err(StageError::SourceDrift(before.path().clone()));
        }
    }
    for (observed, staged_hash) in inventory.files().iter().zip(&hashes) {
        let path = project_root.join(observed.path().as_str());
        let mut source = open_regular(&path, observed.path())?;
        let mut hash = Sha256::new();
        let mut length = 0_u64;
        loop {
            let count = source.read(&mut buffer).map_err(|source| StageError::Io {
                path: path.clone(),
                source,
            })?;
            if count == 0 {
                break;
            }
            length = length
                .checked_add(count as u64)
                .ok_or_else(|| StageError::SourceDrift(observed.path().clone()))?;
            if length > observed.length() {
                return Err(StageError::SourceDrift(observed.path().clone()));
            }
            hash.update(&buffer[..count]);
        }
        if length != observed.length() || hash.finalize().as_slice() != staged_hash {
            return Err(StageError::SourceDrift(observed.path().clone()));
        }
    }
    budget.verify_temp_cap().map_err(StageError::Budget)?;
    budget
        .after_write(project_root, 0)
        .map_err(StageError::Budget)?;
    let files = inventory
        .into_files()
        .into_iter()
        .zip(hashes)
        .map(|(observed, sha256)| {
            let (path, length) = observed.into_path_and_length();
            StagedFile {
                path,
                length,
                sha256,
            }
        })
        .collect();
    Ok(StagedTree {
        owner: Arc::new(owner),
        files,
        copied_bytes,
    })
}

fn looks_like_lfs_pointer(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|text| text.trim_start().lines().next())
        .is_some_and(|line| LFS_VERSION_LINES.contains(&line))
}

fn open_regular(path: &Path, source_path: &SourcePath) -> Result<File, StageError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| StageError::Io {
        path: path.to_owned(),
        source,
    })?;
    if !metadata.file_type().is_file() {
        return Err(StageError::SourceDrift(source_path.clone()));
    }
    let file = File::open(path).map_err(|source| StageError::Io {
        path: path.to_owned(),
        source,
    })?;
    let opened = file.metadata().map_err(|source| StageError::Io {
        path: path.to_owned(),
        source,
    })?;
    if !opened.file_type().is_file() {
        return Err(StageError::SourceDrift(source_path.clone()));
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "thrum-source-stage-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            Self(root)
        }

        fn write(&self, path: &str, bytes: &[u8]) {
            let full = self.0.join(path);
            fs::create_dir_all(full.parent().unwrap()).unwrap();
            fs::write(full, bytes).unwrap();
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn small_plan() -> SourceSpacePlan {
        SourceSpacePlan {
            main_target_bytes: 2 * 1024 * 1024 * 1024,
            wal_stop_bytes: 4 * 1024 * 1024,
            temporary_target_bytes: 16 * 1024 * 1024,
            free_headroom_bytes: 0,
        }
    }

    fn small_limits() -> Limits {
        small_plan().into()
    }

    fn paths() -> Vec<SourcePath> {
        ["project.json", "scenes/main.scene.json", "src/game.rs"]
            .map(|path| SourcePath::parse(path).unwrap())
            .to_vec()
    }

    #[test]
    fn source_edit_yields_new_staged_bytes_without_changing_old_reader() {
        let fixture = Fixture::new();
        fixture.write("project.json", b"{}");
        fixture.write("scenes/main.scene.json", b"first");
        fixture.write("src/game.rs", b"fn game() {}");
        let first = stage_declared_tree_with(&fixture.0, &paths(), small_limits(), |_| {}).unwrap();
        fixture.write("scenes/main.scene.json", b"second");
        let second =
            stage_declared_tree_with(&fixture.0, &paths(), small_limits(), |_| {}).unwrap();
        let scene = SourcePath::parse("scenes/main.scene.json").unwrap();
        let mut old_reader = first.open(&scene).unwrap();
        let old_stage_path = first.owner.path.clone();
        drop(first);
        assert!(old_stage_path.exists());
        let mut old_bytes = Vec::new();
        old_reader.read_to_end(&mut old_bytes).unwrap();
        assert_eq!(old_bytes, b"first");
        let mut new_bytes = Vec::new();
        second
            .open(&scene)
            .unwrap()
            .read_to_end(&mut new_bytes)
            .unwrap();
        assert_eq!(new_bytes, b"second");
        assert_ne!(
            Sha256::digest(&old_bytes).as_slice(),
            Sha256::digest(&new_bytes).as_slice()
        );
        drop(old_reader);
        assert!(!old_stage_path.exists());
        drop(second);
    }

    #[test]
    fn source_drift_refuses_new_stage_and_keeps_prior_stage() {
        let fixture = Fixture::new();
        fixture.write("project.json", b"{}");
        fixture.write("scenes/main.scene.json", b"first");
        fixture.write("src/game.rs", b"fn game() {}");
        let old = stage_declared_tree_with(&fixture.0, &paths(), small_limits(), |_| {}).unwrap();
        let mut changed = false;
        let rejected = stage_declared_tree_with(&fixture.0, &paths(), small_limits(), |path| {
            if !changed && path.as_str() == "scenes/main.scene.json" {
                fixture.write("scenes/main.scene.json", b"drift");
                changed = true;
            }
        });
        assert!(matches!(rejected, Err(StageError::SourceDrift(_))));
        let scene = SourcePath::parse("scenes/main.scene.json").unwrap();
        let mut bytes = Vec::new();
        old.open(&scene).unwrap().read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"first");
        drop(old);
    }

    #[test]
    fn membership_change_and_space_refusal_leave_no_new_stage() {
        let fixture = Fixture::new();
        fixture.write("project.json", b"{}");
        fixture.write("scenes/main.scene.json", b"first");
        fixture.write("src/game.rs", b"fn game() {}");
        let refused_plan = SourceSpacePlan {
            temporary_target_bytes: 0,
            ..small_plan()
        };
        assert!(matches!(
            stage_declared_tree_with_plan(&fixture.0, &paths(), refused_plan),
            Err(StageError::Budget(BudgetError::TemporaryLimit))
        ));
        assert!(!fixture.0.join(".engine/staging").exists());

        let prior = stage_declared_tree_with_plan(&fixture.0, &paths(), small_plan()).unwrap();
        let mut inserted = false;
        let changed = stage_declared_tree_with(&fixture.0, &paths(), small_limits(), |_| {
            if !inserted {
                fixture.write("new.bin", b"master");
                inserted = true;
            }
        });
        assert!(matches!(
            changed,
            Err(StageError::Inventory(InventoryError::UndeclaredFile(_)))
        ));
        let scene = SourcePath::parse("scenes/main.scene.json").unwrap();
        let mut bytes = Vec::new();
        prior.open(&scene).unwrap().read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"first");
        drop(prior);
    }

    #[test]
    fn unresolved_lfs_pointer_refuses_without_replacing_old_stage() {
        let fixture = Fixture::new();
        fixture.write("project.json", b"{}");
        fixture.write("scenes/main.scene.json", b"first");
        fixture.write("src/game.rs", b"fn game() {}");
        let old = stage_declared_tree_with(&fixture.0, &paths(), small_limits(), |_| {}).unwrap();
        for version in [
            "https://git-lfs.github.com/spec/v1",
            "https://hawser.github.com/spec/v1",
            "http://git-media.io/v/2",
        ] {
            fixture.write(
                "scenes/main.scene.json",
                format!(
                    "version {version}\noid sha256:{}\nsize 12345\n",
                    "a".repeat(64)
                )
                .as_bytes(),
            );
            assert!(matches!(
                stage_declared_tree_with(&fixture.0, &paths(), small_limits(), |_| {}),
                Err(StageError::UnresolvedLfsPointer(_))
            ));
        }
        for prefix in [
            "version https://git-lfs.github.com/spec/v1\r\n",
            " \t\nversion https://git-lfs.github.com/spec/v1\r\n",
        ] {
            fixture.write(
                "scenes/main.scene.json",
                format!("{prefix}oid sha256:{}\r\nsize 12345\r\n", "a".repeat(64)).as_bytes(),
            );
            assert!(matches!(
                stage_declared_tree_with(&fixture.0, &paths(), small_limits(), |_| {}),
                Err(StageError::UnresolvedLfsPointer(_))
            ));
        }
        let scene = SourcePath::parse("scenes/main.scene.json").unwrap();
        let mut old_bytes = Vec::new();
        old.open(&scene)
            .unwrap()
            .read_to_end(&mut old_bytes)
            .unwrap();
        assert_eq!(old_bytes, b"first");
        drop(old);
    }

    #[test]
    #[ignore = "local 64 MiB provisional capture throughput probe"]
    fn measure_provisional_stage_64_mib() {
        let fixture = Fixture::new();
        let bytes = vec![0x5a_u8; COPY_CHUNK];
        let declared: Vec<_> = (0..16)
            .map(|index| {
                let path = format!("blobs/{index:03}.bin");
                fixture.write(&path, &bytes);
                SourcePath::parse(&path).unwrap()
            })
            .collect();
        let limits = Limits {
            temporary: 256 * 1024 * 1024,
            ..small_limits()
        };
        let start = std::time::Instant::now();
        let stage = stage_declared_tree_with(&fixture.0, &declared, limits, |_| {}).unwrap();
        let elapsed = start.elapsed();
        assert_eq!(stage.copied_bytes(), 64 * 1024 * 1024);
        std::hint::black_box(stage.files().len());
        eprintln!("captured_mib=64 elapsed={elapsed:?} chunk_mib=4 source_reads=2 stage_writes=1");
    }
}
