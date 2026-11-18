//! Metal tests of the registry's experimental-device admission.
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;

#[cfg(all(target_os = "macos", feature = "creation-counters"))]
mod metal {
    use super::ray_support::support;

    use std::cell::Cell;
    use thrum_render::capabilities::{
        DeviceCapabilities, HardwareFeature, HardwareLaunchError, HardwareRequest,
    };
    use thrum_test_support::{Profile, Restriction};

    #[test]
    fn mac_launch_admits_rays_and_keeps_the_original_recipe_on_fallback() {
        let gate = support::gate("ray query admission");
        let options = wgpu::RequestAdapterOptions::default();
        let base = |adapter: &wgpu::Adapter| {
            Profile::Portable.descriptor(adapter, &Restriction::default())
        };
        let ray = wgpu::Features::EXPERIMENTAL_RAY_QUERY;
        let mask = wgpu::Features::all() - ray;
        let required = HardwareRequest::parse("ray-query").expect("required");
        let preferred = HardwareRequest::parse("ray-query?").expect("preferred");
        assert!(matches!(HardwareRequest::parse("ray-typo"),
        Err(HardwareLaunchError::Unknown(name)) if name == "ray-typo"));
        let open = |names: &[HardwareRequest], mask| {
            pollster::block_on(DeviceCapabilities::open_generation_with_feature_mask(
                &gate.instance,
                &options,
                1,
                names,
                base,
                mask,
            ))
        };
        let (_, ordinary, _, ordinary_caps) =
            open(&[], wgpu::Features::all()).expect("ordinary device");
        assert_eq!(ordinary_caps.hardware(), []);
        assert_eq!(ordinary.features(), base(&gate.adapter).required_features);
        assert_eq!(ordinary.limits(), base(&gate.adapter).required_limits);
        let (_, required_device, _, caps) =
            open(&required, wgpu::Features::all()).expect("required rays");
        assert!(required_device.features().contains(ray));
        assert_eq!(caps.features(), required_device.features());
        assert_eq!(
            caps.hardware()[0].request.feature,
            HardwareFeature::RayQuery
        );
        assert!(caps.hardware()[0].declined.is_none());
        assert!(caps.limits().max_tlas_instance_count >= 100_513);
        assert!(matches!(open(&required, mask),
        Err(HardwareLaunchError::Required { feature: HardwareFeature::RayQuery, reason })
        if reason.contains("does not offer")));

        let (_, fallback, _, caps) =
            open(&preferred, mask).expect("preference leaves the base device usable");
        assert_eq!(fallback.features(), ordinary.features());
        assert_eq!(fallback.limits(), ordinary.limits());
        assert!(
            caps.hardware()[0]
                .declined
                .as_deref()
                .unwrap_or_default()
                .contains("does not offer")
        );
    }

    #[test]
    fn failed_preference_retries_once_and_retains_every_required_name() {
        let gate = support::gate("ray query device retry");
        let options = wgpu::RequestAdapterOptions::default();
        let base = |adapter: &wgpu::Adapter| {
            Profile::Portable.descriptor(adapter, &Restriction::default())
        };
        let unsupported = wgpu::Features::EXPERIMENTAL_RAY_HIT_VERTEX_RETURN;
        assert!(!gate.adapter.features().contains(unsupported));
        for required_name in ["", "mesh-stages"] {
            let required = HardwareRequest::parse(required_name).expect("base names");
            let mut mixed = required.clone();
            mixed.extend(HardwareRequest::parse("ray-query?").expect("preferred rays"));
            let (_, expected, _, _) = pollster::block_on(DeviceCapabilities::open_generation(
                &gate.instance,
                &options,
                1,
                &required,
                base,
            ))
            .expect("base recipe with required names");
            let attempts = Cell::new(0);
            let (_, device, _, caps) = pollster::block_on(DeviceCapabilities::open_generation(
                &gate.instance,
                &options,
                2,
                &mixed,
                |adapter| {
                    let mut descriptor = base(adapter);
                    if attempts.replace(attempts.get() + 1) == 0 {
                        // The registry still admits the preference; wgpu rejects the
                        // resulting device request with this unsupported test flag.
                        descriptor.required_features |= unsupported;
                    }
                    descriptor
                },
            ))
            .expect("one fresh-adapter retry");
            assert_eq!(attempts.get(), 2);
            assert_eq!(device.features(), expected.features());
            assert_eq!(device.limits(), expected.limits());
            assert_eq!(caps.hardware().len(), mixed.len());
            let (ray_query, retained) = caps.hardware().split_last().expect("selection");
            assert_eq!(ray_query.request.feature, HardwareFeature::RayQuery);
            assert!(
                ray_query
                    .declined
                    .as_deref()
                    .unwrap_or_default()
                    .contains("device request with preferred hardware failed")
            );
            assert!(
                retained
                    .iter()
                    .all(|answer| answer.declined.is_none() && answer.request.required)
            );
        }
    }
}
