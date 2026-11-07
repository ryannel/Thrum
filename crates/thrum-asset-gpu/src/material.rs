//! A game-authored WGSL material hook for cooked meshes, replaced by explicit reloads.
//!
//! The hook is one WGSL function:
//!
//! ```wgsl
//! fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
//!     var out = surface;
//!     out.roughness = mix(surface.roughness, 0.95, smoothstep(0.6, 0.9, at.normal.y));
//!     return out;
//! }
//! ```
//!
//! `CookedSurface` holds the surface factors (base colour, roughness, metalness, emission) and
//! `CookedSurfacePoint` where the fragment is shaded (`uv`, `world`, `normal`). The scene's own
//! fragment shading reads the factors the hook returns, so lighting keeps one authority; nothing
//! else of the material record, the bindings or the cull and shadow pipelines changes. Without a
//! hook the cooked mesh pipeline compiles exactly the source it always has.
//!
//! [`MaterialReload::request`] prepares a replacement mesh pipeline outside frame recording. Natively
//! one compile thread runs at a time per handle, and only the newest request waits behind it. In
//! the browser, which has no threads, the request creates the module and pipeline in the caller's
//! task through WebGPU's synchronous methods: wgpu 30.0.1 does not expose
//! `createRenderPipelineAsync`, so a cold compile stalls the page for hundreds of milliseconds.
//! The reload is a development tool and stays out of ordinary play. The candidate becomes eligible only once
//! its compilation messages and its validation error scope have both resolved. The next frame
//! boundary draws it, and the submission carrying that frame promotes it: it becomes the accepted
//! hook, the meshes restart motion history once, and resident products and bindings are kept.
//! A rejected candidate leaves the previous pipeline drawing, with diagnostics translated to the
//! hook's own file, line and column. A newer request supersedes an older one, and a candidate
//! prepared for a retired device or scene surface is dropped. A rebuilt device or surface draws
//! the built-in material until the last accepted hook has compiled and validated for it again
//! (natively, the next frame), then draws the hook; a hook that no longer validates is dropped
//! with its diagnostics.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use thrum_render::contribution::SceneSurface;

/// The scene shader's one read of a fragment's material record, routed through the hook glue.
const ANCHOR: &str = "materials[input.material]";
const HOOKED: &str = "cooked_hooked_material(materials[input.material], input)";

/// WGSL source of a game's material hook and the name its diagnostics use, such as its path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialHook {
    /// Shown in diagnostics as the hook's file.
    pub name: String,
    /// WGSL defining `cooked_surface`; see the [module](self) documentation.
    pub source: String,
}

impl MaterialHook {
    /// Read a hook from a WGSL file, named by its path.
    ///
    /// # Errors
    /// The file cannot be read as UTF-8 text.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read(path: &std::path::Path) -> Result<Self, String> {
        std::fs::read_to_string(path)
            .map(|source| Self {
                name: path.display().to_string(),
                source,
            })
            .map_err(|error| format!("{}: {error}", path.display()))
    }
}

/// One compiler or validation message, located in the hook's own source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialDiagnostic {
    /// The hook's [`MaterialHook::name`].
    pub file: String,
    /// 1-based line in the hook, or 0 when the message points outside it (for example at the
    /// glue that calls a missing `cooked_surface`) or has no location.
    pub line: u32,
    /// 1-based column in characters, or 0 with `line`.
    pub column: u32,
    /// The compiler's message, without its rendering of the combined source.
    pub message: String,
}

impl fmt::Display for MaterialDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}: {}", self.file, self.message)
        } else {
            write!(
                f,
                "{}:{}:{}: {}",
                self.file, self.line, self.column, self.message
            )
        }
    }
}

/// Where the time of one reload went, measured from its request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReloadTiming {
    /// Time spent inside [`MaterialReload::request`] on the caller's thread: queueing, and a
    /// thread spawn when no compile was running, natively; in the browser the synchronous
    /// WebGPU shader module and pipeline creation calls themselves.
    pub call: Duration,
    /// Creating the shader module and pipeline (on the compile thread natively).
    pub prepare: Duration,
    /// Until the compilation messages and validation scope were seen resolved at a frame boundary.
    pub eligible: Duration,
    /// Until the first frame drawing it was submitted.
    pub drawn: Duration,
}

