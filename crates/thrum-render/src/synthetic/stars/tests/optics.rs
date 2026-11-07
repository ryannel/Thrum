//! Optical referee using the production star pipeline and a unit transmission LUT.

use std::time::Duration;

use glam::{Mat3, Quat, Vec3};
use wgpu::{self as gpu, util::DeviceExt};

use super::{LIGHT_BYTES, SCENE_FORMAT, StarPrepared, SyntheticView, metal};

const DEPTH: gpu::TextureFormat = gpu::TextureFormat::Depth32Float;
const STORAGE: f32 = 64.0;
const CROP: u32 = 64;
const DIRECTIONS: [Vec3; 2] = [Vec3::NEG_Z, Vec3::new(0.6, 0.48, -0.64)];

#[derive(Clone, Copy)]
struct Edge {
    corner: [u32; 2],
    opaque: bool,
}

#[derive(Default)]
struct Footprint {
    total: f64,
    on_blocker: f64,
    moments: [f64; 2],
}

impl Footprint {
    fn assert_centred(&self, centre: [f32; 2]) {
        for (moment, expected) in self.moments.into_iter().zip(centre) {
            let actual = moment / self.total;
            assert!(
                (actual - f64::from(expected)).abs() < 0.08,
                "HDR footprint centre {actual} vs {expected}"
            );
        }
    }
}

struct Probe {
    stars: StarPrepared,
    blocker: gpu::RenderPipeline,
    observer: gpu::Buffer,
    exposure: gpu::Buffer,
    transmission: gpu::TextureView,
    flux: [f32; 2],
}

