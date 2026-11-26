use super::{
    EffectGraph, FrameError, Prepared, SceneSurface, Shade, Shapes, SunPath, WeatherMap,
    frame_prepare_error, graph,
};
use crate::{
    Error,
    ground::{GroundGpu, GroundTargets},
    layer::Layer,
    sun_path::SunTarget,
};
use std::sync::PoisonError;

enum Resources {
    New(Box<Prepared>),
    Retained {
        sun: SunTarget,
        sky: Option<Box<Layer>>,
        ground: Option<Box<GroundTargets>>,
    },
}

pub(super) const MISSING_TARGET: FrameError =
    FrameError::InvalidState("cloud Shade requires the scene's GPU Sun-path target");

pub(super) struct Staged {
    resources: Resources,
    generation: u64,
    pub(super) initialization: Vec<wgpu::CommandBuffer>,
    bytes: Vec<(String, u64)>,
}

impl Shade {
    pub(super) fn stage(&self, surface: &SceneSurface<'_>) -> Result<Staged, FrameError> {
        let target = surface.sun_path_target().ok_or(MISSING_TARGET)?;
        // Check access during preparation. Publication updates only prepared
        // handles and keeps any weather lease offered in the meantime.
        drop(
            self.input
                .0
                .lock()
                .map_err(|_| frame_prepare_error(&Error::InputUnavailable))?,
        );
        let mut initialization = Vec::new();
        let (resources, bytes) = if let Some(gpu) = self
            .gpu
            .as_ref()
            .filter(|_| self.generation == surface.generation)
        {
            let sun = gpu
                .sun
                .prepare_target(surface.device, target)
                .map_err(|error| frame_prepare_error(&error))?;
            let ground = gpu
                .ground
                .as_ref()
                .map(|ground| {
                    GroundTargets::new(
                        surface,
                        &gpu.weather,
                        &gpu.sun,
                        &sun.mapping,
                        &ground.field,
                        &ground.phases,
                    )
                    .map(Box::new)
                    .map_err(|error| frame_prepare_error(&error))
                })
                .transpose()?;
            let sky = self.prepare_sky(
                surface,
                &gpu.weather,
                &gpu.sun,
                (&sun.ground, &sun.mapping),
                gpu.placed_map.as_ref(),
                gpu.shapes.as_deref(),
                gpu.sky.as_ref(),
                gpu.ground
                    .as_ref()
                    .zip(ground.as_deref())
                    .map(|(gpu, targets)| (&gpu.field, &gpu.phases, &targets.band)),
                &mut initialization,
            )?;
            let bytes =
                gpu.resource_bytes(sky.as_ref(), &sun.ground, &sun.mapping, ground.as_deref());
            (
                Resources::Retained {
                    sun,
                    sky: sky.map(Box::new),
                    ground,
                },
                bytes,
            )
        } else {
            let gpu = self.prepare_new(surface, target, &mut initialization)?;
            let bytes = gpu.resource_bytes(
                gpu.sky.as_ref(),
                gpu.sun.ground(),
                gpu.sun.mapping(),
                gpu.ground.as_ref().map(|ground| &ground.targets),
            );
            (Resources::New(Box::new(gpu)), bytes)
        };
        Ok(Staged {
            resources,
            generation: surface.generation,
            initialization,
            bytes,
        })
    }

