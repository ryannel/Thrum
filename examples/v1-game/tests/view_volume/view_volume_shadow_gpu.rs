//! The view volume’s direct-sun shadow and forward decoder on Metal.
mod fixture;

use fixture::{SCATTER_GAIN, STORAGE_EXPOSURE, SUN_LUX, Witness};
use thrum_render::{
    atmosphere::{AtmosphereFrame, GpuSunMapping, SunOpticalDepth},
    synthetic::SyntheticView,
};
use wgpu::util::DeviceExt;

fn sample(witness: &Witness, depth: f32) -> ([f32; 4], [f32; 4]) {
    let columns = (witness.grid[0] * witness.grid[1]) as usize * 8;
    let layers = witness.grid[2] - 2;
    let near = layers / 2;
    let coord = (depth / 500.0).sqrt() * f32::from(u16::try_from(near).expect("near layers"));
    let upper = u16::try_from(
        (0..near)
            .find(|layer| {
                coord <= f32::from(u16::try_from(*layer + 1).expect("validated fixture data"))
            })
            .expect("near depth")
            + 1,
    )
    .expect("validated fixture data");
    let layer_depth = |layer| {
        500.0
            * (f32::from(layer) / f32::from(u16::try_from(near).expect("validated fixture data")))
                .powi(2)
    };
    let start = layer_depth(upper - 1);
    let blend = (depth - start) / (layer_depth(upper) - start);
    let interpolate = |bytes: &[u8]| {
        let low = witness.volume_center(&bytes[columns * usize::from(upper - 1)..]);
        let high = witness.volume_center(&bytes[columns * usize::from(upper)..]);
        std::array::from_fn(|i| low[i] + (high[i] - low[i]) * blend)
    };
    (
        interpolate(&witness.scatter),
        interpolate(&witness.transmission),
    )
}

#[test]
fn metal_shadow_and_cloud_dim_only_the_solar_source_and_forward_reads_it_once() {
    let Some(gpu) = fixture::Gpu::open(7, "shadowed view volume") else {
        return;
    };
    let light = fixture::light(true, 1);
    let render = |caster, cloud| {
        let scene = fixture::scene(caster, cloud, true);
        let (mut prepared, mut target) = gpu.prepare(&scene, light);
        fixture::render(&gpu, &mut prepared, &mut target, &scene, light)
    };
    let clear = render(false, false);
    let terrain = render(true, false);
    let cloud = render(false, true);
    let opaque = {
        let mut scene = fixture::scene(false, true, true);
        scene.sun_optical_depth = Some(fixture::sun_path(64.0, -55.0, 1));
        let (mut prepared, mut target) = gpu.prepare(&scene, light);
        fixture::render(&gpu, &mut prepared, &mut target, &scene, light)
    };
    assert!(
        clear.transmission == terrain.transmission && clear.transmission == cloud.transmission,
        "shadow cannot change transmission or absolute night in-scatter"
    );
    let (lit, _) = sample(&clear, 28.0);
    let (shadowed, transmission) = sample(&terrain, 28.0);
    let (clouded, _) = sample(&cloud, 28.0);
    let (blocked, _) = sample(&opaque, 28.0);
    let sky = terrain.center();
    for i in 0..3 {
        assert!(
            shadowed[i] < lit[i] && clouded[i] < lit[i],
            "both shadow producers darken air"
        );
        let expected_cloud = blocked[i] + (lit[i] - blocked[i]) * (-1.0_f32).exp();
        assert!(
            (clouded[i] - expected_cloud).abs() <= 0.003 * lit[i],
            "the sampled optical depth attenuates single scatter at the cell"
        );
        let expected =
            STORAGE_EXPOSURE * (SUN_LUX * shadowed[i] / SCATTER_GAIN + transmission[3] / 65_536.0);
        assert!(
            (sky[i] - expected).abs() <= expected.abs() * 0.006 + 0.000_02,
            "black receiver at 28 m must display the volume's own scatter {sky:?} expected {expected}"
        );
    }
    let from = 14.0;
    let to = 26.0;
    let terrain_a = sample(&terrain, from).0;
    let terrain_b = sample(&terrain, to).0;
    let opaque_a = sample(&opaque, from).0;
    let opaque_b = sample(&opaque, to).0;
    for i in 0..3 {
        let known_blocked = opaque_b[i] - opaque_a[i];
        let actual = terrain_b[i] - terrain_a[i];
        assert!(
            (actual - known_blocked).abs() <= 0.008 * lit[i],
            "between 14 and 26 m the cascade admits no single scatter: {actual} vs {known_blocked}"
        );
    }
    // 55 m is the field boundary. The shadow deficit plateaus beyond it,
    // while the unshadowed transport continues to grow with range.
    let (a, _) = sample(&cloud, 83.0);
    let (b, _) = sample(&cloud, 132.0);
    let (clear_a, _) = sample(&clear, 83.0);
    let (clear_b, _) = sample(&clear, 132.0);
    for i in 0..3 {
        let delta = clear_a[i] - a[i];
        assert!(delta > 0.0);
        assert!((delta - (clear_b[i] - b[i])).abs() <= delta * 0.12);
    }
}

