//! The production Sun-path producer preserves upper attenuation and slant shadows.
mod ground_support;
#[expect(
    dead_code,
    reason = "other clouds tests exercise the shared scene helpers"
)]
mod scene_fixture;

use thrum_cloud_model::{Density, Weather};
use thrum_render::{atmosphere::EnvironmentProducts, binding, runtime::DeviceSite};
use thrum_test_support::{
    Opened,
    readback::{Mip, Readback},
};

#[test]
fn metal_band_carries_the_upper_path_once_and_extends_bank_shade_down_sun() {
    let Some(gpu) = ground_support::open() else {
        return;
    };
    let field = ground_support::field();
    let ground = ground_support::fixture(ground_support::fog(true), false);
    let weather = Weather {
        field: &field,
        surface: Some(&ground.surface),
        ground: Some(&ground.fog),
    };

    let high_elevation = 40.0;
    let upper = ground_support::state(1.0, 0.0);
    let (_, products, environment) = ground_support::render(&gpu, &ground, &upper, high_elevation);
    let upper_expected = weather
        .sun_path(
            [0.0; 3],
            &upper,
            f64::from(high_elevation),
            0.0,
            4.0,
            1150.0,
            2048,
            Density::Coarse,
            |_| None,
        )
        .expect("upper reference")
        .optical_depth
        .value;
    let once = band_centre(&gpu, &products.band)[0];
    let error = |depth: f64, expected: f64| (depth.exp().recip() - expected.exp().recip()).abs();
    assert!(
        upper_expected > 0.3,
        "upper deck must change transmission: {upper_expected}"
    );
    assert!(
        error(f64::from(once), upper_expected) < 0.045,
        "upper depth GPU {once}, CPU {upper_expected}"
    );
    assert!(
        error(f64::from(once) + upper_expected, upper_expected) > 0.045,
        "a doubled upper path must fail"
    );
    let top_layer = products.band.depth_or_array_layers() - 1;
    let top_samples = ground_support::read(&gpu, &products.band, Mip::layer(0, top_layer));
    let top = top_samples[(products.band.width() * products.band.height() / 2) as usize][0];
    let factors = surface_factors(&gpu, &environment, &[[0.0, 95.9, 0.0], [0.0, 96.0, 0.0]]);
    assert!(((-top).exp() - factors[1]).abs() < 0.001);
    assert!((factors[0] - factors[1]).abs() < 0.01);

    let bank = ground_support::state(0.0, 1.0);
    for sun in [4.0, 10.0] {
        let (_, products, _) = ground_support::render(&gpu, &ground, &bank, sun);
        let target = weather
            .sun_path(
                [0.0; 3],
                &bank,
                f64::from(sun),
                0.0,
                4.0,
                96.0,
                2048,
                Density::Coarse,
                |_| None,
            )
            .expect("slant reference");
        let actual = band_centre(&gpu, &products.band)[0];
        assert!(!target.surface_blocked);
        let expected = target.optical_depth.value;
        assert!(
            error(f64::from(actual), expected) < 0.025,
            "{sun}° bank depth GPU {actual}, CPU {expected}"
        );
        assert!(
            error(0.0, expected) > 0.025,
            "a vertical column cannot shade this point"
        );
        eprintln!("{sun}° bank depth GPU {actual}, CPU {expected}");
    }
}

#[test]
fn metal_surface_visibility_changes_under_a_ridge_without_shading_surfaces() {
    let Some(gpu) = ground_support::open() else {
        return;
    };
    let clear = ground_support::state(0.0, 0.0);
    let check = |ridge| {
        let ground = ground_support::fixture(ground_support::fog(true), ridge);
        let (_host, products, environment) = ground_support::render(&gpu, &ground, &clear, 10.0);
        let depth = band_centre(&gpu, &products.band);
        assert_eq!(
            depth[0], 0.0,
            "surface visibility is separate from optical depth"
        );
        (
            depth[2],
            surface_factors(&gpu, &environment, &[[0.0; 3]])[0],
        )
    };
    let (visible, pixels) = check(false);
    let (blocked, under_ridge) = check(true);
    assert_eq!([visible, blocked], [1.0, 0.0]);
    assert!(pixels > 0.0);
    assert_eq!(
        under_ridge, pixels,
        "medium-only ridge visibility affected the surface Sun"
    );
}