/// What became of one request, read with [`MaterialReload::take_outcomes`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MaterialOutcome {
    /// The hook draws from the reported submission on.
    Promoted {
        /// Request serial.
        serial: u64,
        /// The hook's name.
        name: String,
        /// Where the time went.
        timing: ReloadTiming,
    },
    /// The hook failed to compile or validate; the previous pipeline keeps drawing.
    Rejected {
        /// Request serial.
        serial: u64,
        /// Every error, located in the hook where possible.
        diagnostics: Vec<MaterialDiagnostic>,
    },
    /// A newer request arrived before this one could draw.
    Superseded {
        /// Request serial.
        serial: u64,
    },
    /// The device or scene surface it was prepared for was replaced before it could draw;
    /// the replacement draws the last accepted hook.
    Retired {
        /// Request serial.
        serial: u64,
    },
}

impl MaterialOutcome {
    /// The request this outcome answers.
    #[must_use]
    pub fn serial(&self) -> u64 {
        match self {
            Self::Promoted { serial, .. }
            | Self::Rejected { serial, .. }
            | Self::Superseded { serial }
            | Self::Retired { serial } => *serial,
        }
    }
}

impl fmt::Display for MaterialOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Promoted {
                serial,
                name,
                timing,
            } => write!(
                f,
                "material {serial} ({name}) drawn: call {:.2} ms, prepare {:.2} ms, eligible {:.2} ms, drawn {:.2} ms",
                timing.call.as_secs_f64() * 1e3,
                timing.prepare.as_secs_f64() * 1e3,
                timing.eligible.as_secs_f64() * 1e3,
                timing.drawn.as_secs_f64() * 1e3,
            ),
            Self::Rejected {
                serial,
                diagnostics,
            } => {
                write!(
                    f,
                    "material {serial} refused; the previous material keeps drawing"
                )?;
                for diagnostic in diagnostics {
                    write!(f, "\n  {diagnostic}")?;
                }
                Ok(())
            }
            Self::Superseded { serial } => {
                write!(f, "material {serial} superseded by a newer reload")
            }
            Self::Retired { serial } => write!(
                f,
                "material {serial} was prepared for a replaced device; the last accepted material draws again once it validates there (reload again)"
            ),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
type Pending<T> = Pin<Box<dyn Future<Output = T> + Send>>;
#[cfg(target_arch = "wasm32")]
type Pending<T> = Pin<Box<dyn Future<Output = T>>>;

/// What a replacement mesh pipeline compiles against: one prepared scene surface.
#[derive(Clone)]
pub(crate) struct Target {
    pub(crate) generation: u64,
    /// Counts surfaces the contribution has been rebuilt for.
    pub(crate) epoch: u64,
    pub(crate) device: wgpu::Device,
    pub(crate) scene_source: Arc<str>,
    pub(crate) layout: wgpu::PipelineLayout,
    pub(crate) fragment_entry: &'static str,
    pub(crate) color_formats: [wgpu::TextureFormat; 2],
    pub(crate) depth_format: wgpu::TextureFormat,
}

impl Target {
    pub(crate) fn new(surface: &SceneSurface<'_>, epoch: u64) -> Self {
        Self {
            generation: surface.generation,
            epoch,
            device: surface.device.clone(),
            scene_source: surface.shader_source.into(),
            layout: surface.pipeline_layout.clone(),
            fragment_entry: surface.fragment_entry,
            color_formats: surface.color_formats,
            depth_format: surface.depth_format,
        }
    }

    fn identity(&self) -> (u64, u64) {
        (self.generation, self.epoch)
    }
}

/// The hooked shader source and where the hook starts in it.
struct Combined {
    text: String,
    hook_offset: usize,
}

fn combine(scene_source: &str, hook: &str) -> Result<Combined, String> {
    if scene_source.matches(ANCHOR).count() != 1 {
        return Err(format!(
            "the scene shader has no single `{ANCHOR}` material read for the hook to wrap; \
             the cooked material hook does not support this scene shader"
        ));
    }
    let mut text = scene_source.replacen(ANCHOR, HOOKED, 1);
    text.push('\n');
    text.push_str(include_str!("cooked.wgsl"));
    text.push('\n');
    text.push_str(include_str!("material.wgsl"));
    let hook_offset = text.len();
    text.push_str(hook);
    Ok(Combined { text, hook_offset })
}

