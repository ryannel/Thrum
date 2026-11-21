//! Explicit launch requests on this Mac, including a failed preferred device request.
use std::{cell::Cell, io::Write as _};
use thrum_render::capabilities::{
    DeviceRecipe, HardwareFeature, HardwareLaunchError, HardwareRequest,
};
use thrum_test_support::{Profile, Restriction};

#[test]
fn descriptor_adjustment_respects_recipe_restrictions() {
    let restriction = Restriction {
        storage_buffers: Some(7),
        features: Some(wgpu::Features::empty()),
    };
    let profile = Profile::Portable;
    let Some(thrum_test_support::Opened {
        gpu: gate, device, ..
    }) = thrum_test_support::open(
        "descriptor_adjustment_respects_recipe_restrictions",
        Some(wgpu::Backend::Metal),
        profile,
        &restriction,
    )
    else {
        return;
    };
    assert_eq!(device.limits().max_storage_buffers_per_shader_stage, 7);
    assert!(device.features().is_empty());
    let raised = gate.fresh_adapter().device_with_hardware_adjusted(
        profile,
        &restriction,
        2,
        &[],
        |descriptor| {
            descriptor
                .required_limits
                .max_storage_buffers_per_shader_stage = 8;
        },
    );
    assert!(matches!(
        raised,
        Err(HardwareLaunchError::InvalidRestriction)
    ));
    let restored = gate.fresh_adapter().device_with_hardware_adjusted(
        profile,
        &restriction,
        3,
        &[],
        |descriptor| {
            descriptor.required_features = wgpu::Features::TIMESTAMP_QUERY;
        },
    );
    assert!(
        gate.adapter
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY)
    );
    assert!(matches!(
        restored,
        Err(HardwareLaunchError::InvalidRestriction)
    ));
}

#[test]
fn metal_launch_and_preference_retry() {
    let Some(gate) = thrum_test_support::gpu("mesh-stage launch", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let profile = Profile::Portable;
    let unrestricted = Restriction::default();
    let descriptor = |restriction: &Restriction| {
        DeviceRecipe {
            profile,
            restriction: Some(&restriction.for_profile(profile, &gate.adapter)),
            ..Default::default()
        }
        .descriptor(&gate.adapter)
        .expect("valid base recipe")
    };
    let base = descriptor(&unrestricted);
    let open = |requests: &[HardwareRequest]| {
        gate.fresh_adapter()
            .device_with_hardware(profile, &unrestricted, 1, requests)
    };
    assert!(matches!(
        HardwareRequest::parse("mesh_typo"),
        Err(HardwareLaunchError::Unknown(name)) if name == "mesh_typo"
    ));
    let required = HardwareRequest::parse("mesh-stages").expect("required mesh stages");
    let preferred = HardwareRequest::parse("mesh-stages?").expect("preferred mesh stages");
    let (ordinary, _, registry) = open(&[]).expect("ordinary gate launch");
    assert_eq!(ordinary.features(), base.required_features);
    assert_eq!(ordinary.limits(), base.required_limits);
    assert_eq!(registry.hardware(), []);
    let (legacy, _) = gate.device(profile, &unrestricted);
    assert_eq!(legacy.features(), ordinary.features());
    assert_eq!(legacy.limits(), ordinary.limits());
    drop(legacy);
    let stages = wgpu::Features::EXPERIMENTAL_MESH_SHADER;
    let (staged, _, registry) = open(&required).expect("required stages device");
    assert!(staged.features().contains(stages));
    assert_eq!(registry.features(), staged.features());
    assert_eq!(registry.limits(), &staged.limits());
    assert!(registry.hardware()[0].declined.is_none());
    assert!(staged.limits().max_mesh_output_vertices >= 64);
    assert!(staged.limits().max_mesh_output_primitives >= 124);
    assert!(staged.limits().max_task_payload_size >= 256);
    assert!(staged.limits().max_mesh_workgroup_total_count >= 64);
    assert!(staged.limits().max_storage_buffers_per_shader_stage >= 6);

    for (restriction, expected) in [
        (
            Restriction {
                features: Some(wgpu::Features::all() - stages),
                ..Default::default()
            },
            "does not offer",
        ),
        (
            Restriction {
                storage_buffers: Some(5),
                ..Default::default()
            },
            "cannot grant the needed limits",
        ),
    ] {
        let open_restricted = |requests: &[HardwareRequest]| {
            gate.fresh_adapter()
                .device_with_hardware(profile, &restriction, 2, requests)
        };
        assert!(matches!(
            open_restricted(&required),
            Err(HardwareLaunchError::Required { feature: HardwareFeature::MeshStages, reason })
            if reason.contains(expected)
        ));
        let (fallback, _, registry) = open_restricted(&preferred).expect("restricted fallback");
        let base = descriptor(&restriction);
        assert_eq!(fallback.features(), base.required_features);
        assert_eq!(fallback.limits(), base.required_limits);
        let decline = registry.hardware()[0].declined.as_deref().expect("decline");
        assert!(decline.contains(expected), "{decline}");
        writeln!(
            std::io::stdout(),
            "restricted gate: required refused, preferred used original recipe: {decline}"
        )
        .expect("restriction evidence");
    }

    let attempts = Cell::new(0);
    let unsupported = wgpu::Features::EXPERIMENTAL_RAY_HIT_VERTEX_RETURN;
    assert!(!gate.adapter.features().contains(unsupported));
    let (fallback, _, registry) = gate
        .fresh_adapter()
        .device_with_hardware_adjusted(profile, &unrestricted, 3, &preferred, |descriptor| {
            if attempts.replace(attempts.get() + 1) == 0 {
                // Force the real wgpu request to fail after mesh-stages admission.
                descriptor.required_features |= unsupported;
            }
        })
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
        "Metal adapter {:?}; gate checked the empty launch and enabled mesh limits {:?}; failed preference {decline}",
        gate.adapter.get_info(),
        staged.limits()
    )
    .expect("launch evidence");
}