    fn prepare_new(
        &self,
        surface: &SceneSurface<'_>,
        target: (wgpu::Texture, wgpu::Buffer),
        initialization: &mut Vec<wgpu::CommandBuffer>,
    ) -> Result<Prepared, FrameError> {
        let device = surface.device;
        let mut weather = WeatherMap::new(
            device,
            self.config.seed,
            self.config.model,
            self.config.types.clone(),
            self.config.weather.clone(),
        )
        .map_err(|error| frame_prepare_error(&error))?;
        weather.rebuild(surface.generation, 1);
        let shapes = self
            .shapes
            .clone()
            .map(|config| Shapes::new(device, config, &weather).map(Box::new))
            .transpose()
            .map_err(|error| frame_prepare_error(&error))?;
        let placed_map = shapes
            .as_ref()
            .map(|shapes| crate::shapes::map::ShapeMap::new(device, weather.profile(), shapes));
        let mut sun = SunPath::new_with_shapes(
            device,
            &weather,
            self.config.sun.clone(),
            Some(target),
            shapes.as_deref(),
        )
        .map_err(|error| frame_prepare_error(&error))?;
        sun.rebuild(surface.generation, 1);
        initialization.extend(sun.take_initialization());
        let ground = self
            .ground
            .as_ref()
            .map(|config| {
                GroundGpu::new(surface, config, &weather, &sun)
                    .map_err(|error| frame_prepare_error(&error))
            })
            .transpose()?;
        let sky = self.prepare_sky(
            surface,
            &weather,
            &sun,
            (sun.ground(), sun.mapping()),
            placed_map.as_ref(),
            shapes.as_deref(),
            None,
            ground
                .as_ref()
                .map(|gpu| (&gpu.field, &gpu.phases, &gpu.targets.band)),
            initialization,
        )?;
        Ok(Prepared {
            shapes,
            placed_map,
            recorded_placed_state: None,
            sky,
            weather,
            sun,
            ground,
            queue: surface.queue.clone(),
            #[cfg(target_arch = "wasm32")]
            device: wgpu::Device::clone(device),
            #[cfg(target_arch = "wasm32")]
            ground_reference: None,
        })
    }

    fn prepare_sky(
        &self,
        surface: &SceneSurface<'_>,
        weather: &WeatherMap,
        sun: &SunPath,
        target: (&wgpu::Texture, &wgpu::Buffer),
        placed_map: Option<&crate::shapes::map::ShapeMap>,
        shapes: Option<&Shapes>,
        accepted: Option<&Layer>,
        ground: Option<(
            &crate::ground::GroundField,
            &crate::ground::lighting::LightTable,
            &crate::ground::band::Band,
        )>,
        initialization: &mut Vec<wgpu::CommandBuffer>,
    ) -> Result<Option<Layer>, FrameError> {
        self.sky
            .map(|profile| {
                let (sky, command) = Layer::new(
                    surface,
                    weather,
                    sun,
                    target,
                    placed_map.map(|map| &map.texture),
                    profile,
                    shapes,
                    accepted,
                    ground,
                )
                .map_err(|error| frame_prepare_error(&error))?;
                initialization.extend(command);
                Ok(sky)
            })
            .transpose()
    }
}

impl Staged {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn graph(&self, shade: &Shade) -> EffectGraph {
        match &self.resources {
            Resources::New(gpu) => gpu.declaration(
                self.generation,
                gpu.sky.as_ref(),
                gpu.sun.ground(),
                gpu.sun.mapping(),
                gpu.ground.as_ref().map(|ground| &ground.targets),
            ),
            Resources::Retained { sun, sky, ground } => shade
                .gpu
                .as_ref()
                .expect("accepted cloud resources")
                .declaration(
                    self.generation,
                    sky.as_deref(),
                    &sun.ground,
                    &sun.mapping,
                    ground.as_deref(),
                ),
        }
    }

    pub(super) fn install(self, shade: &mut Shade) {
        match self.resources {
            Resources::New(gpu) => shade.gpu = Some(gpu),
            Resources::Retained {
                sun,
                mut sky,
                ground,
            } => {
                let gpu = shade.gpu.as_mut().expect("accepted cloud resources");
                gpu.sun.install_target(sun);
                if let Some((accepted, prepared)) = gpu.ground.as_mut().zip(ground) {
                    accepted.targets = *prepared;
                }

                if let Some((sky, old)) = sky.as_mut().zip(gpu.sky.as_mut()) {
                    sky.inherit_history(old);
                }
                gpu.sky = sky.map(|sky| *sky);
            }
        }
        shade.generation = self.generation;
        shade.apply_quality();
        let gpu = shade.gpu.as_ref().expect("installed cloud resources");
        let mut mailbox = shade.input.0.lock().unwrap_or_else(PoisonError::into_inner);
        mailbox.resources = self.bytes;
        mailbox.weather_columns = Some(gpu.weather.textures()[0].clone());
        mailbox.ground = Some((gpu.sun.ground().clone(), gpu.sun.mapping().clone()));
        mailbox.condensate = gpu.ground.as_ref().map(|ground| crate::GroundProducts {
            band: ground.targets.band.texture.clone(),
            medium: ground.targets.medium.target.clone(),
        });
    }
}

