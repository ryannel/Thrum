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
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("Metal adapter");
        let limits = wgpu::Limits {
            max_storage_buffers_per_shader_stage: 7,
            ..wgpu::Limits::default()
        };
        let (device, _queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: limits,
                ..Default::default()
            })
            .await
            .expect("reduced limits device");
        let site = thrum_render::runtime::DeviceSite::untracked(device);
        let error = thrum_sea::Ambient::new(&site, thrum_sea::Profile::default())
            .err()
            .expect("refusal before shader preparation");
        assert!(
            error.contains("max_storage_buffers_per_shader_stage"),
            "{error}"
        );
    });
}
