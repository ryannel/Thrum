//! Recent completed-frame readouts. No timers: shells supply callback and completion times.
use std::{collections::VecDeque, time::Duration};

use super::TABLE;
use thrum_controls::Controls;
use thrum_render::runtime::FrameSample;

const CAPACITY: usize = 120;
const REFRESH: Duration = Duration::from_millis(250);
const STALE: Duration = Duration::from_secs(2);
type CpuStage = (&'static str, fn(&FrameSample) -> Duration);

/// One bounded, live window shared by the native and browser shells.
pub(crate) struct ProfileWindow {
    samples: VecDeque<FrameSample>,
    intervals: VecDeque<f64>,
    newest: Option<Duration>,
    callback: Option<Duration>,
    refreshed: Option<Duration>,
    enabled: bool,
    generation: u64,
    minimum: u64,
    names: Vec<String>,
    untimed: Vec<String>,
    available: bool,
}

impl ProfileWindow {
    #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(CAPACITY),
            intervals: VecDeque::with_capacity(CAPACITY),
            newest: None,
            callback: None,
            refreshed: None,
            enabled: false,
            generation: 0,
            minimum: 0,
            names: Vec::new(),
            untimed: Vec::new(),
            available: true,
        }
    }

    pub(crate) fn configure(&mut self, enabled: bool, generation: u64, minimum: u64) {
        if generation != self.generation {
            self.names.clear();
            self.untimed.clear();
        }
        if enabled != self.enabled || generation != self.generation {
            self.samples.clear();
            self.intervals.clear();
            self.newest = None;
            self.callback = None;
            self.refreshed = None;
            self.enabled = enabled;
            self.generation = generation;
        }
        self.minimum = minimum;
    }

    pub(crate) fn callback(&mut self, timestamp: Duration) {
        if !self.enabled {
            return;
        }
        if let Some(previous) = self.callback
            && let Some(interval) = timestamp.checked_sub(previous)
        {
            if self.intervals.len() == CAPACITY {
                self.intervals.pop_front();
            }
            self.intervals.push_back(interval.as_secs_f64() * 1e3);
        }
        self.callback = Some(timestamp);
    }

    pub(crate) fn set_available(&mut self, available: bool) {
        if available != self.available {
            self.available = available;
            self.samples.clear();
            self.intervals.clear();
            self.newest = None;
            self.callback = None;
            self.refreshed = None;
        }
    }

    pub(crate) fn push(&mut self, sample: &FrameSample, now: Duration) {
        if !self.enabled || !self.available || sample.submission_order < self.minimum {
            return;
        }
        if self
            .newest
            .is_some_and(|last| now.saturating_sub(last) > STALE)
        {
            self.samples.clear();
            self.intervals.clear();
            self.callback = None;
            self.refreshed = None;
        }
        if let Some(timings) = &sample.gpu_pass_ns
            && !self
                .names
                .iter()
                .map(String::as_str)
                .eq(timings.passes().iter().map(|p| p.name.as_ref()))
        {
            self.names = timings
                .passes()
                .iter()
                .map(|pass| pass.name.to_string())
                .collect();
            self.untimed = timings
                .values()
                .into_iter()
                .filter(|(_, value)| value.is_none())
                .map(|(name, _)| name.to_owned())
                .collect();
        }
        if self.samples.len() == CAPACITY {
            self.samples.pop_front();
        }
        self.samples.push_back(sample.clone());
        self.newest = Some(now);
    }

    pub(crate) fn publish(
        &mut self,
        controls: &mut Controls,
        now: Duration,
        callbacks: u64,
        presented: u64,
        web: bool,
    ) {
        if !self.enabled && self.refreshed.is_some() {
            return;
        }
        let Some(id) = controls.find(TABLE) else {
            return;
        };
        if self
            .refreshed
            .is_some_and(|last| now.saturating_sub(last) < REFRESH)
        {
            return;
        }
        self.refreshed = Some(now);
        let rows = self.rows(now, callbacks, presented, web);
        // The declaration's typed handle is recovered only within this module's reserved group.
        controls.replace_rows(id, rows);
    }

    fn rows(
        &self,
        now: Duration,
        callbacks: u64,
        presented: u64,
        web: bool,
    ) -> Vec<(String, String)> {
        let reason = if !self.enabled {
            Some("profiler off")
        } else if !self.available {
            Some("no recent completed frame (hidden or lost device)")
        } else if self.newest.is_none() {
            Some("waiting for completed frames")
        } else if self.newest.is_some_and(|t| now.saturating_sub(t) > STALE) {
            Some("no recent completed frame")
        } else {
            None
        };
        let mut rows = vec![(
            "Window".into(),
            if self.enabled {
                format!(
                    "median of latest {} / {CAPACITY} frames; newest #{}; 250 ms refresh",
                    self.samples.len(),
                    self.samples.back().map_or(0, |s| s.submission_order)
                )
            } else {
                "profiler off; window cleared".into()
            },
        )];
        rows.push((
            "Callback interval".into(),
            reason.map_or_else(
                || {
                    format!(
                        "{} (latest {} callbacks)",
                        format_ms(median(self.intervals.iter().copied()), None),
                        self.intervals.len()
                    )
                },
                str::to_owned,
            ),
        ));
        rows.push((
            "Present calls / missed callbacks (total)".into(),
            if self.enabled {
                format!("{presented} / {}", callbacks.saturating_sub(presented))
            } else {
                "profiler off".into()
            },
        ));
        let stages: [CpuStage; 4] = [
            ("World tick", |s: &FrameSample| s.tick_cpu),
            ("Extract", |s: &FrameSample| s.extract_cpu),
            ("Upload", |s: &FrameSample| s.upload_cpu),
            ("Encode + submit", |s: &FrameSample| s.encode_cpu),
        ];
        for (label, field) in stages {
            let text = reason.map_or_else(
                || {
                    if label == "World tick"
                        && self
                            .samples
                            .iter()
                            .all(|s| s.completed_ticks == 0 && s.tick_cpu.is_zero())
                    {
                        "no world tick in window".into()
                    } else {
                        format_ms(
                            median(self.samples.iter().map(|s| field(s).as_secs_f64() * 1e3)),
                            None,
                        )
                    }
                },
                str::to_owned,
            );
            rows.push((label.into(), text));
        }
        let latest = self.samples.back();
        let gpu_reason = reason.or_else(|| {
            latest.and_then(|s| {
                if !s.timing_requested {
                    Some("GPU time not requested")
                } else if s.timing_fault {
                    Some("timing batch faulted")
                } else if s.gpu_pass_ns.is_none() {
                    Some("device has no timestamp queries")
                } else {
                    None
                }
            })
        });
        // Keep graph names through off/fault/stale states, replacing every value with its reason.
        if self.names.is_empty() {
            rows.push((
                "GPU passes".into(),
                gpu_reason.unwrap_or("waiting for timestamps").into(),
            ));
        } else {
            for name in &self.names {
                let text = gpu_reason.map_or_else(
                    || {
                        let values: Vec<f64> = self
                            .samples
                            .iter()
                            .filter_map(|s| s.gpu_pass_ns.as_ref()?.get(name))
                            .map(|v| v / 1e6)
                            .collect();
                        let step = web.then(|| browser_step(&values));
                        format_ms(median(values.iter().copied()), step)
                    },
                    str::to_owned,
                );
                rows.push((format!("GPU {name}"), text));
            }
        }
        for name in &self.untimed {
            rows.push((
                format!("GPU {name}"),
                gpu_reason
                    .unwrap_or("not timestamped (transfer/resolve)")
                    .into(),
            ));
        }
        rows
    }
}

