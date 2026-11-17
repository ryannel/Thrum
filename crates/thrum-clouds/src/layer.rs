//! Visible transport, recorded after the scene's opaque and celestial passes.
use crate::{Error, WeatherMap};
use thrum_cloud_model::State;
use thrum_render::{
    atmosphere::EnvironmentProducts,
    contribution::{
        AfterOpaqueKind, AfterOpaquePass, ContributionFrame, EffectAccess, EffectCadence,
        EffectGraph, SceneSurface,
    },
    graph::{AccessKind, ResourceDescriptor, ResourceScope, TextureKind},
};

mod prepare;

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
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    weather: [[f32; 4]; 4],
    physical: [f32; 4],
    erosion: [f32; 4],
    drift: [f32; 4],
    heights: [f32; 4],
    settings: [u32; 4],
}

pub(crate) struct Layer {
    profile: SkyProfile,
    environment: EnvironmentProducts,
    params: wgpu::Buffer,
    queue: wgpu::Queue,
    textures: [wgpu::Texture; 2],
    trace: wgpu::ComputePipeline,
    compose: wgpu::RenderPipeline,
    trace_group: wgpu::BindGroup,
    compose_group: wgpu::BindGroup,
}

impl Layer {
    pub(crate) fn append_graph(&self, graph: &mut EffectGraph, epoch: u64) {
        let first = graph.resources.len();
        for (i, texture) in self.textures.iter().enumerate() {
            graph.resources.push((
                ResourceDescriptor::Texture {
                    dimension: TextureKind::D2,
                    width: texture.width(),
                    height: texture.height(),
                    layers: 1,
                    mips: 1,
                    samples: 1,
                    format: texture.format(),
                    usage: texture.usage(),
                },
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
        for (resource, layers) in [(3, 12), (4, 8), (5, 4)] {
            let mut read = access(resource, AccessKind::SampledRead);
            read.range = 0..layers;
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
        graph
            .after_opaque
            .push(declaration(TRACE, AfterOpaqueKind::Compute, reads));
        graph.after_opaque.push(declaration(
            COMPOSE,
            AfterOpaqueKind::Render,
            (first..first + 2)
                .map(|i| access(i, AccessKind::SampledRead))
                .collect(),
        ));
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "presentation coordinates and physical coefficients narrow to shader floats"
    )]
    pub(crate) fn upload(&self, weather: &WeatherMap, state: &State, frame: &ContributionFrame) {
        let (model, kind) = weather.density_data(state);
        let travel = state.display_travel(
            f64::from(frame.time.display_phase),
            frame.time.tick_duration.as_secs_f64(),
            frame.history_reset,
        );
        let heights = weather.height_range();
        let uniform = Uniform {
            weather: std::array::from_fn(|i| {
                [
                    weather.origins()[i][0] as f32,
                    weather.origins()[i][1] as f32,
                    weather.profile().texels[i] as f32,
                    weather.profile().size as f32,
                ]
            }),
            physical: [
                kind.extinction() as f32,
                model.erosion as f32,
                model.detail_mean as f32,
                kind.droplet_radius as f32,
            ],
            erosion: [model.erosion as f32, 0.0, 0.0, 0.0],
            drift: [travel[0] as f32, travel[1] as f32, 0.0, 0.0],
            heights: [
                heights[0] as f32,
                heights[1] as f32,
                self.profile.step,
                200_000.0,
            ],
            settings: [
                weather.seed(),
                self.profile.light_samples,
                self.profile.extent[0],
                self.profile.extent[1],
            ],
        };
        self.queue
            .write_buffer(&self.params, 0, bytemuck::bytes_of(&uniform));
    }

    pub(crate) fn encode(&self, pass: &mut wgpu::ComputePass<'_>, frame: &ContributionFrame) {
        pass.set_pipeline(&self.trace);
        pass.set_bind_group(0, self.environment.group(frame.exposure_side), &[]);
        pass.set_bind_group(2, &self.trace_group, &[]);
        pass.dispatch_workgroups(
            self.textures[0].width().div_ceil(8),
            self.textures[0].height().div_ceil(8),
            1,
        );
    }

    pub(crate) fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.compose);
        pass.set_bind_group(2, &self.compose_group, &[]);
        pass.draw(0..3, 0..1);
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
    }
}

#[cfg(test)]
mod tests;
