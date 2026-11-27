use crate::ground::{GroundConfig, GroundGpu};
use crate::{
    Error, ShapesConfig, ShapesReport, SunPath, SunProfile, WeatherMap, WeatherProfile,
    shapes::Shapes,
};
use std::sync::{Arc, Mutex};
use thrum_cloud_model::{Field, Profile, State, Type};
use thrum_render::{
    atmosphere::SunOpticalDepth,
    contribution::{
        ContributionFrame, ContributionPasses, EffectGraph, QualityError, QualityInvalidation,
        QualityKnob, QualitySettings, SceneContribution, SceneSurface,
    },
    runtime::FrameError,
};

mod graph;
#[cfg(target_arch = "wasm32")]
mod observation;
mod preparation;
mod quality;
#[cfg(target_arch = "wasm32")]
pub use observation::CloudSubmission;
#[cfg(test)]
mod tests;

fn frame_prepare_error(error: &Error) -> FrameError {
    match error {
        Error::DeviceLimit(_) => FrameError::ContributionDeviceLimit(error.to_string()),
        Error::Model(_) | Error::Unsupported(_) => FrameError::Scene(error.to_string()),
        Error::InputUnavailable => FrameError::InvalidState("cloud mailbox unavailable"),
    }
}

/// Immutable configuration for the Weather and Shade capabilities alone.
#[derive(Clone, Debug)]
pub struct ShadeConfig {
    /// Weather lattice seed.
    pub seed: u32,
    /// Structural weather constants.
    pub model: Profile,
    /// Game-owned type rows, copied once at preparation.
    pub types: Vec<Type>,
    /// Weather-map resolution and age bound.
    pub weather: WeatherProfile,
    /// Sun-ray levels, ground field and cadence bounds.
    pub sun: SunProfile,
}

/// A mailbox for the immutable state of the publication selected for rendering.
/// Publishing here never steps the world or owns a second weather authority.
#[derive(Clone, Default)]
pub struct WeatherInput(Arc<Mutex<Mailbox>>);

#[derive(Default)]
struct Mailbox {
    state: Option<Arc<State>>,
    field_overlay: crate::FieldOverlay,
    ground: Option<(wgpu::Texture, wgpu::Buffer)>,
    condensate: Option<crate::GroundProducts>,
    weather_columns: Option<wgpu::Texture>,
    resources: Vec<(String, u64)>,
    shape_resources: Vec<(String, u64)>,
    sky: Option<crate::SkyProducts>,
    shapes: Option<ShapesReport>,
    placed_map: Option<(wgpu::Texture, [f64; 2], Arc<State>)>,
    #[cfg(target_arch = "wasm32")]
    observation: observation::Observation,
}

impl WeatherInput {
    /// Select an internal field to display, independent of the weather publication.
    /// # Errors
    /// Refuses an unavailable mailbox.
    pub fn set_field_overlay(&self, overlay: crate::FieldOverlay) -> Result<(), Error> {
        self.0
            .lock()
            .map_err(|_| Error::InputUnavailable)?
            .field_overlay = overlay;
        Ok(())
    }

    fn frame_input(&self) -> Option<(Arc<State>, crate::FieldOverlay)> {
        let input = self.0.lock().ok()?;
        Some((Arc::clone(input.state.as_ref()?), input.field_overlay))
    }

    /// Physical weather columns on the current device, for optional readback after submission.
    pub fn weather_columns(&self) -> Option<wgpu::Texture> {
        self.0.lock().ok()?.weather_columns.clone()
    }

    /// Optional readback handles for the latest submitted visible layer.
    pub fn sky_products(&self) -> Option<crate::SkyProducts> {
        self.0.lock().ok()?.sky.clone()
    }
    /// Placement and lease outcomes observed by the latest submitted frame.
    pub fn shapes_report(&self) -> Option<ShapesReport> {
        self.0.lock().ok()?.shapes
    }
    /// The submitted placed-column map and its fine air-frame origin. Each texel is
    /// slot-plus-one, volume weight, un-eroded optical depth and life envelope.
    pub fn placed_map(&self) -> Option<(wgpu::Texture, [f64; 2], Arc<State>)> {
        self.0.lock().ok()?.placed_map.clone()
    }

