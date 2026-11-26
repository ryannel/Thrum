//! The file identity and public wgpu cache operations used during Metal preparation.

use std::{
    fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

use objc2_foundation::NSProcessInfo;
use objc2_metal::MTLDevice as _;
use wgpu::hal::metal::{ArchiveOutcome, with_archive_policy, with_archive_repair};

use super::report;

const MAGIC: &[u8] = b"THRUM METAL ARCHIVE 1";

/// Local pipeline archive requested for a Metal device generation.
#[derive(Clone, Debug)]
pub enum ArchiveRequest {
    /// Open an archive identified by the build's pipeline manifest.
    /// Normal startup may repair the local file before marking the device ready.
    Read {
        /// Local archive file.
        path: PathBuf,
        /// Current pipeline manifest.
        manifest: PathBuf,
        /// Refuse startup on an archive miss.
        strict: bool,
    },
    /// Explicit offline harvest of a local archive.
    Harvest {
        /// New archive file to write after pipeline preparation.
        path: PathBuf,
        /// Pipeline manifest used to identify the archive.
        manifest: PathBuf,
        /// Label omitted by a control run. A single `*` omits all pipelines.
        omit: Option<String>,
    },
}

#[derive(Debug)]
pub(super) struct Archive {
    cache: wgpu::PipelineCache,
    mode: Mode,
}

#[derive(Debug)]
enum Mode {
    Read {
        strict: bool,
        recovery: Option<Recovery>,
    },
    Rebuild(Recovery),
    Harvest {
        path: PathBuf,
        identity: Identity,
        omit: Option<String>,
    },
}

#[derive(Debug)]
struct Recovery {
    path: PathBuf,
    identity: Identity,
    pending: AtomicBool,
    dirty: AtomicBool,
    failed: AtomicBool,
}

impl Recovery {
    fn new(path: PathBuf, identity: Identity, dirty: bool) -> Self {
        Self {
            path,
            identity,
            pending: AtomicBool::new(true),
            dirty: AtomicBool::new(dirty),
            failed: AtomicBool::new(false),
        }
    }

    fn active(&self) -> bool {
        self.pending.load(Ordering::Relaxed) && !self.failed.load(Ordering::Relaxed)
    }

    fn observe(&self, cooked: bool, outcome: ArchiveOutcome) {
        if !cooked || matches!(outcome, ArchiveOutcome::Error | ArchiveOutcome::Unavailable) {
            self.failed.store(true, Ordering::Relaxed);
        }
        if outcome == ArchiveOutcome::Miss {
            self.dirty.store(true, Ordering::Relaxed);
        }
    }

    fn finish(&self, cache: &wgpu::PipelineCache) {
        if !self.pending.swap(false, Ordering::Relaxed)
            || !self.dirty.load(Ordering::Relaxed)
            || self.failed.load(Ordering::Relaxed)
        {
            return;
        }
        let result = cache
            .get_data()
            .ok_or_else(|| io::Error::other("Metal archive serialization failed"))
            .and_then(|data| {
                self.identity.replace(&self.path, &data)?;
                Ok(data.len())
            });
        match result {
            Ok(bytes) => report(format_args!("THRUM_METAL_ARCHIVE_REPAIRED {bytes}")),
            Err(error) => report(format_args!("THRUM_METAL_ARCHIVE_REPAIR_ERROR {error}")),
        }
    }
}

#[derive(Debug)]
struct Identity {
    device: String,
    os: String,
    manifest: String,
}

impl Identity {
    #[expect(
        unsafe_code,
        reason = "read the live adapter's immutable Metal registry identity"
    )]
    fn new(device: &wgpu::Device, manifest: &Path) -> io::Result<Self> {
        // SAFETY: The HAL guard retains the device for the identity query.
        let raw = unsafe { device.as_hal::<wgpu::hal::api::Metal>() }
            .ok_or_else(|| io::Error::other("expected a Metal device"))?;
        Ok(Self {
            device: format!("{:016x}", raw.raw_device().registryID()),
            os: NSProcessInfo::processInfo()
                .operatingSystemVersionString()
                .to_string(),
            manifest: blake3::hash(&fs::read(manifest)?).to_hex().to_string(),
        })
    }

    fn load<'a>(&self, file: &'a [u8]) -> Result<&'a [u8], &'static str> {
        let mut parts = file.splitn(6, |&byte| byte == b'\n');
        let [
            Some(magic),
            Some(device),
            Some(os),
            Some(manifest),
            Some(digest),
            Some(data),
        ] = std::array::from_fn(|_| parts.next())
        else {
            return Err("corrupt");
        };
        if magic != MAGIC || digest != blake3::hash(data).to_hex().as_bytes() {
            return Err("corrupt");
        }
        if device != self.device.as_bytes()
            || os != self.os.as_bytes()
            || manifest != self.manifest.as_bytes()
        {
            return Err("stale");
        }
        Ok(data)
    }

    fn write(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?;
        let result = self.write_contents(&mut file, data);
        if result.is_err() {
            drop(file);
            drop(fs::remove_file(path));
        }
        result
    }

    fn replace(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let (temporary, mut file) = loop {
            let serial = NEXT.fetch_add(1, Ordering::Relaxed);
            let temporary =
                path.with_file_name(format!(".metal-archive-{}-{serial}", std::process::id()));
            match fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
            {
                Ok(file) => break (temporary, file),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        };
        let result = self.write_contents(&mut file, data);
        drop(file);
        let result = result.and_then(|()| fs::rename(&temporary, path));
        if result.is_err() {
            drop(fs::remove_file(temporary));
        }
        result
    }

    fn write_contents(&self, file: &mut fs::File, data: &[u8]) -> io::Result<()> {
        file.write_all(MAGIC)?;
        writeln!(
            file,
            "\n{}\n{}\n{}\n{}",
            self.device,
            self.os,
            self.manifest,
            blake3::hash(data)
        )?;
        file.write_all(data)
    }
}

