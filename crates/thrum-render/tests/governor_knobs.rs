//! An external contribution joins the host's governor using only the public quality seam.

mod support;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_render::{
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    contribution::{
        ContributionFrame, ContributionPasses, QualityError, QualityInvalidation, QualityKnob,
        QualitySettings, SceneContribution, SceneContributions, SceneSurface,
    },
    governor::{ChangeReason, DisplayRecord, ThermalState},
    present::SurfaceProfile,
    runtime::{FrameConfig, FrameError, FrameSample, SurfaceOutput},
    synthetic::{FrameLighting, RenderScale, SyntheticScene},
};

#[derive(Default)]
struct History {
    resets: u32,
    age_marks: u32,
}

struct Effect {
    quality: QualitySettings,
    history: Arc<Mutex<History>>,
}

impl Effect {
    fn new(
        name: &str,
        steps: [u32; 3],
        invalidation: QualityInvalidation,
    ) -> (Box<Self>, Arc<Mutex<History>>) {
        let history = Arc::new(Mutex::new(History::default()));
        let effect = Self {
            quality: QualitySettings::new(vec![QualityKnob {
                name: name.into(),
                effect: "Prepared effect updates per frame",
                value: steps[0],
                range: 1..=4,
                invalidation,
                governor_steps: Some(steps),
            }])
            .expect("admitted tier steps"),
            history: Arc::clone(&history),
        };
        (Box::new(effect), history)
    }
}

impl SceneContribution for Effect {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses::default()
    }

    fn rebuild(&mut self, _: &SceneSurface<'_>) -> Result<(), FrameError> {
        Ok(())
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}

    fn quality_knobs(&self) -> Vec<QualityKnob> {
        self.quality.knobs().to_vec()
    }

    fn set_quality_knob(&mut self, name: &str, value: u32) -> Result<(), QualityError> {
        self.quality.set(name, value)
    }

    fn commit_quality_knobs(&mut self) -> QualityInvalidation {
        let invalidation = self.quality.commit();
        let mut history = self.history.lock().expect("history");
        history.resets += u32::from(invalidation == QualityInvalidation::EffectHistory);
        history.age_marks += u32::from(invalidation == QualityInvalidation::Age);
        invalidation
    }
}

const EXTENT: [u32; 2] = [64; 2];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

fn frame(host: &mut support::Host, output: &wgpu::TextureView, time: Duration) -> FrameSample {
    let report = host
        .drive(
            time,
            Some(&SurfaceOutput {
                view: output,
                extent: EXTENT,
            }),
        )
        .expect("frame drive");
    assert!(report.submitted && report.surface_presented, "{report:?}");
    thrum_test_support::wait::until("governed Metal frame", Duration::from_secs(4), || {
        host.poll_completed().expect("Metal completion")
    })
}

