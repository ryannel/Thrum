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

mod terrain {
    use super::*;
    use glam::{Mat4, Vec3};
    use thrum_render::{
        abi::InstanceRecord,
        synthetic::{SyntheticPointLight, SyntheticScene},
    };

    fn block(slot: u32, center: [f32; 3], half: [f32; 3]) -> InstanceRecord {
        let mut value = InstanceRecord::identity(slot, 0);
        for axis in 0..3 {
            value.current[axis][axis] = half[axis];
            value.normal[axis][axis] = half[axis].recip();
            value.current[3][axis] = center[axis];
        }
        value.previous = value.current;
        value
    }

    async fn main_image(
        cap: &DeviceCapabilities,
        queue: &wgpu::Queue,
        scene: SyntheticScene,
        light: FrameLighting,
        side: u32,
        device: &wgpu::Device,
    ) -> Vec<u8> {
        let mut host = host_with_extent(cap, queue, scene, light, [side, side]).await;
        let (observer, control) = MainObserver::new([side, side]);
        host.set_scene_contribution(Box::new(observer))
            .expect("ordinary main observer");
        host.capture_pixels().expect("ordinary main draw");
        output(device, queue, &control.lock().expect("control").outputs[0])
    }

    #[test]
    fn vertical_displacement_and_texel_edge_mirror_match_with_terrain_shadows() {
        let Some(thrum_test_support::Opened {
            gpu, device, queue, ..
        }) = thrum_test_support::open(
            "vertical terrain equality",
            Some(wgpu::Backend::Metal),
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
        )
        else {
            return;
        };
        pollster::block_on(async {
            let cap = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
            let high = aimed([16.0, 1000.0, 0.0], [-16.0, -1000.0, -3000.0]);
            let main_mirror = aimed([16.0, 10.0, 0.0], [-2.0, -1.7, -1.0]);
            let mirror = aimed([16.0, 0.0, 0.0], [-2.0, 1.7, -1.0]);
            // Match the old fit's own light-space snapping, not just an approximate
            // world distance. The two recovered X coordinates straddle the 16 m edge.
            let recovered = |view: SyntheticView| {
                let eye = Vec3::from_array(view.camera_position());
                let direction = if eye.y.to_bits() == 10.0_f32.to_bits() {
                    Vec3::new(-2.0, -1.7, -1.0)
                } else {
                    Vec3::new(-2.0, 1.7, -1.0)
                };
                let matrix = glam::camera::rh::view::look_at_mat4(eye, eye + direction, Vec3::Y);
                matrix.inverse().w_axis.truncate()
            };
            let a = recovered(main_mirror);
            let b = recovered(mirror);
            assert_ne!(
                (a.x / 32.0).round(),
                (b.x / 32.0).round(),
                "old matrix fit must split the mirror pair"
            );
            eprintln!("mirror recovered positions {a:?}, {b:?}: different 32 m texels");
            for (label, main, view) in [
                ("same position", high, high),
                (
                    "80 m raised, repitched",
                    high,
                    aimed([16.0, 1080.0, 0.0], [-16.0, -1080.0, -3000.0]),
                ),
                ("edge mirror", main_mirror, mirror),
            ] {
                for side in [64, 16] {
                    for count in [0, 1, 2] {
                        let mut scene = SyntheticScene::grid(4).expect("scene");
                        scene.view = main;
                        scene.sun_shadows = true;
                        scene.materials[0].base_rgba = [0.8, 0.8, 0.8, 1.0];
                        scene.instances = vec![
                            block(0, [0.0, -0.5, -3000.0], [600.0, 0.5, 500.0]),
                            block(1, [250.0, 125.0, -3000.0], [125.0, 125.0, 400.0]),
                            block(2, [-3984.0, 3400.0, -2000.0], [1500.0, 1000.0, 0.5]),
                            block(3, [-3184.0, 4200.0, -1200.0], [300.0; 3]),
                        ];
                        scene.local_lights = vec![SyntheticPointLight {
                            position: [-3784.0, 3400.0, -1900.0],
                            radius: 100.0,
                            candela: [10_000.0, 1000.0, 100.0],
                        }];
                        // Authored photometric sky irradiance keeps frame lighting
                        // exactly fixed as the observer moves. The atmosphere case
                        // in the sibling test exercises view-local aerial products.
                        let light = FrameLighting::Photometric(
                            thrum_render::synthetic::PhotometricEnvironment {
                                toward_sun: [1.0 / 3.0_f64.sqrt(); 3],
                                sun_lux: 100_000.0,
                                sun_colour: [1.0; 3],
                                ambient_lux: [10_000.0, 12_000.0, 15_000.0],
                                background_radiance: [100.0; 3],
                            },
                        );
                        let mut reference = scene.clone();
                        reference.view = view
                            .with_sun_shadow_profile(count, 128.0)
                            .expect("reference profile");
                        let mut host = host_with_lighting(&cap, &queue, scene, light).await;
                        let (observer, control) = Observer::new(1);
                        control.lock().expect("control").requests[0] = Some(
                            SecondaryViewRequest::new(&view, [side, side], None, ViewAir::Camera)
                                .expect("request")
                                .with_shadows(count, 128.0)
                                .expect("profile"),
                        );
                        host.set_scene_contribution(Box::new(observer))
                            .expect("slot");
                        host.capture_pixels().expect("view draw");
                        let actual = output(
                            &device,
                            &queue,
                            &control.lock().expect("control").outputs[0],
                        );
                        let expected =
                            main_image(&cap, &queue, reference.clone(), light, side, &device).await;
                        reference.sun_shadows = false;
                        let clear = main_image(&cap, &queue, reference, light, side, &device).await;
                        let mut covered = 0;
                        let mut shadowed = 0;
                        let mut maximum = [0; 2];
                        let inverse = Mat4::from_cols_array_2d(&view.view_proj_columns()).inverse();
                        for y in 0..side as usize {
                            for x in 0..side as usize {
                                let (a, ad) = pixel(&actual, y * 64 + x);
                                let (e, ed) = pixel(&expected, y * side as usize + x);
                                assert_eq!(a[3] > 0.0, ed > 0.0, "{label} coverage");
                                if ed == 0.0 {
                                    continue;
                                }
                                covered += 1;
                                for c in 0..3 {
                                    let steps = half::f16::from_f32(a[c])
                                        .to_bits()
                                        .abs_diff(half::f16::from_f32(e[c]).to_bits());
                                    maximum[0] = maximum[0].max(u32::from(steps));
                                    assert!(
                                        steps <= 1,
                                        "{label} side={side} cascades={count} ({x},{y}) c={c}: {} vs {}, {steps} steps",
                                        a[c],
                                        e[c]
                                    );
                                }
                                let depth_steps = ad.to_bits().abs_diff(ed.to_bits());
                                maximum[1] = maximum[1].max(depth_steps);
                                assert!(depth_steps <= 1, "{label} depth {depth_steps} steps");
                                let clip = glam::Vec4::new(
                                    (x as f32 + 0.5) * 2.0 / side as f32 - 1.0,
                                    1.0 - (y as f32 + 0.5) * 2.0 / side as f32,
                                    ed,
                                    1.0,
                                );
                                let p = inverse * clip;
                                let distance = (p.truncate() / p.w
                                    - Vec3::from_array(view.camera_position()))
                                .length();
                                let clear = pixel(&clear, y * side as usize + x).0;
                                shadowed += usize::from(distance > 128.0 && e[0] < clear[0] * 0.9);
                            }
                        }
                        eprintln!(
                            "{label} side={side} cascades={count}: covered={covered}, height-shadow changed={shadowed}, max colour/depth steps={maximum:?}"
                        );
                        assert!(
                            covered > 10 && shadowed > 0,
                            "nonempty reference with distinguishable hill shadow beyond cascades"
                        );
                    }
                }
            }
        });
    }
}

#[test]
fn entities_match_the_ordinary_main_view_at_full_and_quarter_sides() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "secondary view equality",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
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
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "secondary GPU Sun mapping",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
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
        scene.view = camera(0.0, -2.0);
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
