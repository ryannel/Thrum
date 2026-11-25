use super::SunOpticalDepth;

#[test]
fn gpu_mapping_layout_matches_both_plain_and_atlas_shader() {
    use super::GpuSunMapping;
    use wgpu::naga;
    let source = format!(
        "@group(1) @binding(0) var aerial_sampler:sampler;\n{}\n@compute @workgroup_size(1) fn main() {{ let factor=cloud_factor(vec3<f32>(0.0)); }}",
        crate::shaders::CloudVariant::Gpu.sampling_source()
    );
    let module = naga::front::wgsl::parse_str(&source).expect("GPU cloud-factor source");
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("valid GPU cloud-factor operation");
    let (_, ty) = module
        .types
        .iter()
        .find(|(_, ty)| ty.name.as_deref() == Some(stringify!(GpuSunMapping)))
        .expect("mapping");
    let naga::TypeInner::Struct { members, span } = &ty.inner else {
        panic!("mapping struct")
    };
    assert_eq!(*span as usize, size_of::<GpuSunMapping>());
    let offsets = [
        std::mem::offset_of!(GpuSunMapping, region),
        std::mem::offset_of!(GpuSunMapping, plane_drift),
        std::mem::offset_of!(GpuSunMapping, path_border),
        std::mem::offset_of!(GpuSunMapping, far),
        std::mem::offset_of!(GpuSunMapping, near),
        std::mem::offset_of!(GpuSunMapping, middle),
    ];
    assert_eq!(members.len(), offsets.len());
    for (member, offset) in members.iter().zip(offsets) {
        assert_eq!(member.offset as usize, offset);
    }
}

fn field() -> SunOpticalDepth {
    SunOpticalDepth::new(
        [2, 2],
        &[0.0, 0.5, 1.0, 2.0],
        100.0,
        [-1.0, -2.0, 3.0, 4.0],
        0.01,
        7,
    )
    .unwrap()
}

fn word(bytes: &[u8], index: usize) -> f32 {
    f32::from_le_bytes(bytes[index * 4..index * 4 + 4].try_into().unwrap())
}

#[test]
fn advection_samples_the_supplied_time_without_changing_source_or_contents() {
    let initial = field();
    let moving = initial
        .clone()
        .with_drift([10.0, -2.0])
        .unwrap()
        .with_displacement([1.0, 4.0])
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(&initial.pixels, &moving.pixels));
    assert_eq!(initial.source_epoch(), moving.source_epoch());
    let packed = moving.pack(2.0);
    assert_eq!(word(&packed, 5), 21.0);
    assert_eq!(word(&packed, 6), 0.0);
    assert_eq!(word(&packed, 8), 1.0);
    assert_eq!(word(&moving.with_validity(false).pack(2.0), 8), 0.0);
}

#[test]
fn unsupported_advection_disables_the_sample_without_nonfinite_gpu_values() {
    let moving = field().with_drift([f32::MAX, 1.0]).unwrap();
    let packed = moving.pack(f64::MAX);
    assert!(
        packed
            .as_chunks::<4>()
            .0
            .iter()
            .all(|value| f32::from_le_bytes(*value).is_finite())
    );
    assert_eq!(word(&packed, 8), 0.0);
}

#[test]
fn invalid_optical_depth_or_mapping_is_rejected_before_upload() {
    for value in [-1.0, f32::NAN, f32::INFINITY, 65_505.0] {
        assert!(
            SunOpticalDepth::new([1, 1], &[value], 100.0, [0.0, 0.0, 1.0, 1.0], 0.01, 1).is_err()
        );
    }
    for extent in [[0, 1], [2, 2], [u32::MAX, u32::MAX]] {
        assert!(
            SunOpticalDepth::new(extent, &[0.0], 100.0, [0.0, 0.0, 1.0, 1.0], 0.01, 1).is_err()
        );
    }
    for sine in [0.0, -0.1, 1.0, f32::NAN] {
        assert!(
            SunOpticalDepth::new([1, 1], &[0.0], 100.0, [0.0, 0.0, 1.0, 1.0], sine, 1).is_err()
        );
    }
    assert!(field().with_displacement([f32::NAN, 0.0]).is_err());
    assert!(field().with_drift([0.0, f32::INFINITY]).is_err());
}
