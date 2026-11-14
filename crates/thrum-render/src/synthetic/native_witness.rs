//! Labelled composition diagnostic, disabled for ordinary scene rendering.
use super::{RenderError, SCENE_FORMAT};
use crate::{capabilities::DeviceCapabilities, runtime::DeviceSite};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
};

pub(super) const RESET: &str = "native witness reset";
pub(super) const PASS: &str = "native stateful copy/select witness";
pub(crate) mod graph;

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

pub(super) struct Factory {
    device: DeviceSite,
    queue: wgpu::Queue,
    selection: NativeWitness,
    pipeline: wgpu::ComputePipeline,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    metal: Option<crate::apple_tile::Pipeline>,
    extents: Mutex<BTreeMap<[u32; 2], Weak<Provider>>>,
}

pub(super) struct Provider {
    factory: Arc<Pipelines>,
    pub(super) output: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    histories: [wgpu::Texture; 2],
    zero: wgpu::Texture,
    stale: Option<wgpu::Texture>,
    groups: [wgpu::BindGroup; 2],
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    raw: [crate::apple_tile::Texture; 3],
    extent: [u32; 2],
    reserved: AtomicBool,
    valid: AtomicBool,
    side: AtomicUsize,
    encoded: AtomicU64,
    submitted: AtomicU64,
    discarded: AtomicU64,
    resets: AtomicU64,
    completed: Arc<AtomicU64>,
}

struct Pipelines {
    selection: NativeWitness,
    pipeline: wgpu::ComputePipeline,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    metal: Option<crate::apple_tile::Pipeline>,
}

pub(crate) struct Binding {
    pub(super) provider: Arc<Provider>,
    source: wgpu::Texture,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    raw_source: crate::apple_tile::Texture,
}

/// Exclusive candidate. Every early return/drop invalidates provider history.
pub(crate) struct NativeFrame {
    binding: Arc<Binding>,
    side: usize,
    reset: bool,
    select_history: bool,
    committed: bool,
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
    ) -> Result<Arc<Self>, RenderError> {
        let device = capabilities.site();
        if selection != NativeWitness::Wgsl
            && capabilities.adapter().backend != wgpu::Backend::Metal
        {
            return Err(RenderError::Validation(
                "native witness requires registry Metal backend".into(),
            ));
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS),
            source: wgpu::ShaderSource::Wgsl(include_str!("native_witness.wgsl").into()),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(PASS),
            layout: None,
            module: &shader,
            entry_point: Some("toggle"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let metal = if selection == NativeWitness::Wgsl {
            None
        } else {
            Some(crate::apple_tile::Pipeline::prepare(device).map_err(RenderError::Validation)?)
        };
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        if metal.is_some() {
            device.native_prepared();
        }
        Ok(Arc::new(Self {
            device: device.clone(),
            queue: queue.clone(),
            selection,
            pipeline,
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            metal,
            extents: Mutex::new(BTreeMap::new()),
        }))
    }

    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            clippy::arc_with_non_send_sync,
            reason = "Native completion retains this same binding; WebGPU handles stay on their browser thread"
        )
    )]
    pub(super) fn binding(&self, source: &wgpu::Texture) -> Result<Arc<Binding>, RenderError> {
        let extent = [source.width(), source.height()];
        let mut extents = self
            .extents
            .lock()
            .map_err(|_| RenderError::Validation("native preparation lock poisoned".into()))?;
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
        let raw_source = crate::apple_tile::texture(&source).ok_or(RenderError::StaleTarget)?;
        Ok(Arc::new(Binding {
            provider,
            source,
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            raw_source,
        }))
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
        let raw = [&output, &histories[0], &histories[1]]
            .map(|texture| crate::apple_tile::texture(texture).ok_or(RenderError::StaleTarget));
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let [output_raw, previous_raw, next_raw] = raw;
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
            raw: [output_raw?, previous_raw?, next_raw?],
            extent,
            reserved: AtomicBool::new(false),
            valid: AtomicBool::new(false),
            side: AtomicUsize::new(0),
            encoded: AtomicU64::new(0),
            submitted: AtomicU64::new(0),
            discarded: AtomicU64::new(0),
            resets: AtomicU64::new(0),
            completed: Arc::new(AtomicU64::new(0)),
        })
    }

    pub(super) fn report(&self) -> NativeWitnessReport {
        NativeWitnessReport {
            selection: self.factory.selection,
            encoded: self.encoded.load(Ordering::Relaxed),
            submitted: self.submitted.load(Ordering::Relaxed),
            discarded: self.discarded.load(Ordering::Relaxed),
            resets: self.resets.load(Ordering::Relaxed),
            completed: self.completed.load(Ordering::Relaxed),
            resident_bytes: u64::from(self.extent[0])
                * u64::from(self.extent[1])
                * 8
                * (3 + u64::from(self.stale.is_some()))
                + 8,
        }
    }
}

