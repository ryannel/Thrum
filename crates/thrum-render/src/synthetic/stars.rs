//! Prepared star catalogue and its post-opaque HDR composite.
use std::collections::BTreeSet;

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::atmosphere::AtmospherePrepared;
use crate::graph::{
    Access, AccessKind, CompiledPass, FrameGraph, GraphColorAttachment, Pass, PassScope,
    PipelineKey, ResourceDescriptor, ResourceId, ResourceScope,
};

use super::{
    PreparedSynthetic, Projection, RenderError, SCENE_FORMAT, SceneContext, SyntheticView, exposure,
};

pub(super) const PASS_NAME: &str = "atmosphere stars";
const UNIFORM_LEN: usize = 96;
const UNIFORM_BYTES: u64 = UNIFORM_LEN as u64;
const SHADER: &str = concat!(
    include_str!("../atmosphere/geometry.wgsl"),
    "\n",
    include_str!("stars.wgsl")
);

pub(super) struct StarPrepared {
    catalogue: wgpu::Buffer,
    sampler: wgpu::Sampler,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    count: u32,
}

pub(super) struct StarTarget {
    uniform: wgpu::Buffer,
    groups: [wgpu::BindGroup; 2],
}

impl StarPrepared {
    pub(super) fn new(
        device: &wgpu::Device,
        records: &[[f32; 4]],
        max_storage_bytes: u64,
    ) -> Result<Self, RenderError> {
        let count = u32::try_from(records.len()).map_err(|_| RenderError::SceneCapacity)?;
        let bytes = u64::from(count) * 16;
        if count == 0 || bytes > max_storage_bytes {
            return Err(RenderError::SceneCapacity);
        }
        let mut packed = Vec::with_capacity(records.len() * 16);
        for &[x, y, z, lux] in records {
            let norm = Vec3::new(x, y, z).length_squared();
            if !norm.is_finite() || (norm - 1.0).abs() > 1e-4 || !(0.0..=0.001).contains(&lux) {
                return Err(RenderError::InvalidLighting);
            }
            packed.extend([x, y, z, lux].into_iter().flat_map(f32::to_le_bytes));
        }
        let catalogue = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("equatorial star catalogue"),
            contents: &packed,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS_NAME),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let blend = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(PASS_NAME),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: SCENE_FORMAT,
                    blend: Some(wgpu::BlendState {
                        color: blend,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::COLOR,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let layout = pipeline.get_bind_group_layout(0);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("star transmittance linear clamp"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        Ok(Self {
            catalogue,
            sampler,
            layout,
            pipeline,
            count,
        })
    }

    pub(super) fn target(
        &self,
        device: &wgpu::Device,
        depth: &wgpu::TextureView,
        atmosphere: &AtmospherePrepared,
        exposure_samples: &[wgpu::Buffer; 2],
        (view, extent): (&SyntheticView, [u32; 2]),
    ) -> StarTarget {
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("star view"),
            contents: &StarTarget::view_bytes(view, extent).unwrap_or([0; UNIFORM_LEN]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let groups = std::array::from_fn(|side| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(PASS_NAME),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.catalogue.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(depth),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: exposure_samples[side].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(
                            atmosphere.transmittance_view(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: atmosphere.observer_uniform().as_entire_binding(),
                    },
                ],
            })
        });
        StarTarget { uniform, groups }
    }
}

impl StarTarget {
    pub(super) fn update(
        &self,
        queue: &wgpu::Queue,
        view: &SyntheticView,
        extent: [u32; 2],
    ) -> bool {
        let Some(bytes) = Self::view_bytes(view, extent) else {
            return false;
        };
        queue.write_buffer(&self.uniform, 0, &bytes);
        true
    }

    fn view_bytes(view: &SyntheticView, extent: [u32; 2]) -> Option<[u8; UNIFORM_LEN]> {
        let Projection::Perspective { fx, fy, .. } = view.projection() else {
            return None;
        };
        let rotation = view.celestial_basis()?;
        let transform =
            Mat4::from_cols_array_2d(&view.view_proj_columns()) * Mat4::from_mat3(rotation);
        let up = rotation.row(1);
        let mut bytes = [0; UNIFORM_LEN];
        for (destination, value) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(
            transform
                .to_cols_array()
                .into_iter()
                .chain([extent[0] as f32, extent[1] as f32, fx.recip(), fy.recip()])
                .chain([up.x, up.y, up.z, 0.0]),
        ) {
            destination.copy_from_slice(&value.to_le_bytes());
        }
        Some(bytes)
    }
}

impl PreparedSynthetic {
    pub(super) fn star_key(&self) -> PipelineKey {
        PipelineKey {
            role: 9,
            ..self.key
        }
    }

