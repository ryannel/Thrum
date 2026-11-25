//! A provider's candidate, ordered encoder segment and completion mailbox.
use crate::{
    capabilities::{DeviceCapabilities, Recipe},
    error::RenderError,
    graph::{FrameGraph, GraphTextures, PassBody, ResourceId},
};
use std::{
    any::Any,
    sync::{Arc, Mutex},
};

/// Prepared twin selection, independent of any provider's diagnostic controls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TwinReason {
    /// The generation selected its portable recipe.
    PortableRecipe,
    /// The caller explicitly requested the twin.
    Requested,
    /// The generation is not backed by Metal.
    UnsupportedBackend,
    /// The provider declined native preparation.
    PreparationRefused,
    /// The provider was rejected at a free frame boundary.
    Rejected,
}

/// Identity and admission reason of the selected portable twin.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeTwinSelection {
    /// The prepared portable pass or provider name.
    pub name: &'static str,
    /// Why the segment selected it.
    pub reason: TwinReason,
}

/// Candidate counters for one prepared provider.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeProviderReport {
    /// Provider pass identity.
    pub pass: &'static str,
    /// Selected twin and its reason, or native when absent.
    pub twin: Option<NativeTwinSelection>,
    /// Encoded candidates.
    pub encoded: u64,
    /// Successfully submitted candidates.
    pub submitted: u64,
    /// Encoded candidates discarded before submission.
    pub discarded: u64,
    /// Discarded candidates that kept the last submitted history.
    pub discards_kept_history: u64,
    /// Encoded candidates that received a reset.
    pub resets: u64,
    /// Completed submitted candidates.
    pub completed: u64,
}

/// Native command-buffer observation in Metal's host-seconds clock domain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NativeOutcome {
    /// Queue completion has not delivered this segment yet.
    Pending,
    /// Executed without usable timing (including disabled timing and the twin).
    Unmeasured,
    /// Dedicated native command-buffer GPU start/end, in host seconds.
    Timed([f64; 2]),
    /// Raw command-buffer error; the platform must rebuild this generation.
    Failed,
}

/// One graph segment's completion mailbox, tied to exactly one submission.
#[derive(Clone, Debug)]
pub struct NativeSegment {
    pass: &'static str,
    /// Platform device generation that recorded the work.
    pub generation: u64,
    /// Coordinator submission order, never a world tick or query-buffer ordinal.
    pub submission: u64,
    outcome: Arc<Mutex<NativeOutcome>>,
}

impl NativeSegment {
    /// Graph pass identity shared by the native provider and its twin.
    pub fn pass_name(&self) -> &'static str {
        self.pass
    }

    /// A pending or missing observation never becomes a fabricated zero duration.
    pub fn outcome(&self) -> NativeOutcome {
        self.outcome
            .lock()
            .map_or(NativeOutcome::Failed, |value| *value)
    }
}

/// Scene routing for the provider's declared resources. The segment does not
/// interpret this layout; the provider binds the resources it declared.
pub(crate) struct NativeGraph {
    pub(crate) pass: usize,
    pub(crate) output: ResourceId,
    pub(crate) history: Option<ResourceId>,
    pub(crate) extent: [u32; 2],
    /// Additional resources, in the provider's own bind order.
    pub(crate) bindings: Vec<ResourceId>,
}

/// Values sampled for one candidate, independent of the prepared texture sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NativeFrameValues {
    pub(crate) content_origin: [u32; 2],
    pub(crate) content_extent: [u32; 2],
    pub(crate) raster_jitter: [f32; 2],
}

/// Scene inputs and the portable reconstruction pass, when it supplies a twin.
pub(crate) struct NativeGraphInputs<'a> {
    pub(crate) attachments: [&'a ResourceId; 4],
    pub(crate) exposure: &'a ResourceId,
    pub(crate) normalized_colour: Option<&'a ResourceId>,
    pub(crate) reconstruction_twin: Option<(ResourceId, PassBody)>,
}

/// Provider configuration and graph declaration, shared by the view's slots.
pub(crate) trait NativeProvider: wgpu::WasmNotSendSync {
    fn declare(
        &self,
        graph: &mut FrameGraph,
        inputs: NativeGraphInputs<'_>,
        extent: [u32; 2],
    ) -> Result<NativeGraph, RenderError>;

    fn bind(
        &self,
        textures: &GraphTextures,
        graph: &NativeGraph,
    ) -> Result<Arc<NativeBinding>, RenderError>;
}

/// Prepared resources for one slot. A recording hook may only record on the
/// supplied encoder. In particular, native code ends only its own encoders
/// and never commits, enqueues or waits on the raw command buffer.
pub(crate) trait PreparedProvider: wgpu::WasmNotSendSync + Any {
    fn record_before_native(&self, _encoder: &mut wgpu::CommandEncoder, _reset: bool) {}

    fn record_twin(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        values: NativeFrameValues,
        reset: bool,
    );

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn record_native(&self, buffer: &super::CommandBuffer, values: NativeFrameValues, reset: bool);

