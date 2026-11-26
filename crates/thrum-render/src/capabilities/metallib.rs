//! Optional macOS pipeline preparation using build-time Metal libraries.

use std::{
    borrow::Cow,
    collections::HashMap,
    fmt, fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, PoisonError},
};

use wgpu::{
    hal::metal::{
        StageTranslation, capture_stage_translations, encode_stage_records, split_stage_records,
    },
    naga::ShaderStage,
};

const TRANSLATOR: &str = "wgpu 30.0.1; Naga 30.0.1";
const KEY_FORMAT: &str = "thrum Metal stage 2";

fn digest(parts: &[&[u8]]) -> blake3::Hash {
    let mut key = blake3::Hasher::new();
    for part in parts {
        key.update(&(part.len() as u64).to_le_bytes());
        key.update(part);
    }
    key.finalize()
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
    parallel: bool,
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

    pub(super) fn attach(
        self,
        device: &wgpu::Device,
        admitted: bool,
    ) -> Option<PipelinePreparation> {
        let language = language_version(device)?;
        // A store without admission still records each stage's fallback.
        Some(PipelinePreparation {
            config: self,
            admitted,
            language,
            preserve_invariance: objc2::available!(macos = 11.0),
            device_key: digest(&[
                format!("{:?}", device.adapter_info()).as_bytes(),
                format!(
                    "{:?}",
                    device.features() - wgpu::Features::PASSTHROUGH_SHADERS
                )
                .as_bytes(),
                format!("{:?}", device.limits()).as_bytes(),
            ]),
            inputs: Mutex::default(),
        })
    }
}

#[expect(
    unsafe_code,
    reason = "query the selected Metal device's actual translation language"
)]
fn language_version(device: &wgpu::Device) -> Option<usize> {
    // SAFETY: The guard holds the device alive while reading immutable capabilities.
    Some(unsafe { device.as_hal::<wgpu::hal::api::Metal>() }?.shader_language_version())
}

#[derive(Debug, Default)]
struct Inputs {
    shaders: HashMap<wgpu::ShaderModule, blake3::Hash>,
    bindings: HashMap<wgpu::BindGroupLayout, blake3::Hash>,
    layouts: HashMap<wgpu::PipelineLayout, blake3::Hash>,
}

#[derive(Debug)]
pub(crate) struct PipelinePreparation {
    config: Preparation,
    admitted: bool,
    language: usize,
    preserve_invariance: bool,
    device_key: blake3::Hash,
    inputs: Mutex<Inputs>,
}

impl PipelinePreparation {
    pub(crate) fn parallel(&self) -> bool {
        self.config.parallel
    }

    fn inputs(&self) -> MutexGuard<'_, Inputs> {
        self.inputs.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn shader(&self, shader: wgpu::ShaderModule, source: blake3::Hash) {
        self.inputs().shaders.insert(shader, source);
    }

    pub(crate) fn bindings(
        &self,
        layout: wgpu::BindGroupLayout,
        entries: &[wgpu::BindGroupLayoutEntry],
    ) {
        let mut entries = entries.to_vec();
        entries.sort_unstable_by_key(|entry| entry.binding);
        self.inputs()
            .bindings
            .insert(layout, blake3::hash(format!("{entries:?}").as_bytes()));
    }

    pub(crate) fn layout(
        &self,
        layout: wgpu::PipelineLayout,
        desc: &wgpu::PipelineLayoutDescriptor<'_>,
    ) {
        let mut inputs = self.inputs();
        let mut key = blake3::Hasher::new();
        key.update(&desc.immediate_size.to_le_bytes());
        for group in desc.bind_group_layouts {
            if let Some(group) = group {
                let Some(bindings) = inputs.bindings.get(group) else {
                    return;
                };
                key.update(&[1]);
                key.update(bindings.as_bytes());
            } else {
                key.update(&[0]);
            }
        }
        inputs.layouts.insert(layout, key.finalize());
    }

