use super::{Answer, CascadeSample, Component, Point, Settings};

#[test]
fn query_records_match_the_shader_layout() {
    let source = concat!(
        include_str!("../../gains.wgsl"),
        include_str!("../sample.wgsl")
    );
    let shader = wgpu::naga::front::wgsl::parse_str(source).expect("water query shader");
    macro_rules! check {
        ($host:ty, $name:literal, $($member:ident),+ $(,)?) => {
            let ty = shader.types.iter().map(|(_, ty)| ty).find(|ty| ty.name.as_deref() == Some($name)).expect($name);
            let wgpu::naga::TypeInner::Struct { members, span } = &ty.inner else {
                panic!("expected a shader struct");
            };
            assert_eq!(*span as usize, size_of::<$host>(), "{}", $name);
            let host = [$((stringify!($member), std::mem::offset_of!($host, $member))),+];
            assert_eq!(members.len(), host.len(), "{}", $name);
            for (member, (name, offset)) in members.iter().zip(host) {
                assert_eq!(member.name.as_deref(), Some(name));
                assert_eq!(member.offset as usize, offset, "{}.{name}", $name);
            }
        };
    }
    check!(
        Point,
        "QueryPoint",
        position,
        radius,
        dt,
        level_current_bed,
        slope,
        body,
        has_bed,
        gains
    );
    check!(
        Component,
        "Component",
        vector,
        amplitude,
        phase,
        omega,
        inverse_k,
        padding
    );
    check!(
        Settings,
        "Settings",
        origin,
        count,
        components,
        lengths_chop,
        heights,
        swash_margin
    );
    check!(
        CascadeSample,
        "CascadeSample",
        height,
        slope,
        differential,
        velocity
    );
    check!(
        Answer,
        "Answer",
        height_slope_rate,
        velocity_depth,
        layers,
        current_bed,
        body_flags,
        cascades,
        padding
    );
}
