use crate::synthetic::SyntheticVertex;

#[test]
fn packed_row_vertices_match_the_shader_storage_abi() {
    let shader = crate::shaders::wgsl!("synthetic.wgsl");
    let start = shader
        .find("struct MeshVertex ")
        .expect("arena vertex declaration");
    let end = start + shader[start..].find('}').expect("vertex end") + 1;
    let module = wgpu::naga::front::wgsl::parse_str(&shader[start..end]).expect("shader ABI");
    let (_, ty) = module
        .types
        .iter()
        .find(|(_, ty)| ty.name.as_deref() == Some("MeshVertex"))
        .expect("vertex type");
    let wgpu::naga::TypeInner::Struct { members, span } = &ty.inner else {
        panic!("vertex must be a struct");
    };
    assert_eq!(*span as usize, size_of::<SyntheticVertex>());
    assert_eq!(
        members
            .iter()
            .map(|member| member.offset)
            .collect::<Vec<_>>(),
        [0, 16, 32]
    );
    assert_eq!(std::mem::offset_of!(SyntheticVertex, position), 0);
    assert_eq!(std::mem::offset_of!(SyntheticVertex, normal), 12);
    assert_eq!(std::mem::offset_of!(SyntheticVertex, uv), 24);
    assert_eq!(std::mem::offset_of!(SyntheticVertex, tangent), 32);
}
