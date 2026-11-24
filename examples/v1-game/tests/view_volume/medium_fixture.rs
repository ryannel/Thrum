use crate::fixture::{Gpu, Witness};
use thrum_render::{
    atmosphere::{AtmosphereFrame, ViewVolumeMediumTarget},
    synthetic::SyntheticScene,
};

pub(crate) fn render(
    gpu: &Gpu,
    scene: &SyntheticScene,
    light: AtmosphereFrame,
    coefficient: Option<(&str, [f32; 3])>,
) -> Witness {
    render_and_probe(gpu, scene, light, coefficient, |_| ()).0
}

pub(crate) fn render_and_probe<T>(
    gpu: &Gpu,
    scene: &SyntheticScene,
    light: AtmosphereFrame,
    coefficient: Option<(&str, [f32; 3])>,
    probe: impl FnOnce(&thrum_render::synthetic::PreparedSynthetic) -> T,
) -> (Witness, T) {
    let (mut prepared, mut target) = gpu.prepare(scene, light);
    if let Some((weight, [sigma, solar, ambient])) = coefficient {
        let medium = prepared
            .view_volume_medium_witness()
            .expect("medium target");
        let source = format!(
            "{}\n{}",
            medium.shader_source(),
            format_args!(
                "@compute @workgroup_size(8, 8)
            fn fill(@builtin(global_invocation_id) cell: vec3<u32>) {{
                if any(cell.xy >= textureDimensions(medium_out)) {{ return; }}
                let x = medium_sample_point(cell).w;
                let weight = {weight};
                textureStore(medium_out, vec2<i32>(cell.xy), i32(cell.z),
                    vec4(vec3({sigma:?}, {solar:?}, {ambient:?}) * weight, 0.0));
            }}"
            )
        );
        let device = &gpu.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("analytic slab medium"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("medium probe"),
            entries: &ViewVolumeMediumTarget::layout_entries(),
        });
        let group = medium.bind(device, &layout);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("medium probe"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("analytic slab medium"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("fill"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let size = medium.coefficients.size();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(
                size.width.div_ceil(8),
                size.height.div_ceil(8),
                size.depth_or_array_layers,
            );
        }
        gpu.queue.submit([encoder.finish()]);
    }
    let witness = crate::fixture::render(gpu, &mut prepared, &mut target, scene, light);
    (witness, probe(&prepared))
}

pub(crate) fn centre_layer(witness: &Witness, layer: u32, scatter: bool) -> [f32; 4] {
    let bytes = if scatter {
        &witness.scatter
    } else {
        &witness.transmission
    };
    let start = (witness.grid[0] * witness.grid[1] * 8 * layer) as usize;
    witness.volume_center(&bytes[start..])
}
