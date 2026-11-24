//! The 2D scene half, composed with the same graph, SDR output and surface consumer.

use std::{collections::BTreeSet, sync::Arc};

use super::{
    SpriteFrame, SpriteFrameStats, SpriteScene, SpriteSink,
    pass::{self, SpriteBindings, SpritePass},
    world::SpriteWorld,
};
use crate::{
    capabilities::DeviceCapabilities,
    contribution::{
        ContributionFrame, ContributionPasses, EffectGraph, EffectTime, SceneContribution,
    },
    graph::{
        Access, AccessKind, CompiledGraph, CompiledPass, FrameGraph, GraphColorAttachment,
        GraphTextures, PassBody, PassKind, PassScope, PipelineKey, ResourceDescriptor, ResourceId,
        ResourceScope, TextureKind,
    },
    graph_record::{ExecutableGraph, RecordBinding, RecordFamily},
    pass_key::{PassKey, PipelineRole},
    present::{self, PresentationBinding, SurfacePresenter, SurfaceProfile},
    synthetic::{
        CaptureSource, PixelReadback,
        display::{DISPLAY_REFERRED_PASS, DisplayReferredStage},
    },
    timing::{GpuTimings, PassTiming, TimingCategory, TimingRecord},
};
use crate::{
    error::RenderError,
    render_config::{OUTPUT_FORMAT, SCENE_FORMAT},
};

pub(crate) struct PreparedSprites {
    pub(crate) capabilities: DeviceCapabilities,
    pub(crate) scene: SpriteScene,
    pub(crate) contribution_passes: ContributionPasses,
    pub(crate) effects: EffectGraph,
    sprites: SpritePass,
    world: SpriteWorld,
    ordering: std::time::Duration,
    tone: DisplayReferredStage,
}

pub(crate) struct SpriteTarget {
    pub(crate) extent: [u32; 2],
    pub(crate) source: CaptureSource,
    row_bytes: u32,
    sprite_rows_bytes: u64,
    sprite_order_bytes: u64,
    generation: u64,
    textures: GraphTextures,
    scene: ResourceId,
    sdr: ResourceId,
    scene_view: wgpu::TextureView,
    sdr_view: wgpu::TextureView,
    sprites: SpriteBindings,
    tone_input: wgpu::BindGroup,
    plain: ExecutableGraph<SpriteGraph>,
    capture: ExecutableGraph<SpriteGraph>,
    surface: Option<ResourceId>,
    camera_centre: [f64; 2],
    clip_scale: [f32; 2],
}

pub(crate) struct SpriteUpload {
    pub(crate) rows: u32,
    pub(crate) bytes: usize,
    pub(crate) dropped: u32,
    pub(crate) moving: u32,
    pub(crate) stats: SpriteFrameStats,
    pub(super) clip_scale: [f32; 2],
}

struct SpriteGraph;
impl RecordFamily for SpriteGraph {
    type Frame<'a> = RecordContext<'a>;
    type Error = RenderError;
}

struct RecordContext<'a> {
    scene: &'a PreparedSprites,
    target: &'a SpriteTarget,
    encoder: &'a mut wgpu::CommandEncoder,
    stats: Option<&'a mut SpriteFrameStats>,
    timings: Option<&'a mut GpuTimings>,
    readback: Option<&'a mut PixelReadback>,
    contribution: Option<&'a mut (dyn SceneContribution + 'static)>,
    frame: Option<ContributionFrame>,
    surface: Option<(
        &'a SurfacePresenter,
        &'a PresentationBinding,
        &'a wgpu::TextureView,
        [u32; 2],
    )>,
}

impl PreparedSprites {
    pub(crate) fn target_inputs(&self) -> Self {
        Self {
            capabilities: self.capabilities.clone(),
            scene: self.scene.clone(),
            contribution_passes: self.contribution_passes,
            effects: self.effects.clone(),
            sprites: self.sprites.target_inputs(),
            world: SpriteWorld::new(0),
            ordering: std::time::Duration::ZERO,
            tone: self.tone.clone(),
        }
    }