    /// Publish only after the coordinator has submitted the recording.
    fn submitted(&self, reset: bool, twin: bool);

    fn output(&self) -> &wgpu::Texture;
    fn output_view(&self) -> &wgpu::TextureView;
    fn history(&self) -> Option<&wgpu::Texture> {
        None
    }
}

#[derive(Clone, Copy)]
pub(crate) enum NativePolicy {
    /// Ordinary provider admission follows the registry.
    Recipe,
    /// Explicit native comparison on a Metal device, independent of scene recipe.
    DiagnosticMetal,
    /// Explicitly select a prepared twin.
    Twin(TwinReason),
}

pub(crate) struct NativePreparation {
    twin: Option<TwinReason>,
}

impl NativePreparation {
    pub(crate) fn new(capabilities: &DeviceCapabilities, policy: NativePolicy) -> Self {
        let twin = match policy {
            NativePolicy::Twin(reason) => Some(reason),
            NativePolicy::Recipe if capabilities.recipe() != Ok(Recipe::AppleTile) => {
                Some(TwinReason::PortableRecipe)
            }
            NativePolicy::Recipe | NativePolicy::DiagnosticMetal => {
                if cfg!(any(target_os = "macos", target_os = "ios"))
                    && capabilities.adapter().backend == wgpu::Backend::Metal
                {
                    None
                } else {
                    Some(TwinReason::UnsupportedBackend)
                }
            }
        };
        Self { twin }
    }

    pub(crate) fn twin_reason(&self) -> Option<TwinReason> {
        self.twin
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub(crate) fn prepare_metal<T, E>(
        &mut self,
        site: &crate::runtime::DeviceSite,
        counted: bool,
        prepare: impl FnOnce() -> Result<T, E>,
    ) -> Option<T> {
        if self.twin.is_some() {
            return None;
        }
        if let Ok(provider) = prepare() {
            site.native_prepared(counted);
            Some(provider)
        } else {
            self.twin = Some(TwinReason::PreparationRefused);
            None
        }
    }

    /// Share this state only among the slots for one provider history. A new
    /// generation or incompatible resource layout creates a fresh state.
    pub(crate) fn state(
        &self,
        pass: &'static str,
        twin_name: &'static str,
        twin_resets_on_discard: bool,
    ) -> Arc<NativeState> {
        Arc::new(NativeState {
            twin_name,
            twin_resets_on_discard,
            candidate: Mutex::new(CandidateState {
                report: NativeProviderReport {
                    pass,
                    twin: self.twin.map(|reason| NativeTwinSelection {
                        name: twin_name,
                        reason,
                    }),
                    encoded: 0,
                    submitted: 0,
                    discarded: 0,
                    discards_kept_history: 0,
                    resets: 0,
                    completed: 0,
                },
                reserved: false,
                valid: false,
                failed: false,
                fail_next: false,
            }),
        })
    }
}

pub(crate) struct NativeState {
    twin_name: &'static str,
    twin_resets_on_discard: bool,
    candidate: Mutex<CandidateState>,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "Reservation, reusable history, failure latch and test injection are independent"
)]
struct CandidateState {
    report: NativeProviderReport,
    reserved: bool,
    valid: bool,
    failed: bool,
    fail_next: bool,
}

pub(crate) struct NativeBinding {
    state: Arc<NativeState>,
    provider: Arc<dyn PreparedProvider>,
}

impl NativeBinding {
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            clippy::arc_with_non_send_sync,
            reason = "WebGPU provider resources stay on the browser thread"
        )
    )]
    pub(crate) fn new(state: &Arc<NativeState>, provider: Arc<dyn PreparedProvider>) -> Arc<Self> {
        Arc::new(Self {
            state: Arc::clone(state),
            provider,
        })
    }

    pub(crate) fn report(&self) -> NativeProviderReport {
        self.state.candidate.lock().expect("native state").report
    }

    pub(crate) fn output(&self) -> &wgpu::Texture {
        self.provider.output()
    }

    pub(crate) fn output_view(&self) -> &wgpu::TextureView {
        self.provider.output_view()
    }

    pub(crate) fn prepared<T: PreparedProvider>(&self) -> Option<&T> {
        (self.provider.as_ref() as &dyn Any).downcast_ref()
    }

    pub(crate) fn inject_completion_error(&self) {
        self.state.candidate.lock().expect("native state").fail_next = true;
    }

    pub(crate) fn reject(&self) {
        let mut state = self.state.candidate.lock().expect("native state");
        state.report.twin = Some(NativeTwinSelection {
            name: self.state.twin_name,
            reason: TwinReason::Rejected,
        });
        state.valid = false;
    }

    pub(crate) fn reserve(
        self: &Arc<Self>,
        cut: bool,
        values: NativeFrameValues,
    ) -> Result<NativeFrame, RenderError> {
        let mut state = self.state.candidate.lock().expect("native state");
        if state.reserved {
            return Err(RenderError::NativeCandidateReserved);
        }
        let reset = cut || !state.valid;
        state.reserved = true;
        state.report.encoded += 1;
        state.report.resets += u64::from(reset);
        Ok(NativeFrame {
            binding: Arc::clone(self),
            values,
            reset,
            twin: state.report.twin.is_some(),
            fail_completion: std::mem::take(&mut state.fail_next),
            settled: false,
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            receipt: Mutex::new(None),
        })
    }
}

