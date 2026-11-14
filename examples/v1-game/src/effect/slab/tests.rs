#[test]
fn both_slab_variants_validate_before_device_qualification() {
    for refraction in [false, true] {
        let module = wgpu::naga::front::wgsl::parse_str(&super::shader(refraction))
            .expect("slab WGSL parses");
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("slab vertex/fragment and texture operations validate");
    }
}
