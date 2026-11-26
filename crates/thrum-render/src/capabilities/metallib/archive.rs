//! The file identity and public wgpu cache operations used during Metal preparation.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use objc2_foundation::NSProcessInfo;
use objc2_metal::MTLDevice as _;
use wgpu::hal::metal::{ArchiveOutcome, with_archive_policy};

use super::report;

const MAGIC: &[u8] = b"THRUM METAL ARCHIVE 1";

/// Local pipeline archive requested for a Metal device generation.
#[derive(Clone, Debug)]
pub enum ArchiveRequest {
    /// Open an archive identified by the build's pipeline manifest.
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
    },
    Harvest {
        path: PathBuf,
        identity: Identity,
        omit: Option<String>,
    },
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
        use io::Write as _;

        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?;
        let result = (|| {
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
        })();
        if result.is_err() {
            drop(file);
            drop(fs::remove_file(path));
        }
        result
    }
}

impl Archive {
    pub(super) fn open(
        device: &wgpu::Device,
        request: ArchiveRequest,
        admitted: bool,
    ) -> Option<Self> {
        match request {
            ArchiveRequest::Read {
                path,
                manifest,
                strict,
            } => {
                let loaded = if admitted {
                    Self::load(device, &path, &manifest)
                } else {
                    Err("unsupported")
                };
                match loaded {
                    Ok(cache) => Some(Self {
                        cache,
                        mode: Mode::Read { strict },
                    }),
                    Err(cause) => {
                        report(format_args!("THRUM_METAL_ARCHIVE_REJECT {cause}"));
                        assert!(!strict, "strict Metal archive refused: {cause}");
                        None
                    }
                }
            }
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

    fn load(
        device: &wgpu::Device,
        path: &Path,
        manifest: &Path,
    ) -> Result<wgpu::PipelineCache, &'static str> {
        let identity = Identity::new(device, manifest).map_err(|_| "missing-manifest")?;
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
        matches!(self.mode, Mode::Read { strict: true })
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

    pub(super) fn create<T>(
        &self,
        device: &wgpu::Device,
        label: Option<&str>,
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
            Mode::Read { strict } => {
                let error = strict.then(|| device.push_error_scope(wgpu::ErrorFilter::Internal));
                let (pipeline, outcome) =
                    with_archive_policy(*strict, || create(Some(&self.cache)));
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
