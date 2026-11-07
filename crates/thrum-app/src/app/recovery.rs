//! Main-thread admission of one GPU replacement; no world lifecycle state.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Ready,
    Queued(u64),
    Working(u64),
    Failed,
}

pub(super) struct Recovery {
    pub(super) active: u64,
    last_attempt: u64,
    phase: Phase,
    candidate_lost: bool,
}

pub(super) fn missing_profiles(
    presented: usize,
    retained: usize,
    lost_device: bool,
) -> Result<usize, String> {
    let missing = presented
        .checked_sub(retained)
        .ok_or("more presented samples than present calls")?;
    if missing != 0 && !lost_device {
        return Err(format!(
            "{retained} presented samples for {presented} presents"
        ));
    }
    Ok(missing)
}

impl Recovery {
    pub(super) fn new(generation: u64) -> Self {
        Self {
            active: generation,
            last_attempt: generation,
            phase: Phase::Ready,
            candidate_lost: false,
        }
    }

    pub(super) fn lost(&mut self, generation: u64) -> Result<bool, String> {
        if matches!(self.phase, Phase::Queued(next) | Phase::Working(next) if next == generation) {
            self.candidate_lost = true;
        }
        if generation != self.active || self.phase != Phase::Ready {
            return Ok(false);
        }
        self.queue()?;
        Ok(true)
    }

    fn queue(&mut self) -> Result<(), String> {
        self.last_attempt = self
            .last_attempt
            .checked_add(1)
            .ok_or("device generation exhausted")?;
        self.phase = Phase::Queued(self.last_attempt);
        self.candidate_lost = false;
        Ok(())
    }

    pub(super) fn retry(&mut self) -> Result<(), String> {
        if self.failed() {
            self.queue()?;
        }
        Ok(())
    }

    pub(super) fn queued(&self) -> Option<u64> {
        match self.phase {
            Phase::Queued(next) => Some(next),
            _ => None,
        }
    }

    pub(super) fn accepted(&mut self, generation: u64) {
        debug_assert_eq!(self.phase, Phase::Queued(generation));
        self.phase = Phase::Working(generation);
    }

    pub(super) fn complete(&mut self, attempted: u64, installed: u64, success: bool) -> bool {
        if self.phase != Phase::Working(attempted) {
            return false;
        }
        self.active = installed;
        self.phase = if success && !self.candidate_lost && installed == attempted {
            Phase::Ready
        } else {
            Phase::Failed
        };
        true
    }

    pub(super) fn ready(&self) -> bool {
        self.phase == Phase::Ready
    }
    pub(super) fn failed(&self) -> bool {
        self.phase == Phase::Failed
    }
    pub(super) fn label(&self) -> &'static str {
        match self.phase {
            Phase::Ready => "",
            Phase::Queued(_) | Phase::Working(_) => " | Restoring GPU",
            Phase::Failed => " | GPU unavailable (R retry)",
        }
    }
}

#[cfg(test)]
#[path = "recovery_tests.rs"]
mod tests;
