use super::{GpuRect, GpuRow, GpuView};
use std::mem::{offset_of, size_of};
use wgpu::naga::{TypeInner, front::wgsl};

#[test]
fn sprite_abi_matches_wgsl_layout() {
    let module =
        wgsl::parse_str(&crate::shaders::Shader::Sprite.source()).expect("valid sprite WGSL");
    for (name, size, offsets) in [
        (
            "SpriteView",
            size_of::<GpuView>(),
            &[
                offset_of!(GpuView, clip_per_world),
                offset_of!(GpuView, centre_high),
                offset_of!(GpuView, centre_low),
                offset_of!(GpuView, alpha),
                offset_of!(GpuView, pixels_per_unit),
            ][..],
        ),
        (
            "SpriteRow",
            size_of::<GpuRow>(),
            &[
                offset_of!(GpuRow, current),
                offset_of!(GpuRow, previous),
                offset_of!(GpuRow, scales),
                offset_of!(GpuRow, rotation),
                offset_of!(GpuRow, rotation_delta),
                offset_of!(GpuRow, frame),
                offset_of!(GpuRow, sheet),
                offset_of!(GpuRow, tint),
                offset_of!(GpuRow, flash),
                offset_of!(GpuRow, additive),
                offset_of!(GpuRow, padding0),
                offset_of!(GpuRow, padding1),
                offset_of!(GpuRow, padding2),
            ][..],
        ),
        (
            "SpriteFrameRecord",
            size_of::<GpuRect>(),
            &[
                offset_of!(GpuRect, origin),
                offset_of!(GpuRect, size),
                offset_of!(GpuRect, page),
                offset_of!(GpuRect, kind),
                offset_of!(GpuRect, tile),
                offset_of!(GpuRect, padding),
            ][..],
        ),
    ] {
        let (_, ty) = module
            .types
            .iter()
            .find(|(_, ty)| ty.name.as_deref() == Some(name))
            .expect("sprite ABI struct");
        let TypeInner::Struct { members, span } = &ty.inner else {
            panic!("sprite ABI type must be a struct");
        };
        assert_eq!(*span as usize, size, "{name} size");
        assert_eq!(
            members
                .iter()
                .map(|member| member.offset as usize)
                .collect::<Vec<_>>(),
            offsets,
            "{name} member offsets"
        );
    }
}
