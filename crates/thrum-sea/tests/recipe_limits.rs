//! Sea preparation on native, portable and restricted Metal devices.

#[cfg(target_os = "macos")]
#[test]
fn sea_accepts_native_and_portable_limits_and_refuses_seven_storage_buffers() {
    use thrum_render::capabilities::{DeviceProfile, DeviceRecipe, DeviceRestriction};
    use thrum_sea::{Ambient, AmbientError, Profile};

    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        for profile in [DeviceProfile::Native, DeviceProfile::Portable] {
            let gpu = DeviceRecipe {
                profile,
                ..Default::default()
            }
            .open(&instance, None)
            .await
            .unwrap();
            Ambient::new(gpu.capabilities.site(), Profile::default()).expect("sea preparation");
            assert_eq!(gpu.watch.uncaptured_error(), None);
            eprintln!(
                "{profile:?}: sea prepared on {:?} with {} storage buffers per stage",
                gpu.capabilities.adapter().backend,
                gpu.capabilities
                    .limits()
                    .max_storage_buffers_per_shader_stage
            );
        }

        let limits = wgpu::Limits {
            max_storage_buffers_per_shader_stage: 7,
            ..Default::default()
        };
        let gpu = DeviceRecipe {
            profile: DeviceProfile::Portable,
            restriction: Some(&DeviceRestriction {
                limits: Some(limits),
                ..Default::default()
            }),
            ..Default::default()
        }
        .open(&instance, None)
        .await
        .unwrap();
        assert_eq!(
            gpu.capabilities
                .limits()
                .max_storage_buffers_per_shader_stage,
            7
        );
        assert!(matches!(
            Ambient::new(gpu.capabilities.site(), Profile::default()),
            Err(AmbientError::DeviceLimit {
                limit: "max_storage_buffers_per_shader_stage",
                ..
            })
        ));
    });
}
