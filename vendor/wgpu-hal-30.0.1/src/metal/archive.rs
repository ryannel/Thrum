//! Metal pipeline caches use a file-backed MTLBinaryArchive.

use alloc::{string::ToString as _, vec::Vec};
use core::{
    cell::Cell,
    ffi::CStr,
    sync::atomic::{AtomicU64, Ordering},
};
use std::{
    env, fs, io,
    os::unix::{ffi::OsStrExt, fs::DirBuilderExt},
    path::{Path, PathBuf},
};

use objc2::{available, rc::Retained, runtime::ProtocolObject};
use objc2_foundation::{NSArray, NSError, NSProcessInfo, NSURL};
use objc2_metal::{
    MTLBinaryArchive, MTLBinaryArchiveDescriptor, MTLDevice, MTLGPUFamily, MTLPipelineOption,
};
use parking_lot::Mutex;

type NativeArchive = ProtocolObject<dyn MTLBinaryArchive>;

pub(super) fn supported(device: &ProtocolObject<dyn MTLDevice>) -> bool {
    available!(macos = 11.0, ios = 14.0, tvos = 14.0, visionos = 1.0)
        && (device.supportsFamily(MTLGPUFamily::Apple3)
            || device.supportsFamily(MTLGPUFamily::Mac2))
}

pub(super) fn validation_key(device: &ProtocolObject<dyn MTLDevice>) -> Option<[u8; 16]> {
    if !supported(device) {
        return None;
    }
    // The OS build appears in this string. Invalidate the archive when Metal changes.
    let os = NSProcessInfo::processInfo().operatingSystemVersionString();
    let revision = os
        .to_string()
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        });
    let mut key = [0; 16];
    key[..8].copy_from_slice(&device.registryID().to_be_bytes());
    key[8..].copy_from_slice(&revision.to_be_bytes());
    Some(key)
}

/// A Metal binary archive, either under construction or opened for reading.
#[derive(Debug)]
pub struct PipelineCache {
    archive: Mutex<Archive>,
}

#[derive(Debug)]
struct Archive {
    read: Vec<Retained<NativeArchive>>,
    source: Option<Vec<u8>>,
    output: Option<Retained<NativeArchive>>,
    harvest: bool,
    changed: bool,
    repair_failed: bool,
}

// The archive belongs to the Metal device, which can be used across threads.
// The cache mutex protects all archive access, including the synchronous
// descriptor call that searches it. No native reference escapes that call.
unsafe impl Send for Archive {}

// Metal cannot serialize an imported archive with new entries without losing
// existing entries. Preserve immutable archive bytes and append each repair as
// a separate native archive. A single archive still uses the original format.
const BUNDLE: &[u8] = b"MTLBA\x01\0\0";

fn load_archive(
    device: &ProtocolObject<dyn MTLDevice>,
    bytes: &[u8],
) -> io::Result<Retained<NativeArchive>> {
    let descriptor = MTLBinaryArchiveDescriptor::new();
    let file = if bytes.is_empty() {
        None
    } else {
        let file = ArchiveFile::new()?;
        fs::write(&file.path, bytes)?;
        descriptor.setUrl(Some(&file.url));
        Some(file)
    };
    let result = device.newBinaryArchiveWithDescriptor_error(&descriptor);
    drop(file);
    result.map_err(io::Error::other)
}

fn load_bundle(
    device: &ProtocolObject<dyn MTLDevice>,
    bytes: &[u8],
) -> io::Result<Vec<Retained<NativeArchive>>> {
    let Some(mut tail) = bytes.strip_prefix(BUNDLE) else {
        return Ok(alloc::vec![load_archive(device, bytes)?]);
    };
    let invalid = || io::Error::new(io::ErrorKind::InvalidData, "malformed Metal archive bundle");
    let mut archives = Vec::new();
    while !tail.is_empty() {
        let (length, remaining) = tail.split_first_chunk::<8>().ok_or_else(invalid)?;
        let length = usize::try_from(u64::from_le_bytes(*length)).map_err(|_| invalid())?;
        let (archive, remaining) = remaining.split_at_checked(length).ok_or_else(invalid)?;
        if archive.is_empty() {
            return Err(invalid());
        }
        archives.push(load_archive(device, archive)?);
        tail = remaining;
    }
    if archives.is_empty() {
        return Err(invalid());
    }
    Ok(archives)
}

fn append_segment(bundle: &mut Vec<u8>, archive: &[u8]) {
    bundle.extend_from_slice(&(archive.len() as u64).to_le_bytes());
    bundle.extend_from_slice(archive);
}

