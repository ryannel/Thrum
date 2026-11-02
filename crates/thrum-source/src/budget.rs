//! Native staging preflight under a configurable space plan.
//!
//! This checks provisional capture only. It neither reserves filesystem blocks
//! nor makes a later SQLite admission accepted.

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

use fs4::{FileExt, TryLockError};

use crate::Inventory;

const GIB: u64 = 1024 * 1024 * 1024;
const MIB: u64 = 1024 * 1024;

/// Planning thresholds for native source staging, in bytes.
///
/// A plan guides early refusal and observed free-space checks. It is not a
/// physical quota: SQLite page growth and other processes can exceed an
/// estimate. A durable store must handle real write/space faults atomically.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSpacePlan {
    /// Target for the SQLite main file after candidate publication.
    pub main_target_bytes: u64,
    /// Observed WAL stop target; a later store owns checkpoint/backpressure.
    pub wal_stop_bytes: u64,
    /// Target for managed staging and other temporary files.
    pub temporary_target_bytes: u64,
    /// Desired observed volume free space after projected work.
    pub free_headroom_bytes: u64,
}

impl SourceSpacePlan {
    /// First native planning defaults; callers may select different values.
    pub const DEFAULT: Self = Self {
        main_target_bytes: 52 * GIB,
        wal_stop_bytes: 2 * GIB,
        temporary_target_bytes: 2 * GIB,
        free_headroom_bytes: 8 * GIB,
    };
}

#[derive(Clone, Copy)]
pub(crate) struct Limits {
    pub(crate) main: u64,
    pub(crate) wal: u64,
    pub(crate) temporary: u64,
    pub(crate) free_floor: u64,
}

impl Limits {
    #[cfg(test)]
    pub(crate) const V1: Self = Self {
        main: SourceSpacePlan::DEFAULT.main_target_bytes,
        wal: SourceSpacePlan::DEFAULT.wal_stop_bytes,
        temporary: SourceSpacePlan::DEFAULT.temporary_target_bytes,
        free_floor: SourceSpacePlan::DEFAULT.free_headroom_bytes,
    };
}

impl From<SourceSpacePlan> for Limits {
    fn from(plan: SourceSpacePlan) -> Self {
        Self {
            main: plan.main_target_bytes,
            wal: plan.wal_stop_bytes,
            temporary: plan.temporary_target_bytes,
            free_floor: plan.free_headroom_bytes,
        }
    }
}

#[derive(Default)]
struct PhysicalUsage {
    main: u64,
    wal: u64,
    temporary: u64,
}

/// A provisional source stage cannot fit the selected plan or actual volume.
#[derive(Debug)]
pub enum BudgetError {
    /// Another cooperating native source capture holds the project lock.
    Busy,
    /// The `.engine` directory contains an unsafe entry or path type.
    UnsafeStore(PathBuf),
    /// One byte counter or rounded estimate overflowed.
    Overflow,
    /// SQLite main-file projection exceeds the selected target.
    MainLimit,
    /// WAL already reached the selected stop-and-checkpoint target.
    WalLimit,
    /// Existing managed temporary use plus stage projection exceeds its target.
    TemporaryLimit,
    /// Observed available space cannot preserve the selected planned headroom.
    FreeSpace,
    /// A named filesystem operation failed.
    Io {
        /// Path associated with the failed operation.
        path: PathBuf,
        /// Original operating-system error.
        source: std::io::Error,
    },
}

/// Holds the cross-process advisory staging lock through provisional capture.
pub(crate) struct StageBudget {
    window: SourceWriterWindow,
    granularity: u64,
    projected_stage: u64,
    projected_main: u64,
    projected_wal: u64,
    free_floor: u64,
    temporary_limit: u64,
    written_raw: u64,
}

pub(crate) struct SourceWriterWindow {
    _lock: File,
    engine_dir: PathBuf,
}

impl SourceWriterWindow {
    pub(crate) fn acquire(project_root: &Path) -> Result<Self, BudgetError> {
        let engine_dir = project_root.join(".engine");
        ensure_directory(&engine_dir)?;
        let lock_path = engine_dir.join("source.lock");
        let lock = open_regular_lock(&lock_path)?;
        match FileExt::try_lock(&lock) {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(BudgetError::Busy),
            Err(TryLockError::Error(source)) => {
                return Err(BudgetError::Io {
                    path: lock_path,
                    source,
                });
            }
        }
        Ok(Self {
            _lock: lock,
            engine_dir,
        })
    }
}

