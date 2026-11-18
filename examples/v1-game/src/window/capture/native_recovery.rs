//! Labelled provider failures through the ordinary game and T2 submitted tap.
use super::{CaptureSource, Duration, Game, ImageWriter, Instant, Offscreen, Trace, json};
use thrum_app::offscreen::NativeFault;

pub(super) fn yaw(time: f64, mode: super::super::NativeRecovery) -> f64 {
    // Keep input moving well after the recovery warmups, so a stale recovered
    // image cannot pass merely by matching the final stationary camera.
    let duration = if mode == super::super::NativeRecovery::Disabled {
        2.0
    } else {
        14.0
    };
    ((time - 1.0) / duration).clamp(0.0, 1.0) * 36.0
}

pub(super) struct Recovery<'a> {
    control: Option<bool>,
    next: usize,
    generation: u64,
    hardware: &'a [super::super::HardwareRequest],
}

impl<'a> Recovery<'a> {
    pub(super) fn new(
        mode: super::super::NativeRecovery,
        hardware: &'a [super::super::HardwareRequest],
    ) -> Self {
        let control = match mode {
            super::super::NativeRecovery::Disabled => None,
            super::super::NativeRecovery::Control => Some(true),
            super::super::NativeRecovery::Faults => Some(false),
        };
        Self {
            control,
            next: 0,
            generation: 1,
            hardware,
        }
    }

    pub(super) async fn checkpoint<G: Game>(
        &mut self,
        instance: &wgpu::Instance,
        driver: &mut Offscreen<G>,
        trace: &mut Trace,
        writer: Option<&ImageWriter>,
        origin: Instant,
        scenario_time: Duration,
        source: CaptureSource,
    ) -> Result<(), String> {
        let Some(control) = self.control else {
            return Ok(());
        };
        let Some(&(threshold, fault)) = [
            // Odd reset-relative spans make an omitted binary-tile reset
            // observable instead of accidentally matching its natural parity.
            (121.0 / 120.0, NativeFault::Surface),
            (182.0 / 120.0, NativeFault::CommandError),
            (243.0 / 120.0, NativeFault::DeviceLoss),
            (304.0 / 120.0, NativeFault::Reject),
        ]
        .get(self.next) else {
            return Ok(());
        };
        if scenario_time < Duration::from_secs_f64(threshold) {
            return Ok(());
        }
        super::drain_fixed(driver, trace, writer, origin)?;
        let deadline = Instant::now() + Duration::from_secs(10);
        while driver.pending_frames() != 0 {
            trace.drain(driver, writer, origin, true)?;
            if Instant::now() > deadline {
                return Err("native recovery drain deadline".into());
            }
        }
        let before = driver.native_witness_report();
        let requested = origin.elapsed();
        // An ordinary witness-Off control has no raw native segment to fail.
        // Its device-loss and surface arms still exercise real recovery.
        let applied = if control || (before.is_none() && matches!(fault, NativeFault::CommandError))
        {
            NativeFault::Control
        } else {
            fault
        };
        driver
            .native_fault(applied)
            .map_err(|error| error.to_string())?;
        let needs_device = matches!(applied, NativeFault::CommandError | NativeFault::DeviceLoss);
        if needs_device {
            self.generation += 1;
            let (_, _, queue, capabilities) =
                super::metal_device(instance, self.generation, self.hardware).await?;
            driver
                .recover_device(&capabilities, &queue)
                .await
                .map_err(|error| error.to_string())?;
            // Rebuild the game's actual cooked contribution before restarting
            // the tap. World/controls are retained; the host-time gap is logged.
            super::warmup(driver)?;
        }
        if needs_device {
            // The new provider already reset while rebuilding. Start capture
            // at a known phase after the uncaptured content warmup; surface
            // and rejection arms must rely on their actual engine resets.
            driver.invalidate_history();
        }
        super::enable_stage_capture(driver, self.next as u64 + 2, source)?;
        trace.stalls.push(json!({"kind":"native-provider-checkpoint", "fault":format!("{fault:?}"), "applied":format!("{applied:?}"),
            "control":control,"scenario_ns":scenario_time.as_nanos(), "requested_ns":requested.as_nanos(),
            "returned_ns":origin.elapsed().as_nanos(),"new_generation":self.generation,
            "before":format!("{before:?}"),"after":format!("{:?}",driver.native_witness_report()),
            "raw_error_injection":"CommandError injects a completion observation after actual provider writes; not a physical GPU fault"}));
        self.next += 1;
        Ok(())
    }
}
