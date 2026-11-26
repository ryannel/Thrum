//! Visible transport, recorded after the scene's opaque and celestial passes.
use crate::noise::{DETAIL_PERIOD, Noise, SHAPE_PERIOD};
use crate::{Error, WeatherMap};
use glam::{Mat4, Vec2, Vec3};
use thrum_cloud_model::State;
use thrum_render::{
    atmosphere::EnvironmentProducts,
    contribution::{
        AfterOpaqueKind, AfterOpaquePass, ContributionFrame, EffectAccess, EffectCadence,
        EffectGraph, EffectStep, HistoryBinding, SceneSurface,
    },
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
};

#[cfg(test)]
pub(crate) mod diagnostic;
pub(crate) mod light;
mod overlay;
mod prepare;
mod temporal;

pub(crate) const TRACE: &str = "cloud trace";
pub(crate) const COMPOSE: &str = "cloud compose";
pub(crate) const FIELD: &str = "cloud internal field";
pub(crate) const MAX_TRACE_SAMPLES: u32 = 768;
pub(crate) use overlay::DEFAULT_OVERLAY_SLICES;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Quality {
    pub stride: u32,
    pub light_samples: u32,
    pub layer_percent: u32,
    pub step: f32,
    pub cap: u32,
}

/// Resolution and march controls, independent of the authoritative weather.
#[derive(Clone, Copy, Debug)]
pub struct SkyProfile {
    /// Render extent; the layer never scales to the display extent.
    pub extent: [u32; 2],
    /// Metres between near samples, increasing with ray footprint.
    pub step: f32,
    /// Number of local full-density Sun samples, from zero to three.
    pub light_samples: u32,
    /// Trace one texel per square block; 1, 2 or 4. Resets and frames where
    /// the sparse samples cannot carry the contour trace every texel.
    /// One sparse refresh cycle is this stride squared, in submitted frames.
    pub refresh_stride: u32,
}

/// Read-only handles for optional inspection of the last submitted non-capture Sky layer.
/// The caller waits for GPU completion before readback; these never drive rendering.
#[derive(Clone)]
pub struct SkyProducts {
    /// Resolved RGB in-scatter in scene storage units and transmittance in alpha.
    pub transport: wgpu::Texture,
    /// Current trace RGB in-scatter and transmittance, before history resolve.
    pub fresh_transport: wgpu::Texture,
    /// Resolved front distance in kilometres, opacity and reactive coverage.
    pub front: wgpu::Texture,
    /// Latest traced distance, opacity, sample count and traced flag.
    pub fresh_front: wgpu::Texture,
    /// Prepared droplet phase tables, unchanged from the CPU model.
    pub phase_tables: wgpu::Buffer,
    /// Two cloud-free hemisphere source radiances, E / (2π).
    pub hemisphere_light: wgpu::Buffer,
    /// Whether this submission accepted the layer's previous history.
    pub history_valid: bool,
    /// Layer texels per side of a refresh block.
    pub refresh_stride: u32,
    /// Layer texels per side actually used by the last submitted trace.
    pub trace_stride: u32,
    /// Number of submitted non-capture layer frames since preparation.
    pub submitted_frames: u32,
    /// Jittered projection used by that submission's trace and resolve.
    pub raster_view_projection: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    weather: [[f32; 4]; 4],
    physical: [f32; 4],
    erosion: [f32; 4],
    scattering: [f32; 4],
    drift: [f32; 4],
    heights: [f32; 4],
    light_samples: u32,
    sun_angular_radius: f32,
    render_extent: [u32; 2],
    temporal: [u32; 4],
    noise: [f32; 4],
    bank_noise: [f32; 4],
    density_profile: [f32; 4],
    active_extent: [u32; 2],
    sample_cap: u32,
    field_overlay: u32,
    inverse_raster: [[f32; 4]; 4],
    previous_raster: [[f32; 4]; 4],
}

#[derive(Clone, Copy)]
struct LayerSample {
    travel: [f64; 2],
    coverage_and_kind: [u32; 2],
    environment: thrum_render::atmosphere::EnvironmentEpoch,
    history_valid: bool,
    raster: [[f32; 4]; 4],
    trace_stride: u32,
}