fn open_regular_lock(path: &Path) -> Result<File, BudgetError> {
    for _ in 0..3 {
        let existing = match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_file() => true,
            Ok(_) => return Err(BudgetError::UnsafeStore(path.to_owned())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(source) => {
                return Err(BudgetError::Io {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        let opened = if existing {
            OpenOptions::new().read(true).write(true).open(path)
        } else {
            OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(path)
        };
        let file = match opened {
            Ok(file) => file,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::NotFound
                ) =>
            {
                continue;
            }
            Err(source) => {
                return Err(BudgetError::Io {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        let on_path = fs::symlink_metadata(path).map_err(|source| BudgetError::Io {
            path: path.to_owned(),
            source,
        })?;
        let opened_type = file.metadata().map_err(|source| BudgetError::Io {
            path: path.to_owned(),
            source,
        })?;
        if !on_path.file_type().is_file() || !opened_type.file_type().is_file() {
            return Err(BudgetError::UnsafeStore(path.to_owned()));
        }
        return Ok(file);
    }
    Err(BudgetError::Busy)
}

impl StageBudget {
    pub(crate) fn reserve_with_limits(
        project_root: &Path,
        inventory: &Inventory,
        window: SourceWriterWindow,
        limits: Limits,
    ) -> Result<Self, BudgetError> {
        let granularity =
            fs4::allocation_granularity(project_root).map_err(|source| BudgetError::Io {
                path: project_root.to_owned(),
                source,
            })?;
        if granularity == 0 {
            return Err(BudgetError::Overflow);
        }
        let usage = scan_store_usage(&window.engine_dir, granularity, limits.temporary)?;
        let projected_stage = projected_stage_bytes(inventory, granularity)?;
        let projected_main = inventory
            .total_bytes()
            .checked_mul(2)
            .and_then(|bytes| bytes.checked_add(64 * MIB))
            .and_then(|bytes| {
                bytes.checked_add((inventory.files().len() as u64).checked_mul(granularity)?)
            })
            .ok_or(BudgetError::Overflow)?;
        if usage
            .main
            .checked_add(projected_main)
            .ok_or(BudgetError::Overflow)?
            > limits.main
        {
            return Err(BudgetError::MainLimit);
        }
        if usage.wal >= limits.wal {
            return Err(BudgetError::WalLimit);
        }
        if usage
            .temporary
            .checked_add(projected_stage)
            .ok_or(BudgetError::Overflow)?
            > limits.temporary
        {
            return Err(BudgetError::TemporaryLimit);
        }
        let projected_wal = limits.wal - usage.wal;
        let required_free = projected_stage
            .checked_add(projected_main)
            .and_then(|bytes| bytes.checked_add(projected_wal))
            .and_then(|bytes| bytes.checked_add(limits.free_floor))
            .ok_or(BudgetError::Overflow)?;
        let available = fs4::available_space(project_root).map_err(|source| BudgetError::Io {
            path: project_root.to_owned(),
            source,
        })?;
        if available < required_free {
            return Err(BudgetError::FreeSpace);
        }
        Ok(Self {
            window,
            granularity,
            projected_stage,
            projected_main,
            projected_wal,
            free_floor: limits.free_floor,
            temporary_limit: limits.temporary,
            written_raw: 0,
        })
    }

    pub(crate) fn engine_dir(&self) -> &Path {
        &self.window.engine_dir
    }

    pub(crate) fn after_write(
        &mut self,
        project_root: &Path,
        raw_bytes: u64,
    ) -> Result<(), BudgetError> {
        self.written_raw = self
            .written_raw
            .checked_add(raw_bytes)
            .ok_or(BudgetError::Overflow)?;
        if self.written_raw > self.projected_stage {
            return Err(BudgetError::TemporaryLimit);
        }
        // The initial preflight charged the entire stage. During copying,
        // available_space already reflects actual stage allocations, including
        // rounded blocks and directory metadata. Subtracting only raw bytes
        // from projected_stage here would charge those allocations twice.
        self.check_free(project_root, 0)
    }

    pub(crate) fn before_write(
        &self,
        project_root: &Path,
        incoming_raw: u64,
    ) -> Result<(), BudgetError> {
        let incoming_physical = round_up(incoming_raw, self.granularity)?
            .checked_add(
                self.granularity
                    .checked_mul(2)
                    .ok_or(BudgetError::Overflow)?,
            )
            .ok_or(BudgetError::Overflow)?;
        self.check_free(project_root, incoming_physical)
    }

    fn check_free(&self, project_root: &Path, incoming: u64) -> Result<(), BudgetError> {
        let required_free = incoming
            .checked_add(self.projected_main)
            .and_then(|bytes| bytes.checked_add(self.projected_wal))
            .and_then(|bytes| bytes.checked_add(self.free_floor))
            .ok_or(BudgetError::Overflow)?;
        let available = fs4::available_space(project_root).map_err(|source| BudgetError::Io {
            path: project_root.to_owned(),
            source,
        })?;
        if available < required_free {
            return Err(BudgetError::FreeSpace);
        }
        Ok(())
    }

    pub(crate) fn verify_temp_cap(&self) -> Result<(), BudgetError> {
        scan_store_usage(
            &self.window.engine_dir,
            self.granularity,
            self.temporary_limit,
        )
        .map(|_| ())
    }
}

fn projected_stage_bytes(inventory: &Inventory, granularity: u64) -> Result<u64, BudgetError> {
    let mut bytes = granularity
        .checked_mul(inventory.entry_count() as u64 + 2)
        .ok_or(BudgetError::Overflow)?;
    for file in inventory.files() {
        bytes = bytes
            .checked_add(round_up(file.length(), granularity)?)
            .and_then(|value| value.checked_add(granularity))
            .ok_or(BudgetError::Overflow)?;
    }
    Ok(bytes)
}

fn scan_store_usage(
    engine_dir: &Path,
    granularity: u64,
    temporary_limit: u64,
) -> Result<PhysicalUsage, BudgetError> {
    scan_store_usage_with(engine_dir, granularity, temporary_limit, |_, _| {})
}

fn scan_store_usage_with(
    engine_dir: &Path,
    granularity: u64,
    temporary_limit: u64,
    mut visit: impl FnMut(&Path, bool),
) -> Result<PhysicalUsage, BudgetError> {
    let mut usage = PhysicalUsage::default();
    let stage_root = engine_dir.join("staging");
    let mut directories = vec![engine_dir.to_owned()];
    while let Some(directory) = directories.pop() {
        visit(&directory, true);
        let Some(current) = store_io(&directory, &stage_root, fs::symlink_metadata(&directory))?
        else {
            continue;
        };
        if !current.file_type().is_dir() {
            return Err(BudgetError::UnsafeStore(directory));
        }
        let Some(entries) = store_io(&directory, &stage_root, fs::read_dir(&directory))? else {
            continue;
        };
        for result in entries {
            let Some(entry) = store_io(&directory, &stage_root, result)? else {
                continue;
            };
            let path = entry.path();
            visit(&path, false);
            let Some(kind) = store_io(&path, &stage_root, entry.file_type())? else {
                continue;
            };
            if kind.is_dir() {
                usage.temporary = usage
                    .temporary
                    .checked_add(granularity)
                    .ok_or(BudgetError::Overflow)?;
                if usage.temporary > temporary_limit {
                    return Err(BudgetError::TemporaryLimit);
                }
                directories.push(path);
            } else if kind.is_file() {
                let Some(current) = store_io(&path, &stage_root, fs::symlink_metadata(&path))?
                else {
                    continue;
                };
                if !current.file_type().is_file() {
                    return Err(BudgetError::UnsafeStore(path));
                }
                let Some(file) = store_io(&path, &stage_root, File::open(&path))? else {
                    continue;
                };
                let Some(opened) = store_io(&path, &stage_root, file.metadata())? else {
                    continue;
                };
                if !opened.file_type().is_file() {
                    return Err(BudgetError::UnsafeStore(path));
                }
                let Some(allocated) = store_io(&path, &stage_root, FileExt::allocated_size(&file))?
                else {
                    continue;
                };
                let logical = opened.len();
                let charge = round_up(allocated.max(logical), granularity)?
                    .checked_add(granularity)
                    .ok_or(BudgetError::Overflow)?;
                let bucket = if directory == engine_dir && entry.file_name() == "source.sqlite" {
                    &mut usage.main
                } else if directory == engine_dir && entry.file_name() == "source.sqlite-wal" {
                    &mut usage.wal
                } else {
                    &mut usage.temporary
                };
                *bucket = bucket.checked_add(charge).ok_or(BudgetError::Overflow)?;
                if usage.temporary > temporary_limit {
                    return Err(BudgetError::TemporaryLimit);
                }
            } else {
                return Err(BudgetError::UnsafeStore(path));
            }
        }
    }
    Ok(usage)
}

fn store_io<T>(
    path: &Path,
    stage_root: &Path,
    result: std::io::Result<T>,
) -> Result<Option<T>, BudgetError> {
    match result {
        Ok(value) => Ok(Some(value)),
        // A previous StagedTree/lease may be released in another process
        // while this writer scans. Its private stage name cannot be reused.
        // Available-space checks still count blocks pending physical deletion.
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound
                && path.starts_with(stage_root)
                && path != stage_root =>
        {
            Ok(None)
        }
        Err(source) => Err(BudgetError::Io {
            path: path.to_owned(),
            source,
        }),
    }
}

fn round_up(value: u64, granularity: u64) -> Result<u64, BudgetError> {
    let remainder = value % granularity;
    if remainder == 0 {
        Ok(value)
    } else {
        value
            .checked_add(granularity - remainder)
            .ok_or(BudgetError::Overflow)
    }
}

pub(crate) fn ensure_directory(path: &Path) -> Result<(), BudgetError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(()),
        Ok(_) => Err(BudgetError::UnsafeStore(path.to_owned())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            create_private_directory(path).map_err(|source| BudgetError::Io {
                path: path.to_owned(),
                source,
            })
        }
        Err(source) => Err(BudgetError::Io {
            path: path.to_owned(),
            source,
        }),
    }
}

pub(crate) fn create_private_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = fs::DirBuilder::new();
        builder.mode(0o700);
        builder.create(path)
    }
    #[cfg(not(unix))]
    {
        fs::create_dir(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SourcePath, scan_declared_tree};

    #[test]
    fn tiny_temporary_bucket_refuses_before_stage_creation() {
        let root = std::env::temp_dir().join(format!("thrum-budget-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("project.json"), b"{}").unwrap();
        let inventory =
            scan_declared_tree(&root, &[SourcePath::parse("project.json").unwrap()]).unwrap();
        let window = SourceWriterWindow::acquire(&root).unwrap();
        let limits = Limits {
            temporary: 0,
            ..Limits::V1
        };
        assert!(matches!(
            StageBudget::reserve_with_limits(&root, &inventory, window, limits),
            Err(BudgetError::TemporaryLimit)
        ));
        assert!(!root.join(".engine/staging").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn existing_store_usage_and_writer_lock_are_visible() {
        let root = std::env::temp_dir().join(format!("thrum-budget-store-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("project.json"), b"{}").unwrap();
        let window = SourceWriterWindow::acquire(&root).unwrap();
        assert!(matches!(
            SourceWriterWindow::acquire(&root),
            Err(BudgetError::Busy)
        ));
        fs::write(root.join(".engine/source.sqlite"), vec![1_u8; 4096]).unwrap();
        let inventory =
            scan_declared_tree(&root, &[SourcePath::parse("project.json").unwrap()]).unwrap();
        let limits = Limits {
            main: 1,
            ..Limits::V1
        };
        assert!(matches!(
            StageBudget::reserve_with_limits(&root, &inventory, window, limits),
            Err(BudgetError::MainLimit)
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn retiring_stage_during_store_scan_does_not_refuse_next_capture() {
        let root = std::env::temp_dir().join(format!("thrum-budget-retire-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let engine = root.join(".engine");
        let stage = engine.join("staging/old-stage");
        fs::create_dir_all(&stage).unwrap();
        let staged_file = stage.join("source.bin");
        fs::write(&staged_file, b"old").unwrap();

        let mut removed_at_entry = false;
        scan_store_usage_with(&engine, 4096, Limits::V1.temporary, |path, queued| {
            if !queued && path == staged_file {
                fs::remove_dir_all(&stage).unwrap();
                removed_at_entry = true;
            }
        })
        .unwrap();
        assert!(removed_at_entry);

        fs::create_dir(&stage).unwrap();
        fs::write(&staged_file, b"old").unwrap();
        let mut removed_when_queued = false;
        scan_store_usage_with(&engine, 4096, Limits::V1.temporary, |path, queued| {
            if queued && path == stage {
                fs::remove_dir_all(&stage).unwrap();
                removed_when_queued = true;
            }
        })
        .unwrap();
        assert!(removed_when_queued);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlink_lock_refuses_without_opening_its_target() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!("thrum-budget-link-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join(".engine")).unwrap();
        fs::write(root.join("victim"), b"keep").unwrap();
        symlink("../victim", root.join(".engine/source.lock")).unwrap();
        assert!(matches!(
            SourceWriterWindow::acquire(&root),
            Err(BudgetError::UnsafeStore(_))
        ));
        assert_eq!(fs::read(root.join("victim")).unwrap(), b"keep");
        fs::remove_dir_all(root).unwrap();
    }
}
