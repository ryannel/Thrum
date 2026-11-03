use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use fs4::{FileExt, TryLockError};
use thrum_cook::{
    BuildScratchPlan, MaterializeError, MaterializedSource, materialize_pinned_source,
};
use thrum_source::{NativeSourceStore, PinnedSource};

/// One host's configurable managed-temporary scheduling target and headroom.
/// These are planning values, not reserved filesystem bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JobSpacePlan {
    /// Maximum sum of conservative, active F4 materialization claims.
    pub managed_temporary_target_bytes: u64,
    /// Desired observed free space after all active claims and the proposed job.
    pub free_headroom_bytes: u64,
}

/// One active job's identity within the exclusive host owner.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct JobId {
    incarnation: u128,
    sequence: u64,
}

/// Why a requested complete phase claim was deferred before creating output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpacePressure {
    /// Existing plus proposed conservative claims exceed the selected target.
    ManagedTemporary {
        /// Fixed account charge for the private parent and owner lock.
        control_claim_bytes: u64,
        /// Sum of already admitted phase claims.
        active_claim_bytes: u64,
        /// Full proposed phase claim.
        proposed_claim_bytes: u64,
        /// Selected simultaneous target.
        target_bytes: u64,
    },
    /// One non-atomic free-space observation cannot fit all remaining claims.
    Available {
        /// Proposed phase claim plus selected headroom. Existing completed
        /// input trees are already reflected in observed free space.
        requested_bytes: u64,
        /// Observed available bytes on the destination volume.
        observed_available_bytes: u64,
    },
}

/// Failure to open, admit or close a first-profile native job.
#[derive(Debug)]
pub enum JobError {
    /// Zero target, invalid path ownership, alias or volume mismatch.
    InvalidPlanOrRoot,
    /// A cooperating host already holds the exclusive owner lock.
    Busy,
    /// A retained, incomplete or unknown job root needs inspection before work.
    Quarantined(PathBuf),
    /// Active and proposed work cannot fit the selected scheduling policy.
    SpacePressure(SpacePressure),
    /// Checked claim arithmetic overflowed.
    Overflow,
    /// An ID is not owned by this live host.
    UnknownJob(JobId),
    /// One or more input leases still retain this job's source tree.
    ReadersActive(JobId),
    /// The pin belongs to a different native source store than this project.
    SourceStoreMismatch {
        /// Canonical path selected by the host's live source-store owner.
        expected: PathBuf,
        /// Canonical path held by the pin's issuing source-store owner.
        actual: PathBuf,
    },
    /// Filesystem work failed; a registered job remains quarantined if cleanup is uncertain.
    Io {
        /// Native path associated with the failed operation.
        path: PathBuf,
        /// Original filesystem error.
        source: io::Error,
    },
    /// F4 refused materialization; its original cause is retained.
    Materialize(MaterializeError),
}

struct OwnedMaterialization {
    root: PathBuf,
    root_directory: File,
    input: Option<MaterializedSource>,
    input_directory: File,
    claim_bytes: u64,
}

impl Drop for OwnedMaterialization {
    fn drop(&mut self) {
        // Host destruction can precede the last reader. Keep the registered
        // tree after either owner leaves; restart must inspect it explicitly.
        if let Some(input) = self.input.take() {
            input.retain_for_recovery();
        }
    }
}

/// Owned read lifetime for one complete immutable F4 input tree.
/// Hold this through all reads, including a supervised compiler's lifetime.
pub struct InputLease {
    job: Arc<OwnedMaterialization>,
}

impl InputLease {
    /// Path valid while this lease is held by a cooperating reader.
    pub fn path(&self) -> &Path {
        self.job
            .input
            .as_ref()
            .expect("leased job retains its input")
            .path()
    }
}

impl std::ops::Deref for InputLease {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.path()
    }
}

/// Exclusive first-host account for F4 input jobs on one Unix filesystem.
///
/// Open a canonical, private job parent outside the entire source project.
/// Any root left by an earlier owner, including an unpaired or partial root,
/// refuses new work. The host owns each F4 input until checked close; dropping
/// it without close leaves registration behind for restart quarantine.
pub struct MaterializationHost {
    parent: PathBuf,
    source_store: PathBuf,
    parent_directory: File,
    _owner_lock: File,
    plan: JobSpacePlan,
    control_claim_bytes: u64,
    job_overhead_bytes: u64,
    incarnation: u128,
    next_id: u64,
    active_claim_bytes: u64,
    active: BTreeMap<JobId, Arc<OwnedMaterialization>>,
    quarantined: bool,
}

