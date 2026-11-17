use crate::{Error, SunPath, SunProfile, WeatherMap, WeatherProfile};
use std::sync::{Arc, Mutex};
use thrum_cloud_model::{Field, Profile, State, Type};
use thrum_render::{
    atmosphere::SunOpticalDepth,
    contribution::{
        ContributionFrame, ContributionPasses, EffectGraph, SceneContribution, SceneSurface,
    },
};

mod graph;

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
}

impl WeatherInput {
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

/// A shadow-only cloud effect. It draws no geometry or layer and owns no clock.
pub struct Shade {
    sky: Option<crate::SkyProfile>,
    config: ShadeConfig,
    input: WeatherInput,
    gpu: Option<Prepared>,
    generation: u64,
    dropped: u64,
    valid_recording: bool,
}

struct Prepared {
    sky: Option<crate::layer::Layer>,
    weather: WeatherMap,
    sun: SunPath,
    queue: wgpu::Queue,
}

impl Shade {
    /// Create an unattached effect. Device resources are prepared by `rebuild`.
    /// # Errors
    /// Rejects invalid authoritative field data.
    pub fn new(config: ShadeConfig) -> Result<Self, Error> {
        Field::new(config.seed, config.model, config.types.clone()).map_err(Error::Model)?;
        Ok(Self {
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
    #[must_use]
    pub fn with_sky(mut self, profile: crate::SkyProfile) -> Self {
        self.sky = Some(profile);
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
    pub fn provider(&self) -> Result<SunOpticalDepth, thrum_render::synthetic::RenderError> {
        SunOpticalDepth::gpu_target([self.config.sun.ground_size; 2], 1)
    }
    /// Frames dropped for unavailable or mismatched publication inputs.
    pub fn dropped_frames(&self) -> u64 {
        self.dropped
    }
}

impl SceneContribution for Shade {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        let target = surface
            .sun_path_target()
            .ok_or("cloud Shade requires the scene's GPU Sun-path target")?;
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
                .map_err(|e| e.to_string())?,
            }
        } else {
            WeatherMap::new(
                surface.device,
                self.config.seed,
                self.config.model,
                self.config.types.clone(),
                self.config.weather.clone(),
            )
            .map_err(|e| e.to_string())?
        };
        weather.rebuild(surface.generation, 1);
        let mut sun = SunPath::new(
            surface.device,
            &weather,
            self.config.sun.clone(),
            Some(target),
        )
        .map_err(|e| e.to_string())?;
        sun.rebuild(surface.generation, 1);
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
        self.input
            .0
            .lock()
            .map_err(|_| "cloud mailbox unavailable")?
            .resources = resources;
        self.input
            .0
            .lock()
            .map_err(|_| "cloud mailbox unavailable")?
            .ground = Some((sun.ground().clone(), sun.mapping().clone()));
        self.generation = surface.generation;
        let sky = self
            .sky
            .map(|profile| crate::layer::Layer::new(surface, &weather, &sun, profile))
            .transpose()
            .map_err(|e| e.to_string())?;
        self.gpu = Some(Prepared {
            sky,
            weather,
            sun,
            queue: surface.queue.clone(),
        });
        Ok(())
    }
    fn effect_graph(&self) -> EffectGraph {
        self.gpu.as_ref().map_or_else(EffectGraph::default, |gpu| {
            let mut graph = graph::declaration(gpu, self.generation);
            if let Some(sky) = &gpu.sky {
                sky.append_graph(&mut graph, self.generation);
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
            if let Some(sky) = &gpu.sky {
                sky.upload(&gpu.weather, &state, frame);
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
    fn computes(&self, _frame: &ContributionFrame) -> bool {
        false
    }
    fn encode_compute(&mut self, _pass: &mut wgpu::ComputePass<'_>, _frame: &ContributionFrame) {}
    fn encode_after_compute(&mut self, _encoder: &mut wgpu::CommandEncoder) {}
    fn draw(&self, _pass: &mut wgpu::RenderPass<'_>) {}
    fn encode_after_opaque(
        &mut self,
        _index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &ContributionFrame,
    ) {
        if self.valid_recording
            && let Some(sky) = self.gpu.as_ref().and_then(|gpu| gpu.sky.as_ref())
        {
            sky.encode(pass, frame);
        }
    }
    fn draw_after_opaque(
        &self,
        _index: usize,
        pass: &mut wgpu::RenderPass<'_>,
        _frame: &ContributionFrame,
    ) {
        if self.valid_recording
            && let Some(sky) = self.gpu.as_ref().and_then(|gpu| gpu.sky.as_ref())
        {
            sky.draw(pass);
        }
    }
    fn submitted(&mut self, _history_advanced: bool) {
        if self.valid_recording
            && let Some(gpu) = &mut self.gpu
        {
            gpu.weather.submitted();
            gpu.sun.submitted();
        }
    }
}
