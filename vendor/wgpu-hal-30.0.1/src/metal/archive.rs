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
    raw: Option<Retained<NativeArchive>>,
    mode: Mode,
}

#[derive(Debug)]
enum Mode {
    Harvest(Mutex<bool>),
    ReadOnly,
}

// Harvest serializes access to the mutable archive. A read-only archive is never
// updated and can be searched by concurrent pipeline creations on the same device.
unsafe impl Send for PipelineCache {}
unsafe impl Sync for PipelineCache {}

impl PipelineCache {
    pub(super) fn new(
        device: &ProtocolObject<dyn MTLDevice>,
        data: Option<&[u8]>,
    ) -> Result<Self, crate::PipelineCacheError> {
        let load = || {
            let descriptor = MTLBinaryArchiveDescriptor::new();
            let file = if let Some(bytes) = data.filter(|bytes| !bytes.is_empty()) {
                let file = ArchiveFile::new()?;
                fs::write(&file.path, bytes)?;
                descriptor.setUrl(Some(&file.url));
                Some(file)
            } else {
                None
            };
            let result = device.newBinaryArchiveWithDescriptor_error(&descriptor);
            drop(file);
            result.map_err(io::Error::other)
        };
        if !supported(device) {
            return Err(crate::DeviceError::Unexpected.into());
        }
        let raw = match load() {
            Ok(raw) => Some(raw),
            Err(error) => {
                log::warn!("opening Metal binary archive: {error}");
                // wgpu-core treats all HAL cache creation errors as device loss.
                // Report an unavailable cache separately when the caller handles it.
                if data.is_some() && observe(ArchiveOutcome::Unavailable).is_some() {
                    None
                } else {
                    return Err(crate::DeviceError::Unexpected.into());
                }
            }
        };
        let mode = if data.is_some() {
            Mode::ReadOnly
        } else {
            Mode::Harvest(Mutex::new(false))
        };
        Ok(Self { raw, mode })
    }

    pub(super) fn data(&self) -> Option<Vec<u8>> {
        let Mode::Harvest(populated) = &self.mode else {
            return None;
        };
        let populated = populated.lock();
        if !*populated {
            // Metal cannot serialize an empty archive. An empty payload opens a
            // fresh, read-only native archive on load.
            return Some(Vec::new());
        }
        let raw = self.raw.as_ref()?;
        let serialize = || {
            let file = ArchiveFile::new()?;
            raw.serializeToURL_error(&file.url)
                .map_err(io::Error::other)?;
            fs::read(&file.path)
        };
        serialize()
            .map_err(|error| log::error!("serializing Metal binary archive: {error}"))
            .ok()
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

thread_local! {
    static ARCHIVE_POLICY: Cell<Option<(bool, ArchiveOutcome)>> = const { Cell::new(None) };
}

fn observe(outcome: ArchiveOutcome) -> Option<bool> {
    ARCHIVE_POLICY.with(|policy| {
        let (strict, _) = policy.get()?;
        policy.set(Some((strict, outcome)));
        Some(strict)
    })
}

/// Ask Metal to report hits and misses during a synchronous pipeline creation
/// or a failure to open an archive during a synchronous cache import.
/// In strict mode a miss propagates as a pipeline error. In normal mode it
/// retries the pipeline without the archive.
pub fn with_archive_policy<T>(strict: bool, create: impl FnOnce() -> T) -> (T, ArchiveOutcome) {
    struct Restore<'a>(
        &'a Cell<Option<(bool, ArchiveOutcome)>>,
        Option<(bool, ArchiveOutcome)>,
    );
    impl Drop for Restore<'_> {
        fn drop(&mut self) {
            self.0.set(self.1);
        }
    }
    ARCHIVE_POLICY.with(|policy| {
        let _restore = Restore(
            policy,
            policy.replace(Some((strict, ArchiveOutcome::Unused))),
        );
        let result = create();
        let outcome = policy
            .get()
            .map_or(ArchiveOutcome::Unused, |(_, outcome)| outcome);
        (result, outcome)
    })
}

pub(super) fn create_pipeline<T>(
    cache: Option<&PipelineCache>,
    attach: impl Fn(Option<&NSArray<NativeArchive>>),
    add: impl FnOnce(&NativeArchive) -> Result<(), Retained<NSError>>,
    create: impl Fn(MTLPipelineOption) -> Result<T, Retained<NSError>>,
) -> Result<T, Retained<NSError>> {
    let Some(cache) = cache else {
        return create(MTLPipelineOption::empty());
    };
    let Some(raw) = &cache.raw else {
        observe(ArchiveOutcome::Unavailable);
        return create(MTLPipelineOption::empty());
    };
    let archives = NSArray::from_slice(&[&**raw]);
    match &cache.mode {
        Mode::Harvest(populated) => {
            let mut populated = populated.lock();
            add(raw)?;
            *populated = true;
            attach(Some(&archives));
            create(MTLPipelineOption::empty())
        }
        Mode::ReadOnly => {
            attach(Some(&archives));
            let Some((strict, _)) = ARCHIVE_POLICY.with(Cell::get) else {
                return create(MTLPipelineOption::empty());
            };
            let result = create(MTLPipelineOption::FailOnBinaryArchiveMiss);
            let outcome = match &result {
                Ok(_) => ArchiveOutcome::Hit,
                Err(error) => {
                    // Metal drivers do not expose a public archive-miss error code.
                    let message = error.localizedDescription().to_string();
                    if message.contains("Unable to find function")
                        && message.contains("in binary archive")
                    {
                        ArchiveOutcome::Miss
                    } else {
                        ArchiveOutcome::Error
                    }
                }
            };
            observe(outcome);
            if outcome == ArchiveOutcome::Miss && !strict {
                attach(None);
                return create(MTLPipelineOption::empty());
            }
            result
        }
    }
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