impl Archive {
    pub(super) fn open(
        device: &wgpu::Device,
        request: ArchiveRequest,
        admitted: bool,
        repair: bool,
    ) -> Option<Self> {
        match request {
            ArchiveRequest::Read {
                path,
                manifest,
                strict,
            } => Self::read(device, path, &manifest, strict, admitted, repair),
            ArchiveRequest::Harvest {
                path,
                manifest,
                omit,
            } => {
                assert!(
                    admitted,
                    "Metal pipeline archive harvest requires PIPELINE_CACHE"
                );
                let identity = Identity::new(device, &manifest)
                    .expect("read harvest manifest and device identity");
                let cache = create_cache(device, None);
                Some(Self {
                    cache,
                    mode: Mode::Harvest {
                        path,
                        identity,
                        omit,
                    },
                })
            }
        }
    }

    fn read(
        device: &wgpu::Device,
        path: PathBuf,
        manifest: &Path,
        strict: bool,
        admitted: bool,
        repair: bool,
    ) -> Option<Self> {
        let identity = if admitted {
            Identity::new(device, manifest).map_err(|_| "missing-manifest")
        } else {
            Err("unsupported")
        };
        let loaded = identity
            .as_ref()
            .map_err(|cause| *cause)
            .and_then(|identity| Self::load(device, &path, identity));
        match loaded {
            Ok(cache) => Some(Self {
                cache,
                mode: Mode::Read {
                    strict,
                    recovery: identity
                        .ok()
                        .filter(|_| repair && !strict)
                        .map(|identity| Recovery::new(path, identity, false)),
                },
            }),
            Err(cause) => {
                report(format_args!("THRUM_METAL_ARCHIVE_REJECT {cause}"));
                assert!(!strict, "strict Metal archive refused: {cause}");
                if !repair || !matches!(cause, "absent" | "corrupt" | "stale" | "unreadable") {
                    return None;
                }
                let (cache, outcome) = with_archive_repair(|| create_cache(device, None));
                if outcome == ArchiveOutcome::Unavailable {
                    report(format_args!("THRUM_METAL_ARCHIVE_REPAIR_ERROR unavailable"));
                    return None;
                }
                Some(Self {
                    cache,
                    mode: Mode::Rebuild(Recovery::new(path, identity.ok()?, true)),
                })
            }
        }
    }

