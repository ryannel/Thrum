use super::Parameters;

#[test]
fn lent_wave_records_match_the_seas_host_layouts() {
    let shader = include_str!("../surface.wgsl");
    let layouts = thrum_sea::records::layouts();
    for name in ["Field"] {
        let declaration = shader
            .split_once(&format!("struct {name}"))
            .expect("record")
            .1
            .split_once('}')
            .expect("binding")
            .0;
        let module = wgpu::naga::front::wgsl::parse_str(&format!("struct {name}{declaration}}}"))
            .expect("WGSL record");
        let ty = module
            .types
            .iter()
            .find(|(_, t)| t.name.as_deref() == Some(name))
            .expect("type")
            .1;
        let wgpu::naga::TypeInner::Struct { members, span } = &ty.inner else {
            panic!("record");
        };
        let host = layouts
            .iter()
            .find(|l| l.name == name)
            .expect("sea host record");
        assert_eq!(*span as usize, host.size, "{name}");
        assert_eq!(members.len(), host.members.len(), "{name}");
        for (member, (_, offset)) in members.iter().zip(&host.members) {
            assert_eq!(member.offset as usize, *offset, "{name}");
        }
    }
}
#[test]
fn gpu_parameter_size_and_member_offsets_match_the_actual_shader() {
    let shader = include_str!("../field.wgsl");
    let declaration = shader
        .split_once("struct Parameters")
        .expect("parameters")
        .1
        .split_once("@group")
        .expect("binding")
        .0;
    let module = wgpu::naga::front::wgsl::parse_str(&format!("struct Parameters{declaration}"))
        .expect("WGSL record");
    let ty = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some("Parameters"))
        .expect("type")
        .1;
    let wgpu::naga::TypeInner::Struct { members, span } = &ty.inner else {
        panic!("record");
    };
    assert_eq!(*span as usize, size_of::<Parameters>());
    assert_eq!(
        members
            .iter()
            .map(|m| m.offset as usize)
            .collect::<Vec<_>>(),
        [
            std::mem::offset_of!(Parameters, level_dt_reset),
            std::mem::offset_of!(Parameters, gains),
            std::mem::offset_of!(Parameters, lengths),
            std::mem::offset_of!(Parameters, cache),
            std::mem::offset_of!(Parameters, long_covariance),
            std::mem::offset_of!(Parameters, scatter),
            std::mem::offset_of!(Parameters, heights)
        ]
    );
}
