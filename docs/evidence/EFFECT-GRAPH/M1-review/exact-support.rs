use thrum_render::capabilities::DeviceCapabilities;
use thrum_render::graph::{
    Access, AccessKind, Pass, PassScope, ResourceDescriptor, ResourceId, TextureKind,
};
#[path = "gpu.rs"]
mod gpu;
pub(crate) use gpu::prove_kinds;

pub(crate) fn descriptor(dimension: TextureKind, size: u32) -> ResourceDescriptor {
    ResourceDescriptor::Texture {
        dimension,
        width: size,
        height: size,
        layers: if dimension == TextureKind::Cube {
            6
        } else if dimension == TextureKind::D2 {
            3
        } else {
            1
        },
        mips: 3,
        samples: 1,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
    }
}
pub(crate) fn access(
    resource: &ResourceId,
    range: std::ops::Range<u64>,
    kind: AccessKind,
) -> Access {
    Access {
        resource: resource.clone(),
        range,
        kind,
        store: true,
        view: None,
    }
}
pub(crate) fn pass(name: &str, accesses: Vec<Access>) -> Pass {
    Pass {
        name: name.into(),
        scope: PassScope::Shared,
        pipeline: None,
        accesses,
        external_side_effect: true,
    }
}
pub(crate) fn device(generation: u64) -> (wgpu::Device, wgpu::Queue, DeviceCapabilities) {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("M1 requires Metal");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("Metal device");
        let caps = DeviceCapabilities::capture(&adapter, &device, generation);
        (device, queue, caps)
    })
}