impl MaterializationHost {
    /// Acquire one cooperating owner and inventory pre-existing job roots.
    /// The caller creates the private sibling `<project-name>.thrum-jobs`
    /// beside the canonical project before opening. This method does not
    /// repair or delete uncertain roots.
    pub fn open(
        project_root: &Path,
        source_store: &NativeSourceStore,
        job_parent: &Path,
        plan: JobSpacePlan,
    ) -> Result<Self, JobError> {
        if plan.managed_temporary_target_bytes == 0
            || !project_root.is_absolute()
            || !job_parent.is_absolute()
        {
            return Err(JobError::InvalidPlanOrRoot);
        }
        let project =
            fs::canonicalize(project_root).map_err(|source| io_error(project_root, source))?;
        let source_store_path = source_store.store_path();
        let store_parent = source_store_path
            .parent()
            .ok_or(JobError::InvalidPlanOrRoot)?;
        if !source_store_path.is_absolute()
            || !store_parent.starts_with(&project)
            || store_parent == project
            || fs::canonicalize(store_parent).map_err(|source| io_error(store_parent, source))?
                != store_parent
        {
            return Err(JobError::InvalidPlanOrRoot);
        }
        let source_store = source_store_path.to_owned();
        let parent = fs::canonicalize(job_parent).map_err(|source| io_error(job_parent, source))?;
        let project_name = project.file_name().ok_or(JobError::InvalidPlanOrRoot)?;
        let mut job_name = project_name.to_os_string();
        job_name.push(".thrum-jobs");
        let expected_parent = project
            .parent()
            .ok_or(JobError::InvalidPlanOrRoot)?
            .join(job_name);
        if parent != job_parent || parent != expected_parent {
            return Err(JobError::InvalidPlanOrRoot);
        }
        let project_meta = fs::metadata(&project).map_err(|source| io_error(&project, source))?;
        let parent_meta =
            fs::symlink_metadata(&parent).map_err(|source| io_error(&parent, source))?;
        if !project_meta.is_dir()
            || !parent_meta.is_dir()
            || parent_meta.file_type().is_symlink()
            || project_meta.dev() != parent_meta.dev()
            || project_meta.uid() != parent_meta.uid()
            || parent_meta.permissions().mode() & 0o777 != 0o700
        {
            return Err(JobError::InvalidPlanOrRoot);
        }
        let lock_path = parent.join(".owner.lock");
        let parent_directory = File::open(&parent).map_err(|source| io_error(&parent, source))?;
        if !same_directory(&parent_directory, &parent)? {
            return Err(JobError::Quarantined(parent));
        }
        let owner_lock = open_owner_lock(&lock_path)?;
        match FileExt::try_lock(&owner_lock) {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(JobError::Busy),
            Err(TryLockError::Error(source)) => return Err(io_error(&lock_path, source)),
        }
        for entry in fs::read_dir(&parent).map_err(|source| io_error(&parent, source))? {
            let entry = entry.map_err(|source| io_error(&parent, source))?;
            if entry.file_name() != ".owner.lock" {
                return Err(JobError::Quarantined(entry.path()));
            }
        }
        let mut incarnation_bytes = [0_u8; 16];
        File::open("/dev/urandom")
            .and_then(|mut random| random.read_exact(&mut incarnation_bytes))
            .map_err(|source| io_error(Path::new("/dev/urandom"), source))?;
        let granularity =
            fs4::allocation_granularity(&parent).map_err(|source| io_error(&parent, source))?;
        let control_claim_bytes = granularity.checked_mul(2).ok_or(JobError::Overflow)?;
        let job_overhead_bytes = granularity.checked_mul(2).ok_or(JobError::Overflow)?;
        if granularity == 0 || control_claim_bytes > plan.managed_temporary_target_bytes {
            return Err(JobError::InvalidPlanOrRoot);
        }
        Ok(Self {
            parent,
            source_store,
            parent_directory,
            _owner_lock: owner_lock,
            plan,
            control_claim_bytes,
            job_overhead_bytes,
            incarnation: u128::from_be_bytes(incarnation_bytes),
            next_id: 1,
            active_claim_bytes: 0,
            active: BTreeMap::new(),
            quarantined: false,
        })
    }

    /// Charge a full F4 scratch phase, register its root durably, and copy
    /// exact pinned bytes. A deferral creates no job directory.
    pub fn materialize(
        &mut self,
        pin: &mut PinnedSource,
        scratch: BuildScratchPlan,
    ) -> Result<JobId, JobError> {
        self.materialize_with(pin, scratch, || {})
    }

    fn materialize_with(
        &mut self,
        pin: &mut PinnedSource,
        scratch: BuildScratchPlan,
        after_registration: impl FnOnce(),
    ) -> Result<JobId, JobError> {
        if self.quarantined {
            return Err(JobError::Quarantined(self.parent.clone()));
        }
        if let Err(error) = self.ensure_parent_identity() {
            self.quarantined = true;
            return Err(error);
        }
        if pin.store_path() != self.source_store {
            return Err(JobError::SourceStoreMismatch {
                expected: self.source_store.clone(),
                actual: pin.store_path().to_owned(),
            });
        }
        if scratch.temporary_target_bytes == 0 {
            return Err(JobError::InvalidPlanOrRoot);
        }
        let claim = scratch
            .temporary_target_bytes
            .checked_add(self.job_overhead_bytes)
            .ok_or(JobError::Overflow)?;
        let combined = self
            .active_claim_bytes
            .checked_add(claim)
            .ok_or(JobError::Overflow)?;
        let managed_total = self
            .control_claim_bytes
            .checked_add(combined)
            .ok_or(JobError::Overflow)?;
        if managed_total > self.plan.managed_temporary_target_bytes {
            return Err(JobError::SpacePressure(SpacePressure::ManagedTemporary {
                control_claim_bytes: self.control_claim_bytes,
                active_claim_bytes: self.active_claim_bytes,
                proposed_claim_bytes: claim,
                target_bytes: self.plan.managed_temporary_target_bytes,
            }));
        }
        // Materialize is synchronous under this owner. Earlier F4 jobs have
        // completed their copies, so their allocated bytes are already in the
        // filesystem sample; charging them again would double-count them.
        let requested = claim
            .checked_add(self.plan.free_headroom_bytes)
            .ok_or(JobError::Overflow)?;
        let available =
            fs4::available_space(&self.parent).map_err(|source| io_error(&self.parent, source))?;
        if requested > available {
            return Err(JobError::SpacePressure(SpacePressure::Available {
                requested_bytes: requested,
                observed_available_bytes: available,
            }));
        }
        let id = JobId {
            incarnation: self.incarnation,
            sequence: self.next_id,
        };
        let next_id = self.next_id.checked_add(1).ok_or(JobError::Overflow)?;
        let root = self
            .parent
            .join(format!("job-{:032x}-{:016x}", id.incarnation, id.sequence));
        if let Err(error) = self.ensure_parent_identity() {
            self.quarantined = true;
            return Err(error);
        }
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .map_err(|source| {
                self.quarantined = true;
                io_error(&root, source)
            })?;
        let root_directory = File::open(&root).map_err(|source| {
            self.quarantined = true;
            io_error(&root, source)
        })?;
        // A crash between root creation and record sync is discovered as an
        // unpaired root by the next host before any new admission.
        self.active_claim_bytes = combined;
        self.next_id = next_id;
        if let Err(error) = self
            .ensure_parent_identity()
            .and_then(|()| ensure_root_identity(&root_directory, &root))
        {
            self.quarantined = true;
            return Err(error);
        }
        if let Err(error) = register(&self.parent, &root, id, pin, claim) {
            self.quarantined = true;
            return Err(error);
        }
        if let Err(error) = self
            .ensure_parent_identity()
            .and_then(|()| ensure_root_identity(&root_directory, &root))
        {
            self.quarantined = true;
            return Err(error);
        }
        after_registration();
        match materialize_pinned_source(pin, &root.join("input"), scratch) {
            Ok(input) => {
                let input_directory = match File::open(input.path()) {
                    Ok(directory) => directory,
                    Err(source) => {
                        let path = input.path().to_owned();
                        input.retain_for_recovery();
                        self.quarantined = true;
                        return Err(io_error(&path, source));
                    }
                };
                let identity = self
                    .ensure_parent_identity()
                    .and_then(|()| ensure_root_identity(&root_directory, &root))
                    .and_then(|()| ensure_input_identity(&input_directory, input.path()));
                if let Err(error) = identity {
                    input.retain_for_recovery();
                    self.quarantined = true;
                    return Err(error);
                }
                self.active.insert(
                    id,
                    Arc::new(OwnedMaterialization {
                        root,
                        root_directory,
                        input: Some(input),
                        input_directory,
                        claim_bytes: claim,
                    }),
                );
                Ok(id)
            }
            Err(cause) => {
                if let Err(error) = self
                    .ensure_parent_identity()
                    .and_then(|()| ensure_root_identity(&root_directory, &root))
                    .and_then(|()| retire_empty_root(&self.parent, &root))
                {
                    self.quarantined = true;
                    return Err(error);
                }
                self.active_claim_bytes -= claim;
                Err(JobError::Materialize(cause))
            }
        }
    }