    pub(crate) async fn prepare(
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        scene: SpriteScene,
    ) -> Result<Self, RenderError> {
        capabilities.recipe().map_err(RenderError::Recipe)?;
        let device = capabilities.site();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let sprites = SpritePass::prepare(
            capabilities,
            queue,
            &scene,
            PipelineRole::Sprites.pipeline_key(capabilities.generation(), 0),
        )?;
        let tone = DisplayReferredStage::new(device);
        if let Some(error) = scope.pop().await {
            return Err(RenderError::Gpu(error));
        }
        Ok(Self {
            capabilities: capabilities.clone(),
            world: SpriteWorld::new(scene.world_capacity),
            ordering: std::time::Duration::ZERO,
            scene,
            contribution_passes: ContributionPasses::default(),
            effects: EffectGraph::default(),
            sprites,
            tone,
        })
    }

    pub(crate) fn clear_world_history(&mut self) {
        self.world.clear_history();
        self.sprites.forget_world_source();
    }

    pub(crate) fn inherit_world(&mut self, previous: &mut Self) {
        std::mem::swap(&mut self.world, &mut previous.world);
    }

    pub(crate) fn extract(
        &mut self,
        source: crate::extract::SourceTick,
        visit: impl FnOnce(&mut SpriteSink<'_>),
    ) -> std::time::Duration {
        if self.world.source() == Some(source) {
            return std::time::Duration::ZERO;
        }
        let start = crate::runtime::Instant::now();
        self.world.extract(&self.scene, source, visit);
        let extraction = start.elapsed();
        let order_start = crate::runtime::Instant::now();
        self.world.order_tick(self.scene.order_band);
        self.ordering = order_start.elapsed();
        extraction
    }

    pub(crate) fn upload(
        &mut self,
        queue: &wgpu::Queue,
        target: &mut SpriteTarget,
        frame: SpriteFrame<'_>,
        alpha: f32,
        measure_coverage: bool,
    ) -> Result<SpriteUpload, RenderError> {
        let mut upload = self.sprites.upload(
            queue,
            &self.scene,
            &self.world,
            frame,
            target.extent,
            alpha,
            measure_coverage,
            &mut target.sprites,
        )?;
        upload.stats.order_cpu += std::mem::take(&mut self.ordering);
        target.camera_centre = frame.view.centre;
        target.clip_scale = upload.clip_scale;
        Ok(upload)
    }

    pub(crate) fn record<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        target: &'a SpriteTarget,
        mut timings: Option<&'a mut GpuTimings>,
        readback: Option<&'a mut PixelReadback>,
        contribution: Option<&'a mut (dyn SceneContribution + 'static)>,
        stats: Option<&'a mut SpriteFrameStats>,
        time: EffectTime,
    ) -> Result<(), RenderError> {
        let frame = contribution
            .is_some()
            .then(|| target.contribution_frame(time))
            .transpose()?;
        let graph = target.graph(readback.is_some());
        if let Some(timings) = timings.as_deref_mut() {
            let count = u32::try_from(graph.plan().pipeline_pass_count_before_acquire())
                .map_err(|_| RenderError::GraphPassMismatch)?;
            timings.check_passes(target.generation, count)?;
        }
        graph.execute_before_acquire(&mut RecordContext {
            scene: self,
            target,
            encoder,
            stats,
            timings,
            readback,
            contribution,
            frame,
            surface: None,
        })
    }

    pub(crate) fn present(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &SpriteTarget,
        timings: Option<&mut GpuTimings>,
        readback: bool,
        surface: (
            &SurfacePresenter,
            &PresentationBinding,
            &wgpu::TextureView,
            [u32; 2],
        ),
    ) -> Result<(), RenderError> {
        target
            .graph(readback)
            .execute_after_acquire(&mut RecordContext {
                scene: self,
                target,
                encoder,
                stats: None,
                timings,
                readback: None,
                contribution: None,
                frame: None,
                surface: Some(surface),
            })
    }

    pub(crate) fn storage_report(&self) -> String {
        format!(
            "embedded sprite sheet {:?} {}x{}\nsprite frames bytes={}",
            self.sprites.sheets[0].0.format(),
            self.sprites.sheets[0].0.width(),
            self.sprites.sheets[0].0.height(),
            self.sprites.frames.size()
        )
    }
}