/// The mesh pipeline of `target` with `module`'s entries: the one descriptor every cooked mesh
/// pipeline uses, hooked or not.
pub(crate) fn mesh_pipeline(target: &Target, module: &wgpu::ShaderModule) -> wgpu::RenderPipeline {
    let model_attributes = wgpu::vertex_attr_array![3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Uint32];
    target
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cooked mesh"),
            layout: Some(&target.layout),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("cooked_vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 12,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 12,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![1 => Snorm16x2, 2 => Float32x2],
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: crate::draw::INSTANCE_BYTES,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &model_attributes,
                    }),
                ],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: target.depth_format,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Greater),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module,
                entry_point: Some(target.fragment_entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &target.color_formats.map(|format| {
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })
                }),
            }),
            multiview_mask: None,
            cache: None,
        })
}

/// A compile result that may still be pending.
enum Resolving<T> {
    Pending(Pending<T>),
    Ready(T),
}

impl<T> Resolving<T> {
    /// Poll once, without a runtime: native results are ready on creation, and a browser
    /// promise stores its value when it settles, so a later poll finds it.
    fn poll_now(&mut self) -> bool {
        let Self::Pending(future) = self else {
            return true;
        };
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(value) => {
                *self = Self::Ready(value);
                true
            }
            Poll::Pending => false,
        }
    }

    fn ready(&self) -> Option<&T> {
        match self {
            Self::Ready(value) => Some(value),
            Self::Pending(_) => None,
        }
    }
}

/// A replacement pipeline whose compilation messages and validation scope may still be pending.
struct Candidate {
    serial: u64,
    target: (u64, u64),
    hook: MaterialHook,
    /// Byte offset of the hook in the combined source.
    hook_offset: usize,
    pipeline: Option<wgpu::RenderPipeline>,
    requested: Instant,
    prepare: Duration,
    scope: Resolving<Option<wgpu::Error>>,
    info: Resolving<wgpu::CompilationInfo>,
    /// Errors found before compiling, such as an unsupported scene shader.
    refused: Vec<MaterialDiagnostic>,
}

impl Candidate {
    /// Create the module and pipeline for `hook` under a validation scope. The compilation
    /// messages and the scope resolve later and are polled at frame boundaries.
    fn prepare(target: &Target, hook: MaterialHook, serial: u64, requested: Instant) -> Self {
        let start = Instant::now();
        let mut candidate = Self {
            serial,
            target: target.identity(),
            hook,
            hook_offset: 0,
            pipeline: None,
            requested,
            prepare: Duration::ZERO,
            scope: Resolving::Ready(None),
            info: Resolving::Ready(wgpu::CompilationInfo {
                messages: Vec::new(),
            }),
            refused: Vec::new(),
        };
        match combine(&target.scene_source, &candidate.hook.source) {
            Ok(combined) => {
                let scope = target
                    .device
                    .push_error_scope(wgpu::ErrorFilter::Validation);
                let module = target
                    .device
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("cooked material hook"),
                        source: wgpu::ShaderSource::Wgsl(combined.text.into()),
                    });
                candidate.pipeline = Some(mesh_pipeline(target, &module));
                candidate.info = Resolving::Pending(Box::pin(module.get_compilation_info()));
                candidate.scope = Resolving::Pending(Box::pin(scope.pop()));
                candidate.hook_offset = combined.hook_offset;
            }
            Err(message) => candidate.refused.push(MaterialDiagnostic {
                file: candidate.hook.name.clone(),
                line: 0,
                column: 0,
                message,
            }),
        }
        candidate.prepare = start.elapsed();
        candidate
    }

    /// The validated candidate, ready to draw.
    fn eligible(self, pipeline: wgpu::RenderPipeline, restore: bool) -> Eligible {
        Eligible {
            serial: self.serial,
            pipeline,
            validated: self.requested.elapsed(),
            hook: self.hook,
            requested: self.requested,
            prepare: self.prepare,
            restore,
        }
    }

    /// Whether the compilation messages and the validation scope have both resolved.
    fn resolved(&mut self) -> bool {
        let info = self.info.poll_now();
        let scope = self.scope.poll_now();
        info && scope
    }

    /// Every error, translated to the hook; empty when the candidate is valid. Call once
    /// [`Self::resolved`].
    fn errors(&self) -> Vec<MaterialDiagnostic> {
        let mut errors = self.refused.clone();
        if let Some(info) = self.info.ready() {
            errors.extend(
                info.messages
                    .iter()
                    .filter(|message| message.message_type == wgpu::CompilationMessageType::Error)
                    .map(|message| self.translate(message)),
            );
        }
        if errors.is_empty()
            && let Some(Some(error)) = self.scope.ready()
        {
            // A pipeline error (for example a binding the layout lacks) has no source location.
            errors.push(MaterialDiagnostic {
                file: self.hook.name.clone(),
                line: 0,
                column: 0,
                message: summary(&error.to_string()),
            });
        }
        errors
    }

    fn translate(&self, message: &wgpu::CompilationMessage) -> MaterialDiagnostic {
        let text = summary(&message.message);
        let located = message.location.and_then(|location| {
            let offset = usize::try_from(location.offset).ok()?;
            locate(&self.hook.source, offset.checked_sub(self.hook_offset)?)
        });
        let (line, column, message) = match located {
            Some((line, column)) => (line, column, text),
            None if message.location.is_some() => (
                0,
                0,
                format!("{text} (in the engine's code around the hook)"),
            ),
            None => (0, 0, text),
        };
        MaterialDiagnostic {
            file: self.hook.name.clone(),
            line,
            column,
            message,
        }
    }
}

