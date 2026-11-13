use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// One-shot failure at the staging chunk's mapped-write boundary.
/// Compiled only with `fault-probe`; each residency has its own probe.
#[derive(Clone, Debug, Default)]
pub struct FaultProbe(Arc<AtomicBool>);

impl FaultProbe {
    /// Refuse the next chunk fill through the production abandon branch.
    pub fn fail_next_fill(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub(crate) fn take(&self) -> bool {
        self.0.swap(false, Ordering::AcqRel)
    }
}
