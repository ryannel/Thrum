//! Read-only, complete working-tree inventory for native source capture.
//!
//! This scan is a preflight input, not an immutable snapshot. A later capture
//! must reread bytes and membership under its coordinated writer window.

use std::fs;
use std::path::{Path, PathBuf};

use crate::path::validate_path_refs;
use crate::{PathError, SourcePath, validate_paths};

/// Exact top-level roots excluded from v1 source membership by policy.
/// No unknown generated file is excluded by extension or wildcard.
const EXCLUDED_TOP_LEVEL: [&str; 3] = [".git", ".engine", "target"];
const MAX_ENTRIES: usize = 100_000;
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024 * 1024;

/// One regular source file observed by the preflight scan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InventoryFile {
    path: SourcePath,
    length: u64,
    modified: std::time::SystemTime,
}

impl InventoryFile {
    /// Portable relative source path.
    pub fn path(&self) -> &SourcePath {
        &self.path
    }

    /// Observed byte length, which capture must recheck.
    pub fn length(&self) -> u64 {
        self.length
    }

    pub(crate) fn modified(&self) -> std::time::SystemTime {
        self.modified
    }

    pub(crate) fn into_path_and_length(self) -> (SourcePath, u64) {
        (self.path, self.length)
    }
}

/// Sorted files and their total observed byte length before staging.
pub struct Inventory {
    files: Vec<InventoryFile>,
    total_bytes: u64,
    entry_count: usize,
}

impl Inventory {
    /// Files in portable path order.
    pub fn files(&self) -> &[InventoryFile] {
        &self.files
    }

    /// Sum of observed file lengths, before allocation rounding or overlap.
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// Observed nonexcluded files and directories, including empty ones.
    pub fn entry_count(&self) -> usize {
        self.entry_count
    }

    pub(crate) fn into_files(self) -> Vec<InventoryFile> {
        self.files
    }
}

/// Source membership is missing, ambiguous, unsafe or unreadable.
#[derive(Debug)]
pub enum InventoryError {
    /// The project root is not a regular directory.
    InvalidRoot,
    /// A directory entry is not representable as UTF-8.
    NonUtf8Name,
    /// A declared or observed path is not portable.
    Path(PathError),
    /// A symlink, device, socket or other nonregular entry is in the source tree.
    UnsupportedEntry(PathBuf),
    /// A file was found that the complete declaration omitted.
    UndeclaredFile(SourcePath),
    /// A declared file is absent.
    MissingFile(SourcePath),
    /// More than 100,000 source file/directory entries were encountered.
    TooManyEntries,
    /// One source file exceeded the 16 GiB v1 blob limit.
    FileTooLarge(SourcePath),
    /// The observed byte count overflowed.
    TotalOverflow,
    /// Filesystem enumeration or metadata failed; the path gives context.
    Io {
        /// Path whose enumeration or metadata operation failed.
        path: PathBuf,
        /// Original filesystem error.
        source: std::io::Error,
    },
}

/// Scan one complete declared tree without creating a stage or source revision.
///
/// The v1 policy excludes only exact top-level `.git`, `.engine` and `target`
/// roots. All other regular files must be declared, including code and binary
/// masters. This cannot prove a single filesystem instant against a writer;
/// staging must reread the tree and every file before returning a candidate.
pub fn scan_declared_tree(
    project_root: &Path,
    declared: &[SourcePath],
) -> Result<Inventory, InventoryError> {
    scan_declared_tree_with_hook(project_root, declared, MAX_ENTRIES, |_| {})
}