impl SpriteTarget {
    pub(crate) fn camera_relative_projection(&self) -> glam::Mat4 {
        let [sx, sy] = self.clip_scale;
        glam::Mat4::from_scale(glam::Vec3::new(sx, sy, 1.0))
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "the contribution interface uses f32; conversion checks the finite range"
    )]
    fn contribution_frame(&self, time: EffectTime) -> Result<ContributionFrame, RenderError> {
        let [x, y] = self.camera_centre;
        let [sx, sy] = self.clip_scale;
        let camera = glam::Vec3::new(x as f32, y as f32, 0.0);
        let translation = glam::Vec4::new(
            (-x * f64::from(sx)) as f32,
            (-y * f64::from(sy)) as f32,
            0.0,
            1.0,
        );
        if !camera.is_finite() || !translation.is_finite() {
            return Err(RenderError::InvalidView);
        }
        let mut projection = self.camera_relative_projection();
        projection.w_axis = translation;
        Ok(ContributionFrame {
            time,
            generation: self.generation,
            view_proj: projection.to_cols_array_2d(),
            render_extent: self.extent,
            camera: camera.to_array(),
            exposure_side: 0,
            environment: None,
            view_uniform_offset: 0,
            history_reset: false,
        })
    }

    pub(crate) fn prepare(
        prepared: &PreparedSprites,
        extent: [u32; 2],
        source: CaptureSource,
        surface_profile: Option<SurfaceProfile>,
    ) -> Result<Self, RenderError> {
        let capabilities = &prepared.capabilities;
        let device = capabilities.site();
        let generation = capabilities.generation();
        let row_bytes = extent[0]
            .checked_mul(source.bytes_per_pixel())
            .and_then(|value| value.checked_add(255))
            .map(|value| value / 256 * 256)
            .ok_or(RenderError::TargetExtent)?;
        let readback_bytes = u64::from(row_bytes) * u64::from(extent[1]) + 32;
        if !matches!(source, CaptureSource::Sdr | CaptureSource::SceneLinear) {
            return Err(RenderError::UnsupportedSpriteCapture);
        }
        if extent
            .into_iter()
            .any(|side| side == 0 || side > capabilities.limits().max_texture_dimension_2d)
            || readback_bytes > capabilities.limits().max_buffer_size
            || surface_profile.is_some_and(|profile| profile.extent != extent)
        {
            return Err(RenderError::TargetExtent);
        }

        let mut graph = FrameGraph::default();
        let scene = attachment(&mut graph, extent, SCENE_FORMAT)?;
        let sdr = attachment(&mut graph, extent, OUTPUT_FORMAT)?;
        let key = prepared.sprites.key;
        let tone = PipelineRole::Display.key(key);
        let mut keys = BTreeSet::from([key, tone]);
        let contributions = contribution_passes(prepared, &mut graph, &mut keys)?;
        let mut accesses = sprite_inputs(&mut graph, &prepared.sprites)?;
        accesses.push(store(&scene));
        let sprites = graph.declare(PassKey::Sprites, body(Some(key), accesses));
        for contribution in contributions {
            graph.dependency(contribution, sprites)?;
        }
        let tone_pass = graph.declare(
            PassKey::DisplayReferred,
            body(
                Some(tone),
                vec![
                    Access::new(scene.clone(), 0..1, AccessKind::SampledRead),
                    store(&sdr),
                ],
            ),
        );
        let readback_buffer = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: readback_bytes,
                usage: wgpu::BufferUsages::COPY_DST,
            },
            ResourceScope::Imported { initialized: false },
        )?;
        let checkpoint = if source == CaptureSource::SceneLinear {
            &scene
        } else {
            &sdr
        };
        let readback = body(
            None,
            vec![
                Access::new(checkpoint.clone(), 0..1, AccessKind::CopyRead),
                Access::new(readback_buffer, 0..readback_bytes, AccessKind::CopyWrite),
            ],
        );
        let surface = surface_profile
            .map(|profile| {
                keys.insert(present::pipeline_key(generation));
                present::surface_pass(&mut graph, &sdr, profile, generation)
            })
            .transpose()?;
        graph.validate_capabilities(capabilities)?;
        let plain =
            graph.compile_surface_variant(None, generation, &keys, None, surface.clone())?;
        let capture = graph.compile_surface_variant(
            Some((PassKey::Readback, readback)),
            generation,
            &keys,
            Some(tone_pass),
            surface.clone(),
        )?;
        let textures = GraphTextures::new(
            device,
            &capture,
            capabilities.recipe().map_err(RenderError::Recipe)?,
        );
        let make_view = |resource| {
            textures
                .texture(resource)
                .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()))
                .ok_or(RenderError::GraphPassMismatch)
        };
        let scene_view = make_view(&scene)?;
        let sdr_view = make_view(&sdr)?;
        let tone_input = prepared.tone.bind(device, &scene_view);
        Ok(Self {
            extent,
            source,
            row_bytes,
            generation,
            textures,
            scene,
            sdr,
            scene_view,
            sdr_view,
            sprite_rows_bytes: prepared.sprites.rows.size(),
            sprite_order_bytes: prepared.sprites.order.size(),
            sprites: prepared.sprites.bind(device),
            tone_input,
            plain: bind(plain)?,
            capture: bind(capture)?,
            surface: surface.map(|(surface, _)| surface),
            camera_centre: [0.0; 2],
            clip_scale: [1.0; 2],
        })
    }

    fn graph(&self, capture: bool) -> &ExecutableGraph<SpriteGraph> {
        if capture { &self.capture } else { &self.plain }
    }

    pub(crate) fn plan(&self, capture: bool) -> &CompiledGraph {
        self.graph(capture).plan()
    }

    pub(crate) fn output(&self) -> &wgpu::TextureView {
        &self.sdr_view
    }

    pub(crate) fn readback(&self, device: &wgpu::Device) -> PixelReadback {
        PixelReadback::new(
            device,
            self.generation,
            self.extent,
            self.source,
            self.row_bytes,
            "sprite pixel readback",
        )
    }

    pub(crate) fn allocation(&self) -> (u32, u64) {
        self.textures.allocation()
    }

    pub(crate) fn storage_report(&self) -> String {
        let pixels = u64::from(self.extent[0]) * u64::from(self.extent[1]);
        format!(
            "sprite colour {:?} bytes={}\nSDR output {:?} bytes={}\nsprite view bytes={}\nsprite rows bytes={}\nsprite order bytes={}\nunique textures/nominal bytes={:?}",
            self.scene_view.texture().format(),
            pixels * 8,
            self.sdr_view.texture().format(),
            pixels * 4,
            self.sprites.view.size(),
            self.sprite_rows_bytes,
            self.sprite_order_bytes,
            self.allocation()
        )
    }

    pub(crate) fn timing_metadata_capacity(&self) -> usize {
        self.plan(false)
            .pass_order
            .len()
            .max(self.plan(true).pass_order.len())
    }

    pub(crate) fn timing_metadata(
        &self,
        record: &mut TimingRecord,
        capture: bool,
        presented: bool,
    ) {
        let plan = self.plan(capture);
        let limit = if presented {
            usize::MAX
        } else {
            plan.pre_acquire_passes.unwrap_or(usize::MAX)
        };
        record.effect_passes(
            plan.pass_bindings()
                .filter(|(_, pass)| matches!(pass.key, Some(PassKey::Effect(_))))
                .map(|(_, pass)| Arc::clone(&pass.name)),
        );
        record.unmeasured(
            plan.pass_bindings()
                .take(limit)
                .filter(|(_, pass)| pass.kind == Some(PassKind::Copy))
                .map(|(_, pass)| Arc::clone(&pass.name)),
        );
    }
}

