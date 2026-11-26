//! Screen-space optics capture and receiver admission.
mod regions;

use bytemuck::{Pod, Zeroable};
use std::sync::{Arc, Mutex};
use thrum_render::{
    atmosphere::EnvironmentProducts,
    binding,
    contribution::{
        AfterOpaqueImages, AfterOpaqueKind, AfterOpaquePass, ContributionFrame, EffectCadence,
        SceneSurface,
    },
};
use thrum_sea_model::Spectrum;
use thrum_water_model::Map;

const PASS: &str = "water gather";
const IMAGE_FORMATS: &[wgpu::TextureFormat] = &[
    wgpu::TextureFormat::Rgba16Float,
    wgpu::TextureFormat::R32Float,
];
const GRID_WIDTH: usize = 32;
const GRID_HEIGHT: usize = 18;
const SOURCE: u32 = 1 << 31;
const SHALLOW: u32 = 1 << 30;
const UNDER: u32 = 1 << 29;

/// Maximum number of opaque body boxes read by one water contribution.
const BODY_CAPACITY: usize = 16;

#[derive(Clone, Copy, Default, PartialEq)]
pub(super) struct MirrorRoughness {
    tail_rms_floor: f64,
    shortest_height: f64,
}

impl MirrorRoughness {
    pub(super) fn new(spectrum: &Spectrum, chop: f64, shortest_height: f32) -> Self {
        let deformation = std::iter::once(spectrum.long.as_slice())
            .chain(
                spectrum
                    .cascades
                    .iter()
                    .map(thrum_sea_model::Cascade::components),
            )
            .map(|band| thrum_sea_model::contraction_bound(band, chop).unwrap_or(f64::INFINITY))
            .sum::<f64>();
        // Filtered differentials are convex combinations, with each gain at
        // most one. Leave room for FFT and half-float reconstruction.
        let deformation = if deformation > 0.0 {
            deformation.mul_add(1.05, 0.002)
        } else {
            deformation
        };
        // In world space, the isotropic tail variance cannot shrink by more
        // than the largest stretch of the horizontal Jacobian. When the
        // shader clamps its determinant, use the adjugate's smaller axis.
        let minimum_transform =
            (1.0 / (1.0 + deformation)).min((1.0 - deformation).max(0.0) / 0.05);
        // The other variance terms can only add to the tail in exact arithmetic.
        Self {
            tail_rms_floor: spectrum.tail_slope_variance.sqrt() * minimum_transform,
            shortest_height: f64::from(shortest_height),
        }
    }

    pub(super) fn with_foam(mut self, enabled: bool) -> Self {
        // Whitewater can leave as little as 35% of the slope variance.
        if enabled {
            self.tail_rms_floor *= 0.35_f64.sqrt();
        }
        self
    }

    fn can_mirror(self, gain: f64, depth: f64, limit: f32) -> bool {
        // The depth limiter adds nonnegative swash depth. Omitting that
        // allowance gives a lower bound on the shader's shortest-band gain.
        let gain = if self.shortest_height > 0.0 {
            gain.min(0.6 * depth.max(0.0) / self.shortest_height)
        } else {
            gain
        };
        self.tail_rms_floor * gain <= 1.001 * f64::from(limit)
    }
}

/// The optical coefficients and screen-space trace budget for a water type.
#[derive(Clone, Copy, Debug)]
pub struct Optics {
    /// RGB extinction in inverse metres along a viewed path.
    pub extinction: [f32; 3],
    /// RGB attenuation in inverse metres for incident light above a receiver.
    pub downwelling: [f32; 3],
    /// Maximum reflected path in metres.
    pub reflection_reach: f32,
    /// Largest resolved normal tilt admitted by body tracing, in degrees.
    pub slope_allowance: f32,
    /// Scene reflection fades out by this unresolved RMS slope.
    pub mirror_limit: f32,
}

impl Default for Optics {
    fn default() -> Self {
        Self {
            extinction: [0.62, 0.27, 0.20],
            downwelling: [0.34, 0.16, 0.11],
            reflection_reach: 64.0,
            slope_allowance: 20.0,
            mirror_limit: 0.045,
        }
    }
}

impl Optics {
    fn sight_depth(self) -> f32 {
        -0.01_f32.ln() / self.extinction.into_iter().fold(f32::INFINITY, f32::min)
    }

    pub(super) fn valid(self) -> bool {
        self.extinction
            .into_iter()
            .all(|c| (0.001..=100.0).contains(&c))
            && self
                .downwelling
                .into_iter()
                .all(|c| (0.0..=100.0).contains(&c))
            && (1.0..=256.0).contains(&self.reflection_reach)
            && (1.0..=30.0).contains(&self.slope_allowance)
            && (0.001..=0.2).contains(&self.mirror_limit)
    }
}

/// A world box of opaque geometry that water may reflect or see through.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct BodyBounds {
    min: [f32; 4],
    max: [f32; 4],
}

impl BodyBounds {
    /// Admit finite, ordered world-space corners.
    #[must_use]
    pub fn new(min: [f32; 3], max: [f32; 3]) -> Option<Self> {
        (0..3)
            .all(|axis| min[axis].is_finite() && max[axis].is_finite() && min[axis] <= max[axis])
            .then(|| Self {
                min: [min[0], min[1], min[2], 0.0],
                max: [max[0], max[1], max[2], 0.0],
            })
    }
}

/// A frame's opaque bounds, written by the producer of the corresponding geometry.
#[derive(Clone, Default)]
pub struct OpticsBodies(Arc<Mutex<Vec<BodyBounds>>>);