#[test]
fn metal_tall_fog_keeps_sun_attenuation_through_the_ground_band() {
    let Some(gpu) = ground_support::open() else {
        return;
    };
    let field = ground_support::field();
    let state = ground_support::state(0.0, 1.0);
    let row = thrum_cloud_model::Fog {
        depth: 150.0,
        ..ground_support::fog(false)
    };
    for (ridge, heights) in [
        (
            false,
            &[
                95.9, 96.0, 96.1, 100.0, 125.0, 149.9, 150.0, 167.9, 168.0, 168.1,
            ][..],
        ),
        (true, &[100.0, 150.0, 160.0, 210.0, 228.0][..]),
    ] {
        let ground = ground_support::fixture(row, ridge);
        let weather = Weather {
            field: &field,
            surface: Some(&ground.surface),
            ground: Some(&ground.fog),
        };
        let sun = 10.0;
        let (_, _, environment) = ground_support::render(&gpu, &ground, &state, sun);
        let receivers: Vec<_> = heights.iter().map(|&y| [0.0, y, 0.0]).collect();
        for (point, actual) in receivers
            .iter()
            .zip(surface_factors(&gpu, &environment, &receivers))
        {
            let reference = weather
                .sun_path(
                    point.map(f64::from),
                    &state,
                    f64::from(sun),
                    0.0,
                    4.0,
                    250.0,
                    2048,
                    Density::Coarse,
                    |_| None,
                )
                .expect("slant reference");
            let expected = (-reference.optical_depth.value).exp();
            eprintln!(
                "ridge={ridge}, {} m: GPU {actual:.5}, query {expected:.5}",
                point[1]
            );
            assert!(
                (f64::from(actual) - expected).abs() < 0.035,
                "ridge={ridge}, {point:?}: GPU {actual}, query {expected}"
            );
        }
    }
}

fn band_centre(gpu: &Opened, band: &wgpu::Texture) -> [f32; 4] {
    let values = ground_support::read(gpu, band, Mip::layer(0, 0));
    values[(band.width() * band.height() / 2) as usize]
}

fn surface_factors(
    gpu: &Opened,
    environment: &EnvironmentProducts,
    receivers: &[[f32; 3]],
) -> Vec<f32> {
    let device = DeviceSite::untracked(gpu.device.clone());
    let receiver_count = u32::try_from(receivers.len()).expect("receiver count");
    let bytes = u64::from(receiver_count) * 4;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("surface Sun observer"),
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("surface Sun observer"),
        entries: &[binding::storage(
            0,
            wgpu::ShaderStages::COMPUTE,
            false,
            bytes,
        )],
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("surface Sun observer"),
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: output.as_entire_binding(),
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("surface Sun observer"),
        bind_group_layouts: &[Some(&environment.layout), Some(&layout)],
        immediate_size: 0,
    });
    let points = receivers
        .iter()
        .map(|[x, y, z]| format!("vec3<f32>({x:?}, {y:?}, {z:?})"))
        .collect::<Vec<_>>()
        .join(", ");
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("surface Sun observer"),
        source: wgpu::ShaderSource::Wgsl(
            format!(
                "{}\n
                @group(1) @binding(0) var<storage, read_write> observed: array<f32>;
                const receivers = array({points});
                @compute @workgroup_size(1)
                fn observe(@builtin(global_invocation_id) id: vec3<u32>) {{
                    observed[id.x] = cloud_factor(receivers[id.x]);
                }}",
                environment.shader_source,
            )
            .into(),
        ),
    });
    let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("surface Sun observer"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("observe"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, environment.group(0), &[]);
        pass.set_bind_group(1, &group, &[]);
        pass.dispatch_workgroups(receiver_count, 1, 1);
    }
    let result = Readback::buffer(&device, &mut encoder, &output, ..);
    let submission = gpu.queue.submit([encoder.finish()]);
    result
        .wait(&device, submission)
        .as_chunks::<4>()
        .0
        .iter()
        .copied()
        .map(f32::from_le_bytes)
        .collect()
}
