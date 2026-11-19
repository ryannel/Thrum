//! Device request profiles and restrictions on Apple's Metal adapter.

#[test]
fn native_portable_and_restricted_devices_on_metal() {
    use thrum_render::capabilities::{
        DeviceCapabilities, DeviceProfile, DeviceRecipe, DeviceRestriction,
        FIRST_DEVICE_GENERATION, HardwareLaunchError,
    };

    let Some(gpu) = thrum_test_support::gpu(
        "native_portable_and_restricted_devices_on_metal",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        // Exercise the shell entry point after the gate has admitted the target backend.
        let instance = &gpu.instance;
        let native = DeviceRecipe::default().open(instance, None).await.unwrap();
        let content = DeviceCapabilities::requested_content_features(&native.adapter);
        assert_eq!(native.capabilities.adapter().backend, wgpu::Backend::Metal);
        assert!(native.capabilities.features().contains(content));
        assert!(
            native
                .capabilities
                .features()
                .contains(wgpu::Features::TEXTURE_COMPRESSION_ASTC)
        );
        assert_eq!(native.capabilities.limits(), &native.adapter.limits());
        assert_eq!(native.capabilities.generation(), FIRST_DEVICE_GENERATION);
        assert_eq!(native.watch.generation, FIRST_DEVICE_GENERATION);
        eprintln!(
            "{} native content {content:?}; limits {:?}",
            native.capabilities.adapter().name,
            native.capabilities.limits()
        );

        let portable = DeviceRecipe {
            profile: DeviceProfile::Portable,
            ..Default::default()
        };
        let browser_request = portable.open(instance, None).await.unwrap();
        assert_eq!(
            browser_request.capabilities.limits(),
            &wgpu::Limits::default()
        );
        assert_eq!(
            browser_request.capabilities.features(),
            DeviceCapabilities::requested_timing_features(&browser_request.adapter)
        );

        let limits = wgpu::Limits {
            max_storage_buffers_per_shader_stage: 7,
            ..Default::default()
        };
        let restricted = DeviceRecipe {
            restriction: Some(&DeviceRestriction {
                limits: Some(limits.clone()),
                features: Some(wgpu::Features::empty()),
                ..Default::default()
            }),
            ..portable
        }
        .open(instance, None)
        .await
        .unwrap();
        assert_eq!(restricted.capabilities.limits(), &limits);
        assert!(restricted.capabilities.features().is_empty());

        for restriction in [
            DeviceRestriction {
                features: Some(wgpu::Features::TEXTURE_COMPRESSION_ASTC),
                ..Default::default()
            },
            DeviceRestriction {
                limits: Some(wgpu::Limits {
                    max_storage_buffers_per_shader_stage: wgpu::Limits::default()
                        .max_storage_buffers_per_shader_stage
                        + 1,
                    ..Default::default()
                }),
                ..Default::default()
            },
        ] {
            let result = DeviceRecipe {
                restriction: Some(&restriction),
                ..portable
            }
            .open(instance, None)
            .await;
            assert!(matches!(
                result,
                Err(HardwareLaunchError::InvalidRestriction)
            ));
        }
    });
}