impl Probe {
    fn new(device: &gpu::Device, queue: &gpu::Queue) -> Self {
        let flux = [3.0, 6.0].map(|magnitude| 2.54e-6 * 10.0_f32.powf(-0.4 * magnitude));
        let stars = StarPrepared::new(
            device,
            &std::array::from_fn::<_, 2, _>(|i| {
                let [x, y, z] = DIRECTIONS[i].to_array();
                [x, y, z, flux[i]]
            }),
            device.limits().max_storage_buffer_binding_size,
            &[0; LIGHT_BYTES],
        )
        .expect("production star pipeline");
        let shader = device.create_shader_module(gpu::ShaderModuleDescriptor {
            label: Some("opaque half-plane"),
            source: gpu::ShaderSource::Wgsl(
                "@vertex fn vertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
                    let xy = array(vec2f(-1.0,-1.0),vec2f(3.0,-1.0),vec2f(-1.0,3.0));
                    return vec4f(xy[i],0.5,1.0);
                }"
                .into(),
            ),
        });
        let blocker = device.create_render_pipeline(&gpu::RenderPipelineDescriptor {
            label: Some("opaque half-plane"),
            layout: None,
            vertex: gpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: gpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: gpu::PrimitiveState::default(),
            depth_stencil: Some(gpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(true),
                depth_compare: Some(gpu::CompareFunction::Always),
                stencil: gpu::StencilState::default(),
                bias: gpu::DepthBiasState::default(),
            }),
            multisample: gpu::MultisampleState::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });
        let transmission = texture(
            device,
            [2, 6],
            SCENE_FORMAT,
            gpu::TextureUsages::TEXTURE_BINDING | gpu::TextureUsages::COPY_DST,
        );
        queue.write_texture(
            transmission.as_image_copy(),
            &[0, 0x3c].repeat(48),
            gpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(16),
                rows_per_image: Some(6),
            },
            transmission.size(),
        );
        let mut observer = [0.0; 20];
        // A one-kilometre observer keeps the few subpixel rays below horizontal
        // above the planet's tangent. The fixture has no extinction.
        observer[19] = 1000.0;
        Self {
            stars,
            blocker,
            observer: uniform(device, observer),
            exposure: uniform(device, [STORAGE, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            transmission: transmission.create_view(&gpu::TextureViewDescriptor::default()),
            flux,
        }
    }

    fn render(
        &self,
        device: &gpu::Device,
        queue: &gpu::Queue,
        target: &Target,
        centre: [f32; 2],
        star: u32,
        edge: Option<Edge>,
    ) -> Footprint {
        let [width, height] = target.extent;
        let [tx, ty] = target.tan_fov;
        let ray = Vec3::new(
            (2.0 * centre[0] / width as f32 - 1.0) * tx,
            (1.0 - 2.0 * centre[1] / height as f32) * ty,
            -1.0,
        )
        .normalize();
        // Construct the small angles directly. A rotation-arc shortcut can
        // round a subpixel offset near the optical axis to the identity.
        let yaw = Quat::from_rotation_y((-ray.x).atan2(-ray.z));
        let pitch = Quat::from_rotation_x(ray.y.asin());
        let direction = DIRECTIONS[star as usize];
        let orient = Quat::from_rotation_arc(direction, Vec3::NEG_Z);
        let basis = Mat3::from_quat(yaw * pitch * orient);
        let view = SyntheticView::perspective(
            [0.0; 3],
            [0.0, 0.0, -1.0],
            [0.0, 1.0, 0.0],
            46.0_f32.to_radians(),
            width as f32 / height as f32,
            0.1,
            100.0,
        )
        .expect("known perspective")
        .with_celestial_basis(basis.to_cols_array_2d())
        .expect("rotation basis");
        let view = self
            .stars
            .view_bytes(&view, target.extent)
            .expect("star view");
        let view = device.create_buffer_init(&gpu::util::BufferInitDescriptor {
            label: None,
            contents: &view,
            usage: gpu::BufferUsages::UNIFORM,
        });
        let resources = [
            self.stars.catalogue.as_entire_binding(),
            view.as_entire_binding(),
            gpu::BindingResource::TextureView(&target.depth),
            self.exposure.as_entire_binding(),
            gpu::BindingResource::TextureView(&self.transmission),
            gpu::BindingResource::Sampler(&self.stars.sampler),
            self.observer.as_entire_binding(),
        ];
        let mut bindings = [0, 1, 2, 3, 4, 5, 6].into_iter();
        let entries = resources.map(|resource| gpu::BindGroupEntry {
            binding: bindings.next().expect("one ID per fixture resource"),
            resource,
        });
        let group = device.create_bind_group(&gpu::BindGroupDescriptor {
            label: None,
            layout: &self.stars.layout,
            entries: &entries,
        });
        let mut encoder = device.create_command_encoder(&gpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&gpu::RenderPassDescriptor {
                depth_stencil_attachment: Some(gpu::RenderPassDepthStencilAttachment {
                    view: &target.depth,
                    depth_ops: Some(gpu::Operations {
                        load: gpu::LoadOp::Clear(0.0),
                        store: gpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            if let Some(Edge {
                corner: [x, y],
                opaque: true,
            }) = edge
            {
                pass.set_pipeline(&self.blocker);
                pass.set_scissor_rect(x, y, width - x, height - y);
                pass.draw(0..3, 0..1);
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&gpu::RenderPassDescriptor {
                color_attachments: &[Some(gpu::RenderPassColorAttachment {
                    view: &target.colour_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: gpu::Operations {
                        load: gpu::LoadOp::Clear(gpu::Color::BLACK),
                        store: gpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.stars.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..6, star..star + 1);
        }
        target.flux(device, queue, encoder, centre, edge)
    }
}

struct Target {
    extent: [u32; 2],
    tan_fov: [f32; 2],
    colour: gpu::Texture,
    colour_view: gpu::TextureView,
    depth: gpu::TextureView,
    readback: gpu::Buffer,
}

impl Target {
    fn new(device: &gpu::Device, extent: [u32; 2]) -> Self {
        let ty = (23.0_f32.to_radians()).tan();
        let tx = ty * (extent[0] as f32 / extent[1] as f32);
        let colour = texture(
            device,
            extent,
            SCENE_FORMAT,
            gpu::TextureUsages::RENDER_ATTACHMENT | gpu::TextureUsages::COPY_SRC,
        );
        Self {
            extent,
            tan_fov: [tx, ty],
            colour_view: colour.create_view(&gpu::TextureViewDescriptor::default()),
            colour,
            depth: texture(
                device,
                extent,
                DEPTH,
                gpu::TextureUsages::RENDER_ATTACHMENT | gpu::TextureUsages::TEXTURE_BINDING,
            )
            .create_view(&gpu::TextureViewDescriptor::default()),
            readback: device.create_buffer(&gpu::BufferDescriptor {
                label: None,
                size: u64::from(CROP * CROP * 8),
                mapped_at_creation: false,
                usage: gpu::BufferUsages::COPY_DST | gpu::BufferUsages::MAP_READ,
            }),
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "checked fixture centres are positive and bounded by the small target"
    )]
    fn flux(
        &self,
        device: &gpu::Device,
        queue: &gpu::Queue,
        mut encoder: gpu::CommandEncoder,
        centre: [f32; 2],
        edge: Option<Edge>,
    ) -> Footprint {
        assert!(
            centre.into_iter().zip(self.extent).all(|(centre, extent)| {
                (CROP as f32..(extent - CROP) as f32).contains(&centre)
            })
        );
        let [x, y] = centre.map(|value| value as u32 - CROP / 2);
        let mut source = self.colour.as_image_copy();
        source.origin = gpu::Origin3d { x, y, z: 0 };
        encoder.copy_texture_to_buffer(
            source,
            gpu::TexelCopyBufferInfo {
                buffer: &self.readback,
                layout: gpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(CROP * 8),
                    rows_per_image: Some(CROP),
                },
            },
            gpu::Extent3d {
                width: CROP,
                height: CROP,
                depth_or_array_layers: 1,
            },
        );
        let (send, receive) = std::sync::mpsc::channel();
        encoder.map_buffer_on_submit(&self.readback, gpu::MapMode::Read, .., move |result| {
            send.send(result).expect("reader");
        });
        let submission = queue.submit([encoder.finish()]);
        device
            .poll(gpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(10)),
            })
            .expect("Metal completion");
        receive
            .recv_timeout(Duration::from_secs(10))
            .expect("map callback")
            .expect("mapped pixels");
        let pixels = self
            .readback
            .slice(..)
            .get_mapped_range()
            .expect("HDR pixels");
        let mut flux = Footprint::default();
        for (dy, row) in (0..CROP).zip(pixels.as_chunks::<{ CROP as usize * 8 }>().0) {
            for (dx, pixel) in (0..CROP).zip(row.as_chunks::<8>().0) {
                let value = half(u16::from_le_bytes([pixel[0], pixel[1]]));
                let pixel = [x + dx, y + dy];
                let contribution = value / f64::from(STORAGE)
                    * spherical_pixel(self.extent, pixel, self.tan_fov.map(f64::from));
                flux.total += contribution;
                if edge.is_some_and(|edge| pixel[0] >= edge.corner[0] && pixel[1] >= edge.corner[1])
                {
                    flux.on_blocker += contribution;
                }
                for (moment, axis) in flux.moments.iter_mut().zip(pixel) {
                    *moment += contribution * (f64::from(axis) + 0.5);
                }
            }
        }
        drop(pixels);
        self.readback.unmap();
        flux
    }
}

fn texture(
    device: &gpu::Device,
    [width, height]: [u32; 2],
    format: gpu::TextureFormat,
    usage: gpu::TextureUsages,
) -> gpu::Texture {
    device.create_texture(&gpu::TextureDescriptor {
        label: None,
        size: gpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: gpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

fn uniform<const N: usize>(device: &gpu::Device, values: [f32; N]) -> gpu::Buffer {
    device.create_buffer_init(&gpu::util::BufferInitDescriptor {
        label: None,
        contents: &values
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
        usage: gpu::BufferUsages::UNIFORM,
    })
}

fn half(bits: u16) -> f64 {
    assert!(bits < 0x7c00, "nonfinite or negative HDR star value");
    let exponent = i32::from((bits >> 10) & 31);
    let fraction = f64::from(bits & 1023);
    if exponent == 0 {
        fraction * 2.0_f64.powi(-24)
    } else {
        (1.0 + fraction / 1024.0) * 2.0_f64.powi(exponent - 15)
    }
}

fn spherical_pixel(extent: [u32; 2], [x, y]: [u32; 2], tan_fov: [f64; 2]) -> f64 {
    let slope = |axis: usize, edge: u32| {
        (2.0 * f64::from(edge) / f64::from(extent[axis]) - 1.0) * tan_fov[axis]
    };
    let area = |i, j| {
        let u = slope(0, i);
        let v = slope(1, j);
        (u * v).atan2((1.0 + u * u + v * v).sqrt())
    };
    area(x + 1, y + 1) - area(x + 1, y) - area(x, y + 1) + area(x, y)
}

#[test]
fn production_star_pipeline_conserves_flux_and_masks_depth_edges() {
    pollster::block_on(async {
        let (device, queue, _) = metal().await;
        let probe = Probe::new(&device, &queue);
        let mut max_error = 0.0_f64;
        let mut min_ratio = f64::INFINITY;
        for extent in [[1280, 720], [1920, 1080], [3840, 2160]] {
            let target = Target::new(&device, extent);
            let [w, h] = extent.map(|value| value as f32);
            for [x, y] in [[0.5, 0.5], [0.9, 0.14]] {
                for [dx, dy] in [[0.05, 0.05], [0.5, 0.5], [0.93, 0.23]] {
                    let centre = [w * x + dx, h * y + dy];
                    let measured = [0, 1].map(|star| {
                        let footprint = probe.render(&device, &queue, &target, centre, star, None);
                        footprint.assert_centred(centre);
                        footprint.total
                    });
                    for (actual, expected) in measured.into_iter().zip(probe.flux) {
                        let error = (actual / f64::from(expected) - 1.0).abs();
                        max_error = max_error.max(error);
                        assert!(
                            error <= 0.05,
                            "{extent:?}, {centre:?}: {actual} vs {expected}"
                        );
                    }
                    let ratio = measured[0] / measured[1];
                    min_ratio = min_ratio.min(ratio);
                    assert!(ratio > 14.0, "three magnitudes keep their flux separation");
                }
            }
            for (corner, centre, vertical) in [
                ([extent[0] / 2, 0], [w * 0.5, h * 0.5], false),
                ([0, extent[1] / 7], [w * 0.9, (extent[1] / 7) as f32], true),
            ] {
                let [hidden, visible] = [0.25, -0.25].map(|offset| {
                    let mut position = centre;
                    position[usize::from(vertical)] += offset;
                    let unblocked = probe.render(
                        &device,
                        &queue,
                        &target,
                        position,
                        0,
                        Some(Edge {
                            corner,
                            opaque: false,
                        }),
                    );
                    unblocked.assert_centred(position);
                    let blocked = probe.render(
                        &device,
                        &queue,
                        &target,
                        position,
                        0,
                        Some(Edge {
                            corner,
                            opaque: true,
                        }),
                    );
                    assert_eq!(
                        blocked.on_blocker, 0.0,
                        "opaque samples cannot receive star flux"
                    );
                    (unblocked, blocked)
                });
                assert!(
                    hidden.0.total > 0.0,
                    "same hidden centre must draw without depth"
                );
                assert_eq!(hidden.1.total, 0.0, "covered centre must not leak into sky");
                let (full, clipped) = visible;
                assert!(
                    full.on_blocker > 0.05 * full.total,
                    "unblocked sprite must cross the edge"
                );
                let expected = full.total - full.on_blocker;
                assert!(
                    (clipped.total / expected - 1.0).abs() < 0.001,
                    "{extent:?}: visible {}, expected {expected}",
                    clipped.total
                );
            }
        }
        println!(
            "36 HDR footprints: max flux error {:.4}%, minimum magnitude ratio {min_ratio:.4}",
            100.0 * max_error
        );
    });
}

#[test]
fn resolved_night_footprints_keep_the_magnitude_order() {
    use super::{OffscreenTarget, PreparedSynthetic, SyntheticScene, atmosphere, capture};

    pollster::block_on(async {
        // Perpendicular lux = 2.54e-6 * 10^(-0.4 * magnitude).
        // Directions are symmetric about the camera aim.
        const RECORDS: [[f32; 4]; 2] = [
            [-0.039_968, 0.799_361, -0.599_521, 1.602_631_7e-7],
            [0.039_968, 0.799_361, -0.599_521, 1.011_592_2e-8],
        ];
        let (device, queue, capabilities) = metal().await;
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.star_catalogue = Some(&RECORDS);
        let lighting = atmosphere([0.0, -1.0, 0.0]);
        for [width, height] in [[1280, 720], [1920, 1080], [3840, 2160]] {
            let view = SyntheticView::perspective(
                [0.0, 0.0, 3.0],
                [0.0, 0.8, 2.4],
                [0.0, 1.0, 0.0],
                0.8,
                width as f32 / height as f32,
                0.1,
                20.0,
            )
            .expect("unoccluded sky camera")
            .with_incident_exposure(0.0, -12.0, 18.0)
            .expect("showcase meter")
            .with_low_light_adaptation()
            .expect("low-light meter")
            .with_sdr_tone_map(crate::synthetic::SdrToneMap::NeutralHighlights);
            scene.view = view;
            let mut prepared = PreparedSynthetic::prepare_with_capabilities(
                &capabilities,
                &queue,
                &scene,
                lighting,
                1.0,
            )
            .await
            .expect("night scene with both magnitudes");
            let mut target =
                OffscreenTarget::new(&device, &prepared, width, height).expect("night target");
            let baseline = capture(&device, &queue, &mut prepared, &target);
            scene.view = view
                .with_celestial_basis(Mat3::IDENTITY.to_cols_array_2d())
                .expect("control directions are already in world space");
            prepared
                .update_frame(&queue, &mut target, &scene, lighting, 1.0)
                .expect("show both stars at the same exposure");
            let with_stars = capture(&device, &queue, &mut prepared, &target);
            let (width, height) = (width as usize, height as usize);
            // A horizontal ray offset of 0.04 at 0.8 rad vertical FOV projects
            // to 0.0473 * height pixels. Keep each probe local to its star.
            let offset = 47 * height / 1000;
            let [bright, faint] = [width / 2 - offset, width / 2 + offset].map(|x| {
                let mut excess = (height / 2 - 4..=height / 2 + 4)
                    .flat_map(|y| (x - 4..=x + 4).map(move |x| (y * width + x) * 4))
                    .map(|pixel| with_stars[pixel + 1].saturating_sub(baseline[pixel + 1]))
                    .collect::<Vec<_>>();
                // Check the fourth-brightest sample, beyond a single pixel
                // or the one-code variation from display dither.
                *excess.select_nth_unstable_by(3, |a, b| b.cmp(a)).1
            });
            println!(
                "{width}x{height}: magnitude 3/6 fourth-brightest excess {bright}/{faint} SDR green codes"
            );
            assert!(
                faint >= 8,
                "magnitude 6 must resolve above the sky at {height}p"
            );
            assert!(
                bright >= faint + 8,
                "magnitude gap must resolve at {height}p"
            );
        }
    });
}