pub(crate) fn validate_sprite_timings(
    passes: &[PassTiming],
    presented: bool,
) -> Option<(Option<f64>, Option<f64>)> {
    let get = |name| {
        passes
            .iter()
            .find(|pass| pass.name.as_ref() == name)
            .map(|pass| pass.nanoseconds)
    };
    get(pass::PASS)?;
    get(DISPLAY_REFERRED_PASS)?;
    let present = passes
        .iter()
        .find(|pass| pass.category == TimingCategory::Surface)
        .map(|pass| pass.nanoseconds);
    (present.is_some() == presented).then(|| (present, get(crate::synthetic::CONTRIBUTION_COMPUTE)))
}

fn sprite_inputs(graph: &mut FrameGraph, sprites: &SpritePass) -> Result<Vec<Access>, RenderError> {
    let mut accesses = Vec::with_capacity(4 + sprites.sheets.len());
    for (bytes, usage, kind) in [
        (
            pass::VIEW_BYTES,
            wgpu::BufferUsages::UNIFORM,
            AccessKind::UniformRead,
        ),
        (
            sprites.rows.size(),
            wgpu::BufferUsages::STORAGE,
            AccessKind::StorageRead,
        ),
        (
            sprites.order.size(),
            wgpu::BufferUsages::STORAGE,
            AccessKind::StorageRead,
        ),
        (
            sprites.frames.size(),
            wgpu::BufferUsages::STORAGE,
            AccessKind::StorageRead,
        ),
    ] {
        let id = graph.resource(
            ResourceDescriptor::Buffer { bytes, usage },
            ResourceScope::Imported { initialized: true },
        )?;
        accesses.push(Access::new(id, 0..bytes, kind));
    }
    for (sheet, _) in &sprites.sheets {
        let id = graph.resource(
            ResourceDescriptor::of_texture(sheet),
            ResourceScope::Imported { initialized: true },
        )?;
        accesses.push(Access::new(id, 0..1, AccessKind::SampledRead));
    }
    Ok(accesses)
}