pub(crate) struct Layer {
    quality: Quality,
    environment: EnvironmentProducts,
    lighting: light::Lighting,
    temporal: temporal::Temporal,
    committed: Option<LayerSample>,
    pending: Option<LayerSample>,
    ordinal: u32,
    trace_stride: u32,
    params: wgpu::Buffer,
    queue: wgpu::Queue,
    textures: [wgpu::Texture; 2],
    detail_noise: Noise,
    trace: wgpu::ComputePipeline,
    compose: wgpu::RenderPipeline,
    field_pipeline: wgpu::RenderPipeline,
    field_overlay: crate::FieldOverlay,
    trace_group: wgpu::BindGroup,
    compose_group: wgpu::BindGroup,
    field_group: [wgpu::BindGroup; 2],
    has_placed_map: bool,
    overlay: Option<overlay::Overlay>,
}

impl Layer {
    pub(crate) fn inherit_history(&mut self, accepted: &mut Self) {
        std::mem::swap(&mut self.temporal.clock, &mut accepted.temporal.clock);
    }

    pub(crate) fn append_graph(
        &self,
        graph: &mut EffectGraph,
        epoch: u64,
        shape_noise: EffectAccess,
    ) {
        let first = graph.resources.len();
        for (i, texture) in self.textures.iter().enumerate() {
            graph.resources.push((
                ResourceDescriptor::of_texture(texture),
                ResourceScope::Persistent {
                    owner: 100 + i as u64,
                    epoch,
                    initialized: false,
                },
            ));
        }
        let access = |resource, kind| EffectAccess {
            resource,
            range: 0..1,
            kind,
            history: None,
            view: None,
        };
        let mut reads = vec![
            access(0, AccessKind::SampledRead),
            access(1, AccessKind::SampledRead),
        ];
        reads[0].range = 0..4;
        reads[1].range = 0..4;
        for (resource, entry) in graph.resources.iter().enumerate().take(6).skip(3) {
            let mut read = access(resource, AccessKind::SampledRead);
            if let ResourceDescriptor::Texture { layers, .. } = entry.0 {
                read.range = 0..u64::from(layers);
            }
            reads.push(read);
        }
        reads.push(EffectAccess {
            resource: 7,
            range: 0..96,
            kind: AccessKind::UniformRead,
            history: None,
            view: None,
        });
        reads.extend((first..first + 2).map(|i| access(i, AccessKind::StorageWrite)));
        let detail = graph.resources.len();
        graph.resources.push(self.detail_noise.resource());
        reads.extend([shape_noise, self.detail_noise.read(detail)]);
        let phase = graph.resources.len();
        graph.resources.push((
            ResourceDescriptor::Buffer {
                bytes: self.lighting.phase.size(),
                usage: self.lighting.phase.usage(),
            },
            ResourceScope::Imported { initialized: true },
        ));
        let ambient = graph.resources.len();
        graph.resources.push((
            ResourceDescriptor::Buffer {
                bytes: self.lighting.ambient.size(),
                usage: self.lighting.ambient.usage(),
            },
            ResourceScope::Persistent {
                owner: 102,
                epoch,
                initialized: self.overlay.is_some(),
            },
        ));
        let params = graph.resources.len();
        graph.resources.push((
            ResourceDescriptor::Buffer {
                bytes: self.params.size(),
                usage: self.params.usage(),
            },
            ResourceScope::Imported { initialized: true },
        ));
        let buffer_access = |resource, bytes, kind| EffectAccess {
            resource,
            range: 0..bytes,
            kind,
            history: None,
            view: None,
        };
        let mut hemisphere = declaration(
            light::PASS,
            AfterOpaqueKind::Compute,
            vec![buffer_access(
                ambient,
                self.lighting.ambient.size(),
                AccessKind::StorageWrite,
            )],
        );
        hemisphere.depth = false;
        if self.overlay.is_none() {
            graph.after_opaque.push(hemisphere);
        }
        reads.push(buffer_access(
            phase,
            self.lighting.phase.size(),
            AccessKind::StorageRead,
        ));
        reads.push(buffer_access(
            ambient,
            self.lighting.ambient.size(),
            AccessKind::StorageRead,
        ));
        reads.push(buffer_access(
            params,
            self.params.size(),
            AccessKind::UniformRead,
        ));
        if let Some(overlay) = &self.overlay {
            let overlay_reads = reads
                .iter()
                .filter(|read| read.kind != AccessKind::StorageWrite && read.resource != detail)
                .cloned()
                .map(|mut read| {
                    if read.resource == ambient {
                        read.kind = AccessKind::StorageReadWrite;
                    }
                    read
                })
                .collect();
            overlay.append_graph(graph, epoch, overlay_reads);
        }
        let mut trace = declaration(TRACE, AfterOpaqueKind::Compute, reads);
        trace.cadence = EffectCadence::TimeSliced;
        graph.after_opaque.push(trace);
        let history = graph.resources.len();
        for (i, texture) in self.temporal.textures[0].iter().enumerate() {
            graph.resources.push((
                ResourceDescriptor::of_texture(texture),
                ResourceScope::History {
                    owner: 103 + i as u64,
                    epoch,
                    initialized: true,
                },
            ));
        }
        let mut resolve_reads: Vec<_> = (first..first + 2)
            .map(|i| access(i, AccessKind::SampledRead))
            .collect();
        resolve_reads.push(buffer_access(
            params,
            self.params.size(),
            AccessKind::UniformRead,
        ));
        for resource in history..history + 2 {
            let mut previous = access(resource, AccessKind::SampledRead);
            previous.history = Some(HistoryBinding::Previous);
            let mut next = access(resource, AccessKind::StorageWrite);
            next.history = Some(HistoryBinding::Next);
            resolve_reads.extend([previous, next]);
        }
        let mut resolve = declaration(temporal::PASS, AfterOpaqueKind::Compute, resolve_reads);
        resolve.cadence = EffectCadence::Skippable;
        graph.after_opaque.push(resolve);
        graph.after_opaque.push(declaration(
            COMPOSE,
            AfterOpaqueKind::Render,
            (first..first + 2)
                .map(|i| access(i, AccessKind::SampledRead))
                .chain(std::iter::once(buffer_access(
                    params,
                    self.params.size(),
                    AccessKind::UniformRead,
                )))
                .chain((history..history + 2).map(|i| {
                    let mut read = access(i, AccessKind::SampledRead);
                    read.history = Some(HistoryBinding::Result);
                    read
                }))
                .collect(),
        ));
        let mut refresh_age = access(history + 1, AccessKind::SampledRead);
        refresh_age.history = Some(HistoryBinding::Result);
        graph.after_opaque.push(declaration(
            FIELD,
            AfterOpaqueKind::PreTone,
            [0, 1]
                .map(|resource| EffectAccess {
                    range: 0..4,
                    ..access(resource, AccessKind::SampledRead)
                })
                .into_iter()
                .chain([
                    access(6, AccessKind::SampledRead),
                    access(first + 1, AccessKind::SampledRead),
                    refresh_age,
                    buffer_access(params, self.params.size(), AccessKind::UniformRead),
                ])
                .collect(),
        ));
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "physical data narrows relative to the cloud anchor"
    )]
    pub(crate) fn upload(
        &mut self,
        weather: &WeatherMap,
        state: &State,
        frame: &ContributionFrame,
        shapes: Option<&crate::shapes::Shapes>,
        field_overlay: crate::FieldOverlay,
    ) -> Result<(), Error> {
        let environment = frame.environment.ok_or(Error::Unsupported(
            "cloud light requires an environment frame",
        ))?;
        self.lighting
            .upload(&self.queue, environment.true_sun.angular_radius_radians)?;
        let (model, kind) = weather.density_data(state);
        let travel = state.display_travel(
            f64::from(frame.time.display_phase),
            frame.time.tick_duration.as_secs_f64(),
            frame.history_reset,
        );
        let step = self.temporal.clock.begin(frame.time, u64::MAX, [0; 2]);
        let reset = frame.history_reset
            || step == EffectStep::Reset
            || shapes.is_some_and(crate::shapes::Shapes::history_invalid)
            || self
                .committed
                .is_none_or(|old| old.environment != environment.epoch);
        let previous = self.committed.map_or(travel, |old| old.travel);
        let previous_raster = self.committed.map_or(frame.view_proj, |old| old.raster);
        let inverse_raster = Mat4::from_cols_array_2d(&frame.view_proj).inverse();
        // Preserve the air lattice at long travel without subtracting large
        // rounded floats in the shader. Noise keeps the anchor's wrapped phase.
        let anchor = std::array::from_fn::<_, 2, _>(|i| {
            ((f64::from(frame.camera[[0, 2][i]]) - travel[i]) / 200.0).floor() * 200.0
        });
        let stride = self.quality.stride;
        let active_extent = self.active_extent();
        let parameters = state.parameters();
        let coverage_and_kind =
            [parameters.coverage, parameters.kind].map(|value| (value as f32).to_bits());
        let unreliable_sparse_trace = stride > 1
            && self.committed.is_some_and(|old| {
                // A coverage or type ramp can change optical depth across the
                // whole layer independently of advection.
                old.coverage_and_kind != coverage_and_kind
                    || view_crosses_refresh_block(
                        inverse_raster,
                        Mat4::from_cols_array_2d(&old.raster),
                        Vec3::from_array(frame.camera),
                        active_extent,
                        stride,
                    )
            });
        let full_trace = reset
            || frame.time.capture
            || active_extent.iter().any(|&side| side < stride)
            || unreliable_sparse_trace;
        self.trace_stride = if full_trace { 1 } else { stride };
        self.pending = (!frame.time.capture).then_some(LayerSample {
            travel,
            coverage_and_kind,
            environment: environment.epoch,
            history_valid: !reset,
            raster: frame.view_proj,
            trace_stride: self.trace_stride,
        });
        let mut heights = weather.height_range();
        if let Some(shapes) = shapes {
            heights = shapes.heights(state, kind, model.pitch, heights);
        }
        let (phase, scattering) = self
            .lighting
            .type_parameters(parameters.kind, (1.0 - kind.deck) * 0.33);
        let uniform = Uniform {
            weather: std::array::from_fn(|i| {
                [
                    (weather.origins()[i][0] - anchor[0]) as f32,
                    (weather.origins()[i][1] - anchor[1]) as f32,
                    weather.profile().texels[i] as f32,
                    weather.profile().size as f32,
                ]
            }),
            physical: [
                kind.extinction() as f32,
                model.erosion as f32,
                kind.deck as f32,
                kind.droplet_radius as f32,
            ],
            erosion: phase,
            scattering,
            drift: [
                (travel[0] + anchor[0]) as f32,
                (travel[1] + anchor[1]) as f32,
                (travel[0] - previous[0]) as f32,
                (travel[1] - previous[1]) as f32,
            ],
            heights: [
                heights[0] as f32,
                heights[1] as f32,
                self.quality.step,
                200_000.0,
            ],
            light_samples: self.quality.light_samples,
            sun_angular_radius: environment.true_sun.angular_radius_radians,
            render_extent: frame.render_extent,
            temporal: [
                u32::from(!reset && !frame.time.capture),
                self.ordinal % (stride * stride),
                stride,
                u32::from(full_trace),
            ],
            noise: [
                (anchor[0].rem_euclid(SHAPE_PERIOD) / SHAPE_PERIOD) as f32,
                (anchor[1].rem_euclid(SHAPE_PERIOD) / SHAPE_PERIOD) as f32,
                (anchor[0].rem_euclid(DETAIL_PERIOD) / DETAIL_PERIOD) as f32,
                (anchor[1].rem_euclid(DETAIL_PERIOD) / DETAIL_PERIOD) as f32,
            ],
            // This octave needs its own wrapped phase when the weather
            // anchor moves. The shape-noise phase uses a different period.
            bank_noise: crate::noise::bank_noise(anchor, kind.deck),
            density_profile: [
                model.detail_mean as f32,
                if self.has_placed_map { 1.0 } else { 0.0 },
                0.0,
                0.0,
            ],
            active_extent,
            sample_cap: self.quality.cap,
            field_overlay: field_overlay as u32,
            inverse_raster: inverse_raster.to_cols_array_2d(),
            previous_raster,
        };
        self.queue
            .write_buffer(&self.params, 0, bytemuck::bytes_of(&uniform));
        self.field_overlay = field_overlay;
        if let Some(overlay) = &mut self.overlay {
            overlay.upload(&self.queue, frame, environment, reset);
        }
        Ok(())
    }

    pub(crate) fn encode_overlay(
        &self,
        pass: &mut wgpu::ComputePass<'_>,
        shapes: Option<&crate::shapes::Shapes>,
    ) {
        if let Some(overlay) = &self.overlay {
            if overlay.ambient_due() {
                self.lighting.encode(pass, &self.environment, 0);
            }
            overlay.encode(pass, &self.environment, &self.trace_group, shapes);
        }
    }

    pub(crate) fn overlay_submitted(&mut self) {
        if let Some(overlay) = &mut self.overlay {
            overlay.submitted();
        }
    }

    pub(crate) fn overlay_runs(&self) -> bool {
        self.overlay.as_ref().is_some_and(overlay::Overlay::runs)
    }

    pub(crate) fn has_overlay(&self) -> bool {
        self.overlay.is_some()
    }

    pub(crate) fn encode(
        &mut self,
        index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &ContributionFrame,
        shapes: Option<&crate::shapes::Shapes>,
    ) {
        if index == 0 {
            self.lighting
                .encode(pass, &self.environment, frame.exposure_side);
            return;
        }
        if index == 2 {
            self.temporal.encode(pass, self.active_extent());
            return;
        }
        pass.set_pipeline(&self.trace);
        pass.set_bind_group(0, self.environment.group(frame.exposure_side), &[]);
        pass.set_bind_group(2, &self.trace_group, &[]);
        if let Some(shapes) = shapes {
            pass.set_bind_group(3, &shapes.group, &[]);
        }
        let [width, height] = self.active_extent();
        pass.dispatch_workgroups(
            width.div_ceil(8 * self.trace_stride),
            height.div_ceil(8 * self.trace_stride),
            1,
        );
    }

    pub(crate) fn field_runs(&self) -> bool {
        self.field_overlay != crate::FieldOverlay::Off
    }

    pub(crate) fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &ContributionFrame,
        field: bool,
    ) {
        if field {
            pass.set_pipeline(&self.field_pipeline);
            pass.set_bind_group(3, &self.field_group[frame.exposure_side], &[]);
        } else {
            pass.set_pipeline(&self.compose);
        }
        pass.set_bind_group(
            2,
            if frame.time.capture {
                &self.compose_group
            } else {
                self.temporal.result_group()
            },
            &[],
        );
        pass.draw(0..3, 0..1);
    }

    pub(crate) fn products(&self) -> SkyProducts {
        let side = self.temporal.clock.side(0, HistoryBinding::Previous);
        SkyProducts {
            transport: self.temporal.textures[side][0].clone(),
            fresh_transport: self.textures[0].clone(),
            front: self.temporal.textures[side][1].clone(),
            fresh_front: self.textures[1].clone(),
            phase_tables: self.lighting.phase.clone(),
            hemisphere_light: self.lighting.ambient.clone(),
            history_valid: self.committed.is_some_and(|s| s.history_valid),
            refresh_stride: self.quality.stride,
            trace_stride: self.committed.map_or(1, |s| s.trace_stride),
            submitted_frames: self.ordinal,
            raster_view_projection: self.committed.map_or([[0.0; 4]; 4], |s| s.raster),
        }
    }

    pub(crate) fn resource_bytes(&self) -> Vec<(String, u64)> {
        let mut resources = Vec::new();
        for (i, texture) in self
            .textures
            .iter()
            .chain(self.temporal.textures.iter().flatten())
            .enumerate()
        {
            resources.push((
                format!("cloud layer texture {i}"),
                u64::from(texture.width()) * u64::from(texture.height()) * 8,
            ));
        }
        resources.extend([
            ("cloud detail noise".into(), self.detail_noise.bytes()),
            ("cloud layer parameters".into(), self.params.size()),
            ("cloud phase tables".into(), self.lighting.phase.size()),
            (
                "cloud hemisphere light".into(),
                self.lighting.ambient.size(),
            ),
        ]);
        if let Some(overlay) = &self.overlay {
            resources.push((
                "cloud sky overlay".into(),
                u64::from(overlay.texture.width()) * u64::from(overlay.texture.height()) * 48,
            ));
            resources.push(("cloud overlay rows".into(), 16));
        }
        resources
    }

    fn active_extent(&self) -> [u32; 2] {
        [self.textures[0].width(), self.textures[0].height()]
            .map(|side| (side * self.quality.layer_percent).div_ceil(100))
    }

    pub(crate) fn set_quality(&mut self, quality: Quality) {
        if self.quality != quality {
            self.quality = quality;
            self.invalidate();
        }
    }

    pub(crate) fn invalidate(&mut self) {
        self.temporal.clock.invalidate();
        self.committed = None;
    }

    pub(crate) fn set_overlay_cadence(&mut self, slices: u32) {
        if let Some(overlay) = &mut self.overlay {
            overlay.set_cadence(slices);
        }
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "layer dimensions and the refresh stride are representable in f32"
)]
fn view_crosses_refresh_block(
    inverse_raster: Mat4,
    previous_raster: Mat4,
    camera: Vec3,
    extent: [u32; 2],
    stride: u32,
) -> bool {
    let half_extent = Vec2::from_array(extent.map(|side| side as f32)) * 0.5;
    let block = stride as f32;
    // The corners bound the turn's screen-space displacement. Directions
    // isolate rotation and projection from translation, which cloud history
    // already reprojects through its front. Subtexel raster jitter fits inside
    // a sparse block.
    [
        Vec2::new(-1.0, -1.0),
        Vec2::new(1.0, -1.0),
        Vec2::new(-1.0, 1.0),
        Vec2::new(1.0, 1.0),
    ]
    .into_iter()
    .any(|corner| {
        let world = inverse_raster * corner.extend(0.0).extend(1.0);
        let ray = world.truncate() - camera * world.w;
        let old = previous_raster * ray.extend(0.0);
        old.w <= 0.0
            || ((old.truncate().truncate() / old.w - corner) * half_extent).length_squared()
                > block * block
    })
}

