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
//! [`MaterialReload::request`] prepares a replacement mesh pipeline outside frame recording: on a
//! short-lived thread natively, and inside the caller's task in the browser, which has no threads.
//! WebGPU compiles in the browser's GPU process, so the page's call does not wait for the compile;
//! a GPU process busy compiling can still delay frames. The candidate becomes eligible only once
//! its compilation messages and its validation error scope have both resolved. The next frame
//! boundary draws it, and the submission carrying that frame promotes it: it becomes the accepted
//! hook, the meshes restart motion history once, and resident products and bindings are kept.
//! A rejected candidate leaves the previous pipeline drawing, with diagnostics translated to the
//! hook's own file, line and column. A newer request supersedes an older one, and a candidate
//! prepared for a retired device or scene surface is dropped; a rebuilt device draws the last
//! accepted hook.

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
    /// Time spent inside [`MaterialReload::request`] on the caller's thread: a thread spawn
    /// natively; in the browser the shader module and pipeline creation calls themselves.
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
                "material {serial} was prepared for a replaced device; the last accepted material is drawn (reload again)"
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
    /// Compile `hook` for `target` under a validation scope; nothing waits here.
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
    pub(crate) serial: u64,
    pub(crate) pipeline: wgpu::RenderPipeline,
    hook: MaterialHook,
    requested: Instant,
    prepare: Duration,
    validated: Duration,
}

/// The accepted hook and every request's progress, shared by the contribution and its handles.
#[derive(Default)]
struct State {
    target: Option<Target>,
    /// Newest request serial.
    latest: u64,
    /// Caller time of the newest request.
    latest_call: Duration,
    candidate: Option<Candidate>,
    /// The newest request made before the assets had a scene surface to prepare it for.
    waiting: Option<(MaterialHook, u64, Instant)>,
    accepted: Option<MaterialHook>,
    outcomes: Vec<MaterialOutcome>,
}

