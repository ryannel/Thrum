//! Stored-format comparison against the ordinary main draw at the requested
//! camera, extent and Sun profile. Both see Sun, sky light and a local lamp.
mod support;
use support::*;
use thrum_render::{
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    contribution::{SecondaryViewRequest, ViewAir},
    synthetic::{FrameLighting, SyntheticView},
};

#[test]
fn entities_match_the_ordinary_main_view_at_full_and_quarter_sides() {
    let Some(gpu) = thrum_test_support::gpu("secondary view equality", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    pollster::block_on(async {
        let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        for side in [64, 16] {
            for cascades in [0, 1, 2] {
                let (mut scene, _, view) = lit_view_scene();
                let eye = view.camera_position();
                scene.view = SyntheticView::perspective(
                    eye,
                    [eye[0], eye[1], eye[2] - 10.0],
                    [0.0, 1.0, 0.0],
                    1.0,
                    1.0,
                    0.1,
                    5000.0,
                )
                .expect("away main");
                let light = FrameLighting::Atmosphere(
                    AtmosphereFrame::new([0.7, 0.7, -0.1], 100_000.0, [1.0; 3], eye[1], 1)
                        .expect("air"),
                );
                let mut reference_scene = scene.clone();
                reference_scene.view = view
                    .with_sun_shadow_profile(cascades, 128.0)
                    .expect("reference Sun profile");
                let mut host = host_with_lighting(&capabilities, &queue, scene, light).await;
                let (observer, control) = Observer::new(1);
                control.lock().expect("control").requests[0] = Some(
                    SecondaryViewRequest::new(&view, [side, side], None, ViewAir::Camera)
                        .expect("request")
                        .with_shadows(cascades, 128.0)
                        .expect("Sun"),
                );
                host.set_scene_contribution(Box::new(observer))
                    .expect("observer");
                host.capture_pixels().expect("view draw");
                let actual = output(
                    &device,
                    &queue,
                    &control.lock().expect("control").outputs[0],
                );
                let mut reference =
                    host_with_extent(&capabilities, &queue, reference_scene, light, [side, side])
                        .await;
                let (observer, control) = MainObserver::new([side, side]);
                reference
                    .set_scene_contribution(Box::new(observer))
                    .expect("ordinary observer");
                reference.capture_pixels().expect("ordinary draw");
                let expected = output(
                    &device,
                    &queue,
                    &control.lock().expect("control").outputs[0],
                );
                let mut covered = 0;
                let mut maximum = [0_u32; 2];
                for y in 0..side as usize {
                    for x in 0..side as usize {
                        let (a, ad) = pixel(&actual, y * 64 + x);
                        let (e, ed) = pixel(&expected, y * side as usize + x);
                        assert_eq!(a[3] > 0.0, ed > 0.0, "same coverage");
                        if a[3] == 0.0 {
                            continue;
                        }
                        covered += 1;
                        for channel in 0..3 {
                            let steps = half::f16::from_f32(a[channel])
                                .to_bits()
                                .abs_diff(half::f16::from_f32(e[channel]).to_bits());
                            maximum[0] = maximum[0].max(u32::from(steps));
                            assert!(
                                steps <= 1,
                                "side={side} cascades={cascades} pixel=({x},{y}) channel={channel}: {} vs {}, {steps} stored steps",
                                a[channel],
                                e[channel]
                            );
                        }
                        let steps = ad.to_bits().abs_diff(ed.to_bits());
                        maximum[1] = maximum[1].max(steps);
                        assert!(steps <= 1, "depth {ad} vs {ed}, {steps} stored steps");
                    }
                }
                assert!(covered > 10, "nonempty lit reference");
                eprintln!(
                    "side={side} cascades={cascades}: {covered} pixels, max colour/depth steps={maximum:?}"
                );
            }
        }
    });
}

#[test]
fn a_gpu_written_sun_path_mapping_lights_the_view_like_the_main_draw() {
    use thrum_render::atmosphere::{GpuSunMapping, SunOpticalDepth};
    use wgpu::util::DeviceExt;
    let Some(gpu) =
        thrum_test_support::gpu("secondary GPU Sun mapping", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    pollster::block_on(async {
        let cap = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        let sun = glam::Vec3::new(0.0, 1.0, -1.0).normalize();
        let map = GpuSunMapping {
            region: [-100.0, -100.0, 100.0, 100.0],
            plane_drift: [10.0, 0.0, 0.0, 1.0],
            path_border: [sun.x, sun.y, sun.z, 1.0],
            far: [0.0; 4],
            near: [0.0; 4],
            middle: [0.0; 4],
        };
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("view test GPU Sun mapping"),
            contents: bytemuck::bytes_of(&map),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::STORAGE,
        });
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("view test GPU Sun path"),
            size: wgpu::Extent3d {
                width: 8,
                height: 8,
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
        let texels = vec![half::f16::from_f32(std::f32::consts::LN_2).to_bits(); 8 * 8 * 4];
        queue.write_texture(
            texture.as_image_copy(),
            bytemuck::cast_slice(&texels),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(64),
                rows_per_image: Some(8),
            },
            texture.size(),
        );
        let mut scene = thrum_render::synthetic::SyntheticScene::grid(1).expect("scene");
        scene.view = camera(0.0, 2.0);
        scene.sun_optical_depth =
            Some(SunOpticalDepth::from_gpu(texture, uniform, 1).expect("GPU product"));
        let lighting = FrameLighting::Atmosphere(
            AtmosphereFrame::new(sun.to_array(), 100_000.0, [1.0; 3], 0.0, 1).expect("air"),
        );
        let mut host = host_with_lighting(&cap, &queue, scene.clone(), lighting).await;
        let (observer, control) = Observer::new(1);
        control.lock().expect("control").requests[0] = Some(
            SecondaryViewRequest::new(&camera(0.0, -2.0), [64, 64], None, ViewAir::Camera)
                .expect("request"),
        );
        host.set_scene_contribution(Box::new(observer))
            .expect("observer");
        host.capture_pixels().expect("secondary with GPU mapping");
        let actual = output(
            &device,
            &queue,
            &control.lock().expect("control").outputs[0],
        );
        scene.view = camera(0.0, -2.0);
        let mut reference = host_with_lighting(&cap, &queue, scene.clone(), lighting).await;
        let (observer, control) = MainObserver::new([64, 64]);
        reference
            .set_scene_contribution(Box::new(observer))
            .expect("main observer");
        reference.capture_pixels().expect("ordinary GPU mapping");
        let expected = output(
            &device,
            &queue,
            &control.lock().expect("control").outputs[0],
        );
        scene.sun_optical_depth = None;
        let mut clear = host_with_lighting(&cap, &queue, scene, lighting).await;
        let (observer, control) = MainObserver::new([64, 64]);
        clear
            .set_scene_contribution(Box::new(observer))
            .expect("clear observer");
        clear.capture_pixels().expect("no optical depth");
        let clear = output(
            &device,
            &queue,
            &control.lock().expect("control").outputs[0],
        );
        let mut covered = 0;
        let mut darkened = 0;
        for i in 0..4096 {
            let (a, depth) = pixel(&actual, i);
            if depth == 0.0 {
                continue;
            }
            covered += 1;
            let e = pixel(&expected, i).0;
            for c in 0..3 {
                assert!(
                    half::f16::from_f32(a[c])
                        .to_bits()
                        .abs_diff(half::f16::from_f32(e[c]).to_bits())
                        <= 1,
                    "GPU Sun mapping at pixel {i}, channel {c}"
                );
            }
            darkened += usize::from(a[0] < pixel(&clear, i).0[0] * 0.9);
        }
        eprintln!(
            "GPU Sun mapping: {covered} covered pixels match the main draw; {darkened} darken against the absent provider"
        );
        assert!(
            covered > 100 && darkened > 10,
            "the real mapping must affect the scene's light"
        );
    });
}