impl PipelineCache {
    pub(super) fn new(
        device: &ProtocolObject<dyn MTLDevice>,
        data: Option<&[u8]>,
    ) -> Result<Self, crate::PipelineCacheError> {
        if !supported(device) {
            return Err(crate::DeviceError::Unexpected.into());
        }
        let mut archive = Archive {
            read: Vec::new(),
            source: None,
            output: None,
            harvest: data.is_none(),
            changed: false,
            repair_failed: false,
        };
        let mode = ARCHIVE_POLICY.with(Cell::get).map(|(mode, _)| mode);
        let load = match data {
            Some(bytes) => load_bundle(device, bytes).map(|read| {
                archive.read = read;
                if mode == Some(Policy::Repair) {
                    archive.source = Some(bytes.to_vec());
                }
            }),
            None => load_archive(device, &[]).map(|raw| archive.output = Some(raw)),
        };
        if let Err(error) = load {
            log::warn!("opening Metal binary archive: {error}");
            // wgpu-core treats all HAL cache creation errors as device loss.
            // Report an unavailable cache separately when the caller handles it.
            if mode.is_some_and(|mode| data.is_some() || mode == Policy::Repair) {
                observe(ArchiveOutcome::Unavailable);
            } else {
                return Err(crate::DeviceError::Unexpected.into());
            }
        }
        Ok(Self {
            archive: Mutex::new(archive),
        })
    }

    pub(super) fn data(&self) -> Option<Vec<u8>> {
        let archive = self.archive.lock();
        let raw = archive.output.as_ref()?;
        if archive.repair_failed {
            return None;
        }
        if !archive.changed {
            // Metal cannot serialize an empty archive. An empty payload opens a
            // fresh, read-only native archive on load.
            return archive.harvest.then(Vec::new);
        }
        let serialize = || {
            let file = ArchiveFile::new()?;
            raw.serializeToURL_error(&file.url)
                .map_err(io::Error::other)?;
            fs::read(&file.path)
        };
        let added = serialize()
            .map_err(|error| log::error!("serializing Metal binary archive: {error}"))
            .ok()?;
        let Some(source) = archive.source.as_deref().filter(|bytes| !bytes.is_empty()) else {
            return Some(added);
        };
        let mut bundle = if source.starts_with(BUNDLE) {
            source.to_vec()
        } else {
            let mut bundle = BUNDLE.to_vec();
            append_segment(&mut bundle, source);
            bundle
        };
        append_segment(&mut bundle, &added);
        Some(bundle)
    }
}

/// Result of looking for a pipeline in a read-only Metal binary archive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveOutcome {
    /// No read-only archive was consulted.
    Unused,
    /// Metal created the pipeline in fail-on-miss mode.
    Hit,
    /// Metal reported a missing archive entry.
    Miss,
    /// The native cache file could not be opened.
    Unavailable,
    /// Metal returned a different pipeline error.
    Error,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Policy {
    Fallback,
    Strict,
    Repair,
}

type Observation = Option<(Policy, ArchiveOutcome)>;

thread_local! {
    static ARCHIVE_POLICY: Cell<Observation> = const { Cell::new(None) };
}

fn observe(outcome: ArchiveOutcome) {
    ARCHIVE_POLICY.with(|policy| {
        if let Some((mode, _)) = policy.get() {
            policy.set(Some((mode, outcome)));
        }
    });
}

/// Ask Metal to report hits and misses during a synchronous pipeline creation
/// or a failure to open an archive during a synchronous cache import.
/// In strict mode a miss propagates as a pipeline error. In normal mode it
/// retries the pipeline without the archive.
pub fn with_archive_policy<T>(strict: bool, create: impl FnOnce() -> T) -> (T, ArchiveOutcome) {
    let mode = if strict {
        Policy::Strict
    } else {
        Policy::Fallback
    };
    with_policy(mode, create)
}

/// Add missing pipelines to an existing archive during startup, or populate a
/// fresh cache. Failed repair leaves normal pipeline creation available and
/// disables serialization of the incomplete cache.
pub fn with_archive_repair<T>(create: impl FnOnce() -> T) -> (T, ArchiveOutcome) {
    with_policy(Policy::Repair, create)
}

fn with_policy<T>(mode: Policy, create: impl FnOnce() -> T) -> (T, ArchiveOutcome) {
    struct Restore<'a>(&'a Cell<Observation>, Observation);
    impl Drop for Restore<'_> {
        fn drop(&mut self) {
            self.0.set(self.1);
        }
    }
    ARCHIVE_POLICY.with(|policy| {
        let _restore = Restore(policy, policy.replace(Some((mode, ArchiveOutcome::Unused))));
        let result = create();
        let outcome = policy
            .get()
            .map_or(ArchiveOutcome::Unused, |(_, outcome)| outcome);
        (result, outcome)
    })
}

