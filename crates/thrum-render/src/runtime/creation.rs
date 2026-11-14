//! Proof counters (G3t M0, D0.3), an opt-in diagnostic behind the
//! `creation-counters` feature.
//!
//! **Pipeline events (the guard).** Every production pipeline in this crate is
//! created through a [`DeviceSite`], the generation-owned creation authority
//! that `DeviceCapabilities::capture` mints once per device generation. Each
//! creation records one event on that site's own ledger, so no device equality
//! or process-wide registry is involved, and a device of another instance or
//! generation never reaches it. The event carries the creating thread and the
//! phase it declared: the innermost region that thread has open on the same
//! ledger. A creation with no such region is *unscoped*: build before the
//! generation is ready, play after. An unscoped play pipeline is a violation.
//! Events are counted at creation, so a pipeline created and dropped between
//! frames is caught, and a worker's scoped build never hides another thread's
//! unscoped creation. The cooked asset sites create through the same site,
//! from `SceneSurface::device`; their compile thread declares its own
//! transition with [`DeviceSite::transition`].
//!
//! **Live-object deltas (diagnostic only).** wgpu's per-device live-object
//! counters are read at region and detach boundaries and at retirement, never
//! on the frame path (a frame is one relaxed atomic add), and the net change is
//! charged to the innermost open region, else play. They cover every object
//! kind but are deltas: a create-and-drop inside a window or a creation offset
//! by a deferred destruction is missed. A window names its thread only when
//! serialized (no detached handle, and the closing thread and every open region
//! on the thread that made the generation ready); others are *unattributed*.

use std::{
    cell::RefCell,
    fmt,
    sync::{
        Arc, Mutex, MutexGuard, PoisonError, Weak,
        atomic::{AtomicU64, Ordering},
    },
    thread,
};

/// When a creation happened relative to its generation's readiness.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CreationPhase {
    /// Preparing a generation before it is ready.
    Build,
    /// Replacing resources of a ready generation (visual, surface, contribution
    /// or capture), outside the frame path.
    Transition,
    /// Ordinary frames of a ready generation.
    Play,
}

/// Net live-object changes of the counted kinds.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CreationCounts {
    /// Render pipelines.
    pub render_pipelines: i64,
    /// Compute pipelines.
    pub compute_pipelines: i64,
    /// Shader modules.
    pub shader_modules: i64,
    /// Textures.
    pub textures: i64,
    /// Samplers.
    pub samplers: i64,
    /// Query sets (GPU timestamps).
    pub query_sets: i64,
}

impl CreationCounts {
    fn read(device: &wgpu::Device) -> Self {
        let hal = device.get_internal_counters().hal;
        let value = |live: isize| i64::try_from(live).unwrap_or(i64::MAX);
        Self {
            render_pipelines: value(hal.render_pipelines.read()),
            compute_pipelines: value(hal.compute_pipelines.read()),
            shader_modules: value(hal.shader_modules.read()),
            textures: value(hal.textures.read()),
            samplers: value(hal.samplers.read()),
            query_sets: value(hal.query_sets.read()),
        }
    }

    fn minus(self, base: Self) -> Self {
        Self {
            render_pipelines: self.render_pipelines - base.render_pipelines,
            compute_pipelines: self.compute_pipelines - base.compute_pipelines,
            shader_modules: self.shader_modules - base.shader_modules,
            textures: self.textures - base.textures,
            samplers: self.samplers - base.samplers,
            query_sets: self.query_sets - base.query_sets,
        }
    }

    fn add(&mut self, other: Self) {
        self.render_pipelines += other.render_pipelines;
        self.compute_pipelines += other.compute_pipelines;
        self.shader_modules += other.shader_modules;
        self.textures += other.textures;
        self.samplers += other.samplers;
        self.query_sets += other.query_sets;
    }

    fn is_zero(self) -> bool {
        self == Self::default()
    }

    /// Pipelines of either kind that this change added.
    #[must_use]
    pub fn pipelines(self) -> i64 {
        self.render_pipelines.max(0) + self.compute_pipelines.max(0)
    }
}