    /// Publish the weather lease from the same page the scene extracts.
    /// # Errors
    /// Refuses an unavailable mailbox.
    pub fn publish(&self, state: Arc<State>) -> Result<(), Error> {
        self.0.lock().map_err(|_| Error::InputUnavailable)?.state = Some(state);
        Ok(())
    }

    /// Inspect the current device's submitted ground product and mapping.
    /// The handles are replaced on device rebuild; reading them is optional and
    /// never participates in the effect's progress.
    pub fn ground_product(&self) -> Option<(wgpu::Texture, wgpu::Buffer)> {
        self.0.lock().ok()?.ground.clone()
    }

    /// Current ground-band and coefficient readback handles, if Ground was declared.
    pub fn ground_condensate_products(&self) -> Option<crate::GroundProducts> {
        self.0.lock().ok()?.condensate.clone()
    }

    /// Prepared texture payloads and uniform-buffer bytes, excluding driver allocation overhead.
    pub fn resource_bytes(&self) -> Vec<(String, u64)> {
        self.0.lock().map_or_else(
            |_| Vec::new(),
            |mailbox| {
                mailbox
                    .resources
                    .iter()
                    .chain(&mailbox.shape_resources)
                    .cloned()
                    .collect()
            },
        )
    }
}

/// Weather and cloud shade, optionally extended with [`Self::with_sky`].
/// The default owns no visible layer; neither capability owns a clock.
pub struct Shade {
    quality: QualitySettings,
    sky: Option<crate::SkyProfile>,
    shapes: Option<ShapesConfig>,
    ground: Option<GroundConfig>,
    config: ShadeConfig,
    input: WeatherInput,
    gpu: Option<Box<Prepared>>,
    generation: u64,
    dropped: u64,
    valid_recording: bool,
}

struct Prepared {
    placed_map: Option<crate::shapes::map::ShapeMap>,
    recorded_placed_state: Option<Arc<State>>,
    shapes: Option<Box<Shapes>>,
    sky: Option<crate::layer::Layer>,
    weather: WeatherMap,
    sun: SunPath,
    ground: Option<GroundGpu>,
    queue: wgpu::Queue,
    device: wgpu::Device,
    #[cfg(target_arch = "wasm32")]
    ground_reference: Option<crate::sun_path::ground_reference::GroundReference>,
}

impl Shade {
    /// Create an unattached effect. Device resources are prepared by `rebuild`.
    /// # Errors
    /// Rejects invalid authoritative field data.
    pub fn new(config: ShadeConfig) -> Result<Self, Error> {
        Field::new(config.seed, config.model, config.types.clone()).map_err(Error::Model)?;
        Ok(Self {
            quality: quality::base()?,
            sky: None,
            shapes: None,
            ground: None,
            config,
            input: WeatherInput::default(),
            gpu: None,
            generation: 0,
            dropped: 0,
            valid_recording: false,
        })
    }
    /// Clone the mailbox the game's extractor publishes into.
    pub fn input(&self) -> WeatherInput {
        self.input.clone()
    }
    /// Configure or replace the visible Sky capability. Shadow-only scenes allocate none of it.
    /// # Errors
    /// Refuses unsupported refresh or local-light sample counts.
    pub fn with_sky(mut self, profile: crate::SkyProfile) -> Result<Self, Error> {
        if ![1, 2, 4].contains(&profile.refresh_stride) || profile.light_samples > 3 {
            return Err(Error::Unsupported("Sky refresh or local-light samples"));
        }
        self.configure_quality(quality::sky(profile, self.ground.is_some()))?;
        self.sky = Some(profile);
        Ok(self)
    }
    /// Configure or replace Shapes for the ground path and any declared visible layer.
    /// # Errors
    /// Rejects an invalid placement range or volume capacity.
    pub fn with_shapes(mut self, config: ShapesConfig) -> Result<Self, Error> {
        config.validate()?;
        self.configure_quality([quality::placed()])?;
        self.shapes = Some(config);
        Ok(self)
    }

    /// Select ground condensate. The scene must declare a view-volume medium.
    #[must_use]
    pub fn with_ground(mut self, config: GroundConfig) -> Self {
        if self.sky.is_some() {
            self.fix_ground_scale();
        }
        self.ground = Some(config);
        self
    }

