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
pub(crate) mod prepare;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod resize;
mod temporal;

pub(crate) const TRACE: &str = "cloud trace";
pub(crate) const COMPOSE: &str = "cloud compose";
pub(crate) const FIELD: &str = "cloud internal field";
pub(crate) const MAX_TRACE_SAMPLES: u32 = 768;
pub(crate) use overlay::DEFAULT_OVERLAY_SLICES;
const TRACE_TEXTURE_OWNER: u64 = 100;
const HISTORY_TEXTURE_OWNER: u64 = 103;

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
/// Ground allocates four array layers for the four pixels of a trace texel in row-major order.
/// The single-stop path uses only layer zero. In per-pixel mode, metadata alpha is each stop's
/// surface distance in kilometres, and the fresh sample count applies to the complete texel ray.
#[derive(Clone)]
pub struct SkyProducts {
    /// Resolved RGB in-scatter in scene storage units and transmittance in alpha.
    pub transport: wgpu::Texture,
    /// Current trace RGB in-scatter and transmittance, before history resolve.
    pub fresh_transport: wgpu::Texture,
    /// Resolved front distance in kilometres, opacity and reactive coverage. In Ground,
    /// alpha holds the surface stop distance in kilometres or zero for sky.
    pub front: wgpu::Texture,
    /// Latest front distance, opacity and sample count. Alpha is the Ground stop distance
    /// in kilometres (zero for sky), or the Sky trace flag.
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
    /// Width and height of the last submitted trace rectangle in texels.
    pub active_extent: [u32; 2],
    /// Number of submitted non-capture layer frames since preparation.
    pub submitted_frames: u32,
    /// Jittered projection used by that submission's trace and resolve.
    pub raster_view_projection: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Uniform {
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
    density_profile: [f32; 2],
    per_pixel_stops: u32,
    pad: u32,
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
    active_extent: [u32; 2],
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
    #[cfg(not(target_arch = "wasm32"))]
    resize_bindings: resize::Bindings,
    compose_group: wgpu::BindGroup,
    field_group: [wgpu::BindGroup; 2],
    has_placed_map: bool,
    per_pixel_stops: bool,
    profile_extent: [u32; 2],
    follows_extent: bool,
    render_extent: [u32; 2],
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
                    owner: TRACE_TEXTURE_OWNER + i as u64,
                    epoch,
                    initialized: false,
                },
            ));
        }
        let stop_layers = u64::from(self.textures[0].depth_or_array_layers());
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
        let stop_access = |resource, kind| EffectAccess {
            range: 0..stop_layers,
            ..access(resource, kind)
        };
        reads.extend((first..first + 2).map(|i| stop_access(i, AccessKind::StorageWrite)));
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
        for (i, texture) in self.temporal.targets.textures[0].iter().enumerate() {
            graph.resources.push((
                ResourceDescriptor::of_texture(texture),
                ResourceScope::History {
                    owner: HISTORY_TEXTURE_OWNER + i as u64,
                    epoch,
                    initialized: true,
                },
            ));
        }
        let mut resolve_reads: Vec<_> = (first..first + 2)
            .map(|i| stop_access(i, AccessKind::SampledRead))
            .collect();
        resolve_reads.push(buffer_access(
            params,
            self.params.size(),
            AccessKind::UniformRead,
        ));
        for resource in history..history + 2 {
            let mut previous = stop_access(resource, AccessKind::SampledRead);
            previous.history = Some(HistoryBinding::Previous);
            let mut next = stop_access(resource, AccessKind::StorageWrite);
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
                .map(|i| stop_access(i, AccessKind::SampledRead))
                .chain(std::iter::once(buffer_access(
                    params,
                    self.params.size(),
                    AccessKind::UniformRead,
                )))
                .chain((history..history + 2).map(|i| {
                    let mut read = stop_access(i, AccessKind::SampledRead);
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
        let per_pixel_stops = self.per_pixel_stops;
        self.render_extent = frame.render_extent;
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
        // Without scene motion as a trace input, the current depth is the
        // only verified surface. Use the design's full-trace fallback.
        let full_trace = per_pixel_stops
            || reset
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
            active_extent,
        });
        let mut heights = if per_pixel_stops && parameters.coverage == 0.0 && shapes.is_none() {
            [0.0; 2]
        } else {
            weather.height_range()
        };
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
            ],
            per_pixel_stops: u32::from(per_pixel_stops),
            pad: 0,
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

    pub(crate) fn invalidate_ground(&mut self) {
        if let Some(overlay) = &mut self.overlay {
            overlay.invalidate_ground();
        }
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
        let dispatch = [width, height].map(|side| side.div_ceil(8 * self.trace_stride));
        pass.dispatch_workgroups(dispatch[0], dispatch[1], 1);
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
            transport: self.temporal.targets.textures[side][0].clone(),
            fresh_transport: self.textures[0].clone(),
            front: self.temporal.targets.textures[side][1].clone(),
            fresh_front: self.textures[1].clone(),
            phase_tables: self.lighting.phase.clone(),
            hemisphere_light: self.lighting.ambient.clone(),
            history_valid: self.committed.is_some_and(|s| s.history_valid),
            refresh_stride: self.quality.stride,
            trace_stride: self.committed.map_or(1, |s| s.trace_stride),
            active_extent: self.committed.map_or([0; 2], |s| s.active_extent),
            submitted_frames: self.ordinal,
            raster_view_projection: self.committed.map_or([[0.0; 4]; 4], |s| s.raster),
        }
    }

    pub(crate) fn resource_bytes(&self) -> Vec<(String, u64)> {
        let mut resources = target_bytes(&self.textures, &self.temporal.targets.textures);
        resources.extend(self.fixed_resource_bytes());
        resources
    }

    pub(crate) fn fixed_resource_bytes(&self) -> Vec<(String, u64)> {
        let mut resources = vec![
            ("cloud detail noise".into(), self.detail_noise.bytes()),
            ("cloud layer parameters".into(), self.params.size()),
            ("cloud phase tables".into(), self.lighting.phase.size()),
            (
                "cloud hemisphere light".into(),
                self.lighting.ambient.size(),
            ),
        ];
        if let Some(overlay) = &self.overlay {
            resources.push((
                "cloud sky overlay".into(),
                u64::from(overlay.texture.width()) * u64::from(overlay.texture.height()) * 48,
            ));
            resources.push(("cloud overlay rows".into(), 16));
        }
        resources
    }

    pub(crate) fn has_render_capacity(&self, extent: [u32; 2]) -> bool {
        let texture = &self.textures[0];
        extent[0].div_ceil(2) <= texture.width() && extent[1].div_ceil(2) <= texture.height()
    }

    fn active_extent(&self) -> [u32; 2] {
        if self.per_pixel_stops {
            return self.render_extent.map(|side| side.div_ceil(2));
        }
        let extent = if self.follows_extent {
            self.render_extent
        } else {
            self.profile_extent
        };
        extent.map(|side| (side.div_ceil(2) * self.quality.layer_percent).div_ceil(100))
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

fn target_bytes(
    fresh: &[wgpu::Texture; 2],
    history: &[[wgpu::Texture; 2]; 2],
) -> Vec<(String, u64)> {
    fresh
        .iter()
        .chain(history.iter().flatten())
        .enumerate()
        .map(|(index, texture)| {
            (
                format!("cloud layer texture {index}"),
                u64::from(texture.width())
                    * u64::from(texture.height())
                    * u64::from(texture.depth_or_array_layers())
                    * 8,
            )
        })
        .collect()
}

pub(crate) fn parameter_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("cloud layer parameters"),
        size: size_of::<Uniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn stop_view(texture: &wgpu::Texture) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    })
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
