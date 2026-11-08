//! Scratch-only diagnostic for the real Mac app and its surface owner.

use super::{FrameError, FrameHost, FrameWorld, Instant, PlatformGpu, SceneSource};
use std::{path::PathBuf, time::Duration};

pub(super) struct Witness {
    output: Option<PathBuf>,
    destroy: bool,
    presented: u32,
    captured_before: bool,
    captured_after: bool,
    fired: bool,
}

impl Witness {
    pub(super) fn from_env() -> Result<Self, String> {
        let output = std::env::var_os("THRUM_M1_WITNESS").map(PathBuf::from);
        let destroy = match std::env::var("THRUM_M1_DESTROY").as_deref() {
            Ok("1") => true,
            Ok("0") | Err(std::env::VarError::NotPresent) => false,
            _ => return Err("THRUM_M1_DESTROY must be 0 or 1".into()),
        };
        if destroy && output.is_none() {
            return Err("loss witness needs an output directory".into());
        }
        if let Some(output) = &output {
            std::fs::create_dir(output).map_err(|error| format!("witness directory: {error}"))?;
            eprintln!("m1 witness ready pid={} arm={destroy}", std::process::id());
        }
        Ok(Self { output, destroy, presented: 0, captured_before: false, captured_after: false, fired: false })
    }

    pub(super) fn before_frame<W: FrameWorld, V: SceneSource<W::Page>>(
        &mut self,
        host: &mut FrameHost<W, V>,
        generation: u64,
    ) -> Result<(), FrameError> {
        let Some(output) = &self.output else { return Ok(()); };
        let phase = if generation == 1 && self.presented >= 60 && !self.captured_before
            && output.join("capture-before").is_file()
        {
            self.captured_before = true;
            "before"
        } else if generation > 1 && self.presented >= 40 && !self.captured_after
            && output.join("capture-after").is_file()
        {
            self.captured_after = true;
            "after"
        } else {
            return Ok(());
        };
        let endpoint = host.world().source();
        let pixels = host.capture_pixels()?;
        std::fs::write(output.join(format!("{phase}.rgba")), pixels)
            .map_err(|error| FrameError::Scene(format!("witness RGB: {error}")))?;
        eprintln!("m1 image phase={phase} generation={generation} endpoint={endpoint:?}");
        Ok(())
    }

    pub(super) fn should_destroy(&self, generation: u64) -> bool {
        self.destroy && generation == 1 && self.captured_before && !self.fired
            && self.output.as_ref().is_some_and(|path| path.join("arm").is_file())
    }

    pub(super) fn destroy(&mut self, gpu: &PlatformGpu, surface: u64, sequence: u64) -> FrameError {
        self.fired = true;
        let generation = gpu.watch.generation;
        eprintln!("m1 acquired generation={generation} sequence={sequence} surface={surface}");
        eprintln!("m1 destroying generation={generation} sequence={sequence} queue={:?}", gpu.queue);
        gpu.device.destroy();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !gpu.watch.is_lost() && Instant::now() < deadline {
            if let Err(error) = gpu.device.poll(wgpu::PollType::Poll) {
                return FrameError::Gpu(format!("M1 loss maintenance: {error:?}"));
            }
            std::thread::yield_now();
        }
        if gpu.watch.is_lost() {
            eprintln!("m1 loss callback generation={generation} sequence={sequence}");
            FrameError::Gpu("device loss reported by callback".into())
        } else {
            FrameError::Gpu("M1 destroy fired but loss notification was absent".into())
        }
    }

    pub(super) fn record_abandon(&self, generation: u64, sequence: u64) {
        eprintln!("m1 abandoned generation={generation} sequence={sequence}");
    }

    pub(super) fn recovery_requested(&self, from: u64, generation: u64) {
        if self.output.is_some() {
            eprintln!("m1 recovery requested from={from} generation={generation}");
        }
    }

    pub(super) fn published(&mut self, gpu: &PlatformGpu) {
        if self.output.is_some() {
            self.presented = 0;
            eprintln!("m1 published generation={} queue={:?}", gpu.watch.generation, gpu.queue);
        }
    }

    pub(super) fn presented(&mut self, generation: u64, sequence: u64) {
        if self.output.is_some() {
            self.presented += 1;
            if self.presented == 1 {
                eprintln!("m1 first present generation={generation} sequence={sequence}");
                let unix_ms = std::time::SystemTime::UNIX_EPOCH.elapsed()
                    .unwrap_or_default().as_millis();
                eprintln!("m1 event first-present-{generation} unix_ms={unix_ms}");
            }
        }
    }

    pub(super) fn before_submit(&self, generation: u64, surface: u64, sequence: u64) {
        if self.output.is_some() && self.presented == 0 {
            eprintln!("m1 frame submit generation={generation} surface={surface} sequence={sequence}");
        }
    }

    pub(super) fn finish(&self) {
        if self.output.is_some() {
            eprintln!("m1 witness finish armed={} fired={} before={} after={}",
                self.destroy, self.fired, self.captured_before, self.captured_after);
        }
    }
}
