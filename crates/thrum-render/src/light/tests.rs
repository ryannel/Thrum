use super::*;
use std::mem::offset_of;
use wgpu::naga::{TypeInner, front::wgsl::parse_str};

#[test]
fn light_records_match_wgsl_size_and_member_offsets() {
    for (source, name, bytes, expected) in [
        (
            include_str!("../directional_light.wgsl"),
            "DirectionalLight",
            size_of::<DirectionalLight>(),
            &[
                (
                    "direction_to_light",
                    offset_of!(DirectionalLight, direction_to_light),
                ),
                ("irradiance", offset_of!(DirectionalLight, irradiance)),
                (
                    "half_vector_fresnel",
                    offset_of!(DirectionalLight, half_vector_fresnel),
                ),
                (
                    "ambient_irradiance",
                    offset_of!(DirectionalLight, ambient_irradiance),
                ),
            ][..],
        ),
        (
            include_str!("../environment_light.wgsl"),
            "EnvironmentLight",
            size_of::<EnvironmentLight>(),
            &[
                (
                    "horizontal_irradiance",
                    offset_of!(EnvironmentLight, horizontal_irradiance),
                ),
                ("sky_diffuse", offset_of!(EnvironmentLight, sky_diffuse)),
            ][..],
        ),
    ] {
        let module = parse_str(source).unwrap();
        let (_, record) = module
            .types
            .iter()
            .find(|(_, ty)| ty.name.as_deref() == Some(name))
            .unwrap();
        let TypeInner::Struct { span, members } = &record.inner else {
            panic!("{name} must be a struct");
        };
        assert_eq!(*span as usize, bytes, "{name}");
        assert_eq!(members.len(), expected.len(), "{name}");
        for (member, &(field, offset)) in members.iter().zip(expected) {
            assert_eq!(member.name.as_deref(), Some(field), "{name}");
            assert_eq!(member.offset as usize, offset, "{name}.{field}");
        }
    }
}