/// 1-based line and character column of a byte offset in `source`.
fn locate(source: &str, offset: usize) -> Option<(u32, u32)> {
    let before = source.get(..offset)?;
    let line_start = before.rfind('\n').map_or(0, |at| at + 1);
    let line = before.matches('\n').count() + 1;
    let column = before[line_start..].chars().count() + 1;
    Some((u32::try_from(line).ok()?, u32::try_from(column).ok()?))
}

/// A compiler message without naga's rendering of the combined source: its first line, without
/// the shader label, plus any `= note` lines.
fn summary(message: &str) -> String {
    let mut lines = message
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let first = lines.next().unwrap_or("shader error");
    let first = first
        .strip_prefix("Shader validation ")
        .or_else(|| {
            first
                .strip_prefix("Shader '")
                .and_then(|rest| rest.split_once("' parsing "))
                .map(|(_, rest)| rest)
        })
        .unwrap_or(first);
    let first = first.strip_prefix("error: ").unwrap_or(first);
    let mut text = first.to_owned();
    for note in lines.filter_map(|line| line.strip_prefix("= ")) {
        text.push_str("; ");
        text.push_str(note);
    }
    text
}

/// A validated candidate waiting for the frame boundary that draws it.
pub(crate) struct Eligible {
    serial: u64,
    pub(crate) pipeline: wgpu::RenderPipeline,
    hook: MaterialHook,
    requested: Instant,
    prepare: Duration,
    validated: Duration,
    /// The accepted hook restored on a rebuilt surface, rather than a request.
    restore: bool,
}

/// One request: the hook, its serial and when it was made.
struct Job {
    hook: MaterialHook,
    serial: u64,
    requested: Instant,
}

/// At most one compile at a time, with only the newest request waiting behind it.
struct Queue<J> {
    running: bool,
    waiting: Option<J>,
}

impl<J> Default for Queue<J> {
    fn default() -> Self {
        Self {
            running: false,
            waiting: None,
        }
    }
}

impl<J> Queue<J> {
    /// Make `job` the waiting one; returns the waiting job it replaces, which never compiles.
    fn push(&mut self, job: J) -> Option<J> {
        self.waiting.replace(job)
    }

    /// The waiting job, to compile now, unless a compile is running.
    fn start(&mut self) -> Option<J> {
        if self.running {
            return None;
        }
        let job = self.waiting.take()?;
        self.running = true;
        Some(job)
    }

    /// The running compile ended.
    fn finished(&mut self) {
        self.running = false;
    }
}

/// The accepted hook and every request's progress, shared by the contribution and its handles.
#[derive(Default)]
struct State {
    target: Option<Target>,
    /// Identity of the current scene surface; candidates for any other cannot publish.
    surface: Option<(u64, u64)>,
    /// Newest request serial.
    latest: u64,
    /// Caller time of the newest request.
    latest_call: Duration,
    queue: Queue<Job>,
    candidate: Option<Candidate>,
    /// The accepted hook prepared for a rebuilt surface, drawn once it validates there.
    restoring: Option<Candidate>,
    /// The accepted hook and the serial of the request that brought it.
    accepted: Option<(MaterialHook, u64)>,
    /// Whether the mesh pipeline draws the accepted hook; false while it is being restored.
    drawing: bool,
    outcomes: Vec<MaterialOutcome>,
    /// Counts changes a report shows: outcomes, and whether the accepted hook draws.
    revision: u64,
}

