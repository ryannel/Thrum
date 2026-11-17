use super::*;

#[test]
fn shader_record_matches_rust_offsets() {
    let module = wgpu::naga::front::wgsl::parse_str(
        include_str!("../layer.wgsl")
            .split_once("@group")
            .expect("bindings follow record")
            .0,
    )
    .expect("layer syntax");
    let (_, ty) = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some("CloudParams"))
        .expect("record");
    let wgpu::naga::TypeInner::Struct { members, span } = &ty.inner else {
        panic!("record struct")
    };
    assert_eq!(*span as usize, size_of::<Uniform>());
    let offsets = [
        std::mem::offset_of!(Uniform, weather),
        std::mem::offset_of!(Uniform, physical),
        std::mem::offset_of!(Uniform, erosion),
        std::mem::offset_of!(Uniform, drift),
        std::mem::offset_of!(Uniform, heights),
        std::mem::offset_of!(Uniform, settings),
    ];
    for (member, offset) in members.iter().zip(offsets) {
        assert_eq!(member.offset as usize, offset);
    }
}
