//! Detached preparation of the textures and bindings sized by the render extent.

use super::{HISTORY_TEXTURE_OWNER, Layer, TRACE, TRACE_TEXTURE_OWNER, prepare, temporal};
use thrum_render::{
    contribution::EffectGraph,
    graph::{ResourceDescriptor, ResourceScope},
};

#[derive(Clone)]
enum FixedResource {
    Buffer(wgpu::Buffer, wgpu::BufferAddress, Option<wgpu::BufferSize>),
    View(wgpu::TextureView),
    Sampler(wgpu::Sampler),
}

impl FixedResource {
    fn binding(&self) -> wgpu::BindingResource<'_> {
        match self {
            Self::Buffer(buffer, offset, size) => {
                wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer,
                    offset: *offset,
                    size: *size,
                })
            }
            Self::View(view) => wgpu::BindingResource::TextureView(view),
            Self::Sampler(sampler) => wgpu::BindingResource::Sampler(sampler),
        }
    }
}

const FRESH_BINDINGS: [u32; 2] = [3, 4];

#[derive(Clone)]
pub(super) struct TraceBindings {
    layout: wgpu::BindGroupLayout,
    fixed: Vec<(u32, FixedResource)>,
}

impl TraceBindings {
    pub(super) fn new(
        layout: &wgpu::BindGroupLayout,
        entries: &[wgpu::BindGroupEntry<'_>],
    ) -> Self {
        let fixed = entries
            .iter()
            .filter(|entry| !FRESH_BINDINGS.contains(&entry.binding))
            .map(|entry| {
                let resource = match &entry.resource {
                    wgpu::BindingResource::Buffer(binding) => {
                        FixedResource::Buffer(binding.buffer.clone(), binding.offset, binding.size)
                    }
                    wgpu::BindingResource::TextureView(view) => {
                        FixedResource::View((*view).clone())
                    }
                    wgpu::BindingResource::Sampler(sampler) => {
                        FixedResource::Sampler((*sampler).clone())
                    }
                    _ => unreachable!(
                        "cloud trace uses only individual textures, buffers and samplers"
                    ),
                };
                (entry.binding, resource)
            })
            .collect();
        Self {
            layout: layout.clone(),
            fixed,
        }
    }

    fn bind(&self, device: &wgpu::Device, textures: &[wgpu::Texture; 2]) -> wgpu::BindGroup {
        let views = textures.each_ref().map(super::stop_view);
        let entries: Vec<_> = self
            .fixed
            .iter()
            .map(|(binding, resource)| wgpu::BindGroupEntry {
                binding: *binding,
                resource: resource.binding(),
            })
            .chain(
                FRESH_BINDINGS
                    .into_iter()
                    .zip(&views)
                    .map(|(binding, view)| wgpu::BindGroupEntry {
                        binding,
                        resource: wgpu::BindingResource::TextureView(view),
                    }),
            )
            .collect();
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(TRACE),
            layout: &self.layout,
            entries: &entries,
        })
    }
}

pub(super) struct Bindings {
    pub(super) trace: TraceBindings,
    pub(super) compose_layout: wgpu::BindGroupLayout,
}

pub(crate) struct Inputs {
    trace: TraceBindings,
    compose_layout: wgpu::BindGroupLayout,
    resolve_layout: wgpu::BindGroupLayout,
    params: wgpu::Buffer,
    extent: [u32; 2],
    per_pixel_stops: bool,
}

pub(crate) struct Targets {
    fresh: [wgpu::Texture; 2],
    trace: wgpu::BindGroup,
    compose: wgpu::BindGroup,
    resolve: temporal::Targets,
}

impl Layer {
    pub(crate) fn resize_inputs(&self, extent: [u32; 2]) -> Inputs {
        let size = self.textures[0].size();
        Inputs {
            trace: self.resize_bindings.trace.clone(),
            compose_layout: self.resize_bindings.compose_layout.clone(),
            resolve_layout: self.temporal.layout.clone(),
            params: self.params.clone(),
            per_pixel_stops: self.per_pixel_stops,
            extent: [
                extent[0].max(2 * size.width),
                extent[1].max(2 * size.height),
            ],
        }
    }
}

impl Inputs {
    pub(crate) fn prepare(self, device: &wgpu::Device) -> Targets {
        let fresh = prepare::transport_textures(device, self.extent, self.per_pixel_stops);
        let trace = self.trace.bind(device, &fresh);
        let compose = temporal::compose_group(device, &self.params, &fresh, &self.compose_layout);
        let resolve = temporal::Targets::new(
            device,
            &self.params,
            &fresh,
            &self.compose_layout,
            &self.resolve_layout,
            None,
        );
        Targets {
            fresh,
            trace,
            compose,
            resolve,
        }
    }
}

impl Targets {
    pub(crate) fn update_graph(&self, graph: &mut EffectGraph) {
        // The bindings and subresource accesses are unchanged. Only the
        // descriptors for the trace outputs and the two history pairs grow.
        for (descriptor, scope) in &mut graph.resources {
            let (owner, first, textures) = match *scope {
                ResourceScope::Persistent { owner, .. } => {
                    (owner, TRACE_TEXTURE_OWNER, &self.fresh)
                }
                ResourceScope::History { owner, .. } => {
                    (owner, HISTORY_TEXTURE_OWNER, &self.resolve.textures[0])
                }
                _ => continue,
            };
            for (index, texture) in textures.iter().enumerate() {
                if owner == first + index as u64 {
                    *descriptor = ResourceDescriptor::of_texture(texture);
                }
            }
        }
    }

    pub(crate) fn resource_bytes(&self) -> Vec<(String, u64)> {
        super::target_bytes(&self.fresh, &self.resolve.textures)
    }

    pub(crate) fn install(self, layer: &mut Layer) {
        layer.textures = self.fresh;
        layer.trace_group = self.trace;
        layer.compose_group = self.compose;
        layer.temporal.targets = self.resolve;
        layer.invalidate();
    }
}