impl State {
    fn report(&mut self, outcome: MaterialOutcome) {
        self.outcomes.push(outcome);
        self.revision += 1;
    }

    /// The waiting request and the surface to compile it for, if no compile is running.
    fn start(&mut self) -> Option<(Target, Job)> {
        let target = self.target.clone()?;
        self.queue.start().map(|job| (target, job))
    }

    /// Keep the newest candidate for the current surface; anything else cannot publish.
    fn post(&mut self, candidate: Candidate) {
        if candidate.serial != self.latest {
            self.report(MaterialOutcome::Superseded {
                serial: candidate.serial,
            });
        } else if self.surface != Some(candidate.target) {
            self.report(MaterialOutcome::Retired {
                serial: candidate.serial,
            });
        } else if let Some(older) = self.candidate.replace(candidate) {
            self.report(MaterialOutcome::Superseded {
                serial: older.serial,
            });
        }
    }

    /// The scene surface is now `surface`: candidates and a restore for the old one cannot
    /// publish, and the accepted hook stops drawing until it is restored.
    fn rebuilt(&mut self, surface: (u64, u64)) {
        if let Some(candidate) = self.candidate.take() {
            self.report(MaterialOutcome::Retired {
                serial: candidate.serial,
            });
        }
        self.restoring = None;
        self.surface = Some(surface);
        if self.accepted.is_some() && self.drawing {
            self.drawing = false;
            self.revision += 1;
        }
    }

    /// At a frame boundary on `surface`: consider the newest request's candidate.
    fn consider_candidate(&mut self, held: &mut Option<Eligible>, surface: (u64, u64)) {
        let Some(mut candidate) = self.candidate.take() else {
            return;
        };
        if candidate.serial != self.latest {
            self.report(MaterialOutcome::Superseded {
                serial: candidate.serial,
            });
            return;
        }
        if candidate.target != surface {
            self.report(MaterialOutcome::Retired {
                serial: candidate.serial,
            });
            return;
        }
        if !candidate.resolved() {
            self.candidate = Some(candidate);
            return;
        }
        let errors = candidate.errors();
        match candidate.pipeline.take() {
            Some(pipeline) if errors.is_empty() => {
                if let Some(previous) = held.replace(candidate.eligible(pipeline, false)) {
                    self.report(MaterialOutcome::Superseded {
                        serial: previous.serial,
                    });
                }
            }
            _ => self.report(MaterialOutcome::Rejected {
                serial: candidate.serial,
                diagnostics: errors,
            }),
        }
    }

    /// At a frame boundary on `surface` with nothing newer to draw: hold the accepted hook for
    /// a rebuilt surface once it has validated there, or drop it if it no longer does.
    fn consider_restore(&mut self, held: &mut Option<Eligible>, surface: (u64, u64)) {
        let Some(mut restoring) = self.restoring.take() else {
            return;
        };
        if restoring.target != surface {
            return;
        }
        if !restoring.resolved() {
            self.restoring = Some(restoring);
            return;
        }
        let errors = restoring.errors();
        match restoring.pipeline.take() {
            Some(pipeline) if errors.is_empty() => {
                *held = Some(restoring.eligible(pipeline, true));
            }
            _ => {
                // The built-in material keeps drawing.
                self.accepted = None;
                self.drawing = false;
                self.report(MaterialOutcome::Rejected {
                    serial: restoring.serial,
                    diagnostics: errors,
                });
            }
        }
    }
}

/// Explicit material reloads of one [`crate::CookedAssets`]; clones share it.
#[derive(Clone)]
pub struct MaterialReload(Arc<Mutex<State>>);

impl fmt::Debug for MaterialReload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MaterialReload")
            .field("accepted", &self.accepted())
            .finish_non_exhaustive()
    }
}