    /// Lease the complete immutable F4 input without blocking other jobs.
    pub fn input_lease(&self, id: JobId) -> Result<InputLease, JobError> {
        self.ensure_parent_identity()?;
        let job = self.active.get(&id).ok_or(JobError::UnknownJob(id))?;
        ensure_root_identity(&job.root_directory, &job.root)?;
        ensure_input_identity(
            &job.input_directory,
            job.input
                .as_ref()
                .expect("active job retains its input")
                .path(),
        )?;
        Ok(InputLease {
            job: Arc::clone(job),
        })
    }

    /// Release only after checked F4 cleanup and durable job-root retirement.
    /// Any failure retains the full claim and blocks later admission.
    pub fn close(&mut self, id: JobId) -> Result<(), JobError> {
        if let Err(error) = self.ensure_parent_identity() {
            self.quarantined = true;
            return Err(error);
        }
        let job = self.active.get(&id).ok_or(JobError::UnknownJob(id))?;
        if let Err(error) = ensure_root_identity(&job.root_directory, &job.root) {
            self.quarantined = true;
            return Err(error);
        }
        if let Err(error) = ensure_input_identity(
            &job.input_directory,
            job.input
                .as_ref()
                .expect("active job retains its input")
                .path(),
        ) {
            self.quarantined = true;
            return Err(error);
        }
        let job = self.active.remove(&id).expect("checked active job exists");
        let mut job = match Arc::try_unwrap(job) {
            Ok(job) => job,
            Err(job) => {
                self.active.insert(id, job);
                return Err(JobError::ReadersActive(id));
            }
        };
        if let Err(source) = job
            .input
            .take()
            .expect("active job retains its input")
            .close()
        {
            self.quarantined = true;
            return Err(io_error(&job.root.join("input"), source));
        }
        if let Err(error) = self
            .ensure_parent_identity()
            .and_then(|()| ensure_root_identity(&job.root_directory, &job.root))
            .and_then(|()| retire_empty_root(&self.parent, &job.root))
        {
            self.quarantined = true;
            return Err(error);
        }
        self.active_claim_bytes -= job.claim_bytes;
        Ok(())
    }

    /// Current conservative F4 phase claims, excluding F3 and compiler output.
    pub fn active_claim_bytes(&self) -> u64 {
        self.active_claim_bytes
    }

    fn ensure_parent_identity(&self) -> Result<(), JobError> {
        if same_directory(&self.parent_directory, &self.parent)?
            && same_file(&self._owner_lock, &self.parent.join(".owner.lock"))?
        {
            Ok(())
        } else {
            Err(JobError::Quarantined(self.parent.clone()))
        }
    }
}

fn same_directory(held: &File, path: &Path) -> Result<bool, JobError> {
    let opened = held.metadata().map_err(|source| io_error(path, source))?;
    let on_path = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(source) => return Err(io_error(path, source)),
    };
    Ok(on_path.file_type().is_dir()
        && opened.dev() == on_path.dev()
        && opened.ino() == on_path.ino()
        && on_path.permissions().mode() & 0o777 == 0o700)
}

fn same_file(held: &File, path: &Path) -> Result<bool, JobError> {
    let opened = held.metadata().map_err(|source| io_error(path, source))?;
    let on_path = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(source) => return Err(io_error(path, source)),
    };
    Ok(on_path.file_type().is_file()
        && opened.dev() == on_path.dev()
        && opened.ino() == on_path.ino())
}

fn ensure_root_identity(held: &File, path: &Path) -> Result<(), JobError> {
    if same_directory(held, path)? {
        Ok(())
    } else {
        Err(JobError::Quarantined(path.to_owned()))
    }
}

fn same_input_directory(held: &File, path: &Path) -> Result<bool, JobError> {
    let opened = held.metadata().map_err(|source| io_error(path, source))?;
    let on_path = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(source) => return Err(io_error(path, source)),
    };
    Ok(on_path.file_type().is_dir()
        && opened.dev() == on_path.dev()
        && opened.ino() == on_path.ino())
}

fn ensure_input_identity(held: &File, path: &Path) -> Result<(), JobError> {
    if same_input_directory(held, path)? {
        Ok(())
    } else {
        Err(JobError::Quarantined(path.to_owned()))
    }
}

