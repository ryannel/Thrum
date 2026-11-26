//! Optional macOS pipeline preparation using build-time Metal libraries.

use std::{
    fmt, fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, PoisonError},
};

use wgpu::hal::metal::{
    StageInputs, StageLibrary, StageResolver, StageTranslation, capture_stage_translations,
    encode_stage_records, with_stage_resolver,
};

/// The pinned WGSL translator identity carried in a stage key.
pub const TRANSLATOR: &str = "wgpu 30.0.1; Naga 30.0.1";
const KEY_FORMAT: &str = "thrum Metal stage 1";

/// Hash the inputs to the backend's actual stage translation.
pub fn stage_key(inputs: &StageInputs<'_>, translator: &str) -> Option<String> {
    let source = inputs.wgsl?;
    let options = format!("{:?}", inputs.options);
    let pipeline = inputs.pipeline;
    let mut lengths: Vec<_> = pipeline.binding_array_length_map.iter().collect();
    lengths.sort_unstable_by_key(|(binding, _)| **binding);
    let mut constants: Vec<_> = inputs
        .constants
        .iter()
        .map(|(name, value)| (name, value.to_bits()))
        .collect();
    constants.sort_unstable_by_key(|(name, _)| *name);
    let pipeline = format!(
        "{:?} {} {} {:?} {lengths:?} {constants:?}",
        pipeline.entry_point,
        pipeline.allow_and_force_point_size,
        pipeline.vertex_pulling_transform,
        pipeline.vertex_buffer_mappings,
    );
    let compiler = format!("{} {}", inputs.language_version, inputs.preserve_invariance);
    let mut key = blake3::Hasher::new();
    for part in [
        KEY_FORMAT, translator, source, &options, &pipeline, &compiler,
    ] {
        key.update(&(part.len() as u64).to_le_bytes());
        key.update(part.as_bytes());
    }
    Some(key.finalize().to_hex().to_string())
}

/// Read a build-authored blob and check its key and payload digest.
#[must_use]
pub fn library_payload<'a>(blob: &'a [u8], key: &str) -> Option<&'a [u8]> {
    let mut sections = blob.splitn(3, |&byte| byte == b'\n');
    let recorded_key = sections.next()?;
    let digest = sections.next()?;
    let payload = sections.next()?;
    (recorded_key == key.as_bytes() && digest == blake3::hash(payload).to_hex().as_bytes())
        .then_some(payload)
}

/// Select load or export of stage libraries during Mac pipeline preparation.
#[derive(Clone, Debug, Default)]
pub struct Preparation {
    harvest: Option<PathBuf>,
    store: Option<PathBuf>,
    pub(crate) parallel: bool,
}

impl Preparation {
    /// Record the manifest and export the backend's translations to `output`.
    #[must_use]
    pub fn harvest(output: PathBuf) -> Self {
        Self {
            harvest: Some(output),
            ..Self::default()
        }
    }

    /// Use a local store of offline shader stages built for this game.
    ///
    /// # Safety
    /// The directory must be trusted like the executable. Its blobs must be produced by
    /// the pinned backend and Thrum's cook, with matching WGSL, layouts and settings.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "the caller establishes provenance for native shader bytes"
    )]
    pub unsafe fn trusted_store(store: PathBuf, parallel: bool) -> Self {
        Self {
            store: Some(store),
            parallel,
            ..Self::default()
        }
    }

    /// Retain source identity in wgpu-core for the stage key while keeping
    /// wgpu's interface validation and implicit layouts in use.
    pub fn configure_instance(descriptor: &mut wgpu::InstanceDescriptor) {
        descriptor.flags |= wgpu::InstanceFlags::DEBUG;
    }

    #[expect(
        unsafe_code,
        reason = "only Preparation::trusted_store admits build-provided libraries"
    )]
    pub(crate) fn pipeline<T>(
        &self,
        kind: &str,
        label: Option<&str>,
        create: impl FnOnce() -> T,
    ) -> T {
        let attempt = Arc::new(Attempt {
            store: self.store.clone(),
            stages: Mutex::new(Vec::new()),
        });
        // SAFETY: This opt-in directory is a trusted local build input, like the game
        // executable. Key and digest checks reject stale and damaged build products.
        // With no matching blob the backend keeps its validated WGSL stage.
        let create = || unsafe { with_stage_resolver(Arc::<Attempt>::clone(&attempt), create) };
        let value = if let Some(output) = &self.harvest {
            let (value, translations) = capture_stage_translations(create);
            for stage in translations {
                if let Err(error) = export(output, &stage) {
                    report(format_args!("THRUM_METALLIB_EXPORT_ERROR {error}"));
                }
            }
            value
        } else {
            create()
        };
        let stages = attempt
            .stages
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let stages = stages
            .iter()
            .map(|(key, path)| format!("{key}:{path}"))
            .collect::<Vec<_>>();
        let thread = std::thread::current().id();
        report(format_args!(
            "THRUM_METALLIB_PIPELINE {kind} {thread:?} {} {}",
            stages.join(","),
            label.unwrap_or("")
        ));
        value
    }
}

fn report(args: fmt::Arguments<'_>) {
    writeln!(io::stderr().lock(), "{args}").ok();
}

struct Attempt {
    store: Option<PathBuf>,
    stages: Mutex<Vec<(String, &'static str)>>,
}

impl StageResolver for Attempt {
    fn resolve(&self, inputs: &StageInputs<'_>) -> StageLibrary {
        let key = stage_key(inputs, TRANSLATOR).unwrap_or_else(|| "missing-source".to_owned());
        let bytes = self
            .store
            .as_ref()
            .and_then(|store| fs::read(store.join(format!("{key}.blob"))).ok())
            .and_then(|blob| library_payload(&blob, &key).map(<[u8]>::to_vec));
        let path = if self.store.is_some() {
            "fallback"
        } else {
            "wgsl"
        };
        self.stages
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((key.clone(), path));
        StageLibrary { key, bytes }
    }

    fn loaded(&self, key: &str, success: bool) {
        if success {
            let mut stages = self.stages.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some((_, path)) = stages.last_mut().filter(|(recorded, _)| recorded == key) {
                *path = "cooked";
            }
        }
    }
}

fn export(output: &Path, stage: &StageTranslation) -> io::Result<()> {
    let key = stage
        .key
        .as_ref()
        .ok_or_else(|| io::Error::other("missing stage key"))?;
    if key == "missing-source" {
        return Err(io::Error::other(format!(
            "missing WGSL for {}",
            stage.entry_point
        )));
    }
    let path = output.join(key);
    fs::create_dir_all(&path)?;
    fs::write(path.join("source.metal"), &stage.source)?;
    fs::write(
        path.join("options.json"),
        format!(
            r#"{{"language_version":{},"preserve_invariance":{},"entry_point":"{}"}}"#,
            stage.language_version, stage.preserve_invariance, stage.entry_point,
        ),
    )?;
    fs::write(
        path.join("stage.record"),
        encode_stage_records(std::slice::from_ref(&stage.record), &[]),
    )
}