impl OpticsBodies {
    /// Replace the complete visible body set in publication order.
    /// Returns false and keeps the previous set when capacity is exceeded.
    /// # Panics
    /// Panics if the writer previously poisoned the body lock.
    pub fn publish(&self, boxes: &[BodyBounds]) -> bool {
        if boxes.len() > BODY_CAPACITY {
            return false;
        }
        let mut published = self.0.lock().expect("water optics bodies");
        published.clear();
        published.extend_from_slice(boxes);
        true
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Inputs {
    extinction_sight: [f32; 4],
    downwelling_mirror: [f32; 4],
    reach_slope_scene: [f32; 4],
    dispatch: [u32; 4],
    mask: [[[u32; 4]; GRID_WIDTH / 4]; GRID_HEIGHT],
    bodies: [BodyBounds; BODY_CAPACITY],
    sources: [[u32; 4]; BODY_CAPACITY],
}

pub(super) struct Gather {
    profile: Optics,
    pub(super) buffer: wgpu::Buffer,
    group: wgpu::BindGroup,
    pipeline: wgpu::ComputePipeline,
    fields: regions::Shallows,
    dispatch: [u32; 2],
    pub(super) lens: Option<wgpu::Buffer>,
    pub(super) under: bool,
}

pub(super) fn declaration() -> AfterOpaquePass {
    AfterOpaquePass {
        name: PASS.into(),
        kind: AfterOpaqueKind::Compute,
        cadence: EffectCadence::EveryFrame,
        accesses: Vec::new(),
        depth: true,
        write_depth: false,
        refraction: false,
        images: AfterOpaqueImages::Capture(IMAGE_FORMATS),
    }
}

pub(super) fn sampled(producer: usize) -> AfterOpaqueImages {
    AfterOpaqueImages::Sample { producer, count: 2 }
}

pub(super) fn uniform(stages: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    binding::uniform(14, stages, size_of::<Inputs>() as u64)
}

pub(super) fn entry(buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding: 14,
        resource: buffer.as_entire_binding(),
    }
}

impl Gather {
    pub(super) fn new(
        surface: &SceneSurface<'_>,
        environment: &EnvironmentProducts,
        profile: Optics,
        library: &str,
        underwater: bool,
    ) -> Self {
        let device = surface.device;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("water gather regions"),
            size: size_of::<Inputs>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let regions = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("water gather regions"),
            entries: &[uniform(wgpu::ShaderStages::COMPUTE)],
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("water gather regions"),
            layout: &regions,
            entries: &[entry(&buffer)],
        });
        let inputs = declaration().input_layout(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(PASS),
            bind_group_layouts: &[
                Some(surface.view_layout),
                Some(&inputs),
                Some(&environment.layout),
                Some(&regions),
            ],
            immediate_size: 0,
        });
        let source = library.to_owned()
            + super::VIEW_RECORDS
            + include_str!("optics/regions.wgsl")
            + include_str!("optics/gather.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(PASS),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("gather"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Self {
            profile,
            buffer,
            group,
            pipeline,
            fields: regions::Shallows::default(),
            dispatch: [0; 2],
            lens: underwater.then(|| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("water lens columns"),
                    size: size_of::<super::underwater::Lens>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            }),
            under: false,
        }
    }

    pub(super) fn record(
        &mut self,
        queue: &wgpu::Queue,
        frame: &ContributionFrame,
        map: Option<&Map>,
        opaque: &OpticsBodies,
        water_level: f64,
        displacement: [f32; 2],
        roughness: MirrorRoughness,
        lens: Option<&super::underwater::Lens>,
    ) {
        let boxes = opaque.0.lock().expect("water optics bodies");
        let mut inputs = self.fields.inputs(
            self.profile,
            frame,
            map,
            &boxes,
            water_level,
            displacement,
            roughness,
        );
        self.under = lens.is_some_and(|lens| lens.state[0] != 0);
        if let Some((lens, buffer)) = lens.zip(self.lens.as_ref()) {
            queue.write_buffer(buffer, 0, bytemuck::bytes_of(lens));
        }
        if self.under {
            inputs.dispatch = [0, 0, frame.render_extent[0], frame.render_extent[1]];
            for row in &mut inputs.mask {
                for cell in row.iter_mut().flatten() {
                    *cell |= SOURCE | UNDER;
                }
            }
        }
        self.dispatch = [
            inputs.dispatch[2].div_ceil(8),
            inputs.dispatch[3].div_ceil(8),
        ];
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&inputs));
    }

    pub(super) fn runs(&self) -> bool {
        self.dispatch[0] != 0 && self.dispatch[1] != 0
    }

    pub(super) fn encode(&self, pass: &mut wgpu::ComputePass<'_>, environment: &wgpu::BindGroup) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(2, environment, &[]);
        pass.set_bind_group(3, &self.group, &[]);
        pass.dispatch_workgroups(self.dispatch[0], self.dispatch[1], 1);
    }

    pub(super) fn bytes(&self) -> u64 {
        self.buffer.size()
    }
}

#[cfg(test)]
mod tests {
    use super::{BodyBounds, Inputs};
    use thrum_test_support::{record, records};

    impl records::Kind for BodyBounds {
        const KIND: Option<records::RecordScalar> = None;
    }

    #[test]
    fn optics_regions_keep_the_host_record_layout() {
        let shader = include_str!("optics/regions.wgsl")
            .split_once("@group(3)")
            .expect("records")
            .0;
        records::named(
            "water optics regions",
            shader,
            &[
                record!(BodyBounds as "OpticsBody", min as "low", max as "high"),
                record!(
                    Inputs as "OpticsInputs",
                    extinction_sight,
                    downwelling_mirror,
                    reach_slope_scene,
                    dispatch,
                    mask,
                    bodies,
                    sources
                ),
            ],
        );
    }
}