fn open_owner_lock(path: &Path) -> Result<File, JobError> {
    for _ in 0..3 {
        let existing = match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_file() && metadata.nlink() == 1 => true,
            Ok(_) => return Err(JobError::InvalidPlanOrRoot),
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(source) => return Err(io_error(path, source)),
        };
        let opened = if existing {
            OpenOptions::new().read(true).write(true).open(path)
        } else {
            OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)
        };
        let file = match opened {
            Ok(file) => file,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::AlreadyExists | io::ErrorKind::NotFound
                ) =>
            {
                continue;
            }
            Err(source) => return Err(io_error(path, source)),
        };
        let on_path = fs::symlink_metadata(path).map_err(|source| io_error(path, source))?;
        let opened = file.metadata().map_err(|source| io_error(path, source))?;
        if !on_path.file_type().is_file()
            || on_path.nlink() != 1
            || opened.dev() != on_path.dev()
            || opened.ino() != on_path.ino()
        {
            return Err(JobError::InvalidPlanOrRoot);
        }
        return Ok(file);
    }
    Err(JobError::Busy)
}

fn register(
    parent: &Path,
    root: &Path,
    id: JobId,
    pin: &PinnedSource,
    claim: u64,
) -> Result<(), JobError> {
    sync_dir(parent)?;
    let record_path = root.join("registration");
    let mut record = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&record_path)
        .map_err(|source| io_error(&record_path, source))?;
    let revision = pin.root().revision;
    let mut revision_hex = String::with_capacity(64);
    for byte in revision.as_bytes() {
        use std::fmt::Write as _;
        write!(&mut revision_hex, "{byte:02x}").expect("writing to String cannot fail");
    }
    write!(
        record,
        "thrum-job-v1\nincarnation={:032x}\nsequence={}\nsource-generation={}\nsource-revision={}\nclass=managed-temporary\nphase=f4-input\nclaim-bytes={}\n",
        id.incarnation,
        id.sequence,
        pin.root().generation,
        revision_hex,
        claim,
    )
    .map_err(|source| io_error(&record_path, source))?;
    record
        .sync_all()
        .map_err(|source| io_error(&record_path, source))?;
    sync_dir(root)?;
    sync_dir(parent)
}

fn retire_empty_root(parent: &Path, root: &Path) -> Result<(), JobError> {
    for entry in fs::read_dir(root).map_err(|source| io_error(root, source))? {
        let entry = entry.map_err(|source| io_error(root, source))?;
        if entry.file_name() != "registration" {
            return Err(JobError::Quarantined(entry.path()));
        }
    }
    // The input tree was checked-closed before this barrier. The record stays
    // until that empty-root state has been synced.
    sync_dir(root)?;
    let record = root.join("registration");
    fs::remove_file(&record).map_err(|source| io_error(&record, source))?;
    sync_dir(root)?;
    fs::remove_dir(root).map_err(|source| io_error(root, source))?;
    sync_dir(parent)
}

fn sync_dir(path: &Path) -> Result<(), JobError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| io_error(path, source))
}