    pub(super) fn append_stars(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        opaque: usize,
        (depth, scene, exposure, transmittance, observer): (
            &ResourceId,
            &ResourceId,
            &ResourceId,
            &ResourceId,
            &ResourceId,
        ),
    ) -> Result<Option<usize>, RenderError> {
        let Some(stars) = &self.stars else {
            return Ok(None);
        };
        let records = u64::from(stars.count) * 16;
        let catalogue = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: records,
                usage: wgpu::BufferUsages::STORAGE,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        let view = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: UNIFORM_BYTES,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        let key = self.star_key();
        keys.insert(key);
        let pass = graph.pass(Pass {
            name: PASS_NAME.into(),
            scope: PassScope::View(1),
            pipeline: Some(key),
            accesses: [
                (catalogue, records, AccessKind::StorageRead),
                (view, UNIFORM_BYTES, AccessKind::UniformRead),
                (
                    exposure.clone(),
                    exposure::SAMPLE_BYTES,
                    AccessKind::UniformRead,
                ),
                (transmittance.clone(), 1, AccessKind::SampledRead),
                (observer.clone(), 80, AccessKind::UniformRead),
                (depth.clone(), 1, AccessKind::SampledRead),
                (scene.clone(), 1, AccessKind::AttachmentLoad),
            ]
            .into_iter()
            .map(|(resource, end, kind)| Access {
                resource,
                range: 0..end,
                kind,
                store: true,
            })
            .collect(),
            external_side_effect: false,
        });
        graph.dependency(opaque, pass)?;
        Ok(Some(pass))
    }
}

impl SceneContext<'_> {
    pub(super) fn stars(&mut self, compiled: &CompiledPass) -> Result<(), RenderError> {
        let (stars, target) = self
            .prepared
            .stars
            .as_ref()
            .zip(self.target.stars.as_ref())
            .ok_or(RenderError::GraphPassMismatch)?;
        let mut pass = compiled
            .begin_render(
                self.encoder,
                [Some(GraphColorAttachment {
                    resource: &self.target.ids.scene,
                    range: 0..1,
                    view: &self.target.scene_view,
                    clear: self.prepared.background,
                })],
                None,
                self.generation,
                self.timings.as_deref_mut(),
            )?
            .ok_or(RenderError::GraphPassMismatch)?;
        pass.set_pipeline(&stars.pipeline);
        pass.set_bind_group(0, &target.groups[self.exposure_side], &[]);
        pass.draw(0..6, 0..stars.count);
        Ok(())
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use crate::atmosphere::AtmosphereFrame;
    use crate::capabilities::DeviceCapabilities;
    use crate::synthetic::{FrameLighting, OffscreenTarget, SyntheticScene};

    #[test]
    fn opaque_depth_store_and_star_pass_follow_the_current_target_view() {
        pollster::block_on(async {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::METAL,
                ..wgpu::InstanceDescriptor::new_without_display_handle()
            });
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
                .expect("Metal adapter required for the star graph regression");
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor::default())
                .await
                .expect("Metal device");
            let capabilities = DeviceCapabilities::capture(&adapter, &device, 41);
            let lighting = FrameLighting::Atmosphere(
                AtmosphereFrame::new([0.0, -1.0, 0.0], 127_500.0, [1.0; 3], 2.0, 1)
                    .expect("Earth atmosphere"),
            );
            let mut scene = SyntheticScene::grid(1).expect("scene");
            scene.star_catalogue = Some(&[[0.0, 0.0, -1.0, 9.6e-6]]);
            let day = SyntheticView::perspective(
                [0.0, 0.0, 3.0],
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                1.0,
                1.0,
                0.1,
                20.0,
            )
            .expect("perspective view");
            let night = day
                .with_celestial_basis(glam::Mat3::IDENTITY.to_cols_array_2d())
                .expect("celestial basis");
            scene.view = day;
            let mut prepared = PreparedSynthetic::prepare_with_capabilities(
                &capabilities,
                &queue,
                &scene,
                lighting,
                1.0,
            )
            .await
            .expect("star scene preparation");
            let mut target = OffscreenTarget::new(&device, &prepared, 65, 33).expect("target");
            assert_variant(&target, false);
            for (view, stars) in [(night, true), (day, false)] {
                scene.view = view;
                prepared
                    .update_frame(&queue, &mut target, &scene, lighting, 1.0)
                    .expect("update existing target");
                assert_variant(&target, stars);
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                prepared
                    .encode(&mut encoder, 41, &target, None)
                    .expect("execute selected graph");
                let submission = queue.submit([encoder.finish()]);
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: Some(submission),
                        timeout: Some(std::time::Duration::from_secs(10)),
                    })
                    .expect("submitted variant completes");
                prepared.commit_frame();
            }
        });
    }

    fn assert_variant(target: &OffscreenTarget, stars: bool) {
        for capture in [false, true] {
            let plan = target.graph(capture).plan();
            let passes = plan
                .pass_bindings()
                .map(|(_, pass)| pass)
                .collect::<Vec<_>>();
            let forward = passes
                .iter()
                .position(|pass| pass.name == "synthetic forward opaque")
                .expect("forward pass");
            let tone = passes
                .iter()
                .position(|pass| pass.name == "synthetic tone encode")
                .expect("display pass");
            let star = passes.iter().position(|pass| pass.name == PASS_NAME);
            assert_eq!(star.is_some(), stars);
            if let Some(star) = star {
                assert!(forward < star && star < tone);
            }
            let depth = passes[forward]
                .attachment_ops(&target.ids.depth, 0..1, 0.0)
                .expect("forward depth");
            assert_eq!(
                depth.store,
                if stars {
                    wgpu::StoreOp::Store
                } else {
                    wgpu::StoreOp::Discard
                }
            );
            assert_eq!(
                passes.last().expect("last pass").name,
                if capture {
                    "output readback"
                } else {
                    "synthetic tone encode"
                }
            );
        }
    }
}
