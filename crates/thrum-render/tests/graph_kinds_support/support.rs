use thrum_render::capabilities::DeviceCapabilities;
use thrum_render::graph::{
    Access, AccessKind, Pass, PassScope, ResourceDescriptor, ResourceId, TextureKind,
};
use thrum_render::timing::TimingCategory;
pub(crate) mod gpu;
mod scenes;
pub(crate) use scenes::prove_scenes;

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
        timing: TimingCategory::Effects,
    }
}
pub(crate) fn device(
    gpu: &thrum_test_support::Gpu,
    generation: u64,
) -> (wgpu::Device, wgpu::Queue, DeviceCapabilities) {
    gpu.device_with_hardware(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction {
            features: Some(wgpu::Features::empty()),
            ..Default::default()
        },
        generation,
        &[],
    )
    .expect("admitted graph test device")
}