impl MaterialReload {
    pub(crate) fn new() -> Self {
        Self(Arc::new(Mutex::new(State::default())))
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        // The state stays consistent between statements, so a panicked holder leaves it usable.
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Prepare `hook` to replace the cooked meshes' material, superseding any earlier request,
    /// and return its serial; its [`MaterialOutcome`] arrives through [`Self::take_outcomes`].
    /// Call it outside frame recording, for example from a key or button handler.
    ///
    /// One compile runs at a time. A request made while one runs waits, and a newer request
    /// supersedes it before it compiles. A request made before the assets have drawn their
    /// first frame is prepared when they first build their pipelines; its timing then includes
    /// that wait. Natively the compile runs on a thread and this call only queues it. In the
    /// browser this call creates the shader module and pipeline through WebGPU's synchronous
    /// methods (wgpu 30 has no asynchronous pipeline creation), so a cold compile can stall
    /// the page for hundreds of milliseconds: use it as a development tool, not in play.
    pub fn request(&self, hook: MaterialHook) -> u64 {
        let requested = Instant::now();
        let (serial, start) = {
            let mut state = self.state();
            state.latest += 1;
            let serial = state.latest;
            if let Some(earlier) = state.queue.push(Job {
                hook,
                serial,
                requested,
            }) {
                state.report(MaterialOutcome::Superseded {
                    serial: earlier.serial,
                });
            }
            (serial, state.start())
        };
        if let Some((target, job)) = start {
            self.compile(target, job);
        }
        let call = requested.elapsed();
        let mut state = self.state();
        if state.latest == serial {
            state.latest_call = call;
        }
        serial
    }

    /// Compile `job`, then each request that waited meanwhile, until none waits.
    fn compile_queue(&self, mut target: Target, mut job: Job) {
        loop {
            let candidate = Candidate::prepare(&target, job.hook, job.serial, job.requested);
            let mut state = self.state();
            state.post(candidate);
            state.queue.finished();
            let Some(next) = state.start() else {
                return;
            };
            (target, job) = next;
        }
    }

    /// Native: compile on one thread per burst of requests, which posts each candidate.
    #[cfg(not(target_arch = "wasm32"))]
    fn compile(&self, target: Target, job: Job) {
        let shared = self.clone();
        let (name, serial) = (job.hook.name.clone(), job.serial);
        let spawned = std::thread::Builder::new()
            .name("thrum-cooked-material".into())
            .spawn(move || shared.compile_queue(target, job));
        if let Err(error) = spawned {
            let mut state = self.state();
            state.queue.finished();
            state.report(MaterialOutcome::Rejected {
                serial,
                diagnostics: vec![MaterialDiagnostic {
                    file: name,
                    line: 0,
                    column: 0,
                    message: format!("material compile thread failed to start: {error}"),
                }],
            });
        }
    }

    /// Browser: create the module and pipeline in the caller's task; results resolve later.
    #[cfg(target_arch = "wasm32")]
    fn compile(&self, target: Target, job: Job) {
        self.compile_queue(target, job);
    }

    /// Outcomes since the last call, oldest first.
    pub fn take_outcomes(&self) -> Vec<MaterialOutcome> {
        std::mem::take(&mut self.state().outcomes)
    }

    /// Name of the accepted hook, or `None` for the built-in material. After a device or scene
    /// surface is replaced the accepted hook stays accepted while it is restored; see
    /// [`Self::restoring`].
    #[must_use]
    pub fn accepted(&self) -> Option<String> {
        self.state()
            .accepted
            .as_ref()
            .map(|(hook, _)| hook.name.clone())
    }

    /// Whether the accepted hook is being compiled and validated again for a replaced device or
    /// scene surface, while the built-in material draws.
    #[must_use]
    pub fn restoring(&self) -> bool {
        let state = self.state();
        state.accepted.is_some() && !state.drawing
    }

    /// Counts changes to what [`Self::take_outcomes`] and [`Self::restoring`] report, so an idle
    /// caller can skip building a report.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.state().revision
    }

    /// The scene surface changed to `target`. Candidates for the old one cannot publish, a
    /// waiting request is compiled for this one, and the accepted hook is prepared for it now
    /// (on the caller's thread, as the surface's own pipelines are). It draws once it has
    /// validated, which natively is the next frame; the built-in material draws until then.
    pub(crate) fn retarget(&self, target: &Target) {
        let (accepted, start) = {
            let mut state = self.state();
            state.rebuilt(target.identity());
            state.target = Some(target.clone());
            (state.accepted.clone(), state.start())
        };
        if let Some((hook, serial)) = accepted {
            let candidate = Candidate::prepare(target, hook, serial, Instant::now());
            let mut state = self.state();
            // A newer surface or a newer accepted hook may have arrived meanwhile.
            if state.surface == Some(candidate.target)
                && state.accepted.as_ref().map(|(_, accepted)| *accepted) == Some(serial)
            {
                state.restoring = Some(candidate);
            }
        }
        if let Some((target, job)) = start {
            self.compile(target, job);
        }
    }

