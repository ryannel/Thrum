//! A contribution that knows only the public scene and wgpu seams.

#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
mod support;

#[cfg(all(target_os = "macos", feature = "creation-counters"))]
mod metal {
    use super::{ray_support, support};
    use thrum_render::contribution::{
        ContributionFrame, ContributionPasses, RayInstances, SceneContribution, SceneContributions,
        SceneSurface,
    };
    use thrum_render::synthetic::{FrameLighting, SyntheticLight, SyntheticScene};
    use wgpu::util::DeviceExt;

    struct Geometry {
        mesh: Option<wgpu::Blas>,
        vertices: Option<wgpu::Buffer>,
    }

    fn size() -> wgpu::BlasTriangleGeometrySizeDescriptor {
        wgpu::BlasTriangleGeometrySizeDescriptor {
            vertex_format: wgpu::VertexFormat::Float32x3,
            vertex_count: 3,
            index_format: None,
            index_count: None,
            flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
        }
    }

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
                        [0.0, 0.8, 0.0],
                    ]),
                    usage: wgpu::BufferUsages::BLAS_INPUT,
                },
            ));
            Ok(())
        }

        fn encode_upload(&mut self, encoder: &mut wgpu::CommandEncoder, _: &ContributionFrame) {
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

        fn ray_instances(&self, instances: &mut RayInstances<'_>) {
            let mesh = self.mesh.as_ref().expect("mesh");
            for (x, mask) in [(12.0, 0b11), (20.0, 0b10)] {
                instances.offer(
                    mesh,
                    [1.0, 0.0, 0.0, x, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
                    mask,
                );
            }
        }

        fn computes(&self, _: &ContributionFrame) -> bool {
            false
        }
        fn encode_compute(&mut self, _: &mut wgpu::ComputePass<'_>, _: &ContributionFrame) {}
        fn encode_after_compute(&mut self, _: &mut wgpu::CommandEncoder) {}
        fn draw(&self, _: &mut wgpu::RenderPass<'_>) {}
        fn submitted(&mut self, _: bool) {}
    }

    #[test]
    fn a_contribution_offers_scene_membership_after_upload_without_any_draw() {
        let gpu = ray_support::support::gate("ray instances contribution seam");
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
                Geometry {
                    mesh: None,
                    vertices: None,
                },
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
            queue.submit([encoder.finish()]);
            assert_eq!(
                ray_support::support::answers(&device, &readback),
                [list[1].slot + 1, 0]
            );
        });
    }
}
