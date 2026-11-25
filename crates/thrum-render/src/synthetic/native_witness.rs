//! Labelled composition diagnostic, disabled for ordinary scene rendering.
use super::{RenderError, SCENE_FORMAT};
use crate::{
    apple_tile::{
        NativeBinding, NativeFrameValues, NativeGraph, NativePolicy, NativePreparation,
        NativeProvider, NativeProviderReport, NativeState, PreparedProvider, TwinReason,
    },
    capabilities::DeviceCapabilities,
    graph::{FrameGraph, GraphTextures, ResourceId},
    runtime::DeviceSite,
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};

pub(crate) const PASS: &str = "native stateful copy/select witness";
pub(crate) const RESET_PASS: &str = "native witness reset";
mod graph;

/// Explicit diagnostic selection. Ordinary scenes leave this disabled.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NativeWitness {
    /// No witness resources or commands.
    #[default]
    Off,
    /// Whole raw Metal encoder with a prepared WGSL twin.
    Metal,
    /// Prepared public-wgpu copy/select oracle.
    Wgsl,
    /// Forced optional-provider rejection; uses the already prepared WGSL twin.
    Rejected,
    /// Omission control: the native encoder records nothing.
    NoCommands,
    /// Omission control: native blits remain but the storage kernel is omitted.
    NoTile,
    /// Omission control: provider reset is omitted.
    NoReset,
    /// Omission control: read a preinitialized stale scene.
    StaleSource,
    /// Omission control: never advance the submitted history side.
    StaleHistory,
}

/// Provider outcomes, independent of world ticks and display presentation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NativeWitnessReport {
    /// Actual selected diagnostic.
    pub selection: NativeWitness,
    /// Encoded candidates.
    pub encoded: u64,
    /// Successfully submitted candidates.
    pub submitted: u64,
    /// Encoded candidates discarded before submission.
    pub discarded: u64,
    /// Encoded reset operations, including the first sample.
    pub resets: u64,
    /// Completed submitted candidates.
    pub completed: u64,
    /// Unique full-size output/history and optional stale-control bytes.
    pub resident_bytes: u64,
}

impl NativeWitnessReport {
    pub(super) fn from_provider(
        report: NativeProviderReport,
        requested: NativeWitness,
        resident_bytes: u64,
    ) -> Self {
        let selection = if report.twin.is_some() {
            NativeWitness::Wgsl
        } else {
            requested
        };
        Self {
            selection,
            encoded: report.encoded,
            submitted: report.submitted,
            discarded: report.discarded,
            resets: if requested == NativeWitness::NoReset {
                0
            } else {
                report.resets
            },
            completed: report.completed,
            resident_bytes,
        }
    }
}

pub(super) struct Factory {
    device: DeviceSite,
    queue: wgpu::Queue,
    selection: NativeWitness,
    pipeline: wgpu::ComputePipeline,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    metal: Option<crate::apple_tile::witness::Pipeline>,
    preparation: NativePreparation,
    extents: Mutex<BTreeMap<[u32; 2], Weak<Provider>>>,
}

struct Provider {
    factory: Arc<Pipelines>,
    output: wgpu::Texture,
    view: wgpu::TextureView,
    histories: [wgpu::Texture; 2],
    zero: wgpu::Texture,
    stale: Option<wgpu::Texture>,
    groups: [wgpu::BindGroup; 2],
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    raw: Option<[crate::apple_tile::Texture; 3]>,
    extent: [u32; 2],
    state: Arc<NativeState>,
    side: AtomicUsize,
    phase: AtomicU64,
}

struct Pipelines {
    selection: NativeWitness,
    pipeline: wgpu::ComputePipeline,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    metal: Option<crate::apple_tile::witness::Pipeline>,
}

struct Binding {
    provider: Arc<Provider>,
    source: wgpu::Texture,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    raw_source: Option<crate::apple_tile::Texture>,
}