    /// Share the extractor mailbox when composing this effect with another contribution.
    #[must_use]
    pub fn with_input(mut self, input: WeatherInput) -> Self {
        self.input = input;
        self
    }
    /// Declare the shared GPU field in the scene before preparing it.
    /// # Errors
    /// Rejects an empty target extent.
    pub fn provider(&self) -> Result<SunOpticalDepth, thrum_render::error::RenderError> {
        let sun = SunOpticalDepth::gpu_target([self.config.sun.ground_size; 2], 1)?;
        if let Some(ground) = &self.ground {
            Ok(sun.with_ground_band(crate::ground::band::provider(ground)?))
        } else {
            Ok(sun)
        }
    }
    fn has_sky_overlay(&self) -> bool {
        self.gpu
            .as_ref()
            .and_then(|gpu| gpu.sky.as_ref())
            .is_some_and(crate::layer::Layer::has_overlay)
    }

    /// Frames dropped for unavailable or mismatched publication inputs.
    pub fn dropped_frames(&self) -> u64 {
        self.dropped
    }
}

impl SceneContribution for Shade {
    fn needs_render_capacity(&self, extent: [u32; 2]) -> bool {
        self.ground.is_some()
            && self
                .gpu
                .as_ref()
                .and_then(|gpu| gpu.sky.as_ref())
                .is_some_and(|layer| !layer.has_render_capacity(extent))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn extent_resize_job(
        &self,
        extent: [u32; 2],
        _resize_secondary: bool,
    ) -> Result<Option<thrum_render::contribution::ExtentResizeJob>, FrameError> {
        let Some(gpu) = &self.gpu else {
            return Ok(None);
        };
        let Some(layer) = gpu
            .sky
            .as_ref()
            .filter(|_| self.needs_render_capacity(extent))
        else {
            return Ok(None);
        };
        let device = gpu.device.clone();
        let inputs = layer.resize_inputs(extent);
        let mut graph = self.effect_graph();
        let mut bytes = gpu.resource_bytes(
            None,
            gpu.sun.ground(),
            gpu.sun.mapping(),
            gpu.ground.as_ref().map(|ground| &ground.targets),
        );
        let fixed_layer_bytes = layer.fixed_resource_bytes();
        Ok(Some(Box::new(move |_| {
            let targets = inputs.prepare(&device);
            targets.update_graph(&mut graph);
            bytes.extend(targets.resource_bytes());
            bytes.extend(fixed_layer_bytes);
            Ok(thrum_render::contribution::SecondaryResize::new(
                graph,
                move |shade: &mut Self| {
                    let gpu = shade.gpu.as_mut().expect("cloud resize owner");
                    let layer = gpu.sky.as_mut().expect("cloud layer resize owner");
                    targets.install(layer);
                    let mut mailbox = shade
                        .input
                        .0
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    mailbox.resources = bytes;
                },
            ))
        })))
    }

    fn quality_knobs(&self) -> Vec<QualityKnob> {
        self.quality
            .knobs()
            .iter()
            .filter(|knob| knob.name != quality::OVERLAY || self.has_sky_overlay())
            .cloned()
            .collect()
    }
    fn set_quality_knob(&mut self, name: &str, value: u32) -> Result<(), QualityError> {
        if name == quality::OVERLAY && !self.has_sky_overlay() {
            return Err(QualityError::UnknownKnob { name: name.into() });
        }
        self.quality.set(name, value)
    }
    fn commit_quality_knobs(&mut self) -> QualityInvalidation {
        let invalidation = self.quality.commit();
        if invalidation != QualityInvalidation::None {
            self.apply_quality();
        }
        invalidation
    }
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn supports_candidate_preparation(&self) -> bool {
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn prepare_candidate(
        &self,
        surface: &SceneSurface<'_>,
    ) -> Result<thrum_render::contribution::ContributionCandidate, FrameError> {
        let mut staged = self.stage(surface)?;
        let graph = staged.graph(self);
        let initialization = std::mem::take(&mut staged.initialization);
        Ok(thrum_render::contribution::ContributionCandidate::new(
            self.passes(),
            graph,
            move |shade: &mut Self| staged.install(shade),
        )
        .with_initialization(initialization))
    }

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        let target = surface
            .sun_path_target()
            .ok_or(preparation::MISSING_TARGET)?;
        if let Some(gpu) = &self.gpu
            && self.generation == surface.generation
            && gpu.sun.ground() == &target.0
            && gpu.sun.mapping() == &target.1
            && surface
                .render_capacity
                .is_none_or(|extent| !self.needs_render_capacity(extent))
            && gpu.ground.as_ref().is_none_or(|ground| {
                surface.view_volume_medium_target().is_some_and(|target| {
                    ground.targets.medium.target.coefficients == target.coefficients
                }) && surface
                    .ground_band_target()
                    .is_some_and(|(texture, mapping)| {
                        ground.targets.band.texture == texture
                            && ground.targets.band.mapping == mapping
                    })
            })
        {
            return Ok(());
        }
        let mut staged = self.stage(surface)?;
        if !staged.initialization.is_empty() {
            surface
                .queue
                .submit(std::mem::take(&mut staged.initialization));
        }
        staged.install(self);
        Ok(())
    }

    fn effect_graph(&self) -> EffectGraph {
        self.gpu.as_ref().map_or_else(EffectGraph::default, |gpu| {
            gpu.declaration(
                self.generation,
                gpu.sky.as_ref(),
                gpu.sun.ground(),
                gpu.sun.mapping(),
                gpu.ground.as_ref().map(|ground| &ground.targets),
            )
        })
    }
    fn encode_upload(&mut self, _encoder: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        self.valid_recording = false;
        let Some(gpu) = &mut self.gpu else {
            return;
        };
        let Some((state, field_overlay)) = self.input.frame_input() else {
            gpu.sun.disable(&gpu.queue);
            self.dropped += 1;
            return;
        };
        let Some(environment) = frame.environment else {
            gpu.sun.disable(&gpu.queue);
            self.dropped += 1;
            return;
        };
        if state.tick() != frame.time.tick {
            gpu.sun.disable(&gpu.queue);
            self.dropped += 1;
            return;
        }
        let camera = [f64::from(frame.camera[0]), f64::from(frame.camera[2])];
        let weather = gpu.weather.upload(&gpu.queue, frame.time, &state, camera);
        let result = weather.and_then(|()| {
            if let (Some(shapes), Some(map)) = (&mut gpu.shapes, &mut gpu.placed_map) {
                let (_, kind) = gpu.weather.density_data(&state);
                let instance_bytes = shapes.upload(
                    &gpu.queue,
                    frame.generation,
                    &state,
                    kind,
                    &gpu.weather,
                    camera,
                )?;
                map.upload(&gpu.queue, &gpu.weather, instance_bytes);
                gpu.recorded_placed_state = Some(Arc::clone(&state));
            }
            gpu.sun.upload_with_shapes(
                &gpu.queue,
                &gpu.weather,
                frame.time,
                &state,
                camera,
                environment.true_sun.toward_sun,
                frame.history_reset,
                gpu.shapes.as_deref(),
                gpu.placed_map
                    .as_ref()
                    .is_some_and(crate::shapes::map::ShapeMap::runs),
            )?;
            if let Some(ground) = &mut gpu.ground {
                let travel = state.display_travel(
                    f64::from(frame.time.display_phase),
                    frame.time.tick_duration.as_secs_f64(),
                    frame.history_reset,
                );
                ground.field.set_upper_heights(gpu.sun.frame_heights);
                ground.field.upload(&gpu.queue, &state, travel);
                ground
                    .phases
                    .upload(&gpu.queue, environment.true_sun.angular_radius_radians)?;
                ground.targets.band.upload(&gpu.queue, camera, travel);
                if ground.targets.medium.sky_params.is_some() {
                    ground
                        .targets
                        .medium
                        .lighting
                        .upload(&gpu.queue, environment.true_sun.angular_radius_radians)?;
                }
            }
            Ok(())
        });
        if result.is_err() {
            gpu.sun.disable(&gpu.queue);
            self.dropped += 1;
        } else {
            self.valid_recording = true;
            if let Some(sky) = &mut gpu.sky
                && sky
                    .upload(
                        &gpu.weather,
                        &state,
                        frame,
                        gpu.shapes.as_deref(),
                        field_overlay,
                    )
                    .is_err()
            {
                self.valid_recording = false;
                self.dropped += 1;
            }
        }
    }
    fn effect_runs(&self, index: usize, _frame: &ContributionFrame) -> bool {
        self.valid_recording
            && self.gpu.as_ref().is_some_and(|gpu| {
                let index = match (gpu.ground.is_some(), index) {
                    (true, 6 | 7) => return true,
                    (true, 8..) => index - 2,
                    _ => index,
                };
                match index {
                    0 => gpu.weather.runs(),
                    6 if gpu
                        .sky
                        .as_ref()
                        .is_some_and(crate::layer::Layer::has_overlay) =>
                    {
                        gpu.sky
                            .as_ref()
                            .is_some_and(crate::layer::Layer::overlay_runs)
                    }
                    6 | 7 => gpu
                        .placed_map
                        .as_ref()
                        .is_some_and(crate::shapes::map::ShapeMap::runs),
                    _ => gpu.sun.runs(index - 1),
                }
            })
    }
    fn encode_effect(
        &mut self,
        index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &ContributionFrame,
    ) {
        if self.valid_recording
            && let Some(gpu) = &self.gpu
        {
            let index = match (&gpu.ground, index) {
                (Some(ground), 6) => return ground.targets.band.encode(pass),
                (Some(ground), 7) => {
                    return ground.targets.medium.encode(
                        pass,
                        frame.exposure_side,
                        gpu.shapes.as_deref(),
                    );
                }
                (Some(_), 8..) => index - 2,
                _ => index,
            };
            match index {
                0 => gpu.weather.encode(pass),
                6 if gpu
                    .sky
                    .as_ref()
                    .is_some_and(crate::layer::Layer::has_overlay) =>
                {
                    if let Some(sky) = &gpu.sky {
                        sky.encode_overlay(pass, gpu.shapes.as_deref());
                    }
                }
                6 | 7 => {
                    if let (Some(map), Some(shapes)) = (&gpu.placed_map, &gpu.shapes) {
                        map.encode(shapes, pass);
                    }
                }
                _ => gpu
                    .sun
                    .encode_with_shapes(index - 1, pass, gpu.shapes.as_deref()),
            }
        }
    }
    fn after_opaque_runs(&self, index: usize, frame: &ContributionFrame) -> bool {
        let Some(sky) = self.gpu.as_ref().and_then(|gpu| gpu.sky.as_ref()) else {
            return false;
        };
        match index + usize::from(sky.has_overlay()) {
            2 => !frame.time.capture,
            4 => self.valid_recording && sky.field_runs(),
            _ => true,
        }
    }
    fn encode_after_opaque(
        &mut self,
        index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &ContributionFrame,
    ) {
        if self.valid_recording
            && let Some(gpu) = self.gpu.as_mut()
            && let Some(sky) = &mut gpu.sky
        {
            sky.encode(
                index + usize::from(sky.has_overlay()),
                pass,
                frame,
                gpu.shapes.as_deref(),
            );
        }
    }
    fn draw_after_opaque(
        &self,
        index: usize,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &ContributionFrame,
    ) {
        if self.valid_recording
            && let Some(sky) = self.gpu.as_ref().and_then(|gpu| gpu.sky.as_ref())
        {
            sky.draw(pass, frame, index + usize::from(sky.has_overlay()) == 4);
        }
    }
    fn submitted(&mut self, history_advanced: bool) {
        if self.valid_recording
            && let Some(gpu) = &mut self.gpu
        {
            gpu.weather.submitted();
            gpu.sun.submitted();
            if let Some(sky) = &mut gpu.sky {
                sky.submitted();
                sky.overlay_submitted();
            }
            let shapes_report = gpu.shapes.as_mut().map(|shapes| {
                let mut report = shapes.submitted(history_advanced);
                if let Some(map) = &mut gpu.placed_map {
                    map.submitted(&mut report);
                }
                report
            });
            if (cfg!(target_arch = "wasm32") || gpu.sky.is_some() || gpu.shapes.is_some())
                && let Ok(mut input) = self.input.0.lock()
            {
                if let Some(sky) = &gpu.sky {
                    input.sky = Some(sky.products());
                }
                if let Some(shapes) = &gpu.shapes {
                    input.shapes = shapes_report;
                    input.shape_resources = shapes.resource_bytes();
                }
                if let (Some(map), Some(state)) =
                    (&gpu.placed_map, gpu.recorded_placed_state.take())
                {
                    input.placed_map = Some((map.texture.clone(), gpu.weather.origins()[0], state));
                }
                #[cfg(target_arch = "wasm32")]
                input.observe(gpu, self.generation);
            }
        }
    }
}