/// Exclusive candidate. Unclassified drops invalidate history.
#[expect(
    clippy::struct_excessive_bools,
    reason = "Independent candidate decisions"
)]
pub(crate) struct NativeFrame {
    binding: Arc<NativeBinding>,
    values: NativeFrameValues,
    reset: bool,
    twin: bool,
    fail_completion: bool,
    settled: bool,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    receipt: Mutex<Option<super::metal::Receipt>>,
}

impl NativeFrame {
    pub(crate) fn content_extent(&self) -> [u32; 2] {
        self.values.content_extent
    }

    pub(crate) fn uses_twin(&self) -> bool {
        self.twin
    }

    pub(crate) fn record_before_native(&self, encoder: &mut wgpu::CommandEncoder) {
        self.binding
            .provider
            .record_before_native(encoder, self.reset);
    }

    pub(crate) fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        segments: &mut Vec<wgpu::CommandBuffer>,
    ) {
        let provider = &self.binding.provider;
        if self.twin {
            provider.record_twin(encoder, self.values, self.reset);
            return;
        }
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        {
            let _ = (device, segments);
            unreachable!("native recording requires the admitted Apple backend");
        }
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            let preceding = std::mem::replace(
                encoder,
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default()),
            );
            segments.push(preceding.finish());
            let mut raw = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(self.binding.report().pass),
            });
            let receipt = super::metal::record(&mut raw, |buffer| {
                provider.record_native(buffer, self.values, self.reset);
            });
            *self.receipt.lock().expect("exclusive native receipt") = Some(receipt);
            segments.push(raw.finish());
        }
    }

    pub(crate) fn submitted(
        mut self,
        generation: u64,
        submission: u64,
        timing: bool,
    ) -> (NativeCompletion, NativeSegment) {
        let mut state = self.binding.state.candidate.lock().expect("native state");
        self.binding.provider.submitted(self.reset, self.twin);
        state.report.submitted += 1;
        state.valid = !state.failed;
        state.reserved = false;
        self.settled = true;
        let segment = NativeSegment {
            pass: state.report.pass,
            generation,
            submission,
            outcome: Arc::new(Mutex::new(NativeOutcome::Pending)),
        };
        (
            NativeCompletion {
                state: Arc::clone(&self.binding.state),
                segment: segment.clone(),
                timing,
                fail_completion: self.fail_completion,
                #[cfg(any(target_os = "macos", target_os = "ios"))]
                receipt: self
                    .receipt
                    .lock()
                    .expect("exclusive native receipt")
                    .take(),
                // Only native targets can have native handles. wgpu retains its
                // own resources on the browser thread.
                #[cfg(not(target_arch = "wasm32"))]
                _binding: Arc::clone(&self.binding),
            },
            segment,
        )
    }

    /// The healthy refusal path is the only discard that may keep history.
    pub(crate) fn refuse_acquisition(mut self) -> bool {
        let keep_history = self.twin && !self.binding.state.twin_resets_on_discard;
        self.discard(keep_history)
    }

    fn discard(&mut self, keep_history: bool) -> bool {
        let mut state = self.binding.state.candidate.lock().expect("native state");
        state.valid &= keep_history;
        state.report.discarded += 1;
        state.report.discards_kept_history += u64::from(state.valid);
        state.reserved = false;
        self.settled = true;
        state.valid
    }

    pub(crate) fn history(&self) -> Option<&wgpu::Texture> {
        self.binding.provider.history()
    }
}

impl Drop for NativeFrame {
    fn drop(&mut self) {
        if !self.settled {
            self.discard(false);
        }
    }
}

pub(crate) struct NativeCompletion {
    state: Arc<NativeState>,
    segment: NativeSegment,
    timing: bool,
    fail_completion: bool,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    receipt: Option<super::metal::Receipt>,
    #[cfg(not(target_arch = "wasm32"))]
    _binding: Arc<NativeBinding>,
}

impl NativeCompletion {
    pub(crate) fn completed(self) {
        let outcome = self.observe();
        let mut state = self.state.candidate.lock().expect("native state");
        if outcome == NativeOutcome::Failed {
            state.valid = false;
            state.failed = true;
        }
        state.report.completed += 1;
        *self
            .segment
            .outcome
            .lock()
            .expect("native completion mailbox") = outcome;
    }

    fn observe(&self) -> NativeOutcome {
        if self.fail_completion {
            return NativeOutcome::Failed;
        }
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        if let Some(receipt) = &self.receipt {
            return match receipt.completed(self.timing) {
                Err(()) => NativeOutcome::Failed,
                Ok(Some(times)) => NativeOutcome::Timed(times),
                Ok(None) => NativeOutcome::Unmeasured,
            };
        }
        let _ = self.timing;
        NativeOutcome::Unmeasured
    }
}
