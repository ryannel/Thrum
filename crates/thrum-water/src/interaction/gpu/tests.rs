use super::{Cell, Interaction, Patch, SampleSettings, Step, shader_constants};

#[test]
fn interaction_records_match_the_shader_layout() {
    let step = shader_constants() + include_str!("../step.wgsl");
    let sample = Interaction::sample_source(0, 0, 1, true);
    let validate = |source: &str| {
        let shader = wgpu::naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|error| panic!("{}", error.emit_to_string(source)));
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::empty(),
        )
        .validate(&shader)
        .expect("portable interaction shader");
        shader
    };
    let step = validate(&step);
    let sample = validate(&sample);
    macro_rules! check {
        ($shader:ident, $host:ty, $name:literal, $($field:ident),+ $(,)?) => {
            let ty = $shader.types.iter().map(|(_, ty)| ty)
                .find(|ty| ty.name.as_deref() == Some($name)).expect($name);
            let wgpu::naga::TypeInner::Struct { members, span } = &ty.inner else {
                panic!("expected a shader struct");
            };
            assert_eq!(*span as usize, size_of::<$host>(), "{}", $name);
            let host = [$((stringify!($field), std::mem::offset_of!($host, $field))),+];
            assert_eq!(members.len(), host.len(), "{}", $name);
            for (member, (name, offset)) in members.iter().zip(host) {
                assert_eq!(member.name.as_deref(), Some(name));
                assert_eq!(member.offset as usize, offset, "{}.{name}", $name);
            }
        };
    }
    check!(
        step,
        Step,
        "Step",
        shift,
        current,
        old_base,
        new_base,
        cell_base,
        region,
        source_first,
        source_count,
        depth_index,
        level,
        metres,
        depth,
        seconds,
        ticks
    );
    check!(step, Cell, "Cell", region, low, high, blend);
    check!(
        step, Patch, "Patch", place, axis, footprint, impulse, padding
    );
    check!(
        sample,
        SampleSettings,
        "WakeSettings",
        origins,
        clocks,
        bases
    );
}