fn median(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut values: Vec<_> = values.filter(|v| v.is_finite()).collect();
    values.sort_by(f64::total_cmp);
    if values.is_empty() {
        return None;
    }
    let middle = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        values[middle - 1].midpoint(values[middle])
    } else {
        values[middle]
    })
}

fn browser_step(values: &[f64]) -> f64 {
    // Chrome exposes no quantization policy. Infer the observed integer-nanosecond
    // lattice; all-zero windows use Chrome's default 0.1 ms upper bound.
    let mut lattice = 0;
    for value in values {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "validated nonnegative GPU durations are rounded to integer nanoseconds for their observed lattice"
        )]
        let mut right = (value * 1e6).round() as u64;
        while right != 0 {
            let remainder = lattice % right;
            lattice = right;
            right = remainder;
        }
    }
    if lattice == 0 {
        0.1
    } else {
        lattice as f64 / 1e6
    }
}

fn format_ms(value: Option<f64>, step: Option<f64>) -> String {
    match value {
        None => "waiting for samples".into(),
        Some(value) if step.is_some_and(|step| value < step) => {
            format!("< {} ms (observed step)", step.unwrap_or_default())
        }
        Some(0.0) => "below timer resolution".into(),
        Some(value) if value < 0.001 => step.map_or_else(
            || "< 0.001 ms (readout precision)".into(),
            |step| format!("< 0.001 ms (step {step} ms)"),
        ),
        Some(value) => step.map_or_else(
            || format!("{value:.3} ms"),
            |step| format!("{value:.3} ms (step {step} ms)"),
        ),
    }
}

#[cfg(test)]
mod tests;