fn scan_declared_tree_with_hook(
    project_root: &Path,
    declared: &[SourcePath],
    max_entries: usize,
    mut before_directory: impl FnMut(&Path),
) -> Result<Inventory, InventoryError> {
    if declared.len() > max_entries {
        return Err(InventoryError::TooManyEntries);
    }
    validate_paths(declared).map_err(InventoryError::Path)?;
    let root_metadata =
        fs::symlink_metadata(project_root).map_err(|source| InventoryError::Io {
            path: project_root.to_owned(),
            source,
        })?;
    if !root_metadata.file_type().is_dir() {
        return Err(InventoryError::InvalidRoot);
    }

    let mut pending_dirs = vec![project_root.to_owned()];
    let mut files = Vec::new();
    let mut total_bytes = 0_u64;
    let mut entry_count = 0_usize;
    while let Some(directory) = pending_dirs.pop() {
        before_directory(&directory);
        let current = fs::symlink_metadata(&directory).map_err(|source| InventoryError::Io {
            path: directory.clone(),
            source,
        })?;
        if !current.file_type().is_dir() {
            return Err(InventoryError::UnsupportedEntry(directory));
        }
        let entries = fs::read_dir(&directory).map_err(|source| InventoryError::Io {
            path: directory.clone(),
            source,
        })?;
        for result in entries {
            let entry = result.map_err(|source| InventoryError::Io {
                path: directory.clone(),
                source,
            })?;
            if directory == project_root && excluded_top_level(&entry.file_name()) {
                continue;
            }
            entry_count += 1;
            if entry_count > max_entries {
                return Err(InventoryError::TooManyEntries);
            }
            let path = entry.path();
            let relative = path
                .strip_prefix(project_root)
                .map_err(|_| InventoryError::InvalidRoot)?;
            let relative = relative.to_str().ok_or(InventoryError::NonUtf8Name)?;
            #[cfg(windows)]
            let portable = relative.replace('\\', "/");
            #[cfg(not(windows))]
            let portable = relative.to_owned();
            let source_path = SourcePath::parse(&portable).map_err(InventoryError::Path)?;
            let kind = entry.file_type().map_err(|source| InventoryError::Io {
                path: path.clone(),
                source,
            })?;
            if kind.is_dir() {
                pending_dirs.push(path);
            } else if kind.is_file() {
                let metadata =
                    fs::symlink_metadata(&path).map_err(|source| InventoryError::Io {
                        path: path.clone(),
                        source,
                    })?;
                if !metadata.file_type().is_file() {
                    return Err(InventoryError::UnsupportedEntry(path));
                }
                if metadata.len() > MAX_FILE_BYTES {
                    return Err(InventoryError::FileTooLarge(source_path));
                }
                total_bytes = total_bytes
                    .checked_add(metadata.len())
                    .ok_or(InventoryError::TotalOverflow)?;
                let modified = metadata.modified().map_err(|source| InventoryError::Io {
                    path: path.clone(),
                    source,
                })?;
                files.push(InventoryFile {
                    path: source_path,
                    length: metadata.len(),
                    modified,
                });
            } else {
                return Err(InventoryError::UnsupportedEntry(path));
            }
        }
    }

    validate_path_refs(files.iter().map(|file| &file.path)).map_err(InventoryError::Path)?;
    files.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    let mut expected: Vec<&SourcePath> = declared.iter().collect();
    expected.sort_unstable();
    let mut actual_index = 0;
    let mut expected_index = 0;
    while let (Some(actual), Some(wanted)) = (files.get(actual_index), expected.get(expected_index))
    {
        match actual.path.cmp(wanted) {
            std::cmp::Ordering::Less => {
                return Err(InventoryError::UndeclaredFile(actual.path.clone()));
            }
            std::cmp::Ordering::Greater => {
                return Err(InventoryError::MissingFile((*wanted).clone()));
            }
            std::cmp::Ordering::Equal => {
                actual_index += 1;
                expected_index += 1;
            }
        }
    }
    if let Some(actual) = files.get(actual_index) {
        return Err(InventoryError::UndeclaredFile(actual.path.clone()));
    }
    if let Some(wanted) = expected.get(expected_index) {
        return Err(InventoryError::MissingFile((*wanted).clone()));
    }
    Ok(Inventory {
        files,
        total_bytes,
        entry_count,
    })
}