fn contribution_passes(
    prepared: &PreparedSprites,
    graph: &mut FrameGraph,
    keys: &mut BTreeSet<PipelineKey>,
) -> Result<Vec<usize>, RenderError> {
    let mut ordered = Vec::new();
    if prepared.contribution_passes.upload {
        // The contribution owns the actual copies. This imported token declares the copy stage.
        let token = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: 4,
                usage: wgpu::BufferUsages::COPY_DST,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        ordered.push(graph.declare(
            PassKey::ContributionUpload,
            body(None, vec![Access::new(token, 0..4, AccessKind::CopyWrite)]),
        ));
    }
    if prepared.contribution_passes.compute {
        let key = PipelineRole::ContributionCompute.key(prepared.sprites.key);
        keys.insert(key);
        ordered.push(graph.declare(PassKey::ContributionCompute, body(Some(key), Vec::new())));
    }
    let effects = prepared.effects.append(graph, prepared.sprites.key, keys)?;
    if let Some(&last) = ordered.last() {
        for &simulate in &effects.simulate {
            graph.dependency(last, simulate)?;
        }
    }
    for pair in ordered.windows(2) {
        graph.dependency(pair[0], pair[1])?;
    }
    ordered.extend(effects.simulate);
    Ok(ordered)
}

fn attachment(
    graph: &mut FrameGraph,
    [width, height]: [u32; 2],
    format: wgpu::TextureFormat,
) -> Result<ResourceId, RenderError> {
    Ok(graph.resource(
        ResourceDescriptor::Texture {
            dimension: TextureKind::D2,
            width,
            height,
            layers: 1,
            mips: 1,
            samples: 1,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
        },
        ResourceScope::Transient,
    )?)
}

fn store(id: &ResourceId) -> Access {
    Access::new(id.clone(), 0..1, AccessKind::AttachmentClear)
}

fn body(pipeline: Option<PipelineKey>, accesses: Vec<Access>) -> PassBody {
    PassBody {
        scope: PassScope::View(1),
        pipeline,
        accesses,
        external_side_effect: true,
    }
}