    /// A held replacement can no longer draw (its surface was rebuilt).
    pub(crate) fn release(&self, held: &Eligible) {
        if !held.restore {
            self.state().report(MaterialOutcome::Retired {
                serial: held.serial,
            });
        }
    }

    /// At a frame boundary on the surface `target`: drop a held request that a newer one
    /// superseded, hold the newest candidate once it has resolved valid, and otherwise hold the
    /// accepted hook for a rebuilt surface once it has validated there.
    pub(crate) fn frame_boundary(&self, held: &mut Option<Eligible>, target: (u64, u64)) {
        let mut state = self.state();
        if held
            .as_ref()
            .is_some_and(|eligible| !eligible.restore && eligible.serial != state.latest)
            && let Some(eligible) = held.take()
        {
            state.report(MaterialOutcome::Superseded {
                serial: eligible.serial,
            });
        }
        // A held restore draws first; a newer request is considered once it is promoted.
        if held.as_ref().is_some_and(|eligible| eligible.restore) {
            return;
        }
        state.consider_candidate(held, target);
        if held.is_none() {
            state.consider_restore(held, target);
        }
    }

    /// The submission carrying the first frame that drew `eligible` completed recording.
    pub(crate) fn promote(&self, eligible: Eligible) {
        let mut state = self.state();
        state.drawing = true;
        if eligible.restore {
            state.revision += 1;
            return;
        }
        let timing = ReloadTiming {
            call: state.latest_call,
            prepare: eligible.prepare,
            eligible: eligible.validated,
            drawn: eligible.requested.elapsed(),
        };
        state.report(MaterialOutcome::Promoted {
            serial: eligible.serial,
            name: eligible.hook.name.clone(),
            timing,
        });
        state.accepted = Some((eligible.hook, eligible.serial));
        // A restore of the previous hook is no longer wanted.
        state.restoring = None;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use super::{
        ANCHOR, Candidate, MaterialDiagnostic, MaterialHook, MaterialOutcome, MaterialReload,
        Queue, Resolving, State, combine, locate, summary,
    };

    fn hook() -> MaterialHook {
        MaterialHook {
            name: "hooks/moss.wgsl".into(),
            source: String::new(),
        }
    }

    /// A candidate without a device: still compiling, or resolved and refused.
    fn candidate(serial: u64, target: (u64, u64), resolved: bool) -> Candidate {
        Candidate {
            serial,
            target,
            hook: hook(),
            hook_offset: 0,
            pipeline: None,
            requested: Instant::now(),
            prepare: Duration::ZERO,
            scope: if resolved {
                Resolving::Ready(None)
            } else {
                Resolving::Pending(Box::pin(std::future::pending()))
            },
            info: Resolving::Ready(wgpu::CompilationInfo {
                messages: Vec::new(),
            }),
            refused: vec![MaterialDiagnostic {
                file: "hooks/moss.wgsl".into(),
                line: 0,
                column: 0,
                message: "no longer compiles".into(),
            }],
        }
    }

    fn reload(state: State) -> MaterialReload {
        MaterialReload(Arc::new(Mutex::new(state)))
    }

    #[test]
    fn one_compile_runs_and_only_the_newest_request_waits() {
        let mut queue = Queue::default();
        assert_eq!(queue.push(1), None);
        assert_eq!(queue.start(), Some(1));
        assert_eq!(queue.push(2), None);
        assert_eq!(queue.start(), None, "one compile at a time");
        assert_eq!(queue.push(3), Some(2), "2 never compiles");
        assert_eq!(queue.push(4), Some(3));
        queue.finished();
        assert_eq!(queue.start(), Some(4));
        queue.finished();
        assert_eq!(queue.start(), None);
    }

    #[test]
    fn a_burst_during_a_compile_supersedes_waiting_requests_before_they_compile() {
        let mut state = State::default();
        state.queue.running = true;
        let material = reload(state);
        let serials: Vec<u64> = (0..4).map(|_| material.request(hook())).collect();
        assert_eq!(serials, [1, 2, 3, 4]);
        assert_eq!(
            material.take_outcomes(),
            [1, 2, 3].map(|serial| MaterialOutcome::Superseded { serial })
        );
        let state = material.state();
        assert_eq!(state.queue.waiting.as_ref().map(|job| job.serial), Some(4));
        assert!(state.queue.running, "no second compile started");
    }

    #[test]
    fn an_unconsumed_candidate_replaced_by_a_newer_one_is_superseded() {
        let mut state = State {
            surface: Some((1, 1)),
            latest: 4,
            ..State::default()
        };
        state.post(candidate(4, (1, 1), false));
        state.latest = 5;
        state.post(candidate(5, (1, 1), false));
        assert_eq!(state.outcomes, [MaterialOutcome::Superseded { serial: 4 }]);
        assert_eq!(
            state.candidate.as_ref().map(|candidate| candidate.serial),
            Some(5)
        );
    }

    #[test]
    fn an_accepted_hook_that_fails_on_a_rebuilt_surface_leaves_the_built_in_material() {
        let material = reload(State {
            surface: Some((2, 1)),
            latest: 3,
            accepted: Some((hook(), 3)),
            restoring: Some(candidate(3, (2, 1), true)),
            ..State::default()
        });
        assert!(material.restoring());
        let mut held = None;
        material.frame_boundary(&mut held, (2, 1));
        assert!(held.is_none(), "nothing replaces the built-in pipeline");
        assert_eq!(material.accepted(), None);
        assert!(!material.restoring());
        let outcomes = material.take_outcomes();
        assert!(
            matches!(&outcomes[..], [MaterialOutcome::Rejected { serial: 3, diagnostics }]
                if diagnostics[0].message == "no longer compiles"),
            "{outcomes:?}"
        );
    }

    #[test]
    fn an_unvalidated_restore_keeps_the_accepted_intent_across_another_rebuild() {
        let material = reload(State {
            surface: Some((2, 1)),
            latest: 3,
            accepted: Some((hook(), 3)),
            drawing: true,
            ..State::default()
        });
        let revision = material.revision();
        material.state().rebuilt((2, 2));
        assert!(material.restoring());
        assert!(material.revision() > revision, "a report shows the restore");
        material.state().restoring = Some(candidate(3, (2, 2), false));
        let mut held = None;
        for _ in 0..3 {
            material.frame_boundary(&mut held, (2, 2));
            assert!(held.is_none(), "an unresolved restore never draws");
        }
        assert_eq!(material.accepted().as_deref(), Some("hooks/moss.wgsl"));
        // Replaced again before it resolved: the restore for the old surface is dropped, and
        // the hook stays accepted for the next one.
        material.state().rebuilt((3, 3));
        assert!(material.state().restoring.is_none());
        assert_eq!(material.accepted().as_deref(), Some("hooks/moss.wgsl"));
        assert!(material.restoring());
        assert_eq!(material.take_outcomes(), Vec::<MaterialOutcome>::new());
    }

    #[test]
    fn the_hook_wraps_the_single_material_read_and_follows_the_glue() {
        let scene = format!("fn shade() {{ let material={ANCHOR}; }}");
        let combined = combine(&scene, "fn cooked_surface() {}").expect("one anchor");
        assert!(
            combined
                .text
                .contains("cooked_hooked_material(materials[input.material], input)")
        );
        assert_eq!(
            &combined.text[combined.hook_offset..],
            "fn cooked_surface() {}"
        );
        assert!(combine("fn shade() {}", "").is_err());
        assert!(combine(&format!("{ANCHOR} {ANCHOR}"), "").is_err());
    }

    #[test]
    fn offsets_become_hook_lines_and_character_columns() {
        let hook = "fn a() {}\n// é\n  let x = y;\n";
        assert_eq!(locate(hook, 0), Some((1, 1)));
        let y = hook.find('y').expect("y");
        assert_eq!(locate(hook, y), Some((3, 11)));
        let after_accent = hook.find("\n  let").expect("line 3");
        assert_eq!(locate(hook, after_accent), Some((2, 5)));
        assert_eq!(locate(hook, hook.len() + 1), None);
    }

    #[test]
    fn naga_messages_lose_their_combined_source_rendering() {
        let parse = "\nShader 'cooked material hook' parsing error: expected ';', found 'let'\n   ┌─ wgsl:812:5\n   │\n812 │     let\n   │     ^^^ expected ';'\n\n";
        assert_eq!(summary(parse), "expected ';', found 'let'");
        let validation = "\nShader validation error: Function [4] 'cooked_surface' is invalid\n  ┌─ :3:1\n  │\n  = Expression [2] is invalid\n  = Type resolution failed\n";
        assert_eq!(
            summary(validation),
            "Function [4] 'cooked_surface' is invalid; Expression [2] is invalid; Type resolution failed"
        );
        assert_eq!(
            summary("unresolved value 'roughnes'"),
            "unresolved value 'roughnes'"
        );
    }
}