pub(super) fn create_pipeline<T>(
    device: &ProtocolObject<dyn MTLDevice>,
    cache: Option<&PipelineCache>,
    attach: impl Fn(Option<&NSArray<NativeArchive>>),
    add: impl FnOnce(&NativeArchive) -> Result<(), Retained<NSError>>,
    create: impl Fn(MTLPipelineOption) -> Result<T, Retained<NSError>>,
) -> Result<T, Retained<NSError>> {
    let Some(cache) = cache else {
        return create(MTLPipelineOption::empty());
    };
    let mut archive = cache.archive.lock();
    let mode = ARCHIVE_POLICY.with(Cell::get).map(|(mode, _)| mode);
    if archive.harvest {
        let Some(raw) = archive.output.as_ref() else {
            observe(ArchiveOutcome::Unavailable);
            return create(MTLPipelineOption::empty());
        };
        if let Err(error) = add(raw) {
            if mode != Some(Policy::Repair) {
                return Err(error);
            }
            log::warn!("populating Metal binary archive: {error}");
            archive.repair_failed = true;
            return create(MTLPipelineOption::empty());
        }
        let archives = NSArray::from_slice(&[&**raw]);
        archive.changed = true;
        attach(Some(&archives));
        let result = create(MTLPipelineOption::empty());
        attach(None);
        return result;
    }
    if archive.read.is_empty() {
        observe(ArchiveOutcome::Unavailable);
        return create(MTLPipelineOption::empty());
    }
    let archives = NSArray::from_retained_slice(&archive.read);
    attach(Some(&archives));
    let options = if mode.is_some() {
        MTLPipelineOption::FailOnBinaryArchiveMiss
    } else {
        MTLPipelineOption::empty()
    };
    let result = create(options);
    attach(None);
    drop(archives);
    let Some(mode) = mode else {
        return result;
    };
    let outcome = match &result {
        Ok(_) => ArchiveOutcome::Hit,
        Err(error) => {
            // Metal drivers do not expose a public archive-miss error code.
            let message = error.localizedDescription().to_string();
            if message.contains("Unable to find function") && message.contains("in binary archive")
            {
                ArchiveOutcome::Miss
            } else {
                ArchiveOutcome::Error
            }
        }
    };
    observe(outcome);
    if outcome != ArchiveOutcome::Miss || mode == Policy::Strict {
        return result;
    }
    if mode == Policy::Repair && archive.source.is_some() && !archive.repair_failed {
        let repair = (|| {
            if archive.output.is_none() {
                archive.output = Some(load_archive(device, &[])?);
            }
            if let Some(raw) = &archive.output {
                add(raw).map_err(io::Error::other)?;
            }
            Ok::<_, io::Error>(())
        })();
        match repair {
            Ok(()) => archive.changed = true,
            Err(error) => {
                log::warn!("repairing Metal binary archive: {error}");
                archive.repair_failed = true;
            }
        }
    }
    drop(archive);
    create(MTLPipelineOption::empty())
}

// The native API exchanges file URLs while wgpu exchanges bytes. Use a private
// directory and unlink it on every exit path.
struct ArchiveFile {
    directory: PathBuf,
    path: PathBuf,
    url: Retained<NSURL>,
}

impl ArchiveFile {
    fn new() -> io::Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory = loop {
            let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
            let directory =
                env::temp_dir().join(format!("wgpu-metal-{}-{sequence}", std::process::id()));
            match fs::DirBuilder::new().mode(0o700).create(&directory) {
                Ok(()) => break directory,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        };
        let path = directory.join("archive.metallib");
        let url = file_url(&path);
        Ok(Self {
            directory,
            path,
            url,
        })
    }
}

fn file_url(path: &Path) -> Retained<NSURL> {
    use std::ffi::CString;

    let path = CString::new(path.as_os_str().as_bytes()).expect("archive path must be a file path");
    let path: &CStr = &path;
    // SAFETY: The path is a live, nul-terminated filesystem representation.
    unsafe {
        NSURL::fileURLWithFileSystemRepresentation_isDirectory_relativeToURL(
            core::ptr::NonNull::new_unchecked(path.as_ptr().cast_mut()),
            false,
            None,
        )
    }
}

impl Drop for ArchiveFile {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}