/// Whether the backend reports live-object deltas at all.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Instrumentation {
    /// Not yet known: the generation's build has not finished.
    Pending,
    /// The device's HAL counters are live (Metal, Vulkan).
    Counted,
    /// The backend reports nothing (WebGPU). Every delta row reads zero, which
    /// means "not observed", not "none created".
    Uninstrumented,
    /// An untracked site: no ledger, nothing counted.
    Untracked,
}

/// Whether native object creation is covered by this diagnostic ledger.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeSites {
    /// No Thrum code creates a native object.
    Absent,
    /// Native objects exist, but their creation counts are not instrumented.
    Uninstrumented,
}

/// The kind of pipeline a site created.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PipelineKind {
    /// `create_render_pipeline`.
    Render,
    /// `create_compute_pipeline`.
    Compute,
}

/// Pipeline creation events of one thread, phase and scope.
#[derive(Clone, Debug, PartialEq)]
pub struct PipelineRow {
    /// The creating thread's name, or its id when unnamed.
    pub thread: String,
    /// The phase: the creating thread's declared region, else build before
    /// readiness and play after.
    pub phase: CreationPhase,
    /// The declared region's name, or `"unscoped"`.
    pub label: &'static str,
    /// Render pipelines created.
    pub render: u64,
    /// Compute pipelines created.
    pub compute: u64,
}

/// A live-object delta row: one window owner, phase and region.
#[derive(Clone, Debug, PartialEq)]
pub struct CreationRow {
    /// The region's (or, for play, the closing) thread's name, or its id when
    /// unnamed. Established only for this row's attributed windows.
    pub thread: String,
    /// The phase the changes were charged to.
    pub phase: CreationPhase,
    /// The region's name, or `"frame"` for play.
    pub label: &'static str,
    /// Net changes.
    pub counts: CreationCounts,
    /// Windows with a nonzero change that went to this row.
    pub windows: u64,
    /// Of those, windows that were not serialized, so another thread may have
    /// made the change: their thread is unmeasured.
    pub unattributed: u64,
}

/// An unscoped pipeline created after readiness.
#[derive(Clone, Debug, PartialEq)]
pub struct CreationViolation {
    /// The creating thread.
    pub thread: String,
    /// Frames submitted on this generation before the creation.
    pub after_frame: u64,
    /// The pipeline's kind.
    pub kind: PipelineKind,
}

/// The ledger for one device generation.
#[derive(Clone, Debug, PartialEq)]
pub struct CreationReport {
    /// The device generation counted.
    pub generation: u64,
    /// Whether the backend reports live-object deltas.
    pub instrumentation: Instrumentation,
    /// Native-object creation sites.
    pub native: NativeSites,
    /// The generation's build finished.
    pub ready: bool,
    /// Frames submitted on this generation.
    pub frames: u64,
    /// Pipeline creation events through [`site`], in first-seen order.
    pub pipelines: Vec<PipelineRow>,
    /// Unscoped pipeline creations after readiness.
    pub violations: u64,
    /// The first of them on each creating thread, in first-seen order.
    pub first_violations: Vec<CreationViolation>,
    /// Live-object deltas by window owner, phase and region, in first-seen order.
    pub rows: Vec<CreationRow>,
    /// Regions closed on a different thread than opened (a migrated future);
    /// creations on the new thread were unscoped.
    pub migrated_regions: u64,
}

impl CreationReport {
    fn untracked() -> Self {
        Self {
            generation: 0,
            instrumentation: Instrumentation::Untracked,
            native: NativeSites::Absent,
            ready: false,
            frames: 0,
            pipelines: Vec::new(),
            violations: 0,
            first_violations: Vec::new(),
            rows: Vec::new(),
            migrated_regions: 0,
        }
    }

