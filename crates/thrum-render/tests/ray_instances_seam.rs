//! A contribution that knows only the public scene and wgpu seams.

#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
mod support;

#[cfg(feature = "creation-counters")]
mod metal {
    #[cfg(target_os = "macos")]
    use super::{ray_support, support};
    #[cfg(target_os = "macos")]
    use thrum_render::contribution::{
        ContributionFrame, ContributionPasses, RayInstances, SceneContribution, SceneContributions,
        SceneSurface,
    };
    #[cfg(target_os = "macos")]
    use thrum_render::synthetic::{
        DisplayOverlay, FrameLighting, SyntheticLight, SyntheticScene, SyntheticView,
    };
    #[cfg(target_os = "macos")]
    use wgpu::util::DeviceExt;

    #[cfg(target_os = "macos")]
    struct Geometry {
        mesh: Option<wgpu::Blas>,
        vertices: Option<wgpu::Buffer>,
        placements: Vec<([f32; 3], u8)>,
        prebuild: bool,
    }

    #[cfg(target_os = "macos")]
    impl Geometry {
        fn new(placements: Vec<([f32; 3], u8)>) -> Self {
            Self {
                mesh: None,
                vertices: None,
                placements,
                prebuild: false,
            }
        }

        fn build(&self, encoder: &mut wgpu::CommandEncoder) {
            let size = size();
            let build = wgpu::BlasBuildEntry {
                blas: self.mesh.as_ref().expect("mesh"),
                geometry: wgpu::BlasGeometries::TriangleGeometries(vec![
                    wgpu::BlasTriangleGeometry {
                        size: &size,
                        vertex_buffer: self.vertices.as_ref().expect("vertices"),
                        first_vertex: 0,
                        vertex_stride: 12,
                        index_buffer: None,
                        first_index: None,
                        transform_buffer: None,
                        transform_buffer_offset: None,
                    },
                ]),
            };
            encoder.build_acceleration_structures([&build], []);
        }
    }

    #[cfg(target_os = "macos")]
    fn size() -> wgpu::BlasTriangleGeometrySizeDescriptor {
        wgpu::BlasTriangleGeometrySizeDescriptor {
            vertex_format: wgpu::VertexFormat::Float32x3,
            vertex_count: 6,
            index_format: None,
            index_count: None,
            flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
        }
    }

    #[cfg(target_os = "macos")]
    impl SceneContribution for Geometry {
        fn passes(&self) -> ContributionPasses {
            ContributionPasses {
                upload: true,
                compute: false,
            }
        }

        fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
            assert!(surface.ray_demand);
            self.mesh = Some(surface.device.create_blas(
                &wgpu::CreateBlasDescriptor {
                    label: Some("seam fixture mesh"),
                    flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                    update_mode: wgpu::AccelerationStructureUpdateMode::Build,
                },
                wgpu::BlasGeometrySizeDescriptors::Triangles {
                    descriptors: vec![size()],
                },
            ));
            self.vertices = Some(surface.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("seam fixture positions"),
                    contents: bytemuck::cast_slice(&[
                        [-0.5_f32, -0.4, 0.0],
                        [0.5, -0.4, 0.0],
                        [-0.5, 0.4, 0.0],
                        [-0.5, 0.4, 0.0],
                        [0.5, -0.4, 0.0],
                        [0.5, 0.4, 0.0],
                    ]),
                    usage: wgpu::BufferUsages::BLAS_INPUT,
                },
            ));
            if self.prebuild {
                let mut encoder = surface
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                self.build(&mut encoder);
                surface.queue.submit([encoder.finish()]);
                ray_support::support::wait(surface.device);
            }
            Ok(())
        }

        fn encode_upload(&mut self, encoder: &mut wgpu::CommandEncoder, _: &ContributionFrame) {
            if !self.prebuild {
                self.build(encoder);
            }
        }

        fn ray_instances(&self, instances: &mut RayInstances<'_>) {
            let mesh = self.mesh.as_ref().expect("mesh");
            for &([x, y, z], mask) in &self.placements {
                instances.offer(
                    mesh,
                    [1.0, 0.0, 0.0, x, 0.0, 1.0, 0.0, y, 0.0, 0.0, 1.0, z],
                    mask,
                );
            }
        }
    }

    thrum_test_support::macos_gpu_test! {
    fn a_contribution_offers_scene_membership_after_upload_without_any_draw() {
        let Some(gpu) = ray_support::support::gate("ray instances contribution seam") else {
            return;
        };
        let (device, queue, caps) = ray_support::support::device(&gpu, "ray-query", 1);
        pollster::block_on(async {
            let mut scene = SyntheticScene::grid(1).expect("scene");
            scene.ray_structure = true;
            let view = scene.view;
            let mut host = thrum_render::runtime::FrameHost::new(
                &caps,
                &queue,
                thrum_render::runtime::FrameConfig {
                    render_extent: [64, 64],
                    surface: None,
                },
                support::World,
                support::Source {
                    view,
                    lighting: FrameLighting::Diagnostic(SyntheticLight::default()),
                    instances: vec![([0.0, 0.0, 0.5], 0)],
                    lights: vec![],
                },
                scene,
                1.0,
            )
            .await
            .expect("ray scene");
            let control = host.capture_pixels().expect("control");
            host.set_scene_contribution(Box::new(SceneContributions::default()))
                .expect("attach empty");
            assert_eq!(host.capture_pixels().expect("empty contribution"), control);
            assert_eq!(host.ray_structure_instances().expect("list").len(), 1);
            host.set_scene_contribution(Box::new(SceneContributions::new(vec![Box::new(
                Geometry::new(vec![([12.0, 0.0, 0.0], 0b11), ([20.0, 0.0, 0.0], 0b10)]),
            )])))
            .expect("attach geometry");
            assert_eq!(host.capture_pixels().expect("geometry recording"), control);
            let list = host.ray_structure_instances().expect("list");
            assert_eq!(
                list.len(),
                3,
                "two placements of the same mesh outside the view"
            );
            assert_eq!((list[1].transform[3], list[1].mask), (12.0, 3));
            assert_eq!((list[2].transform[3], list[2].mask), (20.0, 2));
            let probe = ray_support::support::Probe::bind(&device, |layout| {
                host.ray_structure_probe_group(layout)
            });
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let readback = probe.record(
                &device,
                &mut encoder,
                &[[12.0, 0.0, -4.0, 0.0], [20.0, 0.0, -4.0, 0.0]],
            );
            let submission = queue.submit([encoder.finish()]);
            assert_eq!(
                ray_support::support::answers(&device, readback, submission),
                [list[1].slot + 1, 0]
            );
        });
    }
    }

    thrum_test_support::macos_gpu_test! {
    fn traced_view_distinguishes_adjoining_coplanar_placements() {
        let Some(gpu) = ray_support::support::gate("traced view instance identity") else {
            return;
        };
        let (device, queue, caps) = ray_support::support::device(&gpu, "ray-query", 1);
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.ray_structure = true;
        scene.view = SyntheticView::perspective(
            [0.0, 0.0, -3.0],
            [0.0; 3],
            [0.0, 1.0, 0.0],
            0.55,
            1.0,
            0.1,
            1000.0,
        )
        .expect("camera")
        .with_display_overlay(DisplayOverlay::TracedGeometry);
        let mut host = pollster::block_on(thrum_render::runtime::FrameHost::new(
            &caps,
            &queue,
            thrum_render::runtime::FrameConfig {
                render_extent: [64, 64],
                surface: None,
            },
            support::World,
            support::Source {
                view: scene.view,
                lighting: FrameLighting::Diagnostic(SyntheticLight::default()),
                instances: vec![([4.0, 0.0, 0.5], 0)],
                lights: vec![],
            },
            scene,
            1.0,
        ))
        .expect("traced scene");
        let mut planes = Geometry::new(vec![([-0.5, 0.0, 0.25], 0b11), ([0.5, 0.0, 0.25], 0b11)]);
        planes.prebuild = true;
        host.set_scene_contribution(Box::new(planes))
            .expect("attach two copies of the same plane");
        let pixels = host.capture_pixels().expect("submitted traced view");
        let list = host.ray_structure_instances().expect("scene membership");
        assert_eq!(list.len(), 3);
        let probe = ray_support::support::Probe::bind(&device, |layout| {
            host.ray_structure_probe_group(layout)
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let samples = probe.record(
            &device,
            &mut encoder,
            &[[-0.5, 0.0, -3.0, 0.0], [0.5, 0.0, -3.0, 0.0]],
        );
        let submission = queue.submit([encoder.finish()]);
        assert_eq!(
            ray_support::support::answers(&device, samples, submission),
            [list[1].slot + 1, list[2].slot + 1]
        );
        let pixel = |x: usize, y: usize| pixels.as_chunks::<4>().0[y * 64 + x];
        let [left, right, sky] = [pixel(16, 32), pixel(48, 32), pixel(16, 5)];
        assert_ne!(
            left, right,
            "depth, surface orientation and mesh are shared"
        );
        assert_ne!(left, sky);
        assert_ne!(right, sky);
        for y in 24..40 {
            assert_eq!(pixel(31, y), left, "left placement reaches its edge");
            assert_eq!(pixel(32, y), right, "right placement adjoins the left");
        }
        // Move the camera back and narrow its field of view so the same
        // placements still cover the same pixels, at a longer distance.
        host.source_mut().view = SyntheticView::perspective(
            [0.0, 0.0, -210.0],
            [0.0; 3],
            [0.0, 1.0, 0.0],
            2.0 * ((0.55_f32 / 2.0).tan() * 3.25 / 210.25).atan(),
            1.0,
            0.1,
            1000.0,
        )
        .expect("distant camera")
        .with_display_overlay(DisplayOverlay::TracedGeometry);
        let pixels = host.capture_pixels().expect("submitted distant view");
        let far_left = pixels.as_chunks::<4>().0[32 * 64 + 16];
        let far_right = pixels.as_chunks::<4>().0[32 * 64 + 48];
        assert_ne!(far_left, far_right);
        assert_ne!(far_left, sky);
        assert_ne!(far_right, sky);
        for (near, far) in [(left, far_left), (right, far_right)] {
            for channel in 0..3 {
                assert!(
                    far[channel] < near[channel],
                    "distance darkens each identity"
                );
            }
        }
        println!(
            "coplanar placement RGBA8: near {left:?} {right:?}, far {far_left:?} {far_right:?}, sky {sky:?}"
        );
    }
    }

    thrum_test_support::macos_gpu_test! {
    fn visual_replacement_on_the_same_device_reuses_prepared_meshes() {
        let Some(gpu) = ray_support::support::gate("ray mesh visual replacement") else {
            return;
        };
        let (_, queue, caps) = ray_support::support::device(&gpu, "ray-query", 1);
        let caps = caps.with_ray_preparation_observation();
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.ray_structure = true;
        let lighting = FrameLighting::Diagnostic(SyntheticLight::default());
        let mut host = pollster::block_on(thrum_render::runtime::FrameHost::new(
            &caps,
            &queue,
            thrum_render::runtime::FrameConfig {
                render_extent: [64, 64],
                surface: None,
            },
            support::World,
            support::Source {
                view: scene.view,
                lighting,
                instances: vec![([0.0, 0.0, 0.5], 0)],
                lights: vec![],
            },
            scene,
            1.0,
        ))
        .expect("ray scene");
        let initial = host.observe_ray_mesh_builds().expect("initial observer");
        assert_eq!(initial.take_recorded().len(), 2);
        let before = host.capture_pixels().expect("initial frame");
        let mut replacement =
            pollster::block_on(host.visual_prepare_inputs().prepare(lighting, 1.0, None))
                .expect("same-device visual preparation");
        assert!(
            host.publish_visual(&mut replacement)
                .expect("replace visuals")
        );
        let probe = host
            .observe_ray_mesh_builds()
            .expect("replacement observer");
        assert!(probe.take_recorded().is_empty());
        assert_eq!(host.capture_pixels().expect("replaced frame"), before);
        assert!(probe.take_recorded().is_empty());
        println!("same-device visual preparation and first frame built zero mesh structures");
    }
    }
}
