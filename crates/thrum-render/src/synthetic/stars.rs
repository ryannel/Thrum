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
    LIGHT_BYTES, PreparedSynthetic, Projection, RenderError, SCENE_FORMAT, SceneContext,
    SyntheticView, exposure,
};

mod daylight;

pub(super) const PASS_NAME: &str = "atmosphere stars";
const UNIFORM_LEN: usize = 176;
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
    daylight: daylight::DaylightBound,
}

pub(super) struct StarTarget {
    uniform: wgpu::Buffer,
    groups: [wgpu::BindGroup; 2],
}

impl StarPrepared {
    pub(super) fn new(
        device: &crate::runtime::DeviceSite,
        records: &[[f32; 4]],
        max_storage_bytes: u64,
        light: &[u8; LIGHT_BYTES],
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
        // The same footprint constants drive rasterization and the daylight bound.
        let shader = format!(
            "const ANGULAR_SIGMA_DEGREES: f32 = {:.12};\nconst PIXEL_SIGMA: f32 = {:.12};\n\
             const MAX_CENTRE_NDC: f32 = {:.1};\nconst RADIUS_SIGMAS: f32 = {:.1};\n\
             const PIXEL_HALF_WIDTH: f32 = {:.1};\n{SHADER}",
            daylight::ANGULAR_SIGMA_DEGREES,
            daylight::PIXEL_SIGMA,
            daylight::MAX_CENTRE_NDC,
            daylight::RADIUS_SIGMAS,
            daylight::PIXEL_HALF_WIDTH,
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS_NAME),
            source: wgpu::ShaderSource::Wgsl(shader.into()),
        });
        let blend = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        let pipeline = device.render(&wgpu::RenderPipelineDescriptor {
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
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: SCENE_FORMAT,
                        blend: Some(wgpu::BlendState {
                            color: blend,
                            alpha: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::COLOR,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: super::MOTION_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
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
            daylight: daylight::DaylightBound::new(records, light),
        })
    }

    pub(super) fn target(
        &self,
        device: &wgpu::Device,
        depth: &wgpu::TextureView,
        atmosphere: &AtmospherePrepared,
        exposure_samples: &[wgpu::Buffer; 2],
        (view, extent): (&SyntheticView, [u32; 2]),
    ) -> (StarTarget, bool) {
        let view_bytes = self.view_bytes(view, view, extent);
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("star view"),
            contents: &view_bytes.unwrap_or([0; UNIFORM_LEN]),
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
        (StarTarget { uniform, groups }, view_bytes.is_some())
    }

    pub(super) fn update_target(
        &mut self,
        queue: &wgpu::Queue,
        target: &StarTarget,
        view: &SyntheticView,
        previous: &SyntheticView,
        extent: [u32; 2],
        light: &[u8; LIGHT_BYTES],
    ) -> bool {
        self.daylight.update(light);
        let Some(bytes) = self.view_bytes(view, previous, extent) else {
            return false;
        };
        queue.write_buffer(&target.uniform, 0, &bytes);
        true
    }

    fn view_bytes(
        &self,
        view: &SyntheticView,
        previous: &SyntheticView,
        extent: [u32; 2],
    ) -> Option<[u8; UNIFORM_LEN]> {
        let Projection::Perspective { fx, fy, .. } = view.projection() else {
            return None;
        };
        let rotation = view.celestial_basis()?;
        let strength = self.daylight.strength(view, extent);
        if strength <= 0.0 {
            return None;
        }
        let transform =
            Mat4::from_cols_array_2d(&view.view_proj_columns()) * Mat4::from_mat3(rotation);
        let prior = Mat4::from_cols_array_2d(&previous.view_proj_columns())
            * Mat4::from_mat3(previous.celestial_basis().unwrap_or(rotation));
        let jitter = view.pack_view(previous, 1.0);
        let up = rotation.row(1);
        let mut bytes = [0; UNIFORM_LEN];
        for (destination, value) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(
            transform
                .to_cols_array()
                .into_iter()
                .chain([extent[0] as f32, extent[1] as f32, fx.recip(), fy.recip()])
                .chain([up.x, up.y, up.z, strength])
                .chain((prior - transform).to_cols_array()),
        ) {
            destination.copy_from_slice(&value.to_le_bytes());
        }
        if view.raster_jitter_enabled() {
            bytes[160..168].copy_from_slice(&jitter[152..160]);
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
        (depth, scene, motion, exposure, transmittance, observer): (
            &ResourceId,
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
                (motion.clone(), 1, AccessKind::AttachmentLoad),
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
                [
                    Some(GraphColorAttachment {
                        resource: &self.target.ids.scene,
                        range: 0..1,
                        view: &self.target.scene_view,
                        clear: self.prepared.background,
                    }),
                    Some(GraphColorAttachment {
                        resource: &self.target.ids.motion,
                        range: 0..1,
                        view: &self.target.motion_view,
                        clear: wgpu::Color::BLACK,
                    }),
                ],
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
mod tests;