    /// Summed live-object deltas of one phase over all threads and regions.
    #[must_use]
    pub fn phase(&self, phase: CreationPhase) -> CreationCounts {
        let mut total = CreationCounts::default();
        for row in self.rows.iter().filter(|row| row.phase == phase) {
            total.add(row.counts);
        }
        total
    }

    /// Pipeline creation events of one phase over all threads and scopes.
    #[must_use]
    pub fn pipeline_events(&self, phase: CreationPhase) -> u64 {
        self.pipelines
            .iter()
            .filter(|row| row.phase == phase)
            .map(|row| row.render + row.compute)
            .sum()
    }
}

impl fmt::Display for CreationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "generation {} {:?} native {:?} ready {} frames {} violations {} migrated {}",
            self.generation,
            self.instrumentation,
            self.native,
            self.ready,
            self.frames,
            self.violations,
            self.migrated_regions
        )?;
        for row in &self.pipelines {
            write!(
                f,
                "; pipelines {:?}/{}@{}: render {} compute {}",
                row.phase, row.label, row.thread, row.render, row.compute
            )?;
        }
        for row in &self.rows {
            let c = row.counts;
            write!(
                f,
                "; delta {:?}/{}@{} x{} unattributed {}: render {} compute {} shaders {} textures {} samplers {} queries {}",
                row.phase,
                row.label,
                row.thread,
                row.windows,
                row.unattributed,
                c.render_pipelines,
                c.compute_pipelines,
                c.shader_modules,
                c.textures,
                c.samplers,
                c.query_sets
            )?;
        }
        Ok(())
    }
}

/// The pipeline creation authority of one device generation: the device,
/// and its generation's ledger. [`DeviceCapabilities`] mints it once at
/// capture, and every clone of those capabilities shares it, so the ledger is
/// the generation's identity token. It dereferences to the device, so other
/// resources are created as before; pipelines are created only through
/// [`DeviceSite::render`] and [`DeviceSite::compute`].
///
/// [`DeviceCapabilities`]: crate::capabilities::DeviceCapabilities
#[derive(Clone)]
pub struct DeviceSite(Arc<SiteCore>);

struct SiteCore {
    device: wgpu::Device,
    /// `None` for an [`untracked`](DeviceSite::untracked) site.
    ledger: Option<Arc<CreationLedger>>,
}

impl DeviceSite {
    /// The tracked site of a newly captured device generation.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            clippy::arc_with_non_send_sync,
            reason = "the browser device is not Send; the browser host is single-threaded"
        )
    )]
    pub(crate) fn mint(device: &wgpu::Device, generation: u64) -> Self {
        Self(Arc::new(SiteCore {
            device: device.clone(),
            ledger: Some(CreationLedger::start(device, generation)),
        }))
    }

    /// A site that records nothing, for fixtures, benches and convenience
    /// entry points that hold only a raw device. It is not a generation's
    /// identity: re-wrapping a device never reaches that generation's ledger.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            clippy::arc_with_non_send_sync,
            reason = "the browser device is not Send; the browser host is single-threaded"
        )
    )]
    #[must_use]
    pub fn untracked(device: wgpu::Device) -> Self {
        Self(Arc::new(SiteCore {
            device,
            ledger: None,
        }))
    }

    /// `create_render_pipeline`, unchanged, then one event on this site's ledger.
    pub fn render(&self, desc: &wgpu::RenderPipelineDescriptor<'_>) -> wgpu::RenderPipeline {
        let pipeline = self.0.device.create_render_pipeline(desc);
        if let Some(ledger) = &self.0.ledger {
            record(ledger, PipelineKind::Render);
        }
        pipeline
    }

    /// `create_compute_pipeline`, unchanged, then one event on this site's ledger.
    pub fn compute(&self, desc: &wgpu::ComputePipelineDescriptor<'_>) -> wgpu::ComputePipeline {
        let pipeline = self.0.device.create_compute_pipeline(desc);
        if let Some(ledger) = &self.0.ledger {
            record(ledger, PipelineKind::Compute);
        }
        pipeline
    }

    pub(crate) fn native_prepared(&self) {
        if let Some(ledger) = &self.0.ledger {
            ledger.lock().report.native = NativeSites::Uninstrumented;
        }
    }

    /// Declare a transition on the calling thread until the region drops:
    /// pipelines this thread creates through this site meanwhile are charged
    /// to `label`, not to play. For work outside the renderer's own regions,
    /// such as a cooked material compile thread.
    pub fn transition(&self, label: &'static str) -> CreationRegion {
        CreationRegion(
            self.0
                .ledger
                .as_ref()
                .map(|ledger| ledger.region(CreationPhase::Transition, label)),
        )
    }

    /// This generation's ledger.
    pub(crate) fn ledger(&self) -> Ledger {
        Ledger(self.0.ledger.clone())
    }
}

