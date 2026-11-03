//! Temporary ordinary files for a native compiler from accepted source bytes.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use thrum_source::{AcceptedSourceRoot, PinnedSource, SourceStoreError};

/// Caller-selected planning targets for one temporary compiler input tree.
/// These are preflight estimates, not a reserved physical quota.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BuildScratchPlan {
    /// Maximum estimated bytes for this materialized tree and its metadata.
    pub temporary_target_bytes: u64,
    /// Desired observed free space after the projected copy.
    pub free_headroom_bytes: u64,
}

/// Failure to materialize one complete pinned source tree.
#[derive(Debug)]
pub enum MaterializeError {
    /// Destination must be absolute so owner cleanup cannot follow a new CWD.
    InvalidDestination,
    /// The selected planning target is zero or estimate arithmetic overflowed.
    InvalidPlan,
    /// Estimated temporary use or observed free space cannot fit the plan.
    SpacePressure,
    /// Native volume probe failed before creating the destination.
    Probe(io::Error),
    /// The destination could not be created as a new directory.
    Destination(io::Error),
    /// Creating an output path or counting copied bytes failed.
    Output {
        /// Destination path associated with the failure.
        path: PathBuf,
        /// Original filesystem error.
        source: io::Error,
    },
    /// A verified pinned copy or destination write failed; discard the partial tree.
    Source(SourceStoreError),
    /// Cleanup failed after a prior failure; the partial tree must not be used.
    Cleanup {
        /// Partial destination that still needs cleanup.
        path: PathBuf,
        /// Original materialization failure.
        cause: Box<MaterializeError>,
        /// Cleanup filesystem error.
        source: io::Error,
    },
}

/// Owned temporary build input tree; dropping it removes only its new root.
/// Keep this owner alive while an external compiler reads the files.
pub struct MaterializedSource {
    path: PathBuf,
    root: AcceptedSourceRoot,
    file_count: usize,
    copied_bytes: u64,
    live: bool,
}

impl MaterializedSource {
    /// Newly created directory containing the exact pinned file tree.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Accepted source root from which all bytes were verified.
    pub fn source_root(&self) -> AcceptedSourceRoot {
        self.root
    }

    /// Number of materialized files.
    pub fn file_count(&self) -> usize {
        self.file_count
    }

    /// Exact sum of verified file lengths.
    pub fn copied_bytes(&self) -> u64 {
        self.copied_bytes
    }

    /// Remove the temporary tree and report a cleanup failure.
    pub fn close(mut self) -> io::Result<()> {
        let result = fs::remove_dir_all(&self.path);
        // A failed explicit close belongs to the caller's recovery path.
        // Do not attempt a second, unreported path-based removal in Drop.
        self.live = false;
        result
    }

    /// Leave this registered input tree for an outer owner to inspect after
    /// restart, without an implicit path-based removal in `Drop`.
    /// The caller remains responsible for its lifetime and cleanup.
    pub fn retain_for_recovery(mut self) {
        self.live = false;
    }
}

