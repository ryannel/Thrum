use super::*;
use crate::profile::declare;

fn sample(order: u64) -> FrameSample {
    use thrum_render::synthetic::{CullMode, CullingReason, CullingSelection};
    FrameSample {
        native_segment: None,
        display_phase: 1.0,
        observation: Duration::ZERO,
        presentation_lead: Duration::ZERO,
        clock_interpolating: false,
        tick_due_time: None,
        platform_target: None,
        lead_status: None,
        tick: order,
        submission_order: order,
        history_identity: [0; 2],
        jitter_phase: None,
        render_extent: [64; 2],
        governor: None,
        completed_ticks: 1,
        queued_ticks: 0,
        tick_cpu: Duration::from_micros(order),
        extract_cpu: Duration::ZERO,
        visual_age_ticks: 0,
        presentation_alpha: 1.0,
        upload_cpu: Duration::ZERO,
        view_update_ns: 0,
        encode_cpu: Duration::ZERO,
        wall: Duration::ZERO,
        render_cpu: Duration::ZERO,
        gpu_pass_ns: None,
        gpu_present_ns: None,
        gpu_contribution_ns: None,
        surface_presented: true,
        surface_fault: None,
        timing_fault: false,
        timing_requested: true,
        culling_mode: CullMode::Direct,
        shadow_selection: thrum_render::synthetic::ShadowSelection::default(),
        ambient_occlusion: thrum_render::capabilities::AmbientOcclusionSelection::default(),
        culling_selection: CullingSelection {
            requested: CullMode::Direct,
            resolved: CullMode::Direct,
            reason: CullingReason::RequestedDirect,
        },
        camera_culling: None,
        sprites: None,
        uploaded_rows: 0,
        uploaded_bytes: 0,
        moving: 0,
        dropped_presentation: 0,
        dropped_light_references: 0,
        clamped_light_channels: 0,
        clamped_shadow_casters: 0,
        visual_revision: None,
    }
}

#[test]
fn coarse_active_ticks_are_not_reported_as_an_idle_world() {
    let mut window = ProfileWindow::new();
    window.configure(true, 1, 1);
    let mut frame = sample(1);
    frame.tick_cpu = Duration::ZERO;
    window.push(&frame, Duration::ZERO);
    assert_eq!(
        window.rows(Duration::ZERO, 1, 1, true)[3].1,
        "below timer resolution"
    );

    frame.completed_ticks = 0;
    for order in 2..=121 {
        frame.submission_order = order;
        window.push(&frame, Duration::ZERO);
    }
    assert_eq!(
        window.rows(Duration::ZERO, 121, 121, true)[3].1,
        "no world tick in window"
    );

    window.configure(true, 2, 122);
    frame.submission_order = 122;
    frame.tick_cpu = Duration::from_micros(100);
    window.push(&frame, Duration::ZERO);
    assert_eq!(window.rows(Duration::ZERO, 1, 1, true)[3].1, "0.100 ms");
}

#[test]
fn late_completions_roll_the_bounded_window_and_every_unavailable_reason_replaces_numbers() {
    let mut window = ProfileWindow::new();
    window.configure(true, 1, 5);
    window.push(&sample(4), Duration::ZERO);
    assert!(window.samples.is_empty());
    for order in 5..=900 {
        window.push(&sample(order), Duration::from_secs(1));
    }
    assert_eq!(window.samples.len(), CAPACITY);
    assert_eq!(window.samples.front().unwrap().submission_order, 781);
    let rows = window.rows(Duration::from_secs(1), 950, 900, true);
    assert_eq!(
        rows.iter()
            .find(|(name, _)| name == "World tick")
            .unwrap()
            .1,
        "0.841 ms"
    );
    assert!(
        rows.iter()
            .any(|(_, text)| text == "device has no timestamp queries")
    );
    let mut fault = sample(901);
    fault.timing_fault = true;
    window.push(&fault, Duration::from_secs(1));
    assert!(
        window
            .rows(Duration::from_secs(1), 950, 900, true)
            .iter()
            .any(|(_, text)| text == "timing batch faulted")
    );
    fault.timing_requested = false;
    window.push(&fault, Duration::from_secs(1));
    assert!(
        window
            .rows(Duration::from_secs(1), 950, 900, true)
            .iter()
            .any(|(_, text)| text == "GPU time not requested")
    );
    assert!(
        window
            .rows(Duration::from_secs(4), 950, 900, true)
            .iter()
            .any(|(_, text)| text == "no recent completed frame")
    );
    window.configure(false, 1, 902);
    assert!(window.samples.is_empty());
    window.configure(true, 2, 902);
    assert!(
        window
            .rows(Duration::from_secs(4), 950, 900, true)
            .iter()
            .any(|(_, text)| text == "waiting for completed frames")
    );
    window.push(&sample(901), Duration::from_secs(4));
    assert!(window.samples.is_empty());
}

#[test]
fn median_uses_both_centre_values_and_ignores_nonfinite_values() {
    assert_eq!(
        median([3.0, 1.0, 2.0, 4.0, f64::NAN].into_iter()),
        Some(2.5)
    );
    assert_eq!(median(std::iter::empty()), None);
}

#[test]
fn browser_zero_means_below_the_observed_step() {
    assert_eq!(browser_step(&[0.0, 0.1, 0.3]), 0.1);
    assert!(format_ms(Some(0.0), Some(0.1)).starts_with("< 0.1 ms"));
    assert_eq!(format_ms(Some(0.0), None), "below timer resolution");
}

#[test]
fn transitions_clear_samples_and_refresh_immediately_then_at_four_hz() {
    let mut controls = Controls::new();
    declare(&mut controls).unwrap();
    let mut window = ProfileWindow::new();
    window.configure(true, 1, 5);
    window.callback(Duration::ZERO);
    window.callback(Duration::from_millis(10));
    window.publish(&mut controls, Duration::ZERO, 2, 1, false);
    let revision = controls.revision();
    window.publish(&mut controls, Duration::from_millis(249), 3, 2, false);
    assert_eq!(controls.revision(), revision);
    window.publish(&mut controls, Duration::from_millis(250), 3, 2, false);
    assert!(controls.revision() > revision);
    window.configure(false, 1, 5);
    assert!(window.intervals.is_empty());
    assert_eq!(
        window.rows(Duration::ZERO, 2, 1, false)[2].1,
        "profiler off"
    );
    assert!(
        window
            .rows(Duration::ZERO, 2, 1, false)
            .iter()
            .any(|(_, value)| value == "profiler off")
    );
    window.names.push("retired device pass".into());
    window.configure(true, 2, 10);
    assert_eq!(window.names, Vec::<String>::new());
    assert!(window.samples.is_empty());
    assert!(window.newest.is_none());
}