fn io_error(path: &Path, source: io::Error) -> JobError {
    JobError::Io {
        path: path.to_owned(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};
    use thrum_source::{
        NativeSourceStore, SourcePath, SourceSpacePlan, registry_stage::validate_staged_candidate,
        stage_declared_tree_with_plan,
    };

    const MIB: u64 = 1024 * 1024;
    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        base: PathBuf,
        project: PathBuf,
        jobs: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            Self::new_at(&std::env::temp_dir())
        }

        fn new_at(volume: &Path) -> Self {
            let number = NEXT.fetch_add(1, Ordering::Relaxed);
            let base = volume.join(format!("thrum-jobs-{}-{number}", std::process::id()));
            let project = base.join("project");
            let jobs = base.join("project.thrum-jobs");
            fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
            fs::create_dir(&project).unwrap();
            fs::DirBuilder::new().mode(0o700).create(&jobs).unwrap();
            fs::create_dir(project.join(".engine")).unwrap();
            fs::create_dir(project.join("scenes")).unwrap();
            fs::create_dir(project.join("schemas")).unwrap();
            fs::write(
                project.join("project.json"),
                br#"{"id":"2a5ef644-7225-4770-9f4d-e1d9c0a7d515","type":"core.project","schema":2,"fields":{"project.format":1,"project.records":["scenes/main.scene.json"],"project.schemas":[],"project.registryHistory":"schemas/history.registry.json","project.entryScenes":["3a5ef644-7225-4770-9f4d-e1d9c0a7d515"]}}"#,
            )
            .unwrap();
            fs::write(
                project.join("scenes/main.scene.json"),
                br#"{"id":"3a5ef644-7225-4770-9f4d-e1d9c0a7d515","type":"core.scene","schema":1,"fields":{"scene.label":"A"}}"#,
            )
            .unwrap();
            fs::write(
                project.join("schemas/history.registry.json"),
                br#"{"id":"6a5ef644-7225-4770-9f4d-e1d9c0a7d515","type":"core.registry-history","schema":1,"fields":{"registry.fields":[],"registry.versions":[]}}"#,
            )
            .unwrap();
            let mut master = File::create(project.join("master.bin")).unwrap();
            let chunk = [0x5a_u8; 4096];
            for _ in 0..(12 * MIB / 4096) {
                master.write_all(&chunk).unwrap();
            }
            let base = fs::canonicalize(base).unwrap();
            Self {
                project: base.join("project"),
                jobs: base.join("project.thrum-jobs"),
                base,
            }
        }

        fn accepted_store(&self) -> NativeSourceStore {
            let paths = declared_paths();
            let plan = source_plan();
            let store =
                NativeSourceStore::open(&self.project.join(".engine/source.sqlite"), plan).unwrap();
            let stage = stage_declared_tree_with_plan(&self.project, &paths, plan).unwrap();
            let candidate = validate_staged_candidate(&stage, &[]).unwrap();
            let accepted = store.admit(0, 1, 1, [1; 32], &candidate).unwrap();
            assert_eq!(accepted.generation, 1);
            store
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.base).unwrap();
        }
    }

    fn scratch() -> BuildScratchPlan {
        BuildScratchPlan {
            temporary_target_bytes: 16 * MIB,
            free_headroom_bytes: 0,
        }
    }

    fn source_plan() -> SourceSpacePlan {
        SourceSpacePlan {
            main_target_bytes: 1024 * MIB,
            wal_stop_bytes: 64 * MIB,
            temporary_target_bytes: 32 * MIB,
            free_headroom_bytes: 0,
        }
    }

    fn declared_paths() -> [SourcePath; 4] {
        [
            "master.bin",
            "project.json",
            "scenes/main.scene.json",
            "schemas/history.registry.json",
        ]
        .map(|path| SourcePath::parse(path).unwrap())
    }

    fn account() -> JobSpacePlan {
        JobSpacePlan {
            managed_temporary_target_bytes: 24 * MIB,
            free_headroom_bytes: 0,
        }
    }

    #[test]
    fn foreign_pin_with_identical_source_root_refuses_before_registration() {
        let selected = Fixture::new();
        let foreign = Fixture::new();
        let selected_store = selected.accepted_store();
        let foreign_store = foreign.accepted_store();
        let selected_pin = selected_store.pin_current(1).unwrap();
        let mut foreign_pin = foreign_store.pin_current(1).unwrap();
        assert_eq!(selected_pin.root(), foreign_pin.root());
        let mut host = MaterializationHost::open(
            &selected.project,
            &selected_store,
            &selected.jobs,
            account(),
        )
        .unwrap();
        assert!(matches!(
            host.materialize(&mut foreign_pin, scratch()),
            Err(JobError::SourceStoreMismatch { expected, actual })
                if expected == selected_store.store_path() && actual == foreign_store.store_path()
        ));
        assert_eq!(host.active_claim_bytes(), 0);
        assert_eq!(fs::read_dir(&selected.jobs).unwrap().count(), 1);
    }

    #[test]
    fn lock_symlink_refuses_before_touching_existing_or_absent_target() {
        let fixture = Fixture::new();
        let store = fixture.accepted_store();
        let lock = fixture.jobs.join(".owner.lock");
        let existing_victim = fixture.base.join("existing-victim");
        fs::write(&existing_victim, b"untouched").unwrap();
        symlink(&existing_victim, &lock).unwrap();
        assert!(matches!(
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()),
            Err(JobError::InvalidPlanOrRoot)
        ));
        assert_eq!(fs::read(&existing_victim).unwrap(), b"untouched");
        fs::remove_file(&lock).unwrap();
        let absent_victim = fixture.base.join("absent-victim");
        symlink(&absent_victim, &lock).unwrap();
        assert!(matches!(
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()),
            Err(JobError::InvalidPlanOrRoot)
        ));
        assert!(!absent_victim.exists());
    }

    #[test]
    fn second_parent_cannot_start_an_independent_account_for_the_same_project() {
        let fixture = Fixture::new();
        let store = fixture.accepted_store();
        let alternate = fixture.base.join("other-jobs");
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&alternate)
            .unwrap();
        assert!(matches!(
            MaterializationHost::open(&fixture.project, &store, &alternate, account()),
            Err(JobError::InvalidPlanOrRoot)
        ));
        assert_eq!(fs::read_dir(&alternate).unwrap().count(), 0);
        let host =
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()).unwrap();
        assert!(matches!(
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()),
            Err(JobError::Busy)
        ));
        drop(host);
    }

    #[test]
    fn f3_admits_b_while_a_registered_root_is_partial_then_f4_copies_a() {
        let fixture = Fixture::new();
        let store = fixture.accepted_store();
        let mut pin_a = store.pin_current(1).unwrap();
        let a_scene = fs::read(fixture.project.join("scenes/main.scene.json")).unwrap();
        let mut host =
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()).unwrap();
        let id = host
            .materialize_with(&mut pin_a, scratch(), || {
                let jobs = fs::read_dir(&fixture.jobs)
                    .unwrap()
                    .map(|entry| entry.unwrap().path())
                    .filter(|path| path.file_name().unwrap().to_string_lossy().starts_with("job-"))
                    .collect::<Vec<_>>();
                assert_eq!(jobs.len(), 1);
                assert!(jobs[0].join("registration").is_file());
                assert!(!jobs[0].join("input").exists());
                fs::write(
                    fixture.project.join("scenes/main.scene.json"),
                    br#"{"id":"3a5ef644-7225-4770-9f4d-e1d9c0a7d515","type":"core.scene","schema":1,"fields":{"scene.label":"B"}}"#,
                )
                .unwrap();
                let stage = stage_declared_tree_with_plan(
                    &fixture.project,
                    &declared_paths(),
                    source_plan(),
                )
                .unwrap();
                let candidate = validate_staged_candidate(&stage, &[]).unwrap();
                let admitted_b = store.admit(1, 2, 2, [2; 32], &candidate).unwrap();
                assert_eq!(admitted_b.generation, 2);
            })
            .unwrap();
        let input = host.input_lease(id).unwrap();
        assert_eq!(
            fs::read(input.join("scenes/main.scene.json")).unwrap(),
            a_scene
        );
        assert_eq!(pin_a.root().generation, 1);
        let pin_b = store.pin_current(2).unwrap();
        assert_eq!(pin_b.root().generation, 2);
        drop(pin_b);
        let stage_with_complete_a =
            stage_declared_tree_with_plan(&fixture.project, &declared_paths(), source_plan())
                .unwrap();
        validate_staged_candidate(&stage_with_complete_a, &[]).unwrap();
        drop(input);
        host.close(id).unwrap();
        assert_eq!(host.active_claim_bytes(), 0);
    }

    #[test]
    fn active_reader_blocks_only_its_own_refund_while_another_job_is_admitted() {
        let fixture = Fixture::new();
        let store = fixture.accepted_store();
        let mut pin = store.pin_current(1).unwrap();
        let plan = JobSpacePlan {
            managed_temporary_target_bytes: 40 * MIB,
            free_headroom_bytes: 0,
        };
        let mut host =
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, plan).unwrap();
        let a = host.materialize(&mut pin, scratch()).unwrap();
        let reader = host.input_lease(a).unwrap();
        let second_reader = host.input_lease(a).unwrap();
        let b = host.materialize(&mut pin, scratch()).unwrap();
        assert!(reader.join("master.bin").is_file());
        let both_claims = host.active_claim_bytes();
        assert!(matches!(host.close(a), Err(JobError::ReadersActive(id)) if id == a));
        assert_eq!(host.active_claim_bytes(), both_claims);
        assert!(reader.join("master.bin").is_file());
        host.close(b).unwrap();
        assert!(host.active_claim_bytes() > 0);
        drop(reader);
        assert!(matches!(host.close(a), Err(JobError::ReadersActive(id)) if id == a));
        drop(second_reader);
        host.close(a).unwrap();
        assert_eq!(host.active_claim_bytes(), 0);
        drop(host);
        MaterializationHost::open(&fixture.project, &store, &fixture.jobs, plan).unwrap();
    }

    #[test]
    fn lease_outliving_host_preserves_registered_input_for_restart_quarantine() {
        let fixture = Fixture::new();
        let store = fixture.accepted_store();
        let mut pin = store.pin_current(1).unwrap();
        let mut host =
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()).unwrap();
        let id = host.materialize(&mut pin, scratch()).unwrap();
        let reader = host.input_lease(id).unwrap();
        let root = reader.parent().unwrap().to_owned();
        drop(host);
        assert!(root.join("registration").is_file());
        assert!(reader.join("master.bin").is_file());
        drop(reader);
        assert!(root.join("input/master.bin").is_file());
        assert!(matches!(
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()),
            Err(JobError::Quarantined(path)) if path == root
        ));
    }

    #[test]
    fn f3_admits_b_while_a_complete_input_has_a_live_reader() {
        let fixture = Fixture::new();
        let store = fixture.accepted_store();
        let mut pin_a = store.pin_current(1).unwrap();
        let original_scene = fs::read(fixture.project.join("scenes/main.scene.json")).unwrap();
        let mut host =
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()).unwrap();
        let a = host.materialize(&mut pin_a, scratch()).unwrap();
        let reader = host.input_lease(a).unwrap();
        fs::write(
            fixture.project.join("scenes/main.scene.json"),
            br#"{"id":"3a5ef644-7225-4770-9f4d-e1d9c0a7d515","type":"core.scene","schema":1,"fields":{"scene.label":"B"}}"#,
        )
        .unwrap();
        let staged =
            stage_declared_tree_with_plan(&fixture.project, &declared_paths(), source_plan())
                .unwrap();
        let candidate = validate_staged_candidate(&staged, &[]).unwrap();
        let b = store.admit(1, 2, 2, [2; 32], &candidate).unwrap();
        assert_eq!(b.generation, 2);
        assert_eq!(
            fs::read(reader.join("scenes/main.scene.json")).unwrap(),
            original_scene
        );
        assert_eq!(pin_a.root().generation, 1);
        assert!(matches!(host.close(a), Err(JobError::ReadersActive(id)) if id == a));
        drop(reader);
        host.close(a).unwrap();
        assert_eq!(store.current_root().unwrap(), Some(b));
    }

    #[test]
    fn process_kill_after_registration_child() {
        let Some(project) = std::env::var_os("THRUM_JOB_KILL_PROJECT") else {
            return;
        };
        let project = PathBuf::from(project);
        let jobs = PathBuf::from(std::env::var_os("THRUM_JOB_KILL_JOBS").unwrap());
        let marker = PathBuf::from(std::env::var_os("THRUM_JOB_KILL_MARKER").unwrap());
        let store =
            NativeSourceStore::open(&project.join(".engine/source.sqlite"), source_plan()).unwrap();
        let mut pin = store.pin_current(1).unwrap();
        let mut host = MaterializationHost::open(&project, &store, &jobs, account()).unwrap();
        let _ = host.materialize_with(&mut pin, scratch(), || {
            fs::write(&marker, b"registered").unwrap();
            loop {
                std::thread::park();
            }
        });
    }

    #[test]
    fn competing_process_owner_child() {
        let Some(project) = std::env::var_os("THRUM_JOB_LOCK_PROJECT") else {
            return;
        };
        let project = PathBuf::from(project);
        let jobs = PathBuf::from(std::env::var_os("THRUM_JOB_LOCK_JOBS").unwrap());
        let store =
            NativeSourceStore::open(&project.join(".engine/source.sqlite"), source_plan()).unwrap();
        assert!(matches!(
            MaterializationHost::open(&project, &store, &jobs, account()),
            Err(JobError::Busy)
        ));
    }

    #[test]
    fn second_process_cannot_acquire_live_host_lock() {
        let fixture = Fixture::new();
        let store = fixture.accepted_store();
        let host =
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()).unwrap();
        drop(store);
        let output = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("unix::tests::competing_process_owner_child")
            .arg("--nocapture")
            .env("THRUM_JOB_LOCK_PROJECT", &fixture.project)
            .env("THRUM_JOB_LOCK_JOBS", &fixture.jobs)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "child owner check failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        drop(host);
    }

    #[test]
    fn killed_registered_job_is_quarantined_on_restart() {
        let fixture = Fixture::new();
        let store = fixture.accepted_store();
        drop(store);
        let marker = fixture.base.join("registered.ready");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("unix::tests::process_kill_after_registration_child")
            .arg("--nocapture")
            .env("THRUM_JOB_KILL_PROJECT", &fixture.project)
            .env("THRUM_JOB_KILL_JOBS", &fixture.jobs)
            .env("THRUM_JOB_KILL_MARKER", &marker)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(15);
        while !marker.is_file() {
            if Instant::now() > deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("child did not reach durable registration");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        child.kill().unwrap();
        assert!(!child.wait().unwrap().success());
        let store = NativeSourceStore::open(
            &fixture.project.join(".engine/source.sqlite"),
            source_plan(),
        )
        .unwrap();
        assert!(matches!(
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()),
            Err(JobError::Quarantined(_))
        ));
    }

    #[test]
    fn rejected_f4_copy_retires_registration_and_refunds_claim() {
        let fixture = Fixture::new();
        let store = fixture.accepted_store();
        let mut pin = store.pin_current(1).unwrap();
        let mut host =
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()).unwrap();
        let undersized = BuildScratchPlan {
            temporary_target_bytes: 1,
            free_headroom_bytes: 0,
        };
        assert!(matches!(
            host.materialize(&mut pin, undersized),
            Err(JobError::Materialize(_))
        ));
        assert_eq!(host.active_claim_bytes(), 0);
        assert_eq!(fs::read_dir(&fixture.jobs).unwrap().count(), 1);
        let id = host.materialize(&mut pin, scratch()).unwrap();
        host.close(id).unwrap();
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires THRUM_TEST_ENOSPC_VOLUME_DIR on an exclusive task-owned APFS image"]
    fn registered_f4_mid_copy_enospc_retires_root_and_refunds_only_after_cleanup() {
        use thrum_source::SourceStoreErrorKind;

        struct FillerGuard(PathBuf);
        impl Drop for FillerGuard {
            fn drop(&mut self) {
                let _ = fs::remove_file(&self.0);
            }
        }

        let volume = PathBuf::from(
            std::env::var_os("THRUM_TEST_ENOSPC_VOLUME_DIR")
                .expect("set THRUM_TEST_ENOSPC_VOLUME_DIR to a task-owned APFS mount"),
        );
        let total = fs4::total_space(&volume).unwrap();
        assert!((64 * MIB..=512 * MIB).contains(&total));
        let volume_lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(volume.join("thrum-native-enospc-test.lock"))
            .unwrap();
        FileExt::lock(&volume_lock).unwrap();

        let fixture = Fixture::new_at(&volume);
        let filler_path = fixture.base.join("filler.bin");
        let _filler_guard = FillerGuard(filler_path.clone());
        let store = fixture.accepted_store();
        let mut pin = store.pin_current(1).unwrap();
        let mut host =
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()).unwrap();
        let mut before_fill = 0_u64;
        let mut after_fill = 0_u64;
        let failed = host.materialize_with(&mut pin, scratch(), || {
            let registered = fs::read_dir(&fixture.jobs)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .find(|path| {
                    path.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("job-")
                })
                .unwrap();
            assert!(registered.join("registration").is_file());
            assert!(!registered.join("input").exists());
            before_fill = fs4::available_space(&fixture.jobs).unwrap();
            assert!(before_fill >= 16 * MIB);
            let mut filler = File::create(&filler_path).unwrap();
            let bytes = [0x51_u8; MIB as usize];
            let mut blocks = 0_u64;
            loop {
                let available = fs4::available_space(&fixture.jobs).unwrap();
                if available <= 14 * MIB {
                    break;
                }
                if let Err(error) = filler.write_all(&bytes) {
                    assert_eq!(error.raw_os_error(), Some(28));
                    break;
                }
                blocks += 1;
                assert!(blocks <= 512, "fault volume did not fill as planned");
            }
            after_fill = fs4::available_space(&fixture.jobs).unwrap();
            assert!(after_fill < 15 * MIB);
        });
        let cause = match failed {
            Err(JobError::Materialize(MaterializeError::Source(cause))) => cause,
            other => panic!("expected registered native F4 output fault: {other:?}"),
        };
        assert_eq!(cause.kind(), SourceStoreErrorKind::Output);
        let native = std::error::Error::source(&cause)
            .and_then(|error| error.downcast_ref::<io::Error>())
            .and_then(io::Error::raw_os_error);
        assert_eq!(native, Some(28));
        assert_eq!(host.active_claim_bytes(), 0);
        assert_eq!(fs::read_dir(&fixture.jobs).unwrap().count(), 1);
        eprintln!("t2_f4_native_enospc before_fill={before_fill} after_fill={after_fill}");

        fs::remove_file(&filler_path).unwrap();
        let retried = host.materialize(&mut pin, scratch()).unwrap();
        let reader = host.input_lease(retried).unwrap();
        let mut master = File::open(reader.join("master.bin")).unwrap();
        let mut verified_bytes = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let count = master.read(&mut buffer).unwrap();
            if count == 0 {
                break;
            }
            assert!(buffer[..count].iter().all(|byte| *byte == 0x5a));
            verified_bytes += count as u64;
        }
        assert_eq!(verified_bytes, 12 * MIB);
        assert_eq!(store.current_root().unwrap(), Some(pin.root()));
        drop(master);
        drop(reader);
        host.close(retried).unwrap();
        assert_eq!(host.active_claim_bytes(), 0);
        pin.close().unwrap();
    }

    #[test]
    fn uncertain_cleanup_retains_claim_and_refuses_more_work() {
        let fixture = Fixture::new();
        let store = fixture.accepted_store();
        let mut pin = store.pin_current(1).unwrap();
        let mut host =
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()).unwrap();
        let id = host.materialize(&mut pin, scratch()).unwrap();
        let root = host.input_lease(id).unwrap().parent().unwrap().to_owned();
        fs::write(root.join("unexpected"), b"unknown retained owner").unwrap();
        assert!(matches!(host.close(id), Err(JobError::Quarantined(_))));
        assert!(host.active_claim_bytes() > 0);
        assert!(matches!(
            host.materialize(&mut pin, scratch()),
            Err(JobError::Quarantined(_))
        ));
        assert!(root.join("registration").is_file());
        drop(host);
        assert!(matches!(
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()),
            Err(JobError::Quarantined(path)) if path == root
        ));
    }

    #[test]
    fn two_individually_affordable_copies_share_one_conservative_claim_target() {
        let fixture = Fixture::new();
        let store = fixture.accepted_store();
        let mut pin = store.pin_current(1).unwrap();
        let mut host =
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()).unwrap();
        let first = host.materialize(&mut pin, scratch()).unwrap();
        let granularity = fs4::allocation_granularity(&fixture.jobs).unwrap();
        let root_claim = 16 * MIB + 2 * granularity;
        assert!(
            host.input_lease(first)
                .unwrap()
                .join("master.bin")
                .is_file()
        );
        assert_eq!(host.active_claim_bytes(), root_claim);
        let refused = host.materialize(&mut pin, scratch()).unwrap_err();
        assert!(matches!(
            refused,
            JobError::SpacePressure(SpacePressure::ManagedTemporary {
                control_claim_bytes,
                active_claim_bytes,
                proposed_claim_bytes,
                target_bytes,
            }) if control_claim_bytes == 2 * granularity
                && active_claim_bytes == root_claim
                && proposed_claim_bytes == root_claim
                && target_bytes == 24 * MIB
        ));
        assert_eq!(
            fs::read_dir(&fixture.jobs)
                .unwrap()
                .filter(|entry| entry
                    .as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("job-"))
                .count(),
            1
        );
        host.close(first).unwrap();
        assert_eq!(host.active_claim_bytes(), 0);
        let second = host.materialize(&mut pin, scratch()).unwrap();
        assert_ne!(first, second);
        assert!(matches!(host.close(first), Err(JobError::UnknownJob(id)) if id == first));
        assert!(
            host.input_lease(second)
                .unwrap()
                .join("master.bin")
                .is_file()
        );
        host.close(second).unwrap();
        assert_eq!(host.active_claim_bytes(), 0);
        drop(host);
        let mut reopened =
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()).unwrap();
        let third = reopened.materialize(&mut pin, scratch()).unwrap();
        assert_ne!(third, first);
        assert_ne!(third, second);
        assert!(matches!(reopened.close(first), Err(JobError::UnknownJob(id)) if id == first));
        reopened.close(third).unwrap();
        pin.close().unwrap();
        drop(store);
        assert_eq!(fs::read_dir(&fixture.jobs).unwrap().count(), 1);
    }

    #[test]
    fn restart_quarantines_retained_registration_and_held_owner_excludes_peer() {
        let fixture = Fixture::new();
        let store = fixture.accepted_store();
        let mut pin = store.pin_current(1).unwrap();
        let mut host =
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()).unwrap();
        assert!(matches!(
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()),
            Err(JobError::Busy)
        ));
        let id = host.materialize(&mut pin, scratch()).unwrap();
        let root = host.input_lease(id).unwrap().parent().unwrap().to_owned();
        pin.close().unwrap();
        drop(host);
        assert!(root.join("registration").is_file());
        assert!(root.join("input/master.bin").is_file());
        assert!(matches!(
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()),
            Err(JobError::Quarantined(path)) if path == root
        ));
    }

    #[test]
    fn replaced_parent_cannot_receive_old_owner_cleanup() {
        let fixture = Fixture::new();
        let store = fixture.accepted_store();
        let mut pin = store.pin_current(1).unwrap();
        let mut host =
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()).unwrap();
        let id = host.materialize(&mut pin, scratch()).unwrap();
        let root = host.input_lease(id).unwrap().parent().unwrap().to_owned();
        let old_parent = fixture.base.join("old-jobs");
        fs::rename(&fixture.jobs, &old_parent).unwrap();
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&fixture.jobs)
            .unwrap();
        let replacement = fixture.jobs.join(root.file_name().unwrap());
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&replacement)
            .unwrap();
        fs::create_dir(replacement.join("input")).unwrap();
        fs::write(replacement.join("input/sentinel"), b"replacement owner").unwrap();
        assert!(matches!(host.close(id), Err(JobError::Quarantined(_))));
        assert_eq!(
            host.active_claim_bytes(),
            16 * MIB + 2 * fs4::allocation_granularity(&fixture.jobs).unwrap()
        );
        drop(host);
        assert_eq!(
            fs::read(replacement.join("input/sentinel")).unwrap(),
            b"replacement owner"
        );
        assert!(
            old_parent
                .join(root.file_name().unwrap())
                .join("input/master.bin")
                .is_file()
        );
        pin.close().unwrap();
    }

    #[test]
    fn replaced_job_root_cannot_receive_old_owner_cleanup() {
        let fixture = Fixture::new();
        let store = fixture.accepted_store();
        let mut pin = store.pin_current(1).unwrap();
        let mut host =
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()).unwrap();
        let id = host.materialize(&mut pin, scratch()).unwrap();
        let root = host.input_lease(id).unwrap().parent().unwrap().to_owned();
        let moved = fixture.jobs.join("moved-job");
        fs::rename(&root, &moved).unwrap();
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        fs::create_dir(root.join("input")).unwrap();
        fs::write(root.join("input/sentinel"), b"replacement job").unwrap();
        assert!(matches!(host.close(id), Err(JobError::Quarantined(path)) if path == root));
        drop(host);
        assert_eq!(
            fs::read(root.join("input/sentinel")).unwrap(),
            b"replacement job"
        );
        assert!(moved.join("input/master.bin").is_file());
        pin.close().unwrap();
    }

    #[test]
    fn replaced_input_cannot_receive_old_owner_cleanup() {
        let fixture = Fixture::new();
        let store = fixture.accepted_store();
        let mut pin = store.pin_current(1).unwrap();
        let mut host =
            MaterializationHost::open(&fixture.project, &store, &fixture.jobs, account()).unwrap();
        let id = host.materialize(&mut pin, scratch()).unwrap();
        let input = host.input_lease(id).unwrap().to_owned();
        let original = input.with_file_name("old-input");
        fs::rename(&input, &original).unwrap();
        fs::create_dir(&input).unwrap();
        fs::write(input.join("sentinel"), b"replacement input").unwrap();
        assert!(matches!(host.close(id), Err(JobError::Quarantined(path)) if path == input));
        drop(host);
        assert_eq!(
            fs::read(input.join("sentinel")).unwrap(),
            b"replacement input"
        );
        assert!(original.join("master.bin").is_file());
        pin.close().unwrap();
    }
}
