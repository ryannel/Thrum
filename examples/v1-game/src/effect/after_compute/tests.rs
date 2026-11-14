#[test]
fn all_late_compute_and_composition_shaders_validate() {
    for source in [
        include_str!("trace.wgsl"),
        include_str!("resolve.wgsl"),
        include_str!("compose.wgsl"),
        include_str!("epilogue.wgsl"),
    ] {
        let module = wgpu::naga::front::wgsl::parse_str(source).expect("late witness WGSL parses");
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("late witness shader validates");
    }
}