fn gpu_sun_path(gpu: &fixture::Gpu, optical_depth: f32) -> SunOpticalDepth {
    let field = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("horizon optical depth"),
        size: wgpu::Extent3d {
            width: 2,
            height: 2,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    gpu.queue.write_texture(
        field.as_image_copy(),
        bytemuck::cast_slice(&[[half::f16::from_f32(optical_depth).to_bits(), 0, 0, 0]; 4]),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(16),
            rows_per_image: None,
        },
        field.size(),
    );
    let (sine, cosine) = 2.0_f32.to_radians().sin_cos();
    let mapping = gpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("floored Sun path mapping"),
            contents: bytemuck::bytes_of(&GpuSunMapping {
                // The lifted path projects the near air to about 43 km. A ray
                // along the true Sun misses this window and reads clear far air.
                region: [42_000.0, -1000.0, 44_000.0, 1000.0],
                plane_drift: [2500.0, 0.0, 0.0, 1.0],
                path_border: [cosine, sine, 0.0, 100.0],
                far: [0.0; 4],
                near: [0.0; 4],
                middle: [0.0; 4],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::STORAGE,
        });
    SunOpticalDepth::from_gpu(field, mapping, 1).expect("GPU Sun path")
}

#[test]
fn metal_sun_path_uses_the_providers_horizon_rule() {
    let Some(gpu) = fixture::Gpu::open(8, "Sun path across the horizon") else {
        return;
    };
    let mut scene = fixture::scene(false, false, false);
    scene.view = SyntheticView::perspective(
        [0.0, 1000.0, 0.0],
        [1.0, 1000.035, 0.0],
        [0.0, 1.0, 0.0],
        0.75,
        33.0 / 19.0,
        0.1,
        2000.0,
    )
    .expect("camera above ground looks along the setting Sun");
    let clear = gpu_sun_path(&gpu, 0.0);
    let shadow = gpu_sun_path(&gpu, 2.0);
    let mut render = |sun_y, field: SunOpticalDepth| {
        scene.sun_optical_depth = Some(field);
        let light = AtmosphereFrame::new([1.0, sun_y, 0.0], SUN_LUX, [1.0; 3], 1000.0, 1)
            .expect("Sun near the horizon")
            .with_view_volume()
            .with_nightglow();
        let (mut prepared, mut target) = gpu.prepare(&scene, light);
        fixture::render(&gpu, &mut prepared, &mut target, &scene, light)
    };
    for sun_y in [0.0001, 0.0, -0.0001] {
        let lit = render(sun_y, clear.clone());
        let dimmed = render(sun_y, shadow.clone());
        let lit_red = sample(&lit, 400.0).0[0];
        let dimmed_red = sample(&dimmed, 400.0).0[0];
        let terminal_red = dimmed.terminal()[0];
        println!("Sun y {sun_y}: lit {lit_red}, dimmed {dimmed_red}, terminal {terminal_red}");
        assert!(
            lit_red > 0.0 && (0.13..0.5).contains(&(dimmed_red / lit_red)),
            "GPU field with optical depth 2 dims direct light at Sun y {sun_y}"
        );
        assert!(terminal_red < 0.0, "terminal shadow at Sun y {sun_y}");
        assert!(
            lit.transmission == dimmed.transmission,
            "transmission and night are unchanged"
        );
    }
    // At this positive sine a true-Sun ray reaches the CPU field, but the
    // provider's declared minimum sine still disables the shadow.
    let cpu = |tau| {
        SunOpticalDepth::new(
            [2, 2],
            &[tau; 4],
            2500.0,
            [-1000.0, -1000.0, 16_000_000.0, 1000.0],
            0.01,
            1,
        )
        .expect("CPU Sun path")
    };
    let cpu_clear = render(0.0001, cpu(0.0));
    let cpu_opaque = render(0.0001, cpu(64.0));
    assert!(
        cpu_clear.scatter == cpu_opaque.scatter,
        "CPU field is clear below its Sun cutoff"
    );
}