fn declaration(name: &str, kind: AfterOpaqueKind, accesses: Vec<EffectAccess>) -> AfterOpaquePass {
    AfterOpaquePass {
        name: name.into(),
        kind,
        cadence: EffectCadence::EveryFrame,
        accesses,
        depth: kind != AfterOpaqueKind::PreTone,
        write_depth: false,
        refraction: false,
        images: thrum_render::contribution::AfterOpaqueImages::None,
    }
}

#[cfg(test)]
mod tests;

pub(crate) const SCATTERING: &str = include_str!("layer/scattering.wgsl");

pub(crate) fn sun_source(group: u32) -> String {
    format!(
        "@group({group}) @binding(5) var sun_near: texture_2d_array<f32>;
         @group({group}) @binding(6) var sun_middle: texture_2d_array<f32>;
         @group({group}) @binding(7) var sun_far: texture_2d_array<f32>;
         struct CloudMapping {{
             region: vec4<f32>, plane_drift: vec4<f32>, path_border: vec4<f32>,
             far_value: vec4<f32>, near: vec4<f32>, middle: vec4<f32>,
         }}
         @group({group}) @binding(8) var<uniform> mapping: CloudMapping;
         @group({group}) @binding(13) var noise_sampler: sampler;
         {}\n{}",
        include_str!("layer/sun.wgsl"),
        include_str!("layer/sun_windows.wgsl"),
    )
}
