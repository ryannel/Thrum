//! Temporal MetalFX provider. TAAU writes the same output on the twin path.
use super::{
    NativeBinding, NativeFrameValues, NativeGraph, NativeGraphInputs, NativePolicy,
    NativePreparation, NativeProvider, NativeState, PreparedProvider,
};
use crate::{
    capabilities::DeviceCapabilities,
    error::RenderError,
    graph::{Access, AccessKind, FrameGraph, GraphTextures},
    pass_key::PassKey,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, Weak},
};

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod scaler;

pub(crate) const PASS: &str = PassKey::MetalFx.constant_name();
pub(crate) const OUTPUT_USAGE: wgpu::TextureUsages = wgpu::TextureUsages::TEXTURE_BINDING
    .union(wgpu::TextureUsages::STORAGE_BINDING)
    .union(wgpu::TextureUsages::RENDER_ATTACHMENT)
    .union(wgpu::TextureUsages::COPY_DST)
    .union(wgpu::TextureUsages::COPY_SRC);

/// Deliberate control arms for MetalFX target qualification.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MetalFxProbe {
    /// Encode the production inputs and reset decision.
    #[default]
    None,
    /// Reverse both motion axes without altering the scene.
    #[cfg(feature = "creation-counters")]
    ReverseMotion,
    /// Reverse only the horizontal sampling offset.
    #[cfg(feature = "creation-counters")]
    ReverseJitterX,
    /// Reverse only the vertical sampling offset.
    #[cfg(feature = "creation-counters")]
    ReverseJitterY,
    /// Suppress resets after the first native encode.
    #[cfg(feature = "creation-counters")]
    SkipReset,
}

pub(crate) struct Factory {
    capabilities: DeviceCapabilities,
    queue: wgpu::Queue,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    probe: MetalFxProbe,
    extents: Mutex<BTreeMap<[u32; 2], Weak<Extent>>>,
}

struct Extent {
    state: Arc<NativeState>,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    scaler: Option<Mutex<scaler::Scaler>>,
}

struct Binding {
    output: wgpu::Texture,
    output_view: wgpu::TextureView,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    raw: Option<NativeTextures>,
    // Retain the scaler and its history across slots and through completion.
    #[cfg_attr(
        not(any(target_os = "macos", target_os = "ios")),
        expect(
            dead_code,
            reason = "The twin retains its shared admission state across slots"
        )
    )]
    extent: Arc<Extent>,
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
struct NativeTextures {
    inputs: [super::Texture; 5],
    output: super::Texture,
}

impl Factory {
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            clippy::arc_with_non_send_sync,
            reason = "Native and browser providers use the same ownership model; WebGPU handles stay on the browser thread"
        )
    )]
    pub(crate) fn new(
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        probe: MetalFxProbe,
    ) -> Arc<Self> {
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        let _ = probe;
        Arc::new(Self {
            capabilities: capabilities.clone(),
            queue: queue.clone(),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            probe,
            extents: Mutex::new(BTreeMap::new()),
        })
    }

    fn extent(&self, extent: [u32; 2]) -> Result<Arc<Extent>, RenderError> {
        let mut extents = self
            .extents
            .lock()
            .map_err(|_| RenderError::NativePreparationPoisoned)?;
        extents.retain(|_, cached| cached.strong_count() != 0);
        if let Some(provider) = extents.get(&extent).and_then(Weak::upgrade) {
            return Ok(provider);
        }
        let preparation = NativePreparation::new(&self.capabilities, NativePolicy::Recipe);
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let (preparation, scaler) = {
            let site = self.capabilities.site();
            let mut preparation = preparation;
            let scaler = preparation.prepare_metal(site, true, || {
                scaler::Scaler::prepare(site, extent, self.probe)
            });
            (preparation, scaler.map(Mutex::new))
        };
        let provider = Arc::new(Extent {
            state: preparation.state(PASS, "TAAU", false),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            scaler,
        });
        extents.insert(extent, Arc::downgrade(&provider));
        Ok(provider)
    }
}

impl NativeProvider for Factory {
    fn declare(
        &self,
        graph: &mut FrameGraph,
        inputs: NativeGraphInputs<'_>,
        extent: [u32; 2],
    ) -> Result<NativeGraph, RenderError> {
        let (output, mut body) = inputs
            .reconstruction_twin
            .ok_or(RenderError::GraphPassMismatch)?;
        body.accesses.push(Access::new(
            inputs.exposure.clone(),
            0..1,
            AccessKind::SampledRead,
        ));
        let pass = graph.declare_native(PassKey::MetalFx, body);
        let bindings = inputs
            .attachments
            .into_iter()
            .chain([inputs.exposure])
            .cloned()
            .collect();
        Ok(NativeGraph {
            pass,
            output,
            history: None,
            extent,
            bindings,
        })
    }

    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            clippy::arc_with_non_send_sync,
            reason = "WebGPU stays on the browser thread"
        )
    )]
    fn bind(
        &self,
        textures: &GraphTextures,
        graph: &NativeGraph,
    ) -> Result<Arc<NativeBinding>, RenderError> {
        let provider = self.extent(graph.extent)?;
        let [width, height] = graph.extent;
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let output = self
            .capabilities
            .site()
            .create_texture(&wgpu::TextureDescriptor {
                label: Some(PASS),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: crate::render_config::SCENE_FORMAT,
                usage: OUTPUT_USAGE,
                view_formats: &[],
            });
        // The raw native write is invisible to wgpu's lazy initialization.
        self.queue.write_texture(
            output.as_image_copy(),
            &vec![0; width as usize * height as usize * 8],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 8),
                rows_per_image: Some(height),
            },
            size,
        );
        let output_view = output.create_view(&wgpu::TextureViewDescriptor::default());
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let raw = if provider.scaler.is_some() {
            let handles = graph
                .bindings
                .iter()
                .map(|id| {
                    textures
                        .texture(id)
                        .and_then(super::texture)
                        .ok_or(RenderError::StaleTarget)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Some(NativeTextures {
                inputs: handles
                    .try_into()
                    .map_err(|_| RenderError::GraphPassMismatch)?,
                output: super::texture(&output).ok_or(RenderError::StaleTarget)?,
            })
        } else {
            None
        };
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        let _ = textures;
        let state = Arc::clone(&provider.state);
        Ok(NativeBinding::new(
            &state,
            Arc::new(Binding {
                output,
                output_view,
                #[cfg(any(target_os = "macos", target_os = "ios"))]
                raw,
                extent: provider,
            }),
        ))
    }
}

impl PreparedProvider for Binding {
    fn record_twin(&self, _: &mut wgpu::CommandEncoder, _: NativeFrameValues, _: bool) {
        // The scene records its already prepared TAAU pass for this candidate.
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn record_native(&self, buffer: &super::CommandBuffer, values: NativeFrameValues, reset: bool) {
        let raw = self.raw.as_ref().expect("prepared MetalFX textures");
        self.extent
            .scaler
            .as_ref()
            .expect("admitted scaler")
            .lock()
            .expect("exclusive MetalFX recording")
            .encode(buffer, &raw.inputs, &raw.output, values, reset);
    }

    fn submitted(&self, _: bool, _: bool) {}

    fn output(&self) -> &wgpu::Texture {
        &self.output
    }

    fn output_view(&self) -> &wgpu::TextureView {
        &self.output_view
    }
}