impl std::ops::Deref for DeviceSite {
    type Target = wgpu::Device;

    fn deref(&self) -> &wgpu::Device {
        &self.0.device
    }
}

impl fmt::Debug for DeviceSite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceSite")
            .field("device", &self.0.device)
            .field("tracked", &self.0.ledger.is_some())
            .finish()
    }
}

/// An open region on this thread.
struct Scope {
    ledger: Weak<CreationLedger>,
    id: u64,
}

thread_local! {
    static SCOPES: RefCell<Vec<Scope>> = const { RefCell::new(Vec::new()) };
}

/// Record one pipeline creation on `ledger`: in the innermost region this
/// thread has open on that same ledger, else unscoped.
fn record(ledger: &Arc<CreationLedger>, kind: PipelineKind) {
    let target = Arc::as_ptr(ledger);
    let scope = SCOPES.with_borrow_mut(|scopes| {
        scopes.retain(|scope| scope.ledger.strong_count() > 0);
        scopes
            .iter()
            .rev()
            .find(|scope| scope.ledger.as_ptr() == target)
            .map(|scope| scope.id)
    });
    if scope.is_some_and(|id| ledger.event(Some(id), kind)) {
        return;
    }
    ledger.event(None, kind);
}

/// The host's shared handle to its generation's ledger; empty for an
/// untracked site.
#[derive(Clone)]
pub(crate) struct Ledger(Option<Arc<CreationLedger>>);

impl Ledger {
    /// Open a region on the calling thread.
    pub(crate) fn region(&self, phase: CreationPhase, label: &'static str) -> CreationRegion {
        CreationRegion(self.0.as_ref().map(|ledger| ledger.region(phase, label)))
    }

    /// A handle for a detached preparation of this generation.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn detach(&self) -> Detached {
        Detached(self.0.as_ref().map(CreationLedger::detach))
    }

    /// See [`CreationLedger::ready`].
    pub(crate) fn ready(&self) {
        if let Some(ledger) = &self.0 {
            ledger.ready();
        }
    }

    /// See [`CreationLedger::frame`].
    pub(crate) fn frame(&self) {
        if let Some(ledger) = &self.0 {
            ledger.frame();
        }
    }

    /// See [`CreationLedger::report`].
    pub(crate) fn report(&self) -> CreationReport {
        self.0
            .as_ref()
            .map_or_else(CreationReport::untracked, |ledger| ledger.report())
    }

    /// See [`CreationLedger::retire`].
    pub(crate) fn retire(&self) -> CreationReport {
        self.0
            .as_ref()
            .map_or_else(CreationReport::untracked, |ledger| ledger.retire())
    }
}

struct Open {
    id: u64,
    phase: CreationPhase,
    label: &'static str,
    thread: String,
    thread_id: thread::ThreadId,
}

struct State {
    last: CreationCounts,
    open: Vec<Open>,
    next_id: u64,
    /// The thread that last made the generation ready or submitted a frame.
    owner: thread::ThreadId,
    /// Live detached preparation handles that may create on this device.
    detached: usize,
    report: CreationReport,
}

/// Creation counts for one device generation, shared by the host and its
/// detached preparations. Its lock is taken at window boundaries and on each
/// counted pipeline creation.
pub(crate) struct CreationLedger {
    device: wgpu::Device,
    state: Mutex<State>,
    /// Frames submitted: the frame path's only touch, lock-free.
    frames: AtomicU64,
}

