//! Visible transport, recorded after the scene's opaque and celestial passes.
use crate::noise::{DETAIL_PERIOD, Noise, SHAPE_PERIOD};
use crate::{Error, WeatherMap};
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
mod light;
mod prepare;
mod temporal;

pub(crate) const TRACE: &str = "cloud trace";
pub(crate) const COMPOSE: &str = "cloud compose";

/// Resolution and march controls, independent of the authoritative weather.
#[derive(Clone, Copy, Debug)]
pub struct SkyProfile {
    /// Render extent; the layer never scales to the display extent.
    pub extent: [u32; 2],
    /// Metres between near samples, increasing with ray footprint.
    pub step: f32,
    /// Number of local full-density Sun samples, from zero to three.
    pub light_samples: u32,
    /// Trace one texel per square block; 1, 2 or 4. A reset traces all texels.
    /// One refresh cycle is this stride squared, in submitted frames.
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
    inverse_raster: [[f32; 4]; 4],
    previous_raster: [[f32; 4]; 4],
}

#[derive(Clone, Copy)]
struct LayerSample {
    travel: [f64; 2],
    environment: thrum_render::atmosphere::EnvironmentEpoch,
    history_valid: bool,
    raster: [[f32; 4]; 4],
}

pub(crate) struct Layer {
    profile: SkyProfile,
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
    trace_group: wgpu::BindGroup,
    compose_group: wgpu::BindGroup,
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
                initialized: false,
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
        graph.after_opaque.push(hemisphere);
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
        // Preserve the air lattice at long travel without subtracting large
        // rounded floats in the shader. Noise keeps the anchor's wrapped phase.
        let anchor = std::array::from_fn::<_, 2, _>(|i| {
            ((f64::from(frame.camera[[0, 2][i]]) - travel[i]) / 200.0).floor() * 200.0
        });
        self.pending = (!frame.time.capture).then_some(LayerSample {
            travel,
            environment: environment.epoch,
            history_valid: !reset,
            raster: frame.view_proj,
        });
        let stride = self.profile.refresh_stride;
        let full_trace = reset
            || frame.time.capture
            || self.profile.extent.iter().any(|&x| x.div_ceil(2) < stride);
        self.trace_stride = if full_trace { 1 } else { stride };
        let mut heights = weather.height_range();
        if let Some(shapes) = shapes {
            heights = shapes.heights(state, kind, heights);
        }
        let (phase, scattering) = self
            .lighting
            .type_parameters(state.parameters().kind, (1.0 - kind.deck) * 0.33);
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
                self.profile.step,
                200_000.0,
            ],
            light_samples: self.profile.light_samples,
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
            inverse_raster: glam::Mat4::from_cols_array_2d(&frame.view_proj)
                .inverse()
                .to_cols_array_2d(),
            previous_raster,
        };
        self.queue
            .write_buffer(&self.params, 0, bytemuck::bytes_of(&uniform));
        Ok(())
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
            self.temporal
                .encode(pass, [self.textures[0].width(), self.textures[0].height()]);
            return;
        }
        pass.set_pipeline(&self.trace);
        pass.set_bind_group(0, self.environment.group(frame.exposure_side), &[]);
        pass.set_bind_group(2, &self.trace_group, &[]);
        if let Some(shapes) = shapes {
            pass.set_bind_group(3, &shapes.group, &[]);
        }
        pass.dispatch_workgroups(
            self.textures[0].width().div_ceil(8 * self.trace_stride),
            self.textures[0].height().div_ceil(8 * self.trace_stride),
            1,
        );
    }

    pub(crate) fn draw(&self, pass: &mut wgpu::RenderPass<'_>, frame: &ContributionFrame) {
        pass.set_pipeline(&self.compose);
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
            refresh_stride: self.profile.refresh_stride,
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
        resources
    }

    pub(crate) fn set_quality(&mut self, stride: u32, light_samples: u32) {
        self.profile.refresh_stride = stride;
        self.profile.light_samples = light_samples;
        self.temporal.clock.invalidate();
        self.committed = None;
    }
}

fn declaration(name: &str, kind: AfterOpaqueKind, accesses: Vec<EffectAccess>) -> AfterOpaquePass {
    AfterOpaquePass {
        name: name.into(),
        kind,
        cadence: EffectCadence::EveryFrame,
        accesses,
        depth: true,
        write_depth: false,
        refraction: false,
        images: thrum_render::contribution::AfterOpaqueImages::None,
    }
}

#[cfg(test)]
mod tests;