    fn load(
        device: &wgpu::Device,
        path: &Path,
        identity: &Identity,
    ) -> Result<wgpu::PipelineCache, &'static str> {
        let file = fs::read(path).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                "absent"
            } else {
                "unreadable"
            }
        })?;
        let data = identity.load(&file)?;
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
        report(format_args!("THRUM_METAL_ARCHIVE_IMPORT {}", data.len()));
        let (cache, outcome) = with_archive_policy(false, || create_cache(device, Some(data)));
        let internal = pollster::block_on(internal.pop());
        let validation = pollster::block_on(validation.pop());
        if internal.is_some() {
            return Err("device-error");
        }
        if let Some(error) = validation {
            let message = error.to_string();
            report(format_args!("THRUM_METAL_ARCHIVE_HEADER {message}"));
            if message.contains("different device") || message.contains("out of date") {
                return Err("stale");
            }
            return Err("corrupt");
        }
        if outcome == ArchiveOutcome::Unavailable {
            return Err("unreadable");
        }
        Ok(cache)
    }

    pub(super) fn strict(&self) -> bool {
        matches!(self.mode, Mode::Read { strict: true, .. })
    }

    pub(super) fn save(&self) -> io::Result<()> {
        if let Mode::Harvest { path, identity, .. } = &self.mode {
            let data = self
                .cache
                .get_data()
                .ok_or_else(|| io::Error::other("Metal archive serialization failed"))?;
            identity.write(path, &data)?;
            report(format_args!("THRUM_METAL_ARCHIVE_SAVED {}", data.len()));
        }
        Ok(())
    }

    pub(super) fn finish_recovery(&self) {
        if let Mode::Read {
            recovery: Some(recovery),
            ..
        }
        | Mode::Rebuild(recovery) = &self.mode
        {
            recovery.finish(&self.cache);
        }
    }

    pub(super) fn create<T>(
        &self,
        device: &wgpu::Device,
        label: Option<&str>,
        cooked: bool,
        create: impl FnOnce(Option<&wgpu::PipelineCache>) -> T,
    ) -> T {
        let name = label.unwrap_or("");
        match &self.mode {
            Mode::Harvest { omit, .. } => {
                let include = omit
                    .as_deref()
                    .is_none_or(|omit| omit != "*" && omit != name);
                report(format_args!(
                    "THRUM_METAL_ARCHIVE_HARVEST {} {name}",
                    if include { "add" } else { "omit" }
                ));
                create(include.then_some(&self.cache))
            }
            Mode::Rebuild(recovery) => {
                if !recovery.active() || !cooked {
                    recovery.observe(cooked, ArchiveOutcome::Unused);
                    return create(None);
                }
                let (pipeline, outcome) = with_archive_repair(|| create(Some(&self.cache)));
                recovery.observe(cooked, outcome);
                pipeline
            }
            Mode::Read { strict, recovery } => {
                let error = strict.then(|| device.push_error_scope(wgpu::ErrorFilter::Internal));
                let create = || create(Some(&self.cache));
                let (pipeline, outcome) =
                    if cooked && recovery.as_ref().is_some_and(Recovery::active) {
                        with_archive_repair(create)
                    } else {
                        with_archive_policy(*strict, create)
                    };
                if let Some(recovery) = recovery {
                    recovery.observe(cooked, outcome);
                }
                report(format_args!(
                    "THRUM_METAL_ARCHIVE_LOOKUP {outcome:?} {name}"
                ));
                if let Some(error) = error {
                    let detail = pollster::block_on(error.pop());
                    assert!(
                        outcome == ArchiveOutcome::Hit && detail.is_none(),
                        "strict Metal archive failed for {name}: {outcome:?} {detail:?}"
                    );
                }
                pipeline
            }
        }
    }
}

#[expect(
    unsafe_code,
    reason = "only use bytes authenticated against the harvest's digest"
)]
fn create_cache(device: &wgpu::Device, data: Option<&[u8]>) -> wgpu::PipelineCache {
    // SAFETY: The harvest uses no input. The only file-data caller validates
    // the current identity and digest of the cache data produced by get_data.
    unsafe {
        device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
            label: Some("local Metal pipeline archive"),
            data,
            fallback: false,
        })
    }
}