fn bind(plan: CompiledGraph) -> Result<ExecutableGraph<SpriteGraph>, RenderError> {
    let bindings = plan
        .pass_bindings()
        .map(|(id, pass)| {
            type Callback = fn(&mut RecordContext<'_>, &CompiledPass) -> Result<(), RenderError>;
            let key = pass.key.ok_or(RenderError::GraphPassMismatch)?;
            let (kind, record): (PassKind, Callback) = match key {
                PassKey::Sprites => (PassKind::Render, record_sprites),
                PassKey::DisplayReferred => (PassKind::Render, record_tone),
                PassKey::Surface => (PassKind::Render, record_present),
                PassKey::Readback => (PassKind::Copy, record_capture),
                PassKey::ContributionUpload => (PassKind::Copy, record_upload),
                PassKey::ContributionCompute => (PassKind::Compute, record_compute),
                PassKey::Effect(index) => {
                    let index = index as usize;
                    return Ok(RecordBinding::new(
                        id,
                        PassKind::Compute,
                        move |frame, pass| record_effect(frame, index, pass),
                    ));
                }
                // 3D scene passes. Every key is listed, so a new key does not
                // compile until it is bound here or refused.
                PassKey::Opaque
                | PassKey::Display
                | PassKey::ShadowNear
                | PassKey::ShadowFar
                | PassKey::HorizonHeights
                | PassKey::AirShadow
                | PassKey::CasterCull
                | PassKey::Cluster
                | PassKey::History
                | PassKey::Exposure
                | PassKey::Stars
                | PassKey::Atmosphere(_)
                | PassKey::CameraVisibility
                | PassKey::PrepassList
                | PassKey::DepthPrepass
                | PassKey::DepthPyramid
                | PassKey::FinalCull
                | PassKey::CullingStats
                | PassKey::TemporalExposure
                | PassKey::TemporalWitness
                | PassKey::Taau
                | PassKey::AfterOpaque(_)
                | PassKey::AfterOpaqueCopy
                | PassKey::Secondary { .. }
                | PassKey::RayStructureBuild
                | PassKey::NativeReset
                | PassKey::SoftSun(_)
                | PassKey::NativeWitness => return Err(RenderError::GraphPassMismatch),
            };
            Ok(RecordBinding::new(id, kind, record))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(plan.bind(bindings)?)
}

fn record_sprites(frame: &mut RecordContext<'_>, pass: &CompiledPass) -> Result<(), RenderError> {
    let draws = frame.scene.sprites.draw(
        frame.encoder,
        pass,
        &frame.target.scene,
        &frame.target.scene_view,
        &frame.target.sprites,
        frame.timings.as_deref_mut(),
    )?;
    if let Some(stats) = frame.stats.as_deref_mut() {
        stats.draws += draws;
    }
    Ok(())
}

fn record_tone(frame: &mut RecordContext<'_>, pass: &CompiledPass) -> Result<(), RenderError> {
    frame.scene.tone.encode(
        frame.encoder,
        frame.target.generation,
        pass,
        GraphColorAttachment {
            resource: &frame.target.sdr,
            range: 0..1,
            view: &frame.target.sdr_view,
            clear: wgpu::Color::BLACK,
        },
        &frame.target.tone_input,
        frame.timings.as_deref_mut(),
    )
}

fn record_capture(frame: &mut RecordContext<'_>, _: &CompiledPass) -> Result<(), RenderError> {
    let readback = frame.readback.as_deref_mut().ok_or(RenderError::Readback)?;
    let view = if frame.target.source == CaptureSource::SceneLinear {
        &frame.target.scene_view
    } else {
        &frame.target.sdr_view
    };
    readback.copy_texture(frame.encoder, view.texture(), None)
}

fn record_present(frame: &mut RecordContext<'_>, pass: &CompiledPass) -> Result<(), RenderError> {
    let (presenter, binding, view, extent) = frame.surface.ok_or(RenderError::GraphPassMismatch)?;
    let resource = frame
        .target
        .surface
        .as_ref()
        .ok_or(RenderError::GraphPassMismatch)?;
    presenter
        .encode_compiled(
            frame.encoder,
            binding,
            view,
            extent,
            frame.timings.as_deref_mut(),
            pass,
            resource,
        )
        .map_err(RenderError::Present)
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "all graph callbacks share one signature"
)]
fn record_upload(frame: &mut RecordContext<'_>, _: &CompiledPass) -> Result<(), RenderError> {
    if let Some((entry, time)) = frame.contribution.as_deref_mut().zip(frame.frame.as_ref()) {
        entry.encode_upload(frame.encoder, time);
    }
    Ok(())
}

fn record_compute(frame: &mut RecordContext<'_>, pass: &CompiledPass) -> Result<(), RenderError> {
    if let Some((entry, time)) = frame.contribution.as_deref_mut().zip(frame.frame.as_ref()) {
        if entry.computes(time) {
            let mut compute = pass
                .begin_compute(
                    frame.encoder,
                    frame.target.generation,
                    frame.timings.as_deref_mut(),
                )?
                .ok_or(RenderError::GraphPassMismatch)?;
            entry.encode_compute(&mut compute, time);
        }
        entry.encode_after_compute(frame.encoder);
    }
    Ok(())
}

fn record_effect(
    frame: &mut RecordContext<'_>,
    index: usize,
    pass: &CompiledPass,
) -> Result<(), RenderError> {
    if let Some((entry, time)) = frame.contribution.as_deref_mut().zip(frame.frame.as_ref())
        && !time.time.capture
        && entry.effect_runs(index, time)
    {
        let mut compute = pass
            .begin_compute(
                frame.encoder,
                frame.target.generation,
                frame.timings.as_deref_mut(),
            )?
            .ok_or(RenderError::GraphPassMismatch)?;
        entry.encode_effect(index, &mut compute, time);
        drop(compute);
        entry.encode_effect_readback(index, frame.encoder);
    }
    Ok(())
}