impl Prepared {
    pub(super) fn declaration(
        &self,
        generation: u64,
        sky: Option<&Layer>,
        ground: &wgpu::Texture,
        mapping: &wgpu::Buffer,
        ground_targets: Option<&GroundTargets>,
    ) -> EffectGraph {
        let (mut graph, shape_noise) = graph::declaration(self, generation, ground, mapping);
        let ground_reads = self
            .ground
            .as_ref()
            .zip(ground_targets)
            .map(|(ground, targets)| graph::append_ground(&mut graph, ground, targets));
        let overlay_index = graph.passes.len();
        if let Some(sky) = sky {
            sky.append_graph(
                &mut graph,
                generation,
                self.sun.shape_noise.read(shape_noise),
            );
        }
        if let Some(shapes) = &self.shapes {
            let reads = shapes.append_reads(&mut graph);
            for pass in &mut graph.passes[1..4] {
                pass.accesses.extend(reads.iter().cloned());
            }
            if let Some(trace) = graph
                .after_opaque
                .iter_mut()
                .find(|pass| pass.name == crate::layer::TRACE)
            {
                trace.accesses.extend(reads.iter().cloned());
            }
            if sky.is_some_and(Layer::has_overlay) {
                graph.passes[overlay_index]
                    .accesses
                    .extend(reads.iter().cloned());
            }
            if let Some(map) = &self.placed_map {
                let resource = graph.resources.len();
                graph::append_placed_map(&mut graph, map, reads, generation);
                if let Some(field) = graph
                    .after_opaque
                    .iter_mut()
                    .find(|pass| pass.name == crate::layer::FIELD)
                {
                    field
                        .accesses
                        .push(thrum_render::contribution::EffectAccess {
                            resource,
                            range: 0..4,
                            kind: thrum_render::graph::AccessKind::SampledRead,
                            history: None,
                            view: None,
                        });
                }
            }
        }
        if let Some(reads) = ground_reads {
            if sky.is_some_and(Layer::has_overlay) {
                graph.passes[overlay_index]
                    .accesses
                    .extend(reads.iter().cloned());
            }
            if let Some(trace) = graph
                .after_opaque
                .iter_mut()
                .find(|pass| pass.name == crate::layer::TRACE)
            {
                trace.accesses.extend(reads);
            }
        }
        graph
    }

    fn resource_bytes(
        &self,
        sky: Option<&Layer>,
        ground: &wgpu::Texture,
        mapping: &wgpu::Buffer,
        ground_targets: Option<&GroundTargets>,
    ) -> Vec<(String, u64)> {
        let mut resources: Vec<_> = self
            .weather
            .textures()
            .iter()
            .chain(self.sun.products())
            .chain(std::iter::once(ground))
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
        resources.push(("cloud shape noise".into(), self.sun.shape_noise.bytes()));
        if let Some(map) = &self.placed_map {
            resources.extend([
                (
                    "cloud placed weather map".into(),
                    u64::from(map.texture.width()) * u64::from(map.texture.height()) * 64,
                ),
                ("cloud placed weather windows".into(), map.params.size()),
            ]);
        }
        resources.extend(
            self.weather
                .uniforms()
                .iter()
                .chain(self.sun.uniforms())
                .chain(self.weather.axes())
                .chain(std::iter::once(mapping))
                .enumerate()
                .map(|(i, buffer)| (format!("cloud buffer {i}"), buffer.size())),
        );
        if let Some((ground, targets)) = self.ground.as_ref().zip(ground_targets) {
            resources.extend(graph::ground_bytes(ground, targets));
        }
        if let Some(sky) = sky {
            resources.extend(sky.resource_bytes());
        }
        resources
    }
}
