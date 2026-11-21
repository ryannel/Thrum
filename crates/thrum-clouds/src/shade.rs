use crate::{Error, SunPath, SunProfile, WeatherMap, WeatherProfile};
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
#[cfg(target_arch = "wasm32")]
pub use observation::CloudSubmission;
#[cfg(test)]
mod tests;

fn frame_prepare_error(error: &Error) -> FrameError {
    match error {
        Error::DeviceLimit(_) => FrameError::Gpu(error.to_string()),
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
    ground: Option<(wgpu::Texture, wgpu::Buffer)>,
    resources: Vec<(String, u64)>,
    sky: Option<crate::SkyProducts>,
    #[cfg(target_arch = "wasm32")]
    observation: observation::Observation,
}

impl WeatherInput {
    /// Optional readback handles for the latest submitted visible layer.
    pub fn sky_products(&self) -> Option<crate::SkyProducts> {
        self.0.lock().ok()?.sky.clone()
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

    /// Prepared texture payloads and uniform-buffer bytes, excluding driver allocation overhead.
    pub fn resource_bytes(&self) -> Vec<(String, u64)> {
        self.0
            .lock()
            .map_or_else(|_| Vec::new(), |mailbox| mailbox.resources.clone())
    }
}

/// Weather and cloud shade, optionally extended with [`Self::with_sky`].
/// The default owns no visible layer; neither capability owns a clock.
pub struct Shade {
    quality: Option<QualitySettings>,
    sky: Option<crate::SkyProfile>,
    config: ShadeConfig,
    input: WeatherInput,
    gpu: Option<Box<Prepared>>,
    generation: u64,
    dropped: u64,
    valid_recording: bool,
}

struct Prepared {
    sky: Option<crate::layer::Layer>,
    weather: WeatherMap,
    sun: SunPath,
    queue: wgpu::Queue,
    #[cfg(target_arch = "wasm32")]
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
            quality: None,
            sky: None,
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
    /// Add the visible Sky capability. Shadow-only scenes allocate none of it.
    /// # Errors
    /// Refuses unsupported refresh or local-light sample counts.
    pub fn with_sky(mut self, profile: crate::SkyProfile) -> Result<Self, Error> {
        if ![1, 2, 4].contains(&profile.refresh_stride) || profile.light_samples > 3 {
            return Err(Error::Unsupported("Sky refresh or local-light samples"));
        }
        self.quality = Some(
            QualitySettings::new(vec![
                QualityKnob {
                    name: "cloud refresh exponent".into(),
                    effect: "Refresh every cloud texel within 4^value frames",
                    value: profile.refresh_stride.trailing_zeros(),
                    range: 0..=2,
                    invalidation: QualityInvalidation::EffectHistory,
                },
                QualityKnob {
                    name: "cloud local light samples".into(),
                    effect: "Samples used for each cloud local light",
                    value: profile.light_samples,
                    range: 0..=3,
                    invalidation: QualityInvalidation::EffectHistory,
                },
            ])
            .map_err(|_| Error::Unsupported("Sky quality declaration"))?,
        );
        self.sky = Some(profile);
        Ok(self)
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
    pub fn provider(&self) -> Result<SunOpticalDepth, thrum_render::synthetic::RenderError> {
        SunOpticalDepth::gpu_target([self.config.sun.ground_size; 2], 1)
    }
    /// Frames dropped for unavailable or mismatched publication inputs.
    pub fn dropped_frames(&self) -> u64 {
        self.dropped
    }
}

impl SceneContribution for Shade {
    fn quality_knobs(&self) -> Vec<QualityKnob> {
        self.quality
            .as_ref()
            .map_or_else(Vec::new, |q| q.knobs().to_vec())
    }
    fn set_quality_knob(&mut self, name: &str, value: u32) -> Result<(), QualityError> {
        self.quality
            .as_mut()
            .ok_or_else(|| QualityError::UnknownKnob { name: name.into() })?
            .set(name, value)
    }
    fn commit_quality_knobs(&mut self) -> QualityInvalidation {
        let Some(quality) = &mut self.quality else {
            return QualityInvalidation::None;
        };
        let invalidation = quality.commit();
        if invalidation != QualityInvalidation::None {
            let stride = 1 << quality.knobs()[0].value;
            let light_samples = quality.knobs()[1].value;
            if let Some(profile) = &mut self.sky {
                profile.refresh_stride = stride;
                profile.light_samples = light_samples;
            }
            if let Some(layer) = self.gpu.as_mut().and_then(|gpu| gpu.sky.as_mut()) {
                layer.set_quality(stride, light_samples);
            }
        }
        invalidation
    }
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        self.rebuild_for_attachment(surface)
            .map_err(|error| match error {
                FrameError::Scene(message) | FrameError::Gpu(message) => message,
                other => other.to_string(),
            })
    }
    fn rebuild_for_attachment(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        let target = surface.sun_path_target().ok_or(FrameError::InvalidState(
            "cloud Shade requires the scene's GPU Sun-path target",
        ))?;
        if let Some(gpu) = &self.gpu
            && self.generation == surface.generation
            && gpu.sun.ground() == &target.0
            && gpu.sun.mapping() == &target.1
        {
            return Ok(());
        }
        let mut weather = if self.generation == surface.generation {
            match self.gpu.take() {
                Some(gpu) => gpu.weather,
                None => WeatherMap::new(
                    surface.device,
                    self.config.seed,
                    self.config.model,
                    self.config.types.clone(),
                    self.config.weather.clone(),
                )
                .map_err(|error| frame_prepare_error(&error))?,
            }
        } else {
            WeatherMap::new(
                surface.device,
                self.config.seed,
                self.config.model,
                self.config.types.clone(),
                self.config.weather.clone(),
            )
            .map_err(|error| frame_prepare_error(&error))?
        };
        weather.rebuild(surface.generation, 1);
        let mut sun = SunPath::new(
            surface.device,
            &weather,
            self.config.sun.clone(),
            Some(target),
        )
        .map_err(|error| frame_prepare_error(&error))?;
        sun.rebuild(surface.generation, 1);
        sun.prepare_noise(surface.queue);
        let sky = self
            .sky
            .map(|profile| crate::layer::Layer::new(surface, &weather, &sun, profile))
            .transpose()
            .map_err(|error| frame_prepare_error(&error))?;
        let mut resources: Vec<_> = weather
            .textures()
            .iter()
            .chain(sun.products())
            .chain(std::iter::once(sun.ground()))
            .enumerate()
            .map(|(i, texture)| {
                (
                    format!("cloud texture {i} {:?}", texture.format()),
                    u64::from(texture.width())
                        * u64::from(texture.height())
                        * u64::from(texture.depth_or_array_layers())
                        * u64::from(texture.format().block_copy_size(None).unwrap_or(0)),
                )
            })
            .collect();
        resources.push(("cloud shape noise".into(), sun.shape_noise.bytes()));
        resources.extend(
            weather
                .uniforms()
                .iter()
                .chain(sun.uniforms())
                .chain(weather.axes())
                .chain(std::iter::once(sun.mapping()))
                .enumerate()
                .map(|(i, b)| (format!("cloud buffer {i}"), b.size())),
        );
        if let Some(layer) = &sky {
            resources.extend(layer.resource_bytes());
        }
        self.input
            .0
            .lock()
            .map_err(|_| frame_prepare_error(&Error::InputUnavailable))?
            .resources = resources;
        self.input
            .0
            .lock()
            .map_err(|_| frame_prepare_error(&Error::InputUnavailable))?
            .ground = Some((sun.ground().clone(), sun.mapping().clone()));
        self.generation = surface.generation;
        self.gpu = Some(Box::new(Prepared {
            sky,
            weather,
            sun,
            queue: surface.queue.clone(),
            #[cfg(target_arch = "wasm32")]
            device: wgpu::Device::clone(surface.device),
            #[cfg(target_arch = "wasm32")]
            ground_reference: None,
        }));
        Ok(())
    }
    fn effect_graph(&self) -> EffectGraph {
        self.gpu.as_ref().map_or_else(EffectGraph::default, |gpu| {
            let (mut graph, shape_noise) = graph::declaration(gpu, self.generation);
            if let Some(sky) = &gpu.sky {
                sky.append_graph(
                    &mut graph,
                    self.generation,
                    gpu.sun.shape_noise.read(shape_noise),
                );
            }
            graph
        })
    }
    fn encode_upload(&mut self, _encoder: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        self.valid_recording = false;
        let Some(gpu) = &mut self.gpu else {
            return;
        };
        let state = self
            .input
            .0
            .lock()
            .ok()
            .and_then(|input| input.state.as_ref().map(Arc::clone));
        let Some(state) = state else {
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
            gpu.sun.upload(
                &gpu.queue,
                &gpu.weather,
                frame.time,
                &state,
                camera,
                environment.true_sun.toward_sun,
                frame.history_reset,
            )
        });
        if result.is_err() {
            gpu.sun.disable(&gpu.queue);
            self.dropped += 1;
        } else {
            self.valid_recording = true;
            if let Some(sky) = &mut gpu.sky
                && sky.upload(&gpu.weather, &state, frame).is_err()
            {
                self.valid_recording = false;
                self.dropped += 1;
            }
        }
    }
    fn effect_runs(&self, index: usize, _frame: &ContributionFrame) -> bool {
        self.valid_recording
            && self.gpu.as_ref().is_some_and(|gpu| {
                if index == 0 {
                    gpu.weather.runs()
                } else {
                    gpu.sun.runs(index - 1)
                }
            })
    }
    fn encode_effect(
        &mut self,
        index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        _frame: &ContributionFrame,
    ) {
        if self.valid_recording
            && let Some(gpu) = &self.gpu
        {
            if index == 0 {
                gpu.weather.encode(pass);
            } else {
                gpu.sun.encode(index - 1, pass);
            }
        }
    }
    fn after_opaque_runs(&self, index: usize, frame: &ContributionFrame) -> bool {
        index != 2 || !frame.time.capture
    }
    fn encode_after_opaque(
        &mut self,
        index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &ContributionFrame,
    ) {
        if self.valid_recording
            && let Some(sky) = self.gpu.as_mut().and_then(|gpu| gpu.sky.as_mut())
        {
            sky.encode(index, pass, frame);
        }
    }
    fn draw_after_opaque(
        &self,
        _index: usize,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &ContributionFrame,
    ) {
        if self.valid_recording
            && let Some(sky) = self.gpu.as_ref().and_then(|gpu| gpu.sky.as_ref())
        {
            sky.draw(pass, frame);
        }
    }
    fn submitted(&mut self, _history_advanced: bool) {
        if self.valid_recording
            && let Some(gpu) = &mut self.gpu
        {
            gpu.weather.submitted();
            gpu.sun.submitted();
            if let Some(sky) = &mut gpu.sky {
                sky.submitted();
            }
            if (cfg!(target_arch = "wasm32") || gpu.sky.is_some())
                && let Ok(mut input) = self.input.0.lock()
            {
                if let Some(sky) = &gpu.sky {
                    input.sky = Some(sky.products());
                }
                #[cfg(target_arch = "wasm32")]
                input.observe(gpu, self.generation);
            }
        }
    }
}
