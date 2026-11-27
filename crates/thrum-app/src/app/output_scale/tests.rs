use super::*;
use RenderScale::{Half, Native, Percent75};

const WINDOW: [u32; 2] = [1920, 1080];
const FULLSCREEN: [u32; 2] = [3456, 2234];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

fn at(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

fn policy(fullscreen: bool) -> OutputScale {
    resolve(SceneKind::ThreeD, fullscreen, 120.0, Percent75, None)
}

fn windowed() -> ModeChange {
    let mut change = ModeChange::default();
    assert_eq!(change.observe(WINDOW, policy).unwrap().scale.step, Native);
    change
}

#[test]
fn output_scale_policy_and_rounding() {
    use thrum_render::capabilities::DeviceCapabilities;
    assert_eq!(DeviceCapabilities::mac_fullscreen_output_scale(), Native);
    assert_eq!(policy(false).step, Native);
    assert_eq!(
        resolve(SceneKind::ThreeD, true, 60.0, Percent75, None).step,
        Native
    );
    assert_eq!(
        policy(true),
        OutputScale {
            step: Percent75,
            forced: false
        }
    );
    for forced in [None, Some(Half)] {
        assert_eq!(
            resolve(SceneKind::TwoD, true, 120.0, Percent75, forced),
            OutputScale {
                step: Native,
                forced: false
            }
        );
    }
    for (step, extent) in RenderScale::STEPS.into_iter().zip([
        FULLSCREEN,
        [2938, 1899],
        [2592, 1676],
        [2316, 1497],
        [1728, 1117],
    ]) {
        for fullscreen in [false, true] {
            assert_eq!(
                resolve(SceneKind::ThreeD, fullscreen, 120.0, Percent75, Some(step)),
                OutputScale { step, forced: true }
            );
        }
        assert_eq!(
            OutputRequest {
                backing: FULLSCREEN,
                scale: OutputScale { step, forced: true }
            }
            .generation(0, FORMAT, 1)
            .output_extent(),
            extent
        );
    }
}

#[test]
fn mode_change_enters_once_with_early_platform_flag_and_intermediate_extents() {
    let mut change = windowed();
    change.event(ModeEvent::Start(true), at(100));
    for (ms, backing) in [
        (100, WINDOW),
        (600, WINDOW),
        (700, [2500, 1400]),
        (800, [3100, 1800]),
    ] {
        assert_eq!(change.expire(at(ms), || panic!("early mode read")), None);
        assert_eq!(change.observe(backing, |_| policy(true)), None);
        assert_eq!(change.current().unwrap().backing, WINDOW);
    }
    change.event(ModeEvent::Complete(true), at(900));
    let accepted = change.observe(FULLSCREEN, policy).unwrap();
    assert_eq!(accepted.backing, FULLSCREEN);
    assert_eq!(accepted.scale.step, Percent75);
    assert_eq!(change.observe(FULLSCREEN, policy), None);

    // Without the begin/complete hold, reading the early platform flag leaves
    // a mixed request exposed for five times the render owner's debounce.
    let mut control = windowed();
    let mixed = control.observe(WINDOW, |_| policy(true)).unwrap();
    assert_eq!((mixed.backing, mixed.scale.step), (WINDOW, Percent75));
    assert_eq!(control.observe(WINDOW, |_| policy(true)), None);
    assert_eq!(control.current(), Some(mixed));
}

#[test]
fn mode_change_leaves_once_with_late_platform_flag() {
    let mut change = windowed();
    change.event(ModeEvent::Complete(true), at(0));
    assert_eq!(
        change.observe(FULLSCREEN, policy).unwrap().scale.step,
        Percent75
    );
    change.event(ModeEvent::Start(false), at(100));
    for ms in [100, 600] {
        assert_eq!(change.expire(at(ms), || panic!("early mode read")), None);
        assert_eq!(change.observe(WINDOW, |_| policy(true)), None);
        assert_eq!(change.current().unwrap().backing, FULLSCREEN);
    }
    change.event(ModeEvent::Complete(false), at(700));
    let accepted = change.observe(WINDOW, policy).unwrap();
    assert_eq!((accepted.backing, accepted.scale.step), (WINDOW, Native));
    assert_eq!(change.observe(WINDOW, policy), None);
}

#[test]
fn mode_change_uses_the_reported_mode_and_extent_at_the_limit() {
    let mut change = windowed();
    change.event(ModeEvent::Start(true), at(100));
    assert_eq!(change.expire(at(3099), || panic!("early mode read")), None);
    assert_eq!(change.deadline(), Some(at(3100)));
    assert_eq!(change.observe(FULLSCREEN, policy), None);
    let now = at(3100);
    assert_eq!(change.expire(now, || true), Some(true));
    let accepted = change.observe(FULLSCREEN, policy).unwrap();
    assert_eq!(
        (accepted.backing, accepted.scale.step),
        (FULLSCREEN, Percent75)
    );
    assert_eq!(change.observe(FULLSCREEN, policy), None);
    assert_eq!(change.expire(at(4000), || panic!("duplicate limit")), None);
}

#[test]
fn mode_change_allows_recovered_device_at_the_held_output_pair() {
    let mut change = windowed();
    change.event(ModeEvent::Start(true), at(100));
    assert_eq!(change.observe(FULLSCREEN, policy), None);
    let recover = change
        .current()
        .unwrap()
        .generation(2_f64.to_bits(), FORMAT, 2);
    assert_eq!(recover.extent, WINDOW);
    assert_eq!(recover.output_scale, Native);
    assert_eq!(recover.device, 2);
    assert!(change.held());
}

#[test]
fn mode_change_does_not_hold_an_ordinary_resize() {
    let mut change = windowed();
    for backing in [[1700, 900], [1600, 800], [1550, 780]] {
        let request = change.observe(backing, policy).unwrap();
        assert_eq!((request.backing, request.scale.step), (backing, Native));
    }
}
