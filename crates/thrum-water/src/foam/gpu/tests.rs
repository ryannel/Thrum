use super::{Foam, Settings};
use std::mem::offset_of;

#[test]
fn foam_settings_match_the_shader_layout() {
    let source = include_str!("../../field.wgsl").to_owned() + &Foam::source();
    let shader = wgpu::naga::front::wgsl::parse_str(&source).expect("foam sampling shader");
    let settings = shader
        .types
        .iter()
        .find_map(|(_, ty)| (ty.name.as_deref() == Some("FoamSettings")).then_some(&ty.inner))
        .expect("foam settings type");
    let wgpu::naga::TypeInner::Struct { members, span } = settings else {
        panic!("expected foam settings struct");
    };
    assert_eq!(*span as usize, size_of::<Settings>());
    let host = [
        offset_of!(Settings, origins),
        offset_of!(Settings, banks),
        offset_of!(Settings, time_wind),
        offset_of!(Settings, camera),
        offset_of!(Settings, jacobian_variance),
        offset_of!(Settings, long_bounds),
        offset_of!(Settings, hulls),
        offset_of!(Settings, components),
    ];
    assert!(members.iter().map(|member| member.offset as usize).eq(host));
}
