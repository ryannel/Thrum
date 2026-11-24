//! A receiver, optional casters and a camera from which to sample the receiver.

/// Coarse receiving geometry for Sun terminator tests.
pub mod terminator;

use glam::{Mat4, Vec3};
use thrum_render::{
    abi::InstanceRecord,
    capabilities::DeviceCapabilities,
    synthetic::{
        FrameLighting, OffscreenTarget, PhotometricEnvironment, PreparedSynthetic, SyntheticScene,
        SyntheticView,
    },
};

/// Square witness extent.
pub const EXTENT: u32 = 192;

/// Camera above and to one side of the fixture.
/// # Panics
/// The position must fit the camera profile.
pub fn view(centre: Vec3, side: f32) -> SyntheticView {
    SyntheticView::perspective(
        (centre + Vec3::new(side, 1.7, -3.0)).to_array(),
        centre.to_array(),
        Vec3::Y.to_array(),
        0.8,
        1.0,
        0.05,
        100.0,
    )
    .expect("fixture camera")
}

/// Instance of the cube with the given world-space half-extents.
pub fn box_at(slot: u32, centre: Vec3, half: Vec3) -> InstanceRecord {
    let mut record = InstanceRecord::identity(slot, 0);
    for axis in 0..3 {
        record.current[axis][axis] = half[axis];
        record.normal[axis][axis] = half[axis].recip();
    }
    record.current[3][..3].copy_from_slice(&centre.to_array());
    record.previous = record.current;
    record
}

/// Floor with its top at the fixture centre, plus the given casters.
/// # Panics
/// The scene and camera must be valid.
pub fn scene(centre: Vec3, extra: &[InstanceRecord]) -> SyntheticScene {
    let mut scene = SyntheticScene::grid(u32::try_from(extra.len() + 1).expect("fixture size"))
        .expect("fixture scene");
    scene.view = view(centre, 3.2);
    scene.instances[0] = box_at(0, centre - Vec3::Y * 0.1, Vec3::new(3.0, 0.1, 2.0));
    scene.instances[1..].clone_from_slice(extra);
    scene
}

/// Directional source with zero ambient and background light.
pub fn sun(direction: Vec3, lux: f64) -> FrameLighting {
    FrameLighting::Photometric(PhotometricEnvironment {
        toward_sun: direction.to_array().map(f64::from),
        sun_lux: lux,
        sun_colour: [1.0; 3],
        ambient_lux: [0.0; 3],
        background_radiance: [0.0; 3],
    })
}

/// Prepare the fixture and a square offscreen target.
/// # Panics
/// The scene and target must prepare successfully.
pub fn prepare(
    device: &wgpu::Device,
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    scene: &SyntheticScene,
    lighting: FrameLighting,
) -> (PreparedSynthetic, OffscreenTarget) {
    let prepared = pollster::block_on(PreparedSynthetic::prepare_with_capabilities(
        caps, queue, scene, lighting, 1.0,
    ))
    .expect("fixture preparation");
    let target = OffscreenTarget::new(device, &prepared, EXTENT, EXTENT).expect("target");
    (prepared, target)
}

/// Read the SDR pixel covering a world-space point in the fixture.
/// # Panics
/// The point must project inside the image.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "in-bounds projected coordinates select pixels"
)]
pub fn sample(view: &SyntheticView, pixels: &[u8], point: Vec3) -> [u8; 3] {
    let clip = Mat4::from_cols_array_2d(&view.view_proj_columns()) * point.extend(1.0);
    let x = ((0.5 + 0.5 * clip.x / clip.w) * EXTENT as f32) as usize;
    let y = ((0.5 - 0.5 * clip.y / clip.w) * EXTENT as f32) as usize;
    assert!(x < EXTENT as usize && y < EXTENT as usize);
    let offset = (y * EXTENT as usize + x) * 4;
    pixels[offset..offset + 3].try_into().expect("RGB")
}

/// Evaluate the composed Sun operation at two surface points.
/// # Panics
/// The scene must prepare its structure and the probe must submit.
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub fn probe_sun(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &PreparedSynthetic,
    direction: Vec3,
    normal: Vec3,
    points: [Vec3; 2],
) -> [f32; 2] {
    use wgpu::util::DeviceExt;

    // Move only the TLAS declaration so the probe can bind the scene's actual
    // structure through the diagnostic group. The forward operation is intact.
    let source = prepared.forward_source_for_probe().replace(
        "@group(1) @binding(15) var scene_structure",
        "@group(0) @binding(0) var scene_structure",
    );
    // The forward source also declares a view at the old group-0 slot.
    let source = source.replace(
        "@group(0) @binding(0) var<uniform> view:",
        "@group(3) @binding(27) var<uniform> view:",
    );
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("composed Sun visibility probe"),
        source: wgpu::ShaderSource::Wgsl(
            format!("{source}\n{}", include_str!("ray_support/sun_probe.wgsl")).into(),
        ),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("composed Sun visibility probe"),
        layout: None,
        module: &module,
        entry_point: Some("sample_sun"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let structure = prepared
        .ray_structure_probe_group(&pipeline.get_bind_group_layout(0))
        .expect("scene structure");
    let mut light = [0.0_f32; 52];
    light[..3].copy_from_slice(&direction.to_array());
    let directional = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("probe directional light"),
        contents: bytemuck::cast_slice(&light),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let light = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(1),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: directional.as_entire_binding(),
        }],
    });
    let points = points.map(|point| [point.extend(0.0).to_array(), normal.extend(0.0).to_array()]);
    let points = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("probe points on receiver"),
        contents: bytemuck::cast_slice(&points),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let buffer = |label, usage| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: 8,
            usage,
            mapped_at_creation: false,
        })
    };
    let values = buffer(
        "probe visibility",
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    );
    let payload = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(2),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: points.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: values.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &structure, &[]);
        pass.set_bind_group(1, &light, &[]);
        pass.set_bind_group(2, &payload, &[]);
        pass.dispatch_workgroups(2, 1, 1);
    }
    let readback =
        thrum_test_support::readback::Readback::buffer(device, &mut encoder, &values, ..);
    let submission = queue.submit([encoder.finish()]);
    let bytes = readback.wait(device, submission);
    std::array::from_fn(|index| {
        f32::from_le_bytes(bytes[index * 4..][..4].try_into().expect("f32"))
    })
}