impl State {
    /// Keep the newest candidate for the current surface; anything else cannot publish.
    fn post(&mut self, candidate: Candidate) {
        if candidate.serial != self.latest {
            self.outcomes.push(MaterialOutcome::Superseded {
                serial: candidate.serial,
            });
        } else if self.target.as_ref().map(Target::identity) != Some(candidate.target) {
            self.outcomes.push(MaterialOutcome::Retired {
                serial: candidate.serial,
            });
        } else {
            self.candidate = Some(candidate);
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
    /// Call it outside frame recording, for example from a key or button handler. A request made
    /// before the assets have drawn their first frame is prepared when they first build their
    /// pipelines; its timing then includes that wait.
    pub fn request(&self, hook: MaterialHook) -> u64 {
        let requested = Instant::now();
        let (serial, target) = {
            let mut state = self.state();
            state.latest += 1;
            let serial = state.latest;
            let Some(target) = state.target.clone() else {
                if let Some((_, earlier, _)) = state.waiting.replace((hook, serial, requested)) {
                    state
                        .outcomes
                        .push(MaterialOutcome::Superseded { serial: earlier });
                }
                return serial;
            };
            (serial, target)
        };
        self.prepare(target, hook, serial, requested);
        let call = requested.elapsed();
        let mut state = self.state();
        if state.latest == serial {
            state.latest_call = call;
        }
        serial
    }

    /// Native: compile on a short-lived thread, which posts the candidate.
    #[cfg(not(target_arch = "wasm32"))]
    fn prepare(&self, target: Target, hook: MaterialHook, serial: u64, requested: Instant) {
        let shared = self.clone();
        let name = hook.name.clone();
        let spawned = std::thread::Builder::new()
            .name("thrum-cooked-material".into())
            .spawn(move || {
                let candidate = Candidate::prepare(&target, hook, serial, requested);
                shared.state().post(candidate);
            });
        if let Err(error) = spawned {
            self.state().outcomes.push(MaterialOutcome::Rejected {
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
    #[expect(
        clippy::needless_pass_by_value,
        reason = "one signature with the native variant, which moves the target to its thread"
    )]
    fn prepare(&self, target: Target, hook: MaterialHook, serial: u64, requested: Instant) {
        let candidate = Candidate::prepare(&target, hook, serial, requested);
        self.state().post(candidate);
    }

    /// Outcomes since the last call, oldest first.
    pub fn take_outcomes(&self) -> Vec<MaterialOutcome> {
        std::mem::take(&mut self.state().outcomes)
    }

    /// Name of the hook drawn now, or `None` for the built-in material.
    #[must_use]
    pub fn accepted(&self) -> Option<String> {
        self.state().accepted.as_ref().map(|hook| hook.name.clone())
    }

    /// The scene surface changed: candidates for the old one cannot publish, and a request
    /// waiting for the first surface is prepared now. Returns the accepted hook, which the
    /// rebuilt pipelines draw.
    pub(crate) fn retarget(&self, target: Target) -> Option<MaterialHook> {
        let (accepted, waiting) = {
            let mut state = self.state();
            if let Some(candidate) = state.candidate.take() {
                state.outcomes.push(MaterialOutcome::Retired {
                    serial: candidate.serial,
                });
            }
            state.target = Some(target.clone());
            (state.accepted.clone(), state.waiting.take())
        };
        if let Some((hook, serial, requested)) = waiting {
            self.prepare(target, hook, serial, requested);
        }
        accepted
    }

    /// A held candidate can no longer publish (its surface was rebuilt).
    pub(crate) fn retire(&self, serial: u64) {
        self.state()
            .outcomes
            .push(MaterialOutcome::Retired { serial });
    }

    /// The accepted hook no longer compiles against a rebuilt surface: the built-in material
    /// draws instead.
    pub(crate) fn drop_accepted(&self, diagnostics: Vec<MaterialDiagnostic>) {
        let mut state = self.state();
        state.accepted = None;
        let serial = state.latest;
        state.outcomes.push(MaterialOutcome::Rejected {
            serial,
            diagnostics,
        });
    }

    /// At a frame boundary on the surface `target`: drop a held candidate that a newer request
    /// superseded, and hold the newest candidate once it has resolved valid.
    pub(crate) fn frame_boundary(&self, held: &mut Option<Eligible>, target: (u64, u64)) {
        let mut state = self.state();
        if held
            .as_ref()
            .is_some_and(|eligible| eligible.serial != state.latest)
            && let Some(eligible) = held.take()
        {
            state.outcomes.push(MaterialOutcome::Superseded {
                serial: eligible.serial,
            });
        }
        let Some(mut candidate) = state.candidate.take() else {
            return;
        };
        if candidate.serial != state.latest {
            state.outcomes.push(MaterialOutcome::Superseded {
                serial: candidate.serial,
            });
            return;
        }
        if candidate.target != target {
            state.outcomes.push(MaterialOutcome::Retired {
                serial: candidate.serial,
            });
            return;
        }
        if !candidate.resolved() {
            state.candidate = Some(candidate);
            return;
        }
        let errors = candidate.errors();
        match candidate.pipeline.take() {
            Some(pipeline) if errors.is_empty() => {
                if let Some(previous) = held.replace(Eligible {
                    serial: candidate.serial,
                    pipeline,
                    hook: candidate.hook,
                    requested: candidate.requested,
                    prepare: candidate.prepare,
                    validated: candidate.requested.elapsed(),
                }) {
                    state.outcomes.push(MaterialOutcome::Superseded {
                        serial: previous.serial,
                    });
                }
            }
            _ => state.outcomes.push(MaterialOutcome::Rejected {
                serial: candidate.serial,
                diagnostics: errors,
            }),
        }
    }

    /// The submission carrying the first frame that drew `eligible` completed recording.
    pub(crate) fn promote(&self, eligible: Eligible) {
        let mut state = self.state();
        let timing = ReloadTiming {
            call: state.latest_call,
            prepare: eligible.prepare,
            eligible: eligible.validated,
            drawn: eligible.requested.elapsed(),
        };
        state.outcomes.push(MaterialOutcome::Promoted {
            serial: eligible.serial,
            name: eligible.hook.name.clone(),
            timing,
        });
        state.accepted = Some(eligible.hook);
    }
}

/// Build the accepted hook's pipeline for a rebuilt surface, now. Natively the validation is
/// synchronous, so a hook that no longer compiles is reported; in the browser the results are
/// still pending here, and a hook accepted against the same scene shader is trusted.
pub(crate) fn rebuild_accepted(
    target: &Target,
    hook: &MaterialHook,
) -> Result<wgpu::RenderPipeline, Vec<MaterialDiagnostic>> {
    let mut candidate = Candidate::prepare(target, hook.clone(), 0, Instant::now());
    let errors = if candidate.resolved() {
        candidate.errors()
    } else {
        candidate.refused.clone()
    };
    match candidate.pipeline.take() {
        Some(pipeline) if errors.is_empty() => Ok(pipeline),
        _ => Err(errors),
    }
}

#[cfg(test)]
mod tests {
    use super::{ANCHOR, combine, locate, summary};

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