impl Factory {
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            clippy::arc_with_non_send_sync,
            reason = "Shared native/WebGPU ownership model; WebGPU handles stay on their browser thread"
        )
    )]
    pub(super) fn new(
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        selection: NativeWitness,
    ) -> Arc<Self> {
        let device = capabilities.site();
        let policy = match selection {
            NativeWitness::Off => NativePolicy::Recipe,
            NativeWitness::Wgsl => NativePolicy::Twin(TwinReason::Requested),
            NativeWitness::Rejected => NativePolicy::Twin(TwinReason::Rejected),
            _ => NativePolicy::DiagnosticMetal,
        };
        let preparation = NativePreparation::new(capabilities, policy);
        let shader = crate::shaders::Shader::NativeWitness.compile(device, PASS);
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(PASS),
            layout: None,
            module: &shader,
            entry_point: Some("toggle"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let (preparation, metal) = {
            let mut preparation = preparation;
            let metal = preparation.prepare_metal(device, || {
                crate::apple_tile::witness::Pipeline::prepare(device)
            });
            (preparation, metal)
        };
        let selection = match preparation.twin_reason() {
            Some(TwinReason::UnsupportedBackend | TwinReason::PreparationRefused) => {
                NativeWitness::Rejected
            }
            _ => selection,
        };
        Arc::new(Self {
            device: device.clone(),
            queue: queue.clone(),
            selection,
            pipeline,
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            metal,
            preparation,
            extents: Mutex::new(BTreeMap::new()),
        })
    }
}

impl NativeProvider for Factory {
    fn declare(
        &self,
        graph: &mut FrameGraph,
        scene: &ResourceId,
        extent: [u32; 2],
    ) -> Result<NativeGraph, RenderError> {
        graph::append(graph, scene, extent)
    }

    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            clippy::arc_with_non_send_sync,
            reason = "Native completion retains this same binding; WebGPU handles stay on their browser thread"
        )
    )]
    fn bind(
        &self,
        textures: &GraphTextures,
        graph: &NativeGraph,
    ) -> Result<Arc<NativeBinding>, RenderError> {
        let source = graph
            .bindings
            .first()
            .and_then(|source| textures.texture(source))
            .ok_or(RenderError::GraphPassMismatch)?;
        let extent = [source.width(), source.height()];
        let mut extents = self
            .extents
            .lock()
            .map_err(|_| RenderError::NativePreparationPoisoned)?;
        extents.retain(|_, value| value.strong_count() != 0);
        let provider = if let Some(provider) = extents.get(&extent).and_then(Weak::upgrade) {
            provider
        } else {
            let provider = Arc::new(Provider::new(self, extent)?);
            extents.insert(extent, Arc::downgrade(&provider));
            provider
        };
        let source = provider.stale.as_ref().unwrap_or(source).clone();
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let raw_source = provider
            .raw
            .as_ref()
            .map(|_| crate::apple_tile::texture(&source).ok_or(RenderError::StaleTarget))
            .transpose()?;
        let state = Arc::clone(&provider.state);
        Ok(NativeBinding::new(
            &state,
            Arc::new(Binding {
                provider,
                source,
                #[cfg(any(target_os = "macos", target_os = "ios"))]
                raw_source,
            }),
        ))
    }
}

fn texture(device: &wgpu::Device, queue: &wgpu::Queue, [width, height]: [u32; 2]) -> wgpu::Texture {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(PASS),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: SCENE_FORMAT,
        usage: wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        &vec![0; width as usize * height as usize * 8],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 8),
            rows_per_image: Some(height),
        },
        size,
    );
    texture
}

impl Provider {
    fn new(factory: &Factory, extent: [u32; 2]) -> Result<Self, RenderError> {
        if extent.contains(&0) {
            return Err(RenderError::TargetExtent);
        }
        let output = texture(&factory.device, &factory.queue, extent);
        let histories = std::array::from_fn(|_| texture(&factory.device, &factory.queue, extent));
        let zero = texture(&factory.device, &factory.queue, [1, 1]);
        let view = output.create_view(&wgpu::TextureViewDescriptor::default());
        let histories_views = histories
            .each_ref()
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
        let groups = std::array::from_fn(|side| {
            factory
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(PASS),
                    layout: &factory.pipeline.get_bind_group_layout(0),
                    entries: &[&histories_views[side], &view, &histories_views[1 - side]]
                        .into_iter()
                        .enumerate()
                        .map(|(index, view)| wgpu::BindGroupEntry {
                            binding: u32::try_from(index).expect("three bindings"),
                            resource: wgpu::BindingResource::TextureView(view),
                        })
                        .collect::<Vec<_>>(),
                })
        });
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let raw = if factory.metal.is_some() {
            let [output, previous, next] = [&output, &histories[0], &histories[1]]
                .map(|texture| crate::apple_tile::texture(texture).ok_or(RenderError::StaleTarget));
            Some([output?, previous?, next?])
        } else {
            None
        };
        Ok(Self {
            factory: Arc::new(Pipelines {
                selection: factory.selection,
                pipeline: factory.pipeline.clone(),
                #[cfg(any(target_os = "macos", target_os = "ios"))]
                metal: factory.metal.clone(),
            }),
            output,
            view,
            histories,
            zero,
            stale: (factory.selection == NativeWitness::StaleSource)
                .then(|| texture(&factory.device, &factory.queue, extent)),
            groups,
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            raw,
            extent,
            state: factory
                .preparation
                .state(PASS, "WGSL copy/select witness", true),
            side: AtomicUsize::new(0),
            phase: AtomicU64::new(0),
        })
    }
}

