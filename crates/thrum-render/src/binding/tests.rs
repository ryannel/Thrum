use super::*;

const STAGES: wgpu::ShaderStages = wgpu::ShaderStages::COMPUTE;

#[test]
fn each_constructor_gives_the_literal_it_replaces() {
    let buffer_entry = |ty, min_binding_size| wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: STAGES,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size,
        },
        count: None,
    };
    assert_eq!(
        uniform(1, STAGES, 64),
        buffer_entry(wgpu::BufferBindingType::Uniform, wgpu::BufferSize::new(64))
    );
    assert_eq!(
        uniform(1, STAGES, 0),
        buffer_entry(wgpu::BufferBindingType::Uniform, None)
    );
    for read_only in [false, true] {
        assert_eq!(
            storage(1, STAGES, read_only, 16),
            buffer_entry(
                wgpu::BufferBindingType::Storage { read_only },
                wgpu::BufferSize::new(16)
            )
        );
    }
    assert_eq!(
        buffer(1, STAGES, wgpu::BufferBindingType::Uniform, 0),
        uniform(1, STAGES, 0)
    );
    assert_eq!(
        texture(
            2,
            STAGES,
            wgpu::TextureSampleType::Depth,
            wgpu::TextureViewDimension::D2Array
        ),
        wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: STAGES,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Depth,
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        }
    );
    assert_eq!(
        sampler(3, STAGES, wgpu::SamplerBindingType::Filtering),
        wgpu::BindGroupLayoutEntry {
            binding: 3,
            visibility: STAGES,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        }
    );
    assert_eq!(
        storage_texture(
            4,
            STAGES,
            wgpu::StorageTextureAccess::WriteOnly,
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureViewDimension::D2
        ),
        wgpu::BindGroupLayoutEntry {
            binding: 4,
            visibility: STAGES,
            ty: wgpu::BindingType::StorageTexture {
                access: wgpu::StorageTextureAccess::WriteOnly,
                format: wgpu::TextureFormat::Rgba16Float,
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        }
    );
    assert_eq!(
        acceleration_structure(5, wgpu::ShaderStages::FRAGMENT),
        wgpu::BindGroupLayoutEntry {
            binding: 5,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::AccelerationStructure {
                vertex_return: false,
            },
            count: None,
        }
    );
}

/// The device accepts a layout of every kind a portable device supports.
#[test]
fn a_device_accepts_a_layout_of_every_portable_kind() {
    let Some(crate::test_device::Opened { device, .. }) = crate::test_device::open(
        "binding::a_device_accepts_a_layout_of_every_portable_kind",
        None,
        crate::capabilities::DeviceProfile::Portable,
        &crate::capabilities::DeviceRestriction {
            features: Some(wgpu::Features::empty()),
            ..Default::default()
        },
    ) else {
        return;
    };
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let _layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("binding kinds"),
        entries: &[
            uniform(0, STAGES, 16),
            storage(1, STAGES, true, 0),
            storage(2, STAGES, false, 4),
            texture(
                3,
                STAGES,
                wgpu::TextureSampleType::Float { filterable: true },
                wgpu::TextureViewDimension::D2,
            ),
            sampler(4, STAGES, wgpu::SamplerBindingType::Filtering),
            storage_texture(
                5,
                STAGES,
                wgpu::StorageTextureAccess::WriteOnly,
                wgpu::TextureFormat::Rgba16Float,
                wgpu::TextureViewDimension::D2,
            ),
        ],
    });
    let error = pollster::block_on(scope.pop());
    assert!(error.is_none(), "{error:?}");
}
