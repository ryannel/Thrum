//! Default WebGPU limits on the Metal target.
mod support;
#[test]
fn default_webgpu_limits_and_named_profile_refusals() {
    support::ambient(true, "portable");
    let Some(h) = support::Harness::new(true) else {
        return;
    };
    let site = thrum_render::runtime::DeviceSite::untracked(h.device);
    for (profile, name) in [
        (
            thrum_sea::Profile {
                transform_size: 512,
                ..Default::default()
            },
            thrum_sea::ProfileField::TransformSize,
        ),
        (
            thrum_sea::Profile {
                cascades: 4,
                ..Default::default()
            },
            thrum_sea::ProfileField::Cascades,
        ),
        (
            thrum_sea::Profile {
                cache_levels: 9,
                ..Default::default()
            },
            thrum_sea::ProfileField::CacheLevels,
        ),
    ] {
        assert!(
            thrum_sea::Ambient::new(&site, profile)
                .err()
                .is_some_and(|error| matches!(error, thrum_sea::AmbientError::UnsupportedProfile(field) if field == name))
        );
    }
    let Some(gpu) = thrum_test_support::gpu(
        "default_webgpu_limits_and_named_profile_refusals",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    let (device, _queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction {
            storage_buffers: Some(7),
            features: Some(wgpu::Features::empty()),
        },
    );
    let site = thrum_render::runtime::DeviceSite::untracked(device);
    let error = thrum_sea::Ambient::new(&site, thrum_sea::Profile::default())
        .err()
        .expect("refusal before shader preparation");
    assert!(
        matches!(
            error,
            thrum_sea::AmbientError::DeviceLimit {
                limit: "max_storage_buffers_per_shader_stage",
                ..
            }
        ),
        "{error}"
    );
}