impl PreparedProvider for Binding {
    fn record_before_native(&self, encoder: &mut wgpu::CommandEncoder, reset: bool) {
        let provider = &self.provider;
        if reset && provider.factory.selection != NativeWitness::NoReset {
            encoder.copy_texture_to_texture(
                provider.zero.as_image_copy(),
                provider.histories[self.side()].as_image_copy(),
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    fn record_twin(&self, encoder: &mut wgpu::CommandEncoder, _: NativeFrameValues, reset: bool) {
        let provider = &self.provider;
        let side = self.side();
        for destination in [&provider.output, &provider.histories[1 - side]] {
            encoder.copy_texture_to_texture(
                self.source.as_image_copy(),
                destination.as_image_copy(),
                wgpu::Extent3d {
                    width: provider.extent[0],
                    height: provider.extent[1],
                    depth_or_array_layers: 1,
                },
            );
            if self.select_history(reset) {
                let [width, height] = provider.extent;
                let origin = wgpu::Origin3d {
                    x: width / 4,
                    y: height / 4,
                    z: 0,
                };
                encoder.copy_texture_to_texture(
                    wgpu::TexelCopyTextureInfo {
                        origin,
                        ..provider.histories[side].as_image_copy()
                    },
                    wgpu::TexelCopyTextureInfo {
                        origin,
                        ..destination.as_image_copy()
                    },
                    wgpu::Extent3d {
                        width: (width / 4).max(1),
                        height: (height / 4).max(1),
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&provider.factory.pipeline);
        pass.set_bind_group(0, &provider.groups[side], &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn record_native(
        &self,
        buffer: &crate::apple_tile::CommandBuffer,
        _: NativeFrameValues,
        reset: bool,
    ) {
        let provider = &self.provider;
        let side = self.side();
        let raw = provider.raw.as_ref().expect("native textures prepared");
        provider
            .factory
            .metal
            .as_ref()
            .expect("native pipeline prepared")
            .encode(
                buffer,
                &crate::apple_tile::witness::Frame {
                    source: self.raw_source.as_ref().expect("native source prepared"),
                    previous: &raw[1 + side],
                    output: &raw[0],
                    next: &raw[2 - side],
                    extent: provider.extent,
                    select_history: self.select_history(reset),
                    no_commands: provider.factory.selection == NativeWitness::NoCommands,
                    no_tile: provider.factory.selection == NativeWitness::NoTile,
                },
            );
    }

    fn submitted(&self, reset: bool, _: bool) {
        let provider = &self.provider;
        if provider.factory.selection != NativeWitness::StaleHistory {
            provider.side.store(1 - self.side(), Ordering::Relaxed);
        }
        if reset {
            provider.phase.store(1, Ordering::Relaxed);
        } else {
            provider.phase.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn output(&self) -> &wgpu::Texture {
        &self.provider.output
    }

    fn output_view(&self) -> &wgpu::TextureView {
        &self.provider.view
    }

    fn history(&self) -> Option<&wgpu::Texture> {
        Some(&self.provider.histories[1 - self.side()])
    }
}

pub(super) fn report(binding: &NativeBinding) -> Option<NativeWitnessReport> {
    let witness = binding.prepared::<Binding>()?;
    Some(NativeWitnessReport::from_provider(
        binding.report(),
        witness.provider.factory.selection,
        witness.resident_bytes(),
    ))
}

impl Binding {
    fn resident_bytes(&self) -> u64 {
        let provider = &self.provider;
        u64::from(provider.extent[0])
            * u64::from(provider.extent[1])
            * 8
            * (3 + u64::from(provider.stale.is_some()))
            + 8
    }

    fn side(&self) -> usize {
        self.provider.side.load(Ordering::Relaxed)
    }

    fn select_history(&self, reset: bool) -> bool {
        !reset && self.provider.phase.load(Ordering::Relaxed) % 2 == 1
    }
}