impl Binding {
    pub(super) fn reserve(self: &Arc<Self>, cut: bool) -> Result<NativeFrame, RenderError> {
        let provider = &self.provider;
        provider
            .reserved
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| RenderError::Validation("native candidate already encoded".into()))?;
        let reset = cut || !provider.valid.load(Ordering::Acquire);
        let order = provider.submitted.load(Ordering::Relaxed);
        provider.encoded.fetch_add(1, Ordering::Relaxed);
        Ok(NativeFrame {
            binding: Arc::clone(self),
            side: provider.side.load(Ordering::Relaxed),
            reset,
            select_history: !reset && order % 2 == 1,
            committed: false,
        })
    }
}

impl NativeFrame {
    pub(super) fn reset(&self, encoder: &mut wgpu::CommandEncoder) {
        let provider = &self.binding.provider;
        if self.reset && provider.factory.selection != NativeWitness::NoReset {
            encoder.copy_texture_to_texture(
                provider.zero.as_image_copy(),
                provider.histories[self.side].as_image_copy(),
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            provider.resets.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub(super) fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        segments: &mut Vec<wgpu::CommandBuffer>,
    ) {
        let provider = &self.binding.provider;
        if provider.factory.selection == NativeWitness::Wgsl {
            for destination in [&provider.output, &provider.histories[1 - self.side]] {
                encoder.copy_texture_to_texture(
                    self.binding.source.as_image_copy(),
                    destination.as_image_copy(),
                    wgpu::Extent3d {
                        width: provider.extent[0],
                        height: provider.extent[1],
                        depth_or_array_layers: 1,
                    },
                );
                if self.select_history {
                    let [width, height] = provider.extent;
                    let origin = wgpu::Origin3d {
                        x: width / 4,
                        y: height / 4,
                        z: 0,
                    };
                    encoder.copy_texture_to_texture(
                        wgpu::TexelCopyTextureInfo {
                            origin,
                            ..provider.histories[self.side].as_image_copy()
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
            pass.set_bind_group(0, &provider.groups[self.side], &[]);
            pass.dispatch_workgroups(1, 1, 1);
        } else {
            #[cfg(not(any(target_os = "macos", target_os = "ios")))]
            {
                let _ = (device, segments);
                unreachable!("native recording requires the admitted Apple backend");
            }
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            {
                let preceding = std::mem::replace(
                    encoder,
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default()),
                );
                segments.push(preceding.finish());
                let mut raw = device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(PASS) });
                provider
                    .factory
                    .metal
                    .as_ref()
                    .expect("native preparation admitted")
                    .encode(
                        &mut raw,
                        &crate::apple_tile::Frame {
                            source: &self.binding.raw_source,
                            previous: &provider.raw[1 + self.side],
                            output: &provider.raw[0],
                            next: &provider.raw[2 - self.side],
                            extent: provider.extent,
                            select_history: self.select_history,
                            no_commands: provider.factory.selection == NativeWitness::NoCommands,
                            no_tile: provider.factory.selection == NativeWitness::NoTile,
                        },
                    );
                segments.push(raw.finish());
            }
        }
    }

    pub(crate) fn submitted(mut self) -> NativeCompletion {
        let provider = &self.binding.provider;
        if provider.factory.selection != NativeWitness::StaleHistory {
            provider.side.store(1 - self.side, Ordering::Relaxed);
        }
        provider.submitted.fetch_add(1, Ordering::Relaxed);
        provider.valid.store(true, Ordering::Release);
        provider.reserved.store(false, Ordering::Release);
        self.committed = true;
        NativeCompletion {
            completed: Arc::clone(&provider.completed),
            #[cfg(not(target_arch = "wasm32"))]
            _binding: Arc::clone(&self.binding),
        }
    }

    pub(super) fn history(&self) -> &wgpu::Texture {
        &self.binding.provider.histories[1 - self.side]
    }
}

pub(crate) struct NativeCompletion {
    completed: Arc<AtomicU64>,
    // Native handles are retained until delivery. WebGPU owns its resource
    // lifetimes and only the thread-safe counter enters its Send callback.
    #[cfg(not(target_arch = "wasm32"))]
    _binding: Arc<Binding>,
}
impl NativeCompletion {
    pub(crate) fn completed(self) {
        self.completed.fetch_add(1, Ordering::Relaxed);
    }
}

impl Drop for NativeFrame {
    fn drop(&mut self) {
        if !self.committed {
            let provider = &self.binding.provider;
            provider.valid.store(false, Ordering::Release);
            provider.discarded.fetch_add(1, Ordering::Relaxed);
            provider.reserved.store(false, Ordering::Release);
        }
    }
}