/// An open build or transition region. Dropping it closes its window.
#[must_use = "the region closes when dropped"]
pub struct CreationRegion(
    #[expect(dead_code, reason = "held only for its drop, which closes the window")]
    Option<OpenRegion>,
);

struct OpenRegion {
    ledger: Arc<CreationLedger>,
    id: u64,
}

impl Drop for OpenRegion {
    fn drop(&mut self) {
        let id = self.id;
        let ledger = Arc::as_ptr(&self.ledger);
        let popped = SCOPES.with_borrow_mut(|scopes| {
            let index = scopes
                .iter()
                .rposition(|scope| scope.id == id && scope.ledger.as_ptr() == ledger)?;
            scopes.remove(index);
            Some(())
        });
        self.ledger.close(id, popped.is_none());
    }
}

/// A live detached preparation that may create on the device from any thread.
/// While one is alive, no live-object window is attributed to a thread.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct Detached(Option<DetachedLedger>);

#[cfg(not(target_arch = "wasm32"))]
struct DetachedLedger {
    ledger: Arc<CreationLedger>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Detached {
    /// Open a region on the calling thread.
    pub(crate) fn region(&self, phase: CreationPhase, label: &'static str) -> CreationRegion {
        CreationRegion(
            self.0
                .as_ref()
                .map(|detached| detached.ledger.region(phase, label)),
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for DetachedLedger {
    fn drop(&mut self) {
        let mut state = self.ledger.lock();
        self.ledger.flush(&mut state);
        state.detached -= 1;
    }
}

fn thread_name() -> String {
    let current = thread::current();
    current
        .name()
        .map_or_else(|| format!("{:?}", current.id()), str::to_owned)
}

impl CreationLedger {
    /// Start counting on `device` from the objects it holds now.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            clippy::arc_with_non_send_sync,
            reason = "the browser device is not Send; the browser host is single-threaded"
        )
    )]
    fn start(device: &wgpu::Device, generation: u64) -> Arc<Self> {
        Arc::new(Self {
            device: device.clone(),
            state: Mutex::new(State {
                last: CreationCounts::read(device),
                open: Vec::new(),
                next_id: 0,
                owner: thread::current().id(),
                detached: 0,
                report: CreationReport {
                    generation,
                    instrumentation: Instrumentation::Pending,
                    native: NativeSites::Absent,
                    ready: false,
                    frames: 0,
                    pipelines: Vec::new(),
                    violations: 0,
                    first_violations: Vec::new(),
                    rows: Vec::new(),
                    migrated_regions: 0,
                },
            }),
            frames: AtomicU64::new(0),
        })
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Open a region on the calling thread.
    fn region(self: &Arc<Self>, phase: CreationPhase, label: &'static str) -> OpenRegion {
        let mut state = self.lock();
        self.flush(&mut state);
        let id = state.next_id;
        state.next_id += 1;
        state.open.push(Open {
            id,
            phase,
            label,
            thread: thread_name(),
            thread_id: thread::current().id(),
        });
        drop(state);
        SCOPES.with_borrow_mut(|scopes| {
            scopes.push(Scope {
                ledger: Arc::downgrade(self),
                id,
            });
        });
        OpenRegion {
            ledger: Arc::clone(self),
            id,
        }
    }

    /// A handle for a detached preparation of this generation.
    #[cfg(not(target_arch = "wasm32"))]
    fn detach(self: &Arc<Self>) -> DetachedLedger {
        let mut state = self.lock();
        self.flush(&mut state);
        state.detached += 1;
        DetachedLedger {
            ledger: Arc::clone(self),
        }
    }

    fn close(&self, id: u64, migrated: bool) {
        let mut state = self.lock();
        self.flush(&mut state);
        state.open.retain(|open| open.id != id);
        state.report.migrated_regions += u64::from(migrated);
    }

    /// Record one pipeline event, in region `scope` if it is still open.
    /// Returns false, recording nothing, for a scope that has closed.
    fn event(&self, scope: Option<u64>, kind: PipelineKind) -> bool {
        let mut state = self.lock();
        let (phase, label) = match scope {
            Some(id) => match state.open.iter().find(|open| open.id == id) {
                Some(open) => (open.phase, open.label),
                None => return false,
            },
            None if state.report.ready => (CreationPhase::Play, "unscoped"),
            None => (CreationPhase::Build, "unscoped"),
        };
        let thread = thread_name();
        if scope.is_none() && phase == CreationPhase::Play {
            state.report.violations += 1;
            let after_frame = self.frames.load(Ordering::Relaxed);
            let firsts = &mut state.report.first_violations;
            if !firsts.iter().any(|first| first.thread == thread) {
                firsts.push(CreationViolation {
                    thread: thread.clone(),
                    after_frame,
                    kind,
                });
            }
        }
        let rows = &mut state.report.pipelines;
        let index = if let Some(index) = rows
            .iter()
            .position(|row| row.phase == phase && row.label == label && row.thread == thread)
        {
            index
        } else {
            rows.push(PipelineRow {
                thread,
                phase,
                label,
                render: 0,
                compute: 0,
            });
            rows.len() - 1
        };
        match kind {
            PipelineKind::Render => rows[index].render += 1,
            PipelineKind::Compute => rows[index].compute += 1,
        }
        true
    }

    /// The generation is ready: later unscoped pipelines are violations. A
    /// build that added no live pipeline means the backend reports no deltas.
    fn ready(&self) {
        let mut state = self.lock();
        self.flush(&mut state);
        state.report.instrumentation = if state.report.phase(CreationPhase::Build).pipelines() > 0 {
            Instrumentation::Counted
        } else {
            Instrumentation::Uninstrumented
        };
        state.report.ready = true;
        state.owner = thread::current().id();
    }

    /// A frame was submitted. One relaxed atomic add: no lock, no counter read.
    fn frame(&self) {
        self.frames.fetch_add(1, Ordering::Relaxed);
    }

    /// Close the last window of a retired generation and return its report.
    fn retire(&self) -> CreationReport {
        let mut state = self.lock();
        self.flush(&mut state);
        let mut report = state.report.clone();
        report.frames = self.frames.load(Ordering::Relaxed);
        report
    }

    /// The counts through the last window boundary. Reading does not close a
    /// window, so a reader on another thread never takes a play window.
    fn report(&self) -> CreationReport {
        let mut report = self.lock().report.clone();
        report.frames = self.frames.load(Ordering::Relaxed);
        report
    }

    /// Charge the live-object change since the last window to the innermost
    /// open region, or to play.
    fn flush(&self, state: &mut State) {
        let now = CreationCounts::read(&self.device);
        let delta = now.minus(state.last);
        state.last = now;
        if delta.is_zero() {
            return;
        }
        // Serialized: no detached handle, and the closing thread and every open
        // region's thread are the owner. Otherwise another thread may have made
        // the change, whatever thread closed the window.
        let owner = state.owner;
        let unattributed = state.detached != 0
            || thread::current().id() != owner
            || state.open.iter().any(|open| open.thread_id != owner);
        let (phase, label, thread) = match state.open.last() {
            Some(open) => (open.phase, open.label, open.thread.clone()),
            None if state.report.ready => (CreationPhase::Play, "frame", thread_name()),
            None => (CreationPhase::Build, "unscoped", thread_name()),
        };
        let rows = &mut state.report.rows;
        if let Some(row) = rows
            .iter_mut()
            .find(|row| row.phase == phase && row.label == label && row.thread == thread)
        {
            row.counts.add(delta);
            row.windows += 1;
            row.unattributed += u64::from(unattributed);
        } else {
            rows.push(CreationRow {
                thread,
                phase,
                label,
                counts: delta,
                windows: 1,
                unattributed: u64::from(unattributed),
            });
        }
    }
}