impl Drop for MaterializedSource {
    fn drop(&mut self) {
        if self.live {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

/// Stream all files of one accepted pin into a new temporary build directory.
///
/// `destination` must be absolute and absent; its parent must be trusted and private to
/// the build tool. No mutable checkout path is read. A copy error cleans up the
/// partial tree before returning. These files are temporary build inputs, not
/// a durable product or a ready build; no per-file sync is performed.
pub fn materialize_pinned_source(
    pin: &mut PinnedSource,
    destination: &Path,
    plan: BuildScratchPlan,
) -> Result<MaterializedSource, MaterializeError> {
    if !destination.is_absolute() {
        return Err(MaterializeError::InvalidDestination);
    }
    preflight(pin, destination, plan)?;
    fs::create_dir(destination).map_err(MaterializeError::Destination)?;
    let result = materialize_files(pin, destination);
    match result {
        Ok((file_count, copied_bytes)) => Ok(MaterializedSource {
            path: destination.to_owned(),
            root: pin.root(),
            file_count,
            copied_bytes,
            live: true,
        }),
        Err(cause) => match fs::remove_dir_all(destination) {
            Ok(()) => Err(cause),
            Err(source) => Err(MaterializeError::Cleanup {
                path: destination.to_owned(),
                cause: Box::new(cause),
                source,
            }),
        },
    }
}

fn preflight(
    pin: &PinnedSource,
    destination: &Path,
    plan: BuildScratchPlan,
) -> Result<(), MaterializeError> {
    if plan.temporary_target_bytes == 0 {
        return Err(MaterializeError::InvalidPlan);
    }
    let parent = destination.parent().ok_or(MaterializeError::InvalidPlan)?;
    let granularity = fs4::allocation_granularity(parent).map_err(MaterializeError::Probe)?;
    if granularity == 0 {
        return Err(MaterializeError::InvalidPlan);
    }
    let available = fs4::available_space(parent).map_err(MaterializeError::Probe)?;
    let mut projected = granularity;
    // Manifest paths are sorted. Every directory prefix occupies one
    // contiguous range, so the previous path determines which prefixes are
    // new without a whole-tree hash set or repeated prefix hashing.
    let mut previous_path = "";
    for file in pin.manifest().files() {
        let (length, _) = file.content_identity();
        let rounded = length
            .checked_add(granularity - 1)
            .map(|value| value / granularity * granularity)
            .ok_or(MaterializeError::InvalidPlan)?;
        projected = projected
            .checked_add(rounded)
            .and_then(|value| value.checked_add(granularity))
            .ok_or(MaterializeError::InvalidPlan)?;
        let path = file.path().as_str();
        let new_directory_bytes = new_directory_count(previous_path, path)
            .checked_mul(granularity)
            .ok_or(MaterializeError::InvalidPlan)?;
        projected = projected
            .checked_add(new_directory_bytes)
            .ok_or(MaterializeError::InvalidPlan)?;
        previous_path = path;
    }
    if projected > plan.temporary_target_bytes
        || projected
            .checked_add(plan.free_headroom_bytes)
            .is_none_or(|needed| needed > available)
    {
        return Err(MaterializeError::SpacePressure);
    }
    Ok(())
}

fn new_directory_count(previous: &str, current: &str) -> u64 {
    let shared_bytes = previous
        .bytes()
        .zip(current.bytes())
        .take_while(|(left, right)| left == right)
        .count();
    current
        .bytes()
        .enumerate()
        .filter(|(index, byte)| *byte == b'/' && *index >= shared_bytes)
        .count() as u64
}

fn materialize_files(
    pin: &mut PinnedSource,
    destination: &Path,
) -> Result<(usize, u64), MaterializeError> {
    let mut copied_bytes = 0_u64;
    let mut previous_parent = PathBuf::new();
    let file_count = pin.manifest().files().len();
    for index in 0..file_count {
        let source_path = pin.manifest().files()[index].path().clone();
        let path = destination.join(source_path.as_str());
        let parent = path
            .parent()
            .expect("validated source path under destination");
        if parent != previous_parent {
            fs::create_dir_all(parent).map_err(|source| MaterializeError::Output {
                path: parent.to_owned(),
                source,
            })?;
            previous_parent = parent.to_owned();
        }
        let mut output = new_file(&path)?;
        let copied = pin
            .copy_file_to(&source_path, &mut output)
            .map_err(MaterializeError::Source)?;
        copied_bytes =
            copied_bytes
                .checked_add(copied)
                .ok_or_else(|| MaterializeError::Output {
                    path: path.clone(),
                    source: io::Error::other("materialized byte count overflow"),
                })?;
    }
    Ok((file_count, copied_bytes))
}

fn new_file(path: &Path) -> Result<File, MaterializeError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| MaterializeError::Output {
            path: path.to_owned(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::new_directory_count;

    #[test]
    fn sorted_paths_count_only_new_directory_prefixes() {
        let paths = ["a/b/first.rs", "a/b/second.rs", "a/c/third.rs", "z/root.rs"];
        let mut previous = "";
        let mut count = 0;
        for path in paths {
            count += new_directory_count(previous, path);
            previous = path;
        }
        assert_eq!(count, 4); // a, a/b, a/c, z
    }
}
