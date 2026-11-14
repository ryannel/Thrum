//! Default WebGPU limits on the Metal target.
mod support;
#[test]
fn default_webgpu_limits_and_named_profile_refusals() {
    support::ambient(true, "portable");
    let h = support::Harness::new(true);
    let site = thrum_render::runtime::DeviceSite::untracked(h.device);
    for (profile, name) in [
        (
            thrum_sea::Profile {
                transform_size: 512,
                ..Default::default()
            },
            "transform_size",
        ),
        (
            thrum_sea::Profile {
                cascades: 4,
                ..Default::default()
            },
            "cascades",
        ),
        (
            thrum_sea::Profile {
                cache_levels: 9,
                ..Default::default()
            },
            "cache_levels",
        ),
    ] {
        assert!(
            thrum_sea::Ambient::new(&site, profile)
                .err()
                .unwrap()
                .contains(name)
        );
    }
}