fn excluded_top_level(name: &std::ffi::OsStr) -> bool {
    name.to_str()
        .is_some_and(|name| EXCLUDED_TOP_LEVEL.contains(&name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "thrum-source-inventory-{}-{}",
                std::process::id(),
                NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed)
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

    fn declared(paths: &[&str]) -> Vec<SourcePath> {
        paths
            .iter()
            .map(|path| SourcePath::parse(path).unwrap())
            .collect()
    }

    #[test]
    fn complete_source_membership_and_generated_root_policy() {
        let fixture = Fixture::new();
        fixture.write("project.json", b"{}");
        fixture.write("scenes/main.scene.json", b"{}");
        fixture.write("src/game.rs", b"fn main() {}");
        fixture.write(".git/HEAD", b"ref");
        fixture.write(".engine/cache/file", b"cache");
        fixture.write("target/debug/file", b"generated");
        let paths = declared(&["src/game.rs", "project.json", "scenes/main.scene.json"]);
        let inventory = scan_declared_tree(&fixture.0, &paths).unwrap();
        assert_eq!(inventory.files().len(), 3);
        assert_eq!(inventory.total_bytes(), 16);
        fixture.write("new.bin", b"master");
        assert!(matches!(
            scan_declared_tree(&fixture.0, &paths),
            Err(InventoryError::UndeclaredFile(_))
        ));
        fs::remove_file(fixture.0.join("new.bin")).unwrap();
        assert!(matches!(
            scan_declared_tree(
                &fixture.0,
                &declared(&[
                    "src/game.rs",
                    "missing.json",
                    "project.json",
                    "scenes/main.scene.json"
                ])
            ),
            Err(InventoryError::MissingFile(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_and_directory_alias_refuse_inventory() {
        use std::os::unix::fs::symlink;

        let fixture = Fixture::new();
        fixture.write("project.json", b"{}");
        symlink("project.json", fixture.0.join("link.json")).unwrap();
        assert!(matches!(
            scan_declared_tree(&fixture.0, &declared(&["project.json"])),
            Err(InventoryError::UnsupportedEntry(_))
        ));
        fs::remove_file(fixture.0.join("link.json")).unwrap();
        fixture.write("Assets/a.bin", b"a");
        fixture.write("assets/b.bin", b"b");
        assert!(matches!(
            scan_declared_tree(
                &fixture.0,
                &declared(&["project.json", "Assets/a.bin", "assets/b.bin"])
            ),
            Err(InventoryError::Path(PathError::CaseFoldCollision))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn replaced_queued_directory_refuses_before_enumeration() {
        use std::os::unix::fs::symlink;

        let fixture = Fixture::new();
        fixture.write("project.json", b"{}");
        fixture.write("scenes/main.scene.json", b"{}");
        fixture.write(".git/outside.scene.json", b"outside");
        let mut swapped = false;
        let result = scan_declared_tree_with_hook(
            &fixture.0,
            &declared(&["project.json", "scenes/main.scene.json"]),
            MAX_ENTRIES,
            |directory| {
                if !swapped && directory.ends_with("scenes") {
                    fs::remove_dir_all(directory).unwrap();
                    symlink("../.git", directory).unwrap();
                    swapped = true;
                }
            },
        );
        assert!(matches!(result, Err(InventoryError::UnsupportedEntry(_))));
    }

    #[test]
    fn empty_directories_count_against_tree_entry_bound() {
        let fixture = Fixture::new();
        for index in 0..4 {
            fs::create_dir(fixture.0.join(format!("empty-{index}"))).unwrap();
        }
        assert!(matches!(
            scan_declared_tree_with_hook(&fixture.0, &[], 3, |_| {}),
            Err(InventoryError::TooManyEntries)
        ));
        let oversized = declared(&["a", "b", "c", "d"]);
        assert!(matches!(
            scan_declared_tree_with_hook(&fixture.0, &oversized, 3, |_| {}),
            Err(InventoryError::TooManyEntries)
        ));
    }
}
