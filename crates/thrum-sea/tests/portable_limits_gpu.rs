//! Default WebGPU limits on the Metal target.
use thrum_render::capabilities::{DeviceProfile, DeviceRecipe, DeviceRestriction};

use crate::support;

// The frame script, the full grid at each clock age and the refusals under default WebGPU
// limits are each one test, so the runner spreads them over the cores.
#[test]
fn default_webgpu_limits_frame_script() {
    support::frame_script(true);
}

#[test]
fn default_webgpu_limits_match_the_model_early() {
    support::conformance(true, support::Age::Early, "portable");
}

#[test]
fn default_webgpu_limits_match_the_model_a_day_in() {
    support::conformance(true, support::Age::Day, "portable");
}

#[test]
fn default_webgpu_limits_and_named_profile_refusals() {
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
    let opened = pollster::block_on(
        DeviceRecipe {
            profile: DeviceProfile::Portable,
            restriction: Some(&DeviceRestriction {
                limits: Some(wgpu::Limits {
                    max_storage_buffers_per_shader_stage: 7,
                    ..Default::default()
                }),
                features: Some(wgpu::Features::empty()),
                ..Default::default()
            }),
            ..Default::default()
        }
        .open_admitted(&gpu.instance, &gpu.adapter),
    )
    .expect("restricted Metal recipe");
    let error = thrum_sea::Ambient::new(opened.capabilities.site(), thrum_sea::Profile::default())
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