thrum_test_support::macos_gpu_test! {
fn external_knobs_commit_by_tier_and_invalidate_only_their_owned_history() {
    let Some(thrum_test_support::Opened { gpu, device, queue, .. }) =
        thrum_test_support::open("governed external effect", Some(wgpu::Backend::Metal),
            thrum_test_support::Profile::Portable, &thrum_test_support::Restriction::default())
    else { return; };
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    let mut scene = SyntheticScene::grid(1).expect("small scene");
    scene.view = scene.view.with_reconstruction(true);
    let air = AtmosphereFrame::new([0.7, 0.7, 0.0], 100_000.0, [1.0; 3], 1.0, 1).expect("air");
    let mut host = pollster::block_on(support::host_with_config(&caps, &queue, scene, FrameLighting::Atmosphere(air),
        FrameConfig { render_extent: EXTENT, surface: Some(SurfaceProfile { format: FORMAT, extent: EXTENT }) }));
    let output = support::drawable(&device, FORMAT, EXTENT);
    let (changed, changed_history) = Effect::new("steps", [4, 3, 2], QualityInvalidation::EffectHistory);
    let (stable, stable_history) = Effect::new("unchanged", [3, 3, 3], QualityInvalidation::EffectHistory);
    let (cadenced, cadenced_history) = Effect::new("interval", [1, 2, 4], QualityInvalidation::Age);
    host.set_scene_contribution(Box::new(SceneContributions::new(vec![changed, stable, cadenced]))).expect("external attachment");
    let nominal = DisplayRecord { interval: Duration::from_secs(1), thermal: ThermalState::Nominal };
    host.set_render_scale(RenderScale::Half).expect("start at the last scale step");
    assert!(host.enable_governor(nominal));
    host.governor_display_tick(Duration::ZERO, 1);
    let first = frame(&mut host, &output, Duration::ZERO);
    host.governor_present_called();
    let authority = first.tick;
    let pipelines = host.creation_report().pipelines;
    host.governor_display_tick(Duration::from_millis(4), 5);
    let after_gap = frame(&mut host, &output, Duration::from_millis(4));
    assert_eq!(host.governor_missed_ticks(), 3, "coalesced ticks still count");
    assert_eq!(after_gap.governor.expect("state").state.tier, 0);
    host.governor_present_called();
    for tick in 6..=9 {
        host.governor_display_tick(Duration::from_millis((tick - 5) * 5), tick);
    }
    assert_eq!(host.governor_frame().expect("state").state.tier, 0, "staged between frames");
    let tier_one = frame(&mut host, &output, Duration::from_millis(20));
    host.governor_present_called();
    for tick in 10..=13 {
        host.governor_display_tick(Duration::from_millis(20 + (tick - 9) * 5), tick);
    }
    let tier_two = frame(&mut host, &output, Duration::from_millis(40));
    for (tier, sample) in [(1, tier_one), (2, tier_two)] {
        let quality = sample.governor.expect("quality frame");
        assert_eq!(sample.tick, authority);
        assert_eq!(sample.history_identity[1] + 1, sample.history_identity[0]);
        assert_eq!(quality.state.tier, tier);
        assert_eq!(quality.state.reason, ChangeReason::OverBudget);
        assert_eq!(quality.state.scale, RenderScale::Half);
        assert!(quality.aerial_age.is_some_and(|age| age > 0));
        for (suffix, value) in [
            ("steps", 4 - u32::from(tier)),
            ("unchanged", 3),
            ("interval", 1 << tier),
            ("aerial cadence", 1 << tier),
        ] {
            assert!(quality.knobs.iter().any(|knob| knob.name.ends_with(suffix) && knob.value == value), "{suffix}={value} {quality:?}");
        }
    }
    assert_eq!(changed_history.lock().expect("changed effect").resets, 2);
    assert_eq!(stable_history.lock().expect("unchanged effect").resets, 0);
    assert_eq!(cadenced_history.lock().expect("cadenced effect").resets, 0);
    assert_eq!(cadenced_history.lock().expect("cadenced effect").age_marks, 2);
    assert_eq!(host.creation_report().pipelines, pipelines);
    assert_eq!(host.creation_report().violations, 0);

    for name in ["atmosphere.aerial cadence", "interval"] {
        host.set_quality_knob(name, 2).expect("stage manual value");
        assert_eq!(host.quality_knobs().into_iter().find(|knob| knob.name == name).expect("registered").value, 4);
    }
    let edited = frame(&mut host, &output, Duration::from_millis(56));
    let edited = edited.governor.expect("edited frame state");
    for name in ["atmosphere.aerial cadence", "interval"] {
        assert_eq!(host.quality_knobs().into_iter().find(|knob| knob.name == name).expect("registered").value, 2);
        assert_eq!(edited.knobs.iter().find(|knob| knob.name == name).expect("recorded").value, 2);
    }
}

fn deferred_aerial_update_keeps_the_latest_sun_until_submit() {
    let Some(thrum_test_support::Opened { gpu, device, queue, .. }) =
        thrum_test_support::open("governed aerial update", Some(wgpu::Backend::Metal),
            thrum_test_support::Profile::Portable, &thrum_test_support::Restriction::default())
    else { return; };
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    let mut scene = SyntheticScene::grid(1).expect("small scene");
    scene.view = scene.view.with_reconstruction(true);
    let sky = |sun_x| FrameLighting::Atmosphere(
        AtmosphereFrame::new([sun_x, 0.7, 0.0], 100_000.0, [1.0; 3], 1.0, 1).expect("air"));
    let mut host = pollster::block_on(support::host_with_config(&caps, &queue, scene, sky(0.7),
        FrameConfig { render_extent: EXTENT, surface: Some(SurfaceProfile { format: FORMAT, extent: EXTENT }) }));
    let output = support::drawable(&device, FORMAT, EXTENT);
    assert!(host.enable_governor(DisplayRecord {
        interval: Duration::from_secs(1),
        thermal: ThermalState::Serious,
    }));
    frame(&mut host, &output, Duration::ZERO);
    for (index, expected_age) in [1, 2, 3, 0, 1].into_iter().enumerate() {
        let index = u64::try_from(index).unwrap();
        if index < 2 {
            host.continuous_source_mut().lighting = sky(if index == 0 { -0.5 } else { -0.7 });
        }
        let sample = frame(&mut host, &output, Duration::from_millis((index + 1) * 16));
        let quality = sample.governor.expect("quality frame");
        assert_eq!(quality.aerial_age, Some(expected_age), "frame {index}");
        assert!(quality.knobs.iter().any(|knob|
            knob.name == "atmosphere.aerial cadence" && knob.value == 4));
        let timings = sample.gpu_pass_ns.expect("Metal pass timestamps");
        let aerial_ran = timings.passes().iter().any(|pass| pass.name.contains("aerial"));
        assert_eq!(aerial_ran, expected_age == 0, "frame {index}");
    }
}
}
