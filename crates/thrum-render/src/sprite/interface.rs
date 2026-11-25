//! The sprite pass's output-pixel use after the display transform.

use std::{cell::Cell, collections::BTreeSet, sync::Arc, time::Duration};

use super::{
    NamedSheet, SpriteRow, SpriteScene, SpriteView,
    pass::{SpriteBindings, SpritePass, VIEW_BYTES},
};
use crate::{
    capabilities::DeviceCapabilities,
    error::RenderError,
    graph::{
        Access, AccessKind, CompiledPass, FrameGraph, GraphColorAttachment, PassBody, PassScope,
        PipelineKey, ResourceDescriptor, ResourceId, ResourceScope,
    },
    pass_key::{PassKey, PipelineRole},
    render_config::SCENE_FORMAT,
    runtime::Instant,
    timing::GpuTimings,
};

pub(crate) const PASS: &str = PassKey::Interface.constant_name();
pub(crate) const SDR_VIEW: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Embedded sheets in paint-list slot order and the interface's paint capacity.
#[derive(Clone, Debug)]
pub struct InterfaceConfig {
    /// Named sheets used by the paint list.
    pub sheets: Vec<NamedSheet>,
    /// Maximum number of rows considered from each paint list.
    pub row_capacity: u32,
}

/// Immutable sprite rows in paint order, with positions in output pixels from the top left.
#[derive(Clone, Debug)]
pub struct PaintList {
    /// Output extent used to lay out these rows.
    pub extent: [u32; 2],
    /// The first row is drawn first. Sprite layer, sort, flash and additive effects are unused.
    pub rows: Arc<[SpriteRow]>,
}

/// Work recorded by the interface stage for a frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct InterfaceFrameStats {
    /// Row and view bytes written to the queue, excluding preparation uploads.
    pub uploaded_bytes: usize,
    /// Rows dropped for invalid input or capacity.
    pub dropped_rows: u32,
    /// The paint list was laid out for a different extent.
    pub extent_mismatch: bool,
    /// Time spent preparing the stage's uploads for the frame.
    pub upload_cpu: Duration,
    /// CPU time to encode the interface pass or append its draw.
    pub encode_cpu: Duration,
}

/// Descriptors retained by target preparation without retaining GPU state.
#[derive(Clone)]
pub(crate) struct SpriteInputs(Vec<(ResourceDescriptor, AccessKind, u64)>);

impl SpriteInputs {
    pub(super) fn new(sprites: &SpritePass) -> Self {
        let buffers = [
            (
                VIEW_BYTES,
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
        ];
        let mut resources = Vec::with_capacity(buffers.len() + sprites.sheets.len());
        resources.extend(buffers.map(|(bytes, usage, kind)| {
            (ResourceDescriptor::Buffer { bytes, usage }, kind, bytes)
        }));
        resources.extend(sprites.sheets.iter().map(|(sheet, _)| {
            (
                ResourceDescriptor::of_texture(sheet),
                AccessKind::SampledRead,
                1,
            )
        }));
        Self(resources)
    }

    pub(crate) fn reads(&self, graph: &mut FrameGraph) -> Result<Vec<Access>, RenderError> {
        self.0
            .iter()
            .map(|(descriptor, kind, end)| {
                let id = graph.resource(
                    descriptor.clone(),
                    ResourceScope::Imported { initialized: true },
                )?;
                Ok(Access::new(id, 0..*end, *kind))
            })
            .collect()
    }
}

pub(crate) fn append_sdr(
    graph: &mut FrameGraph,
    keys: &mut BTreeSet<PipelineKey>,
    output: &ResourceId,
    reads: Vec<Access>,
    generation: u64,
    display: usize,
    required: bool,
) -> Result<usize, RenderError> {
    let key = PipelineRole::Interface.pipeline_key(generation, 0);
    keys.insert(key);
    let mut accesses = reads;
    accesses.push(Access::new(
        output.clone(),
        0..1,
        AccessKind::AttachmentLoad,
    ));
    let pass = graph.declare(
        PassKey::Interface,
        PassBody {
            scope: PassScope::View(1),
            pipeline: Some(key),
            accesses,
            external_side_effect: required,
        },
    );
    graph.dependency(display, pass)?;
    Ok(pass)
}

pub(crate) fn sdr_view(texture: &wgpu::Texture) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some(PASS),
        format: Some(SDR_VIEW),
        ..Default::default()
    })
}

struct FloatStage {
    pipeline: wgpu::RenderPipeline,
    bindings: SpriteBindings,
    extent: Option<[u32; 2]>,
}

pub(crate) struct PreparedInterface {
    scene: SpriteScene,
    sprites: SpritePass,
    bindings: SpriteBindings,
    float: Option<FloatStage>,
    paint_list: Option<Arc<[SpriteRow]>>,
    sdr_extent: Option<[u32; 2]>,
    dropped_rows: u32,
    encode_cpu: Cell<Duration>,
}