    fn keys<const N: usize>(
        &self,
        layout: Option<&wgpu::PipelineLayout>,
        stages: &[Option<Stage<'_>>; N],
    ) -> [Option<Key>; N] {
        let inputs = self.inputs();
        let layout_key = layout.and_then(|layout| inputs.layouts.get(layout));
        stages.each_ref().map(|stage| {
            let stage = stage.as_ref()?;
            let shader = inputs.shaders.get(stage.module)?;
            let constants: Vec<_> = stage
                .compilation
                .constants
                .iter()
                .map(|(name, value)| (name, value.to_bits()))
                .collect();
            let options = format!(
                "{constants:?} {} {:?} {:?} {:?}",
                stage.compilation.zero_initialize_workgroup_memory,
                stage.topology,
                stage.vertex_buffers,
                wgpu::ShaderRuntimeChecks::checked(),
            );
            let pipeline = digest(&[
                shader.as_bytes(),
                layout_key?.as_bytes(),
                self.device_key.as_bytes(),
                format!("{:?}", stage.kind).as_bytes(),
                stage.entry?.as_bytes(),
                options.as_bytes(),
                &[u8::from(self.preserve_invariance)],
            ]);
            Some(Key {
                library: stage_key(&pipeline, TRANSLATOR, self.language),
                pipeline,
            })
        })
    }

    pub(crate) fn render(
        &self,
        device: &wgpu::Device,
        desc: &wgpu::RenderPipelineDescriptor<'_>,
        create: impl Fn(&wgpu::RenderPipelineDescriptor<'_>) -> wgpu::RenderPipeline,
    ) -> wgpu::RenderPipeline {
        let vertex = Stage {
            kind: ShaderStage::Vertex,
            module: desc.vertex.module,
            entry: desc.vertex.entry_point,
            compilation: &desc.vertex.compilation_options,
            topology: Some(desc.primitive.topology),
            vertex_buffers: desc.vertex.buffers,
        };
        let fragment = desc.fragment.as_ref().map(|fragment| Stage {
            kind: ShaderStage::Fragment,
            module: fragment.module,
            entry: fragment.entry_point,
            compilation: &fragment.compilation_options,
            vertex_buffers: &[],
            ..vertex
        });
        self.pipeline(
            device,
            desc.layout,
            "render",
            desc.label,
            &[Some(vertex), fragment],
            |[vertex, fragment]| {
                let mut desc = desc.clone();
                if let Some(stage) = vertex {
                    desc.vertex.module = &stage.module;
                    desc.vertex.entry_point = Some(&stage.function);
                }
                if let (Some(state), Some(stage)) = (&mut desc.fragment, fragment) {
                    state.module = &stage.module;
                    state.entry_point = Some(&stage.function);
                }
                create(&desc)
            },
        )
    }

    pub(crate) fn compute(
        &self,
        device: &wgpu::Device,
        desc: &wgpu::ComputePipelineDescriptor<'_>,
        create: impl Fn(&wgpu::ComputePipelineDescriptor<'_>) -> wgpu::ComputePipeline,
    ) -> wgpu::ComputePipeline {
        let stage = Stage {
            kind: ShaderStage::Compute,
            module: desc.module,
            entry: desc.entry_point,
            compilation: &desc.compilation_options,
            topology: None,
            vertex_buffers: &[],
        };
        self.pipeline(
            device,
            desc.layout,
            "compute",
            desc.label,
            &[Some(stage)],
            |[stage]| {
                let mut desc = desc.clone();
                if let Some(stage) = stage {
                    desc.module = &stage.module;
                    desc.entry_point = Some(&stage.function);
                }
                create(&desc)
            },
        )
    }

    pub(crate) fn mesh(
        &self,
        device: &wgpu::Device,
        desc: &wgpu::MeshPipelineDescriptor<'_>,
        create: impl Fn(&wgpu::MeshPipelineDescriptor<'_>) -> wgpu::RenderPipeline,
    ) -> wgpu::RenderPipeline {
        let mesh = Stage {
            kind: ShaderStage::Mesh,
            module: desc.mesh.module,
            entry: desc.mesh.entry_point,
            compilation: &desc.mesh.compilation_options,
            topology: Some(desc.primitive.topology),
            vertex_buffers: &[],
        };
        let task = desc.task.as_ref().map(|task| Stage {
            kind: ShaderStage::Task,
            module: task.module,
            entry: task.entry_point,
            compilation: &task.compilation_options,
            ..mesh
        });
        let fragment = desc.fragment.as_ref().map(|fragment| Stage {
            kind: ShaderStage::Fragment,
            module: fragment.module,
            entry: fragment.entry_point,
            compilation: &fragment.compilation_options,
            ..mesh
        });
        self.pipeline(
            device,
            desc.layout,
            "mesh",
            desc.label,
            &[task, Some(mesh), fragment],
            |[task, mesh, fragment]| {
                let mut desc = desc.clone();
                if let (Some(state), Some(stage)) = (&mut desc.task, task) {
                    state.module = &stage.module;
                    state.entry_point = Some(&stage.function);
                }
                if let Some(stage) = mesh {
                    desc.mesh.module = &stage.module;
                    desc.mesh.entry_point = Some(&stage.function);
                }
                if let (Some(state), Some(stage)) = (&mut desc.fragment, fragment) {
                    state.module = &stage.module;
                    state.entry_point = Some(&stage.function);
                }
                create(&desc)
            },
        )
    }

    fn pipeline<T, const N: usize>(
        &self,
        device: &wgpu::Device,
        layout: Option<&wgpu::PipelineLayout>,
        kind: &str,
        label: Option<&str>,
        stages: &[Option<Stage<'_>>; N],
        create: impl Fn(&[Option<CookedStage>; N]) -> T,
    ) -> T {
        let keys = self.keys(layout, stages);
        let mut cooked = std::array::from_fn(|index| {
            if !self.admitted {
                return None;
            }
            load(
                device,
                self.config.store.as_deref()?,
                &keys[index].as_ref()?.library,
                label,
            )
        });
        let pipeline = if let Some(output) = &self.config.harvest {
            let (pipeline, translations) = capture_stage_translations(|| create(&cooked));
            for translation in translations {
                let key = stages.iter().zip(&keys).find_map(|(stage, key)| {
                    let stage = stage.as_ref()?;
                    (stage.kind == translation.stage
                        && stage.entry == Some(&translation.entry_point))
                    .then_some(key.as_ref())?
                });
                let result = key
                    .ok_or_else(|| {
                        io::Error::other(format!(
                            "missing stage key for {}",
                            translation.entry_point
                        ))
                    })
                    .and_then(|key| {
                        if translation.language_version != self.language
                            || translation.preserve_invariance != self.preserve_invariance
                        {
                            return Err(io::Error::other(
                                "Metal compiler settings changed during harvest",
                            ));
                        }
                        export(output, key, &translation)
                    });
                if let Err(error) = result {
                    report(format_args!(
                        "THRUM_METALLIB_EXPORT_ERROR {}: {error}",
                        label.unwrap_or("")
                    ));
                }
            }
            pipeline
        } else if cooked.iter().any(Option::is_some) {
            let (pipeline, valid) = check_creation(device, || create(&cooked));
            if valid {
                pipeline
            } else {
                cooked.fill_with(|| None);
                create(&cooked)
            }
        } else {
            create(&cooked)
        };
        let thread = std::thread::current().id();
        let names = stages
            .iter()
            .zip(&keys)
            .zip(&cooked)
            .filter_map(|((stage, key), cooked)| {
                stage.as_ref()?;
                let key = key
                    .as_ref()
                    .map_or("missing-description", |key| &key.library);
                let path = if cooked.is_some() {
                    "passthrough"
                } else if self.config.store.is_some() {
                    "fallback"
                } else {
                    "wgsl"
                };
                Some(format!("{key}:{path}"))
            })
            .collect::<Vec<_>>();
        report(format_args!(
            "THRUM_METALLIB_PIPELINE {kind} {thread:?} {} {}",
            names.join(","),
            label.unwrap_or("")
        ));
        pipeline
    }
}

struct Stage<'a> {
    kind: ShaderStage,
    module: &'a wgpu::ShaderModule,
    entry: Option<&'a str>,
    compilation: &'a wgpu::PipelineCompilationOptions<'a>,
    topology: Option<wgpu::PrimitiveTopology>,
    vertex_buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
}

struct Key {
    library: String,
    pipeline: blake3::Hash,
}

/// The library key for a harvested pipeline description and toolchain identity.
#[must_use]
pub fn stage_key(pipeline: &blake3::Hash, translator: &str, language: usize) -> String {
    digest(&[
        KEY_FORMAT.as_bytes(),
        pipeline.as_bytes(),
        translator.as_bytes(),
        &language.to_le_bytes(),
    ])
    .to_hex()
    .to_string()
}

struct CookedStage {
    function: String,
    module: wgpu::ShaderModule,
}

#[expect(
    unsafe_code,
    reason = "only the trusted local cook can supply native modules"
)]
fn load(
    device: &wgpu::Device,
    store: &Path,
    key: &str,
    label: Option<&str>,
) -> Option<CookedStage> {
    let blob = fs::read(store.join(format!("{key}.blob"))).ok()?;
    let payload = library_payload(&blob, key)?;
    let (records, _) = split_stage_records(payload).ok()?;
    let [record] = records.as_deref()? else {
        return None;
    };
    let [x, y, z] = record.workgroup_size;
    let (module, valid) = check_creation(device, || {
        // SAFETY: Registry admission enables passthrough. The trusted local cook binds
        // the native bytecode to this shader, pipeline layout and translation settings.
        unsafe {
            crate::shaders::compile_metal_library(
                device,
                wgpu::ShaderModuleDescriptorPassthrough {
                    label,
                    entry_points: Cow::Owned(vec![wgpu::PassthroughShaderEntryPoint {
                        name: Cow::Borrowed(&record.function),
                        workgroup_size: (x, y, z),
                    }]),
                    metallib: Some(Cow::Borrowed(payload)),
                    ..Default::default()
                },
            )
        }
    });
    valid.then(|| {
        report(format_args!(
            "THRUM_METALLIB_PASSTHROUGH {:?} {key} {}",
            std::thread::current().id(),
            label.unwrap_or("")
        ));
        CookedStage {
            module,
            function: record.function.clone(),
        }
    })
}

fn check_creation<T>(device: &wgpu::Device, create: impl FnOnce() -> T) -> (T, bool) {
    let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
    let value = create();
    let internal = pollster::block_on(internal.pop());
    let validation = pollster::block_on(validation.pop());
    (value, internal.is_none() && validation.is_none())
}

fn report(args: fmt::Arguments<'_>) {
    writeln!(io::stderr().lock(), "{args}").ok();
}

fn export(output: &Path, key: &Key, stage: &StageTranslation) -> io::Result<()> {
    let path = output.join(&key.library);
    fs::create_dir_all(&path)?;
    fs::write(path.join("source.metal"), &stage.source)?;
    fs::write(
        path.join("options.json"),
        format!(
            r#"{{"language_version":{},"preserve_invariance":{},"entry_point":"{}","translator":"{TRANSLATOR}","pipeline_key":"{}"}}"#,
            stage.language_version,
            stage.preserve_invariance,
            stage.entry_point,
            key.pipeline.to_hex(),
        ),
    )?;
    fs::write(
        path.join("stage.record"),
        encode_stage_records(std::slice::from_ref(&stage.record), &[]),
    )
}
