use super::Uniform;

#[test]
fn sun_shader_and_uniform_layout_validate() {
    use wgpu::naga;
    let module =
        naga::front::wgsl::parse_str(include_str!("../sun_path.wgsl")).expect("Sun path WGSL");
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("valid Sun compute entries");
    let (_, ty) = module
        .types
        .iter()
        .find(|(_, ty)| ty.name.as_deref() == Some("Params"))
        .expect("parameters");
    let naga::TypeInner::Struct { members, span } = &ty.inner else {
        panic!("parameters struct")
    };
    assert_eq!(*span as usize, size_of::<Uniform>());
    let offsets = [
        std::mem::offset_of!(Uniform, weather),
        std::mem::offset_of!(Uniform, origin),
        std::mem::offset_of!(Uniform, direction),
        std::mem::offset_of!(Uniform, physical),
        std::mem::offset_of!(Uniform, erosion),
        std::mem::offset_of!(Uniform, noise),
        std::mem::offset_of!(Uniform, rows),
    ];
    assert_eq!(members.len(), offsets.len());
    for (member, offset) in members.iter().zip(offsets) {
        assert_eq!(member.offset as usize, offset);
    }
}