impl PreparedInterface {
    pub(crate) async fn prepare(
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        config: &InterfaceConfig,
    ) -> Result<Self, RenderError> {
        let mut scene = SpriteScene::empty(config.row_capacity);
        scene.sheets.clone_from(&config.sheets);
        let device = capabilities.site();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let sprites = SpritePass::prepare(
            capabilities,
            queue,
            &scene,
            PipelineRole::Interface.pipeline_key(capabilities.generation(), 0),
            SDR_VIEW,
            PASS,
        );
        let result = sprites.map(|sprites| {
            sprites.initialize_paint_order(queue);
            let bindings = sprites.bind(device);
            let float = (cfg!(target_os = "macos")
                && capabilities.adapter().backend == wgpu::Backend::Metal)
                .then(|| {
                    let pipeline = SpritePass::pipeline(device, SCENE_FORMAT, PASS);
                    let bindings = sprites.bind_for_pipeline(device, &pipeline);
                    FloatStage {
                        pipeline,
                        bindings,
                        extent: None,
                    }
                });
            Self {
                scene,
                sprites,
                bindings,
                float,
                paint_list: None,
                sdr_extent: None,
                dropped_rows: 0,
                encode_cpu: Cell::new(Duration::ZERO),
            }
        });
        if let Some(error) = scope.pop().await {
            return Err(RenderError::Gpu(error));
        }
        result
    }

    pub(crate) fn config(&self) -> InterfaceConfig {
        InterfaceConfig {
            sheets: self.scene.sheets.clone(),
            row_capacity: self.scene.row_capacity,
        }
    }

    pub(crate) fn inputs(&self) -> SpriteInputs {
        SpriteInputs::new(&self.sprites)
    }

    pub(crate) fn encode_sdr(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        compiled: &CompiledPass,
        output: GraphColorAttachment<'_>,
        timings: Option<&mut GpuTimings>,
    ) -> Result<(), RenderError> {
        let start = Instant::now();
        let extent = [
            output.view.texture().width(),
            output.view.texture().height(),
        ];
        let mut pass = compiled
            .begin_render(
                encoder,
                [Some(output)],
                None,
                self.sprites.key.device_generation,
                timings,
            )?
            .ok_or(RenderError::GraphPassMismatch)?;
        self.draw(&mut pass, extent, false);
        drop(pass);
        self.record_encode(start);
        Ok(())
    }

    pub(crate) fn upload(
        &mut self,
        queue: &wgpu::Queue,
        paint: Option<&PaintList>,
        sdr_extent: Option<[u32; 2]>,
        float_extent: Option<[u32; 2]>,
    ) -> InterfaceFrameStats {
        let start = Instant::now();
        self.encode_cpu.set(Duration::ZERO);
        let Some(paint) = paint else {
            self.paint_list = None;
            self.bindings.count = 0;
            self.dropped_rows = 0;
            return InterfaceFrameStats {
                upload_cpu: start.elapsed(),
                ..InterfaceFrameStats::default()
            };
        };
        let mut uploaded_bytes = 0;
        if self
            .paint_list
            .as_ref()
            .is_none_or(|previous| !Arc::ptr_eq(previous, &paint.rows))
        {
            (uploaded_bytes, self.dropped_rows) =
                self.sprites
                    .upload_paint_rows(queue, &self.scene, &paint.rows, &mut self.bindings);
            self.paint_list = Some(Arc::clone(&paint.rows));
        }
        if self.bindings.count > 0 {
            uploaded_bytes += update_view(queue, &self.bindings, &mut self.sdr_extent, sdr_extent);
            if let Some(float) = &mut self.float {
                uploaded_bytes +=
                    update_view(queue, &float.bindings, &mut float.extent, float_extent);
            }
        }
        InterfaceFrameStats {
            uploaded_bytes,
            dropped_rows: self.dropped_rows,
            extent_mismatch: Some(paint.extent) != float_extent.or(sdr_extent),
            upload_cpu: start.elapsed(),
            encode_cpu: Duration::ZERO,
        }
    }

    pub(crate) fn encode_cpu(&self) -> Duration {
        self.encode_cpu.get()
    }

    fn record_encode(&self, start: Instant) {
        self.encode_cpu.set(self.encode_cpu.get() + start.elapsed());
    }

    pub(crate) fn draw_float(&self, pass: &mut wgpu::RenderPass<'_>, extent: [u32; 2]) {
        if self.bindings.count != 0 {
            let start = Instant::now();
            self.draw(pass, extent, true);
            self.record_encode(start);
        }
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>, extent: [u32; 2], float: bool) {
        if self.bindings.count == 0 {
            return;
        }
        let (pipeline, group) = if float {
            let float = self
                .float
                .as_ref()
                .expect("prepared float interface target");
            (&float.pipeline, &float.bindings.group)
        } else {
            (&self.sprites.pipeline, &self.bindings.group)
        };
        let [width, height] = extent;
        pass.set_viewport(0.0, 0.0, width as f32, height as f32, 0.0, 1.0);
        pass.set_scissor_rect(0, 0, width, height);
        SpritePass::draw_rows(pass, pipeline, group, self.bindings.count);
    }
}

fn update_view(
    queue: &wgpu::Queue,
    bindings: &SpriteBindings,
    cached_extent: &mut Option<[u32; 2]>,
    extent: Option<[u32; 2]>,
) -> usize {
    let Some([width, height]) = extent.filter(|&extent| Some(extent) != *cached_extent) else {
        return 0;
    };
    let view = SpriteView {
        centre: [f64::from(width) / 2.0, -f64::from(height) / 2.0],
        pixels_per_unit: 1.0,
    };
    let scale = view
        .clip_scale([width, height])
        .expect("validated output extent");
    SpritePass::write_view(queue, bindings, view, scale, 1.0);
    *cached_extent = extent;
    usize::try_from(VIEW_BYTES).expect("small sprite view")
}
