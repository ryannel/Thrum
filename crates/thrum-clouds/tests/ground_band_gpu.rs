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
    let (_, products, _) = ground_support::render(&gpu, &ground, &upper, high_elevation);
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
        (depth[2], surface_sun(&gpu, &environment))
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

fn band_centre(gpu: &Opened, band: &wgpu::Texture) -> [f32; 4] {
    let values = ground_support::read(gpu, band, Mip::layer(0, 0));
    values[(band.width() * band.height() / 2) as usize]
}

fn surface_sun(gpu: &Opened, environment: &EnvironmentProducts) -> f32 {
    let device = DeviceSite::untracked(gpu.device.clone());
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("surface Sun observer"),
        size: 4,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("surface Sun observer"),
        entries: &[binding::storage(0, wgpu::ShaderStages::COMPUTE, false, 4)],
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
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("surface Sun observer"),
        source: wgpu::ShaderSource::Wgsl(
            format!(
                "{}\n{}",
                environment.shader_source,
                r"
            @group(1) @binding(0) var<storage, read_write> observed: f32;
            @compute @workgroup_size(1)
            fn observe() {
                let receiver = vec3(0.0);
                observed = sun_irradiance(receiver).r * cloud_factor(receiver);
            }
            "
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
        pass.dispatch_workgroups(1, 1, 1);
    }
    let result = Readback::buffer(&device, &mut encoder, &output, ..);
    let submission = gpu.queue.submit([encoder.finish()]);
    f32::from_le_bytes(
        result
            .wait(&device, submission)
            .try_into()
            .expect("surface Sun lux"),
    )
}
