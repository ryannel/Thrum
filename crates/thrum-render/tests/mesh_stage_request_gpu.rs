//! Explicit launch requests on this Mac, including a failed preferred device request.
#[cfg(target_os = "macos")]
use std::{cell::Cell, io::Write as _};
#[cfg(target_os = "macos")]
use thrum_render::capabilities::{DeviceCapabilities, HardwareRequest};

#[cfg(target_os = "macos")]
#[test]
fn metal_launch_and_preference_retry() {
    let gate = thrum_test_support::gpu("mesh-stage launch", Some(wgpu::Backend::Metal))
        .expect("MESH-SHADING requires this Mac");
    let unknown = HardwareRequest::parse("mesh_typo").expect_err("unknown hardware");
    assert!(unknown.to_string().contains("mesh_typo"), "{unknown}");
    let options = wgpu::RequestAdapterOptions::default();
    let base = |adapter: &wgpu::Adapter| {
        thrum_test_support::Profile::Portable
            .descriptor(adapter, &thrum_test_support::Restriction::default())
    };
    let required = HardwareRequest::parse("mesh-stages").expect("required stages");
    let preferred = HardwareRequest::parse("mesh-stages?").expect("preferred stages");
    let (_, ordinary, _, registry) = pollster::block_on(DeviceCapabilities::open_generation(
        &gate.instance,
        &options,
        1,
        &[],
        base,
    ))
    .expect("ordinary device");
    assert_eq!(ordinary.features(), base(&gate.adapter).required_features);
    assert_eq!(ordinary.limits(), base(&gate.adapter).required_limits);
    assert_eq!(registry.hardware(), []);
    let (_, staged, _, registry) = pollster::block_on(DeviceCapabilities::open_generation(
        &gate.instance,
        &options,
        2,
        &required,
        base,
    ))
    .expect("required stages device");
    assert!(
        staged
            .features()
            .contains(wgpu::Features::EXPERIMENTAL_MESH_SHADER)
    );
    assert_eq!(registry.features(), staged.features());
    assert_eq!(registry.limits(), &staged.limits());
    assert!(registry.hardware()[0].declined.is_none());
    assert!(staged.limits().max_mesh_output_vertices >= 64);
    assert!(staged.limits().max_mesh_output_primitives >= 124);
    assert!(staged.limits().max_task_payload_size >= 256);
    assert!(staged.limits().max_mesh_workgroup_total_count >= 64);
    assert!(staged.limits().max_storage_buffers_per_shader_stage >= 6);

    let attempts = Cell::new(0);
    let unsupported = wgpu::Features::EXPERIMENTAL_RAY_HIT_VERTEX_RETURN;
    assert!(!gate.adapter.features().contains(unsupported));
    let (_, fallback, _, registry) = pollster::block_on(DeviceCapabilities::open_generation(
        &gate.instance,
        &options,
        3,
        &preferred,
        |adapter| {
            let mut descriptor = base(adapter);
            if attempts.replace(attempts.get() + 1) == 0 {
                // Force the real wgpu request to fail after mesh-stages admission.
                descriptor.required_features |= unsupported;
            }
            descriptor
        },
    ))
    .expect("ordinary fallback");
    assert_eq!(attempts.get(), 2);
    assert_eq!(fallback.features(), ordinary.features());
    assert_eq!(fallback.limits(), ordinary.limits());
    let decline = registry.hardware()[0]
        .declined
        .as_deref()
        .expect("visible retry reason");
    assert!(
        decline.contains("device request with preferred hardware failed"),
        "{decline}"
    );
    writeln!(
        std::io::stdout(),
        "Metal adapter {:?}; enabled mesh limits {:?}; preferred fallback {decline}",
        gate.adapter.get_info(),
        staged.limits()
    )
    .expect("launch evidence");
}
